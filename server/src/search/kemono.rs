use crate::models::SearchResult;

pub async fn search_kemono(
    client: &reqwest::Client,
    queries: &[String],
    max_results: usize,
) -> anyhow::Result<Vec<SearchResult>> {
    let mut all_results = Vec::new();

    for query in queries {
        let url = format!(
            "https://kemono.su/api/v1/search?q={}",
            urlencoding(query)
        );

        let resp = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; rv:120.0) Gecko/20100101 Firefox/120.0")
            .header("Accept", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            tracing::warn!("Kemono returned {} for query: {}", resp.status(), query);
            continue;
        }

        let json: serde_json::Value = resp.json().await?;

        if let Some(results) = json.as_array() {
            for r in results.iter().take(max_results) {
                let service = r["service"].as_str().unwrap_or("");
                let user = r["user"].as_str().unwrap_or("");
                let id = r["id"].as_str().unwrap_or("");
                let title = r["title"].as_str().unwrap_or("");
                let content = r["content"].as_str().unwrap_or("");

                let post_url = format!("https://kemono.su/{}/user/{}/post/{}", service, user, id);

                let thumbnail = r["file"]
                    .as_object()
                    .and_then(|f| f["path"].as_str())
                    .map(|path| {
                        format!("https://kemono.su/thumbnail/{}", path)
                    });

                let snippet: String = if !title.is_empty() {
                    format!("{} - {}", user, title)
                } else {
                    let preview: String = content
                        .chars()
                        .take(200)
                        .collect();
                    format!("{} - {}", user, preview)
                };

                all_results.push(SearchResult {
                    title: if title.is_empty() {
                        format!("{} / {}", service, user)
                    } else {
                        title.to_string()
                    },
                    url: post_url,
                    snippet,
                    site: "kemono".into(),
                    thumbnail,
                });
            }
        }
    }

    Ok(all_results)
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}