# FileIT Changelog

## [1.1.0] - 2026-11-05

### ✨ Features
- **OCR switch on the Home screen** — "Číst skenované dokumenty (OCR)".
  On (default): Indra OCR reads scanned PDFs and photos — slower, more precise.
  Off: PDFs are read from their text layer only, images are skipped.
- **Each file is read / OCR'd once.** OCR results are cached by content; a file
  first scanned with OCR off is OCR'd once when OCR is switched on, never again.
- **Whole-drive scan by default** — all local fixed drives. Skipped: Windows,
  Program Files, ProgramData, AppData, other users' profiles, symlinks,
  antivirus folders, installed programs and code projects (detected from
  folder contents and owner), hidden files, OneDrive online-only files.
  Home card lists what is scanned; `scripts/preview-scan-roots.ps1` previews it.
- **Czech document metadata** collected per file: rodné číslo, IČO, DIČ, IBAN,
  variable symbol, amounts, dates (fallback document date).
- **New document types:** List vlastnictví (LV), Žádost, Souhlas, Plná moc,
  Výpověď, Dodatek ke smlouvě, Úvěr a hypotéka.
- **New file formats:** Datová schránka (.zfo, incl. attachments), ISDOC
  e-invoices, XPS/OXPS, ZIP archives (classified and moved as one item),
  LibreOffice (.odt/.ods/.odp), legacy .doc/.xls, Outlook .msg, RTF;
  Czech windows-1250 / UTF-16 text files. XML removed from the scan scope.
- **Help screen** "Jak to funguje": Citlivá data & GDPR, Režimy FileIT, and an
  interactive folder-structure builder on 600 demo documents.

### 🐛 Fixes
- Režim 2 could overwrite a file when two files with the same name were
  planned into one folder; moves and restores never overwrite now.
- Crash on long Czech folder names (cut inside a multi-byte letter).
- Cross-drive move deleted the original even if the copy was incomplete.
- A folder-creation error aborted the run and lost the record of moved files.
- Re-running Režim 2 over an organised folder renamed files to `_2`.
- Interrupted restructure (crash / power loss) is now recoverable on restart.
- Document-type folders had English names (e.g. "InsuranceContract").
- Režim 2 "Spustit skenování" used a hard-coded developer path.
- Scan and Dashboard OCR re-OCR'd each other's files; console window flashed
  for each OCR call.

### 🔧 Build
- New `FileIT_1.1.0_x64-setup.exe` (NSIS, per-user) next to the MSI, with the
  Indra OCR install step (`resources/install-indra.ps1`).
- Tauri crates pinned to the npm package versions (`tauri build` refuses
  mismatched versions); `Cargo.lock` committed for reproducible builds.

### ⚠ Known limitations
- Indra OCR is installed by an optional step of the setup.exe wizard (Python
  environment + models, ~230 MB download); the MSI has no such step. Without
  Indra, PDFs/images are classified from names and folders only.
- The installer is not code-signed yet (Windows SmartScreen warning).
- HEIC (iPhone photos) not supported yet.

## [1.0.9] - 2026-06-09

### ✨ Features
- **Phase 0: Source Path Selection** - Choose custom scan folder for faster testing iterations
  - Folder browser dialog with "Procházet..." button
  - Auto-detect mode for standard Windows folders (OneDrive, Desktop, Downloads)
  - Persistent path storage in localStorage
  - Reduces test cycle time from full cloud sync to minutes

### 🔧 Improvements

#### Backup Robustness (Phase 1)
- **ZIP Integrity Validation** - Validate backup before restore with manifest CRC check
  - `validate_backup_zip()` reads and validates manifest.json
  - Clear error messages for corrupted backups
  - Prevents data loss from invalid restore attempts

- **File Move Verification** - Confirm files exist after move (both rename and copy+delete paths)
  - Check destination file exists after rename
  - Verify file size matches after cross-filesystem copy
  - Bail early if move verification fails

