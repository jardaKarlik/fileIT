// src-tauri/src/classifier/ner.rs
//
// Czech Named Entity Recognition for FileIT.
// Uses regex patterns for structured identifiers (rodné číslo, IČO)
// and keyword dictionaries for institutions and document types.

use crate::types::*;
use anyhow::Result;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// Dictionary structures (mirrors institution_dictionary.json)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct DictionaryJson {
    institutions: Vec<InstitutionEntry>,
    document_type_triggers: HashMap<String, Vec<String>>,
    rodne_cislo_pattern: String,
    ico_pattern: String,
    dic_pattern: String,
}

#[derive(Debug, Deserialize)]
struct InstitutionEntry {
    canonical_name: String,
    category: String,
    keywords: Vec<String>,
    weight: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Compiled NER engine
// ─────────────────────────────────────────────────────────────────────────────

pub struct NerEngine {
    rodne_cislo_re: Regex,
    ico_re: Regex,
    dic_re: Regex,
    date_re: Regex,
    name_re: Regex,
    institutions: Vec<CompiledInstitution>,
    doc_type_triggers: Vec<(DocumentType, Vec<String>)>,
}

struct CompiledInstitution {
    canonical_name: String,
    category: InstitutionCategory,
    keywords: Vec<String>,
    weight: f32,
}

/// Signals derived from the filename alone, before any text extraction.
pub struct FilenameSignals {
    pub doc_type: Option<DocumentType>,
    /// Canonical institution name hint (matches institution_dictionary.json).
    pub institution_name: Option<String>,
    /// Customer name found in the filename via name regex.
    pub customer_name: Option<String>,
}

impl NerEngine {
    pub fn from_json(json: &str) -> Result<Self> {
        let dict: DictionaryJson = serde_json::from_str(json)
            .map_err(|e| anyhow::anyhow!("Failed to parse dictionary: {}", e))?;

        let rodne_cislo_re = Regex::new(&dict.rodne_cislo_pattern)?;
        let ico_re = Regex::new(&dict.ico_pattern)?;
        let dic_re = Regex::new(&dict.dic_pattern)?;

        // Czech date patterns: dd.mm.yyyy, dd. mm. yyyy, dd/mm/yyyy
        let date_re = Regex::new(
            r"(?:^|\s)(\d{1,2})[.\s/](\d{1,2})[.\s/](20\d{2})(?:\s|$)"
        )?;

        // Czech name pattern: Capitalized Surname, Title? Firstname? (very loose)
        // Looks for patterns like "Novák Pavel" or "Horáková Jana" near identifiers
        let name_re = Regex::new(
            r"\b([A-ZÁČĎÉĚÍŇÓŘŠŤÚŮÝŽ][a-záčďéěíňóřšťúůýž]{2,})\s+([A-ZÁČĎÉĚÍŇÓŘŠŤÚŮÝŽ][a-záčďéěíňóřšťúůýž]{2,})\b"
        )?;

        let institutions = dict.institutions.into_iter().map(|e| {
            let category = match e.category.as_str() {
                "bank" => InstitutionCategory::Bank,
                "insurance" => InstitutionCategory::Insurance,
                "government" => InstitutionCategory::Government,
                "utility" => InstitutionCategory::Utility,
                "telco" => InstitutionCategory::Telco,
                "legal" => InstitutionCategory::Legal,
                _ => InstitutionCategory::Other,
            };
            CompiledInstitution {
                canonical_name: e.canonical_name,
                category,
                keywords: e.keywords,
                weight: e.weight,
            }
        }).collect();

        let doc_type_triggers = dict.document_type_triggers.into_iter()
            .filter_map(|(k, v)| {
                let dt = match k.as_str() {
                    "Contract" => DocumentType::Contract,
                    "BankStatement" => DocumentType::BankStatement,
                    "BankLetter" => DocumentType::BankLetter,
                    "Invoice" => DocumentType::Invoice,
                    "PaymentDemand" => DocumentType::PaymentDemand,
                    "TaxReturn" => DocumentType::TaxReturn,
                    "UtilityBill" => DocumentType::UtilityBill,
                    "InsuranceContract" => DocumentType::InsuranceContract,
                    "AdminDecision" => DocumentType::AdminDecision,
                    "Identity" => DocumentType::Identity,
                    "LegalLetter" => DocumentType::LegalLetter,
                    _ => return None,
                };
                Some((dt, v))
            })
            .collect();

        Ok(Self {
            rodne_cislo_re,
            ico_re,
            dic_re,
            date_re,
            name_re,
            institutions,
            doc_type_triggers,
        })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Customer extraction
    // ─────────────────────────────────────────────────────────────────────────

    /// Find all customer references in the text.
    /// Uses rodné číslo as primary anchor; then searches for adjacent names.
    pub fn find_customers(&self, text: &str) -> Vec<CustomerMatch> {
        let mut customers: Vec<CustomerMatch> = Vec::new();

        // Primary path: find rodné číslo then look for nearby names
        for rc_match in self.rodne_cislo_re.find_iter(text) {
            let rc = rc_match.as_str().replace("/", "").replace(" ", "");
            if !self.is_valid_rodne_cislo(&rc) {
                continue;
            }

            // Look for a name in a 500-char window around the rodné číslo
            let start = text.floor_char_boundary(rc_match.start().saturating_sub(500));
            let end = text.floor_char_boundary((rc_match.end() + 500).min(text.len()));
            let window = &text[start..end];
            let name = self.name_re.find(window)
                .map(|m| m.as_str().to_string());

            customers.push(CustomerMatch {
                name: name.unwrap_or_else(|| format!("Klient (r.č. {})", &rc[..6])),
                rodne_cislo: Some(rc),
                ico: None,
                confidence: 0.92,
            });
        }

        // Secondary path: find IČO (business customers)
        for ico_match in self.ico_re.captures_iter(text) {
            let ico = ico_match.get(2).map(|m| m.as_str().to_string());
            if let Some(ref ico_val) = ico {
                // Don't duplicate if already found via rodné číslo
                if customers.iter().any(|c| c.ico.as_deref() == Some(ico_val)) {
                    continue;
                }
                let window_start = ico_match.get(0)
                    .map(|m| text.floor_char_boundary(m.start().saturating_sub(500)))
                    .unwrap_or(0);
                let window_end = ico_match.get(0)
                    .map(|m| text.floor_char_boundary((m.end() + 500).min(text.len())))
                    .unwrap_or(text.len());
                let window = &text[window_start..window_end];
                let name = self.name_re.find(window)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| format!("Firma (IČO {})", ico_val));

                customers.push(CustomerMatch {
                    name,
                    rodne_cislo: None,
                    ico,
                    confidence: 0.85,
                });
            }
        }

        // Deduplicate by rodné číslo / IČO
        customers.dedup_by(|a, b| {
            a.rodne_cislo == b.rodne_cislo && a.rodne_cislo.is_some()
        });

        customers
    }

