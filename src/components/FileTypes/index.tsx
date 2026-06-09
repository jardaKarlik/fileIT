// src/components/FileTypes/index.tsx

import { useStore } from '../../store';
import './FileTypes.css';

const CORE_TYPES = [
  { key: 'pdf',   icon: '📄', name: 'PDF dokumenty',    desc: 'Smlouvy, úřední dopisy, skenované doklady', core: true  },
  { key: 'word',  icon: '📝', name: 'Word dokumenty',   desc: 'Šablony, zprávy, korespondence, dohody',    core: true  },
  { key: 'excel', icon: '📊', name: 'Excel a tabulky',  desc: 'Finanční údaje, seznamy klientů, faktury',  core: true  },
  { key: 'image', icon: '🖼', name: 'Obrázky (JPG, BMP)', desc: 'Skenované doklady, fotky dokumentů',      core: true  },
  { key: 'email', icon: '📧', name: 'Uložené e-maily',  desc: 'Soubory .msg a .eml z pošty',               core: false },
  { key: 'text',  icon: '🗒', name: 'Text a poznámky',  desc: 'Textové soubory, poznámky',                 core: false },
];

const EXTRA = ['ZIP archivy','PowerPoint','XML / JSON','Audio záznamy','CSV data'];

export function FileTypes() {
  const { setScreen, setScanRoots, selectedCategories, toggleCategory } = useStore();

  async function handleStart() {
    if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
      try {
        const { documentDir, desktopDir, downloadDir } = await import('@tauri-apps/api/path');
        const docs      = await documentDir();
        const desktop   = await desktopDir();
        const downloads = await downloadDir();
        setScanRoots([docs, desktop, downloads]);
      } catch (e) {
        console.warn('Could not resolve path dirs:', e);
        setScanRoots([]);
      }
    }
    setScreen('scanning');
  }

  return (
    <div className="section-wrap">
      <div className="ft-eyebrow">Krok 1 ze 3</div>
      <h2 className="ft-title">Jaké soubory máme zkontrolovat?</h2>
      <p className="ft-sub">FileIT se zaměří na vaše důležité dokumenty — smlouvy, úřední dopisy a klientské složky. Tyto typy jsou vybrané automaticky. Další si můžete přidat níže.</p>

      <div className="ft-grid">
        {CORE_TYPES.map(t => (
          <div key={t.key} className={`ft-card ${selectedCategories.includes(t.key) ? 'selected' : ''}`}
            onClick={() => toggleCategory(t.key)}>
            <div className="ft-icon">{t.icon}</div>
            <div className="ft-name">{t.name}</div>
            <div className="ft-desc">{t.desc}</div>
            <span className={`ft-badge ${t.core ? 'core' : 'extra'}`}>{t.core ? 'Základ' : 'Volitelné'}</span>
          </div>
        ))}
      </div>

      <div className="ft-extra-label">Přidat další typy</div>
      <div className="ft-extra-row">
        {EXTRA.map(e => <div key={e} className="ft-extra-chip">+ {e}</div>)}
      </div>

      <div className="info-note" style={{ marginBottom: 24 }}>
        <span>🔒</span>
        <div>FileIT <strong>nikdy</strong> nenahrává vaše soubory nikam mimo váš počítač. Analýza obsahu probíhá výhradně lokálně.</div>
      </div>

      <div className="ft-actions">
        <button className="btn-ghost" onClick={() => setScreen('home')}>← Zpět</button>
        <button className="btn-run" onClick={handleStart}>Spustit prohlížení →</button>
      </div>
    </div>
  );
}
