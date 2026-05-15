use crate::models::SearchResult;
use scraper::{Html, Selector};

pub async fn search_twitter(
    client: &reqwest::Client,
    nitter_url: &str,
    queries: &[String],
    max_results: usize,
) -> anyhow::Result<Vec<SearchResult>> {
    let mut all_results = Vec::new();

    for query in queries {
        let url = format!(
            "{}/search?f=tweets&q={}",
            nitter_url.trim_end_matches('/'),
            urlencoding(query)
        );

        let resp = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; rv:120.0) Gecko/20100101 Firefox/120.0")
            .send()
            .await?;

        if !resp.status().is_success() {
            tracing::warn!("Nitter returned {} for query: {}", resp.status(), query);
            continue;
        }

        let html = resp.text().await?;
        let document = Html::parse_document(&html);

        let item_selector = Selector::parse(".timeline-item").unwrap();
        let content_selector = Selector::parse(".tweet-content").unwrap();
        let link_selector = Selector::parse(".tweet-link").unwrap();
        let img_selector = Selector::parse(".attachment.image img, .still-image img").unwrap();

        let mut count = 0;
        for item in document.select(&item_selector) {
            if count >= max_results {
                break;
            }

            let content = item
                .select(&content_selector)
                .next()
                .map(|c| c.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let tweet_url = item
                .select(&link_selector)
                .next()
                .and_then(|a| a.value().attr("href"))
                .map(|href| {
                    if href.starts_with('/') {
                        format!("{}{}", nitter_url.trim_end_matches('/'), href)
                    } else {
                        href.to_string()
                    }
                })
                .unwrap_or_default();

            let thumbnail = item
                .select(&img_selector)
                .next()
                .and_then(|img| img.value().attr("src"))
                .map(|src| {
                    if src.starts_with('/') {
                        format!("{}{}", nitter_url.trim_end_matches('/'), src)
                    } else {
                        src.to_string()
                    }
                });

            if !content.is_empty() || !tweet_url.is_empty() {
                all_results.push(SearchResult {
                    title: truncate(&content, 100),
                    url: tweet_url,
                    snippet: content,
                    site: "twitter".into(),
                    thumbnail,
                });
                count += 1;
            }
        }
    }

    Ok(all_results)
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_len).collect::<String>())
    }
}