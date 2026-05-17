import type { SearchResult, ContentType } from './types';

function enc(s: string): string {
  return encodeURIComponent(s);
}

export async function searchSearxng(
  searxngUrl: string | undefined,
  queries: string[],
  maxResults: number,
): Promise<SearchResult[]> {
  if (searxngUrl && searxngUrl.trim() !== '') {
    return searchSearxngApi(searxngUrl, queries, maxResults);
  }
  return [];
}

async function searchSearxngApi(
  searxngUrl: string,
  queries: string[],
  maxResults: number,
): Promise<SearchResult[]> {
  const allResults: SearchResult[] = [];
  const seenUrls = new Set<string>();

  for (const q of queries) {
    try {
      const url = `${searxngUrl}/search?q=${enc(q)}&format=json&safesearch=0&categories=general,images,videos&engines=google,bing,duckduckgo,brave,qwant`;
      const resp = await fetch(url);
      if (!resp.ok) continue;

      const data = await resp.json() as { results?: Array<{ title?: string; url?: string; content?: string; img_src?: string }> };

      for (const r of (data.results || []).slice(0, maxResults)) {
        const resultUrl = r.url || '';
        const key = normalizeUrl(resultUrl);
        if (seenUrls.has(key)) continue;
        seenUrls.add(key);

        allResults.push({
          title: r.title || '',
          url: resultUrl,
          snippet: r.content || '',
          site: extractSite(resultUrl),
          thumbnail: r.img_src || null,
          content_type: 'other',
          author: null,
          media_urls: [],
        });
      }
    } catch (e) {
      console.error(`SearXNG error for "${q}":`, e);
    }
  }

  return allResults;
}

async function searchDuckduckgo(
  queries: string[],
  maxResults: number,
): Promise<SearchResult[]> {
  const allResults: SearchResult[] = [];
  const seenUrls = new Set<string>();

  for (const q of queries) {
    try {
      const url = `https://lite.duckduckgo.com/lite/?q=${enc(q)}`;
      const resp = await fetch(url, {
        headers: { 'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; rv:132.0) Gecko/20100101 Firefox/132.0' },
      });
      if (!resp.ok) continue;

      const html = await resp.text();
      const lines = html.split('\n');

      const links: Array<{url: string; title: string}> = [];
      const snippets: string[] = [];

      for (const line of lines) {
        if (line.includes('class="result-link"')) {
          const hrefMatch = line.match(/href="([^"]*)"/);
          if (hrefMatch) {
            links.push({
              url: hrefMatch[1],
              title: line.replace(/<[^>]*>/g, '').trim(),
            });
          }
        } else if (line.includes('class="result-snippet"')) {
          snippets.push(line.replace(/<[^>]*>/g, '').replace(/&[^;]+;/g, ' ').trim());
        }
      }

      for (let i = 0; i < Math.min(links.length, maxResults); i++) {
        const link = links[i];
        const snippet = snippets[i] || '';
        const key = normalizeUrl(link.url);
        if (!seenUrls.has(key)) {
          seenUrls.add(key);
          allResults.push({
            title: link.title || q,
            url: link.url,
            snippet,
            site: extractSite(link.url),
            thumbnail: null,
            content_type: 'other',
            author: null,
            media_urls: [],
          });
        }
      }
    } catch (e) {
      console.error(`DuckDuckGo error for "${q}":`, e);
    }
  }

  return allResults;
}

export async function enrichResults(results: SearchResult[]): Promise<void> {
  const fetches = results.map(async (r) => {
    try {
      const resp = await fetch(r.url, {
        headers: { 'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; rv:132.0) Gecko/20100101 Firefox/132.0' },
        signal: AbortSignal.timeout(8000),
      });
      if (!resp.ok) return;

      const html = await resp.text();
      const meta = extractMeta(html, r.url, r.site);

      if (meta.title && (r.title.length < 10 || !r.title)) r.title = meta.title;
      if (meta.description && (r.snippet.length < 50 || !r.snippet)) r.snippet = meta.description;
      if (meta.author && !r.author) r.author = meta.author;
      if (!r.thumbnail && meta.thumbnails[0]) r.thumbnail = meta.thumbnails[0];
      for (const u of meta.mediaUrls) {
        if (!r.media_urls.includes(u)) r.media_urls.push(u);
      }
      for (const u of meta.thumbnails) {
        if (!r.media_urls.includes(u)) r.media_urls.push(u);
      }
      if (meta.contentTypeHint && r.content_type === 'other') {
        r.content_type = meta.contentTypeHint;
      }
    } catch {
      // page fetch failed, skip enrichment
    }
  });

  await Promise.all(fetches);
}

