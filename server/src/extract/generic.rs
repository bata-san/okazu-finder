use crate::models::{ContentType, SearchResult};
use scraper::{Html, Selector};

#[derive(Default)]
pub struct ExtractedMeta {
    pub title: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub thumbnails: Vec<String>,
    pub media_urls: Vec<String>,
    pub content_type_hint: Option<ContentType>,
    pub tags: Vec<String>,
}

pub fn extract_generic(html: &str, url: &str) -> ExtractedMeta {
    let document = Html::parse_document(html);
    let mut meta = ExtractedMeta::default();

    let title_sel = Selector::parse("title").ok();
    let og_title = Selector::parse("meta[property='og:title'], meta[name='og:title']").ok();
    let og_desc = Selector::parse("meta[property='og:description'], meta[name='og:description']").ok();
    let og_image = Selector::parse("meta[property='og:image'], meta[name='og:image']").ok();
    let og_type = Selector::parse("meta[property='og:type']").ok();
    let twitter_image = Selector::parse("meta[name='twitter:image'], meta[property='twitter:image']").ok();
    let meta_desc = Selector::parse("meta[name='description']").ok();
    let meta_author = Selector::parse("meta[name='author']").ok();
    let img_sel = Selector::parse("img[src]").ok();
    let video_sel = Selector::parse("video source[src], video[src]").ok();
    let a_sel = Selector::parse("a[href]").ok();

    if let Some(sel) = &title_sel {
        if let Some(el) = document.select(sel).next() {
            meta.title = Some(el.text().collect::<String>().trim().to_string());
        }
    }

    if let Some(sel) = &og_title {
        if let Some(el) = document.select(sel).next() {
            if let Some(content) = el.value().attr("content") {
                meta.title = Some(content.to_string());
            }
        }
    }

    if let Some(sel) = &og_desc {
        if let Some(el) = document.select(sel).next() {
            if let Some(content) = el.value().attr("content") {
                meta.description = Some(content.chars().take(500).collect());
            }
        }
    }

    if let Some(sel) = &meta_desc {
        if let Some(el) = document.select(sel).next() {
            if let Some(content) = el.value().attr("content") {
                if meta.description.is_none() {
                    meta.description = Some(content.chars().take(500).collect());
                }
            }
        }
    }

    if let Some(sel) = &meta_author {
        if let Some(el) = document.select(sel).next() {
            if let Some(content) = el.value().attr("content") {
                meta.author = Some(content.to_string());
            }
        }
    }

    for sel in [&og_image, &twitter_image].iter().filter_map(|s| s.as_ref()) {
        for el in document.select(sel) {
            if let Some(content) = el.value().attr("content") {
                let img_url = normalize_url(content, url);
                if !meta.thumbnails.contains(&img_url) && is_image_url(&img_url) {
                    meta.thumbnails.push(img_url);
                }
            }
        }
    }

    if let Some(sel) = &img_sel {
        for el in document.select(sel).take(20) {
            if let Some(src) = el.value().attr("src") {
                let img_url = normalize_url(src, url);
                if !meta.thumbnails.contains(&img_url) && is_image_url(&img_url) {
                    meta.thumbnails.push(img_url);
                }
            }
            if let Some(src) = el.value().attr("data-src") {
                let img_url = normalize_url(src, url);
                if !meta.thumbnails.contains(&img_url) && is_image_url(&img_url) {
                    meta.thumbnails.push(img_url);
                }
            }
        }
    }

    if let Some(sel) = &video_sel {
        for el in document.select(sel) {
            if let Some(src) = el.value().attr("src") {
                meta.media_urls.push(normalize_url(src, url));
            }
        }
    }

    if let Some(sel) = &og_type {
        if let Some(el) = document.select(sel).next() {
            if let Some(otype) = el.value().attr("content") {
                if otype.contains("video") {
                    meta.content_type_hint = Some(ContentType::Video);
                }
            }
        }
    }

    if let Some(sel) = &a_sel {
        for el in document.select(sel) {
            let text = el.text().collect::<String>();
            let href = el.value().attr("href").unwrap_or("");
            let norm_text = text.to_lowercase();
            if norm_text.contains("tag") || norm_text.contains("カテゴリ") || norm_text.contains("category") {
                if !href.is_empty() && !href.starts_with('#') {
                    meta.tags.push(text.trim().to_string());
                }
            }
        }
    }

    meta
}

fn normalize_url(src: &str, base_url: &str) -> String {
    if src.starts_with("http://") || src.starts_with("https://") {
        return src.to_string();
    }
    if src.starts_with("//") {
        return format!("https:{}", src);
    }
    if src.starts_with('/') {
        let domain = if let Some(after_scheme) = base_url.find("://") {
            let rest = &base_url[after_scheme + 3..];
            if let Some(slash) = rest.find('/') {
                &base_url[..after_scheme + 3 + slash]
            } else {
                base_url
            }
        } else {
            base_url
        };
        return format!("{}{}", domain, src);
    }
    format!("{}/{}", base_url.trim_end_matches('/'), src)
}

fn is_image_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
        || lower.ends_with(".avif")
        || lower.contains("img")
        || lower.contains("image")
        || lower.contains("thumbnail")
        || lower.contains("thumb")
}

pub fn apply_meta_to_result(result: &mut SearchResult, meta: &ExtractedMeta) {
    if let Some(ref t) = meta.title {
        if result.title.is_empty() || result.title.len() < 10 {
            result.title = t.clone();
        }
    }
    if let Some(ref d) = meta.description {
        if result.snippet.is_empty() || result.snippet.len() < 50 {
            result.snippet = d.clone();
        }
    }
    if let Some(ref a) = meta.author {
        if result.author.is_none() {
            result.author = Some(a.clone());
        }
    }
    if !meta.thumbnails.is_empty() {
        if result.thumbnail.is_none() {
            result.thumbnail = meta.thumbnails.first().cloned();
        }
        for url in &meta.thumbnails {
            if !result.media_urls.contains(url) {
                result.media_urls.push(url.clone());
            }
        }
    }
    for url in &meta.media_urls {
        if !result.media_urls.contains(url) {
            result.media_urls.push(url.clone());
        }
    }
    if let Some(ref ct) = meta.content_type_hint {
        if result.content_type == ContentType::Other {
            result.content_type = ct.clone();
        }
    }
}