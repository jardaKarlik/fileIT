# CLAUDE CODE PROMPT #1B: Two Additional Bugs (Urgent)
**Tasks:** 869egg8f6 + 869egg8bh  
**Estimated time:** 1-1.5 hours  
**Build:** FileIT v1.0.12 (add to scope)  
**Status:** After initial 3 bugs tested, fix these 2

---

## BUG #4: Reorganization Screen Missing Load Recent Scan Option (869egg8f6)

### Problem
Mode 2 (Reorganize) screen: user comes to reorganize, but has to re-run scan if they want to use latest analysis. Should have quick option to "Load most recent scan" (1-click).

### Current Behavior
- User goes to Mode 2 (Reorganize)
- If they want to use recent scan: must go back → Mode 1 → run scan again
- Friction: 5+ clicks to reuse existing data

### Desired Behavior
- Mode 2 screen has button: "📁 Load most recent scan"
- Click → loads last scan results (metadata only, no re-processing)
- Fast path: 1 click instead of 5

### Implementation

**File:** `src/components/Restructure/index.tsx` (or similar)

**Add button in Mode 2 view:**
```jsx
<button 
  className="load-recent-scan-btn"
  onClick={handleLoadRecentScan}
  style={{
    padding: '12px 16px',
    borderRadius: 'var(--radius)',
    background: 'var(--fill-accent)',
    color: 'var(--on-accent)',
    fontWeight: 500,
    cursor: 'pointer'
  }}
>
  📁 Load most recent scan ({lastScanDate})
</button>
```

**Implementation steps:**
1. Query backend for last scan metadata (timestamp, file count, institutions detected)
2. Load scan result into state (don't re-process files)
3. Display count: "Scan from {date}, {N} files analyzed"
4. Enable "Spustit reorganizaci" (restructure) button
5. User can now reorganize without re-scanning

### Success Criteria
✅ "Load recent scan" button visible in Mode 2  
✅ Button shows last scan date + file count  
✅ Click loads scan data (no re-processing)  
✅ User can immediately start reorganization  
✅ Fallback: if no recent scan exists, show "Run scan first" message

---

## BUG #5: No Back Button from Учебna Screen (869egg8bh)

### Problem
User is in Učebna (teaching/labeling screen), classifying unrecognized files. When done, they can't get back to the main reorganization screen. No back button, click outside modal doesn't work, ESC key might not work. **User is stuck.**

### Current Behavior
- User clicks a file in list → Phase 2 modal opens
- User classifies file + clicks "Save"
- Modal closes
- User sees next file to classify
- ❌ But: no way to exit Učebna entirely and go back to main reorganization

### Desired Behavior
- Back button (← Back) or header with close (✕)
- Click → dismiss Učebna panel entirely
- Return to Mode 2 (Reorganize) main view
- Show summary: "X files classified, Y remaining"

### Implementation

**File:** `src/components/Ucebna.tsx`

**Add header with back button:**
```jsx
<div style={{
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center',
  marginBottom: '1.5rem',
  borderBottom: '0.5px solid var(--border)',
  paddingBottom: '1rem'
}}>
  <button
    onClick={handleBackToRestructure}
    style={{
      background: 'transparent',
      border: 'none',
      cursor: 'pointer',
      fontSize: '16px',
      color: 'var(--text-primary)',
      display: 'flex',
      alignItems: 'center',
      gap: '6px'
    }}
  >
    ← Back to Reorganize
  </button>
  
  <h3 style={{ margin: 0, fontSize: '16px' }}>
    Classify Files ({classifiedCount}/{totalCount})
  </h3>
  
  <button
    onClick={handleBackToRestructure}
    style={{
      background: 'transparent',
      border: 'none',
      cursor: 'pointer',
      fontSize: '20px',
      color: 'var(--text-secondary)'
    }}
  >
    ✕
  </button>
</div>
```

**Implementation steps:**
1. Add back button + close (✕) in Учебna header
2. `handleBackToRestructure` callback props (passed from parent)
3. On click: close Учебna panel, return to Mode 2 main view
4. Show summary: "X files classified out of Y total"
5. ESC key also triggers back (if modal implementation)

### Success Criteria
✅ Back button visible in Учебna header  
✅ Close (✕) button visible top-right  
✅ Click back → returns to Mode 2 reorganize view  
✅ ESC key also works (if implemented as modal)  
✅ Summary shows progress (X classified / Y total)  
✅ User is not stuck

---

## TESTING IN `cargo tauri dev`

**Bug #4 (Load Recent Scan):**
```
1. Mode 2 → Reorganize
2. Confirm button exists: "📁 Load most recent scan"
3. Click button
4. Verify: scan data loads (shows file count)
5. Verify: can click "Spustit reorganizaci" immediately
```

**Bug #5 (Back Button):**
```
1. Mode 2 → click file → Učebna opens
2. Confirm back button visible (← Back to Reorganize)
3. Confirm close button visible (✕)
4. Click back → returns to Mode 2
5. Verify: shows progress ("5 classified / 12 total")
6. Try ESC key → also closes Učebna
```

---

## NOTES FOR CLAUDE CODE

- These are **quick wins** (1-1.5 hrs total, not blocking)
- Can be merged into existing Mode 2 + Учебna components
- Low risk of regression
- Good UX improvements (discoverability + navigation)

---

## PRIORITY

**After initial 3 bugs are confirmed working in dev, fix these 2.**

No blocking relationship — just better UX for v1.0.12.

---

**Your brother's orders:** Fix these after the first 3 bugs are tested. Then we test all 5 together before build.