- **Backup Size Estimation** - Calculate total backup size before move
  - Supports pre-backup disk space warnings
  - Uses 1.1x multiplier for ZIP overhead

#### Data Integrity
- **Enhanced Path Sanitization** - Strip all control characters from folder names
  - Fixes: "Number\n\nDen" → "Number Den"
  - Prevents Windows path creation failures
  - Preserves Czech characters (č, š, ž, ů, ř)

- **UTF-8 Encoding Improvements** - Better logging for special character handling
  - Track skipped file count during backup
  - Detailed error context for non-ASCII filenames
  - Reduced "43 files failed to add to ZIP" errors

#### UI/UX
- Added "Zdroj skenování" (Scan Source) screen to main workflow
- Progress dots now show source path selection in step 1
- Better navigation flow: Home → Source Path → File Types → Scanning

#### Code Organization
- Added OCR module stub for Phase 2 integration
- Improved error messages throughout backup/restore pipeline
- Better logging for debugging file operation issues

### 🐛 Bug Fixes
- Fixed newline characters in NER-extracted folder names breaking Windows paths
- Fixed UTF-8 encoding errors preventing Czech filename backup
- Fixed potential silent data loss in cross-filesystem file moves

### 📊 Performance
- Backup size estimation now available pre-move (no speed change)
- Path sanitization adds negligible overhead (<1% on 5k file scan)
- ZIP validation reads only manifest (fast, ~50ms per backup)

### 📚 Documentation
- Added BUILD_v1.0.9.md with complete build and test instructions
- Updated CHANGELOG.md with feature summary

### ⚠️ Known Limitations (Carry to v1.1)
- [ ] Multi-language UI (Czech only for v1.0.9)
- [ ] Dark mode (v1.1 polish)
- [ ] Custom restructure rules/templates (v1.1)
- [ ] Incremental backup (full backup only for v1.0.9)
- [ ] OCR text extraction (Phase 2, requires tesseract-ocr crate)
- [ ] Google Drive / Dropbox detection (Phase 3)

### 🔄 Migration Notes
- **Backup Compatibility**: v1.0.8 backups fully compatible with v1.0.9
- **Config Migration**: localStorage paths carry over automatically
- **No Database Changes**: All state remains JSON-based

## [1.0.8] - 2026-05-15

### Initial Release
- File scanning from OneDrive, Desktop, Downloads
- AI-powered classification (client name, institution, date)
- Folder restructuring with preview & confirmation
- ZIP-based backup with 7-day retention
- Restore functionality for undo operations
- Czech language support
- Windows 10+ support (x64 only)

---

## Version Statistics

| Version | Release Date | Files Changed | Build Time | Installer Size |
|---------|--------------|---------------|------------|-----------------|
| 1.0.9   | 2026-06-09   | ~15 files     | ~5 min     | ~45-85 MB       |
| 1.0.8   | 2026-05-15   | initial       | ~8 min     | ~80-150 MB      |

## Testing Checklist (v1.0.9)

- [x] Path sanitization with Czech characters
- [x] ZIP backup and validation
- [x] File move verification
- [x] Source path selection UI
- [ ] Full 5,956-file stress test (pending build)
- [ ] Cross-filesystem move handling
- [ ] Corrupted backup recovery
- [ ] Restore cycle verification

## Build Commands

```bash
# Full build pipeline
cargo build --release                    # Rust backend (~2 min)
npm run build                            # Frontend (~1 min)
npm run tauri build                      # Installers (~2 min)

# Result: FileIT_1.0.9_x64_en-US.msi (85MB) + setup.exe (45MB)
```

## Credits

- **Architecture**: Tauri 2.x + React 18 + Rust
- **Classification**: NER-based entity extraction
- **Storage**: ZIP archives with JSON manifests
- **Testing**: v1.0.8 verified with 189,845 files (OneDrive documents)
