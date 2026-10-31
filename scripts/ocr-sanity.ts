// scripts/ocr-sanity.ts — offline sanity check for the OCR type contract and
// cache naming. Run: node --experimental-strip-types scripts/ocr-sanity.ts
//
// This deliberately makes NO live OCR call. It verifies things that are easy
// to break silently:
//   1. The fixture JSON parses into the documented OcrResult shape.
//   2. The cache filename derivation matches what the Rust side writes
//      (<app_data>/ocr-cache/<sha256>.json).
//   3. The legacy Indra pages[] mapping rules (backward compatibility).
//   4. The REAL Indra CLI output shape (verified live on indra-ocr 1.1.2):
//      JSON LINES, one object per page, one per line — mapped the same way
//      src-tauri/src/ocr/mod.rs::indra_jsonl_to_ocr_result maps it.

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

// ── Legacy raw Indra CLI JSON → OcrResult mapping (pages[] shape) ───────────
// Backward compatibility: verifies the old tolerant mapping rules kept in
// src-tauri/src/ocr/mod.rs::indra_json_to_ocr_result against an inline legacy
// fixture (no live sidecar call).
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
            // Missing confidence and bbox — both fall back to null-tolerance.
            { text: '12.02.2024    -1 250,00 Kč' },
          ],
        },
      ],
    },
    {
      texts: [{ text: 'Page-scope fallback text — no lines array.' }],
    },
  ],
  is_scan: true,
};

const mapped = mapIndraJson(JSON.parse(JSON.stringify(indraFixture)) as RawIndraJson);
check('legacy indra fixture maps into OcrResult', mapped.page_count === 2);
check('legacy line blocks map with label "line"', mapped.layout_blocks[0].label === 'line' && mapped.layout_blocks[2].label === 'line');
check('legacy page numbering is 1-based', mapped.layout_blocks.every(b => b.page >= 1 && b.page <= 2));
check('legacy out-of-range confidence clamps into 0..1', mapped.layout_blocks[0].confidence === 1);
check('legacy ok confidence preserved', mapped.layout_blocks[1].confidence === 0.42);
check('legacy missing confidence → null', mapped.layout_blocks[2].confidence === null);
check('legacy bbox accepted as box', mapped.layout_blocks[0].box !== null && mapped.layout_blocks[0].box!.length === 4);
check('legacy box alias for bbox accepted', mapped.layout_blocks[1].box !== null);
check('legacy missing bbox → null box', mapped.layout_blocks[2].box === null);
check('legacy invalid bbox → null box', bboxToTuple([1, 2, 3]) === null && bboxToTuple('junk' as unknown as number[]) === null);
check('legacy non-numeric bbox entry → null box', bboxToTuple([1, 2, 3, 'x' as unknown as number]) === null);
check('legacy language picked from first page', mapped.language === 'ces');
check('legacy texts-without-lines become label "text" blocks', mapped.layout_blocks[3].label === 'text');
check('legacy is_scan defaults true when omitted', mapped.is_scan === true);
check('legacy confidence_avg averages present values only',
  mapped.confidence_avg !== null && Math.abs(mapped.confidence_avg - (1 + 0.42) / 2) < 1e-9);
check('legacy mapped fixture re-parses as OcrResult object', mapped.path === 'fixture.pdf');
check('legacy full_text only from page-scope fallback (lines feed blocks)', mapped.full_text === 'Page-scope fallback text — no lines array.');

// ── REAL Indra CLI JSONL → OcrResult mapping (verified live shape) ──────────
// indra-ocr 1.1.2 (live smoke test, real 2-page PDF, both --out mode and
// stdout mode) emits JSON LINES: one object per page, one per line. Keys per
// line object: file, page (0-based), source ("text"|"ocr"), rotation, skew,
// vertical, note, lines [{box:[x1,y1,x2,y2], text, conf}], markdown (string
// with real layout incl. table HTML), blocks [{label, text}], timings.
// There is NO language field anywhere. This mirrors
// src-tauri/src/ocr/mod.rs::indra_jsonl_to_ocr_result.

