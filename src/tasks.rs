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

pub fn start_live_data_tasks() {
    tokio::task::spawn_local(refresh_live_data_loop());
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
