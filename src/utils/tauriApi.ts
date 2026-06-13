// src/utils/tauriApi.ts
// Thin wrappers around Tauri invoke() calls.
// When running in a browser (dev without Tauri), falls back to mock data
// so the UI can be developed and iterated without a compiled backend.

import {
  CmPayload, CmUpdateResult, CompletedRun, DetectedCloud,
  RestructurePreview, RestructureResult, ScanResult,
  TeachingResult, UnclassifiedPattern, UploadResult,
} from '../types';

// Detect whether we are inside a Tauri window
const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (IS_TAURI) {
    const { invoke: tauriInvoke } = await import('@tauri-apps/api/core');
    return tauriInvoke<T>(cmd, args);
  }
  // Dev mock — returns plausible data so the UI renders without a backend
  return devMock(cmd, args) as T;
}

async function listen(event: string, cb: (payload: unknown) => void): Promise<() => void> {
  if (IS_TAURI) {
    const { listen: tauriListen } = await import('@tauri-apps/api/event');
    const unlisten = await tauriListen(event, (e) => cb(e.payload));
    return unlisten;
  }
  // No-op in dev
  return () => {};
}

// ─────────────────────────────────────────────────────────────────────────────
// API surface
// ─────────────────────────────────────────────────────────────────────────────

export const api = {
  /** Called on app start. Returns pending CompletedRun if one exists. */
  appInit: () => invoke<CompletedRun | null>('app_init'),

  /** Detect installed cloud sync clients. */
  detectClouds: () => invoke<DetectedCloud[]>('detect_clouds'),

  /** Open native folder picker. Returns chosen path or null. */
  pickFolder: () => invoke<string | null>('pick_folder'),

  /** Open a path in Windows Explorer. */
  openInExplorer: (path: string) => invoke<void>('open_in_explorer', { path }),

  /** Detect the OneDrive sync root on Windows. Returns null when not found. */
  getOneDrivePath: () => invoke<string | null>('get_onedrive_path'),

  /** Start a scan. Progress events come via onScanProgress listener. */
  startScan: (roots: string[], fileCategories: string[]) =>
    invoke<ScanResult>('start_scan', { request: { roots, file_categories: fileCategories } }),

  /** Build a restructure preview without moving files. */
  buildPreview: (structure: string[], targetPath: string, destinationMode: string) =>
    invoke<RestructurePreview>('build_preview', {
      request: { structure, target_path: targetPath, destination_mode: destinationMode },
    }),

  /** Execute the restructure. Progress events come via onRestructureProgress. */
  runRestructure: (backupEnabled: boolean) =>
    invoke<RestructureResult>('run_restructure', { request: { backup_enabled: backupEnabled } }),

  /** Create a standalone backup manifest of all scanned files. Returns manifest path. */
  createStandaloneBackup: () => invoke<string>('create_standalone_backup'),

  /** Confirm the restructure result. */
  confirmRestructure: () => invoke<void>('confirm_restructure'),

  /** Restore all files to original locations. */
  restoreRestructure: () => invoke<number>('restore_restructure'),

  /** Check for CM registry update on launch. Silently falls back if offline. */
  checkCmUpdate: () => invoke<CmUpdateResult>('check_cm_update'),

  /** Group unclassified files into pattern clusters for Učebna. */
  groupUnclassifiedPatterns: () => invoke<UnclassifiedPattern[]>('group_unclassified_patterns'),

  /** Record a teaching decision for a pattern. Returns what would be sent to CM. */
  submitTeaching: (patternId: string, institution: string, docType: string) =>
    invoke<TeachingResult>('submit_teaching', { patternId, institution, docType }),

  /** Upload teaching payloads to CM cloud. Requires explicit user confirmation. */
  uploadToCm: (payloads: CmPayload[]) =>
    invoke<UploadResult>('upload_to_cm', { payloads }),

  // ── Event listeners ───────────────────────────────────────────────────────

  onScanProgress: (cb: (p: { files_found: number; message: string; done: boolean }) => void) =>
    listen('scan_progress', cb as any),

  onClassifyProgress: (cb: (p: { done: number; total: number; current: string }) => void) =>
    listen('classify_progress', cb as any),

  onRestructureProgress: (cb: (p: {
    files_moved: number; total_files: number;
    current_file: string; log_line: string; done: boolean; error: string | null;
  }) => void) => listen('restructure_progress', cb as any),

  onRestoreProgress: (cb: (p: {
    restored: number; total: number; log_line: string; done: boolean;
  }) => void) => listen('restore_progress', cb as any),
};

