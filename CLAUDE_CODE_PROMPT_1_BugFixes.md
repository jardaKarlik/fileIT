# CLAUDE CODE PROMPT #1: Bug Diagnosis & Fixes
**Task:** Diagnose and fix 3 critical bugs  
**Estimated time:** 2-3 hours  
**Build:** FileIT v1.0.12  
**Reference:** `CLAUDE_CODE_HANDOFF_v1.0.12.md`

---

## BUG #1: Backup Process Broken (Task 869dm1qc4)

### Diagnosis Steps
1. Read `src-tauri/src/backup/mod.rs` — understand the entire backup flow
2. Trace the flow:
   - Where is the ZIP created?
   - Is the source directory specified correctly?
   - Is the JSON manifest serialized?
   - Are file move operations tracked (source → destination)?
3. Identify what's broken (ZIP creation, manifest, tracking, or restore)

### What We Know
- Backup button exists in Mode 1 (Zálohovat)
- It should create a ZIP + JSON manifest (not just ZIP)
- v1.0.11 switched from full ZIP to lightweight JSON manifest
- Restore should work end-to-end

### Implementation
- Fix the broken component (ZIP creation, manifest, file tracking, or restore)
- Add error handling: if ZIP fails, log error + show to user
- Add error handling: if manifest serialization fails, log error + show to user
- Test in `cargo tauri dev`:
  - Create small test archive (5-10 files)
  - Run backup
  - Verify ZIP exists + manifest exists
  - Run restore to different location
  - Confirm all files present + intact

### Success Criteria
- Backup completes without silent failure
- Manifest captures source→destination moves
- Restore works end-to-end
- Errors are visible to user (not silent)

---

## BUG #2: LOCAL-ONLY Default Path Wrong (Task 869dm1qex)

### Diagnosis Steps
1. Find where output path is configured:
   - Check `.env` file (if exists)
   - Check `src-tauri/Cargo.toml` (env vars)
   - Check `src-tauri/src/lib.rs` (hardcoded defaults)
   - Check `src-tauri/src/main.rs` (initialization)
2. Look for mode detection:
   - Is there a `LOCAL_ONLY` or `MODE` flag?
   - How is it set?
3. Trace the path:
   - What path does the backend use for "local" mode?
   - What path does the UI show?
4. Identify mismatch

### What We Know
- LOCAL-ONLY mode should output to `C:\ReOrganized`
- Currently outputs to cloud path (wrong)
- User reported this in Task 869dm1qex
- This affects all local-only operations

### Implementation
- Change default path to `C:\ReOrganized` when `LOCAL_ONLY=true` or similar flag
- Ensure backend respects this path during scan + reorganization
- Test in `cargo tauri dev`:
  - Start app in LOCAL mode
  - Run a scan
  - Verify output folder is `C:\ReOrganized`
  - Verify files are moved there, not cloud

### Success Criteria
- LOCAL mode uses `C:\ReOrganized` as default
- This path is consistent across scan, reorganization, backup
- No fallback to cloud paths in LOCAL mode

---

## BUG #3: Error Handling in Final Reorg Step (Task 869dm1t3h)

### Diagnosis Steps
1. Read `src-tauri/src/restructure/mod.rs` — find the final reorganization step (where files are actually moved)
2. Look for:
   - Try/catch blocks (Rust: `Result<T>`, `.map_err()`, `?` operator)
   - Error handling at file move step
   - User-facing error messages
3. Trace error flow:
   - If a file move fails (permissions, locked file, missing folder), what happens?
   - Is the error logged?
   - Is the error shown to user?
4. Identify gaps

