// src/components/Dashboard/index.tsx

import { useState } from 'react';
import { useStore } from '../../store';
import { api } from '../../utils/tauriApi';
import './Dashboard.css';

// Helper to get years and file counts for timeline
function getYearsAndCounts(files: any[]) {
  const years: string[] = [];
  const vals: number[] = [];

  // Create map to count files by year
  const yearCounts: { [key: number]: number } = {};

  // Count files by year from document_date or modified date
  files.forEach(file => {
    try {
      const dateStr = file.document_date || file.record?.modified;
      if (!dateStr) return;

      const date = new Date(dateStr);
      if (isNaN(date.getTime())) return;

      const year = date.getFullYear();
      yearCounts[year] = (yearCounts[year] ?? 0) + 1;
    } catch (e) {
      // Ignore parsing errors
    }
  });

  // Sort years and build display arrays
  const sortedYears = Object.keys(yearCounts)
    .map(Number)
    .sort((a, b) => a - b);

  sortedYears.forEach(year => {
    years.push(String(year));
    vals.push(yearCounts[year]);
  });

  // If no years found, return empty arrays
  if (years.length === 0) {
    return { years: [], vals: [] };
  }

  return { years, vals };
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

  const totalFiles = scanResult?.total_files ?? 0;
  const totalGB   = ((scanResult?.total_size_bytes ?? 0) / 1e9).toFixed(1).replace('.', ',');
  const customers = scanResult?.customers_found ?? 0;
  const dupes     = scanResult?.duplicates_found ?? 0;

  // Count unclassified files for the Učebna badge
  const unclassifiedCount = scanResult?.files
    ? scanResult.files.filter(f => !f.institution || f.confidence < 0.65).length
    : 0;

  // Get years and file counts for timeline
  const { years: YEARS, vals: VALS } = getYearsAndCounts(scanResult?.files ?? []);
  const MAX = Math.max(...VALS, 1);

  const topCustomers = getTopCustomers(scanResult?.files ?? []);
  const maxCustCount = topCustomers.length > 0 ? topCustomers[0][1] : 1;

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
            <div className="cta-arrow">
              {backupState === 'loading' ? '…' : backupState === 'done' ? '✓' : '→'}
            </div>
          </div>
        </div>

        {/* Charts row */}
        <div className="charts-row">
          <div className="chart-card">
            <div className="chart-title">Aktivita souborů v čase <span className="chart-tag">podle roku vzniku</span></div>
            <div className="activity-bars">
              {VALS.map((v,i) => (
                <div key={i} className="act-bar-wrap">
                  <div className="act-bar" style={{
                    height: `${(v/MAX)*90}px`,
                    background: v===MAX ? 'linear-gradient(180deg,#FF1493,#FE865B)' : v>25 ? 'var(--pink-soft)' : '#E8D4E8'
                  }} title={`${YEARS[i]}: ${v} souborů`} />
                  <div className="act-month">{YEARS[i]}</div>
                </div>
              ))}
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
