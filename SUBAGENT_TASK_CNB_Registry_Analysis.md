# SUBAGENT TASK: CNB Registry Data Source Analysis
**Objective:** Investigate WS JERRS API structure and recommend canonical lookup table schema for FileIT classification pipeline.

---

## CONTEXT

**Goal:** Build a one-time canonical lookup table of Czech financial institutions (banks, insurers, pension funds, etc.) to seed FileIT's fingerprint database. This table will be baked into the app core and used during classification scans (not as a real-time API call).

**Data Source:** Czech National Bank (CNB) WS JERRS web service  
**API Spec:** https://www.cnb.cz/en/supervision-financial-market/lists-registers/web-services-ws-jerrs/

---

## KEY FINDINGS (FROM WEB FETCH)

### WS JERRS Architecture
- **Type:** SOAP/XML-based web service (not REST/JSON)
- **Authentication:** Commercial SSL certificate required (eIdentity, I.CA, Postsignum)
- **Endpoint:** `https://aplc.cnb.cz/jerrsws/ws`
- **Format:** Synchronous request/response via XML
- **Specification:** WSDL + XSD provided

### Three Available Services
1. **getSubjekt** — Query a specific entity by ID
2. **getSubjekty** — Query all entities of a particular role/type (paginated)
3. **getZastoupeny** — Query agents of a represented entity (restricted)

### Available Data (From Response Examples)
From the provided XML response examples:
- Entity identification (name, registration number, IČ)
- Registered address
- Authorized activities (roles)
- Commencement/termination dates
- Relations to other entities

### Code Lists Provided
- `WSJERRS_role.xlsx` — Entity roles (bank, insurer, pension fund, etc.)
- `WSJERRS_seznamy.xlsx` — Lists available
- `WSJERRS_cinnosti_role.xlsx` — Authorized activities per role

---

## SUBAGENT ANALYSIS NEEDED

### Q1: Data Extraction Strategy
**Question:** How should we extract institution names, official keywords, and visual identifiers?

**Known constraints:**
- WS JERRS returns structured XML (no visual data like logos/colors)
- Public JERRS web interface might have visual data (colors from website PDFs?)
- Need to map roles to FileIT institution types (Bank → Instituce="Air Bank", Insurer → Instituce="Kooperativa")

**Your task:** Recommend:
- Which WS JERRS role codes map to FileIT's classification needs?
- What metadata is extractable from WS JERRS XML?
- Should we supplement WS JERRS with a secondary source (web scraping institution websites for colors)?

---

### Q2: Canonical Lookup Table Schema
**Question:** What should the lookup table structure be?

**Current v1.0.11 model (from INSTITUTION_FINGERPRINTS_CALIBRATED_v1.0.11.md):**
```
Kooperativa (Insurance):
  - Czech Name: "Kooperativa pojišťovna"
  - Keywords: ["Kooperativa", "KOP", "pojisteni"]
  - Color Palette: Red (#C41E3A)
  - Logo: Red + white branding
  - Sector: insurance
  - Confidence Weight: +25% (verified)
```

**Your task:** Design the schema that includes:
- Institution ID (IČ from WS JERRS)
- Institution name (Czech + English)
- Sector/Role code (bank, insurer, pension, etc.)
- Official keywords (Czech + English)
- Color palette (if available)
- Verified flag (came from CNB official source)
- Data source & date

**Example output format** (JSON pseudocode):
```json
{
  "institutions": [
    {
      "ic": "47114983",
      "name_cz": "Česká spořitelna, a.s.",
      "name_en": "Ceska Spoitelna",
      "sector": "bank",
      "keywords_cz": ["spořitelna", "Česká spořitelna", "CS"],
      "keywords_en": ["savings bank", "ceska spoitelna"],
      "colors": {
        "primary": "#E5213B",  // red
        "secondary": "#FFFFFF"  // white
      },
      "verified": true,
      "source": "CNB_JERRS",
      "updated": "2026-08-11"
    }
  ]
}
```

---

### Q3: Implementation Feasibility
**Question:** Can we extract this data in a one-time script, or does it require ongoing maintenance?

**Your task:** Evaluate:
- WS JERRS query complexity (does it require certificate + authorization?)
- Data completeness (are all Czech banks/insurers listed with IČ and names?)
- Visual data availability (do institution websites have official color info we can extract separately?)
- Fallback strategy (if color data isn't available, can we use keyword matching as primary signal?)

---

### Q4: Institutional Coverage
**Question:** Which institutions should we prioritize?

**Your task:** Categorize institutions by priority:
1. **Tier 1 (Must-have):** Major banks (ČNB, Česká spořitelna, Raiffeisenbank, etc.)
2. **Tier 2 (Important):** Insurance companies (Kooperativa, Allianz, Generali, UNIQA, etc.)
3. **Tier 3 (Nice-to-have):** Pension funds, investment firms, smaller regional banks

---

### Q5: Build vs. Buy Decision
**Question:** Should we build a WS JERRS scraper, or use an existing data source?

**Your task:** Research alternatives:
- **Option A:** Build a Rust SOAP client to query WS JERRS (requires cert, ~2 days)
- **Option B:** Scrape public JERRS web UI with Claude in Chrome (1 day, less reliable)
- **Option C:** Use a public open-data mirror (e.g., ARES API from CzechPoint — has IČ lookups)
- **Recommendation:** Which is fastest + most reliable for one-time bootstrap?

---

## DELIVERABLES EXPECTED

When done, subagent should provide:

1. **Canonical Lookup Table Schema** (JSON-like format showing structure)
2. **Sample Data** (3-5 real institutions from CNB with all fields populated)
3. **Extraction Strategy** (pseudocode for how to query WS JERRS or alternative source)
4. **Feasibility Assessment** (time/complexity/reliability for implementation)
5. **Recommendation** (best path forward: build WS JERRS scraper, use ARES API, or hybrid)

---

## CONTEXT FOR SUBAGENT

**Why this matters for FileIT:**
- Current fingerprint database is manually curated from v1.0.11 calibration
- CNB-seeded data would provide authoritative, verified institution metadata
- Reduces false positives during scan (fewer "unrecognized" files)
- Enables better Učebna UX (confidence badges make more sense with verified source)
- Compliance benefit: audit trail shows "matched CNB registry record #47114983"

**Timeline:** This is a v1.0.12 *candidate* feature, not today's task. But investigating the API now means we can decide to include it in the build or defer to v1.0.13.

---

## SUBAGENT: YOUR TURN

Analyze the above and provide:
1. Recommended canonical schema (JSON structure)
2. Sample 3-5 institutions with full data
3. Extraction strategy (which API/source + pseudocode)
4. Time/complexity estimate
5. Clear recommendation on build path

**Use the WS JERRS documentation provided. Feel free to research ARES API (Czech business registry) as an alternative.**
