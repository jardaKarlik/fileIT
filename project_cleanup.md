# FileIT Project Cleanup Guide

## Current State
**Total Size**: ~10GB  
**Breakdown**:
- `fileit/` (v1.0.8): 2.1GB
- `1.0.2/` (v1.0.6): 6.1GB
- Old docs & prototypes: ~800MB

## What's Taking Space

### Large Cached/Build Directories
```
src-tauri/target/               2.0GB  (build cache)
fileit/node_modules/            108MB  (npm cache)
1.0.2/node_modules/             ~200MB (old version)
.cargo/                          ~500MB (Rust registry)
```

### Old Documentation & Prototypes
```
FileIT_Pitch_Deck.pptx           363KB
FileIT_Pitch_Deck_v2.pptx        399KB
fileit_prototype_v2.html         155KB
fileit_full_prototype.html       36KB
FileIT_TechDesign_Summary.docx   27KB
FileIT_ParkingLot_DataSteward.docx 21KB
fileit_source.tar.gz             75KB
fileit_scan_pipeline.html        12KB
fileit_data_model.html           12KB
fileit_runtime_architecture.html 8KB
files/                           4KB
package-lock.json                1KB
```

## Cleanup Strategy (NO ACTION UNTIL YOU APPROVE)

### SAFE TO DELETE (0 risk)

1. **Old Build Artifacts** (1.0.2 version)
   ```
   rm -rf C:\_dev\fileIT\1.0.2
   Space saved: ~6.1GB
   ```
   Reason: Already have v1.0.8 as main, 1.0.2 is completely obsolete

2. **Prototype & Docs Files**
   ```
   rm C:\_dev\fileIT\*.pptx          (2 files, 760KB)
   rm C:\_dev\fileIT\*.docx          (2 files, 48KB)
   rm C:\_dev\fileIT\*_prototype*.html (1 file, 155KB)
   rm C:\_dev\fileIT\fileit_*.html   (5 files, 44KB)
   rm C:\_dev\fileIT\fileit_source.tar.gz (75KB)
   Space saved: ~1.1GB
   ```
   Reason: Design artifacts, not needed in production repo

3. **Build Cache (can be regenerated)**
   ```
   cd C:\_dev\fileIT\fileit
   rm -rf src-tauri/target
   rm -rf node_modules
   Space saved: ~2.1GB
   ```
   Reason: `npm install` and `cargo build` will recreate if needed
   Caveat: Requires re-downloading (~200MB internet)

4. **Old .gitignored entries**
   ```
   rm -rf C:\_dev\fileIT\fileit\.cargo
   rm -rf C:\_dev\fileIT\fileit\files
   Space saved: ~500MB
   ```
   Reason: Already in .gitignore, not tracked

### CONDITIONAL (use with caution)

5. **Rust registry cache**
   - Don't delete if you plan to build offline
   - Safe to delete if you have reliable internet
   - Rebuild takes ~5 minutes
   ```
   rm -rf ~/.cargo/registry/cache
   Space saved: ~300MB
   ```

6. **v1.0.8 Compiled Binaries** (after archiving installers)
   ```
   rm C:\_dev\fileIT\fileit\src-tauri\target\release\fileit.exe
   rm C:\_dev\fileIT\fileit\src-tauri\target\release\fileit_lib.dll
   Space saved: ~20MB (negligible)
   ```
   Reason: Already have installers (MSI/exe in bundle/)
   Keep: Until installers are verified on user machines

## Recommended Cleanup Plan

### Phase 1: Minimal Risk (6.1GB saved)
```bash
# Archive 1.0.2 elsewhere if needed, then:
rm -rf C:\_dev\fileIT\1.0.2
```

### Phase 2: Safe (1.1GB saved)
```bash
# Delete prototype files
rm C:\_dev\fileIT\*.pptx
rm C:\_dev\fileIT\*.docx
rm C:\_dev\fileIT\*_prototype*.html
rm C:\_dev\fileIT\fileit_*.html
rm C:\_dev\fileIT\fileit_source.tar.gz
```

### Phase 3: If Needed (2.1GB saved, needs rebuild)
```bash
# Only if not actively building
rm -rf C:\_dev\fileIT\fileit\src-tauri\target
rm -rf C:\_dev\fileIT\fileit\node_modules

# Rebuild with:
npm install
cargo build --release
```

## Post-Cleanup Verification

Run after cleanup:
```bash
cd C:\_dev\fileIT

# Check total size
du -sh .

# Verify git still works
git status
git log --oneline -3

# Verify source files intact
ls -lh fileit/src-tauri/src/*.rs
ls -lh fileit/src/
```

## Expected Sizes After Cleanup

| Stage | Total Size | Comment |
|-------|-----------|---------|
| Current | ~10GB | Full with old builds |
| After Phase 1 | ~3.9GB | Remove 1.0.2 |
| After Phase 2 | ~2.8GB | Remove docs |
| After Phase 3 | ~500MB | Minimal (src only) |

## Git Cleanup (Optional)

If .gitignored files are in history:
```bash
cd C:\_dev\fileIT\fileit

# Check what's actually tracked
git ls-files | grep "target\|node_modules" 

# If found, they were committed before .gitignore was added
# Safe to ignore — git won't track them now
```

## Important Notes

⚠️ **Before Deleting Anything**:
1. Verify v1.0.8 installers are backed up elsewhere
2. Confirm git history is clean
3. Test that source code is intact
4. Consider keeping 1.0.2 as archive on external drive

✅ **Safe to do anytime**:
- Delete docs/prototypes
- Delete build cache (can regenerate)
- Delete old versions

❌ **Never delete**:
- `fileit/` source code (v1.0.8)
- `.git/` directory
- `src-tauri/Cargo.toml` or `package.json`
- Release installers (until v1.0.9 is ready)

## My Recommendation

**Do NOT run cleanup automatically.** Instead:

1. Review this list with your approval
2. Decide what to keep/remove
3. I'll prepare git commands to clean up safely
4. You approve before executing

This way nothing is lost due to my assumptions.

