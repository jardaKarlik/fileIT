# CNB Mini-Database Integration Analysis
## FileIT v1.0.12 — Local Institution Registry Architecture

**Date:** 2026-08-11  
**Status:** Implemented (v1.0.12)  
**Implementation Status:** ✅ Data built + integrated (see Part 6 status markers)  
**Scope:** Build authoritative local CNB institutions database; integrate into 4-stage classification pipeline

---

## Part 1: Data Source Analysis

### XML Dataset Structure
- **Source:** `C:\_temp\xml\` — 130+ XML export files from Czech National Bank (CNB) JERRS registry
- **Coverage:** ~50,000+ Czech & international financial and supervised entities
  (banks, insurance, pension, investment, payment systems, crypto, alternative
  finance, government agencies, PLUS bound representatives / brokers — the large
  bound-rep categories inflate the raw count; see Part 1 data notes)
- **Categories:** Banks, insurance, pension, investment, payment systems, crypto, alternative finance, government agencies

### Data Notes (generated 2026-08-11, schema v2)
- **`total_records` in `stats` = 127,065** is a **cross-file row count**, *not* unique institutions. `build_cnb_registry.py` counts every `<D-Rec>` element it sees across the 133 files, so an entity registered in several categories is counted once per file.
- **`institution_count` = 50,283** is the true registry size: rows are de-duplicated by the 8-digit **ICO** (`E-Icn`). Verified: **0 duplicate ICOs, 0 collision ICOs** (no ICO maps to multiple names). This is real, fully-loaded production data — **not test data**.
- Why so many: ~100k of the 127k raw rows come from **bound representatives / small brokers** — `bound_reps_insurance` (36k), `bound_reps_consumer_credit` (22k), `bound_reps_pension` (20k), `bound_reps_capital_market` (20k). These are distributors, not standalone banks/insurers — which is why earlier curated estimates (~2,800 "verified institutions") were far lower.
- `build_cnb_registry.py` strips wrapping quotes and collapses whitespace in names (CNB XML embeds `G-Ident` in literal quotes). Embedded quotes that are part of a legal name (e.g. `"A-Z"- FLOR s r.o.`) are preserved.

### File Structure (Sample: `01_banks_foreign_branches.xml`)
```xml
<Cat>
  <D-Rec>
    <E-Icn>29045371</E-Icn>           <!-- IČO (Czech ID) -->
    <F-IcnT>ICO</F-IcnT>               <!-- ID type -->
    <G-Ident>Air Bank a.s.</G-Ident>   <!-- Official name -->
    <H-Addr>Evropská 2690/17</H-Addr>  <!-- Address -->
    <I-City>Praha</I-City>
    <J-PostC>16000</J-PostC>
    <K-CountC>CZ</K-CountC>            <!-- Country code -->
    <L-DateFrom>03.06.2011</L-DateFrom> <!-- Registration date -->
  </D-Rec>
  ...
</Cat>
```

### Known Institutions (Sample)
- **Banks:** Air Bank, Raiffeisenbank, ČSOB, Komerční banka, mBank, Moneta, J&T banka, UniCredit, PPF banka
- **Insurance:** Kooperativa, Allianz, UNIQA, Generali, Česká pojišťovna
- **Pension:** Emporium, OVB, ČSOB
- **Payment Systems:** Payment institutions, electronic money providers, crypto service providers
- **Energy:** ČEZ (state utility)
- **Government:** Tax offices (Finanční úřad), development banks

---

## Part 2: Current Application State

### Classification Pipeline (Current)

**Classifier Module** (`src-tauri/src/classifier/mod.rs`):
```rust
pub async fn classify_with_hints(
    &self,
    record: FileRecord,
    hints: Option<ClassificationHints>,  // ← Stage 1 metadata
) -> ClassifiedFile {
    // Hints contain:
    // - institution_name (from PDF metadata)
    // - confidence_boost (0.55 for strong metadata)
    // - skip_extraction (skip OCR/text parsing)
    // - pdf_producer, pdf_creator
}
```

**Current Merge Order** (lines 89-98):
```rust
let institution = content_institution      // ← Stage 3: Content match
    .or(hint_institution)                  // ← Stage 1: Metadata hint
    .or_else(|| {                          // ← Filename dictionary fallback
        filename_signals.institution_name.as_deref()
            .and_then(|n| self.ner.find_institution_by_name(n))
    })
    .or(filename_signals.institution_direct);
