import type { SiteResults } from '../types';
import { ResultCard } from './ResultCard';

interface Props {
  siteResults: SiteResults;
}

export function ResultList({ siteResults }: Props) {
  const { site, results } = siteResults;
  if (results.length === 0) return null;

  return (
    <div className="site-results">
      <div className="site-results-header">
        <h3>{site}</h3>
        <span className="count">{results.length} 件</span>
      </div>
      {results.map((r, i) => (
        <ResultCard key={`${r.url}-${i}`} result={r} />
      ))}
    </div>
  );
}