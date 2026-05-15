import { Hono } from 'hono';
import { cors } from 'hono/cors';
import type { Env, SearchRequest, SearchResult, SiteResults } from './types';
import { generateQueryPlan } from './llm';
import {
  searchSearxng,
  searchDuckduckgo,
  searchHitomi,
  searchKemono,
  searchMomonga,
} from './search';

const app = new Hono<{ Bindings: Env }>();

app.use('*', cors());

app.get('/api/health', async (c) => {
  const homeUrl = c.env.HOME_SERVER_URL;
  let ollama = false;

  if (homeUrl) {
    try {
      const resp = await fetch(`${homeUrl}/api/tags`);
      ollama = resp.ok;
    } catch {}
  }

  return c.json({
    status: ollama ? 'ok' : 'degraded',
    ollama,
    worker: true,
  });
});

app.post('/api/search', async (c) => {
  const body = await c.req.json<SearchRequest>();
  const query = body.query?.trim();
  if (!query) {
    return c.json({ error: 'query is required' }, 400);
  }

  const maxResults = body.max_results || parseInt(c.env.MAX_RESULTS_PER_SITE) || 20;

  const plan = await generateQueryPlan(c.env, query);

  const siteQueries = plan.site_queries;
  const tasks: Promise<SiteResults>[] = [];

  for (const [site, queries] of Object.entries(siteQueries)) {
    if (!queries || queries.length === 0) continue;

    tasks.push(
      (async (): Promise<SiteResults> => {
        let results: SearchResult[];
        switch (site) {
          case 'searxng':
            results = await searchSearxng(c.env.SEARXNG_URL, queries, maxResults);
            break;
          case 'duckduckgo':
            results = await searchDuckduckgo(queries, maxResults);
            break;
          case 'hitomi':
            results = await searchHitomi(queries, maxResults);
            break;
          case 'kemono':
            results = await searchKemono(queries, maxResults);
            break;
          case 'momonga':
            results = await searchMomonga(queries, maxResults);
            break;
          case 'twitter':
            results = await searchDuckduckgo(
              queries.map((q) => `site:twitter.com OR site:x.com ${q}`),
              maxResults,
            );
            break;
          default:
            results = [];
        }

        return { site, results };
      })(),
    );
  }

  const allResults = await Promise.all(tasks);
  const deduped = deduplicateResults(allResults);

  return c.json({
    query,
    query_plan: plan,
    all_results: deduped,
  });
});

function deduplicateResults(allResults: SiteResults[]): SiteResults[] {
  const seen = new Set<string>();

  for (const sr of allResults) {
    sr.results = sr.results.filter((r) => {
      const key = normalizeUrl(r.url);
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  }

  return allResults.sort((a, b) => b.results.length - a.results.length);
}

function normalizeUrl(url: string): string {
  return url
    .trim()
    .replace(/\/$/, '')
    .replace(/^https?:\/\//, '')
    .replace(/^www\./, '')
    .toLowerCase();
}

export default app;