use crate::element::Element;
use crate::fetcher::{FetchError, Fetcher};
use crate::output::OutputWriter;
use crate::stats::Stats;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::{sleep, Duration};

pub type Callback = Box<dyn Fn(&Element, &mut CrawlContext) + Send + Sync + 'static>;

pub struct RegisteredCallback {
    pub selector: Selector,
    pub callback: Callback,
}

/// Per-callback context. Collects visits and items locally; the task
/// flushes them after all callbacks complete.
pub struct CrawlContext {
    current_url: String,
    pub(crate) pending_visits: Vec<String>,
    pub(crate) pending_items: Vec<serde_json::Value>,
}

impl CrawlContext {
    fn new(current_url: String) -> Self {
        Self {
            current_url,
            pending_visits: Vec::new(),
            pending_items: Vec::new(),
        }
    }

    /// The URL of the page currently being processed.
    pub fn url(&self) -> &str {
        &self.current_url
    }

    pub fn visit(&mut self, url: &str) {
        self.pending_visits.push(normalize_url(url));
    }

    pub fn push_item(&mut self, value: serde_json::Value) {
        self.pending_items.push(value);
    }
}

fn normalize_url(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

pub async fn run(
    start_urls: Vec<String>,
    callbacks: Arc<Vec<RegisteredCallback>>,
    fetcher: Arc<Fetcher>,
    output: Arc<OutputWriter>,
    concurrency: usize,
    stats: Arc<Stats>,
) {
    let queue: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
    let seen: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let semaphore = Arc::new(Semaphore::new(concurrency));

    // Seed the queue with start URLs
    {
        let mut q = queue.lock().await;
        let mut s = seen.lock().await;
        for url in &start_urls {
            let norm = normalize_url(url);
            if s.insert(norm.clone()) {
                q.push_back(norm);
            }
        }
    }

    tracing::info!(
        start_urls = start_urls.len(),
        concurrency,
        "crawl started"
    );

    loop {
        let url = {
            let mut q = queue.lock().await;
            q.pop_front()
        };

        match url {
            None => {
                if semaphore.available_permits() == concurrency {
                    // Queue empty and nothing in flight
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
            Some(url) => {
                let permit = semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("semaphore closed");

                let callbacks = Arc::clone(&callbacks);
                let fetcher = Arc::clone(&fetcher);
                let output = Arc::clone(&output);
                let queue = Arc::clone(&queue);
                let seen = Arc::clone(&seen);
                let stats = Arc::clone(&stats);

                tokio::spawn(async move {
                    let _permit = permit; // dropped at end of task

                    let html = match fetcher.fetch(&url).await {
                        Ok(h) => {
                            stats.pages_fetched.fetch_add(1, Ordering::Relaxed);
                            tracing::info!(url, "fetched");
                            h
                        }
                        Err(e) => {
                            match &e {
                                FetchError::HttpError { status, .. } => {
                                    stats.record_status(*status);
                                    tracing::warn!(url, status, "fetch failed: http error");
                                }
                                FetchError::NetworkError { error, .. } => {
                                    stats.fetch_errors.fetch_add(1, Ordering::Relaxed);
                                    tracing::warn!(url, error, "fetch failed: network error");
                                }
                                FetchError::ParseError { error, .. } => {
                                    stats.fetch_errors.fetch_add(1, Ordering::Relaxed);
                                    tracing::warn!(url, error, "fetch failed: parse error");
                                }
                            }
                            return;
                        }
                    };

                    // Parse HTML and run all callbacks synchronously — Html is !Send
                    // so we must drop it before any .await point.
                    let (all_visits, all_items) = {
                        let doc = Html::parse_document(&html);
                        let mut all_visits: Vec<String> = Vec::new();
                        let mut all_items: Vec<serde_json::Value> = Vec::new();

                        for registered in callbacks.iter() {
                            for el_ref in doc.select(&registered.selector) {
                                let element = Element::from_element_ref(el_ref);
                                let mut ctx = CrawlContext::new(url.clone());
                                (registered.callback)(&element, &mut ctx);
                                all_visits.extend(ctx.pending_visits);
                                all_items.extend(ctx.pending_items);
                            }
                        }
                        (all_visits, all_items)
                        // doc dropped here
                    };

                    // Flush visits to queue (dedup via seen set)
                    {
                        let mut seen_guard = seen.lock().await;
                        let mut queue_guard = queue.lock().await;
                        for visit_url in all_visits {
                            if seen_guard.insert(visit_url.clone()) {
                                queue_guard.push_back(visit_url);
                            }
                        }
                    }

                    // Flush items to output
                    for item in all_items {
                        match output.write_item(&item).await {
                            Ok(_) => {
                                stats.items_written.fetch_add(1, Ordering::Relaxed);
                                tracing::debug!(url, "item written");
                            }
                            Err(e) => {
                                stats.write_errors.fetch_add(1, Ordering::Relaxed);
                                tracing::error!(url, error = %e, "failed to write item");
                            }
                        }
                    }
                });
            }
        }
    }

    stats.print_summary();
}
