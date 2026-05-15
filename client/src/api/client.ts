import type { SearchResponse, HealthResponse } from '../types';

function getApiBase(): string {
  if (typeof window !== 'undefined') {
    const stored = localStorage.getItem('okazu_api_url');
    if (stored) return stored;
  }
  return '/api';
}

export async function search(query: string, maxResults?: number): Promise<SearchResponse> {
  const base = getApiBase();
  const res = await fetch(`${base}/api/search`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ query, max_results: maxResults }),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `Search failed: ${res.statusText}`);
  }
  return res.json();
}

export function searchStream(
  query: string,
  maxResults: number,
  onEvent: (event: string, data: unknown) => void,
  onError: (err: Error) => void,
  onDone: () => void,
): AbortController {
  const controller = new AbortController();
  const base = getApiBase();
  const params = new URLSearchParams({ q: query, max: String(maxResults) });
  const url = `${base}/api/search/stream?${params}`;

  fetch(url, { signal: controller.signal })
    .then(async (response) => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const reader = response.body?.getReader();
      if (!reader) throw new Error('No response body');

      const decoder = new TextDecoder();
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          if (line.startsWith('event:')) {
            const eventType = line.slice(6).trim();
            onEvent(eventType, null);
          } else if (line.startsWith('data:')) {
            try {
              const data = JSON.parse(line.slice(5).trim());
              onEvent('data', data);
            } catch {
              onEvent('data', line.slice(5).trim());
            }
          }
        }
      }
      onDone();
    })
    .catch((err) => {
      if (err.name !== 'AbortError') {
        onError(err);
      }
    });

  return controller;
}

export async function checkHealth(apiUrl?: string): Promise<HealthResponse> {
  const base = apiUrl || getApiBase();
  const res = await fetch(`${base}/api/health`);
  return res.json();
}