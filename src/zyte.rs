use anyhow::{anyhow, Result};
use base64::Engine;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone, Debug)]
pub enum ZyteMode {
    Http,
    Browser,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZyteResponse {
    http_response_body: Option<String>,
    browser_html: Option<String>,
}

pub struct ZyteClient {
    client: Client,
    api_key: String,
    mode: ZyteMode,
}

impl ZyteClient {
    pub fn new(api_key: &str, mode: ZyteMode) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");
        Self {
            client,
            api_key: api_key.to_string(),
            mode,
        }
    }

    pub async fn fetch(&self, url: &str) -> Result<String> {
        match self.fetch_once(url).await {
            Ok(body) => Ok(body),
            Err(e) => {
                // Check if the error is a retryable 5xx
                if e.to_string().contains("5xx") {
                    sleep(Duration::from_secs(2)).await;
                    self.fetch_once(url).await.map_err(|e2| {
                        eprintln!("[ferrous] skip {url} after retry: {e2}");
                        e2
                    })
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn fetch_once(&self, url: &str) -> Result<String> {
        let body = match self.mode {
            ZyteMode::Http => serde_json::json!({
                "url": url,
                "httpResponseBody": true
            }),
            ZyteMode::Browser => serde_json::json!({
                "url": url,
                "browserHtml": true
            }),
        };

        let resp = self
            .client
            .post("https://api.zyte.com/v1/extract")
            .basic_auth(&self.api_key, Some(""))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();

        if status.is_success() {
            let zyte_resp: ZyteResponse = resp.json().await?;
            if let Some(encoded) = zyte_resp.http_response_body {
                let bytes = base64::engine::general_purpose::STANDARD.decode(&encoded)?;
                Ok(String::from_utf8_lossy(&bytes).into_owned())
            } else if let Some(html) = zyte_resp.browser_html {
                Ok(html)
            } else {
                Err(anyhow!("empty response body from Zyte API"))
            }
        } else if status.is_server_error() {
            Err(anyhow!("5xx from Zyte API: {status} for {url}"))
        } else {
            eprintln!("[ferrous] skip {url}: HTTP {status}");
            Err(anyhow!("non-retryable HTTP {status} for {url}"))
        }
    }
}