interface RealLineObj { box?: [number, number, number, number] | null; text?: string; conf?: number; }
interface RealBlockObj { label?: string; text?: string; }
interface RealJsonlPage {
  file?: string;
  page?: number;               // 0-based in the CLI; we map to 1-based
  source?: string;             // "text" = digital layer, anything else = scan
  lines?: RealLineObj[];
  markdown?: string;
  blocks?: RealBlockObj[];
}

function mapIndraJsonl(lines: RealJsonlPage[]): OcrResult {
  const blocks: OcrBlock[] = [];
  let fullText = '';
  let hasOcrPage = false;

  lines.forEach((page, idx) => {
    if (page.source !== 'text') hasOcrPage = true;
    const pageLines: string[] = [];

    for (const line of page.lines ?? []) {
      const text = line.text ?? '';
      if (text) pageLines.push(text);
      blocks.push({
        page: idx + 1,
        label: 'line',
        text,
        confidence: typeof line.conf === 'number' ? clampUnit(line.conf) : null,
        box: bboxToTuple(line.box),
      });
    }

    for (const b of page.blocks ?? []) {
      blocks.push({ page: idx + 1, label: b.label ?? 'block', text: b.text ?? '', confidence: null, box: null });
    }

    if (pageLines.length > 0) fullText += pageLines.join('\n') + '\n';
  });

  const confs = blocks.filter(b => b.label === 'line')
    .map(b => b.confidence).filter((c): c is number => c !== null);
  const confidenceAvg = confs.length > 0 ? confs.reduce((a, b) => a + b, 0) / confs.length : null;

  return {
    path: 'vypis.pdf',
    sha256: null,
    full_text: fullText.trimEnd(),
    language: null, // the JSONL has no language field — never fabricate
    confidence_avg: confidenceAvg,
    page_count: Math.max(lines.length, 1),
    is_scan: hasOcrPage,
    layout_blocks: blocks,
  };
}

// Faithful two-page extraction of the real CLI output recorded in the live
// smoke test (indra-ocr 1.1.2 on a digitally-typed 2-page bank statement PDF).
const REAL_JSONL = [
  '{"file": "C:\\\\Klienti\\\\vypis.pdf", "page": 0, "source": "text", "rotation": 0, "skew": 0.0, "vertical": false, "note": "order=model", "lines": [{"box": [201.6, 197.0, 625.3, 223.7], "text": "STRANA 1 - VYPIS Z UCTU", "conf": 1.0}, {"box": [201.7, 254.6, 305.4, 280.4], "text": "Datum  Popis  Castka", "conf": 0.42}, {"box": null, "text": "01.01.2026 Nakup 1250 Kc"}], "markdown": "## STRANA 1 - VYPIS Z UCTU\\n\\n<table>\\n<tr><th>Datum</th><th>Popis</th><th>Castka</th></tr>\\n<tr><td>01.01.2026 Nakup</td><td>-1250,00 Kc</td></tr>\\n</table>", "blocks": [{"label": "Section-Header", "text": "STRANA 1 - VYPIS Z UCTU"}, {"label": "Table", "text": "Datum Popis Castka 01.01.2026 Nakup 1250 Kc"}], "timings": {"text_ms": 2.56, "det_ms": 100.17, "total_ms": 136.81}}',
  '{"file": "C:\\\\Klienti\\\\vypis.pdf", "page": 1, "source": "ocr", "rotation": 0, "skew": 3.5, "vertical": true, "note": "no text layer; order=model", "lines": [{"box": [10.0, 10.0, 320.4, 25.7], "text": "STRANA 2 POKRACOVANI", "conf": 1.7}, {"box": [10.0, 30.0, 310.4, 45.7], "text": "Zustatek 12345,67 Kc"}], "markdown": "STRANA 2 POKRACOVANI Zustatek 12345,67 Kc", "blocks": [{"label": "Table", "text": "Zustatek 12345,67 Kc"}], "timings": {"text_ms": 1.38, "det_ms": 57.86, "total_ms": 67.40}}',
];

