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

// ── クエリ分解プロンプト ──────────────────────────────────
// OKAZU_DECOMPOSE_PROMPT 環境変数で上書き可能
const QUERY_DECOMPOSE_PROMPT: &str = r#"あなたはアダルトコンテンツ専門の検索クエリ最適化AIです。
ユーザーが入力したキーワード（キャラ名・シリーズ名・シチュエーション・フェチなど）を元に、SearXNGで使う最適な検索クエリを生成してください。

検索対象サイト：
- site:hitomi.la → 同人誌・エロ漫画・CG集・アートブック
- site:kemono.su OR site:kemono.party → Fanbox/Patreon/Boosty/Fantiaの有料コンテンツ
- site:x.com OR site:twitter.com → エロイラスト・R18ファンアート
- site:momon-ga.com → エロ漫画・同人誌の転載ギャラリー
- site:pixiv.net → R18イラスト・漫画
- site:fanbox.cc → クリエイターの有料投稿
- site:patreon.com → クリエイターの有料投稿
- site:fantia.jp → クリエイターの有料投稿
- site:deviantart.com → アートワーク
- site:skeb.jp → コミッション作品
- 一般クエリ（site:なし）→ 上記以外のサイトも含めた広域検索

クエリ生成ルール：
1. 合計4〜8個のクエリを生成（site:あり＋一般クエリを混ぜる）
2. 日本語と英語/ローマ字の両方を含める（例："site:hitomi.la フリーレン フェルン R18" "site:hitomi.la frieren fern hentai"）
3. 検索意図に応じて以下のキーワードを適切に追加：
   - 同人誌系: doujin, 同人, R18, 成人向け, hentai
   - イラスト系: illust, イラスト, fanart, R18, NSFW, えっち
   - 動画系: animation, video, 動画, MP4
   - CG集系: CG集, CG collection, 3D, digital
   - シチュエーション・フェチはそのまま英語/日本語で追加
4. キャラ名の表記ゆれを考慮（漢字/カタカナ/ローマ字）
5. safesearchやSFWフィルターを外す指示は不要（SearXNG側でsafesearch=0を指定済）

必ずJSON配列のみを返してください。説明やマークダウンは一切不要です：
["query1", "query2", "query3"]"#;

// ── コンテンツ分類プロンプト ──────────────────────────────
// OKAZU_CLASSIFY_PROMPT 環境変数で上書き可能
const CLASSIFY_PROMPT: &str = r#"あなたはアダルトコンテンツの分類AIです。各検索結果を正確なカテゴリに分類してください。

カテゴリ定義：
- "manga": 同人誌、エロ漫画、成年コミック、漫画形式のCG集、アートブック
  ※ hitomi.laやmomon-ga.comから来たものは基本manga
  ※ ページ数が多く連続した画像があるコンテンツもmanga
- "cg": 3DCG集、デジタルアートセット、ゲームのエロMOD/リッピング、CGイラスト集
  ※ 複数枚のレンダリング画像で構成されるセット
  ※ patreon/fanboxのCGクリエイター作品もここ
- "video": アニメーション、動画、MP4/WEBM/GIFアニメ、同人アニメ
  ※ youtube/nicovideo/bilibiliのURLは原則video
- "illustration": 一枚絵、イラスト、ファンアート、ラクガキ、スクリーンショット
  ※ twitter/pixiv/skeb/kemonoの単品投稿は基本これ
  ※ 画像が1〜3枚程度の投稿もillustration
- "other": 上記のどれにも当てはまらない、または判断不能

サイト別の分類バイアス（優先度高）：
- hitomi.la → ほぼ "manga"（同人誌サイト）。ごく稀にアートブック系で "illustration"
- kemono.su → 基本 "illustration"（Fanbox投稿の転載）。動画投稿なら "video"
- twitter.com / x.com → 基本 "illustration"。動画付きツイートは "video"
- momon-ga.com → ほぼ "manga"（エロ漫画転載サイト）
- pixiv.net → "illustration" または "manga"（ページ数で判断）
- fanbox.cc / patreon.com / fantia.jp → "illustration" または "cg"（CGクリエイターの場合）
- youtube.com / nicovideo.jp / bilibili.com → 必ず "video"
- skeb.jp / skima.jp → "illustration"（コミッション作品）

作者名の抽出：
- URLやタイトル、スニペットから作者名が推測できる場合は author に設定
- kemonoの場合は "username (service)" 形式
- twitterの場合は @handle または表示名
- pixivの場合はユーザー名

入力は検索結果のJSON配列です。以下の形式のJSONオブジェクトのみを返してください：
{"results":{"0":{"content_type":"manga","author":"サークル名"},"1":{"content_type":"illustration","author":"@artist"}}}"#;

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