use clap::Parser;
use tracing_subscriber::filter::LevelFilter;

pub(crate) mod models;
pub(crate) mod search;


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
}


#[tokio::main]
async fn main() {
    println!("Hello, world!");
}
