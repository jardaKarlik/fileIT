# Bug Extraction Status - v1.0.9

**Date**: 2026-06-09
**Status**: 🤖 Awaiting ClickUp Session Response
**Purpose**: Extract clear bug specifications for remaining 3 bug fixes

---

## What We Know So Far

### ✅ CLEARLY DEFINED (Already being fixed):

**BUG #3: Local Default Path**
- **Status**: CLEAR ✅
- **Issue**: For LOCAL-ONLY mode, default path should be C:\ReOrganized instead of cloud location
- **Sub-Agent**: a3bf5d00d39fc584b working on this
- **Files**: src-tauri/src/commands/mod.rs, cm_registry.rs, src/store/index.ts
- **Spec**: Conditional path selection based on LOCAL vs CLOUD mode

---

### ❓ NEEDS CLARIFICATION (Awaiting extraction):

**BUG #1: Timeline Chart x-axis**
- **Title**: "timeline on scan result screen shows last year only. it should be er year and not month"
- **Sub-Agent**: a87f70af7b9d6e846 working on this
- **What We Think**: Chart x-axis should show YEARS (2024, 2025, 2026) instead of MONTHS (Jan, Feb, Mar...)
- **Chart Image**: There's a chart image on this ClickUp card - NEED TO DESCRIBE IT
- **Status**: UNCLEAR ❓
- **Questions**:
  - What exactly does the chart currently show?
  - Is it months or something else?
  - Should it show years, decades, or what range?
  - Historical data or current year only?

**BUG #2: Backup Not Working**
- **Title**: "zaloha nefunguje - fix needed. whole process check too"
  (zaloha = backup in Czech)
- **Sub-Agent**: a4519be18a0b4c38d working on this
- **What We Know**: The backup process is broken somehow
- **Status**: VERY UNCLEAR ❓
- **Questions**:
  - Does backup creation fail?
  - Does restore fail?
  - Does manifest not save?
  - Specific error message?
  - When did it start failing?
  - Is it related to our recent Phase 1 changes?

---

## What We're Waiting For

### From ClickUp Session:

**Immediate Request Sent**: Extract bug specifications with:
1. Exact description from ClickUp task
2. Any images/attachments (especially the chart for BUG #1)
3. Description of what the image/chart shows
4. Any comments with more details
5. Clarification on what's unclear

**Expected Response Format**:
```
BUG #1 (Timeline)
Description: [exact text from ClickUp]
Chart Shows: [describe the chart image]
Status: Clear / Unclear

BUG #2 (Backup)
Description: [exact text from ClickUp]
Error: [if any]
Status: Clear / Unclear

BUG #3 (Local Path)
Description: [exact text from ClickUp]
Status: CONFIRMED CLEAR
```

---

## Build Pipeline Impact

### Current State:
- Cargo build: 🔨 IN PROGRESS (should be done soon)
- Sub-agents: 🤖 WORKING (waiting for specs to be confirmed)
- Awaiting: Clear bug specifications from ClickUp

### Blocking:
- Sub-agents have GENERAL specs but need EXACT requirements to proceed
- Build can't finalize until all fixes are confirmed

### Timeline Impact:
- +10 min: Once we get clear specs
- +5 min: Sub-agents to implement with confirmed specs
- +10 min: Rebuild if changes needed
- +5 min: Final verification

**Total**: ~30 min to completion once we have clear specs

---

## Tasks in TO DO Column

From page text extraction, we know there are **11 total TO DO tasks**:

**Known (6)**:
1. Phase 2: OCR Integration (High Priority) - Future phase
2. Phase 1: Backup Robustness (Critical) - Already implemented ✅
3. Phase 0: Source Path Selection - Already implemented ✅
4. Timeline chart bug - BUG #1 ❓
5. Backup not working - BUG #2 ❓
6. Local default path - BUG #3 ✅

**Unknown (5)**:
- 7-11: Need extraction from ClickUp session

---

## Next Steps

### When ClickUp Session Responds:
1. ✅ Read extracted descriptions
2. ✅ Confirm understanding of each bug
3. ✅ Ask follow-up questions if still unclear
4. ✅ Send confirmed specs to sub-agents
5. ✅ Monitor sub-agent progress

### Then Proceed With:
1. Apply sub-agent fixes
2. Rebuild if needed
3. Run final tauri build
4. Generate installers
5. Release v1.0.9

---

## Clarification Questions for ClickUp Session

If the extraction is still unclear after receiving it, here are the follow-up questions:

**For BUG #1 (Timeline)**:
- Current behavior: What does the chart show on x-axis right now?
- Expected behavior: What should the x-axis show instead?
- Timeline range: Single year or multiple years?
- Grouping: By year, quarter, month, or something else?

**For BUG #2 (Backup)**:
- When does it fail: During creation or restore?
- Error message: Any specific error text?
- Files affected: Does it fail on all files or specific types?
- Related to recent changes: Did it work before Phase 1 changes?
- What specifically is "whole process check too"?

**For remaining 8 tasks**:
- Are these sub-tasks or separate bugs?
- Priority compared to the 3 main bugs?
- Should we work on them in v1.0.9?

---

## Escalation Path

If ClickUp session doesn't respond within 15 minutes:
1. Mark as escalation needed
2. Proceed with educated guesses based on existing context
3. Document assumptions made
4. Schedule follow-up clarification

---

**Status**: Awaiting ClickUp Session Response  
**Time Sent**: ~10:08 UTC  
**Expected Response**: 10:15-10:25 UTC  
**Current Time**: Check latest notification
