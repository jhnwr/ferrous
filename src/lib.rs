mod crawler;
mod element;
mod output;
mod zyte;

pub use element::Element;
pub use crawler::CrawlContext;
pub use zyte::ZyteMode;

use crawler::{Callback, RegisteredCallback};
use output::OutputWriter;
use scraper::Selector;
use std::sync::Arc;
use zyte::ZyteClient;

pub struct Collector {
    api_key: Option<String>,
    concurrency: usize,
    callbacks: Vec<RegisteredCallback>,
    output_path: String,
    start_urls: Vec<String>,
    mode: ZyteMode,
}

impl Collector {
    pub fn new() -> Self {
        Self {
            api_key: None,
            concurrency: 4,
            callbacks: Vec::new(),
            output_path: "output.jsonl".to_string(),
            start_urls: Vec::new(),
            mode: ZyteMode::Http,
        }
    }

    pub fn zyte_api_key(mut self, key: &str) -> Self {
        self.api_key = Some(key.to_string());
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

    pub fn zyte_mode(mut self, mode: ZyteMode) -> Self {
        self.mode = mode;
        self
    }

    pub async fn run(self) {
        let api_key = self.api_key.expect("zyte_api_key() is required");

        let output = Arc::new(
            OutputWriter::new(&self.output_path)
                .await
                .expect("failed to open output file"),
        );

        let zyte_client = Arc::new(ZyteClient::new(&api_key, self.mode));
        let callbacks = Arc::new(self.callbacks);

        crawler::run(
            self.start_urls,
            callbacks,
            zyte_client,
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
