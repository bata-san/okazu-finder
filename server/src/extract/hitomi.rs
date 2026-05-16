use scraper::{Html, Selector};

pub fn extract_hitomi(html: &str) -> Option<(String, Vec<String>, Option<String>)> {
    let document = Html::parse_document(html);

    let title_sel = Selector::parse("h1, .title, .gallery-title").ok()?;
    let img_sel = Selector::parse(".gallery-content img, .img-container img, img.lazyload").ok()?;
    let artist_sel = Selector::parse("a[href*='artist'], .artist-list a, .related-tags a[href*='artist']").ok();

    let title = document
        .select(&title_sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string());

    let mut images: Vec<String> = Vec::new();
    for el in document.select(&img_sel) {
        if let Some(src) = el.value().attr("data-src")
            .or_else(|| el.value().attr("src"))
        {
            let url = if src.starts_with("//") {
                format!("https:{}", src)
            } else if src.starts_with('/') {
                format!("https://hitomi.la{}", src)
            } else {
                src.to_string()
            };
            if !images.contains(&url) {
                images.push(url);
            }
        }
    }

    let artist = artist_sel.and_then(|sel| {
        document
            .select(&sel)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
    });

    Some((title.unwrap_or_default(), images, artist))
}