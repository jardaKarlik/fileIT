// src/utils/scanRoots.ts
// Single source of truth for the folders a default scan covers.
//
// Default: the user's Documents, Desktop and Downloads folders, recursively
// (system, program and hidden folders are skipped by the Rust scanner).
// Dev override: set VITE_FILEIT_SCAN_ROOT (e.g. C:\_dev\fileIT\sample-vault)
// in `.env.local` to scan only that folder.

const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export interface ScanRoot {
  /** Czech label shown in the UI. */
  label: string;
  /** Absolute path; empty in browser dev mode. */
  path: string;
}

const DEV_ROOT: string | undefined = import.meta.env.VITE_FILEIT_SCAN_ROOT;

/** Resolve the default scan folders. Never throws. */
export async function resolveDefaultScanRoots(): Promise<ScanRoot[]> {
  if (DEV_ROOT && DEV_ROOT.trim()) {
    return [{ label: 'Vývojová složka', path: DEV_ROOT.trim() }];
  }
  if (!IS_TAURI) {
    return [
      { label: 'Dokumenty', path: '' },
      { label: 'Plocha', path: '' },
      { label: 'Stažené soubory', path: '' },
    ];
  }
  const { documentDir, desktopDir, downloadDir } = await import('@tauri-apps/api/path');
  const entries: [string, () => Promise<string>][] = [
    ['Dokumenty', documentDir],
    ['Plocha', desktopDir],
    ['Stažené soubory', downloadDir],
  ];
  const roots: ScanRoot[] = [];
  for (const [label, resolve] of entries) {
    try {
      roots.push({ label, path: await resolve() });
    } catch (e) {
      console.warn(`Could not resolve ${label} folder:`, e);
    }
  }
  return roots;
}
