import { Hono } from 'hono';
import { cors } from 'hono/cors';
import type { Env, SearchRequest } from './types';
import { generateQueryPlan, classifyResults } from './llm';
import { searchSearxng, resolveFxtwitter, enrichResults } from './search';

const app = new Hono<{ Bindings: Env }>();

app.use('*', cors());

app.get('/health', async (c) => {
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

app.post('/search', async (c) => {
  const body = await c.req.json<SearchRequest>();
  const query = body.query?.trim();
  if (!query) {
    return c.json({ error: 'query is required' }, 400);
  }

  const maxResults = body.max_results || parseInt(c.env.MAX_RESULTS_PER_SITE) || 20;

  const plan = await generateQueryPlan(c.env, query);

  const rawResults = await searchSearxng(
    c.env.SEARXNG_URL,
    plan.searxng_queries,
    maxResults,
  );

  await resolveFxtwitter(rawResults);
  await enrichResults(rawResults);

  const classified = await classifyResults(c.env, rawResults);

  return c.json({
    query,
    query_plan: plan,
    classified,
  });
});

export default app;