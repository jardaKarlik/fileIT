// src-tauri/src/classifier/extractor.rs
//
// Text extraction for OFFICE and plaintext formats only.
//
// PDFs and images are extracted EXCLUSIVELY through the Indra OCR sidecar
// (crate::ocr::text_for_classification) — pdf-extract produced mojibake for
// Czech diacritics and Windows OCR was disabled for process-stability
// reasons, so both in-process paths were removed.

use crate::types::FileCategory;
use anyhow::{Context, Result};
use std::io::Read;
use std::path::Path;

const MAX_TEXT_BYTES: usize = 16_384; // 16KB — enough for classification

/// Extract text from a file. Returns (extracted_text, ocr_was_used).
///
/// PDF and image categories are handled by the Indra sidecar in the
/// classification context; this function returns empty text for them.
pub async fn extract_text(path: &Path, category: &FileCategory) -> Result<(String, bool)> {
    match category {
        FileCategory::Word => extract_docx(path).map(|t| (t, false)),
        FileCategory::PowerPoint => extract_pptx(path).map(|t| (t, false)),
        FileCategory::Csv | FileCategory::Xml => extract_plain(path).map(|t| (t, false)),
        FileCategory::Text => extract_plain(path).map(|t| (t, false)),
        FileCategory::Excel => extract_xlsx(path).map(|t| (t, false)),
        FileCategory::Email => extract_plain(path).map(|t| (t, false)),
        // PDFs and images: extraction happens through the Indra sidecar
        // (crate::ocr::text_for_classification), never in-process.
        FileCategory::Pdf | FileCategory::Image | FileCategory::Other => {
            Ok((String::new(), false))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Office Open XML / plaintext extractors
// ─────────────────────────────────────────────────────────────────────────────

fn extract_docx(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Opening DOCX: {:?}", path))?;

    let mut zip = zip::ZipArchive::new(file)
        .with_context(|| "Parsing DOCX as ZIP")?;

    let mut xml_content = String::new();

    if let Ok(mut doc) = zip.by_name("word/document.xml") {
        doc.read_to_string(&mut xml_content)
            .with_context(|| "Reading word/document.xml")?;
    } else {
        return Ok(String::new());
    }

    let text = strip_xml_tags(&xml_content);
    Ok(truncate_text(text))
}

fn extract_xlsx(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Opening XLSX: {:?}", path))?;

    let mut zip = zip::ZipArchive::new(file)
        .with_context(|| "Parsing XLSX as ZIP")?;

    let mut text = String::new();

    if let Ok(mut shared) = zip.by_name("xl/sharedStrings.xml") {
        let mut xml = String::new();
        shared.read_to_string(&mut xml)?;
        text = strip_xml_tags(&xml);
    }

    Ok(truncate_text(text))
}

fn extract_pptx(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Opening PPTX: {:?}", path))?;
    let mut zip = zip::ZipArchive::new(file)
        .with_context(|| "Parsing PPTX as ZIP")?;
    let mut text = String::new();

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name().to_string();
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            let mut xml = String::new();
            entry.read_to_string(&mut xml)?;
            text.push_str(&strip_xml_tags(&xml));
            text.push(' ');
            if text.len() >= MAX_TEXT_BYTES {
                break;
            }
        }
    }

    Ok(truncate_text(text))
}

fn extract_plain(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Opening text file: {:?}", path))?;

    let mut buffer = vec![0u8; MAX_TEXT_BYTES];
    let n = file.read(&mut buffer)
        .with_context(|| "Reading text file")?;
    buffer.truncate(n);

    let text = String::from_utf8_lossy(&buffer).into_owned();
    Ok(text)
}

/// Strip XML/HTML tags from a string, preserving text node content.
fn strip_xml_tags(xml: &str) -> String {
    let mut result = String::with_capacity(xml.len() / 3);
    let mut in_tag = false;
    let mut last_was_space = false;

    for ch in xml.chars() {
        match ch {
            '<' => { in_tag = true; }
            '>' => {
                in_tag = false;
                if !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            }
            _ if !in_tag => {
                result.push(ch);
                last_was_space = ch.is_whitespace();
            }
            _ => {}
        }
    }

    result
}

fn truncate_text(text: String) -> String {
    if text.len() <= MAX_TEXT_BYTES {
        text
    } else {
        let end = text.floor_char_boundary(MAX_TEXT_BYTES);
        let truncated = &text[..end];
        match truncated.rfind(char::is_whitespace) {
            Some(pos) => truncated[..pos].to_string(),
            None => truncated.to_string(),
        }
    }
}
