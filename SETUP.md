# okazu-finder サーバーセットアップ指示書

## 概要

自宅サーバーで以下の3つのサービスを立ち上げる：
1. **Ollama** - LLM推論（クエリ分解 + コンテンツ分類）
2. **SearXNG** - メタ検索エンジン
3. **Cloudflare Tunnel** - 自宅サーバーを安全に公開

---

## 1. Ollama セットアップ

### インストール
```bash
# Linux
curl -fsSL https://ollama.com/install.sh | sh

# Windows
winget install Ollama.Ollama
```

### モデルをプル
```bash
ollama pull gemma3:12b
```

※ 12Bモデルには最低16GBのVRAM推奨。VRAMが足りなければ `gemma3:4b` でも動くが精度は落ちる。
※ uncensored版が必要なら [ollama.com](https://ollama.com) で `gemma3-uncensored` などを探して pull。

### 動作確認
```bash
ollama run gemma3:12b "Hello"
```

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
# ollama用サブドメイン
cloudflared tunnel route dns okazu-home ollama.yourdomain.com

# searxng用サブドメイン
cloudflared tunnel route dns okazu-home search.yourdomain.com
```

### 設定ファイル
`~/.cloudflared/config.yml`:
```yaml
tunnel: <TUNNEL_ID>  # cloudflared tunnel list で確認
credentials-file: /home/youruser/.cloudflared/<TUNNEL_ID>.json

ingress:
  - hostname: ollama.yourdomain.com
    service: http://localhost:11434
  - hostname: search.yourdomain.com
    service: http://localhost:8080
  - service: http_status:404
```

### 起動
```bash
cloudflared tunnel run okazu-home
```

### 常駐化（systemd）
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
export OKAZU_OLLAMA_MODEL="gemma3:12b"
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

## 5. Worker連携設定

Workerから自宅サーバーにアクセスするための設定：

```bash
cd okazu-finder/worker
npx wrangler secret put HOME_SERVER_URL
# → https://ollama.yourdomain.com と入力

npx wrangler secret put SEARXNG_URL
# → https://search.yourdomain.com と入力
```

---

## 6. 動作確認手順

### 1. Ollama確認
```bash
curl http://localhost:11434/api/tags
# → {"models":[{"name":"gemma3:12b",...}]}
```

### 2. SearXNG確認
```bash
curl "http://localhost:8080/search?q=test&format=json"
# → {"results":[...],"query":"test"}
```

### 3. Worker確認
```bash
curl https://okazu-finder-worker.butter3.workers.dev/api/health
# → {"status":"ok","ollama":true,"worker":true}
```

### 4. E2Eテスト
```bash
curl -X POST https://okazu-finder-worker.butter3.workers.dev/api/search \
  -H "Content-Type: application/json" \
  -d '{"query":"フリーレン"}'
```
→ 検索結果 + 分類（manga/cg/video/illustration/other）が返ってくれば成功

---

## トラブルシューティング

| 問題 | 確認事項 |
|---|---|
| Ollamaに繋がらない | `OLLAMA_HOST=0.0.0.0` 設定確認、ファイアウォールで11434開放 |
| SearXNGが結果ゼロ | `settings.yml` の engines がコメント解除されているか確認 |
| レスポンスが遅い | `gemma3:4b` に変更、またはSearXNGの engines を減らす |
| 検索結果が少ない | SearXNGにGoogleのAPIキー設定を追加（[docs](https://docs.searxng.org/)） |
| WorkerがOllamaに繋がらない | Cloudflare Tunnelのステータス確認: `cloudflared tunnel info okazu-home` |

---

## 必須スペック

| 項目 | 最低 | 推奨 |
|---|---|---|
| RAM | 16GB | 32GB+ |
| VRAM | 8GB (4Bモデル) | 16GB+ (12Bモデル) |
| ストレージ | 10GB | 50GB+ |
| ネットワーク | アップロード10Mbps | 100Mbps+ |