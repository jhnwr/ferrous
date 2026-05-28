use ferrous::Collector;

#[tokio::main]
async fn main() {
    let api_key = std::env::var("ZYTE_API_KEY").expect("ZYTE_API_KEY not set");

    Collector::new()
        .zyte_api_key(&api_key)
        .concurrency(2)
        .on_html("article.product_pod", |el, ctx| {
            let title = el.select_attr("h3 a", "title");
            let price = el.select_text("p.price_color");
            if title.is_some() || price.is_some() {
                ctx.push_item(serde_json::json!({
                    "title": title,
                    "price": price,
                }));
            }
        })
        .on_html("li.next a", |el, ctx| {
            if let Some(href) = el.attr("href") {
                let full = format!("https://books.toscrape.com/catalogue/{href}");
                ctx.visit(&full);
            }
        })
        .output("/tmp/books.jsonl")
        .visit("https://books.toscrape.com/catalogue/page-1.html")
        .run()
        .await;

    let content = std::fs::read_to_string("/tmp/books.jsonl").unwrap_or_default();
    let count = content.lines().count();
    println!("Scraped {count} books → /tmp/books.jsonl");
    if count > 0 {
        println!("First item: {}", content.lines().next().unwrap());
    }
}
