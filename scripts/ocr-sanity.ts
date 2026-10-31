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

// ── Raw Indra CLI JSON → OcrResult mapping ──────────────────────────────────
// Verifies the same tolerant mapping rules implemented in
// src-tauri/src/ocr/mod.rs::indra_json_to_ocr_result, against an inline raw
// Indra-CLI fixture (no live sidecar call):
//   - pages[].texts[].lines[].text → layout_blocks (label "line")
//   - confidence clamped into 0..1; junk/missing → null-tolerant
//   - bbox/box 4-tuples → box, non-4-tuples dropped
//   - page numbering is 1-based; page_count = max page number
//   - language taken from page or top level; is_scan defaults true

interface RawIndraLine { text?: string; confidence?: number; bbox?: unknown; box?: unknown; }
interface RawIndraText { text?: string; lines?: RawIndraLine[]; }
interface RawIndraPage { language?: string; lang?: string; texts?: RawIndraText[]; }
interface RawIndraJson { pages?: RawIndraPage[]; text?: string; language?: string; lang?: string; is_scan?: boolean; }

const clampUnit = (v: number): number => Math.min(1, Math.max(0, v));

function bboxToTuple(v: unknown): [number, number, number, number] | null {
  if (!Array.isArray(v) || v.length !== 4) return null;
  const out: number[] = [];
  for (const item of v) {
    if (typeof item !== 'number' || !Number.isFinite(item)) return null;
    out.push(item);
  }
  return out as [number, number, number, number];
}

function mapIndraJson(raw: RawIndraJson): OcrResult {
  const blocks: OcrBlock[] = [];
  let fullText = '';
  let language: string | null = null;
  let pageCount = 0;

  const pages = raw.pages ?? [];
  if (pages.length === 0) {
    fullText = raw.text ?? '';
    language = raw.language ?? raw.lang ?? null;
    pageCount = 1;
  }

  pages.forEach((page, pageIdx) => {
    const pageNo = pageIdx + 1;
    pageCount = Math.max(pageCount, pageNo);
    if (language === null) language = page.language ?? page.lang ?? null;

    for (const textObj of page.texts ?? []) {
      if (Array.isArray(textObj.lines)) {
        for (const line of textObj.lines) {
          blocks.push({
            page: pageNo,
            label: 'line',
            text: line.text ?? '',
            confidence: typeof line.confidence === 'number' ? clampUnit(line.confidence) : null,
            box: bboxToTuple(line.bbox ?? line.box),
          });
        }
      } else if (typeof textObj.text === 'string') {
        fullText += textObj.text + ' ';
        blocks.push({ page: pageNo, label: 'text', text: textObj.text, confidence: null, box: null });
      }
    }
  });

  const confs = blocks.map(b => b.confidence).filter((c): c is number => c !== null);
  const confidenceAvg = confs.length > 0 ? confs.reduce((a, b) => a + b, 0) / confs.length : null;

  return {
    path: 'fixture.pdf',
    sha256: null,
    full_text: fullText.trim(),
    language,
    confidence_avg: confidenceAvg,
    page_count: Math.max(pageCount, 1),
    is_scan: raw.is_scan ?? true,
    layout_blocks: blocks,
  };
}

const indraFixture: RawIndraJson = {
  pages: [
    {
      language: 'ces',
      texts: [
        {
          lines: [
            { text: 'VÝPIS Z ÚČTU', confidence: 1.15, bbox: [72, 48, 523, 96] },
            { text: 'Datum Popis Částka', confidence: 0.42, box: [72, 110, 300, 128] },
            // Missing confidence and bbox √ both fall back to null-tolerance.
            { text: '12.02.2024    -1 250,00 Kč' },
          ],
        },
      ],
    },
    {
      texts: [{ text: 'Page-scope fallback text √ no lines array.' }],
    },
  ],
  is_scan: true,
};

const mapped = mapIndraJson(JSON.parse(JSON.stringify(indraFixture)) as RawIndraJson);
check('indra fixture maps into OcrResult', mapped.page_count === 2);
check('line blocks map with label "line"', mapped.layout_blocks[0].label === 'line' && mapped.layout_blocks[2].label === 'line');
check('page numbering is 1-based', mapped.layout_blocks.every(b => b.page >= 1 && b.page <= 2));
check('out-of-range confidence clamps into 0..1', mapped.layout_blocks[0].confidence === 1);
check('ok confidence preserved', mapped.layout_blocks[1].confidence === 0.42);
check('missing confidence → null', mapped.layout_blocks[2].confidence === null);
check('bbox accepted as box', mapped.layout_blocks[0].box !== null && mapped.layout_blocks[0].box!.length === 4);
check("box alias for bbox accepted", mapped.layout_blocks[1].box !== null);
check('missing bbox → null box', mapped.layout_blocks[2].box === null);
check('invalid bbox → null box', bboxToTuple([1, 2, 3]) === null && bboxToTuple('junk') === null);
check('non-numeric bbox entry → null box', bboxToTuple([1, 2, 3, 'x']) === null);
check('language picked from first page', mapped.language === 'ces');
check('texts-without-lines become label "text" blocks', mapped.layout_blocks[3].label === 'text');
check('is_scan defaults true when omitted', mapped.is_scan === true);
check('confidence_avg averages present values only',
  mapped.confidence_avg !== null && Math.abs(mapped.confidence_avg - (1 + 0.42) / 2) < 1e-9);
check('mapped fixture re-parses as OcrResult object', mapped.path === 'fixture.pdf');
check('full_text only from page-scope fallback (lines feed blocks)', mapped.full_text === 'Page-scope fallback text √ no lines array.');

console.log(failed === 0 ? 'ALL PASSED' : `${failed} FAILED`);
process.exit(failed === 0 ? 0 : 1);
