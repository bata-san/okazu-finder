use crate::models::SearchResult;
use crate::models::ContentType;

pub async fn search_searxng(
    client: &reqwest::Client,
    searxng_url: &str,
    queries: &[String],
    max_results: usize,
) -> anyhow::Result<Vec<SearchResult>> {
    let mut all_results = Vec::new();

    for query in queries {
        let url = format!(
            "{}/search?q={}&format=json&safesearch=0&categories=general,images,videos&engines=google,bing,duckduckgo,brave,qwant",
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
                let url_str = r["url"].as_str().unwrap_or("").to_string();
                let site = extract_site_name(&url_str);

                all_results.push(SearchResult {
                    title: r["title"].as_str().unwrap_or("").to_string(),
                    url: url_str,
                    snippet: r["content"].as_str().unwrap_or("").to_string(),
                    site,
                    thumbnail: r["img_src"].as_str().map(|s| s.to_string()),
                    content_type: ContentType::Other,
                    author: None,
                    media_urls: Vec::new(),
                });
            }
        }
    }

    Ok(all_results)
}

fn extract_site_name(url: &str) -> String {
    if url.contains("hitomi.la") { return "hitomi".into(); }
    if url.contains("kemono.su") || url.contains("kemono.party") { return "kemono".into(); }
    if url.contains("momon-ga.com") { return "momonga".into(); }
    if url.contains("x.com") || url.contains("twitter.com") { return "twitter".into(); }
    if url.contains("pixiv.net") { return "pixiv".into(); }
    if url.contains("fanbox.cc") { return "fanbox".into(); }
    if url.contains("patreon.com") { return "patreon".into(); }
    if url.contains("fantia.jp") { return "fantia".into(); }
    if url.contains("deviantart.com") { return "deviantart".into(); }
    if url.contains("discord.com") || url.contains("discordapp.net") { return "discord".into(); }
    if url.contains("reddit.com") { return "reddit".into(); }
    if url.contains("imgur.com") { return "imgur".into(); }
    if url.contains("nicovideo.jp") { return "nicovideo".into(); }
    if url.contains("youtube.com") || url.contains("youtu.be") { return "youtube".into(); }
    if url.contains("bilibili.com") { return "bilibili".into(); }
    if url.contains("skeb.jp") { return "skeb".into(); }
    if url.contains("skima.jp") { return "skima".into(); }
    if let Some(start) = url.find("://") {
        let rest = &url[start + 3..];
        if let Some(end) = rest.find('/') {
            return rest[..end].to_string();
        }
    }
    "unknown".into()
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}