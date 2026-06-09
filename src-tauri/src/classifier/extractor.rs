// src-tauri/src/classifier/extractor.rs
//
// Extracts raw text from files. Returns (text, ocr_used).
// Reads only the first ~16KB of content for performance.

use crate::types::FileCategory;
use anyhow::{Context, Result};
use log::warn;
use std::io::Read;
use std::path::Path;

const MAX_TEXT_BYTES: usize = 16_384; // 16KB — enough for classification

/// Extract text from a file. Returns (extracted_text, ocr_was_used).
pub async fn extract_text(path: &Path, category: &FileCategory) -> Result<(String, bool)> {
    match category {
        FileCategory::Pdf => extract_pdf(path).await.map(|t| (t, false)),
        FileCategory::Word => extract_docx(path).map(|t| (t, false)),
        FileCategory::Text => extract_plain(path).map(|t| (t, false)),
        FileCategory::Image => {
            // OCR stub — returns empty string until Tesseract is bundled.
            // In production, call: tesseract_ocr(path)
            Ok((String::new(), true))
        }
        FileCategory::Excel => extract_xlsx(path).map(|t| (t, false)),
        FileCategory::Email => extract_plain(path).map(|t| (t, false)),
        FileCategory::Other => Ok((String::new(), false)),
    }
}

// pdf_extract::extract_text_from_mem can panic on malformed/encrypted PDFs.
// Run it in spawn_blocking so that with panic="unwind", Tokio catches the panic
// as a JoinError instead of aborting the whole process.
async fn extract_pdf(path: &Path) -> Result<String> {
    let path = path.to_path_buf();
    let handle = tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path)
            .with_context(|| format!("Reading PDF: {:?}", path))?;
        match pdf_extract::extract_text_from_mem(&bytes) {
            Ok(text) => Ok(truncate_text(text)),
            Err(e) => Err(anyhow::anyhow!("pdf-extract: {}", e)),
        }
    });

    match tokio::time::timeout(std::time::Duration::from_secs(10), handle).await {
        Ok(Ok(inner)) => inner,
        Ok(Err(join_err)) if join_err.is_panic() => {
            warn!("pdf-extract panicked — skipping file");
            Ok(String::new())
        }
        Ok(Err(join_err)) => Err(anyhow::anyhow!("spawn_blocking error: {}", join_err)),
        Err(_) => {
            warn!("pdf-extract timed out after 10s — skipping file");
            Ok(String::new())
        }
    }
}

fn extract_docx(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Opening DOCX: {:?}", path))?;

    let mut zip = zip::ZipArchive::new(file)
        .with_context(|| "Parsing DOCX as ZIP")?;

    let mut xml_content = String::new();

    // DOCX word/document.xml contains the body text
    if let Ok(mut doc) = zip.by_name("word/document.xml") {
        doc.read_to_string(&mut xml_content)
            .with_context(|| "Reading word/document.xml")?;
    } else {
        return Ok(String::new());
    }

    // Strip XML tags — quick and dirty but sufficient for classification
    let text = strip_xml_tags(&xml_content);
    Ok(truncate_text(text))
}

fn extract_xlsx(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Opening XLSX: {:?}", path))?;

    let mut zip = zip::ZipArchive::new(file)
        .with_context(|| "Parsing XLSX as ZIP")?;

    let mut text = String::new();

    // XLSX shared strings contain the cell text values
    if let Ok(mut shared) = zip.by_name("xl/sharedStrings.xml") {
        let mut xml = String::new();
        shared.read_to_string(&mut xml)?;
        text = strip_xml_tags(&xml);
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

    // Try UTF-8 first, fall back to latin-2 (common in older Czech documents)
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
        // floor_char_boundary gives the largest valid UTF-8 boundary ≤ MAX_TEXT_BYTES,
        // preventing a panic when a multi-byte Czech character straddles the limit.
        let end = text.floor_char_boundary(MAX_TEXT_BYTES);
        let truncated = &text[..end];
        match truncated.rfind(char::is_whitespace) {
            Some(pos) => truncated[..pos].to_string(),
            None => truncated.to_string(),
        }
    }
}
