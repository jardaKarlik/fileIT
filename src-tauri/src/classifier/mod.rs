// src-tauri/src/classifier/mod.rs
//
// FileIT classifier — reads file content and extracts:
//   • Customer identity (rodné číslo, IČO, name)
//   • Issuing institution (from keyword dictionary)
//   • Document type (from trigger phrase matching)
//   • Document date
//   • Overall confidence score

mod extractor;
mod ner;
mod scorer;

pub use extractor::extract_text;
pub use ner::NerEngine;
pub use scorer::score_classification;

use crate::types::*;
use anyhow::Result;
use log::{debug, info, warn};

/// The top-level classifier. Instantiated once, shared across the session.
pub struct Classifier {
    pub ner: NerEngine,
}

/// Optional Stage 1 hints from PDF metadata / CM registry, passed to `classify_with_hints`.
pub struct ClassificationHints {
    /// Canonical institution name from metadata fingerprint.
    pub institution_name: Option<String>,
    /// Additive confidence boost.
    pub confidence_boost: f32,
    /// If true, skip all content extraction for this file.
    pub skip_extraction: bool,
    /// Raw PDF /Producer string for Učebna grouping.
    pub pdf_producer: Option<String>,
    /// Raw PDF /Creator string for Učebna grouping.
    pub pdf_creator: Option<String>,
}

impl Classifier {
    /// Load from the bundled institution dictionary.
    pub fn new(dictionary_json: &str) -> Result<Self> {
        let ner = NerEngine::from_json(dictionary_json)?;
        Ok(Self { ner })
    }

    /// Classify with optional Stage 1 metadata hints.
    /// `hints = None` runs the original pipeline unchanged.
    pub async fn classify_with_hints(
        &self,
        record: FileRecord,
        hints: Option<ClassificationHints>,
    ) -> ClassifiedFile {
        let ext = record.extension.as_str();
        let category = FileCategory::from_extension(ext);

        let (skip_extraction, confidence_boost, pdf_producer, pdf_creator) = match &hints {
            Some(h) => (
                h.skip_extraction,
                h.confidence_boost,
                h.pdf_producer.clone(),
                h.pdf_creator.clone(),
            ),
            None => (false, 0.0f32, None, None),
        };

        // Stage 1 institution from metadata hints
        let hint_institution: Option<InstitutionMatch> = hints
            .as_ref()
            .and_then(|h| h.institution_name.as_deref())
            .and_then(|name| self.ner.find_institution_by_name(name));

        // Stage 2 — filename signals (always runs)
        let filename_signals = self.ner.signals_from_filename(&record.name);

        // Stage 3 — content extraction (skipped when skip_extraction=true)
        info!("Classifying: {:?} (skip_extraction={})", record.name, skip_extraction);
        let (raw_text, ocr_used) = if skip_extraction {
            (String::new(), false)
        } else {
            match extract_text(&record.path, &category).await {
                Ok(result) => result,
                Err(e) => {
                    warn!("Text extraction failed for {:?}: {}", record.path, e);
                    (String::new(), false)
                }
            }
        };

        let text_lower = raw_text.to_lowercase();

        // NER pass on content
        let mut customers = self.ner.find_customers(&raw_text);
        let content_institution = self.ner.find_institution(&text_lower);
        let content_doc_type = self.ner.find_document_type(&text_lower);
        let document_date = self.ner.find_date(&raw_text);

        // Merge: content wins → hint wins → filename fills gap
        let institution = content_institution
            .or(hint_institution)
            .or_else(|| {
                filename_signals.institution_name.as_deref()
                    .and_then(|n| self.ner.find_institution_by_name(n))
            });

        let document_type = if content_doc_type != DocumentType::Unknown {
            content_doc_type
        } else {
            filename_signals.doc_type.clone().unwrap_or(DocumentType::Unknown)
        };

        if customers.is_empty() {
            if let Some(name) = filename_signals.customer_name.clone() {
                customers.push(CustomerMatch {
                    name,
                    rodne_cislo: None,
                    ico: None,
                    confidence: 0.55,
                });
            }
        }

        // Filename corroboration boost
        let filename_corroborates = filename_signals.doc_type
            .as_ref()
            .map(|ft| ft == &document_type && document_type != DocumentType::Unknown)
            .unwrap_or(false)
            || filename_signals.institution_name.as_deref()
                .zip(institution.as_ref())
                .map(|(hint, inst)| hint == inst.canonical_name)
                .unwrap_or(false);

        let primary_customer = customers.first().cloned();
        let base_confidence = score_classification(
            &primary_customer,
            &institution,
            &document_type,
            ocr_used,
            raw_text.len(),
            filename_corroborates,
        );

        // Apply Stage 1 metadata boost
        let confidence = (base_confidence + confidence_boost).min(0.98);

        let text_preview = if raw_text.is_empty() {
            None
        } else {
            Some(raw_text.chars().take(500).collect())
        };

        debug!(
            "Classified {:?}: customer={:?}, inst={:?}, type={:?}, conf={:.2}",
            record.name,
            primary_customer.as_ref().map(|c| &c.name),
            institution.as_ref().map(|i| &i.canonical_name),
            document_type,
            confidence
        );

        ClassifiedFile {
            record,
            category,
            customer: primary_customer,
            all_customers: customers,
            institution,
            document_type,
            document_date,
            confidence,
            ocr_used,
            text_preview,
            is_duplicate: false,
            duplicate_of: None,
            pdf_producer,
            pdf_creator,
        }
    }

    /// Classify without hints — preserves backward compatibility.
    pub async fn classify(&self, record: FileRecord) -> ClassifiedFile {
        self.classify_with_hints(record, None).await
    }
}
