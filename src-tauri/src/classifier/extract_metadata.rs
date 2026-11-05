// src-tauri/src/classifier/extract_metadata.rs
//
//! Regex-based extraction of Czech document metadata.
//!
//! Pure functions only — no I/O, no side effects, no global mutable state.
//! All regexes are compiled once via `once_cell::sync::Lazy`.
//!
//! ExtractedMMDD/PPP(P)` form (checksum validated)
//!   • IČO         — 8-digit company registration number
//!   • DIČ         — Czech VAT number (CZxxxxxxxx / CZxxxxxxxxxx)
//!   • IBAN        — CZ IBAN (24 characters, spaces tolerated in the text)
//!   • VS          — variable symbol (VS:, Var. symbol:, Variabilní symbol:)
//!   • amounts     — CZK amounts, e.g. `1 234,56 Kč`, `1200 CZK`
//!   • dates       — `d.M.yyyy`, `d. M. yyyy` and `d. MMMM yyyy` (Czech month names)

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Czech month names
// ---------------------------------------------------------------------------

const MONTHS_CZ: [&str; 12] = [
    "ledna",
    "února",
    "března",
    "dubna",
    "května",
    "června",
    "července",
    "srpna",
    "září",
    "října",
    "listopadu",
    "prosince",
];

// ---------------------------------------------------------------------------
// Compiled regexes
//
// NOTE: the `regex` crate supports neither look-around nor backreferences,
// so all patterns below are written with plain syntax only.
// ---------------------------------------------------------------------------

/// RRMMDD/PPP(P) — rodné číslo. The 6-digit prefix encodes the birth date,
/// an optional `/` separates the 3-digit (pre-1954) or 4-digit suffix.
/// `\b` guards prevent matching into longer digit runs (account numbers).
static RC_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(\d{2})(\d{2})(\d{2})\s*/?\s*(\d{3,4})\b").unwrap());

/// IČO — exactly 8 standalone digits. `\b` at both ends rejects the middle of
/// a 9+ digit run, so an IČO nested in an account number or DIČ is skipped.
static ICO_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b(\d{8})\b").unwrap());

/// DIČ — the literal `DIČ`/`DIC`/`IČ DPH` prefix followed by `CZ` + 8 digits
/// (company) or 10 digits (self-employed person, birth-number based).
/// The `\d{8}` branch first fails its trailing `\b` when more digits follow,
/// then falls back to the `\d{10}` branch.
static DIC_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(?:DIČ|DIC|IČ\s*DPH)\s*[:.]?\s*(CZ\s?(\d{8}|\d{10}))\b").unwrap());

/// IBAN — `CZ` + 22 alphanumeric characters in groups separated by spaces.
/// `(?:[A-Z0-9]\s?){22}` walks alnum with optional single spaces; with the
/// trailing `\b` the engine backtracks into the exact alignment.
static IBAN_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(CZ\s?(?:[A-Z0-9]\s?){22})\b").unwrap());

/// Variable symbol — `VS:`, `Var. symbol:`, `Variabilní symbol:` followed by
/// 1–10 digits.
static VS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(?:\bVS\b|Var\.?\s*symbol|Variabilní\s+symbol)\s*[:.]?\s*(\d{1,10})\b",
    )
    .unwrap()
});

/// Amounts — `1 234,56 Kč`, `1200 CZK`, `24 200,00 KC`. Free-spacing mode
/// (`x`) lets the pattern be written readably; `[[:space:]]` stays explicit.
static AMOUNT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
        (?:\d{1,3}(?:[[:space:]]\d{3})*|\d+)
        (?:,\d{2})?
        [[:space:]]*
        (?:Kč|CZK|KC\b)
        ",
    )
    .unwrap()
});

/// Date `24.12.2023` / `24. 12. 2023`.
static DATE_DOT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(\d{1,2})\.\s?(\d{1,2})\.\s?(\d{4})\b").unwrap());

/// Date `1. února 2024` — lowercase Czech month names.
static DATE_WORD_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(\d{1,2})\.\s?([a-zá-ž]+)\s?(\d{4})\b").unwrap());

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Czech document metadata extracted from raw text.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExtractedMetadata {
    pub rodne_cislo: Option<String>,
    pub ico: Option<String>,
    pub dic: Option<String>,
    pub iban: Option<String>,
    pub variable_symbol: Option<String>,
    pub amounts: Vec<String>,
    pub czech_dates: Vec<String>,
}

