// src/components/Scanning/index.tsx
import { useEffect, useRef, useState } from 'react';
import { useStore } from '../../store';
import { api } from '../../utils/tauriApi';
import './Scanning.css';

// True when running inside a compiled Tauri window (not browser dev mode)
const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

type LogLine = { cls: string; text: string };

export function Scanning() {
  const { setScreen, setScanResult, selectedCategories, scanRoots, detectedClouds } = useStore();
  const [lines, setLines]           = useState<LogLine[]>([]);
  const [pct, setPct]               = useState(0);
  const [statusText, setStatusText] = useState('Inicializace skeneru…');
  const [done, setDone]             = useState(false);
  const logRef   = useRef<HTMLDivElement>(null);
  const didStart = useRef(false);

  const appendLine = (line: LogLine) =>
    setLines(prev => [...prev.slice(-300), line]);

  // ── Real Tauri path ──────────────────────────────────────────────────────
  async function runRealScan() {
    const roots = scanRoots.length > 0
      ? scanRoots
      : ['C:\\_temp'];

    appendLine({ cls: '', text: '$ fileit scan --inicializace' });

    // Await listener registration so no early events are missed, and keep
    // unlisten handles so they can be cleaned up when the scan finishes.
    const [unlistenScan, unlistenClassify] = await Promise.all([
      api.onScanProgress(p => {
        if (p.message) appendLine({ cls: 'hit', text: p.message });
        if (p.files_found > 0) {
          setStatusText(`Nalezeno ${p.files_found} souborů…`);
          setPct(Math.min(45, Math.round(p.files_found / 6)));
        }
        if (p.done) {
          appendLine({ cls: '', text: `Celkem ${p.files_found} souborů — spouštíme klasifikaci…` });
        }
      }),
      api.onClassifyProgress(p => {
        if (p.current) appendLine({ cls: 'found', text: `✓ ${p.current}` });
        const classifyPct = 45 + Math.round((p.done / Math.max(p.total, 1)) * 50);
        setPct(Math.min(95, classifyPct));
        setStatusText(`Klasifikuji ${p.done} / ${p.total}…`);
      }),
    ]);

    api.startScan(roots, selectedCategories)
      .then(result => {
        setScanResult(result);
        setPct(100);
        setStatusText(`Hotovo — ${result.total_files} souborů, ${result.customers_found} klientů`);
        appendLine({
          cls: 'done',
          text: `✓ Analýza dokončena · ${result.total_files} souborů · ${result.customers_found} klientů`,
        });
        if (result.duplicates_found > 0) {
          appendLine({ cls: '', text: `Detekce duplikátů: ${result.duplicates_found} možných duplikátů` });
        }
        setDone(true);
        unlistenScan();
        unlistenClassify();
      })
      .catch(err => {
        appendLine({ cls: 'hit', text: `⚠ Chyba: ${err}` });
        setStatusText('Chyba při skenování');
        unlistenScan();
        unlistenClassify();
      });
  }

  // ── Dev browser simulation — generic labels, no hardcoded names ──────────
  function runSimulation() {
    const DEV_LINES: LogLine[] = [
      { cls: '',      text: '$ fileit scan --inicializace' },
      { cls: 'hit',   text: '→ Složka 1 — prohledávám…' },
      { cls: 'hit',   text: '→ Složka 2 — prohledávám…' },
      { cls: 'hit',   text: '→ Složka 3 — prohledávám…' },
      { cls: '',      text: 'Načítání slovníku institucí (27 záznamů)…' },
      { cls: 'found', text: '✓ Klient identifikován (rodné číslo)' },
      { cls: 'found', text: '✓ Instituce rozpoznána — banka' },
      { cls: 'found', text: '✓ Klient identifikován (rodné číslo)' },
      { cls: 'found', text: '✓ Instituce: banka' },
      { cls: 'found', text: '✓ Instituce rozpoznána — pojišťovna' },
      { cls: 'found', text: '✓ Klient identifikován (IČO)' },
      { cls: '',      text: 'OCR zpracovává skenované stránky…' },
      { cls: 'found', text: '✓ Rodné číslo rozpoznáno — klient přiřazen' },
      { cls: 'found', text: '✓ Instituce: pojišťovna' },
      { cls: '',      text: 'Kontrola duplikátů…' },
      { cls: 'done',  text: '✓ Analýza dokončena — výsledky připraveny' },
    ];

    let i = 0;
    const iv = setInterval(() => {
      if (i >= DEV_LINES.length) {
        clearInterval(iv);
        api.startScan([], selectedCategories).then(result => {
          setScanResult(result);
          setPct(100);
          setStatusText(`Hotovo — ${result.total_files} souborů, ${result.customers_found} klientů`);
          setDone(true);
        });
        return;
      }
      appendLine(DEV_LINES[i]);
      setPct(Math.round(((i + 1) / DEV_LINES.length) * 95));
      setStatusText(DEV_LINES[i].text.replace(/^[✓→⚠] /, '').replace(/^\$ /, ''));
      i++;
    }, 320);

    return () => clearInterval(iv);
  }

  useEffect(() => {
    if (didStart.current) return;
    didStart.current = true;
    if (IS_TAURI) {
      runRealScan();
    } else {
      return runSimulation();
    }
  }, []);

  useEffect(() => {
    if (logRef.current) logRef.current.scrollTop = logRef.current.scrollHeight;
  }, [lines]);

  const runningCloud = detectedClouds.find(c => c.is_running);

  return (
    <div className="scan-wrap">
      <div className="scan-anim">
        <div className={`scan-ring ${done ? 'done' : ''}`} />
        <div className={`scan-ring ${done ? 'done' : ''}`} />
        <div className={`scan-ring ${done ? 'done' : ''}`} />
        <div className="scan-inner">{done ? '✓' : '⚙'}</div>
      </div>

      <h2 className="scan-title">{done ? 'Analýza dokončena.' : 'Čteme vaše soubory…'}</h2>
      <p className="scan-sub">
        {done
          ? 'Všechny soubory byly analyzovány. Klikněte níže a prohlédněte si výsledky.'
          : 'FileIT prohlíží vybrané disky a nahlíží do dokumentů, aby rozuměl, o co se jedná. Může to trvat několik minut.'
        }
      </p>

      <div className="scan-bar"><div className="scan-fill" style={{ width: `${pct}%` }} /></div>
      <div className="scan-status">{statusText}</div>

      <div className="scan-log" ref={logRef}>
        {lines.map((l, i) => (
          <div key={i} className={`scan-log-line ${l.cls}`}>{l.text}</div>
        ))}
        {!done && <div className="progress-cursor" />}
      </div>

      {runningCloud && !done && (
        <div className="scan-cloud-detect">
          <div className="scan-cloud-label">☁ Rozpoznáno cloudové úložiště</div>
          <div className="scan-cloud-text">
            {runningCloud.name} je nainstalován a běží. Po dokončení analýzy budete moci
            zálohovat uspořádané soubory.
          </div>
        </div>
      )}

      <button
        className="btn-run"
        style={{ margin: '0 auto', display: 'flex' }}
        onClick={() => setScreen('dashboard')}
        disabled={!done}
      >
        Zobrazit výsledky →
      </button>
    </div>
  );
}
