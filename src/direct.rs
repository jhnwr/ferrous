use std::time::Duration;
use wreq_util::Emulation;

use crate::fetcher::FetchError;

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

    pub async fn fetch(&self, url: &str) -> Result<String, FetchError> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| FetchError::NetworkError {
                error: e.to_string(),
                url: url.to_string(),
            })?;

        let status = resp.status();

        if status.is_success() {
            resp.text().await.map_err(|e| FetchError::ParseError {
                error: e.to_string(),
                url: url.to_string(),
            })
        } else {
            let code = status.as_u16();
            tracing::warn!(url, status = code, "direct client returned non-2xx");
            Err(FetchError::HttpError {
                status: code,
                url: url.to_string(),
            })
        }
    }
}
