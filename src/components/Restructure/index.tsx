// src/components/Restructure/index.tsx
// FIXED: Mode 2 Workflow Bug
// - Check if analysis exists before showing Mode 2
// - Show warning modal if no/old analysis
// - Use real analysis data instead of MOCK_FILES
// - Display analysis age in UI

import React, { useMemo, useState, useRef, useEffect } from 'react';
import { useStore } from '../../store';
import { api } from '../../utils/tauriApi';
import { analyzeAnalysisState, sample3RandomBatches } from '../../utils/analysisUtils';
import { GroupDimension, DetectedCloud, ClassifiedFile } from '../../types';
import { AnalysisWarningModal } from './AnalysisWarningModal';
import './Restructure.css';

const DIMENSIONS: Record<GroupDimension, { name: string; icon: string; iconClass: string; coverageText: string; coverage: number }> = {
  customer: { name: 'Jméno klienta',       icon: '👤', iconClass: 'i-customer', coverageText: '',                    coverage: 1.0 },
  date:     { name: 'Měsíc a rok',         icon: '📅', iconClass: 'i-date',     coverageText: '',                    coverage: 1.0 },
  inst:     { name: 'Vydávající instituce', icon: '🏦', iconClass: 'i-inst',     coverageText: '242 souborů (85 %)',  coverage: 0.85 },
  doc:      { name: 'Instituce',             icon: '📑', iconClass: 'i-doc',      coverageText: '198 souborů (70 %)',  coverage: 0.70 },
};

// ─────────────────────────────────────────────────────────────────────────────
// Mock file dataset for fallback/demo (284 files)
// ─────────────────────────────────────────────────────────────────────────────
const CUSTOMERS = [
  { name: 'Novák Pavel', n: 22 }, { name: 'Horáková Jana', n: 18 }, { name: 'Müller Thomas', n: 15 },
  { name: 'Svoboda Lucie', n: 14 }, { name: 'Kratochvíl Petr', n: 12 }, { name: 'Dvořák Martin', n: 11 },
  { name: 'Benešová Eva', n: 10 }, { name: 'Pokorný Jan', n: 9 }, { name: 'Veselá Hana', n: 9 },
  { name: 'Hájek Tomáš', n: 8 }, { name: 'Procházka Josef', n: 8 },
  ...Array.from({ length: 27 }, (_, i) => ({ name: `Klient ${i + 12}`, n: Math.max(2, 7 - i) })),
];
const MONTHS = ['2025-09','2025-10','2025-11','2026-01','2026-02','2026-03','2026-04'];
const INSTS  = ['ČEZ','Česká spořitelna','Kooperativa','Finanční úřad','ČSOB','Allianz','ÚSTŠ'];
const DOCS   = ['Smlouva','Faktura','Vyúčtování','Doklad','Dopis'];

const MOCK_FILES = (() => {
  const files: Array<{ customer: string | null; date: string; inst: string | null; doc: string | null }> = [];
  CUSTOMERS.forEach(c => {
    for (let i = 0; i < c.n; i++) {
      files.push({
        customer: c.name,
        date: MONTHS[(i + c.name.length) % MONTHS.length],
        inst: Math.random() < 0.85 ? INSTS[(i + c.name.length * 3) % INSTS.length] : null,
        doc:  Math.random() < 0.70 ? DOCS[(i + c.name.length * 7) % DOCS.length] : null,
      });
    }
  });
  while (files.length < 284) {
    const i = files.length;
    files.push({ customer: null, date: MONTHS[i % MONTHS.length], inst: INSTS[i % INSTS.length], doc: null });
  }
  return files;
})();

type FileObj = typeof MOCK_FILES[0];
type TreeNode = { count: number; children: Record<string, TreeNode> | null; isUnknown: boolean };

