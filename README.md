# ferrous

> **Learning project.** Built to explore Rust async patterns and web scraping concepts. Not production-ready.

A minimal Rust scraping framework built on [Zyte API](https://www.zyte.com/zyte-api/). Heavily inspired by [Go Colly](https://go-colly.org/) — register CSS selector callbacks, queue URLs, let the framework handle everything else.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
ferrous = { git = "https://github.com/jhnwr/ferrous" }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
```

## Quick start

```rust
use ferrous::Collector;

#[tokio::main]
async fn main() {
    Collector::new()
        .zyte_api_key("YOUR_ZYTE_API_KEY")
        .concurrency(5)
        .on_html("article.product_pod", |el, ctx| {
            let title = el.select_attr("h3 a", "title");
            let price = el.select_text("p.price_color");
            ctx.push_item(serde_json::json!({ "title": title, "price": price }));
        })
        .on_html("li.next a", |el, ctx| {
            if let Some(href) = el.attr("href") {
                ctx.visit(&format!("https://books.toscrape.com/catalogue/{href}"));
            }
        })
        .output("books.jsonl")
        .visit("https://books.toscrape.com/catalogue/page-1.html")
        .run()
        .await;
}
```

Items are written to `books.jsonl` as newline-delimited JSON, one object per line.

## Direct HTTP (no Zyte API)

Ferrous can also make requests directly via [wreq](https://github.com/0x676e67/wreq), an HTTP client with browser TLS fingerprint emulation. This avoids the Zyte API requirement for sites that don't need it.

Enable the `wreq` feature:

```toml
[dependencies]
ferrous = { git = "https://github.com/jhnwr/ferrous", features = ["wreq"] }
```

Then use `.direct()` or `.direct_with_emulation()` instead of `.zyte_api_key()`:

```rust
use ferrous::{Collector, Emulation};

Collector::new()
    .direct()                                          // Chrome137 by default
    // or: .direct_with_emulation(Emulation::Safari18_5)
    .concurrency(8)
    .on_html("h1", |el, ctx| {
        ctx.push_item(serde_json::json!({ "title": el.text() }));
    })
    .visit("https://example.com")
    .run()
    .await;
```

> **Build prerequisites:** wreq uses BoringSSL which requires `cmake` (`brew install cmake` on macOS, `apt-get install cmake` on Linux).

## API reference

### `Collector` builder

| Method | Description |
|---|---|
| `.zyte_api_key(key: &str)` | Zyte API authentication key. |
| `.zyte_mode(mode: ZyteMode)` | `ZyteMode::Http` (default) or `ZyteMode::Browser`. |
| `.direct()` | Use direct HTTP with Chrome137 emulation. Requires `wreq` feature. |
| `.direct_with_emulation(e: Emulation)` | Use direct HTTP with a specific browser emulation. Requires `wreq` feature. |
| `.concurrency(n: usize)` | Max concurrent requests. Default: `4`. |
| `.on_html(selector: &str, callback)` | Register a callback fired for each element matching `selector`. |
| `.output(path: &str)` | Output file path. Default: `output.jsonl`. |
| `.visit(url: &str)` | Add a start URL. Can be called multiple times. |
| `.run().await` | Start the crawl. Returns when complete. |

### `Element`

Passed as the first argument to every `on_html` callback.

| Method | Returns | Description |
|---|---|---|
| `.text()` | `Option<String>` | Text content of the matched element. |
| `.attr(name: &str)` | `Option<String>` | Attribute value on the matched element. |
| `.select_text(selector: &str)` | `Option<String>` | Text of the first child matching `selector`. |
| `.select_attr(selector: &str, attr: &str)` | `Option<String>` | Attribute of the first child matching `selector`. |

### `CrawlContext`

Passed as the second argument to every `on_html` callback.

| Method | Description |
|---|---|
| `.url() -> &str` | The URL of the page currently being processed. |
| `.visit(url: &str)` | Queue a URL. Silently dropped if already seen. |
| `.push_item(value: serde_json::Value)` | Write an item to the output file immediately. |

## Shared state in callbacks

Callbacks are `Fn + Send + Sync + 'static`. If you need to share mutable state across callbacks, use `Arc<Mutex<T>>`:

```rust
use std::sync::{Arc, Mutex};

let counter = Arc::new(Mutex::new(0u32));

let c = counter.clone();
collector.on_html("div.item", move |el, ctx| {
    let mut n = c.lock().unwrap();
    *n += 1;
    ctx.push_item(serde_json::json!({ "n": *n }));
});
```

## Logging

Ferrous uses [`tracing`](https://docs.rs/tracing) for structured logging. By default, no output is produced — you need to install a subscriber in your project.

Add to your `Cargo.toml`:

```toml
tracing-subscriber = "0.3"
```

Call this before `.run()`:

```rust
tracing_subscriber::fmt::init();
```

Default log level is `INFO`, which shows:
- Crawl start/stop
- Each page fetched
- HTTP errors and retries
- Scrape summary at completion

Use `RUST_LOG=ferrous=debug` for per-item output, or `RUST_LOG=ferrous=warn` to see only errors. Scoping to `ferrous` avoids noisy debug output from underlying dependencies.

### Scrape summary

At the end of every run, ferrous prints a summary:

```
INFO ferrous: scrape complete
INFO ferrous:   duration:       14.3s
INFO ferrous:   pages fetched:  50
INFO ferrous:   items written:  1000
INFO ferrous:   http status codes:
INFO ferrous:     200:  50
INFO ferrous:     404:   2
INFO ferrous:   errors:
INFO ferrous:     fetch errors:  0
INFO ferrous:     write errors:  0
```

The errors section is omitted if there are none.

## Notes

- All HTTP goes through Zyte API — no direct requests to target sites.
- 5xx responses are retried once after 2 seconds. Persistent failures are logged and skipped.
- URL deduplication is per-run and in-memory. No persistent crawl state.

## License

MIT
