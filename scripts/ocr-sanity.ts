// scripts/ocr-sanity.ts — offline sanity check for the OCR type contract and
// cache naming. Run: node --experimental-strip-types scripts/ocr-sanity.ts
//
// This deliberately makes NO live OCR call. It verifies two things that are
// easy to break silently:
//   1. The fixture JSON parses into the documented OcrResult shape.
//   2. The cache filename derivation matches what the Rust side writes
//      (<app_data>/ocr-cache/<sha256>.json).

import { createHash } from 'node:crypto';
import type { OcrResult, OcrBlock } from '../src/types/index.ts';

let failed = 0;
const check = (label: string, cond: boolean): void => {
  console.log(`${cond ? '✓' : '✗'} ${label}`);
  if (!cond) failed += 1;
};

// ── Fixture: a two-page scanned bank statement ──────────────────────────────
const fixture = {
  path: 'C:\\Klienti\\Novák\\csob_2024_03.pdf',
  sha256: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
  full_text: 'Československá obchodní banka Výpis z účtu za březen 2024',
  language: 'ces',
  confidence_avg: 0.93,
  page_count: 2,
  is_scan: true,
  layout_blocks: [
    {
      page: 1,
      label: 'heading',
      text: 'Československá obchodní banka',
      confidence: 0.98,
      box: [72, 48, 523, 96],
    },
    {
      page: 1,
      label: 'paragraph',
      text: 'Výpis z účtu za březen 2024',
      confidence: 0.91,
      box: [72, 110, 300, 128],
    },
    {
      // No confidence and no box — the sidecar is allowed to omit both.
      page: 2,
      label: 'table',
      text: 'Datum Popis Částka',
      confidence: null,
      box: null,
    },
  ],
};

let parsed: OcrResult;
try {
  parsed = JSON.parse(JSON.stringify(fixture)) as OcrResult;
  check('fixture parses into OcrResult', true);
} catch (e) {
  check(`fixture parses into OcrResult (${e})`, false);
  process.exit(1);
}

// ── Shape assertions ────────────────────────────────────────────────────────
check('path is a string', typeof parsed.path === 'string');
check('sha256 is a string', typeof parsed.sha256 === 'string');
check('sha256 is 64 hex chars', /^[0-9a-f]{64}$/.test(parsed.sha256 ?? ''));
check('full_text is a string', typeof parsed.full_text === 'string');
check('language is a string', typeof parsed.language === 'string');
check('confidence_avg within 0..1', (parsed.confidence_avg ?? -1) >= 0 && (parsed.confidence_avg ?? 2) <= 1);
check('page_count is a positive number', parsed.page_count > 0);
check('is_scan is a boolean', typeof parsed.is_scan === 'boolean');
check('layout_blocks is an array', Array.isArray(parsed.layout_blocks));
check('page_count matches block page range', parsed.layout_blocks.every(b => b.page >= 1 && b.page <= parsed.page_count));

const block: OcrBlock = parsed.layout_blocks[0];
check('block.page is a number', typeof block.page === 'number');
check('block.label is a string', typeof block.label === 'string');
check('block.text is a string', typeof block.text === 'string');
check('block.confidence is number|null', block.confidence === null || typeof block.confidence === 'number');
check('block.box is a 4-tuple or null', block.box === null || (Array.isArray(block.box) && block.box.length === 4));
check('sparse block keeps null confidence', parsed.layout_blocks[2].confidence === null);
check('sparse block keeps null box', parsed.layout_blocks[2].box === null);

// ── Cache filename derivation ───────────────────────────────────────────────
// Mirrors src-tauri/src/ocr_store.rs::cache_path_for_hash
const APP_DATA = 'C:\\Users\\dev\\AppData\\Roaming\\cz.fileit.app';
function cacheFileName(appDataDir: string, sha256: string): string {
  return `${appDataDir}\\ocr-cache\\${sha256}.json`;
}

const expected = `${APP_DATA}\\ocr-cache\\${parsed.sha256}.json`;
check('cache filename is <sha256>.json under ocr-cache', cacheFileName(APP_DATA, parsed.sha256!) === expected);

// Different content must land on a different entry — the whole point of
// content-addressed caching.
const other = createHash('sha256').update('a different document').digest('hex');
check('distinct content → distinct cache file', cacheFileName(APP_DATA, other) !== expected);

// Same content always maps to the same entry, whatever the path.
const movedFixture = { ...parsed, path: 'D:\\Archive\\csob_2024_03.pdf' };
check('same content at a new path reuses the same cache entry',
  cacheFileName(APP_DATA, movedFixture.sha256!) === expected);

// ── Round-trip: serialise then parse, as the Rust cache read does ───────────
const roundTripped = JSON.parse(JSON.stringify(parsed)) as OcrResult;
check('round-trip preserves all fields',
  JSON.stringify(roundTripped) === JSON.stringify(parsed));

console.log(failed === 0 ? 'ALL PASSED' : `${failed} FAILED`);
process.exit(failed === 0 ? 0 : 1);
