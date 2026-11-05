// src/utils/platform.ts
// Platform facts from the Rust side (OS, path separator, default target
// folder, file-manager name), loaded once before the first render (see
// main.tsx) and then read synchronously. Browser dev mode and any failure
// fall back to the Windows values the UI always used.

import { api } from './tauriApi';
import type { PlatformInfo } from '../types';

export const WINDOWS_DEFAULTS: PlatformInfo = {
  os: 'windows',
  path_separator: '\\',
  default_target_path: 'C:\\ReOrganized',
  file_manager: 'Průzkumník',
  home_dir: null,
};

let current: PlatformInfo = WINDOWS_DEFAULTS;

/** Resolves once platform info is known. Never rejects. */
export const platformReady: Promise<PlatformInfo> = api
  .platformInfo()
  .then((info) => (current = { ...WINDOWS_DEFAULTS, ...info }))
  .catch((e) => {
    console.warn('platform_info failed, assuming Windows:', e);
    return current;
  });

export function platform(): PlatformInfo {
  return current;
}

export const isMac = () => current.os === 'macos';
export const isWindows = () => current.os === 'windows';

/** Join path parts with the platform separator. */
export function joinPath(base: string, ...parts: string[]): string {
  const sep = current.path_separator;
  const trimmed = base.replace(/[\\/]+$/, '');
  return [trimmed, ...parts].join(sep);
}

/** Last component of a Windows or POSIX path. */
export function baseName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts.length ? parts[parts.length - 1] : path;
}

/** "v Průzkumníku" / "ve Finderu" / "ve správci souborů". */
export function inFileManager(): string {
  switch (current.os) {
    case 'windows': return 'v Průzkumníku';
    case 'macos': return 've Finderu';
    default: return 've správci souborů';
  }
}

/**
 * Example paths in help screens are written Windows-style
 * ("C:\\FileIT\\Organized\\"); show them POSIX-style elsewhere
 * ("~/FileIT/Organized/").
 */
export function illustrativePath(windowsPath: string): string {
  if (current.os === 'windows') return windowsPath;
  return windowsPath.replace(/^[A-Za-z]:\\/, '~/').replace(/\\/g, '/');
}
