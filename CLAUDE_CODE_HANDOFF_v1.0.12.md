# CLAUDE CODE HANDOFF — FileIT v1.0.12 EXECUTION
**Status:** ACTIVE EXECUTION  
**Start Time:** 2026-08-11, 10:15 CZ  
**Approval:** Jarda confirmed all 5 decisions

---

## YOUR MISSION (CLAUDE CODE / "YOUR BROTHER")

You are now the **execution agent** for FileIT v1.0.12. I (Claude planning/research) coordinate; you build.

**Scope:** Bugs (3) + Učebna (Phases 1-2) + Local testing in `cargo tauri dev`

---

## PHASE 1: BUG FIXES (Est. 2-3 hrs)

### Task 869dm1qc4: Backup Process Broken
**Problem:** Backup ZIP creation or file move tracking failing  
**Diagnosis steps:**
1. Read `src-tauri/src/backup/mod.rs` — understand current backup flow
2. Check if ZIP creation works locally in dev
3. Check if JSON manifest serialization works
4. Check if file move tracking captures source→destination correctly
5. Verify restore flow end-to-end

**Expected fix location:** `src-tauri/src/backup/mod.rs`  
**Test:** Create a small test archive, run backup, verify ZIP + manifest, restore files to different location, confirm all files present

**Success criteria:** Backup works, restore works, no silent failures

---

### Task 869dm1qex: LOCAL-ONLY Default Path Wrong
**Problem:** Should be `C:\ReOrganized`, not cloud path  
**Diagnosis steps:**
1. Find where default path is set (likely `src-tauri/lib.rs` or `.env` or `src-tauri/Cargo.toml`)
2. Check if LOCAL-ONLY mode flag is respected
3. Verify config flow: env var → backend → UI

**Expected fix:** Change default output path to `C:\ReOrganized` when `LOCAL_ONLY=true`

**Test:** Run app in LOCAL mode, check that reorganized files go to `C:\ReOrganized`, not cloud

**Success criteria:** Local-only mode respects correct path

---

### Task 869dm1t3h: Error Handling in Final Reorg Step
**Problem:** Silent failures during file moves, no error logging  
**Diagnosis steps:**
1. Read `src-tauri/src/restructure/mod.rs` — find final move step
2. Check for try/catch or error handling
3. Add structured error logging: operation, source file, destination, error type, timestamp

**Expected fix:** Add error logging + user-facing error modal (not silent)

**Test:** Deliberately trigger an error (file locked, permissions denied), verify error is logged + shown to user

**Success criteria:** Errors are visible to user + logged to audit trail

---

## PHASE 2: UČEBNA PHASE 1 + 2 (Est. 3-4 hrs)

### Phase 1: Information Density (Filename + Origin + Confidence)
**File:** `src/components/Ucebna.tsx`  
**Changes:**
1. Replace card grid with list layout
2. Add filename display (original, masked format for privacy)
3. Add folder origin badge (`📁 From: C:\path\to\folder`)
4. Add confidence badges (color-coded: 🟢 >75%, 🟡 50-75%, 🔴 <50%)
5. Display signal pills (keyword, text match, color, logo, path)
6. Make entire card clickable (cursor pointer, hover effect)

**Design reference:** `CLAUDE_DESIGN_BRIEF_UcebnaPhase1.md`  
**Colors:** Use Neon Twilight system (indigo, deep pink, coral)

**Test:** Render Učebna with 3-5 unrecognized files, confirm UI shows all elements

---

### Phase 2: File Preview + Suggestions (New Component)
**File:** `src/components/UcebnaPreview.tsx` (create new)  
**Behavior:**
1. Click card → side panel opens (30% width, right side)
2. Panel shows:
   - File preview (PDF first page thumbnail + OCR text snippet)
   - Institution picker dropdown or chips
   - Question: "What was the strongest signal?" (checkbox group)
     - ☐ Filename keyword
     - ☐ Color branding
     - ☐ Logo/visual
     - ☐ Document content (text)
     - ☐ Original folder path

