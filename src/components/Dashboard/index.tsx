// src/components/Dashboard/index.tsx

import { useEffect, useState } from 'react';
import { useStore } from '../../store';
import { api } from '../../utils/tauriApi';
import './Dashboard.css';

const TIMELINE_YEARS = 12;

type OcrState = 'unknown' | 'installed' | 'unavailable';
type OcrRunState = 'idle' | 'running' | 'done' | 'error' | 'unavailable';

type TimelineData = {
  years: string[];
  vals: number[];
  startYear: number;
  endYear: number;
  outsideCount: number;
};

// Keep the dashboard timeline bounded. Real-world office data can contain
// corrupt/default dates (for example year 100), which must not stretch the
// chart or make recent activity unreadable.
export function getTimelineData(files: any[], now = new Date()): TimelineData {
  const endYear = now.getFullYear();
  const startYear = endYear - TIMELINE_YEARS + 1;
  const yearCounts: Record<number, number> = {};

  for (let year = startYear; year <= endYear; year += 1) {
    yearCounts[year] = 0;
  }

  let outsideCount = 0;
  files.forEach(file => {
    const dateStr = file.document_date || file.record?.modified;
    const date = dateStr ? new Date(dateStr) : null;
    const year = date && !Number.isNaN(date.getTime()) ? date.getFullYear() : null;

    if (year === null || year < startYear || year > endYear) {
      outsideCount += 1;
    } else {
      yearCounts[year] += 1;
    }
  });

  const years = Object.keys(yearCounts).map(Number);
  return {
    years: years.map(String),
    vals: years.map(year => yearCounts[year]),
    startYear,
    endYear,
    outsideCount,
  };
}

// Derive top customers from real scan data
function getTopCustomers(files: any[]): [string, number][] {
  const counts: Record<string, number> = {};
  files.forEach(f => {
    const name = f.customer?.name;
    if (name) counts[name] = (counts[name] ?? 0) + 1;
  });
  return Object.entries(counts)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 6) as [string, number][];
}