/// Extract Czech metadata from raw document text.
///
/// Pure function — takes `&str`, returns `ExtractedMetadata`; no I/O, no
/// side effects.
pub fn extract_metadata(text: &str) -> ExtractedMetadata {
    ExtractedMetadata {
        rodne_cislo: extract_rodne_cislo(text),
        ico: extract_ico(text),
        dic: extract_dic(text),
        iban: extract_iban(text),
        variable_symbol: extract_variable_symbol(text),
        amounts: extract_amounts(text),
        czech_dates: extract_czech_dates(text),
    }
}

/// Extract the first checksum-valid rodné číslo from the text.
pub fn extract_rodne_cislo(text: &str) -> Option<String> {
    for cap in RC_RE.captures_iter(text) {
        let prefix = format!(
            "{}{}{}",
            cap.get(1).unwrap().as_str(),
            cap.get(2).unwrap().as_str(),
            cap.get(3).unwrap().as_str()
        );
        let suffix = cap.get(4).unwrap().as_str();
        if rodne_cislo_valid(&prefix, suffix) {
            return Some(format!("{}/{}", prefix, suffix));
        }
    }
    None
}

/// Extract the first standalone 8-digit IČO from the text.
pub fn extract_ico(text: &str) -> Option<String> {
    ICO_RE
        .captures(text)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
}

/// Extract the DIČ (Czech VAT id) if the explicit prefix is present.
pub fn extract_dic(text: &str) -> Option<String> {
    DIC_RE
        .captures(text)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().replace(' ', "")))
}

/// Extract the first CZ IBAN, normalized without spaces, uppercased.
pub fn extract_iban(text: &str) -> Option<String> {
    IBAN_RE
        .captures(text)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().replace(' ', "").to_uppercase()))
        .map(|s| s.trim().to_string())
}

/// Extract the first variable symbol.
pub fn extract_variable_symbol(text: &str) -> Option<String> {
    VS_RE
        .captures(text)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
}

/// Extract all CZK amounts, deduplicated, in order of appearance.
pub fn extract_amounts(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for m in AMOUNT_RE.find_iter(text) {
        let normalized = m.as_str().split_whitespace().collect::<Vec<_>>().join(" ");
        if !out.contains(&normalized) {
            out.push(normalized);
        }
    }
    out
}

