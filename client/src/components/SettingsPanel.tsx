import { useState, useEffect } from 'react';
import type { HealthResponse } from '../types';
import { checkHealth } from '../api/client';

interface Props {
  onClose: () => void;
}

function getStored(key: string, fallback: string): string {
  if (typeof window !== 'undefined') {
    return localStorage.getItem(key) || fallback;
  }
  return fallback;
}

export function SettingsPanel({ onClose }: Props) {
  const [apiUrl, setApiUrl] = useState(
    () => getStored('okazu_api_url', ''),
  );
  const [ollamaUrl, setOllamaUrl] = useState(
    () => getStored('okazu_ollama_url', 'http://localhost:11434'),
  );
  const [ollamaModel, setOllamaModel] = useState(
    () => getStored('okazu_ollama_model', 'gemma3:12b'),
  );
  const [searxngUrl, setSearxngUrl] = useState(
    () => getStored('okazu_searxng_url', 'http://localhost:8080'),
  );
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [checking, setChecking] = useState(false);

  const doHealthCheck = async (url?: string) => {
    setChecking(true);
    try {
      const h = await checkHealth(url || apiUrl || undefined);
      setHealth(h);
    } catch {
      setHealth({ ollama: false, worker: false, status: 'error' });
    } finally {
      setChecking(false);
    }
  };

  useEffect(() => {
    doHealthCheck();
  }, []);

  const handleSave = () => {
    if (apiUrl) {
      localStorage.setItem('okazu_api_url', apiUrl);
    } else {
      localStorage.removeItem('okazu_api_url');
    }
    localStorage.setItem('okazu_ollama_url', ollamaUrl);
    localStorage.setItem('okazu_ollama_model', ollamaModel);
    localStorage.setItem('okazu_searxng_url', searxngUrl);
    onClose();
    window.location.reload();
  };

  return (
    <div className="settings-overlay" onClick={onClose}>
      <div className="settings-panel" onClick={(e) => e.stopPropagation()}>
        <h2>設定</h2>

        {health && (
          <div className="status-bar" style={{ justifyContent: 'flex-start', marginBottom: 16 }}>
            <span>
              <span className={`status-dot ${health.worker ? 'ok' : 'down'}`} /> API
            </span>
            <span>
              <span className={`status-dot ${health.ollama ? 'ok' : 'down'}`} /> Ollama
            </span>
          </div>
        )}

        <div className="field">
          <label>API URL (Worker)</label>
          <input
            type="text"
            value={apiUrl}
            onChange={(e) => setApiUrl(e.target.value)}
            placeholder="https://okazu-finder-worker.USERNAME.workers.dev"
          />
        </div>

        <div className="field">
          <label>自宅サーバー URL</label>
          <input
            type="text"
            value={ollamaUrl}
            onChange={(e) => setOllamaUrl(e.target.value)}
            placeholder="http://localhost:11434"
          />
        </div>

        <div className="field">
          <label>Ollama Model</label>
          <input
            type="text"
            value={ollamaModel}
            onChange={(e) => setOllamaModel(e.target.value)}
          />
        </div>

        <div className="field">
          <label>SearXNG URL</label>
          <input
            type="text"
            value={searxngUrl}
            onChange={(e) => setSearxngUrl(e.target.value)}
          />
        </div>

        <div style={{ display: 'flex', justifyContent: 'flex-end', marginBottom: 16 }}>
          <button
            className="btn-secondary"
            onClick={() => doHealthCheck(apiUrl || undefined)}
            disabled={checking}
            style={{ fontSize: '0.8rem', padding: '6px 12px' }}
          >
            {checking ? '確認中...' : '接続確認'}
          </button>
        </div>

        <div className="actions">
          <button className="btn-secondary" onClick={onClose}>
            キャンセル
          </button>
          <button className="btn-primary" onClick={handleSave}>
            保存して再読込
          </button>
        </div>
      </div>
    </div>
  );
}