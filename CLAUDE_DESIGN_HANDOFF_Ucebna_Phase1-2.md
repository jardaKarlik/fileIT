# Claude Design Handoff: Učebna Phase 1-2 UX
**Project:** FileIT v1.0.12  
**Component:** Učebna (Teaching/Labeling UI)  
**Constraint:** Intuitive UX — smart design, 3-click max, rich tooltips/descriptions

---

## DESIGN PHILOSOPHY

### Core Principle: Intuitive + Minimal
- **3-click maximum** to classify a file
- **No jargon** — explain domain terms (confidence, signal, institution)
- **Tooltips everywhere** — hover for context
- **Smart defaults** — pre-select most likely option
- **Visual hierarchy** — important info first

### Tone
- **Friendly but professional** — users are financial advisors, not ML engineers
- **Reduce cognitive load** — clear visual feedback, no hidden states
- **Trust through transparency** — show WHY the system thinks something is X

---

## PHASE 1: Information Density (List View)

### Purpose
Replace card grid with list layout showing:
- What file we're looking at
- Why we think it's unrecognized (confidence score + evidence)
- Invitation to help (click to classify)

### Key UX Elements

#### 1. File Card (List Item)
**Click anywhere on card → Phase 2 modal opens**

**Card anatomy (left to right):**
```
[FILE ICON] [FILENAME + ORIGIN] [CONFIDENCE BADGE] [→ ARROW]
            [SIGNAL PILLS]
```

**Content Details:**
- **File icon:** Large (48×48px), clear visual indicator (📄 PDF, 📊 image, etc.)
- **Filename:** Original name (masked for privacy, e.g., `[BANK]_2024_statement_01.pdf`)
  - Tooltip: "Original filename from your folder"
- **Folder origin badge:** `📁 From: C:\Downloads\Air Bank\2024`
  - Tooltip: "Where this file came from — helps the system learn"
- **Confidence badge:** Right-aligned, color-coded
  - 🟢 Green (75%+): "System is fairly confident"
  - 🟡 Yellow (50-75%): "System is uncertain, your help needed"
  - 🔴 Red (<50%): "System has no clue, please classify"
  - **Tooltip on hover:** "Confidence = how certain the system is this is [institution name]"
- **Signal pills:** Small tags showing evidence
  - Example pills: "keyword: statement", "text: 78%", "color: red detected", "logo: not found", "path: Air Bank folder"
  - **Tooltip on each pill:** "This pattern was detected: [specific detail]"
- **Arrow indicator:** Subtle "→" shows card is clickable

#### 2. Visual Hierarchy
- **Bold/large:** Filename (primary info)
- **Medium:** Confidence badge (status)
- **Small/muted:** Folder origin (context)
- **Tiny pills:** Signal evidence (for power users)

#### 3. Hover States
- **Background tint:** Subtle highlight
- **Cursor:** Pointer
- **All pills become readable** (opacity increases)

---

## PHASE 2: File Preview + Classification (Side Panel)

### Purpose
User clicks a card → Panel slides in from right (30% width)
Panel shows: "What is this file, really?"

**3-click flow:**
1. Click card → Panel opens
2. Select institution (dropdown) → Smart pre-selection if confidence >50%
3. Confirm signal checkboxes (optional, smart defaults) → Save

---

### UX Detail: Side Panel Layout

#### Header
```
[✕ Close]  "Classify File"
```
- Close button top-right
- Title centered, clear
- No other chrome