function groupBy(files: FileObj[], dims: GroupDimension[]): Record<string, TreeNode> {
  if (!dims.length) return {};
  const [dim, ...rest] = dims;
  const UNKNOWN: Record<GroupDimension, string> = {
    customer: 'Neznámý klient', date: 'Neurčeno', inst: 'Bez instituce', doc: 'Neurčený typ',
  };
  const groups: Record<string, FileObj[]> = {};
  files.forEach(f => {
    const v = f[dim] ?? UNKNOWN[dim];
    if (!groups[v]) groups[v] = [];
    groups[v].push(f);
  });
  const result: Record<string, TreeNode> = {};
  Object.keys(groups)
    .sort((a, b) => a === UNKNOWN[dim] ? 1 : b === UNKNOWN[dim] ? -1 : groups[b].length - groups[a].length)
    .forEach(k => {
      result[k] = {
        count: groups[k].length,
        children: rest.length ? groupBy(groups[k], rest) : null,
        isUnknown: k === UNKNOWN[dim],
      };
    });
  return result;
}

/**
 * Convert ClassifiedFile[] from scanResult into FileObj[] format
 * compatible with groupBy() logic.
 */
function convertScanResultToFileObjects(files: ClassifiedFile[]): FileObj[] {
  return files.map(f => ({
    customer: f.customer?.name ?? null,
    date: f.document_date
      ? new Date(f.document_date).toISOString().slice(0, 7)
      : 'Neurčeno',
    inst: f.institution?.canonical_name ?? null,
    doc: f.document_type === 'Unknown' ? null : (f.document_type as string),
  }));
}

// ─────────────────────────────────────────────────────────────────────────────
// Component
// ─────────────────────────────────────────────────────────────────────────────

