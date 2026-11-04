# CLAUDE DESIGN BRIEF: FileIT Učebna UX Redesign
**Copy-paste this entire brief into your Claude Design session**

---

## Project Overview
**Product:** FileIT (Czech financial document organizer)  
**Component:** Učebna (Training/Classroom UX for manual file classification)  
**Phase:** 1 (Information Density)  
**Goal:** Help users classify unrecognized documents with minimal cognitive load by surfacing context clues.

**Target Users:** Czech financial advisors (age 30–65) manually organizing client document archives. Files are often named opaquely; folder structure is the most reliable hint.

**Time to Classify (target):** <10 seconds per file (currently ~30 seconds)

---

## Problem Statement

**Current Experience (v1.0.11):**
Users see a card-grid of pattern clusters with:
- Thumbnail image (PDF preview)
- Color swatches (detected brand colors)
- Signal pills (keyword matches, confidence score)

**What's missing:**
- **No filename context** — users don't know the original filename
- **No folder origin** — critical hint about which institution/client
- **No confidence indicator** — users guess if they should trust the suggestion
- **No preview option** — they click blindly, hoping to see more detail

**Real-world scenario:**
*User sees a PDF thumbnail. It could be from Bank A or Bank B (colors are similar). They have no other clues. They guess wrong. System re-scans, they have to re-classify.*

---

## Success Criteria

✅ User can classify a file in <10 seconds  
✅ Filename and folder origin visible at a glance  
✅ Confidence badges (color-coded) show system's certainty  
✅ Signal pills explain *why* the system made a guess  
✅ Click-to-preview available (opens side panel, v1.0.12)  

---

## Design Requirements

### Layout & Structure
**Format:** List cards (not grid)  
**Width:** Full-width, responsive (works on laptop 1440px, tablet 768px, portrait okay)  
**Spacing:** 12px gap between cards  
**Density:** Comfortable (not cramped)

### Card Anatomy
Each unrecognized file shown as a single card containing:

#### 1. Header Row
- **Left:** File icon (📄 for PDF, 📊 for images, etc.) — 48×48px
- **Center:** 
  - **Filename** (bold, 14px) — show original masked format
    - Example: `[BANK]_2024_statement_01.pdf`
    - Masked to protect PII but still readable
  - **Folder origin** (12px, muted) — shows where file came from
    - Example: `📁 From: C:\Downloads\Air Bank\2024`
    - **Compliance note:** This is the key context for users
- **Right:** Confidence badge (12px font)
  - Format: `87% guess` or `87% confident`
  - Color: 🟢 green (75%+), 🟡 yellow (50-75%), 🔴 red (<50%)
  - Use semantic background colors (bg-success, bg-warning, bg-danger)

#### 2. Signal Pills Row
Multiple short pills showing which fingerprints matched:
- `keyword: statement` (text keyword found)
- `text match: 78%` (OCR content similarity)
- `color: RBAC Blue` (brand color detected)
- `logo: unclear` (logo detected but weak match)
- `path: bank folder` (original folder hints at institution)

**Style:** Muted background, 11px font, 4px padding

#### 3. Click Target
Entire card is clickable (whole area, not just a button)  
Cursor changes to pointer on hover  
Background color shifts slightly (var(--surface-1)) on hover  
**Destination:** Opens a side panel (Phase 2, v1.0.12)

---

## Visual Design Specs

### Colors
Use **Neon Twilight Design System:**
- **Primary accent:** Indigo (`--text-accent`, `--border-accent`)
- **Success:** Green (`--text-success`, `--bg-success`) — high confidence
- **Warning:** Amber (`--text-warning`, `--bg-warning`) — medium confidence
- **Danger:** Red (`--text-danger`, `--bg-danger`) — low confidence
- **Neutral:** Gray for structural elements (`--border`, `--surface-1`)

### Typography
- **Filename:** 14px, font-weight 500 (medium)
- **Folder origin & supporting text:** 12px, color var(--text-secondary)
- **Signal pills:** 11px, color var(--text-primary) on tinted backgrounds
- **No title case:** All sentence case (e.g., "From: C:\Downloads")

