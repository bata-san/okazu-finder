export interface SearchRequest {
  query: string;
  max_results?: number;
  content_types?: ContentType[];
}

export type ContentType = 'manga' | 'cg' | 'video' | 'illustration' | 'other';

export interface SearchResult {
  title: string;
  url: string;
  snippet: string;
  site: string;
  thumbnail: string | null;
  content_type: ContentType;
  author: string | null;
  media_urls: string[];
}

export interface QueryPlan {
  original_query: string;
  searxng_queries: string[];
}

export interface ClassifiedResults {
  manga: SearchResult[];
  cg: SearchResult[];
  video: SearchResult[];
  illustration: SearchResult[];
  other: SearchResult[];
}

export interface SearchResponse {
  query: string;
  query_plan: QueryPlan;
  classified: ClassifiedResults;
}

export interface HealthResponse {
  status: string;
  ollama: boolean;
  worker?: boolean;
  searxng?: boolean;
  fxtwitter?: boolean;
}

export const CONTENT_TYPE_LABELS: Record<ContentType, string> = {
  manga: '漫画',
  cg: 'CG集',
  video: '動画',
  illustration: 'イラスト',
  other: 'その他',
};