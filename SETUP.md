# okazu-finder サーバーセットアップ指示書

## アーキテクチャ概要

```
ブラウザ (https://okazu-finder.pages.dev)
    │
    └──→ Cloudflare Worker (https://okazu-finder-worker.butter3.workers.dev)
              │
              ├──→ SearXNG (search.sandwich-butter.tech:8080) ← Cloudflare Tunnel
              │     ├── Google
              │     ├── Bing
              │     ├── DuckDuckGo
              │     └── Brave
              │
              └──→ Ollama (ollama.sandwich-butter.tech:11434) ← Cloudflare Tunnel
                    └── Gemma 4 E4B Uncensored (gemma4u)
```

---

## 1. Ollama セットアップ

### インストール
```bash
# Linux
curl -fsSL https://ollama.com/install.sh | sh

# Windows
winget install Ollama.Ollama
```

### モデルをプル（Gemma 4 Uncensored - CPU動作対応）

使用モデル: [HauhauCS/Gemma-4-E4B-Uncensored-HauhauCS-Aggressive](https://huggingface.co/HauhauCS/Gemma-4-E4B-Uncensored-HauhauCS-Aggressive)
- 4Bパラメータ（省メモリ）
- GGUFフォーマット、複数量子化レベルあり
- 内蔵GPU/CPU専用でも十分動作
- 0/465 Refusals（完全無規制）
- 日本語・英語マルチリンガル

```bash
# IQ3_M（4.4GB）- 省メモリ + 十分な品質（推奨）
ollama pull hf.co/HauhauCS/Gemma-4-E4B-Uncensored-HauhauCS-Aggressive:IQ3_M

# または Q4_K_M（5.0GB）- 品質重視
ollama pull hf.co/HauhauCS/Gemma-4-E4B-Uncensored-HauhauCS-Aggressive:Q4_K_M
```

プル後、短い名前でエイリアス作成:
```bash
ollama cp "hf.co/HauhauCS/Gemma-4-E4B-Uncensored-HauhauCS-Aggressive:IQ3_M" gemma4u
```

### 動作確認
```bash
ollama run gemma4u "Hello. 日本語で応答してください。"
```
→ 日本語で返ってくればOK

### 必要メモリ
| 量子化 | ファイルサイズ | 必要RAM (推測) |
|---|---|---|
| IQ3_M | 4.4 GB | 8 GB |
| Q4_K_M | 5.0 GB | 10 GB |
| Q5_K_M | 5.4 GB | 12 GB |

※ 内蔵GPU（共有メモリ）の場合はシステムRAMがそのまま使われる。

### ネットワーク設定（重要）
デフォルトでは `127.0.0.1:11434` のみ listen。外部からアクセスするには：

**Linux**:
```bash
sudo systemctl edit ollama.service
```
```
[Service]
Environment=OLLAMA_HOST=0.0.0.0
```
```bash
sudo systemctl daemon-reload
sudo systemctl restart ollama
```

**Windows**: 環境変数 `OLLAMA_HOST=0.0.0.0` を設定して再起動。

### 確認
```bash
curl http://localhost:11434/api/tags
```

---

## 2. SearXNG セットアップ

### Dockerでインストール（推奨）
```bash
mkdir -p ~/searxng && cd ~/searxng
```

`docker-compose.yaml`:
```yaml
services:
  searxng:
    image: searxng/searxng:latest
    container_name: searxng
    ports:
      - "8080:8080"
    volumes:
      - ./searxng-settings:/etc/searxng:rw
    environment:
      - SEARXNG_BASE_URL=http://localhost:8080/
    cap_drop:
      - ALL
    cap_add:
      - CHOWN
      - SETGID
      - SETUID
```

### 設定ファイル
`searxng-settings/settings.yml`:
```yaml
use_default_settings: true
search:
  safe_search: 0
  formats:
    - html
    - json
server:
  secret_key: "okazu-finder-random-secret-change-me"
  bind_address: "0.0.0.0"
  limiter: false
engines:
  # 有効化するエンジン（コメント解除）
  - name: google
    engine: google
    shortcut: g
  - name: duckduckgo
    engine: duckduckgo
    shortcut: ddg
  - name: bing
    engine: bing
    shortcut: bi
  - name: brave
    engine: brave
    shortcut: br
  - name: qwant
    engine: qwant
    shortcut: qw
outgoing:
  request_timeout: 10.0
  max_request_timeout: 15.0
```

### 起動
```bash
docker compose up -d
```

### 動作確認
```bash
curl "http://localhost:8080/search?q=test&format=json"
```

---

## 3. Cloudflare Tunnel セットアップ

SearXNGとOllamaを外部公開せず、Cloudflare Tunnel経由で安全に接続する。

### インストール
```bash
# Linux
curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 -o cloudflared
chmod +x cloudflared
sudo mv cloudflared /usr/local/bin/

# Windows
winget install Cloudflare.cloudflared
```

### ログイン
```bash
cloudflared tunnel login
```

### トンネル作成
```bash
cloudflared tunnel create okazu-home
```

### DNS設定
```bash
# 既存の sandwich-butter.tech ゾーンを使用
cloudflared tunnel route dns okazu-home ollama.sandwich-butter.tech
cloudflared tunnel route dns okazu-home search.sandwich-butter.tech
```

### 設定ファイル（参考・既存設定）
`~/.cloudflared/config.yml`:
```yaml
tunnel: <TUNNEL_ID>  # cloudflared tunnel list で確認
credentials-file: /home/youruser/.cloudflared/<TUNNEL_ID>.json

ingress:
  - hostname: ollama.sandwich-butter.tech
    service: http://localhost:11434
  - hostname: search.sandwich-butter.tech
    service: http://localhost:8080
  - service: http_status:404
```

※ Windows版は `%USERPROFILE%\.cloudflared\config.yml` に配置

### 起動
```bash
cloudflared tunnel run okazu-home
```

### 常駐化（Windows サービス）

すでにサービス登録済みの場合：
```
cloudflared.exe service install eyJhIjoiMDkwYWE5ZmYxZWFmN2ZjOGYwOGRkNGQxOTEzZDU3ZDkiLCJ0IjoiNWU0ODY3ZjctYjBlOS00YjE0LWIyM2YtYTc1Mzg5MDkwODQ2IiwicyI6IlVrblhyaWd1bVFqdExtK3luZjRkTW5pYm14Y2dWNFE2Q2diejcyWnErMmM9In0=
```

※ このトークンは既存の設定からコピー。`services.msc` → `Cloudflare Tunnel` が「実行中」ならOK。

### 常駐化（Linux systemd）
```bash
sudo cloudflared service install
```

---

## 4. okazu-finder サーバー起動（オプション）

Workerを使わず直接Rustサーバーを動かす場合：

### ビルド
```bash
cd okazu-finder/server
cargo build --release
```

### 環境変数
```bash
export OKAZU_OLLAMA_URL="http://localhost:11434"
export OKAZU_OLLAMA_MODEL="gemma4u"
export OKAZU_SEARXNG_URL="http://localhost:8080"
```

### 起動
```bash
./target/release/okazu-finder-server
# → http://0.0.0.0:3001 でlisten
```

### カスタムプロンプト（任意）
```bash
# LLMクエリ分解のプロンプトをカスタマイズ
export OKAZU_DECOMPOSE_PROMPT="You are a search query optimizer..."

# コンテンツ分類のプロンプトをカスタマイズ
export OKAZU_CLASSIFY_PROMPT="You are a content classifier..."
```

---

## 5. Worker + Pages 連携設定

Workerから自宅サーバーにアクセスするための設定：

### 5-1. SearXNG を Worker に通知
```bash
cd okazu-finder/worker
npx wrangler secret put SEARXNG_URL
# → https://search.sandwich-butter.tech と入力
```

### 5-2. Ollama を Worker に通知
```bash
npx wrangler secret put HOME_SERVER_URL
# → https://ollama.sandwich-butter.tech と入力
```

### 5-3. デプロイ確認
```bash
npx wrangler deploy
```

### 既存のデプロイ URL（変更不要）

| サービス | URL |
|---|---|
| フロントエンド (Pages) | `https://okazu-finder.pages.dev` |
| API (Worker) | `https://okazu-finder-worker.butter3.workers.dev` |
| 自宅サーバー (Tunnel) | `https://OKZ-finder.sandwich-butter.tech` |

---

## 6. 動作確認手順

### 1. Ollama確認
```bash
curl http://localhost:11434/api/tags
# → {"models":[{"name":"gemma4u:latest",...}]}
```

### 2. SearXNG確認
```bash
curl "http://localhost:8080/search?q=test&format=json"
# → {"results":[...],"query":"test"}
```

### 3. Worker ヘルスチェック
```bash
curl https://okazu-finder-worker.butter3.workers.dev/health
# → {"status":"ok","ollama":true,"worker":true}
```
※ SEARXNG_URL と HOME_SERVER_URL が設定済みなら ollama:true になる

### 4. Worker 検索テスト
```bash
curl -s -X POST https://okazu-finder-worker.butter3.workers.dev/search \
  -H "Content-Type: application/json" \
  -d '{"query":"フリーレン","max_results":5}'
```
→ 検索結果 + 分類（manga/cg/video/illustration/other）が返ってくれば成功

### 5. Pages フロントエンド確認
ブラウザで以下を開く:
```
https://okazu-finder.pages.dev
```
→ 検索バーが表示され、デフォルトでWorkerを向いている
→ 設定（⚙）からAPI URLの変更も可能

### 6. 自宅サーバー直アクセス確認
```
https://OKZ-finder.sandwich-butter.tech
```
→ Pagesと同一のフロントエンド + サーバー内蔵APIの両方が動作

---

## トラブルシューティング

| 問題 | 確認事項 |
|---|---|---|
| Ollamaに繋がらない | `OLLAMA_HOST=0.0.0.0` 設定確認、ファイアウォールで11434開放 |
| SearXNGが結果ゼロ | `settings.yml` の engines がコメント解除されているか確認 |
| レスポンスが遅い | IQ3_M（4.4GB）を使用。必要ならSearXNGのenginesを減らす |
| メモリ不足 | IQ3_M量子化（4.4GB）に変更、またはシステムRAM増設 |
| 検索結果が少ない | SearXNGにGoogleのAPIキー設定を追加（[docs](https://docs.searxng.org/)） |
| WorkerがOllamaに繋がらない | Cloudflare Tunnelのステータス確認: `cloudflared tunnel info okazu-home` |

---

## 必須スペック

| 項目 | 最低 | 推奨 |
|---|---|---|
| RAM | 8GB | 16GB+ |
| ストレージ | 10GB | 50GB+ |
| ネットワーク | アップロード10Mbps | 100Mbps+ |

※ Gemma 4 E4B (IQ3_M) は4.4GB。内蔵GPUの共有メモリ環境でもCPU推論で動作。
※ VRAM専用GPU不要。CPUのみで4Bモデルなら十分な速度が出る（10-20 token/s）。