#### Section 1: File Info
```
File
──────────────────
[FILENAME]
[File size: 245 KB, Type: PDF]
```
- **Tooltip on filename:** Click to see full original path
- Show basic file metadata (helps user verify they're classifying the right thing)

#### Section 2: File Preview
```
Preview
──────────────────
[PDF THUMBNAIL - FIRST PAGE]
[OCR TEXT SNIPPET (50 chars, …)]

"Click to expand preview in full-screen"
```
- **Visual:** Show first page thumbnail (200×200px minimum)
- **Text:** Show OCR excerpt from document (key phrases user will recognize)
- **Tooltip:** "This is what the system sees in the document"
- **Hover:** Becomes clickable to expand full preview (optional Phase 2.5 feature)

#### Section 3: Institution Picker
```
Which institution is this?
──────────────────
[Dropdown: Select an institution...]

[If confidence >50%]
💡 "System thinks this might be [Institution Name]. 
    Click to confirm or browse all →"
```
- **Dropdown/Search:** Populate from backend (all known institutions)
- **Smart default:** If system confidence >50%, pre-select that institution
- **Tooltip on dropdown:** "Not sure? Hover institution names to see keywords we use to recognize them"
- **Institution name tooltip:** Show keywords that match (e.g., "Česká spořitelna — keywords: spořitelna, CS")

#### Section 4: Strongest Signal (Optional Feedback)
```
What helped you recognize this? (Optional)
──────────────────
☐ Filename keyword (e.g., "statement")
☐ Color branding (e.g., red logo)
☐ Logo/visual
☐ Document content (text from PDF)
☐ Original folder path

"Hint: Checking these helps us learn faster →"
```
- **Presentation:** Checkboxes (multiple select OK — user might see multiple signals)
- **Smart default:** Pre-check the 1-2 signals with highest confidence
- **Tooltip on group:** "Tell us which signals convinced you. This speeds up our learning."
- **Tooltip on each checkbox:** "[Signal name] — e.g., if you saw 'Air Bank' written in the header"

#### Section 5: Action Buttons
```
[Cancel]  [Save & Classify]
```
- **Cancel:** Close panel, no change (quick abort)
- **Save & Classify:** 
  - Disabled until institution is selected
  - On click: Send choice to backend (log + learn)
  - Success feedback: "✓ Classified! Processing next file..."
  - Panel closes automatically

---

### UX Flow (3-Click Path)

**User journey:**
```
Sees unrecognized file in list
     ↓
[CLICK 1] Card → Panel opens
     ↓
Panel shows: preview + dropdown with pre-selected institution
     ↓
[CLICK 2] Confirm pre-selected institution (or pick different one) 
          + optionally check "strongest signal" boxes
     ↓
[CLICK 3] Click "Save & Classify"
     ↓
✓ Done, panel closes, next file highlighted
```

**Best case: 3 clicks** (card → confirm → save)  
**Typical case: 4-5 clicks** (if user browses institution list or checks signal boxes)

---

## DESIGN SYSTEM CONSTRAINTS

### Color Usage
Since you don't have formal docs, **extract from codebase:**
- **Backgrounds:** Look at Dashboard.tsx for surface colors
- **Accents:** Check existing button/badge colors
- **Text:** Find primary/secondary/muted text colors
- **Status colors:** Look for success (green), warning (yellow), danger (red) usage

**Instruction to Claude Design:**
> "Review `src/components/Dashboard.tsx` and `src/App.tsx` for:
> - Background colors (surface, surface-1, surface-2, etc.)
> - Text colors (primary, secondary, muted)
> - Accent colors (buttons, badges, highlights)
> - Status colors (success=green, warning=yellow, danger=red)
> Apply these same colors to Učebna. Do NOT use org template colors — match the app's existing palette."

### Typography
- **Headers:** Sans-serif, bold (16-20px)
- **Body text:** Sans-serif, regular (13-14px)
- **Small/muted:** Sans-serif, regular (11-12px)
- **Tooltips:** Sans-serif, regular (11px), light background, subtle shadow

### Spacing
- **Card padding:** 12px
- **Section gaps:** 1.5rem
- **Component gaps:** 8px

### Radius
- **Cards:** 12px
- **Buttons:** 8px
- **Badges:** 6px

### Interactions
- **Hover:** Background tint + cursor pointer
- **Disabled:** Opacity 50% + cursor not-allowed
- **Loading:** Spinner or pulse animation
- **Success:** ✓ checkmark, fade out in 1.5s

---

## DESIGN HANDOFF: SCREENSHOTS NEEDED

**To get Claude Design on track, Jarda will provide:**

1. **Dashboard.tsx screenshot** — Show current color scheme, typography, spacing
2. **Any existing modal/panel** — How FileIT currently does side panels
3. **Any existing badges/pills** — How status/tag display looks
4. **Any existing form elements** — Dropdowns, checkboxes, buttons

**With these, Claude Design can:**
- Match your existing aesthetic exactly
- Avoid org template imposition
- Deliver Učebna Phase 1-2 in app style (not foreign design)

---

## DELIVERABLES FROM CLAUDE DESIGN

1. **Učebna Phase 1 mockup:** List layout with file cards + confidence badges
2. **Učebna Phase 2 mockup:** Side panel with preview + institution picker + signal checkboxes
3. **Interaction spec:** Hover states, click flows, loading states
4. **CSS guidance:** Color vars + spacing to use (extracted from your codebase)

---

## NEXT STEP

**Jarda:**
1. Capture 3-4 screenshots from FileIT showing:
   - Color palette (existing component)
   - Typography (headers, body, small text)
   - Spacing/padding (cards, sections)
   - Any existing modals/panels
2. Send to Claude Design with this handoff doc
3. Request: "Match our app's existing design system exactly"

**Claude Design will then:**
- Deliver mockups in your app's visual style
- No org template interference
- Ready for Claude Code to implement

---

**Main constraint reminder for Claude Design:**
✅ **Intuitive UX**  
✅ **3 clicks max**  
✅ **Smart defaults** (pre-select likely institution)  
✅ **Rich tooltips** (every element has hover help)  
✅ **Match existing app colors/typography**  
❌ **NO org templates**  
❌ **NO jargon without explanation**  
❌ **NO hidden states**
