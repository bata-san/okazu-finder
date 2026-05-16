pub fn extract_kemono(json: &str) -> Option<(String, Vec<String>, Option<String>)> {
    let data: serde_json::Value = serde_json::from_str(json).ok()?;
    let post = data.as_object()?;

    let title = post.get("title")?.as_str().unwrap_or("").to_string();
    let user = post.get("user")?.as_str().unwrap_or("").to_string();
    let service = post.get("service")?.as_str().unwrap_or("").to_string();

    let mut images: Vec<String> = Vec::new();
    if let Some(file) = post.get("file") {
        if let Some(path) = file.get("path").and_then(|p| p.as_str()) {
            images.push(format!("https://kemono.su/data{}", path));
        }
    }
    if let Some(attachments) = post.get("attachments").and_then(|a| a.as_array()) {
        for att in attachments {
            if let Some(path) = att.get("path").and_then(|p| p.as_str()) {
                images.push(format!("https://kemono.su/data{}", path));
            }
        }
    }

    let author = Some(format!("{} ({})", user, service));

    Some((title, images, author))
}