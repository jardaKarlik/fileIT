// analysisUtils.ts
// Utilities for checking analysis state and formatting analysis age

import { ScanResult } from '../types';

/**
 * Analyzes the state of scan/analysis data.
 * Returns the scenario and metadata needed by Mode 2 entry logic.
 */
export function analyzeAnalysisState(scanResult: ScanResult | null) {
  if (!scanResult || !scanResult.files || scanResult.files.length === 0) {
    return {
      scenario: 'no_analysis' as const,
      hasAnalysis: false,
      isOldAnalysis: false,
      ageText: null,
    };
  }

  // The scan was performed in the current session, so the analysis is fresh.
  // Use the scan_timestamp if the backend provides one; otherwise treat as "now".
  const scanTime = (scanResult as any).scan_timestamp
    ? new Date((scanResult as any).scan_timestamp).getTime()
    : Date.now();

  const analysisAgeMs = Date.now() - scanTime;
  const analysisAgeDays = Math.max(0, Math.floor(analysisAgeMs / (1000 * 60 * 60 * 24)));

  const ANALYSIS_OLD_THRESHOLD_DAYS = 30;
  const isOld = analysisAgeDays >= ANALYSIS_OLD_THRESHOLD_DAYS;

  return {
    scenario: isOld ? ('old_analysis' as const) : ('has_analysis' as const),
    hasAnalysis: true,
    isOldAnalysis: isOld,
    ageText: formatAnalysisAge(analysisAgeDays),
    ageDays: analysisAgeDays,
  };
}

/**
 * Format analysis age into user-friendly Czech text.
 * Examples: "dnes", "2 dny", "3 týdny", "2 měsíce"
 */
export function formatAnalysisAge(days: number): string {
  if (days === 0) return 'dnes';
  if (days === 1) return 'včera';
  if (days < 7) return `${days} dní`;
  if (days < 30) {
    const weeks = Math.floor(days / 7);
    return `${weeks} ${weeks === 1 ? 'týden' : 'týdnů'} zpátky`;
  }
  const months = Math.floor(days / 30);
  return `${months} ${months === 1 ? 'měsíc' : 'měsíce'} zpátky`;
}

/**
 * Sample 3 random batches of 200 items from the files.
 * Used for Mode 2 preview when analysis exists.
 */
export function sample3RandomBatches(
  files: any[],
  batchSize: number = 200
): any[] {
  if (files.length <= batchSize) return files;

  const batches: any[] = [];
  const stride = Math.max(1, Math.floor(files.length / 3));

  // Pick 3 random start positions
  for (let i = 0; i < 3; i++) {
    const randomStart = Math.floor(Math.random() * stride);
    const batchStart = i * stride + randomStart;
    const batch = files.slice(batchStart, batchStart + batchSize);
    if (batch.length > 0) {
      batches.push(...batch);
    }
  }

  // Deduplicate by file path
  const seen = new Set<string>();
  return batches.filter(f => {
    const path = f.record?.path || f.path;
    if (seen.has(path)) return false;
    seen.add(path);
    return true;
  });
}
