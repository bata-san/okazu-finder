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