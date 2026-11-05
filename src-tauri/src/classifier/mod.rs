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
mod formats;
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
        } else if matches!(category, FileCategory::DataBox | FileCategory::Archive) {
            let (text, used, skipped) = extract_container(&record, &category, ocr).await;
            ocr_skipped = skipped;
            (text, used)
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

/// Text of a container file: a Datová schránka message (.zfo: envelope +
/// attachments) or a ZIP archive (documents inside, within limits). Inner
/// files are written to a temporary folder and read with the normal
/// extractors; PDFs and images go through Indra like any other file (and
/// its content-hash cache). Returns (text, ocr_used, ocr_skipped).
async fn extract_container(
    record: &FileRecord,
    category: &FileCategory,
    ocr: Option<&OcrExtractContext>,
) -> (String, bool, bool) {
    let path = record.path.clone();
    let is_zfo = *category == FileCategory::DataBox;
    let unpacked = tokio::task::spawn_blocking(move || -> anyhow::Result<(String, Vec<formats::InnerFile>)> {
        if is_zfo {
            formats::extract_zfo(&path)
        } else {
            let files = formats::zip_document_entries(&path)?;
            let names: Vec<String> = files.iter().map(|f| f.name.clone()).collect();
            Ok((format!("Archiv ZIP: {}\n", names.join(", ")), files))
        }
    })
    .await;
    let (mut text, files) = match unpacked {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            warn!("Cannot unpack {:?}: {}", record.name, e);
            return (String::new(), false, false);
        }
        Err(e) => {
            warn!("Unpack task failed for {:?}: {}", record.name, e);
            return (String::new(), false, false);
        }
    };

    let tmp = std::env::temp_dir().join(format!("fileit-unpack-{}", uuid::Uuid::new_v4()));
    if std::fs::create_dir_all(&tmp).is_err() {
        return (extractor::truncate_for_classification(text), false, false);
    }
    let (mut ocr_used, mut ocr_skipped) = (false, false);
    for (i, f) in files.into_iter().enumerate() {
        if text.len() >= 16_384 {
            break;
        }
        // Index prefix: two attachments may share a name.
        let safe: String = f.name.chars().map(|c| if "\\/:*?\"<>|".contains(c) { '_' } else { c }).collect();
        let inner = tmp.join(format!("{}_{}", i, safe));
        if std::fs::write(&inner, &f.bytes).is_err() {
            continue;
        }
        let ext = inner.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let cat = FileCategory::from_extension(&ext);
        let inner_text = match cat {
            FileCategory::DataBox | FileCategory::Archive | FileCategory::Other => continue, // no nesting
            FileCategory::Pdf | FileCategory::Image => match ocr {
                Some(ctx) => {
                    let ctx = ctx.clone();
                    let p = inner.clone();
                    let sha = crate::scanner::compute_hash_for_manifest(&inner).unwrap_or_default();
                    match tokio::task::spawn_blocking(move || {
                        crate::ocr::text_for_classification(&ctx.app_data_dir, &ctx.command, &p, &sha, ctx.ocr_enabled)
                    })
                    .await
                    {
                        Ok(Ok(t)) => {
                            ocr_used |= t.is_scan;
                            ocr_skipped |= t.ocr_skipped;
                            t.text
                        }
                        _ => String::new(),
                    }
                }
                None => {
                    ocr_skipped = true;
                    String::new()
                }
            },
            _ => extract_text(&inner, &cat).await.map(|(t, _)| t).unwrap_or_default(),
        };
        if !inner_text.trim().is_empty() {
            text.push_str(&format!("\n--- {} ---\n", f.name));
            text.push_str(&inner_text);
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    (extractor::truncate_for_classification(text), ocr_used, ocr_skipped)
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

#[cfg(test)]
mod doc_type_tests {
    use super::*;

    #[test]
    fn recognises_new_czech_document_types() {
        let dict = include_str!("../../resources/institution_dictionary.json");
        let c = Classifier::new(dict).expect("dictionary loads");
        let cases = [
            ("výpis z katastru nemovitostí, list vlastnictví číslo lv 1234, katastrální území brno, parcela 12/3", DocumentType::PropertyRecord),
            ("žádost o poskytnutí příspěvku. žadatel: jan novák. tímto žádám o", DocumentType::Application),
            ("uděluji souhlas se zpracováním osobních údajů. odvolání souhlasu je možné", DocumentType::Consent),
            ("plná moc. zmocnitel jan novák zmocňuji tímto zmocněnec", DocumentType::PowerOfAttorney),
            ("výpověď smlouvy. tímto vypovídám smlouvu, výpovědní lhůta dva měsíce", DocumentType::Termination),
            ("úvěrová smlouva, hypoteční úvěr, úroková sazba 4,9 %, splátkový kalendář, rpsn", DocumentType::LoanContract),
        ];
        for (text, expected) in cases {
            assert_eq!(c.ner.find_document_type(text), expected, "{}", text);
        }
        let f = c.ner.signals_from_filename("LV_parcela_Brno.pdf");
        assert_eq!(f.doc_type, Some(DocumentType::PropertyRecord));
        let f = c.ner.signals_from_filename("zadost-o-hypoteku.pdf");
        assert!(matches!(f.doc_type, Some(DocumentType::LoanContract) | Some(DocumentType::Application)));
        assert_eq!(DocumentType::InsuranceContract.label_cs(), "Pojistná smlouva");
    }
}

#[cfg(test)]
mod container_tests {
    use super::*;
    use std::io::Write;

    fn record(path: &std::path::Path, ext: &str) -> FileRecord {
        FileRecord {
            path: path.to_path_buf(),
            name: path.file_name().unwrap().to_string_lossy().into_owned(),
            extension: ext.into(),
            size_bytes: std::fs::metadata(path).unwrap().len(),
            modified: chrono::Utc::now(),
            sha256: None,
        }
    }

    #[tokio::test]
    async fn zip_is_classified_from_documents_inside() {
        let c = Classifier::new(include_str!("../../resources/institution_dictionary.json")).unwrap();
        let dir = std::env::temp_dir().join(format!("fileit-zipcls-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("doklady.zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&p).unwrap());
        let o: zip::write::SimpleFileOptions = Default::default();
        z.start_file("faktura.txt", o).unwrap();
        z.write_all("FAKTURA – daňový doklad. IČO: 27082440. Variabilní symbol 2024001. Celkem k úhradě 1 200,00 Kč. Datum splatnosti 15.03.2024".as_bytes()).unwrap();
        z.finish().unwrap();
        let cf = c.classify(record(&p, "zip")).await;
        assert_eq!(cf.document_type, DocumentType::Invoice);
        assert_eq!(cf.metadata.as_ref().and_then(|m| m.ico.clone()).as_deref(), Some("27082440"));
        assert!(cf.text_preview.unwrap_or_default().contains("Archiv ZIP: faktura.txt"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn outlook_msg_subject_and_body_are_read() {
        let c = Classifier::new(include_str!("../../resources/institution_dictionary.json")).unwrap();
        let dir = std::env::temp_dir().join(format!("fileit-msg-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("posta.msg");
        {
            let mut cf = cfb::create(&p).unwrap();
            let utf16 = |s: &str| s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect::<Vec<u8>>();
            cf.create_stream("/__substg1.0_0037001F").unwrap().write_all(&utf16("Výpověď pojistné smlouvy")).unwrap();
            cf.create_stream("/__substg1.0_1000001F").unwrap()
                .write_all(&utf16("Dobrý den, tímto vypovídám smlouvu. Výpovědní lhůta je dva měsíce. Jan Dvořák")).unwrap();
            cf.flush().unwrap();
        }
        let cf = c.classify(record(&p, "msg")).await;
        let preview = cf.text_preview.clone().unwrap_or_default();
        assert!(preview.contains("Předmět: Výpověď pojistné smlouvy"), "{}", preview);
        assert_eq!(cf.document_type, DocumentType::Termination);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
