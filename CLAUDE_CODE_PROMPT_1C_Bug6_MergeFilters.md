# CLAUDE CODE PROMPT #1C: Bug #6 — Merge & Rename Institution Filters
**Task:** 869egg8f6 (add to scope)  
**Estimated time:** 30 min  
**Build:** FileIT v1.0.12 (quick UI cleanup)  
**Status:** After all 5 bugs tested, fix this one

---

## BUG #6: Confusing Dual Institution Filters (UI Cleanup)

### Problem
Two institution filters doing essentially the same thing:
1. "Vydávající instituce" (Issuing Institution) — 242 files (85%)
2. "Instituce" (Institution) — secondary mentions, contextual

**Issue:** Secondary detection is unreliable (contextual, not mathematical), adds complexity for minimal value. Should merge into single authoritative filter.

### Root Cause
- Original design assumed two levels of institution matching
- Secondary detection (mentioned in document) requires contextual analysis
- Multiple mentions make "primary" vs "secondary" ambiguous
- Not worth the complexity

### Solution: MERGE & RENAME

**Keep:** One institution filter box  
**Rename:** "Vydávající instituce" → **"Nejčastěji Zmíněná"** (Most Frequently Mentioned)  
**Remove:** The "Instituce" secondary filter entirely  
**Data:** Keep existing 242 souborů (85 %) data structure

---

## Implementation

### File: Restructure Filter Component
**File:** `src/components/Restructure/Restructure.tsx` (or wherever institution filters live)

**Before:**
```jsx
<FilterBox 
  label="Vydávající instituce"
  count={242}
  percentage={85}
/>

<FilterBox 
  label="Instituce"
  count={2}
  note="Použito na úrovni 2"
/>
```

**After:**
```jsx
<FilterBox 
  label="Nejčastěji Zmíněná"
  count={242}
  percentage={85}
  tooltip="Institution detected in document (issuer)"
/>

{/* Remove the secondary "Instituce" filter entirely */}
```

### Changes Required

1. **Rename filter label:**
   - "Vydávající instituce" → "Nejčastěji Zmíněná"

2. **Update filter logic:**
   - Combine both institution detection into single classifier
   - Use primary issuer detection (most reliable signal)
   - Remove secondary mention detection from this filter

3. **Clean up UI:**
   - Remove the secondary "Instituce" filter box
   - Keep the data (242 files, 85%) for the merged filter
   - Optional: Add tooltip explaining the filter

4. **Optional CSS updates:**
   - If secondary filter had unique styling, consolidate into primary
   - Keep icon color/badge consistent

---

## Testing in `cargo tauri dev`

**Before testing:** Confirm current state shows 2 institution filters

**After fix:**
```
1. Restructure screen loads
2. Confirm only ONE institution filter visible: "Nejčastěji Zmíněná"
3. Confirm shows: "242 souborů (85%)"
4. Confirm secondary "Instituce" filter is gone
5. Reorganize still works with merged filter
6. No regressions in other filters
```

---

## Success Criteria

✅ One institution filter visible (not two)  
✅ Filter renamed to "Nejčastěji Zmíněná"  
✅ Data shows 242 files (85%) — original data preserved  
✅ Secondary filter completely removed  
✅ No UI glitches or layout issues  
✅ Reorganize flow works end-to-end  

---

## Notes

- This is a **UI simplification**, not a logic change
- You're not losing data, just simplifying the filter presentation
- Makes the app cleaner + easier to understand
- Reduces cognitive load (one authoritative institution classifier)

---

## Timeline

**After all 5 bugs pass local testing:**
- Fix Bug #6 (30 min)
- Test Bug #6 (10 min)
- All 6 bugs confirmed ✅
- Move to Učebna Phase 1-2 UX (3-4 hrs)

---

**Your brother's orders:** This is a quick win after bugs 1-5 are confirmed. Keep the app clean. 🧹
