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
        FileCategory::Word if has_ext(path, "doc") => super::formats::extract_legacy_office(path).map(|t| (truncate_text(t), false)),
        FileCategory::Word => extract_docx(path).map(|t| (t, false)),
        FileCategory::PowerPoint => extract_pptx(path).map(|t| (t, false)),
        FileCategory::LibreOffice => extract_odf(path).map(|t| (t, false)),
        FileCategory::Csv | FileCategory::Xml => extract_plain(path).map(|t| (t, false)),
        FileCategory::Text => {
            let is_rtf = path.extension().map(|e| e.eq_ignore_ascii_case("rtf")).unwrap_or(false);
            if is_rtf { extract_rtf(path) } else { extract_plain(path) }.map(|t| (t, false))
        }
        FileCategory::Excel if has_ext(path, "xls") => super::formats::extract_legacy_office(path).map(|t| (truncate_text(t), false)),
        FileCategory::Excel => extract_xlsx(path).map(|t| (t, false)),
        FileCategory::Email if has_ext(path, "msg") => super::formats::extract_msg(path).map(|t| (truncate_text(t), false)),
        FileCategory::Email => extract_plain(path).map(|t| (t, false)),
        FileCategory::Isdoc => super::formats::extract_isdoc(path).map(|t| (truncate_text(t), false)),
        FileCategory::Xps => super::formats::extract_xps(path).map(|t| (truncate_text(t), false)),
        // .zfo and .zip contain other files; the classifier unpacks them
        // (it has the OCR context for PDFs and images inside).
        FileCategory::DataBox | FileCategory::Archive => Ok((String::new(), false)),
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

/// LibreOffice / OpenDocument (.odt, .ods, .odp): a ZIP whose text is in
/// content.xml, for all three kinds.
fn extract_odf(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Opening OpenDocument: {:?}", path))?;
    let mut zip = zip::ZipArchive::new(file)
        .with_context(|| "Parsing OpenDocument as ZIP")?;
    let mut xml = String::new();
    match zip.by_name("content.xml") {
        Ok(mut entry) => {
            entry.read_to_string(&mut xml).with_context(|| "Reading content.xml")?;
        }
        Err(_) => return Ok(String::new()),
    }
    Ok(truncate_text(strip_xml_tags(&xml)))
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

fn has_ext(path: &Path, ext: &str) -> bool {
    path.extension().map(|e| e.eq_ignore_ascii_case(ext)).unwrap_or(false)
}

pub(crate) fn truncate_for_classification(text: String) -> String {
    truncate_text(text)
}

fn read_head(path: &Path, max: usize) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Opening text file: {:?}", path))?;
    let mut buffer = Vec::with_capacity(max);
    // `take` + read_to_end: a single read() may return fewer bytes than asked.
    file.take(max as u64)
        .read_to_end(&mut buffer)
        .with_context(|| "Reading text file")?;
    Ok(buffer)
}

fn extract_plain(path: &Path) -> Result<String> {
    Ok(decode_text(&read_head(path, MAX_TEXT_BYTES)?))
}

/// Decode text bytes: BOM (UTF-8 / UTF-16) → UTF-8 → windows-1250.
/// Czech Windows tools (Notepad before 2019, Excel CSV export, many bank
/// exports) write windows-1250; reading that as UTF-8 destroys every
/// diacritic and with it customer names like "Dvořák".
pub(crate) fn decode_text(bytes: &[u8]) -> String {
    if let Some((enc, bom_len)) = encoding_rs::Encoding::for_bom(bytes) {
        let (text, _) = enc.decode_without_bom_handling(&bytes[bom_len..]);
        return text.into_owned();
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        // Valid UTF-8 cut mid-character by the read limit.
        Err(e) if e.error_len().is_none() => {
            String::from_utf8_lossy(&bytes[..e.valid_up_to()]).into_owned()
        }
        Err(_) => encoding_rs::WINDOWS_1250.decode_without_bom_handling(bytes).0.into_owned(),
    }
}

/// Plain text from an RTF document: drops control words, font/colour/style
/// tables and other destination groups, decodes \'hh (windows-1250) and \uN.
fn extract_rtf(path: &Path) -> Result<String> {
    // RTF is verbose: read more than the text limit so the body is reached.
    let raw = read_head(path, MAX_TEXT_BYTES * 8)?;
    Ok(truncate_text(rtf_to_text(&raw)))
}

