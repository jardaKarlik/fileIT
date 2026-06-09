// src-tauri/src/classifier/scorer.rs
//
// Confidence scoring for FileIT classification results.
//
// Score bands:
//   0.85–1.00  High confidence — auto-accept, no review needed
//   0.40–0.84  Medium — flagged for user review in Confidence Review UI (v2)
//   0.00–0.39  Low — classified as Unknown, placed in fallback folder

use crate::types::{CustomerMatch, DocumentType, InstitutionMatch, Score};

/// Compute an overall confidence score from the classification signals.
pub fn score_classification(
    customer: &Option<CustomerMatch>,
    institution: &Option<InstitutionMatch>,
    document_type: &DocumentType,
    ocr_used: bool,
    text_length: usize,
    filename_corroborates: bool,
) -> Score {
    let mut score: f32 = 0.0;
    let mut weight_total: f32 = 0.0;

    // ── Customer (weight 0.40) ─────────────────────────────────────────────
    // Rodné číslo identification is the strongest single signal in Czech
    // financial document context.
    let customer_weight = 0.40_f32;
    if let Some(c) = customer {
        score += c.confidence * customer_weight;
    }
    weight_total += customer_weight;

    // ── Institution (weight 0.35) ──────────────────────────────────────────
    let inst_weight = 0.35_f32;
    if let Some(i) = institution {
        score += i.confidence * inst_weight;
    }
    weight_total += inst_weight;

    // ── Document type (weight 0.25) ────────────────────────────────────────
    let doc_weight = 0.25_f32;
    let type_conf = match document_type {
        DocumentType::Unknown => 0.0,
        _ => 0.80, // any non-unknown type classification is reasonably confident
    };
    score += type_conf * doc_weight;
    weight_total += doc_weight;

    // Normalise to 0-1
    let base = if weight_total > 0.0 {
        score / weight_total
    } else {
        0.0
    };

    // ── Modifiers ─────────────────────────────────────────────────────────

    // OCR-extracted text is less reliable — apply a modest penalty
    let ocr_penalty = if ocr_used { 0.10 } else { 0.0 };

    // Very short text means we had little to work with
    let text_penalty = if text_length < 100 {
        0.15
    } else if text_length < 300 {
        0.05
    } else {
        0.0
    };

    // Boost when we have both customer AND institution — corroborating signals
    let corroboration_boost = if customer.is_some() && institution.is_some() {
        0.06
    } else {
        0.0
    };

    // Boost when filename independently agrees with content on type or institution
    let filename_boost = if filename_corroborates { 0.08 } else { 0.0 };

    let final_score = (base + corroboration_boost + filename_boost - ocr_penalty - text_penalty)
        .clamp(0.0, 1.0);

    // Round to 2 decimal places for cleaner display
    (final_score * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CustomerMatch, InstitutionCategory, InstitutionMatch};

    fn make_customer(conf: f32) -> CustomerMatch {
        CustomerMatch {
            name: "Novák Pavel".to_string(),
            rodne_cislo: Some("8001011234".to_string()),
            ico: None,
            confidence: conf,
        }
    }

    fn make_institution(conf: f32) -> InstitutionMatch {
        InstitutionMatch {
            canonical_name: "Česká spořitelna".to_string(),
            category: InstitutionCategory::Bank,
            confidence: conf,
        }
    }

    #[test]
    fn high_confidence_with_all_signals() {
        let score = score_classification(
            &Some(make_customer(0.92)),
            &Some(make_institution(0.90)),
            &DocumentType::BankStatement,
            false,
            2000,
            false,
        );
        assert!(score >= 0.85, "Expected high confidence, got {}", score);
    }

    #[test]
    fn medium_confidence_with_institution_only() {
        let score = score_classification(
            &None,
            &Some(make_institution(0.75)),
            &DocumentType::Invoice,
            false,
            800,
            false,
        );
        assert!(score >= 0.40 && score < 0.85, "Expected medium confidence, got {}", score);
    }

    #[test]
    fn low_confidence_with_no_signals() {
        let score = score_classification(
            &None,
            &None,
            &DocumentType::Unknown,
            true,
            50,
            false,
        );
        assert!(score < 0.40, "Expected low confidence, got {}", score);
    }
}
