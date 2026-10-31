# OCR & metadata roadmap

_Status: proposal — not implemented. Local-only constraint applies: FileIT
reads personal documents, so nothing may leave the machine._

## Why OCR

Today classification leans on text that is only available for digitally born
PDFs (`text_preview`) plus filename/layout signals. Scanned pages (image-only
PDFs, photos) fall back to weak signals. Real OCR unlocks:

- 100 %-coverage document-type detection (including scanned contracts/statements)
- full participant names, dates, amounts — the fields the business cares about
- full-text search across the whole vault

## Engine choice: Indra OCR (datalab-to) as phase 1

[datalab-to/indra](https://github.com/datalab-to/indra) — from the Surya/Marker
team. A 1M-parameter OCR model that runs on CPU via ONNX Runtime (no PyTorch,
no GPU). Why it wins for FileIT:

| Fact | Why it matters here |
| --- | --- |
| **Czech explicitly supported** (`cs` in the HF language list, 80+ langs) | Our documents are Czech/Slovak |
| Reads **digital PDFs' text layer and OCRs only what is missing** | One unified pipeline — no separate scanned/digital branch |
| **JSON output: lines, boxes, layout blocks, per-line confidences** (`--format json`) | Structured extraction (dates, amounts, names) gets positions, not just text soup |
| Layout labels + **reading order** (headings, tables, captions, footnotes) | Tables and multi-column bank statements parse correctly |
| ~210 ms/page on 4 cores; 15× Tesseract, 10× PaddleOCR PP-OCR (throughput table) | 11k-file vaults stay interactive on office PCs |
| OmniParseBench 66.7 (PaddleOCR PP-StructureV3 58.4, Tesseract 19.3) | Accuracy near 1B-param models |
| Code **Apache-2.0**; model weights on HF under **OpenRAIL** | App bundling OK; review OpenRAIL use terms once before shipping |
| Handles pages rotated/tilted up to 45°, inverted text | Real-world scanner output |

Known limitations (from the README): handwriting is weak (25.2 on the
handwriting tests), degraded old scans score low (old_scans 20.0), and
PaddleOCR's full PP-StructureV3 still reads plain text slightly better without
Indra's `--math` mode. For degraded scans a later phase can add a heavier
model as an opt-in.

Integration shape (matches FileIT's existing owned-process patterns):

1. `pip install indra-ocr` into a managed Python sidecar (or vendored venv).
2. Model folder cached once (`--models` points at a local dir; pin a version).
3. Rust backend queues image-only pages/files after classification; the CLI
   runs with `--cores` capped (e.g. 4) so the UI stays responsive.
4. Per-file cache keyed by sha256 — re-scans never re-OCR.
5. JSON output → `full_text`, `ocr_confidence_avg`, layout blocks → extraction.

Fallback engines (keep in mind, not phase 1): Tesseract 5 (minimal deps),
PaddleOCR (PP-StructureV3 table comparison), Surya (layout-only needs).

## Pipeline sketch

1. During scan: record `is_scan` per file (still useful for progress planning).
2. Post-scan queue: image files → OCR worker (Indra CLI, `--format json`).
3. Worker emits: full text (reading order), per-line confidence, language.
4. Extraction stage runs on the structured text (see metadata list below).
5. Results merge back into `ClassifiedFile` as new optional fields; the UI and
   the visualization consume them.

## Metadata to collect/generate (target list)

**Content**
- `full_text` (FTS5-indexed) + `text_language` + `ocr_confidence_avg`
- keywords / topics, one-line summary (phase 3, optional LLM)

**Entities**
- participant person names (NER) → feeds client matching (rodné číslo/IČO already exist)
- organizations + `ico` / `dic`, addresses, emails, phone numbers
- signature / stamp presence (image heuristics, phase 2)

**Document facts**
- document date, due date (splatnost), billing period
- total amount + currency + VAT breakdown
- invoice number, variable symbol (VS), bank account / IBAN

**Quality / technical**
- page count, page orientation, DPI estimate
- is_scan, skew/blur score, per-page OCR confidence → a *quality* dimension
  for the graph (find the files whose classification can't be trusted)

**Relationships**
- near-duplicate clusters (already have exact dupes), same-VS / same-invoice links

**Governance**
- PII flags (rodné číslo / personal data found) → retention + GDPR posture
- retention category suggestion per document type

## How it feeds the visualization

New graph dimensions become possible once these fields exist:
`OCR confidence`, `Rok (ověřeno z dokumentu)`, `Klient (z textu, ne ze souboru)`,
`Obsahuje PII`, `Jazyk`, `Skutečná instituce (z textu)`. Document-type accuracy
stops depending on filenames entirely.

## Open questions before implementation

1. Scope: OCR only matching file categories (pdf/image) or everything?
2. Sidecar packaging on Windows: bundled venv vs BYO `pip install indra-ocr`.
3. Model download policy: bundled, first-run download, or BYO folder.
4. Core budget for the OCR worker (UI responsiveness during long runs).
5. OpenRAIL terms sign-off for commercial distribution of the weights.
