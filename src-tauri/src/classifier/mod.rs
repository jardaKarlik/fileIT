// src-tauri/src/classifier/mod.rs
//
// FileIT classifier — reads file content and extracts:
//   • Customer identity (rodné číslo, IČO, name)
//   • Issuing institution (from keyword dictionary)
//   • Document type (from trigger phrase matching)
//   • Document date
//   • Overall confidence score

mod extractor;
mod extract_metadata;
mod ner;
mod scorer;

pub use extract_metadata::{extract_metadata, ExtractedMetadata};
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
    /// `ocr` carries the Indra sidecar context used to extract text from
    /// PDFs and images; `None` degrades those categories to path-signal-only
    /// classification.
    pub async fn classify_with_hints(
        &self,
        record: FileRecord,
        hints: Option<ClassificationHints>,
        ocr: Option<&OcrExtractContext>,
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

        // Stage 3 — content extraction (skipped when skip_extraction=true).
        // PDFs and images go EXCLUSIVELY through the Indra sidecar (cache
        // first, live run on miss) — pdf-extract and Windows OCR are gone.
        // Office/plaintext formats keep their native extractors.
        info!("Classifying: {:?} (skip_extraction={})", record.name, skip_extraction);
        let mut ocr_skipped = false;
        let (raw_text, ocr_used) = if skip_extraction {
            (String::new(), false)
        } else if matches!(category, FileCategory::Pdf | FileCategory::Image) {
            match ocr {
                Some(ctx) => {
                    let ctx = ctx.clone();
                    let path = record.path.clone();
                    let sha = record.sha256.clone().unwrap_or_default();
                    match tokio::task::spawn_blocking(move || {
                        crate::ocr::text_for_classification(
                            &ctx.app_data_dir,
                            &ctx.command,
                            std::path::Path::new(&path),
                            &sha,
                            ctx.ocr_enabled,
                        )
                    })
                    .await
                    {
                        Ok(Ok(t)) => {
                            ocr_skipped = t.ocr_skipped;
                            (t.text, t.is_scan)
                        }
                        Ok(Err(e)) => {
                            warn!("Indra extraction failed for {:?}: {}", record.name, e);
                            (String::new(), false)
                        }
                        Err(e) => {
                            warn!("Indra extraction task failed for {:?}: {}", record.name, e);
                            (String::new(), false)
                        }
                    }
                }
                None => {
                    warn!(
                        "Indra sidecar unavailable — classifying {:?} from path signals only",
                        record.name
                    );
                    ocr_skipped = true;
                    (String::new(), false)
                }
            }
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
        // Structured Czech identifiers (IČO, DIČ, IBAN, VS, amounts, dates).
        let metadata = if raw_text.trim().is_empty() {
            None
        } else {
            Some(extract_metadata(&raw_text))
        };

        // Prefer the NER date; fall back to the first Czech date in the text
        // ("15.03.2024" or "15. března 2024") so fewer files end up undated.
        let document_date = self.ner.find_date(&raw_text).or_else(|| {
            metadata
                .as_ref()
                .and_then(|m| m.czech_dates.first())
                .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%d.%m.%Y").ok())
                .and_then(|d| d.and_hms_opt(12, 0, 0))
                .map(|dt| dt.and_utc())
        });

        // Merge: content wins → hint wins → filename dictionary → filename direct
        let institution = content_institution
            .or(hint_institution)
            .or_else(|| {
                filename_signals.institution_name.as_deref()
                    .and_then(|n| self.ner.find_institution_by_name(n))
            })
            .or(filename_signals.institution_direct);

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
            // Classification never runs the Indra sidecar — it only consumes
            // whatever text layer it found. A later explicit OCR run fills this.
            ocr: None,
            ocr_skipped,
            metadata,
        }
    }

    /// Classify without hints — preserves backward compatibility.
    #[allow(dead_code)]
    pub async fn classify(&self, record: FileRecord) -> ClassifiedFile {
        self.classify_with_hints(record, None, None).await
    }
}

/// Indra sidecar context for in-classification text extraction.
#[derive(Debug, Clone)]
pub struct OcrExtractContext {
    pub app_data_dir: std::path::PathBuf,
    pub command: String,
    /// Home-screen OCR switch for this scan.
    pub ocr_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plain-text document goes through the full classifier and its Czech
    /// identifiers end up on the ClassifiedFile.
    #[tokio::test]
    async fn classification_collects_czech_metadata() {
        let dict = include_str!("../../resources/institution_dictionary.json");
        let classifier = Classifier::new(dict).expect("dictionary loads");

        let dir = std::env::temp_dir().join(format!("fileit-meta-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("faktura.txt");
        std::fs::write(
            &path,
            "FAKTURA\nDodavatel: Firma s.r.o., IČO: 27082440, DIČ: CZ27082440\n\
             Číslo účtu (IBAN): CZ65 0800 0000 1920 0014 5399\n\
             Variabilní symbol: 2024001\nDatum vystavení: 15. března 2024\n\
             Celkem k úhradě: 12 500,00 Kč\n",
        )
        .unwrap();

        let record = FileRecord {
            path: path.clone(),
            name: "faktura.txt".into(),
            extension: "txt".into(),
            size_bytes: std::fs::metadata(&path).unwrap().len(),
            modified: chrono::Utc::now(),
            sha256: None,
        };
        let cf = classifier.classify(record).await;
        let m = cf.metadata.expect("metadata collected");
        assert_eq!(m.ico.as_deref(), Some("27082440"));
        assert_eq!(m.dic.as_deref(), Some("CZ27082440"));
        assert_eq!(m.iban.as_deref(), Some("CZ6508000000192000145399"));
        assert_eq!(m.variable_symbol.as_deref(), Some("2024001"));
        assert!(!m.amounts.is_empty(), "amounts: {:?}", m.amounts);
        assert!(m.czech_dates.contains(&"15.03.2024".to_string()), "{:?}", m.czech_dates);
        assert!(cf.document_date.is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