```

### Scanner Module (`src-tauri/src/scanner/mod.rs`)

**File Tracking:**
- ✅ SHA-256 hash computed (first 64KB)
- ✅ `modified` timestamp captured (`SystemTime`)
- ⚠️ **Issue #1:** No logic to detect file modification on re-scan
- ⚠️ **Issue #2:** No persistent scan cache (FileRecord created fresh each scan)

**Duplicate Detection:**
```rust
let is_dup = if let Some(ref hash) = sha256 {
    if let Some(_existing) = hash_map.get(hash) {
        true  // ← Marks duplicates in current scan
    } else {
        hash_map.insert(hash.clone(), path.clone());
        false
    }
};
```
✅ Works **within a single scan run** — but each scan is independent (SessionState is new every time).

### Session State (`src-tauri/src/types.rs`)

```rust
pub struct SessionState {
    pub scanned_files: Vec<FileRecord>,      // ← Fresh for each scan
    pub classified_files: Vec<ClassifiedFile>,
    pub pattern_data: HashMap<String, GroupedPatternData>,
    // ...
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),  // ← New ID every time
            // ...
        }
    }
}
```

---

## Part 3: Missing Logic (Detected Issues)

### Issue #1: Stage 1→2 Skip Logic **NOT IMPLEMENTED**
**Scenario:** File path contains `...Pojistovny/Koop/insurance_doc.pdf`

**Expected Behavior (v1.0.12):**
1. Scanner detects path contains "Pojistovny" (insurance)
2. Stage 1: Extract from filename/path → `Institution: "Pojistovny"` (matches CNB db)
3. Confidence boost: 0.55
4. **Skip Stage 2 (content extraction)** ← This logic is missing

**Current Behavior:**
```rust
let (skip_extraction, confidence_boost, _) = match &hints {
    Some(h) => (h.skip_extraction, h.confidence_boost, ...),
    None => (false, 0.0f32, ...),  // ← Always false unless hints passed
};
```
- Hints are passed **only** if metadata extractor populated them
- No logic to set hints based on filepath matching CNB registry
- **→ Content extraction ALWAYS runs** (slow, PII risk)

### Issue #2: File Modification Detection **NOT IMPLEMENTED**
**Scenario:** User scanned same folder twice; file `contract.pdf` was updated between scans

**Expected Behavior:**
- Scan 1: `contract.pdf`, modified=2026-01-15, hash=abc123
- **User updates file: Scan 2 should detect change**
- Scan 2: `contract.pdf`, modified=2026-08-11 ← Different!
- Result: **Re-classify the file** (not skip it)

**Current Behavior:**
- No persistent scan history stored
- Each `SessionState` is brand new (new UUID)
- No way to know if file was updated since last scan
- **→ User must manually mark files for re-scan**

### Issue #3: Stage 1 Institution Matching **MISSING**
- No mechanism to match CNB institutions to filename/path patterns
- No confidence scoring for path-based matches
- No way to extract institution keywords from file path

---

## Part 4: Integration Architecture (Recommended)

### 4.1 Mini-Database Design

**Location:** `src-tauri/src/data/cnb_registry.json`

```json
{
  "institutions": [
    {
      "ic": "29045371",
      "name_cz": "Air Bank a.s.",
      "sector": "bank",
      "keywords": ["air", "airbank", "bank", "banka"],
      "registration_date": "2011-06-03",
      "country": "CZ"
    },
    {
      "ic": null,
      "name_cz": "Finanční úřad",
      "sector": "government",
      "keywords": ["finanční", "úřad", "tax", "daň"],
      "registration_date": null,
      "country": "CZ"
    },
    ...
  ],
  "metadata": {
    "total_count": 2847,
    "generated_at": "2026-08-11",
    "categories": {
      "bank": 450,
      "insurance": 300,
      "pension": 120,
      "investment": 280,
      "payment": 180,
      "other": 1517
    }
  }
}
```

### 4.2 Rust Registry Structure

**File:** `src-tauri/src/classifier/cnb_registry.rs`

```rust
use lazy_static::lazy_static;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct InstitutionEntry {
    pub ic: Option<String>,
    pub name_cz: String,
    pub sector: String,  // "bank", "insurance", "pension", etc.
    pub keywords: Vec<String>,
}

lazy_static! {
    static ref CNB_REGISTRY: CnbRegistry = CnbRegistry::load();
}

pub struct CnbRegistry {
    pub institutions: Vec<InstitutionEntry>,
    // Quick lookups
    pub by_ic: HashMap<String, InstitutionEntry>,
    pub by_name: HashMap<String, InstitutionEntry>,
    pub by_keyword: HashMap<String, Vec<InstitutionEntry>>,
}

