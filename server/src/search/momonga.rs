use crate::models::SearchResult;
use scraper::{Html, Selector};

pub async fn search_momonga(
    client: &reqwest::Client,
    queries: &[String],
    max_results: usize,
) -> anyhow::Result<Vec<SearchResult>> {
    let mut all_results = Vec::new();

    for query in queries {
        let url = format!(
            "https://momon-ga.com/?q={}",
            urlencoding(query)
        );

        let resp = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; rv:120.0) Gecko/20100101 Firefox/120.0")
            .send()
            .await?;

        if !resp.status().is_success() {
            tracing::warn!("Momon-ga returned {} for query: {}", resp.status(), query);
            continue;
        }

        let html = resp.text().await?;
        let document = Html::parse_document(&html);

        let item_selector = Selector::parse("a[href*='/view/'], .image-item, .gallery-item").unwrap();
        let img_selector = Selector::parse("img").unwrap();

        let mut count = 0;
        for item in document.select(&item_selector) {
            if count >= max_results {
                break;
            }

            let href = item.value().attr("href").unwrap_or("");
            let full_url = if href.starts_with('/') {
                format!("https://momon-ga.com{}", href)
            } else if href.starts_with("http") {
                href.to_string()
            } else {
                format!("https://momon-ga.com/{}", href)
            };

            let alt_text = item
                .select(&img_selector)
                .next()
                .and_then(|img| img.value().attr("alt"))
                .unwrap_or("")
                .to_string();

            let thumbnail = item
                .select(&img_selector)
                .next()
                .and_then(|img| {
                    img.value()
                        .attr("data-src")
                        .or_else(|| img.value().attr("src"))
                })
                .map(|src| {
                    if src.starts_with('/') {
                        format!("https://momon-ga.com{}", src)
                    } else if src.starts_with("http") {
                        src.to_string()
                    } else {
                        format!("https://momon-ga.com/{}", src)
                    }
                });

            if !full_url.is_empty() {
                all_results.push(SearchResult {
                    title: if alt_text.is_empty() {
                        query.clone()
                    } else {
                        alt_text
                    },
                    url: full_url,
                    snippet: String::new(),
                    site: "momonga".into(),
                    thumbnail,
                });
                count += 1;
            }
        }

        if all_results.iter().filter(|r| r.site == "momonga").count() == 0 {
            if let Some(fallback) = try_fallback_parse(&document, query) {
                all_results.extend(fallback.into_iter().take(max_results));
            }
        }
    }

    Ok(all_results)
}

fn try_fallback_parse(document: &Html, query: &str) -> Option<Vec<SearchResult>> {
    let link_selector = Selector::parse("a").unwrap();
    let mut results = Vec::new();

    for link in document.select(&link_selector) {
        let href = link.value().attr("href").unwrap_or("");
        if href.contains("/view/") || href.contains("/photo/") || href.contains("/image/") {
            let full_url = if href.starts_with('/') {
                format!("https://momon-ga.com{}", href)
            } else {
                href.to_string()
            };

            let text = link.text().collect::<String>().trim().to_string();
            results.push(SearchResult {
                title: if text.is_empty() {
                    query.to_string()
                } else {
                    text
                },
                url: full_url,
                snippet: String::new(),
                site: "momonga".into(),
                thumbnail: None,
            });
        }
    }

    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}