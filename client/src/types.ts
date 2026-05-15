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
  worker?: boolean;
  searxng?: boolean;
}

export interface StreamEvent {
  type: 'plan' | 'site_results' | 'done' | 'error';
  data: unknown;
}