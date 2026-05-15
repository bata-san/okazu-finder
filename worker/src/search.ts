import type { SearchResult } from './types';

function enc(s: string): string {
  return encodeURIComponent(s);
}

export async function searchSearxng(
  searxngUrl: string | undefined,
  queries: string[],
  maxResults: number,
): Promise<SearchResult[]> {
  if (!searxngUrl) return [];

  const results: SearchResult[] = [];
  for (const q of queries) {
    try {
      const url = `${searxngUrl}/search?q=${enc(q)}&format=json&safesearch=0&categories=general,images`;
      const resp = await fetch(url);
      if (!resp.ok) continue;

      const data = await resp.json() as { results?: Array<{ title?: string; url?: string; content?: string; img_src?: string }> };
      for (const r of (data.results || []).slice(0, maxResults)) {
        results.push({
          title: r.title || '',
          url: r.url || '',
          snippet: r.content || '',
          site: 'searxng',
          thumbnail: r.img_src || null,
        });
      }
    } catch (e) {
      console.error(`SearXNG error for "${q}":`, e);
    }
  }
  return results;
}

export async function searchDuckduckgo(
  queries: string[],
  maxResults: number,
): Promise<SearchResult[]> {
  const results: SearchResult[] = [];
  for (const q of queries) {
    try {
      const url = `https://lite.duckduckgo.com/lite/?q=${enc(q)}`;
      const resp = await fetch(url, {
        headers: { 'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; rv:120.0) Gecko/20100101 Firefox/120.0' },
      });
      if (!resp.ok) continue;

      const html = await resp.text();
      const rowRegex = /<tr[^>]*>[\s\S]*?<a[^>]*class="result-link"[^>]*href="([^"]*)"[^>]*>([\s\S]*?)<\/a>[\s\S]*?<td[^>]*class="result-snippet"[^>]*>([\s\S]*?)<\/td>[\s\S]*?<\/tr>/gi;

      let match;
      let count = 0;
      while ((match = rowRegex.exec(html)) !== null && count < maxResults) {
        const url = match[1];
        const title = match[2].replace(/<[^>]*>/g, '').trim();
        const snippet = match[3].replace(/<[^>]*>/g, '').trim();
        if (title && url) {
          results.push({ title, url, snippet, site: 'duckduckgo', thumbnail: null });
          count++;
        }
      }
    } catch (e) {
      console.error(`DuckDuckGo error for "${q}":`, e);
    }
  }
  return results;
}

export async function searchHitomi(
  queries: string[],
  maxResults: number,
): Promise<SearchResult[]> {
  const results: SearchResult[] = [];
  for (const q of queries) {
    try {
      const url = `https://hitomi.la/search.html?q=${enc(q)}`;
      const resp = await fetch(url, {
        headers: {
          'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; rv:120.0) Gecko/20100101 Firefox/120.0',
          'Referer': 'https://hitomi.la/',
        },
      });
      if (!resp.ok) continue;

      const html = await resp.text();
      const galleryRegex = /<li[^>]*>[\s\S]*?<a[^>]*href="([^"]*)"[^>]*>[\s\S]*?<img[^>]*(?:data-src|src)="([^"]*)"[^>]*>/gi;

      let match;
      let count = 0;
      while ((match = galleryRegex.exec(html)) !== null && count < maxResults) {
        let href = match[1];
        let thumb = match[2];

        if (href.startsWith('/')) href = `https://hitomi.la${href}`;
        if (thumb.startsWith('//')) thumb = `https:${thumb}`;
        else if (thumb.startsWith('/')) thumb = `https://hitomi.la${thumb}`;

        results.push({
          title: q,
          url: href,
          snippet: '',
          site: 'hitomi',
          thumbnail: thumb || null,
        });
        count++;
      }
    } catch (e) {
      console.error(`Hitomi error for "${q}":`, e);
    }
  }
  return results;
}

export async function searchKemono(
  queries: string[],
  maxResults: number,
): Promise<SearchResult[]> {
  const results: SearchResult[] = [];
  for (const q of queries) {
    try {
      const url = `https://kemono.su/api/v1/search?q=${enc(q)}`;
      const resp = await fetch(url, {
        headers: {
          'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; rv:120.0) Gecko/20100101 Firefox/120.0',
          'Accept': 'application/json',
        },
      });
      if (!resp.ok) continue;

      const data = await resp.json() as Array<{
        service?: string;
        user?: string;
        id?: string;
        title?: string;
        content?: string;
        file?: { path?: string };
      }>;

      for (const post of data.slice(0, maxResults)) {
        const postUrl = `https://kemono.su/${post.service || ''}/user/${post.user || ''}/post/${post.id || ''}`;
        results.push({
          title: post.title || `${post.service} / ${post.user}`,
          url: postUrl,
          snippet: post.content?.slice(0, 200) || '',
          site: 'kemono',
          thumbnail: post.file?.path
            ? `https://kemono.su/thumbnail/${post.file.path}`
            : null,
        });
      }
    } catch (e) {
      console.error(`Kemono error for "${q}":`, e);
    }
  }
  return results;
}

export async function searchMomonga(
  queries: string[],
  maxResults: number,
): Promise<SearchResult[]> {
  const results: SearchResult[] = [];
  for (const q of queries) {
    try {
      const url = `https://momon-ga.com/?q=${enc(q)}`;
      const resp = await fetch(url, {
        headers: { 'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; rv:120.0) Gecko/20100101 Firefox/120.0' },
      });
      if (!resp.ok) continue;

      const html = await resp.text();
      const linkRegex = /<a[^>]*href="([^"]*(?:\/view\/|\/photo\/|\/image\/)[^"]*)"[^>]*>([\s\S]*?)<\/a>/gi;
      const imgRegex = /<img[^>]*(?:data-src|src)="([^"]*)"[^>]*>/gi;

      let linkMatch;
      let count = 0;
      while ((linkMatch = linkRegex.exec(html)) !== null && count < maxResults) {
        let href = linkMatch[1];
        const text = linkMatch[2].replace(/<[^>]*>/g, '').trim();

        if (href.startsWith('/')) href = `https://momon-ga.com${href}`;

        const imgSection = html.slice(Math.max(0, linkMatch.index - 200), linkMatch.index + 200);
        imgRegex.lastIndex = 0;
        const imgMatch = imgRegex.exec(imgSection);
        let thumb = imgMatch?.[1] || null;
        if (thumb?.startsWith('/')) thumb = `https://momon-ga.com${thumb}`;

        results.push({
          title: text || q,
          url: href,
          snippet: '',
          site: 'momonga',
          thumbnail: thumb,
        });
        count++;
      }
    } catch (e) {
      console.error(`Momon-ga error for "${q}":`, e);
    }
  }
  return results;
}