/// Extract all Czech dates normalized to `dd.MM.yyyy`, deduplicated,
/// in order of appearance. `d. MMMM yyyy` resolves the month via the
/// Czech month-name table.
pub fn extract_czech_dates(text: &str) -> Vec<String> {
    // (start offset, normalized date); offsets let us emit dates in true
    // document order even though the two patterns are scanned sequentially.
    let mut found: Vec<(usize, String)> = Vec::new();

    for cap in DATE_DOT_RE.captures_iter(text) {
        let d = parse_num(cap.get(1));
        let m = parse_num(cap.get(2));
        let y = parse_num(cap.get(3));
        if (1..=31).contains(&d) && (1..=12).contains(&m) && (1900..=2100).contains(&y) {
            if let Some(c) = cap.get(0) {
                found.push((c.start(), format!("{:02}.{:02}.{}", d, m, y)));
            }
        }
    }

    for cap in DATE_WORD_RE.captures_iter(text) {
        let d = parse_num(cap.get(1));
        let month_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let y = parse_num(cap.get(3));
        if (1..=31).contains(&d) && (1900..=2100).contains(&y) {
            if let (Some(c), Some(m)) = (cap.get(0), month_from_cz_name(month_name)) {
                found.push((c.start(), format!("{:02}.{:02}.{}", d, m, y)));
            }
        }
    }

    found.sort_by_key(|(start, _)| *start);

    // Deduplicate, keeping the first occurrence in document order.
    let mut out: Vec<String> = Vec::new();
    for (_, s) in found {
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn parse_num(c: Option<regex::Match>) -> u32 {
    c.and_then(|m| m.as_str().parse().ok()).unwrap_or(0)
}

/// Validate a rodné číslo: birth month must resolve into 01–12 (with the
/// +50 marker for women), and the number must satisfy the mod-11 checksum.
/// 9-digit form (pre-1954): the whole number is divisible by 11.
/// 10-digit form: the 9 first digits mod 11 must equal the last digit.
fn rodne_cislo_valid(prefix: &str, suffix: &str) -> bool {
    let digits: String = format!("{}{}", prefix, suffix)
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();

    // Birth month sanity: digits 3–4 of the prefix.
    if digits.len() < 4 {
        return false;
    }
    let month: u32 = match digits[2..4].parse() {
        Ok(m) => m,
        Err(_) => return false,
    };
    let real_month = if month > 50 { month - 50 } else { month };
    if !(1..=12).contains(&real_month) {
        return false;
    }

    // Checksum: both forms must satisfy the mod-11 rule.
    //   9-digit (pre-1954): the whole 9-digit number is divisible by 11.
    //   10-digit: first 9 digits mod 11 must equal the check digit
    //   (a remainder of 10 maps to check digit 0).
    if digits.len() == 9 {
        let num: u32 = digits.parse().unwrap_or(u32::MAX);
        return num != u32::MAX && num % 11 == 0;
    }
    if digits.len() == 10 {
        let num9: u32 = digits[..9].parse().unwrap_or(u32::MAX);
        let last: u32 = digits[9..10].parse().unwrap_or(u32::MAX);
        let rem = num9 % 11;
        let check = if rem == 10 { 0 } else { rem };
        return num9 != u32::MAX && last != u32::MAX && last == check;
    }
    false
}

fn month_from_cz_name(name: &str) -> Option<u32> {
    MONTHS_CZ
        .iter()
        .position(|m| m.eq_ignore_ascii_case(name))
        .map(|i| (i + 1) as u32)
}

// ---------------------------------------------------------------------------
// Tests — realistic Czech fixtures
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // A freelance client's (OSVČ) account statement: their DIČ is birth-
    // number derived, so RC/DIČ digits intentionally overlap.
    const BANK_STATEMENT: &str = "\
Česká spořitelna, a.s.
číslo účtu: 123456789/0800
IBAN: CZ55 0800 0000 0012 3456 7890
Datum: 24. 12. 2023
Variabilní symbol: 123456789
Obrat: 15 480,50 Kč
Plátce: Jan Novák, rodné číslo 850415/1007, DIČ: CZ8504151007\
";

    const INVOICE: &str = "\
Faktura č. 2024001
Dodavatel: ACME s.r.o., IČO: 27123456, DIČ: CZ27123456
Odběratel: Beta a.s.
Datum vystavení: 1. února 2024
Datum splatnosti: 15. 2. 2024
Variabilní symbol: 2024001
Celkem k úhradě: 24 200,00 Kč\
";

    const EMPLOYMENT_CONTRACT: &str = "\
Pracovní smlouva
Zaměstnanec: Jana Malá, rodné číslo 990229/5678
Zaměstnavatel: Gamma spol. s r.o., IČO: 88776655
Datum nástupu: 1. 3. 2024
Měsíční mzda: 35 000 Kč
VS: 88001\
";

    // ---- combined ----------------------------------------------------------

    #[test]
    fn option_fields_defaults() {
        let md = extract_metadata("");
        assert!(md.rodne_cislo.is_none());
        assert!(md.ico.is_none());
        assert!(md.dic.is_none());
        assert!(md.iban.is_none());
        assert!(md.variable_symbol.is_none());
        assert!(md.amounts.is_empty());
        assert!(md.czech_dates.is_empty());
    }

    // ---- rodné číslo -------------------------------------------------------

    #[test]
    fn rc_bank_statement() {
        let md = extract_metadata(BANK_STATEMENT);
        assert_eq!(md.rodne_cislo.as_deref(), Some("850415/1007"));
    }

    #[test]
    fn rc_employment_contract() {
        let md = extract_metadata(EMPLOYMENT_CONTRACT);
        assert_eq!(md.rodne_cislo.as_deref(), Some("990229/5678"));
    }

    #[test]
    fn rc_rejects_invalid_month() {
        assert!(extract_rodne_cislo("rodné číslo 851315/1234").is_none());
    }

    #[test]
    fn rc_rejects_failed_checksum() {
        // Correct month, but 8504151234 % 11 != 0 for any valid 10-digit form.
        assert!(extract_rodne_cislo("rodné číslo 850415/1234").is_none());
    }

    #[test]
    fn rc_rejects_no_digits() {
        assert!(extract_rodne_cislo("empty statement").is_none());
    }

    #[test]
    fn rc_skips_account_number() {
        // 123456789/0800 is an account number; the only legal extraction
        // from it (123456/789) fails the checksum, so nothing is returned.
        assert!(extract_rodne_cislo("účet: 123456789/0800").is_none());
    }

    // ---- IČO ---------------------------------------------------------------

    #[test]
    fn ico_invoice() {
        let md = extract_metadata(INVOICE);
        assert_eq!(md.ico.as_deref(), Some("27123456"));
    }

    #[test]
    fn ico_employment() {
        let md = extract_metadata(EMPLOYMENT_CONTRACT);
        assert_eq!(md.ico.as_deref(), Some("88776655"));
    }

    #[test]
    fn ico_ignores_longer_digit_runs() {
        // A 9-digit run contains no standalone 8-digit IČO.
        assert_eq!(extract_ico("123456789"), None);
    }

    // ---- DIČ ---------------------------------------------------------------

    #[test]
    fn dic_invoice() {
        let md = extract_metadata(INVOICE);
        assert_eq!(md.dic.as_deref(), Some("CZ27123456"));
    }

    #[test]
    fn dic_bank_statement_ten_digit() {
        // OSVČ DIČ is the 10-digit birth-number form.
        let md = extract_metadata(BANK_STATEMENT);
        assert_eq!(md.dic.as_deref(), Some("CZ8504151007"));
    }

    #[test]
    fn dic_not_detected_without_prefix() {
        assert_eq!(extract_dic("CZ12345678"), None);
    }

    // ---- IBAN --------------------------------------------------------------

    #[test]
    fn iban_bank_statement() {
        let md = extract_metadata(BANK_STATEMENT);
        assert_eq!(md.iban.as_deref(), Some("CZ5508000000001234567890"));
    }

    #[test]
    fn iban_not_detected_for_others() {
        assert_eq!(extract_iban("DE89 3704 0044 0532 0130 00"), None);
    }

    // ---- Variable symbol ---------------------------------------------------

    #[test]
    fn vs_bank_statement() {
        let md = extract_metadata(BANK_STATEMENT);
        assert_eq!(md.variable_symbol.as_deref(), Some("123456789"));
    }

    #[test]
    fn vs_invoice() {
        let md = extract_metadata(INVOICE);
        assert_eq!(md.variable_symbol.as_deref(), Some("2024001"));
    }

    // ---- Amounts -----------------------------------------------------------

    #[test]
    fn amounts_bank_statement() {
        let md = extract_metadata(BANK_STATEMENT);
        assert_eq!(md.amounts, vec!["15 480,50 Kč"]);
    }

    #[test]
    fn amounts_invoice() {
        let md = extract_metadata(INVOICE);
        assert_eq!(md.amounts, vec!["24 200,00 Kč"]);
    }

    #[test]
    fn amounts_czk_suffix() {
        assert_eq!(extract_amounts("1200 CZK"), vec!["1200 CZK"]);
    }

    // ---- Dates -------------------------------------------------------------

    #[test]
    fn dates_bank_statement() {
        let md = extract_czech_dates(BANK_STATEMENT);
        assert_eq!(md, vec!["24.12.2023"]);
    }

    #[test]
    fn dates_invoice_word_month() {
        let md = extract_czech_dates(INVOICE);
        assert_eq!(md, vec!["01.02.2024", "15.02.2024"]);
    }

    #[test]
    fn dates_employment() {
        let md = extract_czech_dates(EMPLOYMENT_CONTRACT);
        assert_eq!(md, vec!["01.03.2024"]);
    }

    #[test]
    fn dates_invalid_month_skipped() {
        assert!(extract_czech_dates("24.13.2023").is_empty());
    }

    #[test]
    fn dates_deduplication() {
        let md = extract_czech_dates("24.12.2023 24. 12. 2023");
        assert_eq!(md.len(), 1);
    }
}
