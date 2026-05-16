use crate::models::SearchResult;
use crate::models::ContentType;

pub async fn resolve_fxtwitter(
    client: &reqwest::Client,
    results: &mut [SearchResult],
) {
    for r in results.iter_mut() {
        if !r.site.contains("twitter") && !r.url.contains("x.com") && !r.url.contains("twitter.com") {
            continue;
        }

        let tweet_id = extract_tweet_id(&r.url);
        if tweet_id.is_empty() {
            continue;
        }

        let api_url = format!("https://api.fxtwitter.com/status/{}", tweet_id);
        match client.get(&api_url).send().await {
            Ok(resp) => {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(tweet) = json["tweet"].as_object() {
                        r.author = tweet["author"]
                            .as_object()
                            .and_then(|a| a["name"].as_str())
                            .or_else(|| tweet["author"].as_object().and_then(|a| a["screen_name"].as_str()))
                            .map(|s| s.to_string());

                        r.title = tweet["text"]
                            .as_str()
                            .map(|t| t.chars().take(100).collect())
                            .unwrap_or_else(|| r.title.clone());

                        if let Some(media) = tweet["media"].as_object() {
                            if let Some(photos) = media["photos"].as_array() {
                                for photo in photos {
                                    if let Some(url) = photo["url"].as_str() {
                                        r.media_urls.push(url.to_string());
                                    }
                                }
                            }
                            if let Some(videos) = media["videos"].as_array() {
                                for video in videos {
                                    if let Some(url) = video["url"].as_str() {
                                        r.media_urls.push(url.to_string());
                                        r.content_type = ContentType::Video;
                                    }
                                }
                            }
                        }

                        if r.content_type == ContentType::Other && !r.media_urls.is_empty() {
                            r.content_type = ContentType::Illustration;
                        }

                        if let Some(photo) = r.media_urls.first() {
                            r.thumbnail = Some(photo.clone());
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("FxTwitter resolve error for {}: {}", tweet_id, e);
            }
        }
    }
}

fn extract_tweet_id(url: &str) -> String {
    if let Some(pos) = url.find("/status/") {
        let rest = &url[pos + 8..];
        let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        return rest[..end].to_string();
    }
    if let Some(pos) = url.find("/statuses/") {
        let rest = &url[pos + 10..];
        let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        return rest[..end].to_string();
    }
    String::new()
}