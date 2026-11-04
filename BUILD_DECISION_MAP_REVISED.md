# FileIT v1.0.12 BUILD DECISION MAP (REVISED)
**Based on Jarda's corrections and clarifications**  
**Status:** READY FOR SUBAGENT + CLAUDE CODE EXECUTION  
**Date:** 2026-08-11, 10:00 CZ

---

## YOUR CLARIFICATIONS (INTEGRATED)

### 🔧 Key Correction: AI Browser = One-Time Bootstrap
- **NOT:** Real-time AI browser query during classification
- **IS:** One-time static lookup table → baked into app core
- **Runtime:** Fast fingerprint matching against lookup table (deterministic, no compliance risk)

### 🔵 Decision 1: Close Stale ClickUp Tasks
**Your answer:** YES  
**Status:** ✅ 5 tasks already marked complete (just need formal closure)  
**Action:** I'll close them now.

### 🔵 Decision 2: CNB Registry Import (Three-Step)
**Your answer:** Use subagent for analysis  
**Subagent task:** `SUBAGENT_TASK_CNB_Registry_Analysis.md` (ready for you to review + send)  
**What subagent should deliver:**
- Canonical lookup table schema
- Sample data (3-5 real Czech institutions)
- Extraction strategy (which API, how to query)
- Time/complexity estimate
- Clear recommendation (build WS JERRS scraper vs. ARES API vs. hybrid)

**Timeline:**
- Subagent: 1-2 hours
- Claude Code implementation (if approved): 2-3 hours
- **Decision:** Include in v1.0.12 or defer to v1.0.13 after subagent report?

### 🔵 Decision 3: Skills Import
**Your answer:** Import locally (project-only), don't go global  
**Skills to add:**
- `document-audit-extraction` → metadata scoring
- `scan-organizer` → domain recognition (insurance, banking)
- `pdf-extractor` → preview generation (Učebna Phase 2)

**Timing:** After bug fixes are tested (Decision 5 below)

### 🔵 Decision 4: Build Timing
**Your answer:** Test bugs in dev first (`cargo tauri dev`)  
**Then:** Decide what goes into v1.0.12 build  
**Workflow:**
1. Fix bugs locally, test in dev
2. You confirm bugs work in dev
3. Decide: build now or add Učebna Phase 1 UX too?
4. If yes to UX: also test Phase 1 in dev
5. Once confirmed, trigger v1.0.12 build

**Action:** I prep focused bug-fix prompts for Claude Code. You test. Then decide scope.

### 🔵 Decision 5: Učebna Phase 1 (Implied from Your Brief)
**Your earlier request:** Implement filename + origin + confidence badges  
**Confirm?** Should this go in v1.0.12 or wait for Phase 2?

---

## PHASED EXECUTION PLAN

### PHASE 1: CNB Registry Analysis (Parallel, right now)

**You do:**
1. Read `SUBAGENT_TASK_CNB_Registry_Analysis.md`
2. Send to subagent (or other analyst)
3. Get back: schema + samples + extraction strategy

**Subagent deliverables (1-2 hrs):**
- JSON schema for canonical lookup table
- Sample institutions (3-5 real entries)
- Pseudocode extraction strategy
- Feasibility assessment
- Recommendation (build / defer / hybrid)

**Claude does (meanwhile):**
- Close 5 stale ClickUp tasks
- Prep Claude Code prompts for bug fixes

---

### PHASE 2: Bug Fixes + Local Testing (Today)

**You confirm:** Fix 3 critical bugs?
- Backup process (869dm1qc4)
- LOCAL-ONLY default path (869dm1qex)
- Error handling (869dm1t3h)

**Claude Code work** (2-3 hrs):
- Read source files, diagnose issues
- Write fix implementations
- Test in `cargo tauri dev`

**You test** (30 min):
- Run dev mode locally
- Confirm bugs are fixed
- Note any regressions

**Outcome:** "Bugs work in dev, ready for v1.0.12 build" ✅

---

### PHASE 3: Učebna Phase 1 UX (If Approved)

**You confirm:** Add Phase 1 UX to v1.0.12 scope?
- Filename display (masked format)
- Folder origin badge
- Confidence score badges (color-coded)
- Signal pills + preview modal wiring

**Claude Code work** (2-3 hrs):
- Redesign `src/components/Ucebna.tsx`
- Create `src/components/UcebnaPreview.tsx`
- Wire preview modal

