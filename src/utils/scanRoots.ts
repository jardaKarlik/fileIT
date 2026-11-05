// src/utils/scanRoots.ts
// Single source of truth for the folders a default scan covers.
//
// Windows default: every local fixed drive (C:\, D:\, …), recursively — people keep
// work folders like C:\Klienti outside Documents. The Rust scanner skips
// Windows, Program Files, AppData, hidden and cloud-only (online) files.
// macOS default: the home folder plus cloud folders in ~/Library/CloudStorage
// and iCloud Drive (~/Library itself is skipped).
// Dev override: set VITE_FILEIT_SCAN_ROOT (e.g. C:\_dev\fileIT\sample-vault)
// in `.env.local` to scan only that folder.

import { api } from './tauriApi';
import { baseName, isWindows, platform } from './platform';

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
    const roots = await api.defaultScanRoots();
    return roots.map((path) => ({ label: scanRootLabel(path), path }));
  } catch (e) {
    console.warn('Could not resolve default scan roots:', e);
    return [];
  }
}

/** Czech label for a default scan root. */
export function scanRootLabel(path: string): string {
  if (isWindows()) return `Disk ${path}`;
  const home = platform().home_dir;
  if (home && path.replace(/\/+$/, '') === home.replace(/\/+$/, '')) return 'Domovská složka';
  if (path.includes('com~apple~CloudDocs')) return 'iCloud Drive';
  if (path.includes('/Library/CloudStorage/')) return `Cloud ${baseName(path)}`;
  return path;
}