interface ExtractedMeta {
  title: string | null;
  description: string | null;
  author: string | null;
  thumbnails: string[];
  mediaUrls: string[];
  contentTypeHint: ContentType | null;
}

function extractMeta(html: string, baseUrl: string, site: string): ExtractedMeta {
  const meta: ExtractedMeta = {
    title: null,
    description: null,
    author: null,
    thumbnails: [],
    mediaUrls: [],
    contentTypeHint: null,
  };

  // Site-specific extraction
  if (site === 'hitomi') {
    const galleryMatch = html.match(/<h1[^>]*>([^<]*)<\/h1>/i)
      || html.match(/<div[^>]*class="[^"]*title[^"]*"[^>]*>([^<]*)<\/div>/i);
    if (galleryMatch) meta.title = galleryMatch[1].trim();

    const imgMatches = html.matchAll(/<img[^>]*(?:data-src|src)="([^"]*)"[^>]*>/gi);
    for (const m of imgMatches) {
      meta.mediaUrls.push(resolveUrl(m[1], baseUrl));
    }
    meta.contentTypeHint = 'manga';
  } else if (site === 'momonga') {
    const titleMatch = html.match(/<h1[^>]*>([^<]*)<\/h1>/i)
      || html.match(/<title>([^<]*)<\/title>/i);
    if (titleMatch) meta.title = titleMatch[1].trim();

    const imgMatches = html.matchAll(/<img[^>]*(?:data-src|data-lazy-src|src)="([^"]*)"[^>]*>/gi);
    for (const m of imgMatches) {
      meta.mediaUrls.push(resolveUrl(m[1], baseUrl));
    }
    meta.contentTypeHint = 'manga';
  } else if (site === 'kemono') {
    // Kemono returns JSON through API, already handled
    const imgMatches = html.matchAll(/https:\/\/kemono\.su\/data\/[^"'\s]+/gi);
    for (const m of imgMatches) {
      meta.mediaUrls.push(m[0]);
    }
    meta.contentTypeHint = 'illustration';
  }

  // Generic OG/meta extraction
  const ogTitle = html.match(/<meta[^>]+property="og:title"[^>]+content="([^"]*)"/i)
    || html.match(/<meta[^>]+name="og:title"[^>]+content="([^"]*)"/i);
  if (ogTitle && !meta.title) meta.title = ogTitle[1];

  const ogDesc = html.match(/<meta[^>]+property="og:description"[^>]+content="([^"]*)"/i)
    || html.match(/<meta[^>]+name="og:description"[^>]+content="([^"]*)"/i)
    || html.match(/<meta[^>]+name="description"[^>]+content="([^"]*)"/i);
  if (ogDesc) meta.description = ogDesc[1].slice(0, 500);

  const ogImage = html.match(/<meta[^>]+property="og:image"[^>]+content="([^"]*)"/i)
    || html.match(/<meta[^>]+name="twitter:image"[^>]+content="([^"]*)"/i);
  if (ogImage) meta.thumbnails.push(resolveUrl(ogImage[1], baseUrl));

  const metaAuthor = html.match(/<meta[^>]+name="author"[^>]+content="([^"]*)"/i);
  if (metaAuthor) meta.author = metaAuthor[1];

  // General image extraction
  const imgTagMatches = html.matchAll(/<img[^>]*(?:data-src|src)="([^"]*)"[^>]*>/gi);
  let imgCount = 0;
  for (const m of imgTagMatches) {
    if (imgCount >= 10) break;
    const imgUrl = resolveUrl(m[1], baseUrl);
    if (isImageUrl(imgUrl)) {
      meta.mediaUrls.push(imgUrl);
      imgCount++;
    }
  }

  // Video extraction
  const videoMatches = html.matchAll(/<(?:video|source)[^>]+src="([^"]*)"[^>]*>/gi);
  for (const m of videoMatches) {
    meta.mediaUrls.push(resolveUrl(m[1], baseUrl));
    meta.contentTypeHint = 'video';
  }

  if (meta.contentTypeHint && (meta.mediaUrls.length > 5 || html.includes('gallery') || html.includes('doujin'))) {
    meta.contentTypeHint = 'manga';
  }

  return meta;
}

