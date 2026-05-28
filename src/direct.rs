use anyhow::{anyhow, Result};
use std::time::Duration;
use wreq_util::Emulation;

pub struct DirectClient {
    client: wreq::Client,
}

impl DirectClient {
    pub fn new(emulation: Emulation) -> Self {
        let client = wreq::Client::builder()
            .emulation(emulation)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build wreq client");
        Self { client }
    }

    pub async fn fetch(&self, url: &str) -> Result<String> {
        let resp = self.client.get(url).send().await?;
        let status = resp.status();
        if status.is_success() {
            Ok(resp.text().await?)
        } else if status.is_server_error() {
            Err(anyhow!("5xx from server: {status} for {url}"))
        } else {
            eprintln!("[ferrous] skip {url}: HTTP {status}");
            Err(anyhow!("non-retryable HTTP {status} for {url}"))
        }
    }
}
