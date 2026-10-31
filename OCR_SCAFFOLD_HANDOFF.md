# Indra OCR scaffold — handoff for an independent agent session

_Goal: implement Phase-1 OCR scaffolding without touching the visualization
code. The viz lives on `feature/sunburst-visualization` and is being finalized
in parallel — do NOT modify `src/components/Dashboard/*`, `src/components/Graph/*`,
`src/utils/vizTree.ts`, or `src/components/{Scanning,Home,Restructure}/*`._

Read `OCR_METADATA_PLAN.md` first for the why. This document is the what/how.

## Deliverables

1. **Types** — extend `src/types/index.ts`:
   ```ts
   export interface OcrResult {
     path: string;
     sha256: string | null;
     full_text: string;
     language: string | null;
     confidence_avg: number | null;   // 0..1, null when no OCR ran
     page_count: number;
     is_scan: boolean;
     layout_blocks: OcrBlock[];       // from indra --format json
   }
   export interface OcrBlock {
     page: number;
     label: string;                   // heading/paragraph/table/…
     text: string;
     confidence: number | null;
     box: [number, number, number, number] | null; // x0,y0,x1,y1 (pt)
   }
   ```
   Add optional fields to `ClassifiedFile`: `ocr?: OcrResult` (may be absent —
   never break the existing contract).

2. **Rust backend** (`src-tauri/`):
   - `src-tauri/src/ocr/mod.rs`: `ocr_config` (detect sidecar), `ocr_queue`
     (files needing OCR), `run_ocr_batch(paths, cores)` → writes results to
     the AppData store, emits `ocr_progress` events
     `{ done, total, current_path, failed_count }`.
   - Sidecar contract: locate a venv/CLI via `INDRA_OCR_COMMAND` env var
     (default `indra-ocr`), model folder via `INDRA_OCR_MODELS`; if missing →
     command returns `capability_unavailable`-style JSON, no crash, UI shows
     "OCR není nainstalován".
   - Cache: `<AppData>/ocr-cache/<sha256>.json`; skip when present.
   - Concurrency: cap workers (default 4, env `INDRA_OCR_CORES`).
   - Invoke commands: `check_ocr`, `run_ocr`, `get_ocr(path)`.
3. **Frontend glue**:
   - `src/utils/tauriApi.ts`: `checkOcr()`, `runOcr()`, `onOcrProgress(cb)`.
   - Dashboard: an "OCR text" status row (installed/not, queued count) — read-only.
   - Do NOT auto-run OCR inside the scan; expose an explicit "Spustit OCR" action.
4. **Tests/scripts**: `scripts/ocr-sanity.ts` validating cache round-trip and
   JSON parsing with a fixture (no live model call in CI).

## Acceptance criteria

- `npm run build` passes; `cargo check` passes.
- With no Indra installed: app fully functional, OCR UI shows unavailable state.
- Cache hit path adds zero OCR invocations.
- Existing scan/restructure/graph flows unchanged (no behavioral diff).

## Branch

`feature/ocr-scaffold` off `feature/sunburst-visualization` (rebase target).
Conventional commits; report real checks run, no simulated-provider claims.
