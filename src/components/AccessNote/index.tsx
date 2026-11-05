// src/components/AccessNote/index.tsx
// Short note on folders the scan could not open, and how to grant access.
// macOS: privacy settings (Desktop / Documents / Downloads prompts, Full
// Disk Access for external drives and cloud folders).

import { isMac } from '../../utils/platform';
import './AccessNote.css';

/** How to grant Full Disk Access on macOS. Shown on Home and after a scan. */
export function FullDiskAccessHint() {
  return (
    <span className="access-hint">
      Otevřete <strong>Nastavení systému → Soukromí a zabezpečení → Plný přístup k disku</strong>,
      zapněte FileIT (případně ho přidejte tlačítkem +) a FileIT spusťte znovu.
    </span>
  );
}

/** Banner after a scan: N folders were skipped for lack of permission. */
export function DeniedFoldersNote({ folders }: { folders: string[] }) {
  if (!folders.length) return null;
  const shown = folders.slice(0, 5);
  return (
    <div className="access-note" role="status">
      <div className="access-note-title">
        🔒 {folders.length === 1 ? '1 složku' : `${folders.length} složek`} se nepodařilo otevřít — chybí oprávnění.
        Zbytek skenování proběhl normálně.
      </div>
      <ul className="access-note-list">
        {shown.map((f) => <li key={f}><code>{f}</code></li>)}
        {folders.length > shown.length && <li>… a další ({folders.length - shown.length})</li>}
      </ul>
      <div className="access-note-help">
        {isMac()
          ? <>macOS chrání Plochu, Dokumenty, Stažené soubory, externí disky a cloudové složky. <FullDiskAccessHint /></>
          : <>Tyto složky jsou chráněné oprávněními systému. Pokud je potřebujete prohlédnout, zkontrolujte jejich oprávnění.</>}
      </div>
    </div>
  );
}
