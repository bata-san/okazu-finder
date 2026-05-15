use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_ollama_url")]
    pub ollama_url: String,
    #[serde(default = "default_ollama_model")]
    pub ollama_model: String,
    #[serde(default = "default_searxng_url")]
    pub searxng_url: String,
    #[serde(default = "default_nitter_url")]
    pub nitter_url: String,
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,
    #[serde(default = "default_max_results_per_site")]
    pub max_results_per_site: usize,
}

fn default_ollama_url() -> String {
    "http://localhost:11434".into()
}

fn default_ollama_model() -> String {
    "gemma3:12b".into()
}

fn default_searxng_url() -> String {
    "http://localhost:8080".into()
}

fn default_nitter_url() -> String {
    "https://nitter.net".into()
}

fn default_request_timeout() -> u64 {
    30
}

fn default_max_results_per_site() -> usize {
    20
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            ollama_url: std::env::var("OKAZU_OLLAMA_URL")
                .unwrap_or_else(|_| default_ollama_url()),
            ollama_model: std::env::var("OKAZU_OLLAMA_MODEL")
                .unwrap_or_else(|_| default_ollama_model()),
            searxng_url: std::env::var("OKAZU_SEARXNG_URL")
                .unwrap_or_else(|_| default_searxng_url()),
            nitter_url: std::env::var("OKAZU_NITTER_URL")
                .unwrap_or_else(|_| default_nitter_url()),
            request_timeout: std::env::var("OKAZU_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_request_timeout),
            max_results_per_site: std::env::var("OKAZU_MAX_RESULTS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_max_results_per_site),
        }
    }
}