fn rtf_to_text(raw: &[u8]) -> String {
    const SKIP_DESTS: &[&str] = &[
        "fonttbl", "colortbl", "stylesheet", "info", "pict", "object",
        "header", "footer", "listtable", "listoverridetable", "rsidtbl",
        "generator", "themedata", "colorschememapping", "latentstyles", "datastore",
    ];
    let mut out = String::new();
    let mut pending: Vec<u8> = Vec::new(); // \'hh bytes awaiting decoding
    // Per group: is it skipped?
    let mut stack: Vec<bool> = vec![false];
    let mut skip_chars = 0usize; // fallback chars after \uN
    let flush = |pending: &mut Vec<u8>, out: &mut String| {
        if !pending.is_empty() {
            out.push_str(&encoding_rs::WINDOWS_1250.decode_without_bom_handling(pending).0);
            pending.clear();
        }
    };
    let mut i = 0;
    while i < raw.len() {
        let skipping = *stack.last().unwrap_or(&false);
        let b = raw[i];
        match b {
            b'{' => {
                flush(&mut pending, &mut out);
                // "{\*" starts an ignorable destination.
                let ignorable = raw.get(i + 1) == Some(&b'\\') && raw.get(i + 2) == Some(&b'*');
                stack.push(skipping || ignorable);
                i += 1;
            }
            b'}' => {
                flush(&mut pending, &mut out);
                if stack.len() > 1 { stack.pop(); }
                i += 1;
            }
            b'\\' => {
                let next = raw.get(i + 1).copied().unwrap_or(b' ');
                if next == b'\'' {
                    let hex = raw.get(i + 2..i + 4).and_then(|h| std::str::from_utf8(h).ok());
                    if let Some(v) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                        if !skipping {
                            if skip_chars > 0 { skip_chars -= 1; } else { pending.push(v); }
                        }
                    }
                    i += 4;
                } else if next.is_ascii_alphabetic() {
                    flush(&mut pending, &mut out);
                    let start = i + 1;
                    let mut j = start;
                    while j < raw.len() && raw[j].is_ascii_alphabetic() { j += 1; }
                    let word = std::str::from_utf8(&raw[start..j]).unwrap_or("");
                    let num_start = j;
                    if j < raw.len() && raw[j] == b'-' { j += 1; }
                    while j < raw.len() && raw[j].is_ascii_digit() { j += 1; }
                    let num: Option<i32> = std::str::from_utf8(&raw[num_start..j]).ok().and_then(|n| n.parse().ok());
                    if j < raw.len() && raw[j] == b' ' { j += 1; }
                    if SKIP_DESTS.contains(&word) {
                        if let Some(top) = stack.last_mut() { *top = true; }
                    } else if !skipping {
                        match word {
                            "par" | "line" | "row" | "sect" | "page" => out.push('\n'),
                            "tab" | "cell" => out.push(' '),
                            "u" => {
                                if let Some(n) = num {
                                    let code = if n < 0 { (n + 65536) as u32 } else { n as u32 };
                                    if let Some(c) = char::from_u32(code) { out.push(c); }
                                    skip_chars = 1; // \uc1 default: one fallback char follows
                                }
                            }
                            _ => {}
                        }
                    }
                    i = j;
                } else {
                    // Escaped symbol: \\ \{ \} or \~ (nbsp), \- etc.
                    if !skipping {
                        match next {
                            b'\\' | b'{' | b'}' => out.push(next as char),
                            b'~' => out.push(' '),
                            _ => {}
                        }
                    }
                    i += 2;
                }
            }
            b'\r' | b'\n' => i += 1,
            _ => {
                if !skipping {
                    if skip_chars > 0 {
                        skip_chars -= 1;
                    } else {
                        flush(&mut pending, &mut out);
                        out.push(b as char);
                    }
                }
                i += 1;
            }
        }
    }
    flush(&mut pending, &mut out);
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_czech_encodings() {
        // "Dvořák" in windows-1250
        let cp1250 = [0x44, 0x76, 0x6F, 0xF8, 0xE1, 0x6B];
        assert_eq!(decode_text(&cp1250), "Dvořák");
        assert_eq!(decode_text("Dvořák".as_bytes()), "Dvořák");
        // UTF-16LE with BOM
        let mut u16le = vec![0xFF, 0xFE];
        for u in "Řeka".encode_utf16() { u16le.extend_from_slice(&u.to_le_bytes()); }
        assert_eq!(decode_text(&u16le), "Řeka");
        // UTF-8 cut mid-character keeps the valid prefix
        let cut = &"ář".as_bytes()[..3];
        assert_eq!(decode_text(cut), "á");
    }

    #[test]
    fn reads_libreoffice_documents() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("fileit-odf-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("smlouva.odt");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let opts: zip::write::SimpleFileOptions = Default::default();
        zip.start_file("mimetype", opts).unwrap();
        zip.write_all(b"application/vnd.oasis.opendocument.text").unwrap();
        zip.start_file("content.xml", opts).unwrap();
        zip.write_all("<office:document-content><office:body><office:text><text:p>Smlouva pro Jan Dvořák</text:p><text:p>IČO 27082440</text:p></office:text></office:body></office:document-content>".as_bytes()).unwrap();
        zip.finish().unwrap();
        let text = extract_odf(&path).unwrap();
        assert!(text.contains("Smlouva pro Jan Dvořák"), "{}", text);
        assert!(text.contains("IČO 27082440"), "{}", text);
        assert_eq!(FileCategory::from_extension("ods"), FileCategory::LibreOffice);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rtf_plain_text() {
        let rtf = br"{\rtf1\ansi\ansicpg1250{\fonttbl{\f0 Arial;}}{\*\generator Word;}\f0 Smlouva pro Jan Dvo\'f8\'e1k\par \u345?ada}";
        let t = rtf_to_text(rtf);
        assert!(t.contains("Smlouva pro Jan Dvořák"), "{}", t);
        assert!(t.contains("řada"), "{}", t);
        assert!(!t.contains("Arial") && !t.contains("Word"), "{}", t);
    }
}
