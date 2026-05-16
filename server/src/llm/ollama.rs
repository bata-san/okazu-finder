use serde::{Deserialize, Serialize};
use crate::models::{ClassifiedResults, ContentType, QueryPlan, SearchResult};

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
pub struct OllamaChatResponse {
    pub message: OllamaMessageContent,
}

#[derive(Debug, Deserialize)]
pub struct OllamaMessageContent {
    pub content: String,
}

const QUERY_DECOMPOSE_PROMPT: &str = r#"You are a search query optimizer. Given a user's query, generate SearXNG search queries to find relevant content across the web.

Generate queries using site: operators for known platforms:
- site:hitomi.la for doujinshi/manga/artbooks
- site:kemono.su OR site:kemono.party for creator content (Fanbox/Patreon/Boosty)
- site:x.com OR site:twitter.com for illustrations/fanart
- site:momon-ga.com for manga/image galleries
- site:pixiv.net for illustrations
- site:fanbox.cc for creator content
- site:patreon.com for creator content
- site:fantia.jp for creator content
- site:deviantart.com for artwork
- site:skeb.jp for commissions
- Use general queries (no site:) for broader discovery

Context about content types by site:
- hitomi.la: primarily manga/doujinshi, some illustration artbooks
- kemono.su: primarily illustrations from Fanbox/Patreon
- x.com/twitter.com: primarily illustrations
- momon-ga.com: primarily manga
- pixiv.net: illustrations and manga

Rules:
1. Generate 4-8 queries total, mixing site-specific and general queries
2. Use both Japanese and English/romaji keywords
3. Target different content types (try to find manga, illustrations, videos)
4. Be specific but cover variations

Return ONLY valid JSON array of strings, no markdown, no explanation:
["query1", "query2", "query3"]"#;

const CLASSIFY_PROMPT: &str = r#"You are a content classifier. Classify each search result into exactly one category.

Categories:
- "manga": Doujinshi, manga, comics, artbooks, manga collections
- "cg": CG collections, 3D renders, digital art sets, game rips
- "video": Animations, videos, motion content, MP4/WEBM
- "illustration": Single illustrations, fanart, image posts, screenshots
- "other": Cannot determine or doesn't fit above

Site biases for classification:
- hitomi.la: Usually "manga" (doujinshi), sometimes "illustration" (artbooks)
- kemono.su: Usually "illustration" (Fanbox posts)
- twitter.com / x.com: Usually "illustration"
- momon-ga.com: Usually "manga"
- pixiv.net: "illustration" or "manga" depending on context
- youtube.com / nicovideo.jp / bilibili.com: Usually "video"
- fanbox.cc / patreon.com / fantia.jp: Usually "illustration"

For each result, also extract an author/artist name if it can be inferred from the title or snippet.

Input is a JSON array of search results. Output must be a JSON object mapping result IDs to their classification:

Return ONLY valid JSON, no markdown, no explanation:
{
  "results": {
    "0": {"content_type": "manga", "author": "Artist Name"},
    "1": {"content_type": "illustration", "author": null}
  }
}"#;

fn get_query_decompose_prompt() -> String {
    std::env::var("OKAZU_DECOMPOSE_PROMPT")
        .unwrap_or_else(|_| QUERY_DECOMPOSE_PROMPT.to_string())
}

fn get_classify_prompt() -> String {
    std::env::var("OKAZU_CLASSIFY_PROMPT")
        .unwrap_or_else(|_| CLASSIFY_PROMPT.to_string())
}

pub async fn generate_query_plan(
    client: &reqwest::Client,
    ollama_url: &str,
    model: &str,
    query: &str,
) -> anyhow::Result<QueryPlan> {
    let body = OllamaChatRequest {
        model: model.to_string(),
        messages: vec![
            OllamaMessage { role: "system".into(), content: get_query_decompose_prompt() },
            OllamaMessage { role: "user".into(), content: query.to_string() },
        ],
        stream: false,
        format: Some("json".into()),
    };

    let content = ollama_chat(client, ollama_url, &body).await?;
    let cleaned = clean_json(&content);

    let queries: Vec<String> = serde_json::from_str(&cleaned)
        .unwrap_or_else(|_| vec![query.to_string()]);

    Ok(QueryPlan {
        original_query: query.to_string(),
        searxng_queries: queries,
    })
}

pub async fn classify_results(
    client: &reqwest::Client,
    ollama_url: &str,
    model: &str,
    results: &[SearchResult],
) -> anyhow::Result<ClassifiedResults> {
    if results.is_empty() {
        return Ok(ClassifiedResults::new());
    }

    let items: Vec<serde_json::Value> = results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            serde_json::json!({
                "id": i.to_string(),
                "title": r.title,
                "url": r.url,
                "snippet": r.snippet,
                "site": r.site,
            })
        })
        .collect();

    let items_json = serde_json::to_string(&items)?;
    let prompt = format!("Classify these search results:\n\n{}", items_json);

    let body = OllamaChatRequest {
        model: model.to_string(),
        messages: vec![
            OllamaMessage { role: "system".into(), content: get_classify_prompt() },
            OllamaMessage { role: "user".into(), content: prompt },
        ],
        stream: false,
        format: Some("json".into()),
    };

    let content = ollama_chat(client, ollama_url, &body).await?;
    let cleaned = clean_json(&content);

    let classification: serde_json::Value = serde_json::from_str(&cleaned)?;
    let result_map = classification["results"].as_object();

    let mut out = ClassifiedResults::new();

    for (i, mut r) in results.iter().cloned().enumerate() {
        let key = i.to_string();
        if let Some(map) = result_map {
            if let Some(cls) = map.get(&key) {
                r.content_type = match cls["content_type"].as_str() {
                    Some("manga") => ContentType::Manga,
                    Some("cg") => ContentType::Cg,
                    Some("video") => ContentType::Video,
                    Some("illustration") => ContentType::Illustration,
                    _ => ContentType::Other,
                };
                r.author = cls["author"].as_str().map(|s| s.to_string());
            }
        }

        let list = match r.content_type {
            ContentType::Manga => &mut out.manga,
            ContentType::Cg => &mut out.cg,
            ContentType::Video => &mut out.video,
            ContentType::Illustration => &mut out.illustration,
            ContentType::Other => &mut out.other,
        };
        list.push(r);
    }

    Ok(out)
}

async fn ollama_chat(
    client: &reqwest::Client,
    ollama_url: &str,
    body: &OllamaChatRequest,
) -> anyhow::Result<String> {
    let resp = client
        .post(format!("{}/api/chat", ollama_url))
        .json(body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Ollama error {}: {}", status, text);
    }

    let chat_resp: OllamaChatResponse = resp.json().await?;
    Ok(chat_resp.message.content.trim().to_string())
}

fn clean_json(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return trimmed[start..=end].to_string();
        }
    }
    if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            return trimmed[start..=end].to_string();
        }
    }
    trimmed.to_string()
}