use ferrous::{Collector, Emulation};
use serde_json::json;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    let count = Arc::new(Mutex::new(0u32));
    let count_clone = Arc::clone(&count);

    Collector::new()
        .direct_with_emulation(Emulation::Chrome137)
        .concurrency(8)
        .output("books_direct.jsonl")
        .visit("https://books.toscrape.com/catalogue/page-1.html")
        .on_html("article.product_pod", move |el, ctx| {
            let title = el.select_text("h3 a");
            let price = el.select_text("p.price_color");
            if let (Some(t), Some(p)) = (title, price) {
                ctx.push_item(json!({ "title": t, "price": p }));
                let mut c = count_clone.lock().unwrap();
                *c += 1;
                if *c % 20 == 0 {
                    println!("scraped {} books", *c);
                }
            }
        })
        .on_html("li.next a", |el, ctx| {
            if let Some(href) = el.attr("href") {
                let next = format!("https://books.toscrape.com/catalogue/{href}");
                ctx.visit(&next);
            }
        })
        .run()
        .await;

    println!("done — total books: {}", count.lock().unwrap());
}
