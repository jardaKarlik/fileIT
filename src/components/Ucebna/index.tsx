// src/components/Ucebna/index.tsx
// Učebna — pattern teaching screen.
// Users identify unclassified file patterns by their visual/metadata fingerprint.
// No filenames are ever shown. Interaction unit is a pattern (cluster), not a file.

import { useEffect, useState } from 'react';
import { useStore } from '../../store';
import { api } from '../../utils/tauriApi';
import { UnclassifiedPattern } from '../../types';
import './Ucebna.css';

const KNOWN_INSTITUTIONS = [
  'Česká spořitelna',
  'ČSOB',
  'Komerční banka',
  'Raiffeisenbank CZ',
  'Allianz',
  'ČPP',
];

const DOC_TYPES = [
  { value: 'bank_statement',    label: 'Výpis z účtu' },
  { value: 'bank_confirmation', label: 'Potvrzení banky' },
  { value: 'insurance_contract',label: 'Pojistná smlouva' },
  { value: 'tax_form',          label: 'Daňový formulář' },
  { value: 'contract',          label: 'Smlouva' },
  { value: 'invoice',           label: 'Faktura' },
  { value: 'other',             label: 'Jiné' },
];

interface TeachState {
  institution: string;
  docType: string;
  customInstitution: string;
  showCustom: boolean;
}

interface PendingPayload {
  patternId: string;
  institution: string;
  docType: string;
}

