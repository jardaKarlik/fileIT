// src/components/Progress/index.tsx
import { useEffect, useRef, useState } from 'react';
import { useStore } from '../../store';
import { api } from '../../utils/tauriApi';
import { CompletedRun, FileError } from '../../types';
import './Progress.css';

const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export function Progress() {
  const {
    setScreen, setCompletedRun, setLastRun,
    progressLines, progressPct, progressCurrent,
    appendProgressLine, setProgressPct, setProgressCurrent, resetProgress,
    backupEnabled,
  } = useStore();

  const logRef = useRef<HTMLDivElement>(null);
  const unlistenRef = useRef<(() => void) | null>(null);
  const didStart = useRef(false);
  const [moveErrors, setMoveErrors] = useState<FileError[]>([]);
  const [showErrors, setShowErrors] = useState(false);

  useEffect(() => {
    if (didStart.current) return;
    didStart.current = true;

    resetProgress();
    appendProgressLine('$ fileit restructure --inicializace');

    if (IS_TAURI) {
      runRealRestructure();
    } else {
      simulateProgress();
    }

    return () => { unlistenRef.current?.(); };
  }, []);

  // Auto-scroll log to bottom
  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [progressLines]);

  // ── Real Tauri path ──────────────────────────────────────────────────────
  // Set up the progress listener BEFORE calling runRestructure so no events
  // are missed. Then drive the actual file moves via runRestructure and build
  // the completedRun from the response — never from hardcoded local state.
  async function runRealRestructure() {
    const unlisten = await api.onRestructureProgress((p) => {
      appendProgressLine(p.log_line);
      if (p.total_files > 0) {
        setProgressPct(Math.round((p.files_moved / p.total_files) * 100));
      }
      setProgressCurrent(p.current_file || 'Zpracovává se…');
      if (p.error) {
        appendProgressLine(`⚠ ${p.error}`);
      }
    });
    unlistenRef.current = unlisten;

    try {
      const result = await api.runRestructure(backupEnabled);
      unlisten();
      unlistenRef.current = null;

      setProgressPct(100);
      setProgressCurrent('Dokončeno');

      if (result.failed_count > 0) {
        setMoveErrors(result.errors);
        appendProgressLine(`⚠ ${result.failed_count} soubor(ů) se nepodařilo přesunout — viz přehled níže`);
      }

      const run: CompletedRun = {
        session_id: result.session_id,
        completed_at: new Date().toISOString(),
        duration_seconds: result.duration_seconds,
        files_moved: result.files_moved,
        folders_created: result.folders_created,
        unknown_count: result.unknown_count,
        // target_path comes from the backend — authoritative, never hardcoded
        target_path: result.target_path,
        backup_manifest_path: result.backup_manifest_path ?? '',
        manifest: { id: '', created_at: '', session_id: result.session_id, moves: [] },
        confirmation_status: 'pending',
        auto_confirm_at: new Date(Date.now() + 30 * 86_400_000).toISOString(),
        backup_delete_at: null,
      };
      setCompletedRun(run);
      setLastRun(run.completed_at, run.files_moved);
      setTimeout(() => setScreen('confirmation'), 600);
    } catch (err) {
      unlisten();
      unlistenRef.current = null;
      appendProgressLine(`⚠ Chyba: ${err}`);
      setProgressCurrent('Chyba — viz záznamy výše');
    }
  }

  // ── Dev browser simulation (no Tauri) ───────────────────────────────────
  function simulateProgress() {
    const lines = [
      { cls: '', text: 'Inicializace…' },
      { cls: 'hit', text: '→ C:\\Users\\Anna\\Documents — 124 souborů' },
      { cls: 'hit', text: '→ C:\\Users\\Anna\\Desktop — 47 souborů' },
      { cls: 'hit', text: '→ C:\\Users\\Anna\\Downloads — 113 souborů' },
      { cls: 'found', text: '✓ Manifest zálohy vytvořen' },
      { cls: '', text: 'Přesouváme soubory…' },
      { cls: 'found', text: '✓ Novák Pavel\\2025-11 — 8 souborů' },
      { cls: 'found', text: '✓ Novák Pavel\\2025-10 — 14 souborů' },
      { cls: 'found', text: '✓ Horáková Jana\\2025-11 — 5 souborů' },
      { cls: 'found', text: '✓ Horáková Jana\\2025-09 — 13 souborů' },
      { cls: 'found', text: '✓ Müller Thomas\\2026-01 — 15 souborů' },
      { cls: 'found', text: '✓ Svoboda Lucie\\2025-10 — 14 souborů' },
      { cls: 'found', text: '✓ Kratochvíl Petr\\2026-02 — 12 souborů' },
      { cls: '', text: '… zpracovávám zbývající klienty …' },
      { cls: 'found', text: '✓ Neznámý klient — 11 souborů (bez identifikace)' },
      { cls: 'done', text: '✓ Uspořádání dokončeno · 284 souborů · 165 složek' },
    ];

    let i = 0;
    const interval = setInterval(() => {
      if (i >= lines.length) {
        clearInterval(interval);
        setProgressPct(100);
        setProgressCurrent('Dokončeno');
        // In dev mode: call the mock runRestructure to exercise the API path
        api.runRestructure(backupEnabled).then((result) => {
          const run: CompletedRun = {
            session_id: result.session_id,
            completed_at: new Date().toISOString(),
            duration_seconds: result.duration_seconds,
            files_moved: result.files_moved,
            folders_created: result.folders_created,
            unknown_count: result.unknown_count,
            target_path: result.target_path,
            backup_manifest_path: result.backup_manifest_path ?? '',
            manifest: { id: '', created_at: '', session_id: result.session_id, moves: [] },
            confirmation_status: 'pending',
            auto_confirm_at: new Date(Date.now() + 30 * 86_400_000).toISOString(),
            backup_delete_at: null,
          };
          setCompletedRun(run);
          setLastRun(run.completed_at, run.files_moved);
          setTimeout(() => setScreen('confirmation'), 800);
        });
        return;
      }
      const pct = Math.round((i / lines.length) * 100);
      setProgressPct(pct);
      setProgressCurrent(lines[i].text.replace(/^[✓→] /, ''));
      appendProgressLine(lines[i].text);
      i++;
    }, 280);
  }

  return (
    <div className="progress-wrap">
      <div className="progress-eyebrow">Probíhá</div>
      <h2 className="progress-title">Přesouváme vaše soubory…</h2>
      <p className="progress-sub">Nic nevypínejte. Uspořádání právě probíhá — jakmile to dokončíme, ukážeme vám výsledek a vyzveme vás ke kontrole.</p>

      <div className="progress-big-bar">
        <div className="progress-big-fill" style={{ width: `${progressPct}%` }} />
      </div>
      <div className="progress-status-row">
        <span className="progress-status-text">{progressCurrent}</span>
        <span><strong>{progressPct} %</strong></span>
      </div>

      <div className="progress-log-pane" ref={logRef}>
        {progressLines.map((line, i) => {
          const cls = line.startsWith('✓') ? 'found' : line.startsWith('→') ? 'hit' : line.startsWith('$') ? 'cmd' : line.startsWith('⚠') ? 'hit' : '';
          return (
            <div key={i} className={`progress-log-line ${cls}`}>{line}</div>
          );
        })}
        <div className="progress-cursor" />
      </div>

      {moveErrors.length > 0 && (
        <div style={{
          marginTop: 16, padding: '12px 16px',
          background: 'rgba(255, 80, 80, 0.08)', border: '1px solid rgba(255, 80, 80, 0.3)',
          borderRadius: 8, fontSize: 13,
        }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <strong style={{ color: '#ff5050' }}>⚠ {moveErrors.length} soubor(ů) se nepodařilo přesunout</strong>
            <button
              onClick={() => setShowErrors(v => !v)}
              style={{ background: 'none', border: 'none', cursor: 'pointer', color: 'inherit', fontSize: 12, opacity: 0.7 }}
            >
              {showErrors ? 'Skrýt ↑' : 'Zobrazit detaily ↓'}
            </button>
          </div>
          {showErrors && (
            <div style={{ marginTop: 8, display: 'flex', flexDirection: 'column', gap: 6 }}>
              {moveErrors.map((err, i) => (
                <div key={i} style={{ fontSize: 11, opacity: 0.85 }}>
                  <div style={{ fontWeight: 600, wordBreak: 'break-all' }}>{err.source.split('\\').pop()}</div>
                  <div style={{ opacity: 0.7 }}>{err.error_message}</div>
                </div>
              ))}
              <div style={{ marginTop: 4, opacity: 0.6, fontSize: 11 }}>
                Chyby jsou zaznamenány v AppData/error_logs/. Ostatní soubory byly úspěšně přesunuty.
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
