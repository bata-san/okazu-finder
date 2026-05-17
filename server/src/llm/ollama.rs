use serde::{Deserialize, Serialize};
use crate::models::{ClassifiedResults, ContentType, QueryPlan, SearchResult};

#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_ctx: Option<u32>,
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    options: Option<OllamaOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keep_alive: Option<String>,
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

// ── クエリ分解プロンプト ──────────────────────────────────
const QUERY_DECOMPOSE_PROMPT: &str = "Generate EXACTLY 2 search queries for adult content. Output a JSON array with exactly 2 strings, like [\"q1\",\"q2\"]. NEVER wrap in an object. The first query must use site:hitomi.la. The second must use site:pixiv.net. Use Japanese and English terms.";

// ── コンテンツ分類プロンプト ──────────────────────────────
const CLASSIFY_PROMPT: &str = "Classify each search result by URL and title.\nCategories: manga (doujin/comic), cg (3D/CG sets), illustration (single image/fanart), video, other.\nRules: hitomi.la/momon-ga=manga. pixiv/twitter/kemono=illustration. youtube/nicovideo/bilibili=video.\nOutput JSON format: {\"results\":{\"0\":{\"content_type\":\"...\",\"author\":\"...\"},...}}";

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
            OllamaMessage { role: "user".into(), content: format!("{}. Query: {}", get_query_decompose_prompt(), query) },
        ],
        stream: false,
        format: Some("json".into()),
        options: Some(OllamaOptions {
            num_predict: Some(768),
            num_ctx: Some(4096),
        }),
        keep_alive: Some("30m".into()),
    };

    let content = ollama_chat(client, ollama_url, &body).await?;
    let cleaned = clean_json(&content);

    let queries: Vec<String> = if let Ok(arr) = serde_json::from_str::<Vec<String>>(&cleaned) {
        arr
    } else if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&cleaned) {
        if let Some(arr) = obj.get("queries").and_then(|v| v.as_array()) {
            arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
        } else if let Some(arr) = obj.get("results").and_then(|v| v.as_array()) {
            arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
        } else {
            vec![query.to_string()]
        }
    } else {
        vec![query.to_string()]
    };

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
    let prompt = format!("{} Results: {}", get_classify_prompt(), items_json);

        let body = OllamaChatRequest {
        model: model.to_string(),
        messages: vec![
            OllamaMessage { role: "user".into(), content: prompt },
        ],
        stream: false,
        format: Some("json".into()),
        options: Some(OllamaOptions {
            num_predict: Some(2048),
            num_ctx: Some(4096),
        }),
        keep_alive: Some("30m".into()),
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