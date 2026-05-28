use base64::Engine;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;
use serde::Deserialize;

use crate::fetcher::FetchError;

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

    pub async fn fetch(&self, url: &str) -> Result<String, FetchError> {
        match self.fetch_once(url).await {
            Ok(body) => Ok(body),
            Err(e) => {
                if let FetchError::HttpError { status, .. } = &e {
                    if *status >= 500 {
                        tracing::warn!(url, "5xx from Zyte API, retrying");
                        sleep(Duration::from_secs(2)).await;
                        return self.fetch_once(url).await.map_err(|e2| {
                            tracing::warn!(url, error = %e2, "skip after retry");
                            e2
                        });
                    }
                }
                Err(e)
            }
        }
    }

    async fn fetch_once(&self, url: &str) -> Result<String, FetchError> {
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
            .await
            .map_err(|e| FetchError::NetworkError {
                error: e.to_string(),
                url: url.to_string(),
            })?;

        let status = resp.status();

        if status.is_success() {
            let zyte_resp: ZyteResponse = resp.json().await.map_err(|e| FetchError::ParseError {
                error: e.to_string(),
                url: url.to_string(),
            })?;

            if let Some(encoded) = zyte_resp.http_response_body {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(&encoded)
                    .map_err(|e| FetchError::ParseError {
                        error: e.to_string(),
                        url: url.to_string(),
                    })?;
                Ok(String::from_utf8_lossy(&bytes).into_owned())
            } else if let Some(html) = zyte_resp.browser_html {
                Ok(html)
            } else {
                Err(FetchError::ParseError {
                    error: "empty response body from Zyte API".to_string(),
                    url: url.to_string(),
                })
            }
        } else {
            let code = status.as_u16();
            tracing::warn!(url, status = code, "Zyte API returned non-2xx");
            Err(FetchError::HttpError {
                status: code,
                url: url.to_string(),
            })
        }
    }
}
