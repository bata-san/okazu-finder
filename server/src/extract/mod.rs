pub mod generic;
pub mod hitomi;
pub mod kemono;
pub mod momonga;

use crate::models::SearchResult;
use generic::{apply_meta_to_result, extract_generic, ExtractedMeta};
use std::time::Duration;

pub async fn enrich_results(
    client: &reqwest::Client,
    results: &mut [SearchResult],
) {
    let extract_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; rv:132.0) Gecko/20100101 Firefox/132.0")
        .build()
        .unwrap_or_else(|_| client.clone());

    let futures: Vec<_> = results.iter_mut().enumerate().map(|(i, r)| {
        let client = extract_client.clone();
        let url = r.url.clone();
        let site = r.site.clone();
        tokio::spawn(async move {
            let html = match client.get(&url).send().await {
                Ok(resp) => resp.text().await.unwrap_or_default(),
                Err(_) => return None,
            };

            let meta = match site.as_str() {
                "hitomi" => extract_site_specific(&html, &site, &url),
                "momonga" => extract_site_specific(&html, &site, &url),
                _ => {
                    let mut m = extract_generic(&html, &url);
                    if site == "kemono" {
                        if let Some((title, images, author)) = kemono::extract_kemono(&html) {
                            m.title = Some(title);
                            m.media_urls = images;
                            m.author = author;
                        }
                    }
                    m
                }
            };

            Some((i, meta))
        })
    }).collect();

    for future in futures {
        if let Ok(Some((idx, meta))) = future.await {
            apply_meta_to_result(&mut results[idx], &meta);
        }
    }
}

fn extract_site_specific(html: &str, site: &str, _url: &str) -> ExtractedMeta {
    let mut meta = ExtractedMeta::default();

    match site {
        "hitomi" => {
            if let Some((title, images, artist)) = hitomi::extract_hitomi(html) {
                meta.title = if title.is_empty() { None } else { Some(title) };
                meta.media_urls = images;
                meta.author = artist;
                meta.content_type_hint = Some(crate::models::ContentType::Manga);
            }
        }
        "momonga" => {
            if let Some((title, images, author)) = momonga::extract_momonga(html) {
                meta.title = if title.is_empty() { None } else { Some(title) };
                meta.media_urls = images;
                meta.author = author;
                meta.content_type_hint = Some(crate::models::ContentType::Manga);
            }
        }
        _ => {
            meta = extract_generic(html, _url);
        }
    }

    meta
}