impl CnbRegistry {
    pub fn load() -> Self { /* Load from JSON */ }
    
    pub fn find_by_keyword(&self, keyword: &str) -> Option<&InstitutionEntry> {
        // Case-insensitive, fuzzy match
    }
    
    pub fn find_by_path(&self, path: &str) -> Option<(f32, &InstitutionEntry)> {
        // Extract path components, match against keywords
        // Return (confidence, entry)
    }
}
```

### 4.3 Integration Points

#### **Stage 0.5: Path-Based Institution Detection (NEW)**
**Location:** `src-tauri/src/classifier/path_detector.rs`

```rust
pub fn detect_institution_from_path(path: &Path) -> Option<(String, f32)> {
    // Split path: C:\Dokumenty\Pojistovny\Kooperativa\dokument.pdf
    // → tokens = ["pojistovny", "kooperativa"]
    // → lookup against CNB_REGISTRY.by_keyword
    // → return ("Kooperativa", confidence=0.60)
    
    let lazy_registry = &*CNB_REGISTRY;
    // ... matching logic
}
```

Called **before Stage 1** metadata extraction:
```rust
// In: commands/mod.rs → scan_command
let path_hint = detect_institution_from_path(&file.path);
let mut hints = ClassificationHints {
    institution_name: path_hint.map(|(n, _)| n),
    confidence_boost: path_hint.map(|(_, conf)| conf).unwrap_or(0.0),
    skip_extraction: path_hint.is_some() && confidence > 0.55,
    ...
};
```

#### **Stage 1: Metadata + CNB Lookup (Enhanced)**
- PDF metadata (company, creator, producer)
- Match against `CNB_REGISTRY.by_name` → confidence boost
- If match found **and confidence > 0.55**: `skip_extraction = true`

#### **Stage 2: Content Extraction (Conditional)**
- Only if `skip_extraction = false`
- Extracts: OCR text, email headers, DOCX properties
- Still runs for borderline cases (0.40–0.54 confidence)

#### **Stage 3: NER Content Analysis (Unchanged)**
- Finds customers (rodné číslo, IČO, name)
- Finds document type (invoice, contract, letter, etc.)
- Merges with Stage 1 institution if no content match

#### **Stage 4: Scoring & Confidence (Calibrated)**
- If Stage 1 institution match found:
  - Add 0.55 boost (0.55 for path, 0.55 for metadata, max 0.95)
- If filename corroborates: +0.10
- Base score: 0.10–0.40 (customer + doc type only)
- Final: min(sum, 0.98)

---

## Part 5: File Modification Detection (NEW)

### Design

**Problem:** How to know if a file was updated since last scan?

**Solution:** Persistent **scan manifest** stored in AppData

**Location:** `C:\Users\{user}\AppData\Local\cz.fileit.app\scan_history\`

```json
// scan_manifests/2026_08_11_scan_001.json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "scan_at": "2026-08-11T14:30:00Z",
  "roots": ["C:\\Dokumenty", "C:\\OneDrive"],
  "files": [
    {
      "path": "C:\\Dokumenty\\2024\\invoice.pdf",
      "sha256": "abc123def456...",
      "modified": "2026-08-10T15:22:00Z",
      "size_bytes": 145920,
      "classified": true,
      "confidence": 0.89,
      "institution": "Air Bank"
    },
    ...
  ]
}
```

### Logic: Re-Scan Detection

**In: `src-tauri/src/scanner/mod.rs::scan_directories()`**

```rust
pub async fn scan_directories(
    app: &AppHandle,
    roots: Vec<PathBuf>,
    allowed_categories: &[FileCategory],
    extra_skip_dirs: &[PathBuf],
    // NEW PARAM:
    previous_manifest: Option<&ScanManifest>,
) -> Result<Vec<FileRecord>> {
    // For each file found in current scan:
    if let Some(prev) = previous_manifest.as_ref() {
        if let Some(prev_entry) = prev.find_by_path(&current_file_path) {
            if prev_entry.sha256 != current_sha256 
                || prev_entry.modified != current_modified {
                // FILE WAS UPDATED → needs re-classification
                current_record.needs_reclassify = true;
            } else {
                // Unchanged → can reuse previous ClassifiedFile
                current_record.can_reuse_classification = true;
            }
        }
    }
}
```

### Storage in SessionState

```rust
pub struct SessionState {
    pub scanned_files: Vec<FileRecord>,
    pub classified_files: Vec<ClassifiedFile>,
    // NEW:
    pub previous_manifest: Option<ScanManifest>,
    pub files_needing_reclassify: Vec<PathBuf>,
    pub reused_classifications: usize,  // UI metric
}
```

### UI Optimization (Optional)

```typescript
// Progress bar: "Reading: 1,247 files (850 unchanged, re-scanning 397 modified)"
// Only re-classify changed files → 70–80% speed improvement
```

---

## Part 6: Implementation Roadmap

### Step 1: Build CNB Mini-Database (30 min) — ✅ DONE
- Parse all 130 XML files → single JSON (`scripts/build_cnb_registry.py`)
- Generate registry loader (`cnb_registry.rs`)
- Load into memory (embedded via `include_str!`)
- Add keyword extraction (token index)

### Step 2: Path-Based Detection (1 hour) — ✅ DONE (in `cnb_registry.rs`)
- Module `cnb_registry.rs::CnbRegistry::detect_from_path`
- Tokenize file path (ICO-in-file → exact component → token keyword)
- Fuzzy-match against CNB keyword tokens
- Return (institution, confidence) 0.60 – 0.95

### Step 3: Stage 0.5 Integration (30 min) — ✅ DONE
- Hook path detector into `commands/mod.rs::start_scan` (Signal 2 path block)
- Populate hints from path detection (only when NER found nothing on path)
- `skip_extraction` intentionally NOT auto-enabled: the ~50k registry includes
  bound reps / brokers, so boosts are conservative (0.15 / 0.30) and content
  extraction is kept as the authoritative second check

### Step 4: File Modification Detection (1.5 hours) — ✅ DONE
- New type: `ScanManifest` (`scan_manifest.rs`)
- Save manifest after each scan (`latest.json`)
- Load previous manifest at start of scan
- Detect changes by SHA-256 + modified timestamp
- Reuse classification for unchanged files; re-classify modified/new files

### Step 5: Testing & Calibration (1 hour)
- Test path matching with real file structure
- Verify confidence scores
- Measure OCR skip rate (target: 60–70%)
- Benchmark: re-scan speed improvement

**Total: ~4.5 hours for full integration**

---

## Part 7: Compliance & Data Governance

### Privacy Safeguards ✅
- CNB registry contains **zero PII** (only institution names, IČO, addresses)
- Path-based matching **does not transmit** file paths to cloud
- Scan manifest stored **locally only**
- Modified timestamp detection is **local operation** (no network)

### Audit Trail ✅
- Each file gets `skip_extraction` flag → logged
- Classification source (Path | Metadata | Content) → logged
- Re-classification reason (File modified) → logged

### Compliance Notes
- ✅ No new cloud transmission
- ✅ No new PII collection
- ✅ Dry-run default unchanged
- ✅ User control: can still manually override any match

---

## Part 8: Questions for Jarda

1. **Confidence Score Calibration:** For path matches, should we use:
   - Static 0.60 (path detected institution name in folder)
   - Or dynamic based on "distance" from root (deeper = higher confidence)

2. **Re-Scan Behavior:** When user re-scans and file is unchanged:
   - Reuse previous classification silently?
   - Or re-classify anyway (slower but more thorough)?

3. **Sector Weighting:** Should Stage 1 use sector-specific keyword weights?
   - E.g., `/Pojistovny/` → insurance sector boost (try insurance sector plugins first)

4. **Modified Detection Scope:** Should we persist manifests for:
   - Last scan only?
   - Last 5 scans (for audit trail)?
   - Indefinitely?

---

## Summary

✅ **Data ready:** ~50,283 CNB institutions built (`cnb_registry.json`, schema v2)  
✅ **Architecture designed:** pipeline with path + metadata + content matching  
✅ **Implemented (v1.0.12):**
  - CNB token/path institution matching wired into `start_scan` (Signal 2)
  - Scan manifest persistence for incremental re-scans (load + save)
  - Manifest save wired, so `latest.json` now actually persists
  - Unit tests for CNB detection (3), all passing
⚠️ **Remaining consideration:** Stage 1→2 skip logic is intentionally conservative
  (no auto `skip_extraction` from the noisy ~50k registry); OCR still runs as the
  authoritative check unless a strong CM/fingerprint match fires. Optionally
  curate the registry to core financial categories to shrink binary/startup cost.

🎯 **Next:** Full `cargo test` green (9/9), verify scan end-to-end, commit v1.0.12.

