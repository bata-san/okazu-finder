use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub max_results: usize,
    #[serde(default)]
    pub content_types: Vec<ContentType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Manga,
    Cg,
    Video,
    Illustration,
    Other,
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentType::Manga => write!(f, "manga"),
            ContentType::Cg => write!(f, "cg"),
            ContentType::Video => write!(f, "video"),
            ContentType::Illustration => write!(f, "illustration"),
            ContentType::Other => write!(f, "other"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub site: String,
    pub thumbnail: Option<String>,
    pub content_type: ContentType,
    pub author: Option<String>,
    pub media_urls: Vec<String>,
    pub score: Option<i32>,
    pub source_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub original_query: String,
    pub searxng_queries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedResults {
    pub manga: Vec<SearchResult>,
    pub cg: Vec<SearchResult>,
    pub video: Vec<SearchResult>,
    pub illustration: Vec<SearchResult>,
    pub other: Vec<SearchResult>,
}

impl ClassifiedResults {
    pub fn new() -> Self {
        ClassifiedResults {
            manga: Vec::new(),
            cg: Vec::new(),
            video: Vec::new(),
            illustration: Vec::new(),
            other: Vec::new(),
        }
    }

    pub fn total(&self) -> usize {
        self.manga.len() + self.cg.len() + self.video.len() + self.illustration.len() + self.other.len()
    }

    pub fn all_sorted(self) -> Vec<(ContentType, Vec<SearchResult>)> {
        let mut cats = vec![
            (ContentType::Manga, self.manga),
            (ContentType::Cg, self.cg),
            (ContentType::Video, self.video),
            (ContentType::Illustration, self.illustration),
            (ContentType::Other, self.other),
        ];
        cats.retain(|(_, v)| !v.is_empty());
        cats
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub query_plan: QueryPlan,
    pub classified: ClassifiedResults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub ollama: bool,
    pub searxng: bool,
    pub fxtwitter: bool,
}