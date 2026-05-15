import type { QueryPlan, Env } from './types';

// Default system prompt for query optimization.
// Customize it by setting the SYSTEM_PROMPT env var in wrangler.toml or dashboard.
const SYSTEM_PROMPT = `You are an expert search query optimizer for finding multimedia content across platforms.

Given a user's description of what they want to find (character name, series, artist, subject, etc.), generate optimized search queries for different platforms.

Platform details:
- twitter: Social media for illustrations/artwork. Use relevant hashtags, names in both Japanese and English. Include terms like "イラスト", "fanart", "illust".
- hitomi: Large doujinshi/manga repository (hitomi.la). Use English/romaji titles, character names, series names, artist names.
- kemono: Creator content aggregator (kemono.su). Search by artist handle, character name, series name.
- momonga: Japanese image gallery (momon-ga.com). Use Japanese keywords, character names, series names, descriptive terms.
- searxng: General web metasearch. Use broad queries to find content on other sites.

Rules:
1. Generate 2-4 queries per platform
2. Mix Japanese and English/romaji keywords where the content is Japanese in origin
3. Be specific but cover common variations
4. Include relevant tags and terms for each platform
5. Do NOT add "SFW" or safety modifiers that would restrict search results

Return ONLY valid JSON, no markdown, no explanation:
{"twitter":["q1","q2"],"hitomi":["q1","q2"],"kemono":["q1","q2"],"momonga":["q1","q2"],"searxng":["q1","q2"]}`;

export async function generateQueryPlan(
  env: Env,
  query: string,
): Promise<QueryPlan> {
  const homeUrl = env.HOME_SERVER_URL;
  if (!homeUrl) {
    return fallbackQueryPlan(query);
  }

  try {
    const resp = await fetch(`${homeUrl}/api/chat`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: env.OLLAMA_MODEL,
        messages: [
          { role: 'system', content: SYSTEM_PROMPT },
          { role: 'user', content: query },
        ],
        stream: false,
        format: 'json',
      }),
    });

    if (!resp.ok) {
      console.error('Ollama error:', resp.status);
      return fallbackQueryPlan(query);
    }

    const data = await resp.json() as { message: { content: string } };
    const content = data.message.content.trim();
    const cleaned = cleanJson(content);

    try {
      const siteQueries = JSON.parse(cleaned) as Record<string, string[]>;
      return { original_query: query, site_queries: siteQueries };
    } catch {
      console.error('Failed to parse LLM response:', content);
      return fallbackQueryPlan(query);
    }
  } catch (e) {
    console.error('LLM fetch error:', e);
    return fallbackQueryPlan(query);
  }
}

function fallbackQueryPlan(query: string): QueryPlan {
  return {
    original_query: query,
    site_queries: {
      searxng: [query],
      duckduckgo: [query],
      hitomi: [query],
      kemono: [query],
      twitter: [query],
      momonga: [query],
    },
  };
}

function cleanJson(raw: string): string {
  const trimmed = raw.trim();
  const start = trimmed.indexOf('{');
  const end = trimmed.lastIndexOf('}');
  if (start >= 0 && end > start) {
    return trimmed.slice(start, end + 1);
  }
  return trimmed;
}