    /// Validate Czech rodné číslo format and checksum (post-1954 numbers).
    fn is_valid_rodne_cislo(&self, rc: &str) -> bool {
        let digits: String = rc.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() != 9 && digits.len() != 10 {
            return false;
        }

        let yy: u32 = digits[..2].parse().unwrap_or(0);
        let mm: u32 = digits[2..4].parse().unwrap_or(0);
        let dd: u32 = digits[4..6].parse().unwrap_or(0);

        // Month: 1-12 (male) or 51-62 (female)
        let mm_norm = if mm > 50 { mm - 50 } else { mm };
        if mm_norm < 1 || mm_norm > 12 { return false; }
        if dd < 1 || dd > 31 { return false; }

        // 10-digit numbers (post-1953) must be divisible by 11
        if digits.len() == 10 {
            let n: u64 = digits.parse().unwrap_or(1);
            if n % 11 != 0 { return false; }
        }

        true
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Institution matching
    // ─────────────────────────────────────────────────────────────────────────

    pub fn find_institution(&self, text_lower: &str) -> Option<InstitutionMatch> {
        let mut best: Option<(&CompiledInstitution, f32)> = None;

        for inst in &self.institutions {
            let mut hit_count = 0usize;
            for kw in &inst.keywords {
                if text_lower.contains(kw.as_str()) {
                    hit_count += 1;
                }
            }
            if hit_count == 0 {
                continue;
            }
            // For institutions with many keywords, require ≥2 hits to prevent
            // a single generic term matching an unrelated document.
            if hit_count == 1 && inst.keywords.len() > 2 {
                continue;
            }

            // Confidence: proportion of keywords hit × institution weight
            let raw_conf = (hit_count as f32 / inst.keywords.len() as f32) * inst.weight;
            let conf = raw_conf.min(0.98);

            if best.is_none() || conf > best.as_ref().unwrap().1 {
                best = Some((inst, conf));
            }
        }

        best.map(|(inst, conf)| InstitutionMatch {
            canonical_name: inst.canonical_name.clone(),
            category: inst.category.clone(),
            confidence: conf,
        })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Document type matching
    // ─────────────────────────────────────────────────────────────────────────

    pub fn find_document_type(&self, text_lower: &str) -> DocumentType {
        let mut best_type = DocumentType::Unknown;
        let mut best_score = 0usize;

        for (dt, triggers) in &self.doc_type_triggers {
            let hits = triggers.iter()
                .filter(|t| text_lower.contains(t.as_str()))
                .count();
            if hits > best_score {
                best_score = hits;
                best_type = dt.clone();
            }
        }

        best_type
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Date extraction
    // ─────────────────────────────────────────────────────────────────────────

    pub fn find_date(&self, text: &str) -> Option<DateTime<Utc>> {
        for cap in self.date_re.captures_iter(text) {
            let (Ok(day), Ok(month), Ok(year)): (Result<u32, _>, Result<u32, _>, Result<i32, _>) = (
                cap[1].parse(),
                cap[2].parse(),
                cap[3].parse(),
            ) else {
                continue;
            };
            if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                if let Some(dt) = date.and_hms_opt(0, 0, 0) {
                    return Some(Utc.from_utc_datetime(&dt));
                }
            }
        }
        None
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Filename signal extraction
    // ─────────────────────────────────────────────────────────────────────────

    /// Extract doc type, institution, and customer name signals from the filename.
    /// Separators (_, -) are normalised to spaces before matching.
    pub fn signals_from_filename(&self, name: &str) -> FilenameSignals {
        let stem = std::path::Path::new(name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(name);
        let normalized = stem.replace(['_', '-'], " ");
        let lower = normalized.to_lowercase();

        FilenameSignals {
            doc_type: doc_type_from_filename(&lower),
            institution_name: institution_hint_from_filename(&lower),
            // Name regex needs original case (requires capital letters)
            customer_name: self.name_re
                .find(&normalized)
                .map(|m| m.as_str().to_string()),
        }
    }

    /// Look up an institution by canonical name and return a filename-confidence
    /// InstitutionMatch (0.65). Used when filename hints at an institution but
    /// content keyword matching found nothing.
    pub fn find_institution_by_name(&self, canonical_name: &str) -> Option<InstitutionMatch> {
        self.institutions
            .iter()
            .find(|i| i.canonical_name == canonical_name)
            .map(|i| InstitutionMatch {
                canonical_name: i.canonical_name.clone(),
                category: i.category.clone(),
                confidence: 0.65,
            })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Filename mining helpers — module-private
// ─────────────────────────────────────────────────────────────────────────────

/// Map normalised (lowercase, separators→spaces) filename stem to a document type.
/// More specific patterns are checked first.
fn doc_type_from_filename(lower: &str) -> Option<DocumentType> {
    if lower.contains("zelena karta") || lower.contains("zelená karta") {
        Some(DocumentType::InsuranceContract)
    } else if lower.contains("pojistka") || lower.contains("pojisteni") || lower.contains("pojistění") {
        Some(DocumentType::InsuranceContract)
    } else if lower.contains("smlouva") {
        Some(DocumentType::Contract)
    } else if lower.contains("vydana faktura") || lower.contains("vydaná faktura") || lower.contains("faktura") {
        Some(DocumentType::Invoice)
    } else if lower.contains("potvrzeni o platbe") || lower.contains("potvrzení o platbě")
           || lower.contains("potvrzeni o prijmu") || lower.contains("potvrzení o příjmu") {
        Some(DocumentType::PaymentDemand)
    } else if lower.contains("platba") || lower.contains("platby") {
        Some(DocumentType::PaymentDemand)
    } else if lower.contains("vypis") || lower.contains("výpis") {
        Some(DocumentType::BankStatement)
    } else if lower.contains("přehled přijatých plateb") || lower.contains("predpisy zaloh") || lower.contains("předpisy záloh") {
        Some(DocumentType::TaxReturn)
    } else if lower.contains("dpfo") || lower.contains("dpfdp") || lower.contains("dphdp") || lower.contains("dphkh") {
        Some(DocumentType::TaxReturn)
    } else if lower.contains("dorucenka") || lower.contains("doručenka")
           || lower.contains("rozhodnuti") || lower.contains("rozhodnutí") {
        Some(DocumentType::AdminDecision)
    } else if lower.contains("priloha") || lower.contains("příloha")
           || lower.contains("zadost") || lower.contains("žádost") {
        Some(DocumentType::AdminDecision)
    } else {
        None
    }
}

/// Map unambiguous filename abbreviations to institution canonical names.
/// Deliberately conservative — only patterns unique to Czech financial documents.
fn institution_hint_from_filename(lower: &str) -> Option<String> {
    if lower.contains("rbcz") {
        Some("Raiffeisenbank".to_string())
    } else if lower.contains("csob") {
        Some("ČSOB".to_string())
    } else if lower.contains("cssz") {
        Some("ČSSZ".to_string())
    } else if lower.contains("dpfdp") || lower.contains("dphdp")
           || lower.contains("dphkh") || lower.contains("dpfo") {
        Some("Finanční úřad".to_string())
    } else {
        None
    }
}
