# CLAUDE CODE PROMPT #2: Učebna Phase 1-2 UX Implementation
**Task:** Redesign Učebna component (Phase 1-2)  
**Estimated time:** 3-4 hours  
**Files:** `src/components/Ucebna.tsx` (modify) + `src/components/UcebnaPreview.tsx` (create)  
**Reference:** `CLAUDE_DESIGN_BRIEF_UcebnaPhase1.md`

---

## PHASE 1: Information Density Redesign (Ucebna.tsx)

### Current State (v1.0.11)
- Card grid layout
- Shows pattern clusters (not individual files)
- Clickable to expand
- Shows thumbnails, color swatches, signal pills

### Desired State (Phase 1)
- List layout (vertical stack)
- Shows individual unrecognized files
- Each card displays:
  - **File icon** (📄 for PDF, 📊 for images, etc.) — 48×48px
  - **Filename** (bold, 14px) — original masked format, e.g., `[BANK]_2024_statement_01.pdf`
  - **Folder origin** (12px muted) — e.g., `📁 From: C:\Downloads\Air Bank\2024`
  - **Confidence badge** (right side) — color-coded (🟢 >75%, 🟡 50-75%, 🔴 <50%)
  - **Signal pills** — keywords matched, text similarity %, color detected, logo status, path hints

### Implementation Details

#### Layout Structure
```jsx
// Pseudo-structure
<div className="ucebna-container">
  <header>
    <h3>Učebna: Unrecognized Files (8 remaining)</h3>
    <p>Help the system learn by classifying...</p>
  </header>
  
  <div className="file-cards">
    {unrecognizedFiles.map(file => (
      <FileCard 
        file={file} 
        onClick={() => openPreviewPanel(file)}
      />
    ))}
  </div>
</div>
```

#### FileCard Component
```jsx
<div 
  className="file-card"
  onClick={onClickCard}
  style={{
    display: 'flex',
    alignItems: 'flex-start',
    gap: '12px',
    padding: '12px',
    border: '0.5px solid var(--border)',
    borderRadius: '12px',
    cursor: 'pointer',
    transition: 'background 0.15s'
  }}
  onMouseOver={(e) => e.currentTarget.style.background = 'var(--surface-1)'}
  onMouseOut={(e) => e.currentTarget.style.background = 'var(--surface-2)'}
>
  {/* File icon */}
  <div style={{
    width: '48px',
    height: '48px',
    background: 'var(--bg-accent)',
    borderRadius: '8px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: '20px'
  }}>
    📄
  </div>

  {/* Content */}
  <div style={{ flex: 1, minWidth: 0 }}>
    {/* Filename + Confidence */}
    <div style={{ display: 'flex', alignItems: 'baseline', gap: '8px', marginBottom: '6px' }}>
      <span style={{
        fontWeight: 500,
        fontSize: '14px',
        color: 'var(--text-primary)',
        wordBreak: 'break-word'
      }}>
        {file.filename}
      </span>
      <span style={{
        fontSize: '11px',
        background: getConfidenceColor(file.confidence),
        color: getConfidenceTextColor(file.confidence),
        padding: '2px 6px',
        borderRadius: 'var(--radius)',
        flexShrink: 0,
        whiteSpace: 'nowrap'
      }}>
        {file.confidence}% guess
      </span>
    </div>

    {/* Folder origin */}
    <div style={{
      fontSize: '12px',
      color: 'var(--text-secondary)',
      marginBottom: '4px'
    }}>
      📁 <span style={{
        display: 'inline-block',
        padding: '0 4px',
        background: 'var(--bg-success)',
        color: 'var(--text-success)',
        borderRadius: '4px',
        fontSize: '11px'
      }}>
        {file.folderPath}
      </span>
    </div>

    {/* Signal pills */}
    <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap' }}>
      {file.signals.map(signal => (
        <span key={signal.id} style={{
          fontSize: '11px',
          background: 'var(--bg-accent)',
          color: 'var(--text-accent)',
          padding: '2px 6px',
          borderRadius: 'var(--radius)'
        }}>
          {signal.label}
        </span>
      ))}
    </div>
  </div>

  {/* Arrow indicator */}
  <div style={{
    flexShrink: 0,
    width: '32px',
    height: '32px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    background: 'var(--fill-field)',
    borderRadius: 'var(--radius)',
    color: 'var(--text-secondary)'
  }}>
    ➜
  </div>
</div>
```

#### Confidence Color Helper
```jsx
function getConfidenceColor(confidence) {
  if (confidence >= 75) return 'var(--bg-success)'; // green
  if (confidence >= 50) return 'var(--bg-warning)'; // yellow
  return 'var(--bg-danger)'; // red
}

function getConfidenceTextColor(confidence) {
  if (confidence >= 75) return 'var(--text-success)';
  if (confidence >= 50) return 'var(--text-warning)';
  return 'var(--text-danger)';
}
```

