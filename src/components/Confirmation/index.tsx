// src/components/Confirmation/index.tsx
import { useState } from 'react';
import { useStore } from '../../store';
import { api } from '../../utils/tauriApi';
import './Confirmation.css';

export function Confirmation() {
  const { setScreen, completedRun, setCompletedRun } = useStore();
  const [showRestoreModal, setShowRestoreModal] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const [toast, setToast] = useState('');

  const run = completedRun!;
  if (!run) return null;

  function showToast(msg: string) {
    setToast(msg);
    setTimeout(() => setToast(''), 3000);
  }

  function formatDate(iso: string) {
    return new Date(iso).toLocaleString('cs-CZ', {
      day: 'numeric', month: 'long', year: 'numeric',
      hour: '2-digit', minute: '2-digit',
    });
  }

  function formatDuration(secs: number) {
    if (secs < 60) return `${secs} s`;
    return `${Math.floor(secs / 60)} min ${secs % 60} s`;
  }

  function autoConfirmDaysLeft() {
    const diff = new Date(run.auto_confirm_at).getTime() - Date.now();
    return Math.max(0, Math.floor(diff / 86_400_000));
  }

  async function handleConfirm() {
    try {
      await api.confirmRestructure();
      setCompletedRun(null);
      showToast('✓ Uspořádání potvrzeno. Záloha bude smazána za 7 dní.');
      setTimeout(() => setScreen('home'), 1500);
    } catch (e) {
      showToast('Chyba při potvrzování. Zkuste to znovu.');
    }
  }

  async function handleRestore() {
    setRestoring(true);
    try {
      const restored = await api.restoreRestructure();
      setShowRestoreModal(false);
      setCompletedRun(null);
      showToast(`↶ Obnoveno ${restored} souborů na původní místa.`);
      setTimeout(() => setScreen('home'), 1500);
    } catch (e) {
      showToast('Chyba při obnově. Zkuste to znovu.');
    } finally {
      setRestoring(false);
    }
  }

  async function openFolder() {
    await api.openInExplorer(run.target_path);
  }

  return (
    <div className="conf-wrap">
      <span className="corner-tr">FILEIT · V1.0 · DONE</span>
      <span className="corner-bl">N°07 · potvrzení výsledku</span>

      <div className="conf-head-row">
        <div className="conf-check-icon">✓</div>
        <div>
          <div className="conf-eyebrow">Hotovo</div>
          <div className="conf-timestamp">
            {formatDate(run.completed_at)} · trvalo {formatDuration(run.duration_seconds)}
          </div>
        </div>
      </div>

      <h1 className="conf-title">
        Uspořádáno. Teď se <em>podívejte</em>, prosím.
      </h1>
      <p className="conf-sub">
        FileIT přesunul vaše soubory do nové struktury. Otevřete si cílovou složku, projděte několik klientů, a pokud jsou všechny na svém místě, vraťte se sem a potvrďte výsledek.
      </p>

      {/* Stat cards */}
      <div className="conf-stats">
        <div className="conf-stat-card">
          <div className="conf-stat-label">Přesunuto</div>
          <div className="conf-stat-val c1">{run.files_moved}</div>
          <div className="conf-stat-sub">souborů</div>
        </div>
        <div className="conf-stat-card">
          <div className="conf-stat-label">Klientů</div>
          <div className="conf-stat-val c2">{run.folders_created}</div>
          <div className="conf-stat-sub">nových složek</div>
        </div>
        <div className="conf-stat-card">
          <div className="conf-stat-label">Cílová složka</div>
          <div className="conf-stat-path">{run.target_path}</div>
        </div>
        <div className="conf-stat-card amber">
          <div className="conf-stat-label">Neznámé</div>
          <div className="conf-stat-val c4">{run.unknown_count}</div>
          <div className="conf-stat-sub">zvlášť ve složce</div>
        </div>
      </div>

      {/* Verification block */}
      <div className="conf-verify">
        <div className="conf-verify-head">
          <div className="conf-verify-icon">📂</div>
          <div>
            <div className="conf-verify-title">Jak ověřit, že je vše na svém místě</div>
            <div className="conf-verify-sub">Nic nepospíchejte — tohle okno tu na vás počká.</div>
          </div>
        </div>
        <div className="conf-steps">
          {[
            <>Otevřete cílovou složku v Průzkumníku — <code className="conf-path">{run.target_path}</code></>,
            'Vyberte si pár klientů, které znáte, a projděte jejich složky — jsou dokumenty tam, kde by měly být?',
            <><span className="conf-amber">Mrkněte do složky Neznámý klient</span> — možná něco poznáte a bude třeba to ručně přesunout.</>,
            <><strong>Vraťte se sem a rozhodněte</strong> — Potvrdit, nebo Vrátit zpět.</>,
          ].map((text, i) => (
            <div key={i} className="conf-step">
              <div className={`conf-step-num ${i === 3 ? 'pink' : ''}`}>{i + 1}</div>
              <div className="conf-step-text">{text}</div>
            </div>
          ))}
        </div>
        <button className="conf-open-folder" onClick={openFolder}>
          <span>📂</span> Otevřít cílovou složku
        </button>
      </div>

      {/* Backup active strip */}
      <div className="conf-backup-strip">
        <span>🛟</span>
        <div>
          <strong>Bezpečnostní záloha je stále aktivní.</strong>{' '}
          Záloha souborů je uložena v{' '}
          <code className="conf-backup-filename">
            {run.backup_zip_path.split('\\').pop()}
          </code>
          . Po potvrzení ji automaticky smažeme za 7 dní.
        </div>
      </div>

      {/* Decision cards */}
      <div className="conf-decisions">
        <div className="conf-decision confirm">
          <div className="conf-decision-head">
            <div className="conf-decision-icon ok">✓</div>
            <div className="conf-decision-title">Všechno je v pořádku</div>
          </div>
          <p className="conf-decision-desc">Soubory jsem zkontroloval/a a jsou přesně tam, kde mají být.</p>
          <button className="conf-decision-btn ok" onClick={handleConfirm}>
            Potvrdit uspořádání →
          </button>
        </div>

        <div className="conf-decision restore">
          <div className="conf-decision-head">
            <div className="conf-decision-icon undo">↶</div>
            <div className="conf-decision-title">Něco není v pořádku</div>
          </div>
          <p className="conf-decision-desc">Chci vrátit všechny soubory zpět na jejich původní místa.</p>
          <button className="conf-decision-btn undo" onClick={() => setShowRestoreModal(true)}>
            Vrátit soubory zpět ↶
          </button>
        </div>
      </div>

      {/* Persistence footer */}
      <div className="conf-footer">
        <div className="conf-persist">
          <span>⏱</span>
          <span>
            Toto okno se zobrazí i při příštím spuštění. Po{' '}
            <strong>{autoConfirmDaysLeft()} dnech</strong>{' '}
            se automaticky považuje za potvrzené.
          </span>
        </div>
        <span className="conf-home-link" onClick={() => setScreen('home')}>Zpět na domů</span>
      </div>

      {/* Restore confirmation modal */}
      {showRestoreModal && (
        <div className="modal-overlay" onClick={() => !restoring && setShowRestoreModal(false)}>
          <div className="modal-card restore-modal" onClick={e => e.stopPropagation()}>
            <div className="restore-modal-icon">↶</div>
            <div className="restore-modal-title">Vrátit všechny soubory zpět?</div>
            <div className="restore-modal-sub">
              Tato akce vrátí všech <strong>{run.files_moved} souborů</strong> přesně na jejich původní místa — před tím, než FileIT cokoliv přesunul. Celé uspořádání bude zrušeno.
            </div>
            <div className="restore-modal-actions">
              <button className="modal-btn-secondary" onClick={() => setShowRestoreModal(false)} disabled={restoring}>Zrušit</button>
              <button className="restore-btn-confirm" onClick={handleRestore} disabled={restoring}>
                {restoring ? 'Obnovuji…' : 'Ano, vrátit zpět'}
              </button>
            </div>
          </div>
        </div>
      )}

      <div className={`toast ${toast ? 'show' : ''}`}><div className="toast-dot" />{toast}</div>
    </div>
  );
}
