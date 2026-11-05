// src/utils/scanRoots.ts
// Single source of truth for the folders a default scan covers.
//
// Default: every local fixed drive (C:\, D:\, …), recursively — people keep
// work folders like C:\Klienti outside Documents. The Rust scanner skips
// Windows, Program Files, AppData, hidden and cloud-only (online) files.
// Dev override: set VITE_FILEIT_SCAN_ROOT (e.g. C:\_dev\fileIT\sample-vault)
// in `.env.local` to scan only that folder.

import { api } from './tauriApi';

export interface ScanRoot {
  /** Czech label shown in the UI. */
  label: string;
  /** Absolute path; empty when unknown. */
  path: string;
}

const DEV_ROOT: string | undefined = import.meta.env.VITE_FILEIT_SCAN_ROOT;

/** Resolve the default scan roots. Never throws. */
export async function resolveDefaultScanRoots(): Promise<ScanRoot[]> {
  if (DEV_ROOT && DEV_ROOT.trim()) {
    return [{ label: 'Vývojová složka', path: DEV_ROOT.trim() }];
  }
  try {
    const drives = await api.defaultScanRoots();
    return drives.map((path) => ({ label: `Disk ${path}`, path }));
  } catch (e) {
    console.warn('Could not resolve local drives:', e);
    return [];
  }
}
