use crate::models::SearchResult;

pub async fn search_searxng(
    client: &reqwest::Client,
    searxng_url: &str,
    queries: &[String],
    max_results: usize,
) -> anyhow::Result<Vec<SearchResult>> {
    let mut all_results = Vec::new();

    for query in queries {
        let url = format!(
            "{}/search?q={}&format=json&safesearch=0&categories=general,images",
            searxng_url,
            urlencoding(query)
        );

        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            tracing::warn!("SearXNG returned {} for query: {}", resp.status(), query);
            continue;
        }

        let json: serde_json::Value = resp.json().await?;
        if let Some(results) = json["results"].as_array() {
            for r in results.iter().take(max_results) {
                all_results.push(SearchResult {
                    title: r["title"].as_str().unwrap_or("").to_string(),
                    url: r["url"].as_str().unwrap_or("").to_string(),
                    snippet: r["content"].as_str().unwrap_or("").to_string(),
                    site: "searxng".into(),
                    thumbnail: r["img_src"].as_str().map(|s| s.to_string()),
                });
            }
        }
    }

    Ok(all_results)
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}