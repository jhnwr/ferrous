use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct Stats {
    pub pages_fetched: AtomicUsize,
    pub items_written: AtomicUsize,
    pub fetch_errors: AtomicUsize,
    pub write_errors: AtomicUsize,
    pub status_counts: Mutex<HashMap<u16, usize>>,
    pub start_time: Instant,
}

impl Stats {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            pages_fetched: AtomicUsize::new(0),
            items_written: AtomicUsize::new(0),
            fetch_errors: AtomicUsize::new(0),
            write_errors: AtomicUsize::new(0),
            status_counts: Mutex::new(HashMap::new()),
            start_time: Instant::now(),
        })
    }

    pub fn record_status(&self, code: u16) {
        let mut counts = self.status_counts.lock().unwrap();
        *counts.entry(code).or_insert(0) += 1;
    }

    pub fn print_summary(&self) {
        let elapsed = self.start_time.elapsed();
        let secs = elapsed.as_secs_f64();
        let pages = self.pages_fetched.load(Ordering::Relaxed);
        let items = self.items_written.load(Ordering::Relaxed);
        let fetch_errs = self.fetch_errors.load(Ordering::Relaxed);
        let write_errs = self.write_errors.load(Ordering::Relaxed);

        tracing::info!("scrape complete");
        tracing::info!("  duration:       {:.1}s", secs);
        tracing::info!("  pages fetched:  {}", pages);
        tracing::info!("  items written:  {}", items);

        let counts = self.status_counts.lock().unwrap();
        if !counts.is_empty() {
            tracing::info!("  http status codes:");
            let mut sorted: Vec<_> = counts.iter().collect();
            sorted.sort_by_key(|(k, _)| *k);
            for (status, count) in sorted {
                tracing::info!("    {}:  {}", status, count);
            }
        }

        if fetch_errs > 0 || write_errs > 0 {
            tracing::info!("  errors:");
            tracing::info!("    fetch errors:  {}", fetch_errs);
            tracing::info!("    write errors:  {}", write_errs);
        }
    }
}
