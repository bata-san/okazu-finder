export interface SearchRequest {
  query: string;
  sites?: string[];
  max_results?: number;
}

export interface QueryPlan {
  original_query: string;
  site_queries: Record<string, string[]>;
}

export interface SearchResult {
  title: string;
  url: string;
  snippet: string;
  site: string;
  thumbnail: string | null;
}

export interface SiteResults {
  site: string;
  results: SearchResult[];
}

export interface SearchResponse {
  query: string;
  query_plan: QueryPlan;
  all_results: SiteResults[];
}

export interface HealthResponse {
  status: string;
  ollama: boolean;
  worker: boolean;
}

export interface Env {
  HOME_SERVER_URL?: string;
  SEARXNG_URL?: string;
  OLLAMA_MODEL: string;
  MAX_RESULTS_PER_SITE: string;
}