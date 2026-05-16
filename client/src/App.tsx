import { useState, useCallback } from 'react';
import { SearchBar } from './components/SearchBar';
import { QueryPlanView } from './components/QueryPlanView';
import { ResultCard } from './components/ResultCard';
import { SettingsPanel } from './components/SettingsPanel';
import type { ClassifiedResults, ContentType, QueryPlan } from './types';
import { CONTENT_TYPE_LABELS } from './types';
import { search } from './api/client';

const ALL_TABS: ContentType[] = ['manga', 'cg', 'video', 'illustration', 'other'];

export default function App() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [queryPlan, setQueryPlan] = useState<QueryPlan | null>(null);
  const [classified, setClassified] = useState<ClassifiedResults | null>(null);
  const [activeTab, setActiveTab] = useState<ContentType | null>(null);
  const [showSettings, setShowSettings] = useState(false);

  const handleSearch = useCallback(async (q: string) => {
    setLoading(true);
    setError(null);
    setQueryPlan(null);
    setClassified(null);
    setActiveTab(null);

    try {
      const res = await search(q);
      setQueryPlan(res.query_plan);
      setClassified(res.classified);
      const firstNonEmpty = ALL_TABS.find((t) => res.classified[t].length > 0);
      if (firstNonEmpty) setActiveTab(firstNonEmpty);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed');
    } finally {
      setLoading(false);
    }
  }, []);

  const activeResults = classified && activeTab ? classified[activeTab] : [];

  const getTabCount = (tab: ContentType): number => classified?.[tab]?.length ?? 0;
  const totalResults = classified
    ? ALL_TABS.reduce((sum, t) => sum + classified[t].length, 0)
    : 0;

  return (
    <div className="app">
      <button className="settings-toggle" onClick={() => setShowSettings(true)} title="設定">
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
          <span>Searching and classifying...</span>
        </div>
      )}

      {queryPlan && <QueryPlanView plan={queryPlan} />}

      {classified && (
        <>
          <div className="tab-bar">
            {ALL_TABS.map((tab) => {
              const count = getTabCount(tab);
              if (count === 0) return null;
              return (
                <button
                  key={tab}
                  className={`tab-btn ${activeTab === tab ? 'active' : ''}`}
                  onClick={() => setActiveTab(tab)}
                >
                  {CONTENT_TYPE_LABELS[tab]}
                  <span className="tab-count">{count}</span>
                </button>
              );
            })}
            <span className="tab-total">{totalResults} 件</span>
          </div>

          {activeResults.length > 0 && (
            <section className="results-section">
              {activeResults.map((r, i) => (
                <ResultCard key={`${r.url}-${i}`} result={r} />
              ))}
            </section>
          )}
        </>
      )}

      {!loading && !classified && (
        <div className="empty-state">
          <div className="icon">🔍</div>
          <p>Enter a character name, series, or topic to search across platforms.</p>
        </div>
      )}

      {showSettings && <SettingsPanel onClose={() => setShowSettings(false)} />}
    </div>
  );
}