export function Ucebna() {
  const { setScreen } = useStore();
  const [patterns, setPatterns]       = useState<UnclassifiedPattern[]>([]);
  const [loading, setLoading]         = useState(true);
  const [activeId, setActiveId]       = useState<string | null>(null);
  const [teachState, setTeachState]   = useState<TeachState>({
    institution: '', docType: '', customInstitution: '', showCustom: false,
  });
  const [taught, setTaught]           = useState<Set<string>>(new Set());
  const [pending, setPending]         = useState<PendingPayload[]>([]);
  const [showRescan, setShowRescan]   = useState(false);
  const [uploading, setUploading]     = useState(false);
  const [uploadMsg, setUploadMsg]     = useState('');
  const [contributions, setContribs]  = useState(0);
  const [todayContribs, setTodayContribs] = useState(0);
  useEffect(() => {
    api.groupUnclassifiedPatterns()
      .then(ps => { setPatterns(ps); setLoading(false); })
      .catch(() => setLoading(false));
  }, []);

  // ESC key → go back to dashboard
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (activeId) closeTeachPanel();
        else setScreen('dashboard');
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [activeId]);

  function openTeachPanel(patternId: string) {
    setActiveId(patternId);
    setTeachState({ institution: '', docType: '', customInstitution: '', showCustom: false });
  }

  function closeTeachPanel() {
    setActiveId(null);
  }

  async function submitTeaching() {
    if (!activeId) return;
    const institution = teachState.showCustom
      ? teachState.customInstitution.trim()
      : teachState.institution;
    if (!institution || !teachState.docType) return;

    try {
      await api.submitTeaching(activeId, institution, teachState.docType);
      setTaught(prev => new Set([...prev, activeId]));
      setPending(prev => [...prev, { patternId: activeId, institution, docType: teachState.docType }]);
      setShowRescan(true);
      setTodayContribs(n => n + 1);
      setContribs(n => n + 1);
      closeTeachPanel();
    } catch {
      // ignore — user can retry
    }
  }

  async function handleUpload() {
    if (pending.length === 0) return;
    setUploading(true);
    setUploadMsg('');
    try {
      const payloads = pending.map(p => ({
        pdf_producer: patterns.find(pat => pat.pattern_id === p.patternId)?.pdf_producer ?? null,
        pdf_creator:  patterns.find(pat => pat.pattern_id === p.patternId)?.pdf_creator ?? null,
        color_buckets: [],
        logo_phash: null,
        institution: p.institution,
        doc_type: p.docType,
      }));
      const result = await api.uploadToCm(payloads);
      setUploadMsg(
        result.uploaded > 0
          ? `✓ ${result.uploaded} vzorů odesláno do CM`
          : `${result.queued} vzorů zařazeno do fronty`
      );
      if (result.uploaded > 0) setPending([]);
    } catch {
      setUploadMsg('Odeslání selhalo — vzory jsou uloženy lokálně');
    } finally {
      setUploading(false);
    }
  }

  const activePattern = patterns.find(p => p.pattern_id === activeId);

  if (loading) {
    return (
      <div className="ucebna-loading">
        <div className="spinner" />
        <p>Načítání vzorů…</p>
      </div>
    );
  }

  if (patterns.length === 0) {
    return (
      <div className="ucebna-empty">
        <button className="ucebna-back-btn" onClick={() => setScreen('dashboard')} style={{ marginBottom: 16 }}>
          ← Zpět na přehled
        </button>
        <div className="ucebna-empty-icon">✓</div>
        <h2>Všechny soubory jsou klasifikovány</h2>
        <p>Žádné neznámé vzory k rozpoznání.</p>
      </div>
    );
  }

  return (
    <div className="ucebna">
      {/* ── Header ─────────────────────────────────────────────────────────── */}
      <div className="ucebna-header">
        <button
          className="ucebna-back-btn"
          onClick={() => setScreen('dashboard')}
          title="Zpět na přehled (ESC)"
        >
          ← Zpět na přehled
        </button>
        <div className="ucebna-header-center">
          <h1 className="ucebna-title">Učebna</h1>
          <p className="ucebna-subtitle">
            {taught.size > 0
              ? `${taught.size} rozpoznáno z ${patterns.length} vzorů`
              : 'Pomáháte, jak chcete a kdy chcete'}
          </p>
        </div>
        <div className="ucebna-header-right">
          <div className="ucebna-counter">
            <span className="counter-total">{contributions}</span>
            <span className="counter-label">příspěvků celkem</span>
            {todayContribs > 0 && (
              <span className="counter-today">+{todayContribs} dnes</span>
            )}
          </div>
          <button
            className="ucebna-close-btn"
            onClick={() => setScreen('dashboard')}
            title="Zavřít (ESC)"
          >
            ✕
          </button>
        </div>
      </div>

      {/* ── Rescan banner ───────────────────────────────────────────────────── */}
      {showRescan && (
        <div className="ucebna-rescan-banner">
          <span>Naučili jste CM {taught.size} nových vzorů.</span>
          <button className="btn-rescan" onClick={() => window.location.reload()}>
            Spustit re-sken
          </button>
        </div>
      )}

      <div className="ucebna-layout">
        {/* ── Pattern cards ─────────────────────────────────────────────────── */}
        <div className="ucebna-cards">
          {patterns.map(pat => (
            <div
              key={pat.pattern_id}
              className={`pattern-card ${taught.has(pat.pattern_id) ? 'taught' : ''} ${activeId === pat.pattern_id ? 'active' : ''}`}
              onClick={() => !taught.has(pat.pattern_id) && openTeachPanel(pat.pattern_id)}
            >
              {/* Schematic thumbnail */}
              <div className="pattern-thumb">
                <div
                  className="thumb-header"
                  style={{
                    background: pat.thumbnail_colors[0]
                      ? colorBucketToHex(pat.thumbnail_colors[0])
                      : '#e0e0e0',
                  }}
                />
                <div className="thumb-lines">
                  {[...Array(4)].map((_, i) => (
                    <div key={i} className="thumb-line" style={{ width: `${65 + i * 8}%` }} />
                  ))}
                </div>
              </div>

              {/* Card body */}
              <div className="pattern-body">
                <div className="pattern-file-count">
                  {pat.file_count} souborů sdílí tento vzor
                </div>
                <div className="pattern-signals">
                  {pat.signals.map((sig, i) => (
                    <span key={i} className={`signal-pill signal-${sig.strength}`}>
                      {sig.label}
                    </span>
                  ))}
                </div>
                {pat.thumbnail_colors.length > 0 && (
                  <div className="pattern-colors">
                    {pat.thumbnail_colors.map((c, i) => (
                      <span
                        key={i}
                        className="color-swatch"
                        style={{ background: colorBucketToHex(c) }}
                        title={c}
                      />
                    ))}
                  </div>
                )}
                {taught.has(pat.pattern_id) ? (
                  <div className="pattern-taught-badge">✓ Naučeno</div>
                ) : (
                  <button className="btn-teach">Rozpoznat</button>
                )}
              </div>
            </div>
          ))}
        </div>

        {/* ── Teach panel ───────────────────────────────────────────────────── */}
        {activeId && activePattern && (
          <div className="teach-panel">
            <div className="teach-panel-header">
              <h3>Rozpoznat vzor</h3>
              <button className="btn-close" onClick={closeTeachPanel}>✕</button>
            </div>

            <p className="teach-hint">
              {activePattern.file_count} souborů — vyberte instituce
            </p>

            {/* Privacy note */}
            <div className="privacy-note">
              Do CM odejde pouze: producent PDF, hash loga, barevná paleta —
              žádný název ani obsah souboru
            </div>

            {/* Institution picker */}
            <div className="teach-section">
              <label className="teach-label">Instituce</label>
              <div className="institution-grid">
                {KNOWN_INSTITUTIONS.map(inst => (
                  <button
                    key={inst}
                    className={`inst-btn ${teachState.institution === inst && !teachState.showCustom ? 'selected' : ''}`}
                    onClick={() => setTeachState(s => ({ ...s, institution: inst, showCustom: false }))}
                  >
                    {inst}
                  </button>
                ))}
                <button
                  className={`inst-btn inst-btn-other ${teachState.showCustom ? 'selected' : ''}`}
                  onClick={() => setTeachState(s => ({ ...s, showCustom: true, institution: '' }))}
                >
                  Jiná…
                </button>
              </div>
              {teachState.showCustom && (
                <input
                  className="custom-inst-input"
                  placeholder="Název instituce"
                  value={teachState.customInstitution}
                  onChange={e => setTeachState(s => ({ ...s, customInstitution: e.target.value }))}
                  autoFocus
                />
              )}
            </div>

            {/* Document type */}
            <div className="teach-section">
              <label className="teach-label">Instituce</label>
              <div className="doctype-list">
                {DOC_TYPES.map(dt => (
                  <button
                    key={dt.value}
                    className={`doctype-btn ${teachState.docType === dt.value ? 'selected' : ''}`}
                    onClick={() => setTeachState(s => ({ ...s, docType: dt.value }))}
                  >
                    {dt.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Actions */}
            <div className="teach-actions">
              <button className="btn-skip" onClick={closeTeachPanel}>
                Přeskočit
              </button>
              <button
                className="btn-confirm"
                disabled={
                  !teachState.docType ||
                  (teachState.showCustom
                    ? !teachState.customInstitution.trim()
                    : !teachState.institution)
                }
                onClick={submitTeaching}
              >
                Potvrdit
              </button>
            </div>
          </div>
        )}
      </div>

      {/* ── CM upload section ──────────────────────────────────────────────── */}
      {pending.length > 0 && (
        <div className="ucebna-upload">
          <div className="upload-count">{pending.length} vzorů připraveno k odeslání</div>
          <button
            className="btn-upload"
            onClick={handleUpload}
            disabled={uploading}
          >
            {uploading ? 'Odesílám…' : 'Přispět do CM'}
          </button>
          {uploadMsg && <span className="upload-msg">{uploadMsg}</span>}
        </div>
      )}
    </div>
  );
}

// Convert a color bucket string like "H220-240/S80+" to a display hex color
function colorBucketToHex(bucket: string): string {
  const hMatch = bucket.match(/H(\d+)-(\d+)/);
  if (!hMatch) return '#cccccc';
  const hue = (parseInt(hMatch[1]) + parseInt(hMatch[2])) / 2;
  return `hsl(${hue}, 60%, 55%)`;
}