const parsedPages: RealJsonlPage[] = REAL_JSONL.map(l => JSON.parse(l) as RealJsonlPage);
const realPages: RealJsonlPage[] = JSON.parse(JSON.stringify(parsedPages)) as RealJsonlPage[];
const r = mapIndraJsonl(realPages);

check('real JSONL fixture parses', realPages.length === 2);
check('page_count = number of JSONL lines', r.page_count === 2);

const line1 = r.layout_blocks[0];
const line2 = r.layout_blocks[1];
const line3 = r.layout_blocks[2];
// Fixture block order: 3 page-1 lines, 2 page-1 blocks, 2 page-2 lines, 1 page-2 block.
const strana2FirstLine = r.layout_blocks[5];

// Per-page mapping, 1-based.
check('real block labels carried', r.layout_blocks.filter(b => b.label === 'Section-Header' || b.label === 'Table').length === 3);
check('OcrBlock.page is 1-based (JSONL page is 0-based)',
  line1.page === 1 && line2.page === 1 && line3.page === 1 && strana2FirstLine.page === 2);
check('page_count matches max block page', r.layout_blocks.every(b => b.page >= 1 && b.page <= r.page_count));

// is_scan derived from source.
check('source "text" page does not force is_scan alone', !mapIndraJsonl([{ source: 'text', lines: [] }]).is_scan);
check('source "ocr" → is_scan=true', mapIndraJsonl([{ source: 'ocr', lines: [] }]).is_scan === true);
check('missing source → is_scan=true', mapIndraJsonl([{}]).is_scan === true);
check('source anything-but-"text" counts as scan', mapIndraJsonl([{ source: 'OCR' }]).is_scan === true);
check('real 2-page fixture (text+ocr) → is_scan=true', r.is_scan === true);
check('all-text source multi-page → is_scan=false',
  mapIndraJsonl([{ source: 'text', lines: [] }, { source: 'text', lines: [] }]).is_scan === false);

// conf → confidence, clamping and missing.
check('line conf 1.0 → confidence 1.0', line1.confidence === 1.0);
check('line conf 0.42 preserved', line2.confidence === 0.42);
check('out-of-range conf 1.7 clamps to 1.0', strana2FirstLine.confidence === 1.0);
check('missing conf → null', line3.confidence === null);
check('null conf → null', mapIndraJsonl([{ lines: [{ text: 'x', conf: undefined }, { text: 'y', conf: null as unknown as number }] }]).layout_blocks[1].confidence === null);
check('conf -0.5 clamps to 0.0', mapIndraJsonl([{ lines: [{ text: 'x', conf: -0.5 }] }]).layout_blocks[0].confidence === 0.0);

// box handling.
check('line box maps through', Array.isArray(line1.box) && line1.box.length === 4);
check('null line box → null', line3.box === null);

// markdown presence (real layout incl. table HTML).
check('page 1 markdown contains table HTML', realPages[0].markdown!.includes('<table>'));
check('markdown is NOT appended into full_text', !r.full_text.includes('<table>'));

// full_text = newline-joined line texts in order.
check('full_text joins page-1 line texts with newlines',
  r.full_text.startsWith('STRANA 1 - VYPIS Z UCTU\nDatum  Popis  Castka\n01.01.2026 Nakup 1250 Kc\nSTRANA 2 POKRACOVANI'));

// language: the real JSONL has NO language field.
check('language stays null (JSONL has no language field)', r.language === null);

// confidence_avg over present line confs only.
const presentLineConfs = [1.0, 0.42, 1.0]; // line 4 (strana2 first) clamped to 1.0; missing→excluded
const expectedAvg = presentLineConfs.reduce((a, b) => a + b, 0) / presentLineConfs.length;
check('confidence_avg = mean of present line confs', r.confidence_avg !== null && Math.abs(r.confidence_avg - expectedAvg) < 1e-9);

// block labels pass straight through.
check('block labels carried into OcrBlock.label',
  r.layout_blocks.some(b => b.label === 'Section-Header') && r.layout_blocks.some(b => b.label === 'Table'));

console.log(failed === 0 ? 'ALL PASSED' : `${failed} FAILED`);
process.exit(failed === 0 ? 0 : 1);
