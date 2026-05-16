import type { SearchResult } from '../types';
import { CONTENT_TYPE_LABELS } from '../types';

interface Props {
  result: SearchResult;
}

export function ResultCard({ result }: Props) {
  return (
    <a
      className="result-card"
      href={result.url}
      target="_blank"
      rel="noopener noreferrer"
    >
      <div className="thumbnail">
        {result.thumbnail ? (
          <img src={result.thumbnail} alt="" loading="lazy" />
        ) : (
          <span>NO IMG</span>
        )}
      </div>
      <div className="info">
        <div className="title">{result.title || result.url}</div>
        <div className="result-meta">
          <span className={`content-badge ${result.content_type}`}>
            {CONTENT_TYPE_LABELS[result.content_type]}
          </span>
          <span className="url">{new URL(result.url).hostname}</span>
        </div>
        {result.author && (
          <div className="result-author">by {result.author}</div>
        )}
        {result.media_urls.length > 0 && (
          <div className="result-media-count">
            {result.media_urls.length} media
          </div>
        )}
        {result.snippet && (
          <div className="snippet">{result.snippet}</div>
        )}
      </div>
    </a>
  );
}