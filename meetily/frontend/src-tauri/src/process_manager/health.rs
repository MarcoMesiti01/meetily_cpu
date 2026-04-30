use super::types::{BACKEND_URL, FWS_URL};
use reqwest::Client;
use std::time::Duration;
use tokio::time::{sleep, Instant};

pub async fn is_backend_ready() -> bool {
    let client = match Client::builder().timeout(Duration::from_secs(3)).build() {
        Ok(client) => client,
        Err(_) => return false,
    };

    client
        .get(format!("{}/transcription-config", BACKEND_URL))
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

pub async fn is_faster_whisper_ready() -> bool {
    let client = match Client::builder().timeout(Duration::from_secs(5)).build() {
        Ok(client) => client,
        Err(_) => return false,
    };

    client
        .get(format!(
            "{}/transcription-providers/faster-whisper-server/health",
            BACKEND_URL
        ))
        .query(&[("serverUrl", FWS_URL)])
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

pub async fn wait_until<F, Fut>(timeout: Duration, interval: Duration, mut check: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = Instant::now() + timeout;
    loop {
        if check().await {
            return true;
        }

        if Instant::now() >= deadline {
            return false;
        }

        sleep(interval).await;
    }
}
