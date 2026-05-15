# okazu-finder

Multi-platform content discovery powered by local LLM. Searches across Twitter, Hitomi, Kemono, Momon-ga, SearXNG, and DuckDuckGo simultaneously, with AI-generated query optimization.

## Architecture

```
[Browser] ←→ [Cloudflare Worker] ←→ [Home Server: Ollama + SearXNG]
                     │
                     └→ External sites (Hitomi, Kemono, Momon-ga, DDG)
```

- **Rust Backend** (`server/`) — Self-hosted API server for direct access
- **Cloudflare Worker** (`worker/`) — Edge API for production deployment  
- **React Frontend** (`client/`) — Web UI built with Vite + TypeScript

## How It Works

1. Enter a query (character name, series, topic)
2. The local LLM generates optimized search queries per platform
3. All platforms are searched in parallel
4. Results are deduplicated and displayed

## Quick Start

### Prerequisites
- [Ollama](https://ollama.com) with a model (e.g. `ollama pull gemma3:12b`)
- [SearXNG](https://github.com/searxng/searxng) instance (optional, for metasearch)
- [Rust](https://rustup.rs) and [Node.js](https://nodejs.org)

### Self-Hosted

```bash
# Terminal 1 — Rust API server
cd server
cargo run --release

# Terminal 2 — Frontend dev server
cd client
npm install && npm run dev
```

### Cloudflare Worker Deployment

```bash
cd worker
npm install

# Set your home server URL (Ollama)
npx wrangler secret put HOME_SERVER_URL

# Deploy
npx wrangler deploy
```

Set the Worker URL as the API endpoint in the frontend settings (gear icon).

## Configuration

| Env Variable | Default | Description |
|---|---|---|
| `OKAZU_OLLAMA_URL` | `http://localhost:11434` | Ollama API URL |
| `OKAZU_OLLAMA_MODEL` | `gemma3:12b` | Model name |
| `OKAZU_SEARXNG_URL` | `http://localhost:8080` | SearXNG instance URL |
| `OKAZU_SYSTEM_PROMPT` | *(built-in)* | Custom LLM system prompt |
| `HOME_SERVER_URL` | — | (Worker only) Home server URL |

## Project Structure

```
okazu-finder/
├── server/          # Rust (Axum) API server
│   └── src/
│       ├── llm/     # Ollama client + query planner
│       └── search/  # Per-site search modules
├── worker/          # Cloudflare Worker (Hono)
│   └── src/
│       ├── index.ts # API routes
│       ├── llm.ts   # LLM relay
│       └── search.ts# External site fetchers
└── client/          # React + Vite + TypeScript
    └── src/
        ├── components/  # UI components
        └── api/         # API client
```

## License

MIT