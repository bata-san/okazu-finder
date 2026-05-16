use scraper::{Html, Selector};

pub fn extract_momonga(html: &str) -> Option<(String, Vec<String>, Option<String>)> {
    let document = Html::parse_document(html);

    let title_sel = Selector::parse("h1, .entry-title, .post-title, title").ok()?;
    let img_sel = Selector::parse(
        ".entry-content img, .post-content img, .gallery img, img.attachment, img.wp-post-image, .view-image img"
    ).ok()?;
    let author_sel = Selector::parse(".author, .entry-author, .post-author").ok();

    let title = document
        .select(&title_sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string());

    let mut images: Vec<String> = Vec::new();
    for el in document.select(&img_sel) {
        if let Some(src) = el.value().attr("data-src")
            .or_else(|| el.value().attr("data-lazy-src"))
            .or_else(|| el.value().attr("src"))
        {
            let url = if src.starts_with("//") {
                format!("https:{}", src)
            } else if src.starts_with('/') {
                format!("https://momon-ga.com{}", src)
            } else {
                src.to_string()
            };
            if !images.contains(&url) && !url.contains("avatar") {
                images.push(url);
            }
        }
    }

    let author = author_sel.and_then(|sel| {
        document
            .select(&sel)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
    });

    Some((title.unwrap_or_default(), images, author))
}