**You test** (30 min):
- Confirm Učebna shows new UI
- Confirm filename + origin visible
- Confirm badges display correctly

**Outcome:** "Učebna Phase 1 works in dev, ready for v1.0.12 build" ✅

---

### PHASE 4: CNB Registry Decision Point

**After subagent report arrives:**

**Option A: Include in v1.0.12**
- Claude Code builds WS JERRS scraper (or ARES API wrapper)
- Generates canonical lookup table JSON
- Merges into `cm_registry.rs`
- Adds 2-3 hrs to build time

**Option B: Defer to v1.0.13**
- Ship v1.0.12 with bugs + Učebna Phase 1
- Use v1.0.13 for CNB registry bootstrap + Phase 2 (preview modal)

**Option C: Hybrid**
- Do research now, build scraper for v1.0.13
- But create a placeholder in v1.0.12 for easy data injection

**My recommendation:** Wait for subagent output, then decide. If extraction is simple (~1 hr), include it. If complex (>2 hrs), defer.

---

### PHASE 5: v1.0.12 Build

**Trigger:** After phases 2-4 are confirmed locally in `cargo tauri dev`

**Claude Code:** `cargo tauri build` → FileIT-v1.0.12.msi

**You:** Manual testing of MSI
- Backup works end-to-end
- Default path correct
- Učebna shows new UI
- No regressions

**Parallel:** While you test MSI, I design Phase 3 (smart rescanning workflow)

---

## TIMELINE SUMMARY

| Phase | Task | Owner | Time | Status |
|-------|------|-------|------|--------|
| **0** | CNB Registry Analysis | Subagent | 1-2 hrs | **BLOCKED** (awaiting your send to subagent) |
| **1** | Bug Fixes (3 tasks) | Claude Code | 2-3 hrs | Ready to execute |
| **2** | Local bug testing | You | 30 min | Awaiting Phase 1 done |
| **3** | Učebna Phase 1 UX | Claude Code | 2-3 hrs | Ready to execute (if approved) |
| **4** | Local UX testing | You | 30 min | Awaiting Phase 3 done |
| **5** | CNB Registry decision | Jarda + Claude | - | Awaiting subagent report |
| **6** | v1.0.12 Build | Claude Code | 30 min | Awaiting phases 2-4 confirmed |
| **7** | MSI Testing | You | 1-2 hrs | Awaiting Phase 6 done |

**Total:** ~8-10 hrs from now, depending on CNB registry decision

---

## IMMEDIATE NEXT STEPS (NOW)

### For Jarda:
1. ✅ Review `SUBAGENT_TASK_CNB_Registry_Analysis.md`
2. ⏸️ Send to subagent (when ready)
3. ✅ Confirm: Fix 3 bugs?
4. ✅ Confirm: Add Učebna Phase 1 to v1.0.12?
5. ⏸️ Wait for subagent report on CNB registry

### For Claude:
1. ✅ Close 5 stale ClickUp tasks (start now)
2. ⏸️ Wait for Jarda's confirmation on bugs + UX
3. ⏸️ Prep Claude Code prompts (ready to go once confirmed)
4. ⏸️ Wait for subagent CNB report

---

## RISK MITIGATION

**If subagent report takes >2 hrs:**
- Proceed without CNB registry in v1.0.12
- Ship bugs + Učebna Phase 1 now
- Add CNB registry to v1.0.13

**If bug fixes discover more issues:**
- Ship v1.0.12a patch same day
- Log issues, address in v1.0.13

**If Učebna Phase 1 needs design review:**
- Ship with basic styling
- v1.0.12a polish pass after Claude Design feedback

---

## DECISION POINTS FOR JARDA

### ✅ Are you ready to:
1. Send `SUBAGENT_TASK_CNB_Registry_Analysis.md` to subagent?
2. Fix 3 critical bugs today (in Claude Code)?
3. Add Učebna Phase 1 UX to v1.0.12?
4. Wait for subagent CNB report before final build decision?
5. Use the revised timeline above?

**Reply format:**
```
1. YES / AWAITING [details]
2. YES / NO / CHANGES
3. YES / NO / DEFER TO 1.0.13
4. YES / NO
5. YES / CHANGES [list]
```

---

**Report prepared:** 2026-08-11, 10:05 CZ  
**Status:** AWAITING JARDA'S CONFIRMATION ON 5 DECISION POINTS  
**Claude Code:** Standing by for confirmation to start Phase 1 (bug fixes)
