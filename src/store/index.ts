// src/store/index.ts
// Zustand global store for FileIT.
// All screens read from and write to this single store.

import { create } from 'zustand';
import {
  CompletedRun, DetectedCloud, GroupDimension,
  RestructurePreview, ScanResult, DestinationMode,
} from '../types';

// ─────────────────────────────────────────────────────────────────────────────
// Screen routing
// ─────────────────────────────────────────────────────────────────────────────

export type Screen =
  | 'home'
  | 'how_it_works'
  | 'file_types'
  | 'scanning'
  | 'dashboard'
  | 'restructure'
  | 'progress'
  | 'confirmation'
  | 'ucebna';

// ─────────────────────────────────────────────────────────────────────────────
// Store shape
// ─────────────────────────────────────────────────────────────────────────────

interface FileITState {
  // ── Routing ──────────────────────────────────────────────────────────────
  screen: Screen;
  setScreen: (s: Screen) => void;

  // ── Scan config ───────────────────────────────────────────────────────────
  scanRoots: string[];
  setScanRoots: (roots: string[]) => void;
  selectedCategories: string[];
  toggleCategory: (cat: string) => void;

  // ── Scan results ──────────────────────────────────────────────────────────
  scanResult: ScanResult | null;
  setScanResult: (r: ScanResult | null) => void;

  // ── Restructure config ────────────────────────────────────────────────────
  structure: GroupDimension[];
  setStructure: (s: GroupDimension[]) => void;
  addDimension: (dim: GroupDimension) => void;
  removeDimension: (dim: GroupDimension) => void;
  moveDimension: (dim: GroupDimension, delta: -1 | 1) => void;

  destinationMode: DestinationMode;
  setDestinationMode: (m: DestinationMode) => void;
  targetPath: string;
  setTargetPath: (p: string) => void;
  backupEnabled: boolean;
  setBackupEnabled: (v: boolean) => void;

  // ── Preview ───────────────────────────────────────────────────────────────
  preview: RestructurePreview | null;
  setPreview: (p: RestructurePreview | null) => void;

  // ── Progress ──────────────────────────────────────────────────────────────
  progressLines: string[];
  progressPct: number;
  progressCurrent: string;
  appendProgressLine: (line: string) => void;
  setProgressPct: (pct: number) => void;
  setProgressCurrent: (s: string) => void;
  resetProgress: () => void;

  // ── Confirmation ──────────────────────────────────────────────────────────
  completedRun: CompletedRun | null;
  setCompletedRun: (r: CompletedRun | null) => void;

  // ── Cloud detection ───────────────────────────────────────────────────────
  detectedClouds: DetectedCloud[];
  setDetectedClouds: (clouds: DetectedCloud[]) => void;

  // ── Last run (for Home screen) ────────────────────────────────────────────
  lastRunTimestamp: string | null;
  lastRunFileCount: number | null;
  setLastRun: (timestamp: string, fileCount: number) => void;
}

// ─────────────────────────────────────────────────────────────────────────────
// Default scan roots — Windows user folders
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// Store
// ─────────────────────────────────────────────────────────────────────────────

export const useStore = create<FileITState>((set) => ({
  // ── Routing ──────────────────────────────────────────────────────────────
  screen: 'home',
  setScreen: (screen) => set({ screen }),

  // ── Scan config ───────────────────────────────────────────────────────────
  scanRoots: [],
  setScanRoots: (roots) => set({ scanRoots: roots }),
  selectedCategories: ['pdf', 'word', 'excel', 'image'],
  toggleCategory: (cat) => set((s) => {
    const has = s.selectedCategories.includes(cat);
    return {
      selectedCategories: has
        ? s.selectedCategories.filter((c) => c !== cat)
        : [...s.selectedCategories, cat],
    };
  }),

  // ── Scan results ──────────────────────────────────────────────────────────
  scanResult: null,
  setScanResult: (scanResult) => set({ scanResult }),

  // ── Restructure config ────────────────────────────────────────────────────
  structure: ['customer', 'date'],
  setStructure: (structure) => set({ structure }),
  addDimension: (dim) => set((s) => {
    if (s.structure.length >= 3 || s.structure.includes(dim)) return s;
    return { structure: [...s.structure, dim] };
  }),
  removeDimension: (dim) => set((s) => ({
    structure: s.structure.filter((d) => d !== dim),
  })),
  moveDimension: (dim, delta) => set((s) => {
    const idx = s.structure.indexOf(dim);
    if (idx < 0) return s;
    const newIdx = idx + delta;
    if (newIdx < 0 || newIdx >= s.structure.length) return s;
    const next = [...s.structure];
    [next[idx], next[newIdx]] = [next[newIdx], next[idx]];
    return { structure: next };
  }),

  destinationMode: { type: 'local_only' },
  setDestinationMode: (destinationMode) => set({ destinationMode }),
  targetPath: 'C:\\FileIT\\Organized\\',
  setTargetPath: (targetPath) => set({ targetPath }),
  backupEnabled: true,
  setBackupEnabled: (backupEnabled) => set({ backupEnabled }),

  // ── Preview ───────────────────────────────────────────────────────────────
  preview: null,
  setPreview: (preview) => set({ preview }),

  // ── Progress ──────────────────────────────────────────────────────────────
  progressLines: [],
  progressPct: 0,
  progressCurrent: 'Připravuje se…',
  appendProgressLine: (line) => set((s) => ({
    progressLines: [...s.progressLines.slice(-200), line], // keep last 200 lines
  })),
  setProgressPct: (progressPct) => set({ progressPct }),
  setProgressCurrent: (progressCurrent) => set({ progressCurrent }),
  resetProgress: () => set({
    progressLines: [],
    progressPct: 0,
    progressCurrent: 'Připravuje se…',
  }),

  // ── Confirmation ──────────────────────────────────────────────────────────
  completedRun: null,
  setCompletedRun: (completedRun) => set({ completedRun }),

  // ── Cloud detection ───────────────────────────────────────────────────────
  detectedClouds: [],
  setDetectedClouds: (detectedClouds) => set({ detectedClouds }),

  // ── Last run ──────────────────────────────────────────────────────────────
  lastRunTimestamp: null,
  lastRunFileCount: null,
  setLastRun: (timestamp, fileCount) => set({
    lastRunTimestamp: timestamp,
    lastRunFileCount: fileCount,
  }),
}));
