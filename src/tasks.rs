use std::collections::HashMap;
use std::time::Duration;

use tokio::time::interval;

pub fn start_vote_update_tasks() {
    tokio::spawn(check_votes_loop());
}

async fn check_votes_loop() {
    let mut interval = interval(Duration::from_secs(300));

    loop {
        interval.tick().await;

        if let Err(e) = crate::models::bots::refresh_latest_votes().await {
            error!("Failed to update bot votes due to error: {}", e);
        }

        if let Err(e) = crate::models::packs::refresh_latest_votes().await {
            error!("Failed to update pack votes due to error: {}", e);
        }
    }
}

pub fn start_live_data_tasks(a7s_uri: String, a7s_auth: String) {
    tokio::spawn(refresh_trending_scores(a7s_uri, a7s_auth));
    tokio::spawn(refresh_live_data_loop());
}

async fn refresh_live_data_loop() {
    let mut interval = interval(Duration::from_secs(1200));

    loop {
        interval.tick().await;

        if let Err(e) = crate::models::bots::refresh_latest_data().await {
            error!("Failed to update bot data due to error: {}", e);
        }

        if let Err(e) = crate::models::packs::refresh_latest_data().await {
            error!("Failed to update pack data due to error: {}", e);
        }
    }
}

async fn refresh_trending_scores(a7s_uri: String, a7s_auth: String) {
    let mut interval = interval(Duration::from_secs(300));

    loop {
        interval.tick().await;

        if let Err(e) = fetch_pack_trending(&a7s_uri, &a7s_auth).await {
            error!("Failed to update pack trending data due to error: {}", e);
        }

        if let Err(e) = fetch_bot_trending(&a7s_uri, &a7s_auth).await {
            error!("Failed to update bot trending data due to error: {}", e);
        }

        info!("Refreshed trending scores for entities!");
    }
}

async fn fetch_bot_trending(a7s_uri: &str, a7s_auth: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let data = client
        .get(format!("{}/bots/trending-scores", a7s_uri))
        .bearer_auth(a7s_auth)
        .send()
        .await?
        .json::<HashMap<i64, String>>()
        .await?;

    let data = data
        .into_iter()
        .filter_map(|(k, v)| Some((k, v.parse::<f64>().ok()?)))
        .collect();

    crate::models::bots::set_bot_trending_data(data);

    Ok(())
}

async fn fetch_pack_trending(a7s_uri: &str, a7s_auth: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let data = client
        .get(format!("{}/packs/trending-scores", a7s_uri))
        .bearer_auth(a7s_auth)
        .send()
        .await?
        .json::<HashMap<i64, String>>()
        .await?;

    let data = data
        .into_iter()
        .filter_map(|(k, v)| Some((k, v.parse::<f64>().ok()?)))
        .collect();

    crate::models::packs::set_pack_trending_data(data);
    Ok(())
}
