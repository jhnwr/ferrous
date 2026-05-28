# Ferrous — Design Decisions & Observations

## 1. wreq — Optional Direct HTTP Client
Added an optional `wreq` feature flag for users who don't want to use Zyte API.
Uses `wreq` + `wreq-util` crates for browser emulation (Chrome, Safari, Firefox profiles).
Exposed via `.direct()` and `.direct_with_emulation(Emulation)` builder methods.
`Emulation` re-exported as `ferrous::Emulation`.
Requires cmake at build time (BoringSSL dependency).

## 2. ctx.url() — URL Context in Callbacks
Added `.url() -> &str` to `CrawlContext` so callbacks can access the current page URL.
Useful for constructing absolute URLs from relative hrefs and for item metadata.

## 3. Logging + Scrape Summary
Replacing all `eprintln!` calls with `tracing` crate for structured, async-aware logging.
Users opt in to output by adding `tracing-subscriber` and calling `tracing_subscriber::fmt::init()`.

Introducing a `Stats` struct (`src/stats.rs`) tracking:
- pages fetched
- items written
- fetch errors / write errors
- per HTTP status code counts

Introducing a `FetchError` enum (in `src/fetcher.rs`) to replace `anyhow::Error` on fetch paths:
- `HttpError { status, url }` — non-2xx responses
- `NetworkError { error, url }` — timeout, DNS failures
- `ParseError { error, url }` — base64/utf8 decode failures

Chose `FetchError` enum over passing `Arc<Stats>` into clients directly, to keep the
error type extensible for future additions.

Scrape summary printed at `INFO` level on completion:
- duration, pages fetched, items written
- HTTP status code breakdown
- error counts (section omitted if zero)
