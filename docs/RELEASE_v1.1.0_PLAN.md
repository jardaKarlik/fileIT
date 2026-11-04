# FileIT v1.1.0 — Indra OCR release plan

Goal: minor release adding Indra OCR, a Home-screen OCR switch, and a
guarantee that a file scanned / OCR'd once is not processed again.

## Done on branch `lee/awesome-curie-ek1wko`

| Area | Change |
|------|--------|
| `src-tauri/src/ocr/mod.rs` | Indra OCR wrapper: finds `indra-ocr`, runs `--format text --max-pages 5`, 120 s timeout, hidden console window, UTF-8 forced |
| `src-tauri/src/classifier/extractor.rs` | OCR only when enabled; Indra first, Windows OCR fallback; reports `needs_ocr` / `ocr_attempted` / `ocr_engine` |
| `src-tauri/src/extraction_cache.rs` | Persistent per-content cache (`extract_cache/<sha256>-<size>.json`) with unit tests |
| `src-tauri/src/classifier/mod.rs` | `ExtractOptions { ocr_enabled, cache }`; cache lookup / upgrade / write |
| `src-tauri/src/commands/mod.rs` | `start_scan` takes `ocr_enabled`, returns cache/OCR stats; new `get_ocr_status`, `clear_scan_cache` |
| `src/components/Home` | OCR switch card with explanation + engine availability note |
| `src/store`, `src/utils/tauriApi.ts`, `src/types` | `ocrEnabled` (persisted in localStorage), API + types |
| `src/components/Scanning` | Passes the flag, shows "reused from cache" and "OCR'd" counts |
| Versions | `Cargo.toml`, `tauri.conf.json` → 1.1.0 |

## Re-scan rules (extraction cache)

| Cached entry | Scan with OCR off | Scan with OCR on |
|---|---|---|
| Native text was fine | reuse | reuse |
| Scanned doc, OCR never tried | reuse (no text) | **OCR once**, then cache |
| Scanned doc, OCR already tried | reuse | reuse — never OCR'd again |
| Different size or first-64 KB hash | re-extract | re-extract |

Bump `CACHE_VERSION` in `extraction_cache.rs` when extraction changes enough
that old results should be discarded.

## Remaining before release

1. **Ship Indra with the installer.** Indra is a Python package. Options:
   - Freeze it with PyInstaller (`pyinstaller --onedir` around the
     `indra_ocr` CLI entry point), place it in `src-tauri/binaries/` and add
     `"externalBin": ["binaries/indra-ocr"]` to `tauri.conf.json` (Tauri
     needs the file named `indra-ocr-x86_64-pc-windows-msvc.exe`). FileIT
     already looks for `indra-ocr.exe` next to `fileit.exe`.
   - Pre-download the ~76 MB models from Hugging Face
     (`datalab-to/indra-ocr`) and bundle them; set `FILEIT_INDRA_MODEL_DIR`
     (or pass `--model-dir`) so first use works offline.
   - Check licences: code Apache-2.0, **weights modified OpenRAIL-M** — confirm
     commercial use terms before shipping.
2. **Test on Windows** with a real document set:
   - OCR off: scan time should match v1.0.11 minus Windows OCR time.
   - OCR on: scanned PDFs / JPGs get customers and institutions.
   - Second scan: log shows "Znovu použito z minulé analýzy: N souborů",
     no Indra processes are started for unchanged files.
   - After a restructure, re-scan of the target folder hits the cache.
3. **Czech accuracy check** — Indra notes Czech diacritics are weaker
   (e.g. ú for ů). Verify NER (names, rodné číslo, IČO) on OCR text; consider
   diacritic-insensitive institution matching if needed.
4. **Performance (optional)**: one Indra process per file reloads the model.
   For large scans, batch all files that need OCR into one
   `indra-ocr <files…> --out <dir>` call.
5. Optional UI: a "Vymazat mezipaměť analýzy" (clear cache) action in a
   settings/help screen, calling `api.clearScanCache()`.
6. Build MSI/NSIS installers, update `README`, tag `v1.1.0`.