// ─────────────────────────────────────────────────────────────────────────────
// Dev mocks — used when running in browser without Tauri
// ─────────────────────────────────────────────────────────────────────────────

function devMock(cmd: string, _args?: Record<string, unknown>): unknown {
  switch (cmd) {
    case 'app_init':
      return null;

    case 'detect_clouds':
      return [
        { name: 'OneDrive', root_path: 'C:\\Users\\Anna\\OneDrive', is_running: true },
        { name: 'Dropbox', root_path: 'C:\\Users\\Anna\\Dropbox', is_running: false },
      ] satisfies DetectedCloud[];

    case 'get_onedrive_path':
      return 'C:\\Users\\Dev\\OneDrive\\Documents\\reOrganized';

    case 'pick_folder':
      return 'C:\\Klienti\\2026\\';

    case 'open_in_explorer':
      console.log('[dev] open_in_explorer called');
      return undefined;

    case 'start_scan':
      return {
        total_files: 284,
        total_size_bytes: 2_576_351_232,
        customers_found: 38,
        duplicates_found: 24,
        files: [],
      } satisfies ScanResult;

    case 'build_preview':
      return {
        total_files: 284,
        total_folders: 38,
        total_subfolders: 127,
        unknown_count: 11,
        duplicate_count: 24,
        target_path: 'C:\\FileIT\\Organized\\',
        planned_moves: [],
        target_has_existing_files: false,
        existing_file_count: 0,
      } satisfies RestructurePreview;

    case 'run_restructure':
      return {
        session_id: 'dev-session-001',
        files_moved: 273,
        folders_created: 165,
        unknown_count: 11,
        duration_seconds: 252,
        backup_manifest_path: 'C:\\AppData\\FileIT\\backups\\manifest_2026-04-17_143300.json',
        target_path: 'C:\\FileIT\\Organized\\',
      } satisfies RestructureResult;

    case 'create_standalone_backup':
      return 'C:\\Users\\dev\\AppData\\Roaming\\cz.fileit.app\\backups\\inventory_2026-06-12_120000.json';

    case 'confirm_restructure':
    case 'restore_restructure':
      return undefined;

    case 'check_cm_update':
      return { updated: false, version: '2026-05-06', fingerprint_count: 4 } satisfies CmUpdateResult;

    case 'group_unclassified_patterns':
      return [
        {
          pattern_id: 'p_Scriptura_XSL_FO_R8_5',
          file_count: 14,
          signals: [
            { label: 'Producent: Scriptura XSL-FO R8.5', strength: 'strong' },
            { label: 'Velikost: S', strength: 'weak' },
          ],
          thumbnail_colors: ['H220-240/S80+'],
          pdf_producer: 'Scriptura XSL-FO R8.5',
          pdf_creator: null,
          size_bucket: 'S',
        },
        {
          pattern_id: 'p_size_M',
          file_count: 3,
          signals: [{ label: 'Velikost: M', strength: 'weak' }],
          thumbnail_colors: [],
          pdf_producer: null,
          pdf_creator: null,
          size_bucket: 'M',
        },
      ] satisfies UnclassifiedPattern[];

    case 'submit_teaching':
      return {
        local_fingerprint_added: true,
        cm_payload: {
          pdf_producer: 'Scriptura XSL-FO R8.5',
          pdf_creator: null,
          color_buckets: [],
          logo_phash: null,
          institution: 'Raiffeisenbank CZ',
          doc_type: 'bank_statement',
        },
      } satisfies TeachingResult;

    case 'upload_to_cm':
      return { uploaded: 1, queued: 0 } satisfies UploadResult;

    default:
      console.warn('[dev] Unhandled mock command:', cmd);
      return null;
  }
}
