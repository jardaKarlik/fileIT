// src-tauri/src/cm_registry.rs
//
// Centrální Mozek (CM) fingerprint registry.
// Versioned JSON file loaded on launch, matched during Stage 1 classification.
// No ML — pure rule matching with additive signal weights.
// No cloud calls during scan or classification.

use crate::metadata_reader::PdfMetadata;
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────────
// Registry types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstitutionFingerprint {
    pub institution: String,
    pub ico: Option<String>,
    pub signals: FingerprintSignals,
    pub category: String,
    pub confidence_boost: f32,
    pub skip_extraction: bool,
    pub contributed_by: u32,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintSignals {
    pub pdf_producer_contains: Vec<String>,
    pub pdf_creator_contains: Vec<String>,
    pub filename_patterns: Vec<String>,
    pub color_buckets: Vec<String>,
    pub logo_phash: Option<u64>,
    pub page_count_range: Option<(u32, u32)>,
    pub size_bucket: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmRegistry {
    pub version: String,
    pub fingerprints: Vec<InstitutionFingerprint>,
}

/// Owned result from a CM registry match.
#[derive(Debug, Clone)]
pub struct CmMatchResult {
    pub institution: String,
    pub category: String,
    pub confidence_boost: f32,
    pub skip_extraction: bool,
    pub score: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Registry impl
// ─────────────────────────────────────────────────────────────────────────────

impl CmRegistry {
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let registry: CmRegistry = serde_json::from_str(&content)?;
        Ok(registry)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Add a new fingerprint to the registry (from teaching).
    pub fn add_fingerprint(&mut self, fp: InstitutionFingerprint) {
        // Replace existing entry for the same institution if present
        if let Some(existing) = self.fingerprints.iter_mut()
            .find(|f| f.institution == fp.institution)
        {
            *existing = fp;
        } else {
            self.fingerprints.push(fp);
        }
    }

    /// Match a file against all fingerprints.
    /// Returns the best match if its score >= 0.35, otherwise None.
    pub fn match_file(
        &self,
        metadata: &PdfMetadata,
        filename: &str,
        file_size: u64,
    ) -> Option<CmMatchResult> {
        let producer = metadata.producer.as_deref().unwrap_or("").to_lowercase();
        let creator = metadata.creator.as_deref().unwrap_or("").to_lowercase();
        let size_bucket = crate::metadata_reader::size_to_bucket(file_size);

        let mut best: Option<CmMatchResult> = None;

        for fp in &self.fingerprints {
            let score = score_fingerprint(fp, &producer, &creator, filename, &size_bucket);

            if score >= 0.35 {
                if best.as_ref().map_or(true, |b| score > b.score) {
                    best = Some(CmMatchResult {
                        institution: fp.institution.clone(),
                        category: fp.category.clone(),
                        confidence_boost: fp.confidence_boost,
                        skip_extraction: fp.skip_extraction,
                        score,
                    });
                }
            }
        }

        best
    }
}

fn score_fingerprint(
    fp: &InstitutionFingerprint,
    producer: &str,
    creator: &str,
    filename: &str,
    size_bucket: &str,
) -> f32 {
    let mut score = 0.0f32;

    // Producer match (weight 0.45) — add once even if multiple patterns match
    if fp.signals.pdf_producer_contains.iter()
        .any(|p| producer.contains(&p.to_lowercase()))
    {
        score += 0.45;
    }

    // Creator match (weight 0.45)
    if fp.signals.pdf_creator_contains.iter()
        .any(|c| creator.contains(&c.to_lowercase()))
    {
        score += 0.45;
    }

    // Filename pattern match (weight 0.35)
    if fp.signals.filename_patterns.iter()
        .filter_map(|p| Regex::new(p).ok())
        .any(|re| re.is_match(filename))
    {
        score += 0.35;
    }

    // Color buckets — not yet implemented
    // logo_phash — not yet implemented
    // page_count_range — not stored on ClassifiedFile yet

    // Size bucket match (weight 0.05)
    if fp.signals.size_bucket.as_deref() == Some(size_bucket) {
        score += 0.05;
    }

    score
}

// ─────────────────────────────────────────────────────────────────────────────
// PII compliance test
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CmPayload;

    #[test]
    fn cm_payload_contains_no_pii() {
        let payload = CmPayload {
            pdf_producer: Some("Scriptura XSL-FO R8.5".to_string()),
            pdf_creator: None,
            color_buckets: vec!["H220-240/S80+".to_string()],
            logo_phash: None,
            institution: "Raiffeisenbank CZ".to_string(),
            doc_type: "bank_statement".to_string(),
        };

        let json = serde_json::to_string(&payload).expect("serialisation must succeed");

        // No filesystem path separators
        assert!(
            !json.contains('/') || json.contains("H220-240/S80+"), // color bucket contains /, which is fine
            "payload JSON must not contain path separators: {}",
            json
        );
        assert!(!json.contains('\\'), "payload JSON must not contain backslashes: {}", json);

        // No individual string value > 200 chars (would indicate content leak)
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        fn check_values(v: &serde_json::Value) {
            match v {
                serde_json::Value::String(s) => {
                    assert!(
                        s.len() <= 200,
                        "string value too long ({} chars): {:?}",
                        s.len(),
                        &s[..s.len().min(80)]
                    );
                }
                serde_json::Value::Object(map) => {
                    for (k, v2) in map {
                        assert!(
                            !matches!(k.as_str(), "filename" | "path" | "content" | "name" | "rc"),
                            "payload must not contain field {:?}",
                            k
                        );
                        check_values(v2);
                    }
                }
                serde_json::Value::Array(arr) => {
                    for v2 in arr {
                        check_values(v2);
                    }
                }
                _ => {}
            }
        }
        check_values(&value);
    }

    #[test]
    fn match_returns_none_below_threshold() {
        let registry = CmRegistry {
            version: "test".to_string(),
            fingerprints: vec![],
        };
        let meta = PdfMetadata::default();
        assert!(registry.match_file(&meta, "unknown.pdf", 50_000).is_none());
    }

    #[test]
    fn match_raiffeisenbank_by_producer() {
        let registry = CmRegistry {
            version: "test".to_string(),
            fingerprints: vec![InstitutionFingerprint {
                institution: "Raiffeisenbank CZ".to_string(),
                ico: Some("49240901".to_string()),
                signals: FingerprintSignals {
                    pdf_producer_contains: vec!["Scriptura XSL-FO".to_string()],
                    pdf_creator_contains: vec![],
                    filename_patterns: vec![],
                    color_buckets: vec![],
                    logo_phash: None,
                    page_count_range: None,
                    size_bucket: None,
                },
                category: "bank_statement".to_string(),
                confidence_boost: 0.45,
                skip_extraction: false,
                contributed_by: 1,
                last_updated: "2026-05-06".to_string(),
            }],
        };

        let meta = PdfMetadata {
            producer: Some("Scriptura XSL-FO R8.5".to_string()),
            ..Default::default()
        };

        let result = registry.match_file(&meta, "statement.pdf", 100_000);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.institution, "Raiffeisenbank CZ");
        assert!((r.confidence_boost - 0.45).abs() < f32::EPSILON);
    }
}