### What We Know
- Final reorg step is moving files from temp → final destinations
- Currently, errors are silent (files disappear without message)
- This is a compliance risk (audit trail shows files moved, but they weren't)
- Need structured error logging

### Implementation
1. Add error handling to final reorg step:
   - Wrap file move in error handler
   - On error: log structured error (timestamp, source file, destination, error type)
   - On error: show error modal to user (not silent failure)
2. Structured error log format:
   ```
   {
     "timestamp": "2026-08-11T10:30:00Z",
     "operation": "file_move",
     "source": "C:\\temp\\file.pdf",
     "destination": "C:\\ReOrganized\\Klient\\2024\\Instituce\\file.pdf",
     "error_type": "permission_denied",
     "error_message": "Access denied: file is locked by another process"
   }
   ```
3. Test in `cargo tauri dev`:
   - Create a test scenario that triggers an error (lock a file during move, deny permissions, etc.)
   - Verify error is logged
   - Verify error modal appears to user
   - Verify user can dismiss error + continue

### Success Criteria
- Errors are not silent
- Errors are logged to audit trail
- Errors are shown to user (modal with clear message)
- User can see which files failed + why

---

## TESTING IN `cargo tauri dev`

After fixing each bug, test in dev mode:

### Bug #1 (Backup)
```bash
cargo tauri dev

# In app:
1. Create small test archive (5-10 files, ~10MB)
2. Mode 1 → Spustit skenování (run scan on test folder)
3. Dashboard → Zálohovat (click backup)
4. Verify:
   - No errors in console
   - Backup ZIP created
   - JSON manifest created
   - Backup completes
5. Restore: Extract ZIP to new location
6. Confirm: All files present + intact
```

### Bug #2 (Local Path)
```bash
cargo tauri dev

# In app:
1. Confirm LOCAL_ONLY mode active
2. Mode 1 → Spustit skenování
3. Wait for scan
4. Check filesystem:
   - Files should be at C:\ReOrganized\...
   - NOT at cloud path
5. If wrong, check console for error messages
```

### Bug #3 (Error Handling)
```bash
cargo tauri dev

# In app:
1. Mode 1 → Spustit skenování
2. During "Uspořádat" (reorganization):
   - Lock one of the source files (open in notepad)
   - This will cause move to fail
3. Verify:
   - Error modal appears (not silent)
   - Error message is clear
   - Reorganization stops gracefully
   - Other files still moved (partial success)
4. Check logs: error should be recorded
```

---

## CODE STRUCTURE REFERENCE

**Rust backend files:**
- `src-tauri/src/lib.rs` — Entry point, exports commands
- `src-tauri/src/main.rs` — Tauri app setup
- `src-tauri/src/backup/mod.rs` — Backup logic (ZIP, manifest)
- `src-tauri/src/restructure/mod.rs` — File reorganization (moves files)
- `src-tauri/src/scanner/mod.rs` — File scanning
- `src-tauri/src/types.rs` — Shared types

**React frontend files:**
- `src/components/Dashboard.tsx` — Main UI
- `src/components/Ucebna.tsx` — Training UI (we'll modify this later)

---

## COMMON PITFALLS

❌ **Don't:** Assume path separators are forward slashes (`/`). Windows uses backslashes (`\`). Rust strings need escaping: `C:\\ReOrganized\\...`

❌ **Don't:** Ignore error types. If `File::open()` returns `Err`, you need to `match` or `.map_err()` to handle it.

❌ **Don't:** Log errors without context. Always include: what file, where was it going, why it failed.

❌ **Don't:** Ship silent failures. Users need to know something went wrong.

✅ **Do:** Test each fix individually before moving to the next bug.

✅ **Do:** Test in `cargo tauri dev` before we build MSI.

✅ **Do:** Check console output (dev mode shows all logs).

---

## NEXT STEPS

1. Read this prompt
2. Examine `src-tauri/src/backup/mod.rs` → diagnose Bug #1
3. Fix Bug #1 → test in dev
4. Examine path config → diagnose Bug #2
5. Fix Bug #2 → test in dev
6. Examine `src-tauri/src/restructure/mod.rs` → diagnose Bug #3
7. Fix Bug #3 → test in dev
8. **Once all 3 bugs pass local testing:** Confirm with Jarda, then move to Phase 2 (Učebna UX)

---

**Your brother (Claude Code) takes it from here.** 🚀

Good luck. Hit any blockers? Ask. I'm coordinating.
