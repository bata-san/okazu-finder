addEventListener('fetch', (event) => {
  event.respondWith(handleRequest(event.request));
});

async function handleRequest(request) {
  const html = await fetch('https://lite.duckduckgo.com/lite/?q=frieren', {
    headers: { 'User-Agent': 'Mozilla/5.0' }
  }).then(r => r.text());

  const lines = html.split('\n');
  let results = [];
  let currentUrl = '';
  let currentTitle = '';

  for (const line of lines) {
    if (line.includes('class="result-link"')) {
      const hrefMatch = line.match(/href="([^"]*)"/);
      if (hrefMatch) {
        currentUrl = hrefMatch[1];
        currentTitle = line.replace(/<[^>]*>/g, '').trim();
      }
    } else if (currentUrl && line.includes('class="result-snippet"')) {
      const snippet = line.replace(/<[^>]*>/g, '').trim();
      results.push({ url: currentUrl, title: currentTitle, snippet });
      currentUrl = '';
      if (results.length >= 3) break;
    }
  }

  return new Response(JSON.stringify(results, null, 2), {
    headers: { 'Content-Type': 'application/json' }
  });
}