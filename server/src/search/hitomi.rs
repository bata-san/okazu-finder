use crate::models::SearchResult;
use scraper::{Html, Selector};

pub async fn search_hitomi(
    client: &reqwest::Client,
    queries: &[String],
    max_results: usize,
) -> anyhow::Result<Vec<SearchResult>> {
    let mut all_results = Vec::new();

    for query in queries {
        let url = format!(
            "https://hitomi.la/search.html?q={}",
            urlencoding(query)
        );

        let resp = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; rv:120.0) Gecko/20100101 Firefox/120.0")
            .header("Referer", "https://hitomi.la/")
            .send()
            .await?;

        if !resp.status().is_success() {
            tracing::warn!("Hitomi returned {} for query: {}", resp.status(), query);
            continue;
        }

        let html = resp.text().await?;
        let document = Html::parse_document(&html);

        let gallery_selector = Selector::parse(".gallery-content > div > ul > li").unwrap();
        let link_selector = Selector::parse("a").unwrap();
        let img_selector = Selector::parse("img").unwrap();
        let title_selector = Selector::parse(".caption, .title").unwrap();

        let mut count = 0;
        for item in document.select(&gallery_selector) {
            if count >= max_results {
                break;
            }

            let link = item.select(&link_selector).next();
            let href = link.and_then(|a| a.value().attr("href")).unwrap_or("");
            let full_url = if href.starts_with('/') {
                format!("https://hitomi.la{}", href)
            } else {
                href.to_string()
            };

            let title = item
                .select(&title_selector)
                .next()
                .map(|t| t.text().collect::<String>().trim().to_string())
                .unwrap_or_else(|| query.clone());

            let thumbnail = item
                .select(&img_selector)
                .next()
                .and_then(|img| {
                    img.value()
                        .attr("data-src")
                        .or_else(|| img.value().attr("src"))
                })
                .map(|src| {
                    if src.starts_with("//") {
                        format!("https:{}", src)
                    } else if src.starts_with('/') {
                        format!("https://hitomi.la{}", src)
                    } else {
                        src.to_string()
                    }
                });

            if !full_url.is_empty() {
                all_results.push(SearchResult {
                    title,
                    url: full_url,
                    snippet: String::new(),
                    site: "hitomi".into(),
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