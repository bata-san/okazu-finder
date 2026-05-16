pub mod searxng;
pub mod fxtwitter;

use crate::llm::classify_results;
use crate::models::{ClassifiedResults, ContentType, QueryPlan, SearchResult};
use searxng::search_searxng;
use fxtwitter::resolve_fxtwitter;
use std::collections::{HashMap, HashSet};

pub async fn execute_search(
    client: &reqwest::Client,
    plan: &QueryPlan,
    searxng_url: &str,
    max_results: usize,
) -> Vec<SearchResult> {
    let mut raw_results = search_searxng(client, searxng_url, &plan.searxng_queries, max_results)
        .await
        .unwrap_or_default();

    deduplicate(&mut raw_results);

    resolve_fxtwitter(client, &mut raw_results).await;

    raw_results
}

pub async fn classify_and_group(
    client: &reqwest::Client,
    ollama_url: &str,
    model: &str,
    results: Vec<SearchResult>,
) -> ClassifiedResults {
    if results.is_empty() {
        return ClassifiedResults::new();
    }

    match classify_results(client, ollama_url, model, &results).await {
        Ok(classified) => classified,
        Err(e) => {
            tracing::error!("Classification error: {}", e);
            heuristic_classify(results)
        }
    }
}

fn heuristic_classify(results: Vec<SearchResult>) -> ClassifiedResults {
    let mut out = ClassifiedResults::new();
    let site_mapping: HashMap<&str, ContentType> = HashMap::from([
        ("hitomi", ContentType::Manga),
        ("momonga", ContentType::Manga),
        ("kemono", ContentType::Illustration),
        ("fanbox", ContentType::Illustration),
        ("patreon", ContentType::Illustration),
        ("fantia", ContentType::Illustration),
        ("twitter", ContentType::Illustration),
        ("pixiv", ContentType::Illustration),
        ("deviantart", ContentType::Illustration),
        ("skeb", ContentType::Illustration),
        ("skima", ContentType::Illustration),
        ("youtube", ContentType::Video),
        ("nicovideo", ContentType::Video),
        ("bilibili", ContentType::Video),
    ]);

    for mut r in results {
        if r.content_type == ContentType::Other {
            r.content_type = site_mapping
                .get(r.site.as_str())
                .cloned()
                .unwrap_or(ContentType::Other);
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

    out
}

fn deduplicate(results: &mut Vec<SearchResult>) {
    let mut seen: HashSet<String> = HashSet::new();
    results.retain(|r| {
        let key = normalize_url(&r.url);
        seen.insert(key)
    });
}

fn normalize_url(url: &str) -> String {
    url.trim()
        .trim_end_matches('/')
        .strip_prefix("https://")
        .unwrap_or_else(|| url.strip_prefix("http://").unwrap_or(url))
        .strip_prefix("www.")
        .unwrap_or(url)
        .to_lowercase()
}