# FileIT v1.0.9 Build Instructions

**Status**: AUTO-MODE In Progress
**Date**: 2026-06-09

## Phase Summary

### ✅ Completed (AUTO-MODE)
1. **Path Sanitization Fix** - Strip newlines/tabs from folder names
2. **UTF-8 Encoding Enhancement** - Better error logging for Czech filenames
3. **ZIP Integrity Validation** - Validate backups before restore
4. **File Move Verification** - Confirm files exist after move
5. **Backup Size Estimation** - Pre-move disk space calculation
6. **Phase 0 UI** - Source Path selection screen

### 🔨 In Progress
- Cargo build --release (Rust backend compilation)
- Frontend build ready to follow

### ⏳ Pending
- Phase 2: OCR Integration (tesseract-ocr)
- Phase 3: Cloud Detection (Google Drive, Dropbox)
- Phase 4: Performance optimization
- Phase 5: Testing & QA

## Build Steps (Sequential)

### Step 1: Rust Backend (in progress)
```bash
cd src-tauri
cargo build --release
```
Expected: ~2-3 min, output to `src-tauri/target/release/fileit.dll`

### Step 2: Frontend Build (ready)
```bash
npm run build
```
Expected: ~1 min, output to `dist/`

### Step 3: Tauri App Build
```bash
npm run tauri build
```
Expected: ~2 min, outputs:
- `src-tauri/target/release/bundle/msi/FileIT_1.0.9_x64_en-US.msi` (~85MB)
- `src-tauri/target/release/bundle/nsis/FileIT_1.0.9_x64-setup.exe` (~45MB)

## Key Fixes in v1.0.9

### Path Sanitization (restructure/mod.rs:255)
**Issue**: Folder names with embedded newlines break Windows path creation
**Fix**: Strip all control characters + collapse multiple spaces
```rust
.filter(|c| !c.is_control() && !c.is_whitespace())
```

### UTF-8 Encoding (backup/mod.rs)
**Issue**: 43 files skipped due to Czech special characters
**Fix**: Improved error context, track skipped count, better logging

### ZIP Validation (backup/mod.rs)
**Issue**: No way to detect corrupted backups before restore
**Fix**: `validate_backup_zip()` reads manifest and validates structure

### File Move Verification (restructure/mod.rs:230-268)
**Issue**: Silent data loss if move fails
**Fix**: Verify destination exists, check file sizes match

## Test Plan After Build

1. **Unit Test**: Path sanitization with Czech characters
   ```bash
   cargo test --lib restructure
   ```

2. **Integration Test**: Full scan → classify → preview → restructure cycle
   - Use Source Path feature to scan small folder first
   - Expected: 5,000+ files in ~1 minute

3. **Backup Validation**: Verify backup can be restored
   - Test on corrupted ZIP (should fail gracefully)
   - Test on valid backup (should pass)

4. **Performance**: Measure scan + classification speed
   - Target: 189,845 files in <2 minutes

## Rollback Plan

If build fails:
1. Check error output
2. Fix issue in relevant file
3. Rerun cargo build
4. Do NOT commit broken state

If runtime errors:
1. Keep v1.0.8 installer available
2. Tag v1.0.9 as pre-release
3. Jump to v1.0.10 if critical

## Version Checklist

- [ ] Rust backend compiles cleanly
- [ ] Frontend builds with npm run build
- [ ] Installers generated successfully
- [ ] Git commits clean and tagged
- [ ] QA test plan executed
- [ ] Ready for release

## Team Space Integration

Tasks from backup_analysis.md:
- [x] Path sanitization (complete)
- [x] UTF-8 encoding (complete)
- [x] ZIP validation (complete)
- [x] File move verification (complete)
- [ ] OCR integration (Phase 2)
- [ ] Cloud detection (Phase 3)
- [ ] Performance optimization (Phase 4)

---
**Next**: Monitor cargo build completion → npm run build → npm run tauri build
