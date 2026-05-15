import { useState, useCallback } from 'react';
import { SearchBar } from './components/SearchBar';
import { QueryPlanView } from './components/QueryPlanView';
import { ResultList } from './components/ResultList';
import { SettingsPanel } from './components/SettingsPanel';
import type { QueryPlan, SiteResults } from './types';
import { search } from './api/client';

export default function App() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [queryPlan, setQueryPlan] = useState<QueryPlan | null>(null);
  const [results, setResults] = useState<SiteResults[]>([]);
  const [showSettings, setShowSettings] = useState(false);

  const handleSearch = useCallback(async (q: string) => {
    setLoading(true);
    setError(null);
    setQueryPlan(null);
    setResults([]);

    try {
      const res = await search(q);
      setQueryPlan(res.query_plan);
      setResults(res.all_results);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed');
    } finally {
      setLoading(false);
    }
  }, []);

  return (
    <div className="app">
      <button
        className="settings-toggle"
        onClick={() => setShowSettings(true)}
        title="設定"
      >
        ⚙
      </button>

      <header className="header">
        <h1>okazu-finder</h1>
        <p>Multi-platform content discovery with local LLM</p>
      </header>

      <section className="search-section">
        <SearchBar onSearch={handleSearch} loading={loading} />
      </section>

      {error && <div className="error-banner">{error}</div>}

      {loading && (
        <div className="loading">
          <div className="spinner" />
          <span>Searching across platforms...</span>
        </div>
      )}

      {queryPlan && <QueryPlanView plan={queryPlan} />}

      {results.length > 0 && (
        <section className="results-section">
          {results.map((sr) => (
            <ResultList key={sr.site} siteResults={sr} />
          ))}
        </section>
      )}

      {!loading && !queryPlan && results.length === 0 && (
        <div className="empty-state">
          <div className="icon">🔍</div>
          <p>Enter a character name, series, or topic to search across multiple platforms.</p>
        </div>
      )}

      {showSettings && (
        <SettingsPanel onClose={() => setShowSettings(false)} />
      )}
    </div>
  );
}