export function Restructure() {
  const {
    setScreen, structure, addDimension, removeDimension, moveDimension,
    destinationMode, setDestinationMode, targetPath, setTargetPath,
    backupEnabled, setBackupEnabled, setPreview, detectedClouds, scanResult,
  } = useStore();

  // ──── ANALYSIS WARNING MODAL STATE ─────────────────────────────────────────
  const analysisState = useMemo(() => analyzeAnalysisState(scanResult), [scanResult]);
  const [showWarningModal, setShowWarningModal] = useState(analysisState.scenario !== 'has_analysis');
  const [useAnalysisData, setUseAnalysisData] = useState(analysisState.scenario === 'has_analysis' && !analysisState.isOldAnalysis);

  // When Restructure mounts, check if we should show warning
  useEffect(() => {
    if (analysisState.scenario !== 'has_analysis') {
      setShowWarningModal(true);
    } else {
      setShowWarningModal(false);
    }
  }, [analysisState.scenario]);

  // ──── MODAL ACTION HANDLERS ────────────────────────────────────────────────
  const handleWarningUseSampleData = () => {
    setUseAnalysisData(false);
    setShowWarningModal(false);
  };

  const handleWarningStartScan = () => {
    setScreen('scanning');
  };

  const handleWarningProceedAnyway = () => {
    setUseAnalysisData(true);
    setShowWarningModal(false);
  };

  // ──── FILE DATA SELECTION ──────────────────────────────────────────────────
  // Use real analysis data if available and user hasn't rejected it
  // Otherwise fall back to MOCK_FILES for demo
  const fileData = useMemo(() => {
    if (useAnalysisData && scanResult?.files && scanResult.files.length > 0) {
      // Sample 3 random batches from real data
      const sampled = sample3RandomBatches(scanResult.files, 200);
      return convertScanResultToFileObjects(sampled);
    }
    // Fallback to mock data
    return MOCK_FILES;
  }, [useAnalysisData, scanResult]);

  // ──── REST OF STATE ────────────────────────────────────────────────────────
  const [showCloudModal, setShowCloudModal] = useState(false);
  const [cloudModalVariant, setCloudModalVariant] = useState<'default' | 'list'>('default');
  const [selectedCloud, setSelectedCloud] = useState<DetectedCloud | null>(
    detectedClouds.find(c => c.is_running) ?? null
  );
  const [collisionWarning, setCollisionWarning] = useState(false);
  const [toast, setToast] = useState('');
  const toastTimer = useRef<ReturnType<typeof setTimeout>>();
  const [isBuilding, setIsBuilding] = useState(false);

  const primaryCloud = detectedClouds.find(c => c.is_running) ?? detectedClouds[0] ?? null;
  const c2cEnabled = detectedClouds.filter(c => c.is_running).length >= 2;

  // ── Live tree from file data (now potentially real analysis data) ───────────
  const tree = useMemo(() => groupBy(fileData, structure), [fileData, structure]);
  const topKeys   = Object.keys(tree);
  const normalKeys  = topKeys.filter(k => !tree[k].isUnknown);
  const unknownKeys = topKeys.filter(k => tree[k].isUnknown);
  const topFolders  = topKeys.length;
  const subfolders  = topKeys.reduce((acc, k) => acc + (tree[k].children ? Object.keys(tree[k].children!).length : 0), 0);
  void unknownKeys.reduce((acc, k) => acc + tree[k].count, 0);

  function showToast(msg: string) {
    setToast(msg);
    clearTimeout(toastTimer.current);
    toastTimer.current = setTimeout(() => setToast(''), 3000);
  }

  async function handlePickFolder() {
    const path = await api.pickFolder();
    if (path) {
      setTargetPath(path);
      setCollisionWarning(true); // simulate collision check
    }
  }

  function handleSelectDest(mode: 'local' | 'cloud' | 'c2c') {
    if (mode === 'local') {
      setDestinationMode({ type: 'local_only' });
      // Set default LOCAL-ONLY path when switching to local mode
      setTargetPath('C:\\ReOrganized');
    } else if (mode === 'cloud') {
      setShowCloudModal(true);
      setCloudModalVariant('default');
    }
    // c2c handled separately
  }

  async function confirmCloud(cloud: DetectedCloud) {
    setSelectedCloud(cloud);
    setDestinationMode({ type: 'local_plus_cloud', cloud_name: cloud.name });

    // For OneDrive: probe the actual sync root via registry/env first.
    // For other clouds: fall back to <root>\reOrganized.
    let path: string;
    if (cloud.name.toLowerCase().includes('onedrive')) {
      const detected = await api.getOneDrivePath().catch(() => null);
      path = detected ?? `${cloud.root_path}\\reOrganized`;
    } else {
      path = `${cloud.root_path}\\reOrganized`;
    }
    setTargetPath(path);
    setShowCloudModal(false);
    showToast(`Cíl: ${cloud.name} — ${path}`);
  }

  async function handleBuildPreview() {
    setIsBuilding(true);
    try {
      const preview = await api.buildPreview(
        structure,
        targetPath,
        destinationMode.type
      );
      setPreview(preview);
    } finally {
      setIsBuilding(false);
    }
  }

  async function handleRun() {
    await handleBuildPreview();
    setScreen('progress');
  }

  const isLocalPlusCloud = destinationMode.type === 'local_plus_cloud';
  const isLocal = destinationMode.type === 'local_only';

  // ──── RENDER WITH WARNING MODAL ────────────────────────────────────────────
  return (
    <>
      {/* WARNING MODAL */}
      {showWarningModal && (
        <AnalysisWarningModal
          scenario={analysisState.scenario === 'old_analysis' ? 'old_analysis' : 'no_analysis'}
          analysisAgeText={analysisState.ageText ?? undefined}
          onUseSampleData={handleWarningUseSampleData}
          onStartScan={handleWarningStartScan}
          onProceedAnyway={analysisState.isOldAnalysis ? handleWarningProceedAnyway : undefined}
        />
      )}

      {/* MAIN CONTENT */}
      <div className="rs-wrap">
        <span className="corner-tr">FILEIT · V1.0 · USPOŘÁDÁNÍ</span>
        <span className="corner-bl">N°05 · uspořádání</span>

        <div className="rs-eyebrow-row">
          <span className="eyebrow">Hlavní akce</span>
          <span className="rs-mark">N°05 · uspořádání</span>
        </div>
        <h2 className="rs-title">Jak to máme uspořádat?</h2>
        <p className="rs-sub">Vyberte, kam soubory poputují a jak je chcete seskupit. Nahlédnete, co to udělá, <em>než</em> cokoli přesuneme.</p>

        {/* SCAN STATUS BAR — always visible, 1-click load or re-scan */}
        <div className="scan-status-bar">
          {analysisState.hasAnalysis ? (
            useAnalysisData ? (
              <>
                <span className="scan-status-icon">✓</span>
                <span className="scan-status-text">
                  {`Analýza načtena — ${scanResult!.total_files} souborů${analysisState.ageText ? ` · ${analysisState.ageText}` : ''}`}
                </span>
                <button className="scan-status-action" onClick={handleWarningStartScan}>
                  ↺ Nový sken
                </button>
              </>
            ) : (
              <>
                <span className="scan-status-question">Použít tuto analýzu?</span>
                <span className="scan-status-text">
                  {`${analysisState.scanDateFormatted} · ${scanResult!.total_files} souborů`}
                  {analysisState.ageText && analysisState.ageText !== 'dnes' && (
                    <span className="scan-status-age"> · {analysisState.ageText}</span>
                  )}
                </span>
                <button className="scan-status-yn yes" onClick={handleWarningProceedAnyway}>ANO</button>
                <button className="scan-status-yn no" onClick={handleWarningStartScan}>NE</button>
              </>
            )
          ) : (
            <>
              <span className="scan-status-icon warn">⚠</span>
              <span className="scan-status-text">Žádná analýza — pro přesné uspořádání doporučujeme nejprve spustit sken</span>
              <button className="scan-status-action primary" onClick={handleWarningStartScan}>
                ↺ Spustit sken
              </button>
              <button className="scan-status-action" onClick={handleWarningUseSampleData}>
                Použít demo data
              </button>
            </>
          )}
        </div>

        {/* 1 / KAM */}
        <div className="rs-section-head">
          <span>1 / Kam</span>
        </div>
        <div className="dest-grid">
          <div className={`dest-card ${isLocal ? 'selected' : ''}`} onClick={() => handleSelectDest('local')}>
            <div className="dest-row">
              <div className="dest-icon local">💻</div>
              <div className="dest-name">Jen v tomto počítači</div>
              {isLocal && <div className="dest-check">✓</div>}
            </div>
            <div className="dest-desc">Uspořádáme lokálně. Nic neopustí váš disk.</div>
            {isLocal && (
              <div className="dest-path-row">
                <div className="dest-path-label">Cílová složka</div>
                <div className="dest-path-field">
                  <div className="dest-path-input">{targetPath}</div>
                  <button className="dest-browse" onClick={(e) => { e.stopPropagation(); handlePickFolder(); }}>Procházet…</button>
                </div>
              </div>
            )}
          </div>

          <div className={`dest-card ${isLocalPlusCloud ? 'selected' : ''}`} onClick={() => handleSelectDest('cloud')}>
            <div className="dest-row">
              <div className="dest-icon cloud">☁</div>
              <div className="dest-name">Lokálně + cloud</div>
              {isLocalPlusCloud && <div className="dest-check">✓</div>}
            </div>
            <div className="dest-desc">Uložíme do složky sledované klientem cloudu — ten zbytek zajistí sám.</div>
            {isLocalPlusCloud && selectedCloud ? (
              <div className="dest-path-row">
                <div className="dest-cloud-confirmed">
                  <div className="dest-cloud-mini-icon onedrive">☁</div>
                  <span className="dest-cloud-name-sm">{selectedCloud.name}</span>
                  <span className="dest-cloud-ready">· připraveno</span>
                </div>
                <div className="dest-path-field" style={{ marginTop: 6 }}>
                  <div className="dest-path-input">{targetPath}</div>
                  <button className="dest-browse" onClick={(e) => { e.stopPropagation(); handlePickFolder(); }}>Procházet…</button>
                </div>
              </div>
            ) : (
              primaryCloud && (
                <div className="dest-cloud-dot">
                  <div className="dot" />
                  <span>{primaryCloud.name} rozpoznán</span>
                </div>
              )
            )}
          </div>

          <div className={`dest-card ${!c2cEnabled ? 'disabled' : ''}`}
            title={!c2cEnabled ? 'Pro tento režim jsou potřeba dva cloudové klienti.' : undefined}
          >
            <div className="dest-row">
              <div className="dest-icon c2c">⇄</div>
              <div className="dest-name">Z cloudu do cloudu</div>
            </div>
            <div className="dest-desc">Přesuneme soubory mezi dvěma cloudovými úložišti.</div>
            {!c2cEnabled && (
              <div className="dest-disabled-reason">Rozpoznán jen jeden cloud. Pro tento režim jsou potřeba dva.</div>
            )}
          </div>
        </div>

        <p className="rs-tip">
          Tip: můžete zvolit libovolnou složku — existující klientský adresář, síťový disk, nebo nové místo. FileIT stávající soubory <strong>nepřepíše</strong>, jen do složky přidá uspořádané soubory.
        </p>

        {collisionWarning && (
          <div className="collision-warn">
            ⚠️
            <div>Tato složka už obsahuje soubory. FileIT vedle nich jen přidá nové klientské podsložky — <strong>vaše stávající soubory zůstanou nedotčeny</strong>.</div>
          </div>
        )}

        {/* 2 / JAK SESKUPIT */}
        <div className="rs-section-head" style={{ marginTop: 28 }}>
          <span>2 / Jak seskupit</span>
          <span className="rs-section-hint">Klikněte na dimenzi a přidejte ji do struktury vpravo. Maximum 3 úrovně.</span>
        </div>

        <div className="group-grid">
          <div className="palette-card">
            <div className="palette-label">Dostupné dimenze</div>
            <div className="palette-grid">
              {(Object.entries(DIMENSIONS) as [GroupDimension, typeof DIMENSIONS['customer']][]).map(([key, dim]) => {
                const usedIdx = structure.indexOf(key);
                const used = usedIdx >= 0;
                const maxed = structure.length >= 3 && !used;
                return (
                  <div key={key} className={`palette-chip ${used ? `used lvl${usedIdx + 1}` : ''}`}
                    onClick={() => !used && !maxed && addDimension(key)}
                  >
                    <div className={`chip-icon ${dim.iconClass}`}>{dim.icon}</div>
                    <div className="chip-text">
                      <div className="chip-name">{dim.name}</div>
                      <div className="chip-meta">
                        {used ? `Použito na úrovni ${usedIdx + 1}` : dim.coverageText || 'Dostupné pro všechny soubory'}
                      </div>
                    </div>
                    {used
                      ? <div className={`chip-num lvl${usedIdx + 1}`}>{usedIdx + 1}</div>
                      : maxed
                        ? <span className="chip-max">Max.</span>
                        : <button className="chip-add" onClick={(e) => { e.stopPropagation(); addDimension(key); }}>+ Přidat</button>
                    }
                  </div>
                );
              })}
            </div>
          </div>

          <div className="stack-card">
            <div className="stack-inner">
              <div className="stack-label">Vaše struktura</div>
              {structure.map((key, idx) => (
                <div key={key} className="stack-row">
                  <div className="stack-num">{idx + 1}</div>
                  <div className="stack-name">{DIMENSIONS[key].name}</div>
                  <div className="stack-arrows">
                    <button className="stack-arrow" disabled={idx === 0} onClick={() => moveDimension(key, -1)}>▲</button>
                    <button className="stack-arrow" disabled={idx === structure.length - 1} onClick={() => moveDimension(key, 1)}>▼</button>
                  </div>
                  <button className="stack-remove" onClick={() => removeDimension(key)}>×</button>
                </div>
              ))}
              {structure.length < 3 && (
                <div className="stack-empty">
                  <div className="stack-empty-text">
                    {structure.length === 0 ? 'Vyberte dimenzi z palety vlevo' : 'Přidejte další dimenzi pro další úroveň'}
                  </div>
                </div>
              )}
              <div className="stack-counter">{structure.length} / 3 úrovně použity</div>
            </div>
          </div>
        </div>

        {/* 3 / NÁHLED */}
        <div className="rs-section-head">
          <span>3 / Náhled</span>
          <span className="rs-section-hint">Takto to bude vypadat na vašem disku.</span>
        </div>

        <div className="preview-card">
          <div className="preview-stats">
            <div className="p-stat"><span className="p-stat-val c1">{fileData.length}</span><span className="p-stat-lbl">souborů</span></div>
            <div className="p-stat-sep" />
            <div className="p-stat"><span className="p-stat-val c2">{topFolders}</span><span className="p-stat-lbl">složek {structure[0] ? DIMENSIONS[structure[0]]?.name?.toLowerCase() : ''}</span></div>
            <div className="p-stat-sep" />
            <div className="p-stat"><span className="p-stat-val c3">{subfolders}</span><span className="p-stat-lbl">podsložek</span></div>
            <div className="preview-path">{targetPath}</div>
          </div>

          <div className="tree">
            {structure.length === 0 ? (
              <div className="tree-empty">Zatím žádná struktura — vyberte dimenzi z palety výše.</div>
            ) : (
              <>
                <div className="tree-root"><span>📁</span> Organized</div>
                <div className="tree-branch">
                  {normalKeys.slice(0, 5).map((k, i) => (
                    <React.Fragment key={k}>
                      <div className="tree-node">
                        <span className="tree-conn">├─</span>
                        <span>📁</span> {k}
                        <span className="tree-count">{tree[k].count} souborů</span>
                      </div>
                      {tree[k].children && i < 2 && (
                        <div className="tree-sub">
                          {Object.keys(tree[k].children!).slice(0, 3).map(sk => (
                            <div key={sk} className="tree-subnode">
                              <span className="tree-conn">├─</span>
                              <span>📁</span>{sk}
                              <span className="tree-count">{tree[k].children![sk].count}</span>
                            </div>
                          ))}
                        </div>
                      )}
                    </React.Fragment>
                  ))}
                  {normalKeys.length > 5 && (
                    <div className="tree-ellipsis">
                      <span className="tree-conn">├─</span> … a dalších {normalKeys.length - 5} složek
                    </div>
                  )}
                  {unknownKeys.map(k => (
                    <div key={k} className="tree-unknown">
                      <span className="tree-conn">└─</span><span>📁</span> {k}
                      <span className="tree-count">{tree[k].count} souborů · bez identifikace</span>
                    </div>
                  ))}
                </div>
              </>
            )}
          </div>
        </div>

        {/* BACKUP STRIP */}
        <div className={`backup-strip ${!backupEnabled ? 'off' : ''}`}>
          <div className="backup-icon">{backupEnabled ? '🛟' : '⚠'}</div>
          <div className="backup-text">
            <div className="backup-title">Bezpečnostní záloha — {backupEnabled ? 'zapnuto' : 'vypnuto'}</div>
            <div className="backup-desc">
              {backupEnabled
                ? <>Před jakýmkoli přesunem vytvoříme zálohu <code className="backup-filename">manifest_{new Date().toISOString().slice(0,10)}.json</code>. Cokoli se nepovede, vrátíme zpět jedním klikem.</>
                : <><strong>Bez zálohy je uspořádání jednosměrná akce.</strong> Pokud se něco nepovede, obnova souborů bude vyžadovat ruční práci.</>
              }
            </div>
          </div>
          <label className="backup-toggle-label">
            <div className={`toggle-pill ${!backupEnabled ? 'off' : ''}`} onClick={() => setBackupEnabled(!backupEnabled)}>
              <div className="toggle-thumb" />
            </div>
            <span>{backupEnabled ? 'Zapnuto' : 'Vypnuto'}</span>
          </label>
        </div>

        {/* FOOTER */}
        <div className="rs-footer">
          <button className="btn-ghost" onClick={() => setScreen('dashboard')}>← Zpět na přehled</button>
          <div className="footer-action">
            <div className="footer-summary">
              <div>Připraveno přesunout <strong>{fileData.length} souborů</strong></div>
              <div><small>do <strong>{topFolders + subfolders} nových složek</strong></small></div>
            </div>
            <button className="btn-run" onClick={handleRun} disabled={structure.length === 0 || isBuilding}>
              {isBuilding ? 'Připravuje se…' : 'Spustit uspořádání →'}
            </button>
          </div>
        </div>

        {/* CLOUD CONFIRMATION MODAL */}
        {showCloudModal && (
          <div className="modal-overlay" onClick={() => setShowCloudModal(false)}>
            <div className="modal-card" onClick={e => e.stopPropagation()}>
              {cloudModalVariant === 'default' ? (
                <>
                  <div className="modal-head">
                    <div className="modal-icon">☁</div>
                    <div>
                      <div className="modal-eyebrow">Potvrzení cíle</div>
                      <div className="modal-title">Použít {primaryCloud?.name ?? 'cloud'} pro zálohu?</div>
                      <div className="modal-sub">Nalezli jsme na vašem počítači klienta {primaryCloud?.name}. Uspořádané soubory uložíme do jeho sledované složky.</div>
                    </div>
                  </div>
                  {primaryCloud && (
                    <div className="modal-cloud-card">
                      <div className="modal-cloud-icon-wrap"><div className="cloud-brand onedrive">☁</div></div>
                      <div className="modal-cloud-text">
                        <div className="modal-cloud-name">{primaryCloud.name} {primaryCloud.is_running && <span className="badge-running"><span className="dot" />běží</span>}</div>
                        <div className="modal-cloud-path">{primaryCloud.root_path}</div>
                      </div>
                    </div>
                  )}
                  <div className="modal-actions">
                    <button className="modal-btn-primary" onClick={() => primaryCloud && confirmCloud(primaryCloud)}>Ano, použít {primaryCloud?.name}</button>
                    <div className="modal-actions-row">
                      <button className="modal-btn-secondary" onClick={() => { setShowCloudModal(false); setDestinationMode({ type: 'local_only' }); setTargetPath('C:\\ReOrganized'); }}>Ne, zrušit</button>
                      <button className="modal-btn-alt" onClick={() => setCloudModalVariant('list')}>Zvolit jiný cloud</button>
                    </div>
                  </div>
                </>
              ) : (
                <>
                  <div className="modal-head">
                    <div className="modal-icon">☁</div>
                    <div>
                      <div className="modal-eyebrow">Vyberte cloud</div>
                      <div className="modal-title">Který cloud má FileIT použít?</div>
                    </div>
                  </div>
                  <div className="modal-cloud-list">
                    {detectedClouds.map(cloud => (
                      <div key={cloud.name}
                        className={`cloud-list-item ${!cloud.is_running ? 'disabled' : ''} ${selectedCloud?.name === cloud.name ? 'selected' : ''}`}
                        onClick={() => cloud.is_running && setSelectedCloud(cloud)}
                      >
                        <div className="cloud-list-icon-wrap"><div className="cloud-brand onedrive">☁</div></div>
                        <div className="cloud-list-text">
                          <div className="cloud-list-name">{cloud.name} {!cloud.is_running && <span style={{ color: 'var(--ink3)', fontSize: 10 }}>· neběží</span>}</div>
                          <div className="cloud-list-path">{cloud.is_running ? cloud.root_path : 'klient nainstalovaný, ale neběží'}</div>
                        </div>
                        {selectedCloud?.name === cloud.name && <div className="cloud-list-check">✓</div>}
                      </div>
                    ))}
                  </div>
                  <div className="modal-footer-row">
                    <button className="modal-btn-back" onClick={() => setCloudModalVariant('default')}>← Zpět</button>
                    <button className="modal-btn-confirm" onClick={() => selectedCloud && confirmCloud(selectedCloud)}>
                      Pokračovat s {selectedCloud?.name ?? '…'}
                    </button>
                  </div>
                </>
              )}
            </div>
          </div>
        )}

        <div className={`toast ${toast ? 'show' : ''}`}><div className="toast-dot" />{toast}</div>
      </div>
    </>
  );
}
