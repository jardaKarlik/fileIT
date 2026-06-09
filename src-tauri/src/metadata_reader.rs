// src-tauri/src/metadata_reader.rs
//
// Stage 1 of the PDF classification pipeline.
// Reads the last 2 KB of a PDF (trailer + /Info dict) in microseconds.
// Never touches content streams. Never calls pdf-extract.

use chrono::NaiveDate;
use log::warn;
use std::path::Path;

const TRAILER_READ_BYTES: u64 = 2048;

pub struct PdfMetadata {
    pub producer: Option<String>,
    pub creator: Option<String>,
    pub creation_date: Option<NaiveDate>,
    pub title: Option<String>,
    /// Canonical institution name matched from hardcoded fingerprint table.
    pub institution_hint: Option<String>,
    /// Additive confidence boost to apply after Stage 1 match.
    pub confidence_boost: f32,
    /// If true, skip all content extraction — the file yields no useful text.
    pub skip_extraction: bool,
}

impl Default for PdfMetadata {
    fn default() -> Self {
        Self {
            producer: None,
            creator: None,
            creation_date: None,
            title: None,
            institution_hint: None,
            confidence_boost: 0.0,
            skip_extraction: false,
        }
    }
}

/// Read PDF /Info metadata from the last 2 KB of the file.
/// Always returns a valid struct — never panics, never returns Err.
pub fn read_pdf_metadata(path: &Path) -> PdfMetadata {
    match read_inner(path) {
        Ok(m) => m,
        Err(e) => {
            warn!("PDF metadata read failed for {:?}: {}", path, e);
            PdfMetadata::default()
        }
    }
}

fn read_inner(path: &Path) -> anyhow::Result<PdfMetadata> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = std::fs::File::open(path)?;
    let file_size = file.metadata()?.len();
    let read_start = file_size.saturating_sub(TRAILER_READ_BYTES);
    file.seek(SeekFrom::Start(read_start))?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let producer = extract_pdf_string(&buf, b"/Producer");
    let creator = extract_pdf_string(&buf, b"/Creator");
    let title_raw = extract_pdf_string(&buf, b"/Title");
    let creation_date = extract_pdf_string(&buf, b"/CreationDate")
        .and_then(|s| parse_pdf_date(&s));

    let title = title_raw.map(|t| clean_decoded_string(&t));

    let mut meta = PdfMetadata {
        producer,
        creator,
        creation_date,
        title,
        institution_hint: None,
        confidence_boost: 0.0,
        skip_extraction: false,
    };

    apply_hardcoded_fingerprints(&mut meta);
    Ok(meta)
}

// ─────────────────────────────────────────────────────────────────────────────
// PDF string extraction
// ─────────────────────────────────────────────────────────────────────────────

/// Find a PDF key like /Producer and return its string value.
/// Handles literal strings (...) and hex strings <...>.
fn extract_pdf_string(buf: &[u8], key: &[u8]) -> Option<String> {
    let pos = buf.windows(key.len()).position(|w| w == key)?;
    let rest = &buf[pos + key.len()..];

    // Skip whitespace after key
    let start = rest.iter().position(|&b| !matches!(b, b' ' | b'\t' | b'\r' | b'\n'))?;
    let rest = &rest[start..];

    match rest.first()? {
        b'(' => extract_literal_string(&rest[1..]),
        b'<' => {
            let end = rest.iter().skip(1).position(|&b| b == b'>')?;
            let hex_bytes: Vec<u8> = rest[1..1 + end]
                .chunks(2)
                .filter_map(|c| {
                    let s = std::str::from_utf8(c).ok()?;
                    u8::from_str_radix(s.trim(), 16).ok()
                })
                .collect();
            decode_bytes_to_string(&hex_bytes)
        }
        _ => None,
    }
}

fn extract_literal_string(buf: &[u8]) -> Option<String> {
    let mut result = Vec::new();
    let mut depth = 1i32;
    let mut i = 0;

    while i < buf.len() {
        match buf[i] {
            b'\\' if i + 1 < buf.len() => {
                result.push(buf[i + 1]);
                i += 2;
            }
            b'(' => {
                depth += 1;
                result.push(b'(');
                i += 1;
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                result.push(b')');
                i += 1;
            }
            b => {
                result.push(b);
                i += 1;
            }
        }
    }

    decode_bytes_to_string(&result)
}

fn decode_bytes_to_string(bytes: &[u8]) -> Option<String> {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16BE with BOM
        let pairs: Vec<u16> = bytes[2..]
            .chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| ((c[0] as u16) << 8) | (c[1] as u16))
            .collect();
        String::from_utf16(&pairs).ok()
    } else {
        // Latin-1 / ASCII
        Some(bytes.iter().map(|&b| b as char).collect())
    }
}

fn clean_decoded_string(s: &str) -> String {
    s.replace('\0', "").trim().to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Date parsing
// ─────────────────────────────────────────────────────────────────────────────

fn parse_pdf_date(s: &str) -> Option<NaiveDate> {
    // Format: D:YYYYMMDDHHmmSS or D:YYYYMMDD
    let s = s.trim_start_matches("D:");
    if s.len() < 8 {
        return None;
    }
    let year: i32 = s[0..4].parse().ok()?;
    let month: u32 = s[4..6].parse().ok()?;
    let day: u32 = s[6..8].parse().ok()?;
    NaiveDate::from_ymd_opt(year, month, day)
}

// ─────────────────────────────────────────────────────────────────────────────
// Hardcoded fingerprint table — seed data matching cm_registry_seed.json
// ─────────────────────────────────────────────────────────────────────────────

fn apply_hardcoded_fingerprints(meta: &mut PdfMetadata) {
    let producer = meta.producer.as_deref().unwrap_or("").to_lowercase();
    let creator = meta.creator.as_deref().unwrap_or("").to_lowercase();

    if producer.contains("openpdf") {
        // EPO / Finanční úřad forms: generated, no useful content text
        meta.institution_hint = Some("Finanční úřad / EPO".to_string());
        meta.confidence_boost = 0.45;
        meta.skip_extraction = true;
        return;
    }
    if producer.contains("scriptura xsl-fo") {
        meta.institution_hint = Some("Raiffeisenbank CZ".to_string());
        meta.confidence_boost = 0.45;
        return;
    }
    if creator.contains("livecycle designer") {
        meta.institution_hint = Some("ČSOB".to_string());
        meta.confidence_boost = 0.45;
        return;
    }
    if producer.contains("elephant orchestra") {
        meta.institution_hint = Some("ČPP".to_string());
        meta.confidence_boost = 0.45;
        return;
    }
    if producer.contains("abcpdf") {
        // Corroboration signal only — lower boost
        meta.institution_hint = Some("ČPP".to_string());
        meta.confidence_boost = 0.15;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Color extraction — stub pending pdfium-render integration
// ─────────────────────────────────────────────────────────────────────────────

/// Extract dominant color buckets from PDF page 1.
/// Returns empty vec until pdfium-render is wired in.
pub fn extract_dominant_colors(_path: &Path) -> Vec<String> {
    vec![]
}

// ─────────────────────────────────────────────────────────────────────────────
// Size bucketing helper (shared with cm_registry)
// ─────────────────────────────────────────────────────────────────────────────

pub fn size_to_bucket(size_bytes: u64) -> String {
    if size_bytes < 50_000 {
        "XS".to_string()
    } else if size_bytes < 200_000 {
        "S".to_string()
    } else if size_bytes < 1_000_000 {
        "M".to_string()
    } else {
        "L".to_string()
    }
}
