# Build Status & Migration Notes

## Migration Date
2026-06-09 — Migrated from desktop workstation to laptop

## Current State
- **All source files present** ✓
- **Node dependencies installed** ✓
- **Rust toolchain available** ✓
- **Ready for build validation**

## Known Issues (from PROJECT_MEMORY.md)

### High Priority (v1 blockers)
1. **OCR Integration** — Image classification uses stub (returns empty)
   - Location: `src-tauri/src/classifier/extractor.rs:20-24`
   - Status: Awaiting Tesseract or Windows OCR API integration
   
2. **Cloud Detection** — Basic registry checks only
   - Location: `src-tauri/src/cm_registry.rs`
   - Needed: OneDrive/Google Drive/Dropbox detection improvement

3. **Error Handling** — Generic messages for file access issues
   
4. **Performance** — Large scans need pagination/worker optimization

### Medium Priority
- Učebna sync with central CM service
- Backup ZIP integrity validation
- Cross-platform testing (Windows 10/11)

### Low Priority / Polish
- Animation refinements
- Accessibility (ARIA, keyboard nav)
- Dark/light theme toggle
- User documentation

## Files to Update
- [ ] package.json: Sync version 1.0.0 → 1.0.8
- [ ] CHANGELOG.md: Create (none exists yet)
- [ ] README.md: Create user/dev guide

## Next Checkpoints
1. Quick build validation (dev mode)
2. Full app build (production bundle)
3. Version synchronization
4. Priority issue triage