export function Dashboard() {
  const { setScreen, scanResult } = useStore();

  const [backupState, setBackupState] = useState<'idle' | 'loading' | 'done' | 'error'>('idle');
  const [backupPath, setBackupPath] = useState<string>('');
  const [exportState, setExportState] = useState<'idle' | 'loading' | 'done'>('idle');

  // ── OCR status — read-only, no OCR ever runs on its own ──────────────────
  const [ocrState, setOcrState] = useState<OcrState>('unknown');
  const [ocrCached, setOcrCached] = useState(0);
  const [ocrRun, setOcrRun] = useState<OcrRunState>('idle');
  const [ocrProgress, setOcrProgress] = useState<{ done: number; total: number } | null>(null);
  const [ocrMsg, setOcrMsg] = useState('');

  const totalFiles = scanResult?.total_files ?? 0;
  const totalGB   = ((scanResult?.total_size_bytes ?? 0) / 1e9).toFixed(1).replace('.', ',');
  const customers = scanResult?.customers_found ?? 0;
  const dupes     = scanResult?.duplicates_found ?? 0;

  // Count unclassified files for the Učebna badge
  const unclassifiedCount = scanResult?.files
    ? scanResult.files.filter(f => !f.institution || f.confidence < 0.65).length
    : 0;

  // Get years and file counts for timeline
  const {
    years: YEARS,
    vals: VALS,
    startYear: timelineStartYear,
    endYear: timelineEndYear,
    outsideCount: timelineOutsideCount,
  } = getTimelineData(scanResult?.files ?? []);
  const MAX = Math.max(...VALS, 1);

  const topCustomers = getTopCustomers(scanResult?.files ?? []);
  const maxCustCount = topCustomers.length > 0 ? topCustomers[0][1] : 1;

  // Files that could plausibly need OCR: images and PDFs. We only ever send
  // these, never anything else — the sidecar would reject the rest anyway.
  const ocrCandidates = (scanResult?.files ?? []).filter(
    f => f.category === 'image' || f.category === 'pdf'
  );

  // Probe the sidecar once on mount. A failed probe is a normal "not installed"
  // outcome, not an error worth alarming the user about.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const avail = await api.checkOcr();
        if (cancelled) return;
        setOcrState(avail.installed ? 'installed' : 'unavailable');
        setOcrCached(avail.cached_count);
      } catch (e) {
        if (!cancelled) setOcrState('unavailable');
        console.error('OCR check failed:', e);
      }
    })();
    return () => { cancelled = true; };
  }, []);

  async function handleOcr() {
    if (ocrRun === 'running') return;
    if (ocrCandidates.length === 0) return;

    setOcrRun('running');
    setOcrProgress({ done: 0, total: ocrCandidates.length });
    setOcrMsg('');

    const unlisten = await api.onOcrProgress(p => {
      setOcrProgress({ done: p.done, total: p.total });
    });

    try {
      const result = await api.runOcr(ocrCandidates.map(f => f.record.path));
      if (result.skipped_unavailable) {
        setOcrRun('unavailable');
        setOcrMsg('OCR není nainstalován');
      } else {
        setOcrRun('done');
        const parts = [`Zpracováno ${result.processed}`];
        if (result.from_cache > 0) parts.push(`${result.from_cache} z mezipaměti`);
        if (result.failed > 0) parts.push(`${result.failed} selhalo`);
        setOcrMsg(parts.join(' · '));
      }
      // Refresh the cached counter so the status row stays truthful.
      const avail = await api.checkOcr();
      setOcrCached(avail.cached_count);
      setOcrState(avail.installed ? 'installed' : 'unavailable');
    } catch (e) {
      console.error('OCR run failed:', e);
      setOcrRun('error');
      setOcrMsg('OCR selhalo — zkuste to znovu');
    } finally {
      unlisten();
      setOcrProgress(null);
    }
  }

  async function handleBackup() {
    if (backupState === 'loading') return;
    setBackupState('loading');
    try {
      const zipPath = await api.createStandaloneBackup();
      setBackupPath(zipPath);
      setBackupState('done');
    } catch (e) {
      console.error('Backup failed:', e);
      setBackupState('error');
      setTimeout(() => setBackupState('idle'), 4000);
    }
  }

  function handleExport() {
    if (!scanResult?.files?.length) return;
    setExportState('loading');

    try {
      const headers = ['Soubor', 'Cesta', 'Klient', 'Instituce', 'Instituce (typ)', 'Datum dokumentu', 'Shoda %', 'Duplicita'];
      const rows = scanResult.files.map(f => [
        f.record.name,
        f.record.path,
        f.customer?.name ?? '',
        f.institution?.canonical_name ?? '',
        f.document_type ?? '',
        f.document_date ? new Date(f.document_date).toLocaleDateString('cs-CZ') : '',
        Math.round((f.confidence ?? 0) * 100) + '%',
        f.is_duplicate ? 'ano' : 'ne',
      ]);

      const csvContent = [headers, ...rows]
        .map(row => row.map(cell =>
          '"' + String(cell ?? '').replace(/"/g, '""') + '"'
        ).join(';'))
        .join('\n');

      const blob = new Blob(['﻿' + csvContent], { type: 'text/csv;charset=utf-8;' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `fileit_relace_${new Date().toISOString().slice(0, 10)}.csv`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      setExportState('done');
      setTimeout(() => setExportState('idle'), 3000);
    } catch (e) {
      console.error('Export failed:', e);
      setExportState('idle');
    }
  }

  return (
    <div className="dash-screen">
      {/* Hero band */}
      <div className="dash-hero">
        <div className="dash-hero-inner">
          <div className="dash-hero-top">
            <div>
              <div className="dash-title">Přehled vašich souborů</div>
              <div className="dash-subtitle">Prohlédnuto {totalFiles} souborů · {new Date().toLocaleDateString('cs-CZ', { day:'numeric',month:'long',year:'numeric' })}</div>
            </div>
            <div className="dash-top-actions">
              {/* Učebna — bonus feature, outlined ghost, left-most */}
              <button className="btn-hero-ucebna" onClick={() => setScreen('ucebna')}>
                <span>✦</span> Učebna
                {unclassifiedCount > 0 && (
                  <span className="ucebna-badge">{unclassifiedCount}</span>
                )}
              </button>
              <button className="btn-hero-ghost"
                onClick={handleExport}
                disabled={exportState === 'loading'}>
                {exportState === 'loading' ? 'Exportuji…' : exportState === 'done' ? '✓ Exportováno' : 'Exportovat relaci'}
              </button>
              <button className="btn-hero-primary" onClick={() => setScreen('scanning')}>
                <span>↺</span> Znovu prohlédnout
              </button>
            </div>
          </div>
          <div className="stats-row">
            {[
              { label: 'Celkem souborů', val: totalFiles, sub: 'ze 3 disků' },
              { label: 'Celková velikost', val: `${totalGB} GB`, sub: '~180 MB měsíčně' },
              { label: 'Nalezeno klientů', val: customers, sub: 'z obsahu souborů' },
              { label: 'Duplikáty', val: dupes, sub: 'jen informativně', warn: true },
            ].map(s => (
              <div key={s.label} className="stat-card-hero">
                <div className="stat-label-hero">{s.label}</div>
                <div className="stat-value-hero">{s.val}</div>
                <div className={`stat-sub-hero ${s.warn ? 'warn' : ''}`}>{s.sub}</div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* OCR status — read-only status plus one explicit action */}
      <div className="dash-ocr-row">
        <div className="ocr-status">
          <span className="ocr-label">OCR</span>
          {ocrState === 'unknown' && <span className="ocr-value">Kontroluji…</span>}
          {ocrState === 'installed' && <span className="ocr-value ok">✓ Nainstalováno</span>}
          {ocrState === 'unavailable' && <span className="ocr-value off">OCR není nainstalován</span>}
          <span className="ocr-cached" title="Výsledky v mezipaměti">
            {ocrCached} v mezipaměti
          </span>
        </div>
        <div className="ocr-action">
          {ocrProgress && (
            <span className="ocr-progress">
              {ocrProgress.done}/{ocrProgress.total}
            </span>
          )}
          <button
            className="btn-ocr"
            onClick={handleOcr}
            disabled={ocrRun === 'running' || ocrState !== 'installed' || ocrCandidates.length === 0}
            title={
              ocrState !== 'installed'
                ? 'OCR není nainstalován'
                : ocrCandidates.length === 0
                  ? 'Nejsou žádné soubory vhodné pro OCR'
                  : 'Spustit OCR nad skenovanými soubory'
            }
          >
            {ocrRun === 'running'
              ? (ocrProgress ? `OCR… ${ocrProgress.done}/${ocrProgress.total}` : 'OCR…')
              : 'Spustit OCR'}
          </button>
          {ocrMsg && <span className={`ocr-msg ${ocrRun === 'error' || ocrRun === 'unavailable' ? 'bad' : ''}`}>{ocrMsg}</span>}
        </div>
      </div>

      {/* Body */}
      <div className="dash-body">
        {/* CTAs */}
        <div className="hero-ctas">
          <div className="cta-card restructure" onClick={() => setScreen('restructure')}>
            <div className="cta-card-content">
              <div className="cta-card-label">Hlavní akce</div>
              <div className="cta-card-title">Uspořádat</div>
              <div className="cta-card-desc">Roztřídit {totalFiles} souborů do logických složek podle vašich pravidel.</div>
            </div>
            <div className="cta-arrow">→</div>
          </div>
          <div className="cta-card backup" onClick={handleBackup}
            style={{ opacity: backupState === 'loading' ? 0.7 : 1,
                     cursor: backupState === 'loading' ? 'not-allowed' : 'pointer' }}>
            <div className="cta-card-content">
              <div className="cta-card-label">Chraňte svou práci</div>
              <div className="cta-card-title">Zálohovat</div>
              <div className="cta-card-desc">
                {backupState === 'idle' && 'Vytvoříme zálohu souborů — JSON manifest se seznamem všech nalezených souborů. Uložíme do AppData.'}
                {backupState === 'loading' && 'Vytváříme zálohu…'}
                {backupState === 'done' && (
                  <span>✓ Záloha uložena{backupPath && <><br /><small style={{ opacity: 0.7, wordBreak: 'break-all' }}>{backupPath}</small></>}</span>
                )}
                {backupState === 'error' && '⚠ Záloha selhala — zkuste to znovu'}
              </div>
            </div>
            <div className={`cta-arrow${backupState === 'idle' ? ' idle' : ''}`}>
              {backupState === 'loading' ? '…' : backupState === 'done' ? '✓' : '→'}
            </div>
          </div>
        </div>

        {/* Charts row */}
        <div className="charts-row">
          <div className="chart-card timeline-card">
            <div className="chart-title">
              Aktivita souborů v čase
              <span className="chart-tag">posledních {TIMELINE_YEARS} let ({timelineStartYear}–{timelineEndYear})</span>
            </div>
            <div className="timeline-content">
              <div className="timeline-outside" title="Soubory mimo zobrazené období">
                <div className="timeline-outside-count">{timelineOutsideCount}</div>
                <div className="timeline-outside-label">mimo období</div>
              </div>
              <div className="activity-bars">
                {VALS.map((v, i) => (
                  <div key={i} className="act-bar-wrap">
                    <div className="act-bar" style={{
                      height: `${(v / MAX) * 90}px`,
                      background: v === MAX ? 'linear-gradient(180deg,#FF1493,#FE865B)' : v > 25 ? 'var(--pink-soft)' : '#E8D4E8'
                    }} title={`${YEARS[i]}: ${v} souborů`} />
                    <div className="act-month">{YEARS[i]}</div>
                  </div>
                ))}
              </div>
            </div>
          </div>

          <div className="chart-card">
            <div className="chart-title">Soubory podle klientů</div>
            {topCustomers.length === 0 && (
              <div className="cust-row"><div className="cust-name" style={{ color: 'var(--ink3)' }}>Žádní klienti nenalezeni</div></div>
            )}
            {topCustomers.map(([name, count]) => (
              <div key={name} className="cust-row">
                <div className="cust-name">{name}</div>
                <div className="cust-bar-wrap"><div className="cust-bar" style={{ width: `${Math.round(count/maxCustCount*100)}%` }} /></div>
                <div className="cust-count">{count}</div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
