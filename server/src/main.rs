mod config;
mod models;
mod routes;
mod search;
mod llm;
mod extract;

use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use axum::routing::get_service;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cfg = config::Config::from_env();

    tracing::info!("Ollama URL: {}", cfg.ollama_url);
    tracing::info!("Ollama Model: {}", cfg.ollama_model);
    tracing::info!("SearXNG URL: {}", cfg.searxng_url);

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(cfg.request_timeout))
        .user_agent("okazu-finder/0.1.0")
        .build()?;

    let state = routes::AppState {
        config: Arc::new(cfg),
        http_client,
    };

    let serve_dir = get_service(
        ServeDir::new("../client/dist")
    ).handle_error(|err| async move {
        (
            axum::http::StatusCode::NOT_FOUND,
            format!("File not found: {}", err),
        )
    });

    let app = routes::router()
        .fallback_service(serve_dir)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:3001";
    tracing::info!("okazu-finder server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}