function resolveUrl(src: string, base: string): string {
  if (!src) return '';
  if (src.startsWith('http://') || src.startsWith('https://')) return src;
  if (src.startsWith('//')) return 'https:' + src;
  if (src.startsWith('/')) {
    try {
      const u = new URL(base);
      return u.origin + src;
    } catch {
      return base + src;
    }
  }
  return base.replace(/\/$/, '') + '/' + src;
}

function isImageUrl(url: string): boolean {
  const l = url.toLowerCase();
  return l.endsWith('.jpg') || l.endsWith('.jpeg') || l.endsWith('.png')
    || l.endsWith('.gif') || l.endsWith('.webp') || l.endsWith('.avif')
    || l.includes('img') || l.includes('image') || l.includes('thumb');
}

export async function resolveFxtwitter(results: SearchResult[]): Promise<void> {
  for (const r of results) {
    if (!isTwitterUrl(r.url)) continue;

    const tweetId = extractTweetId(r.url);
    if (!tweetId) continue;

    try {
      const resp = await fetch(`https://api.fxtwitter.com/status/${tweetId}`);
      if (!resp.ok) continue;

      const json = await resp.json() as {
        tweet?: {
          author?: { name?: string; screen_name?: string };
          text?: string;
          media?: {
            photos?: Array<{ url: string }>;
            videos?: Array<{ url: string }>;
          };
        };
      };

      const tweet = json.tweet;
      if (!tweet) continue;

      r.author = tweet.author?.name || tweet.author?.screen_name || null;
      r.title = tweet.text?.slice(0, 100) || r.title;

      if (tweet.media?.photos) {
        for (const photo of tweet.media.photos) {
          r.media_urls.push(photo.url);
        }
      }
      if (tweet.media?.videos) {
        for (const video of tweet.media.videos) {
          r.media_urls.push(video.url);
          r.content_type = 'video';
        }
      }

      if (r.content_type === 'other' && r.media_urls.length > 0) {
        r.content_type = 'illustration';
      }
      if (r.media_urls[0]) {
        r.thumbnail = r.media_urls[0];
      }
    } catch (e) {
      console.error(`FxTwitter error for ${tweetId}:`, e);
    }
  }
}

function isTwitterUrl(url: string): boolean {
  return url.includes('x.com/') || url.includes('twitter.com/');
}

function extractTweetId(url: string): string {
  const match = url.match(/\/status(?:es)?\/(\d+)/);
  return match?.[1] || '';
}

function extractSite(url: string): string {
  if (url.includes('hitomi.la')) return 'hitomi';
  if (url.includes('kemono.su') || url.includes('kemono.party')) return 'kemono';
  if (url.includes('momon-ga.com')) return 'momonga';
  if (url.includes('x.com') || url.includes('twitter.com')) return 'twitter';
  if (url.includes('pixiv.net')) return 'pixiv';
  if (url.includes('fanbox.cc')) return 'fanbox';
  if (url.includes('patreon.com')) return 'patreon';
  if (url.includes('fantia.jp')) return 'fantia';
  if (url.includes('deviantart.com')) return 'deviantart';
  if (url.includes('youtube.com') || url.includes('youtu.be')) return 'youtube';
  if (url.includes('nicovideo.jp')) return 'nicovideo';
  if (url.includes('bilibili.com')) return 'bilibili';
  if (url.includes('skeb.jp')) return 'skeb';
  if (url.includes('skima.jp')) return 'skima';

  try {
    return new URL(url).hostname;
  } catch {
    return 'unknown';
  }
}

function normalizeUrl(url: string): string {
  return url.trim()
    .replace(/\/$/, '')
    .replace(/^https?:\/\//, '')
    .replace(/^www\./, '')
    .toLowerCase();
}