import type { QueryPlan, SearchResult, ClassifiedResults, Env } from './types';

// ── クエリ分解プロンプト ──────────────────────────────────
const QUERY_DECOMPOSE_PROMPT = `あなたはアダルトコンテンツ専門の検索クエリ最適化AIです。
ユーザーが入力したキーワード（キャラ名・シリーズ名・シチュエーション・フェチなど）を元に、SearXNGで使う最適な検索クエリを生成してください。

検索対象サイト：
- site:hitomi.la → 同人誌・エロ漫画・CG集・アートブック
- site:kemono.su OR site:kemono.party → Fanbox/Patreon/Boosty/Fantiaの有料コンテンツ
- site:x.com OR site:twitter.com → エロイラスト・R18ファンアート
- site:momon-ga.com → エロ漫画・同人誌の転載ギャラリー
- site:pixiv.net → R18イラスト・漫画
- site:fanbox.cc → クリエイターの有料投稿
- site:patreon.com → クリエイターの有料投稿
- site:fantia.jp → クリエイターの有料投稿
- site:deviantart.com → アートワーク
- site:skeb.jp → コミッション作品
- 一般クエリ（site:なし）→ 上記以外のサイトも含めた広域検索

クエリ生成ルール：
1. 合計4〜8個のクエリを生成（site:あり＋一般クエリを混ぜる）
2. 日本語と英語/ローマ字の両方を含める（例："site:hitomi.la フリーレン フェルン R18" "site:hitomi.la frieren fern hentai"）
3. 検索意図に応じて以下のキーワードを適切に追加：
   - 同人誌系: doujin, 同人, R18, 成人向け, hentai
   - イラスト系: illust, イラスト, fanart, R18, NSFW, えっち
   - 動画系: animation, video, 動画, MP4
   - CG集系: CG集, CG collection, 3D, digital
   - シチュエーション・フェチはそのまま英語/日本語で追加
4. キャラ名の表記ゆれを考慮（漢字/カタカナ/ローマ字）
5. safesearchやSFWフィルターを外す指示は不要（SearXNG側でsafesearch=0を指定済）

必ずJSON配列のみを返してください。説明やマークダウンは一切不要です：
["query1", "query2", "query3"]`;

// ── コンテンツ分類プロンプト ──────────────────────────────
const CLASSIFY_PROMPT = `あなたはアダルトコンテンツの分類AIです。各検索結果を正確なカテゴリに分類してください。

カテゴリ定義：
- "manga": 同人誌、エロ漫画、成年コミック、漫画形式のCG集、アートブック
  ※ hitomi.laやmomon-ga.comから来たものは基本manga
  ※ ページ数が多く連続した画像があるコンテンツもmanga
- "cg": 3DCG集、デジタルアートセット、ゲームのエロMOD/リッピング、CGイラスト集
  ※ 複数枚のレンダリング画像で構成されるセット
  ※ patreon/fanboxのCGクリエイター作品もここ
- "video": アニメーション、動画、MP4/WEBM/GIFアニメ、同人アニメ
  ※ youtube/nicovideo/bilibiliのURLは原則video
- "illustration": 一枚絵、イラスト、ファンアート、ラクガキ、スクリーンショット
  ※ twitter/pixiv/skeb/kemonoの単品投稿は基本これ
  ※ 画像が1〜3枚程度の投稿もillustration
- "other": 上記のどれにも当てはまらない、または判断不能

サイト別の分類バイアス（優先度高）：
- hitomi.la → ほぼ "manga"（同人誌サイト）。ごく稀にアートブック系で "illustration"
- kemono.su → 基本 "illustration"（Fanbox投稿の転載）。動画投稿なら "video"
- twitter.com / x.com → 基本 "illustration"。動画付きツイートは "video"
- momon-ga.com → ほぼ "manga"（エロ漫画転載サイト）
- pixiv.net → "illustration" または "manga"（ページ数で判断）
- fanbox.cc / patreon.com / fantia.jp → "illustration" または "cg"（CGクリエイターの場合）
- youtube.com / nicovideo.jp / bilibili.com → 必ず "video"
- skeb.jp / skima.jp → "illustration"（コミッション作品）

作者名の抽出：
- URLやタイトル、スニペットから作者名が推測できる場合は author に設定
- kemonoの場合は "username (service)" 形式
- twitterの場合は @handle または表示名
- pixivの場合はユーザー名

入力は検索結果のJSON配列です。以下の形式のJSONオブジェクトのみを返してください：
{"results":{"0":{"content_type":"manga","author":"サークル名"},"1":{"content_type":"illustration","author":"@artist"}}}`;

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