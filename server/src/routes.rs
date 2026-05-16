use axum::{
    extract::State,
    http::StatusCode,
    response::sse::{Event, Sse},
    Json,
};
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::models::*;
use crate::search;
use crate::llm;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<crate::config::Config>,
    pub http_client: reqwest::Client,
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/search", axum::routing::post(search_handler))
        .route("/search/stream", axum::routing::get(search_stream_handler))
        .route("/health", axum::routing::get(health_handler))
}

async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    let ollama = check_ollama(&state.http_client, &state.config.ollama_url).await;
    let searxng = check_searxng(&state.http_client, &state.config.searxng_url).await;
    let fxtwitter = check_fxtwitter(&state.http_client).await;

    Json(HealthResponse {
        status: if ollama && searxng { "ok".into() } else { "degraded".into() },
        ollama,
        searxng,
        fxtwitter,
    })
}

async fn check_ollama(client: &reqwest::Client, url: &str) -> bool {
    client.get(format!("{}/api/tags", url)).send().await.map(|r| r.status().is_success()).unwrap_or(false)
}

async fn check_searxng(client: &reqwest::Client, url: &str) -> bool {
    client.get(url.to_string()).send().await.map(|r| r.status().is_success()).unwrap_or(false)
}

async fn check_fxtwitter(client: &reqwest::Client) -> bool {
    client.get("https://api.fxtwitter.com/status/20").send().await.map(|r| r.status().is_success()).unwrap_or(false)
}

async fn search_handler(
    State(state): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    let max_results = if req.max_results > 0 {
        req.max_results
    } else {
        state.config.max_results_per_site
    };

    tracing::info!("Search: {}", req.query);

    let plan = llm::generate_query_plan(
        &state.http_client,
        &state.config.ollama_url,
        &state.config.ollama_model,
        &req.query,
    )
    .await
    .map_err(|e| {
        tracing::error!("Query plan error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, format!("LLM error: {}", e))
    })?;

    tracing::info!("SearXNG queries: {:?}", plan.searxng_queries);

    let raw_results = search::execute_search(
        &state.http_client,
        &plan,
        &state.config.searxng_url,
        max_results,
    )
    .await;

    tracing::info!("Raw results: {}", raw_results.len());

    let classified = search::classify_and_group(
        &state.http_client,
        &state.config.ollama_url,
        &state.config.ollama_model,
        raw_results,
    )
    .await;

    tracing::info!(
        "Classified: manga={}, cg={}, video={}, illustration={}, other={}",
        classified.manga.len(),
        classified.cg.len(),
        classified.video.len(),
        classified.illustration.len(),
        classified.other.len(),
    );

    Ok(Json(SearchResponse {
        query: req.query,
        query_plan: plan,
        classified,
    }))
}

async fn search_stream_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let query = params.get("q").cloned().unwrap_or_default();
    let max_results = params
        .get("max")
        .and_then(|s| s.parse().ok())
        .unwrap_or(state.config.max_results_per_site);

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let state = state.clone();

    tokio::spawn(async move {
        let plan = match llm::generate_query_plan(
            &state.http_client,
            &state.config.ollama_url,
            &state.config.ollama_model,
            &query,
        ).await {
            Ok(p) => {
                let _ = tx.send(Ok(Event::default().event("plan").json_data(&p).unwrap_or_default()));
                p
            }
            Err(e) => {
                let _ = tx.send(Ok(Event::default().event("error").data(format!("LLM error: {}", e))));
                return;
            }
        };

        let raw_results = search::execute_search(
            &state.http_client,
            &plan,
            &state.config.searxng_url,
            max_results,
        ).await;

        let _ = tx.send(Ok(Event::default().event("results").json_data(&serde_json::json!({
            "count": raw_results.len()
        })).unwrap_or_default()));

        let classified = search::classify_and_group(
            &state.http_client,
            &state.config.ollama_url,
            &state.config.ollama_model,
            raw_results,
        ).await;

        let _ = tx.send(Ok(Event::default().event("classified").json_data(&classified).unwrap_or_default()));
        let _ = tx.send(Ok(Event::default().event("done").json_data(&serde_json::json!({"total": classified.total()})).unwrap_or_default()));
    });

    Sse::new(UnboundedReceiverStream::new(rx))
}