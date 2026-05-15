import type { SearchResult } from '../types';

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
        <div className="url">{result.url}</div>
        {result.snippet && (
          <div className="snippet">{result.snippet}</div>
        )}
      </div>
    </a>
  );
}