#### CSS Variables Used
- `--surface-1`, `--surface-2` — card backgrounds
- `--border` — hairline borders
- `--text-primary`, `--text-secondary` — text colors
- `--bg-success`, `--text-success` — green (high confidence)
- `--bg-warning`, `--text-warning` — yellow (medium confidence)
- `--bg-danger`, `--text-danger` — red (low confidence)
- `--bg-accent`, `--text-accent` — indigo (signal pills)
- `--radius` — 8px corner radius
- `--fill-field` — button background

### State Management
```jsx
const [unrecognizedFiles, setUnrecognizedFiles] = useState([]);
const [selectedFile, setSelectedFile] = useState(null);
const [previewPanelOpen, setPreviewPanelOpen] = useState(false);

function handleCardClick(file) {
  setSelectedFile(file);
  setPreviewPanelOpen(true);
}
```

### Data Structure Expected
```typescript
interface UnrecognizedFile {
  id: string;
  filename: string; // original masked format
  folderPath: string; // e.g., "C:\Downloads\Air Bank\2024"
  confidence: number; // 0-100
  signals: Array<{
    id: string;
    label: string; // "keyword: statement" or "text match: 78%"
  }>;
  // ... other fields for Phase 2 (preview modal)
}
```

---

## PHASE 2: File Preview + Suggestions (UcebnaPreview.tsx - NEW FILE)

### Create New Component: `src/components/UcebnaPreview.tsx`

#### Structure
```jsx
import React, { useState } from 'react';

interface UcebnaPreviewProps {
  file: UnrecognizedFile;
  isOpen: boolean;
  onClose: () => void;
  onConfirm: (institution: string, signals: string[]) => void;
}

export function UcebnaPreview({ file, isOpen, onClose, onConfirm }: UcebnaPreviewProps) {
  const [selectedInstitution, setSelectedInstitution] = useState<string>('');
  const [selectedSignals, setSelectedSignals] = useState<Set<string>>(new Set());

  if (!isOpen) return null;

  return (
    <div style={{
      position: 'fixed',
      right: 0,
      top: 0,
      bottom: 0,
      width: '30%',
      background: 'var(--surface-2)',
      borderLeft: '0.5px solid var(--border)',
      padding: '1.5rem',
      overflowY: 'auto',
      zIndex: 1000
    }}>
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginBottom: '1.5rem'
      }}>
        <h3 style={{ margin: 0, fontSize: '16px', fontWeight: 500 }}>
          Classify File
        </h3>
        <button
          onClick={onClose}
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

      {/* File info */}
      <div style={{ marginBottom: '1.5rem' }}>
        <p style={{ margin: '0 0 0.5rem', fontSize: '12px', color: 'var(--text-secondary)' }}>
          File
        </p>
        <p style={{ margin: 0, fontSize: '14px', fontWeight: 500, wordBreak: 'break-word' }}>
          {file.filename}
        </p>
      </div>

      {/* File preview */}
      <div style={{
        marginBottom: '1.5rem',
        background: 'var(--surface-1)',
        borderRadius: '12px',
        padding: '1rem',
        minHeight: '200px',
        border: '0.5px solid var(--border)'
      }}>
        <p style={{ margin: '0 0 0.5rem', fontSize: '12px', color: 'var(--text-secondary)' }}>
          Preview
        </p>
        {/* TODO: Render PDF thumbnail here (Phase 2 enhancement) */}
        {/* TODO: Render OCR text snippet here (Phase 2 enhancement) */}
        <p style={{ margin: 0, fontSize: '13px', color: 'var(--text-muted)' }}>
          [PDF preview goes here]
        </p>
      </div>

      {/* Institution picker */}
      <div style={{ marginBottom: '1.5rem' }}>
        <label style={{
          display: 'block',
          marginBottom: '0.5rem',
          fontSize: '12px',
          color: 'var(--text-secondary)'
        }}>
          Which institution?
        </label>
        <select
          value={selectedInstitution}
          onChange={(e) => setSelectedInstitution(e.target.value)}
          style={{
            width: '100%',
            padding: '8px 12px',
            borderRadius: 'var(--radius)',
            border: '0.5px solid var(--border)',
            background: 'var(--surface-2)',
            color: 'var(--text-primary)',
            fontSize: '13px'
          }}
        >
          <option value="">Select an institution...</option>
          {/* TODO: Populate with institutions from backend */}
          <option value="air_bank">Air Bank</option>
          <option value="ceska_sporitelna">Česká spořitelna</option>
          <option value="kooperativa">Kooperativa</option>
          {/* ... etc */}
        </select>
      </div>

      {/* Signal checkboxes */}
      <div style={{ marginBottom: '1.5rem' }}>
        <p style={{
          margin: '0 0 0.5rem',
          fontSize: '12px',
          color: 'var(--text-secondary)'
        }}>
          What was the strongest signal?
        </p>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {[
            { id: 'filename', label: 'Filename keyword' },
            { id: 'color', label: 'Color branding' },
            { id: 'logo', label: 'Logo/visual' },
            { id: 'text', label: 'Document content (text)' },
            { id: 'path', label: 'Original folder path' }
          ].map(signal => (
            <label key={signal.id} style={{
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              cursor: 'pointer',
              fontSize: '13px'
            }}>
              <input
                type="checkbox"
                checked={selectedSignals.has(signal.id)}
                onChange={(e) => {
                  const newSignals = new Set(selectedSignals);
                  if (e.target.checked) {
                    newSignals.add(signal.id);
                  } else {
                    newSignals.delete(signal.id);
                  }
                  setSelectedSignals(newSignals);
                }}
              />
              <span>{signal.label}</span>
            </label>
          ))}
        </div>
      </div>

      {/* Action buttons */}
      <div style={{
        display: 'flex',
        gap: '8px',
        marginTop: '2rem'
      }}>
        <button
          onClick={onClose}
          style={{
            flex: 1,
            padding: '8px 12px',
            borderRadius: 'var(--radius)',
            border: '0.5px solid var(--border)',
            background: 'transparent',
            cursor: 'pointer',
            fontSize: '13px',
            color: 'var(--text-primary)'
          }}
        >
          Cancel
        </button>
        <button
          onClick={() => {
            if (selectedInstitution) {
              onConfirm(selectedInstitution, Array.from(selectedSignals));
              onClose();
            }
          }}
          disabled={!selectedInstitution}
          style={{
            flex: 1,
            padding: '8px 12px',
            borderRadius: 'var(--radius)',
            border: '0.5px solid var(--border)',
            background: selectedInstitution ? 'var(--fill-accent)' : 'var(--fill-disabled)',
            cursor: selectedInstitution ? 'pointer' : 'not-allowed',
            fontSize: '13px',
            color: selectedInstitution ? 'var(--on-accent)' : 'var(--text-disabled)',
            fontWeight: 500
          }}
        >
          Save
        </button>
      </div>
    </div>
  );
}
```

