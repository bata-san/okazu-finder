import type { QueryPlan, SearchResult, ClassifiedResults, Env } from './types';

const QUERY_DECOMPOSE_PROMPT = `You are a search query optimizer. Given a user's query, generate SearXNG search queries to find relevant content across the web.

Generate queries using site: operators for known platforms:
- site:hitomi.la for doujinshi/manga/artbooks
- site:kemono.su OR site:kemono.party for creator content (Fanbox/Patreon/Boosty)
- site:x.com OR site:twitter.com for illustrations/fanart
- site:momon-ga.com for manga/image galleries
- site:pixiv.net for illustrations
- site:fanbox.cc for creator content
- site:patreon.com for creator content
- site:fantia.jp for creator content
- site:deviantart.com for artwork
- site:skeb.jp for commissions
- Use general queries (no site:) for broader discovery

Context about content types by site:
- hitomi.la: primarily manga/doujinshi, some illustration artbooks
- kemono.su: primarily illustrations from Fanbox/Patreon
- x.com/twitter.com: primarily illustrations
- momon-ga.com: primarily manga
- pixiv.net: illustrations and manga

Rules:
1. Generate 4-8 queries total, mixing site-specific and general queries
2. Use both Japanese and English/romaji keywords
3. Target different content types
4. Be specific but cover variations

Return ONLY valid JSON array of strings, no markdown, no explanation:
["query1", "query2", "query3"]`;

const CLASSIFY_PROMPT = `You are a content classifier. Classify each search result into exactly one category.

Categories:
- "manga": Doujinshi, manga, comics, artbooks, manga collections
- "cg": CG collections, 3D renders, digital art sets, game rips
- "video": Animations, videos, motion content, MP4/WEBM
- "illustration": Single illustrations, fanart, image posts, screenshots
- "other": Cannot determine or doesn't fit above

Site biases:
- hitomi.la: Usually "manga", sometimes "illustration"
- kemono.su: Usually "illustration"
- twitter.com / x.com: Usually "illustration"
- momon-ga.com: Usually "manga"
- pixiv.net: "illustration" or "manga"
- youtube.com / nicovideo.jp / bilibili.com: Usually "video"
- fanbox.cc / patreon.com / fantia.jp: Usually "illustration"

Return ONLY valid JSON, no markdown:
{"results":{"0":{"content_type":"manga","author":"Artist"},"1":{"content_type":"illustration","author":null}}}`;

export async function generateQueryPlan(env: Env, query: string): Promise<QueryPlan> {
  const homeUrl = env.HOME_SERVER_URL;
  if (!homeUrl) {
    return { original_query: query, searxng_queries: [query] };
  }

  try {
    const content = await ollamaChat(env, homeUrl, QUERY_DECOMPOSE_PROMPT, query);
    const cleaned = cleanJson(content);
    const queries: string[] = JSON.parse(cleaned);
    return { original_query: query, searxng_queries: queries.length > 0 ? queries : [query] };
  } catch (e) {
    console.error('Query plan error:', e);
    return { original_query: query, searxng_queries: [query] };
  }
}

export async function classifyResults(env: Env, results: SearchResult[]): Promise<ClassifiedResults> {
  const homeUrl = env.HOME_SERVER_URL;
  if (!homeUrl || results.length === 0) {
    return heuristicClassify(results);
  }

  try {
    const items = results.map((r, i) => ({
      id: String(i),
      title: r.title,
      url: r.url,
      snippet: r.snippet,
      site: r.site,
    }));

    const content = await ollamaChat(env, homeUrl, CLASSIFY_PROMPT, JSON.stringify(items));
    const cleaned = cleanJson(content);
    const parsed = JSON.parse(cleaned);
    const resultMap = parsed.results || {};

    const out: ClassifiedResults = { manga: [], cg: [], video: [], illustration: [], other: [] };

    results.forEach((r, i) => {
      const cls = resultMap[String(i)];
      const ct = cls?.content_type || determineTypeBySite(r.site);
      const list = out[ct as keyof ClassifiedResults] || out.other;
      list.push({ ...r, content_type: ct, author: cls?.author || null });
    });

    return out;
  } catch (e) {
    console.error('Classification error:', e);
    return heuristicClassify(results);
  }
}

function heuristicClassify(results: SearchResult[]): ClassifiedResults {
  const out: ClassifiedResults = { manga: [], cg: [], video: [], illustration: [], other: [] };

  for (const r of results) {
    const ct = determineTypeBySite(r.site);
    const list = out[ct] || out.other;
    list.push({ ...r, content_type: ct });
  }

  return out;
}

function determineTypeBySite(site: string): 'manga' | 'cg' | 'video' | 'illustration' | 'other' {
  const mapping: Record<string, 'manga' | 'cg' | 'video' | 'illustration'> = {
    hitomi: 'manga',
    momonga: 'manga',
    kemono: 'illustration',
    fanbox: 'illustration',
    patreon: 'illustration',
    fantia: 'illustration',
    twitter: 'illustration',
    pixiv: 'illustration',
    deviantart: 'illustration',
    skeb: 'illustration',
    skima: 'illustration',
    youtube: 'video',
    nicovideo: 'video',
    bilibili: 'video',
  };
  return mapping[site] || 'other';
}

async function ollamaChat(env: Env, homeUrl: string, systemPrompt: string, userMessage: string): Promise<string> {
  const resp = await fetch(`${homeUrl}/api/chat`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: env.OLLAMA_MODEL,
      messages: [
        { role: 'system', content: systemPrompt },
        { role: 'user', content: userMessage },
      ],
      stream: false,
      format: 'json',
    }),
  });

  if (!resp.ok) throw new Error(`Ollama error ${resp.status}`);
  const data = await resp.json() as { message: { content: string } };
  return data.message.content.trim();
}

function cleanJson(raw: string): string {
  const trimmed = raw.trim();
  const start = trimmed.indexOf(trimmed.startsWith('[') ? '[' : '{');
  const end = trimmed.lastIndexOf(trimmed.startsWith('[') ? ']' : '}');
  if (start >= 0 && end > start) return trimmed.slice(start, end + 1);
  return trimmed;
}