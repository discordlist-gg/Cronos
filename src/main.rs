#[macro_use]
extern crate tracing;

use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use governor::clock::DefaultClock;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::Quota;
use once_cell::sync::OnceCell;
use poem::http::{Method, StatusCode};
use poem::listener::TcpListener;
use poem::middleware::Cors;
use poem::{Endpoint, EndpointExt, IntoResponse, Request, Response, Route, Server};
use poem_openapi::{OpenApiService, Tags};
use tokio::sync::Semaphore;
use tracing_subscriber::filter::LevelFilter;

pub(crate) mod models;
mod routes;
pub(crate) mod search;
mod tasks;

type Ratelimiter = governor::RateLimiter<
    String,
    DefaultKeyedStateStore<String>,
    DefaultClock,
    governor::middleware::StateInformationMiddleware,
>;
static GLOBAL_RATELIMITER: OnceCell<Ratelimiter> = OnceCell::new();

#[derive(Tags)]
pub enum ApiTags {
    Bots,
    Packs,
}

#[derive(Debug, Parser)]
#[clap(version, about)]
pub struct Config {
    #[clap(short, long, env, default_value = "127.0.0.1:7700")]
    /// The address for the webserver to bind to.
    bind: String,

    #[clap(long, env, default_value = "info")]
    /// The level in which to display logs.
    log_level: LevelFilter,

    #[clap(long, env, default_value = "127.0.0.1:9042")]
    /// A list of known cluster nodes seperated by a `;`.
    cluster_nodes: String,

    #[clap(long)]
    init_tables: bool,

    #[clap(long, env)]
    data_path: String,

    #[clap(long, env, default_value_t = 50)]
    max_concurrency: usize,

    #[clap(short, long, env, default_value = "http://127.0.0.1:7700/v0")]
    /// The exposed address of the server.
    exposed_address: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Config = Config::parse();

    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var(
            "RUST_LOG",
            format!("{},poem=debug,scylla=info,hyper=info", args.log_level),
        );
    }
    tracing_subscriber::fmt::init();

    {
        let nodes = args.cluster_nodes.split(';').collect::<Vec<&str>>();

        models::connection::connect(&nodes, args.init_tables).await?;
    }

    tasks::start_vote_update_tasks();
    tasks::start_live_data_tasks();

    {
        let limiter = Arc::new(Semaphore::new(args.max_concurrency));
        let base_path = Path::new(&args.data_path);
        search::index_impls::bots::init_index(
            &base_path.join("bots"),
            limiter.clone(),
            args.max_concurrency,
        )
        .await?;

        search::index_impls::packs::init_index(
            &base_path.join("packs"),
            limiter.clone(),
            args.max_concurrency,
        )
        .await?;

        search::index_impls::packs::writer().full_refresh().await?;
        search::index_impls::bots::writer().full_refresh().await?;
    }

    let api_service = OpenApiService::new(
        (routes::bots::BotApi, routes::packs::PackApi),
        "Cronos API",
        env!("CARGO_PKG_VERSION"),
    )
    .description("The Dlist api system.")
    .server(args.exposed_address);

    let ui = api_service.redoc();
    let spec = api_service.spec();

    let app = Route::new()
        .nest("/v0", api_service)
        .nest("/ui", ui)
        .at("/spec", poem::endpoint::make_sync(move |_| spec.clone()))
        .around(global_ratelimiter)
        .around(log)
        .with(
            Cors::new()
                .allow_origins([
                    "http://localhost:3000",
                    "https://discordlist.gg",
                    "https://beta.discordlist.gg",
                ])
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::DELETE,
                    Method::PUT,
                    Method::OPTIONS,
                ])
                .allow_credentials(true)
                .max_age(200),
        );

    Server::new(TcpListener::bind(args.bind))
        .run_with_graceful_shutdown(
            app,
            async move {
                let _ = tokio::signal::ctrl_c().await;
            },
            Some(Duration::from_secs(2)),
        )
        .await?;

    Ok(())
}

macro_rules! get_limit {
    ($env_var:expr) => {{
        std::env::var_os($env_var)
            .map(|v| v.to_string_lossy().parse::<u32>())
            .transpose()
            .ok()
            .flatten()
            .map(NonZeroU32::new)
            .flatten()
    }};
}

async fn global_ratelimiter<E: Endpoint>(
    next: E,
    req: Request,
) -> poem::Result<Response> {
    let limiter = GLOBAL_RATELIMITER.get_or_init(|| {
        let limit_per_sec =
            get_limit!("RATELIMITER_QUOTA_PER_SEC").map(Quota::per_second);
        let limit_per_min =
            get_limit!("RATELIMITER_QUOTA_PER_MIN").map(Quota::per_minute);
        let limit_per_hour =
            get_limit!("RATELIMITER_QUOTA_PER_HOUR").map(Quota::per_hour);

        let limit_burst = get_limit!("RATELIMITER_QUOTA_BURST");

        let quota = if let Some(q) = limit_per_sec {
            q
        } else if let Some(q) = limit_per_min {
            q
        } else if let Some(q) = limit_per_hour {
            q
        } else {
            Quota::per_minute(NonZeroU32::new(120).unwrap())
        };

        governor::RateLimiter::keyed(
            limit_burst.map(|v| quota.allow_burst(v)).unwrap_or(quota),
        )
        .with_middleware()
    });

    if let Some(ip) = req.header("CF-Connecting-IP") {
        let snapshot = match limiter.check_key(&String::from(ip)) {
            Ok(v) => v,
            Err(detail) => {
                let res = Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .body(detail.to_string())
                    .into_response();

                return Ok(res);
            },
        };

        next.call(req).await.map(|v| {
            let mut res = v.into_response();

            let headers = res.headers_mut();
            headers.insert(
                "ratelimit-burst-remaining",
                snapshot
                    .remaining_burst_capacity()
                    .to_string()
                    .parse()
                    .unwrap(),
            );
            headers.insert(
                "ratelimit-burst-replenished-in",
                snapshot
                    .quota()
                    .burst_size_replenished_in()
                    .as_secs_f32()
                    .to_string()
                    .parse()
                    .unwrap(),
            );

            res
        })
    } else {
        next.call(req).await.map(|v| v.into_response())
    }
}

async fn log<E: Endpoint>(next: E, req: Request) -> poem::Result<Response> {
    let method = req.method().clone();
    let path = req.uri().clone();

    let start = Instant::now();
    let res = next.call(req).await;
    let elapsed = start.elapsed();

    let mut resp = match res {
        Ok(r) => r.into_response(),
        Err(e) => e.into_response(),
    };

    if resp.status().as_u16() >= 500 {
        let body = resp.take_body();
        error!(
            "{} -> {} {} [ {:?} ] - {:?}",
            method.as_str(),
            resp.status().as_u16(),
            resp.status().canonical_reason().unwrap_or(""),
            elapsed,
            path.path(),
        );
        error!(
            "^^^ Continued from above -> {:?}",
            body.into_bytes().await.ok()
        );

        resp.set_body("An internal server error has occurred.");
    } else {
        info!(
            "{} -> {} {} [ {:?} ] - {:?}",
            method.as_str(),
            resp.status().as_u16(),
            resp.status().canonical_reason().unwrap_or(""),
            elapsed,
            path.path(),
        );
    }

    Ok(resp)
}