#### Wiring in Ucebna.tsx
```jsx
import { UcebnaPreview } from './UcebnaPreview';

export function Ucebna() {
  const [selectedFile, setSelectedFile] = useState<UnrecognizedFile | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);

  function handleFileCardClick(file: UnrecognizedFile) {
    setSelectedFile(file);
    setPreviewOpen(true);
  }

  function handlePreviewConfirm(institution: string, signals: string[]) {
    // TODO: Call backend to log user's classification choice
    // This triggers Phase 3 workflow (but we're not building that today)
    console.log('User classified file as:', institution, signals);
    // Don't trigger rescan yet — that's Phase 3
  }

  return (
    <>
      {/* Phase 1 list layout */}
      <div className="ucebna-container">
        {/* ... file cards ... */}
      </div>

      {/* Phase 2 preview panel */}
      {selectedFile && (
        <UcebnaPreview
          file={selectedFile}
          isOpen={previewOpen}
          onClose={() => {
            setPreviewOpen(false);
            setSelectedFile(null);
          }}
          onConfirm={handlePreviewConfirm}
        />
      )}
    </>
  );
}
```

---

## TESTING IN `cargo tauri dev`

### Phase 1 Tests
```
1. Ucebna renders with list layout ✓
2. Each file card shows:
   - File icon ✓
   - Filename (masked format) ✓
   - Folder origin badge ✓
   - Confidence badge (correct color) ✓
   - Signal pills ✓
3. Hover effect works (background tint) ✓
4. Click card does NOT crash (even if Phase 2 not wired yet) ✓
```

### Phase 2 Tests
```
1. Click card → side panel opens ✓
2. Panel shows file info ✓
3. Panel shows institution dropdown (populated) ✓
4. Can select institution ✓
5. Can check "strongest signal" checkboxes ✓
6. Save button disabled until institution selected ✓
7. Click Save → panel closes, file state updates ✓
8. Close button (✕) works ✓
```

### No Regressions
```
1. Other tabs/modes still work ✓
2. No console errors ✓
3. No visual glitches ✓
```

---

## DELIVERABLES

### Modified Files
- `src/components/Ucebna.tsx` → Phase 1-2 implementation

### New Files
- `src/components/UcebnaPreview.tsx` → Phase 2 preview panel

### Success Criteria
✅ Učebna Phase 1 renders correctly (list, filename, origin, confidence, signals)  
✅ Učebna Phase 2 modal opens + allows classification + closes  
✅ No crashes or console errors  
✅ All CSS uses Neon Twilight system (no hardcoded colors)  
✅ Responsive (works on 1440px and narrower)  

---

## AFTER THIS PROMPT

Once you finish Bug Fixes (Prompt #1), move to this (Prompt #2).

Test both in `cargo tauri dev` and confirm with Jarda.

Then we build v1.0.12. 🚀
