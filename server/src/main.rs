mod config;
mod models;
mod routes;
mod search;
mod llm;
mod extract;

use std::sync::Arc;
use tower_http::cors::CorsLayer;

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

    let app = routes::router()
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:3001";
    tracing::info!("okazu-finder server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}