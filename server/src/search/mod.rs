pub mod searxng;
pub mod duckduckgo;
pub mod twitter;
pub mod hitomi;
pub mod kemono;
pub mod momonga;

use crate::models::{QueryPlan, SiteResults};
use searxng::search_searxng;
use duckduckgo::search_duckduckgo;
use twitter::search_twitter;
use hitomi::search_hitomi;
use kemono::search_kemono;
use momonga::search_momonga;
use std::collections::HashMap;

pub fn get_enabled_sites(plan: &QueryPlan) -> Vec<&str> {
    plan.site_queries
        .iter()
        .filter(|(_, queries)| !queries.is_empty())
        .map(|(site, _)| site.as_str())
        .collect()
}

pub async fn execute_search(
    client: &reqwest::Client,
    plan: &QueryPlan,
    searxng_url: &str,
    nitter_url: &str,
    max_results: usize,
) -> Vec<SiteResults> {
    let tasks: Vec<_> = plan
        .site_queries
        .iter()
        .filter(|(_, queries)| !queries.is_empty())
        .map(|(site, queries)| {
            let site = site.clone();
            let queries = queries.clone();
            let client = client.clone();
            let searxng_url = searxng_url.to_string();
            let nitter_url = nitter_url.to_string();
            tokio::spawn(async move {
                let results = match site.as_str() {
                    "searxng" => search_searxng(&client, &searxng_url, &queries, max_results).await,
                    "duckduckgo" => search_duckduckgo(&client, &queries, max_results).await,
                    "twitter" => search_twitter(&client, &nitter_url, &queries, max_results).await,
                    "hitomi" => search_hitomi(&client, &queries, max_results).await,
                    "kemono" => search_kemono(&client, &queries, max_results).await,
                    "momonga" => search_momonga(&client, &queries, max_results).await,
                    _ => Ok(Vec::new()),
                };

                match results {
                    Ok(r) => {
                        tracing::info!("[{}] Found {} results", site, r.len());
                        Some(SiteResults {
                            site: site.to_string(),
                            results: r,
                        })
                    }
                    Err(e) => {
                        tracing::error!("[{}] Search error: {}", site, e);
                        Some(SiteResults {
                            site: site.to_string(),
                            results: Vec::new(),
                        })
                    }
                }
            })
        })
        .collect();

    let mut all_results = Vec::new();
    for task in tasks {
        if let Ok(Some(site_results)) = task.await {
            all_results.push(site_results);
        }
    }

    all_results.sort_by_key(|s| -(s.results.len() as i32));
    all_results
}

pub fn deduplicate_results(all_results: &mut [SiteResults]) {
    let mut seen_urls: HashMap<String, bool> = HashMap::new();

    for site_result in all_results.iter_mut() {
        site_result.results.retain(|r| {
            let key = normalize_url(&r.url);
            if seen_urls.contains_key(&key) {
                false
            } else {
                seen_urls.insert(key, true);
                true
            }
        });
    }
}

fn normalize_url(url: &str) -> String {
    let url = url.trim_end_matches('/');
    let url = url.strip_prefix("https://").unwrap_or(url);
    let url = url.strip_prefix("http://").unwrap_or(url);
    let url = url.strip_prefix("www.").unwrap_or(url);
    url.to_lowercase()
}

pub fn extract_content_summary(results: &[SiteResults]) -> String {
    let total: usize = results.iter().map(|s| s.results.len()).sum();
    let mut summary = format!("Found {} results across {} sites.\n\n", total, results.len());

    for site_result in results {
        summary.push_str(&format!("## {} ({} results)\n\n", site_result.site, site_result.results.len()));
        for (i, r) in site_result.results.iter().take(5).enumerate() {
            summary.push_str(&format!(
                "{}. **{}**\n   {}\n   {}\n\n",
                i + 1,
                r.title,
                r.url,
                r.snippet
            ));
        }
    }

    summary
}