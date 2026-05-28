#[cfg(feature = "wreq")]
mod direct;
mod crawler;
mod element;
mod fetcher;
mod output;
mod zyte;

pub use element::Element;
pub use crawler::CrawlContext;
pub use zyte::ZyteMode;

#[cfg(feature = "wreq")]
pub use wreq_util::Emulation;

use crawler::{Callback, RegisteredCallback};
use fetcher::Fetcher;
use output::OutputWriter;
use scraper::Selector;
use std::sync::Arc;
use zyte::ZyteClient;

#[cfg(feature = "wreq")]
use direct::DirectClient;

enum ClientConfig {
    Zyte { api_key: String, mode: ZyteMode },
    #[cfg(feature = "wreq")]
    Direct { emulation: wreq_util::Emulation },
}

pub struct Collector {
    client_config: Option<ClientConfig>,
    concurrency: usize,
    callbacks: Vec<RegisteredCallback>,
    output_path: String,
    start_urls: Vec<String>,
}

impl Collector {
    pub fn new() -> Self {
        Self {
            client_config: None,
            concurrency: 4,
            callbacks: Vec::new(),
            output_path: "output.jsonl".to_string(),
            start_urls: Vec::new(),
        }
    }

    pub fn zyte_api_key(mut self, key: &str) -> Self {
        self.client_config = Some(ClientConfig::Zyte {
            api_key: key.to_string(),
            mode: ZyteMode::Http,
        });
        self
    }

    pub fn zyte_mode(mut self, mode: ZyteMode) -> Self {
        if let Some(ClientConfig::Zyte { mode: ref mut m, .. }) = self.client_config {
            *m = mode;
        }
        self
    }

    #[cfg(feature = "wreq")]
    pub fn direct(mut self) -> Self {
        self.client_config = Some(ClientConfig::Direct {
            emulation: wreq_util::Emulation::Chrome137,
        });
        self
    }

    #[cfg(feature = "wreq")]
    pub fn direct_with_emulation(mut self, emulation: wreq_util::Emulation) -> Self {
        self.client_config = Some(ClientConfig::Direct { emulation });
        self
    }

    pub fn concurrency(mut self, n: usize) -> Self {
        self.concurrency = n;
        self
    }

    pub fn on_html<F>(mut self, selector: &str, callback: F) -> Self
    where
        F: Fn(&Element, &mut CrawlContext) + Send + Sync + 'static,
    {
        let sel = Selector::parse(selector)
            .unwrap_or_else(|_| panic!("invalid CSS selector: {selector}"));
        self.callbacks.push(RegisteredCallback {
            selector: sel,
            callback: Box::new(callback) as Callback,
        });
        self
    }

    pub fn output(mut self, path: &str) -> Self {
        self.output_path = path.to_string();
        self
    }

    pub fn visit(mut self, url: &str) -> Self {
        self.start_urls.push(url.to_string());
        self
    }

    pub async fn run(self) {
        let fetcher = match self.client_config.expect("no client configured: call zyte_api_key() or direct()") {
            ClientConfig::Zyte { api_key, mode } => {
                Fetcher::Zyte(ZyteClient::new(&api_key, mode))
            }
            #[cfg(feature = "wreq")]
            ClientConfig::Direct { emulation } => {
                Fetcher::Direct(DirectClient::new(emulation))
            }
        };

        let output = Arc::new(
            OutputWriter::new(&self.output_path)
                .await
                .expect("failed to open output file"),
        );

        let fetcher = Arc::new(fetcher);
        let callbacks = Arc::new(self.callbacks);

        crawler::run(
            self.start_urls,
            callbacks,
            fetcher,
            output,
            self.concurrency,
        )
        .await;
    }
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}