### Spacing & Radius
- **Card padding:** 12px
- **Icon size:** 48×48px
- **Icon to text gap:** 12px
- **Card border-radius:** 12px
- **Pill border-radius:** var(--radius) (8px)
- **Card border:** 0.5px solid var(--border)
- **Card background:** var(--surface-2) or white

### Micro-Interactions
- **Hover:** Background shifts to var(--surface-1), cursor becomes pointer
- **Click feedback:** Slight scale down (98%) for 0.15s
- **Disabled state:** Not applicable (cards are always active)
- **Loading state:** Skeleton card outline while fetching preview (Phase 2)

---

## Wireframe Specs (Text-Based)

```
┌─ Učebna: Unrecognized Files (8 remaining) ────────────────────────┐
│                                                                       │
│ 📄  [BANK]_2024_statement_Q2.pdf          [🟢 87% guess]          │
│     📁 From: C:\Downloads\Air Bank\2024                             │
│     keyword: statement  | text match: 78%  | color: RBAC Blue       │
│                                                                       │
│ 📊  RAB_pojisteni_2024.pdf                [🟡 52% guess]          │
│     📁 From: C:\temp\Documents\Insurance                             │
│     keyword: pojisteni  | logo: unclear                              │
│                                                                       │
│ ❓  invoice_2024_07.pdf                   [🔴 18% guess]          │
│     📁 From: C:\Users\Download                                       │
│     keyword: invoice  | no patterns matched                          │
│                                                                       │
└───────────────────────────────────────────────────────────────────┘
```

---

## Phase 2 Expansion (v1.0.12 Follow-up)
*(You'll design this next, but context for Phase 1 consistency)*

When user clicks a card:
- **Side panel opens** (30% of screen width, right side)
- **Shows:** File thumbnail (PDF first page), OCR text snippet
- **Picker:** "Which institution?" with dropdown or chips
- **Question:** "What was the strongest signal?" → checkbox group
  - ☐ Filename keyword
  - ☐ Color branding
  - ☐ Logo/visual
  - ☐ Document content (text)
  - ☐ Original folder path

---

## Accessibility & Compliance Notes

✅ **Color is not the only indicator** — badges also have text (87%, 52%, 18%)  
✅ **No PII in filenames** — use masked format ([BANK] instead of actual bank name)  
✅ **Keyboard navigation** — Tab between cards, Enter to open panel  
✅ **Screen reader friendly** — Card describes: "filename, folder origin, confidence"  
✅ **Audit trail ready** — Each user choice is logged (compliance requirement)

---

## Design Files / Resources Needed

From FileIT team:
- ✅ Current Neon Twilight color tokens (indigo, deep pink, coral hex values)
- ✅ Sample filenames (real, masked)
- ✅ Sample folder paths (from test data)
- ✅ Current "8 unrecognized files" example dataset

---

## Questions for Design Review

1. Should the filename be truncated on narrow screens (mobile), or should it wrap?
2. Should we show 1-2 signal pills by default, or all of them?
3. For confidence badges, is color + percentage clear, or do we need an icon too (⭐, 🎯)?
4. Should the folder path be a clickable link (reveals folder in file explorer)? *[Compliance: probably not]*

---

## Next Steps

1. **Design creates:** Mockup of single card + card states (hover, active, disabled)
2. **Review with Claude:** Confirm colors, spacing, hierarchy
3. **Design creates:** Micro-interaction spec (animations, transitions)
4. **Handoff to CODE:** Full component spec for React/TypeScript build

---

**Deliverable Timeline:**
- Today: Mockup + spacing spec ✅
- Tomorrow: Micro-interactions + state variations
- Day 3: Handoff to CODE for implementation

---

**Project:** FileIT v1.0.12  
**Design Phase:** 1 (Information Density)  
**Last updated:** 2026-08-11  
**Status:** READY FOR DESIGN REVIEW
