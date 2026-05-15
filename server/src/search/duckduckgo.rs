use crate::models::SearchResult;
use scraper::{Html, Selector};

pub async fn search_duckduckgo(
    client: &reqwest::Client,
    queries: &[String],
    max_results: usize,
) -> anyhow::Result<Vec<SearchResult>> {
    let mut all_results = Vec::new();

    for query in queries {
        let url = format!(
            "https://lite.duckduckgo.com/lite/?q={}",
            urlencoding(query)
        );

        let resp = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; rv:120.0) Gecko/20100101 Firefox/120.0")
            .send()
            .await?;

        if !resp.status().is_success() {
            tracing::warn!("DuckDuckGo returned {}", resp.status());
            continue;
        }

        let html = resp.text().await?;
        let document = Html::parse_document(&html);

        let row_selector = Selector::parse("table tr").unwrap();
        let link_selector = Selector::parse("a.result-link").unwrap();
        let snippet_selector = Selector::parse("td.result-snippet").unwrap();

        let mut count = 0;
        for row in document.select(&row_selector) {
            if count >= max_results {
                break;
            }

            let title = row
                .select(&link_selector)
                .next()
                .map(|a| a.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let url = row
                .select(&link_selector)
                .next()
                .and_then(|a| a.value().attr("href"))
                .unwrap_or("")
                .to_string();

            let snippet = row
                .select(&snippet_selector)
                .next()
                .map(|td| td.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if !title.is_empty() && !url.is_empty() {
                all_results.push(SearchResult {
                    title,
                    url,
                    snippet,
                    site: "duckduckgo".into(),
                    thumbnail: None,
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