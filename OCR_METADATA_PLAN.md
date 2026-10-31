# OCR & metadata roadmap

_Status: proposal (v1) — not implemented. Local-only constraint applies: FileIT
reads personal documents, so nothing may leave the machine._

## Why OCR

Today classification leans on text that is only available for digitally born
PDFs (`text_preview`) plus filename/layout signals. Scanned pages (image-only
PDFs, photos) fall back to weak signals. Real OCR unlocks:

- 100 %-coverage document-type detection (including scanned contracts/statements)
- full participant names, dates, amounts — the fields the business cares about
- full-text search across the whole vault

## Engine options (all local)

| Engine | Strengths | Watch-outs |
| --- | --- | --- |
| **Tesseract 5** (`ces` traineddata) | Apache-2.0, CLI tool, trivial BYO integration, CPU-only | Weak layout/tables; needs preprocessing (deskew, binarize) |
| **PaddleOCR** (PP-OCRv4/v5) | Best accuracy/speed among open OCR, 80+ langs, PP-Structure for tables | Python runtime, bigger download |
| **Surya** | Layout + reading order + tables + line detection, modern ML | GPL code base — license review before bundling |
| **DocTR** | Clean API, detection + recognition | Weaker Czech documentation |
| **VLM** (GOT-OCR 2.0, Qwen2-VL) | Can emit structured JSON directly from a page | Heavy (GPU-friendly), overkill for phase 1 |

Recommendation: **Phase 1 = Tesseract CLI (BYO, like the Ghidra model in REA)**,
**Phase 2 = PaddleOCR or Surya sidecar** for layout-aware extraction. Same
pattern as existing bridges: owned process, bounded resources, per-file cache.

## Pipeline sketch

1. During scan: record `is_scan` (image-only PDF vs digital) per file.
2. After classification pass 1: queue image files for OCR (sha256-keyed cache,
   so re-scans never re-OCR).
3. OCR worker emits: full text (page-ordered), per-word confidence, language.
4. Extraction stage runs on the text (see metadata list below).
5. Results merge back into `ClassifiedFile` as new optional fields; the UI and
   the visualization can consume them.

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

1. Scope decision: OCR only matching files (pdf/image categories) or all files?
2. Tesseract bundling vs BYO install (REA precedent: bring-your-own).
3. Windows packaging for a Python sidecar (Paddle/Surya) — size and install UX.
4. Where OCR runs: scan phase (inline) vs post-scan queue (recommended).
5. License review if Surya is chosen.
