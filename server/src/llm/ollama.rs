use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize)]
pub struct OllamaStreamChunk {
    pub message: Option<OllamaMessageContent>,
}

// Customize SYSTEM_PROMPT via OKAZU_SYSTEM_PROMPT env var for your use case.
// The default prompt is a generic multimedia search optimizer.
const DEFAULT_SYSTEM_PROMPT: &str = r#"You are an expert search query optimizer for finding multimedia content across platforms.

Given a user's description of what they want to find (character name, series, artist, subject, etc.), generate optimized search queries for different platforms.

Platform details:
- twitter: Social media for illustrations/artwork. Use relevant hashtags, names in both Japanese and English. Include terms like "イラスト", "fanart", "illust".
- hitomi: Large doujinshi/manga repository (hitomi.la). Use English/romaji titles, character names, series names, artist names. Do NOT translate Japanese names.
- kemono: Creator content aggregator (kemono.su). Search by artist handle, character name, series name.
- momonga: Japanese image gallery (momon-ga.com). Use Japanese keywords, character names, series names, descriptive terms.
- searxng: General web metasearch for remaining sources. Use broad queries to find content on other sites (e.g., "character_name artwork", "artist_name gallery").

Rules:
1. Generate 2-4 queries per platform
2. Mix Japanese and English/romaji keywords where the content is Japanese in origin
3. Be specific but cover common variations (full name + common nicknames)
4. Include relevant tags and terms for each platform
5. Do NOT add "SFW" or safety modifiers that would restrict search results

Return ONLY valid JSON, no markdown, no explanation, no code fences:
{"twitter":["q1","q2"],"hitomi":["q1","q2"],"kemono":["q1","q2"],"momonga":["q1","q2"],"searxng":["q1","q2"]}"#;

fn get_system_prompt() -> String {
    std::env::var("OKAZU_SYSTEM_PROMPT")
        .unwrap_or_else(|_| DEFAULT_SYSTEM_PROMPT.to_string())
}

pub async fn generate_query_plan(
    client: &reqwest::Client,
    ollama_url: &str,
    model: &str,
    query: &str,
) -> anyhow::Result<crate::models::QueryPlan> {
    let body = OllamaChatRequest {
        model: model.to_string(),
        messages: vec![
            OllamaMessage {
                role: "system".into(),
                content: get_system_prompt(),
            },
            OllamaMessage {
                role: "user".into(),
                content: query.to_string(),
            },
        ],
        stream: false,
        format: Some("json".into()),
    };

    let resp = client
        .post(format!("{}/api/chat", ollama_url))
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Ollama error {}: {}", status, text);
    }

    let chat_resp: OllamaChatResponse = resp.json().await?;
    let content = chat_resp.message.content.trim().to_string();

    let cleaned = clean_json_response(&content);

    let site_queries: std::collections::HashMap<String, Vec<String>> =
        serde_json::from_str(&cleaned).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse query plan JSON: {}. Raw response: {}",
                e,
                content
            )
        })?;

    Ok(crate::models::QueryPlan {
        original_query: query.to_string(),
        site_queries,
    })
}

fn clean_json_response(raw: &str) -> String {
    let trimmed = raw.trim();

    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return trimmed[start..=end].to_string();
        }
    }

    trimmed.to_string()
}