**Wiring:**
- `useState` to manage preview panel open/close
- `useState` for selected institution + signal selection
- On confirm: call backend to log choice (but don't trigger rescan yet — Phase 3 does that)

**Test:** Click card, panel opens, can select institution + signals, panel closes on save

---

## PHASE 3: LOCAL DEV TESTING (You do this, but I'll note requirements)

### Testing Workflow
1. **Start dev mode:** `cargo tauri dev`
2. **Test bugs:**
   - Backup works (create archive, restore to different folder)
   - LOCAL path is `C:\ReOrganized` (check filesystem)
   - Error handling shows errors (trigger one)
3. **Test Učebna Phase 1:**
   - Card grid is now list layout ✓
   - Filename visible ✓
   - Folder origin visible ✓
   - Confidence badges display (color-coded) ✓
   - Signal pills show ✓
4. **Test Učebna Phase 2:**
   - Click card → side panel opens ✓
   - PDF preview appears ✓
   - Can select institution ✓
   - Can choose strongest signal ✓
   - Panel closes on save ✓
5. **No regressions:**
   - Scan mode still works
   - Dashboard still loads
   - No console errors

### Sign-Off Criteria
You confirm: "All bugs + Učebna Phase 1-2 working in dev, ready for build"

---

## PHASE 4: v1.0.12 BUILD (Claude Code after dev approval)

Once you confirm local testing passes:
```bash
cargo tauri build
```

Output: `FileIT-v1.0.12.msi`

---

## PHASE 5: CNB REGISTRY DECISION (Parallel)

**You do (now):**
1. Send `SUBAGENT_TASK_CNB_Registry_Analysis.md` to subagent
2. Subagent returns: schema + samples + extraction strategy (1-2 hrs)

**Decision:**
- **If simple extraction (<1 hr):** Include CNB registry in v1.0.12 build
- **If complex (>2 hrs):** Defer to v1.0.13

**Claude Code action (if included):**
- Build WS JERRS scraper or ARES API wrapper (per subagent recommendation)
- Generate canonical lookup table JSON
- Merge into `src-tauri/src/cm_registry.rs`
- Add to v1.0.12 build

---

## EXECUTION CHECKLIST

### Code Changes Required

| File | Change | Priority |
|------|--------|----------|
| `src-tauri/src/backup/mod.rs` | Fix backup ZIP + manifest + restore | 🔴 CRITICAL |
| `src-tauri/lib.rs` or `.env` | Fix LOCAL-ONLY default path | 🔴 CRITICAL |
| `src-tauri/src/restructure/mod.rs` | Add error logging | 🔴 CRITICAL |
| `src/components/Ucebna.tsx` | Redesign card list + UI | 🟡 HIGH |
| `src/components/UcebnaPreview.tsx` | Create preview modal (new file) | 🟡 HIGH |

### Dependencies to Check
- React hooks (useState) available
- Tauri file system API available
- JSON serialization for error logs
- CSS variables for Neon Twilight theme

### Timeline
- Bugs: 2-3 hrs
- Učebna Phase 1-2: 3-4 hrs
- **Total code time:** 5-7 hrs
- Local testing (you): 1-2 hrs
- Build: 30 min
- **Total:** ~8-10 hrs

---

## SUCCESS CRITERIA

✅ Backup works end-to-end (create, restore)  
✅ LOCAL path is `C:\ReOrganized`  
✅ Errors are visible + logged  
✅ Učebna Phase 1 UI renders correctly  
✅ Učebna Phase 2 modal works (preview + selection)  
✅ No regressions in existing features  
✅ v1.0.12.msi builds successfully  

---

## HANDOFF COMPLETE

**To Claude Code:** Read this document, start with bug diagnosis (Phase 1). 

**To Jarda:** You'll test locally when Phase 1-2 are done. Meanwhile, send subagent task.

**To me (Claude planning):** I'm coordinating; call on me if you hit blockers or need research on API structures.

---

**Execution starts NOW.** 🚀

**Next message:** Claude Code's first prompt for bug diagnosis.
