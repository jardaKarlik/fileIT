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
        FileCategory::Pdf   => extract_pdf(path).await,
        FileCategory::Word  => extract_docx(path).map(|t| (t, false)),
        FileCategory::Text  => extract_plain(path).map(|t| (t, false)),
        FileCategory::Image => extract_image_ocr(path).await,
        FileCategory::Excel => extract_xlsx(path).map(|t| (t, false)),
        FileCategory::Email => extract_plain(path).map(|t| (t, false)),
        FileCategory::Other => Ok((String::new(), false)),
    }
}

// pdf_extract::extract_text_from_mem can panic on malformed/encrypted PDFs.
// Run it in spawn_blocking so that with panic="unwind", Tokio catches the panic
// as a JoinError instead of aborting the whole process.
// If native text layer is empty/sparse, fall back to Windows OCR.
async fn extract_pdf(path: &Path) -> Result<(String, bool)> {
    let path_buf = path.to_path_buf();
    let handle = tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path_buf)
            .with_context(|| format!("Reading PDF: {:?}", path_buf))?;
        match pdf_extract::extract_text_from_mem(&bytes) {
            Ok(text) => Ok(text),
            Err(e) => Err(anyhow::anyhow!("pdf-extract: {}", e)),
        }
    });

    let native = match tokio::time::timeout(std::time::Duration::from_secs(10), handle).await {
        Ok(Ok(Ok(text))) => text,
        Ok(Ok(Err(_))) | Ok(Err(_)) => {
            warn!("pdf-extract failed or panicked for {:?} — trying Windows OCR", path);
            String::new()
        }
        Err(_) => {
            warn!("pdf-extract timed out for {:?} — trying Windows OCR", path);
            String::new()
        }
    };

    // Enough native text — no OCR needed
    if native.split_whitespace().count() >= 20 {
        return Ok((truncate_text(native), false));
    }

    // Sparse/empty text layer → likely a scanned PDF. Try Windows OCR.
    let path_buf = path.to_path_buf();
    let ocr = tokio::task::spawn_blocking(move || try_windows_ocr_pdf(&path_buf))
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    if ocr.split_whitespace().count() > native.split_whitespace().count() {
        Ok((truncate_text(ocr), true))
    } else {
        Ok((truncate_text(native), false))
    }
}

/// OCR path for image files (JPG, PNG, TIFF, BMP).
async fn extract_image_ocr(path: &Path) -> Result<(String, bool)> {
    let path_buf = path.to_path_buf();
    let text = tokio::task::spawn_blocking(move || try_windows_ocr_image(&path_buf))
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let used_ocr = !text.is_empty();
    Ok((truncate_text(text), used_ocr))
}

// ─────────────────────────────────────────────────────────────────────────────
// Windows built-in OCR — Windows.Media.Ocr + Windows.Data.Pdf
// Zero external dependencies; uses the same OCR engine as OneNote / Edge.
// ─────────────────────────────────────────────────────────────────────────────

/// Render each PDF page via Windows.Data.Pdf and OCR with Windows.Media.Ocr.
/// Returns None if Windows OCR is unavailable or the PDF yields no text.
#[cfg(target_os = "windows")]
fn try_windows_ocr_pdf(path: &Path) -> Option<String> {
    use windows::{
        core::{Interface, HSTRING},
        Data::Pdf::PdfDocument,
        Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap},
        Media::Ocr::OcrEngine,
        Storage::StorageFile,
        Storage::Streams::{IRandomAccessStream, InMemoryRandomAccessStream},
    };

    let abs = std::fs::canonicalize(path).ok()?;
    let h   = HSTRING::from(abs.to_str()?);

    let file   = StorageFile::GetFileFromPathAsync(&h).ok()?.get().ok()?;
    let doc    = PdfDocument::LoadFromFileAsync(&file).ok()?.get().ok()?;
    // TryCreate returns Result<OcrEngine>; fails gracefully if no language available
    let engine = OcrEngine::TryCreateFromUserProfileLanguages().ok()?;

    let mut text  = String::new();
    let count     = doc.PageCount().ok()?;

    for i in 0..count.min(5) {
        let page   = doc.GetPage(i).ok()?;
        let stream = InMemoryRandomAccessStream::new().ok()?;
        // RenderToStreamAsync writes PNG into the stream
        page.RenderToStreamAsync(&stream).ok()?.get().ok()?;

        // Seek back to beginning so BitmapDecoder can read it
        let istream: IRandomAccessStream = stream.cast().ok()?;
        istream.Seek(0).ok()?;

        let decoder = windows::Graphics::Imaging::BitmapDecoder::CreateAsync(&istream)
            .ok()?.get().ok()?;
        let raw    = decoder.GetSoftwareBitmapAsync().ok()?.get().ok()?;
        // OcrEngine requires Bgra8 format
        let bitmap = SoftwareBitmap::Convert(&raw, BitmapPixelFormat::Bgra8).ok()?;

        if let Some(page_text) = engine.RecognizeAsync(&bitmap).ok()
            .and_then(|op| op.get().ok())
            .and_then(|r| r.Text().ok())
            .map(|h| h.to_string())
        {
            text.push_str(&page_text);
            text.push(' ');
        }
    }

    if text.split_whitespace().count() >= 3 { Some(text) } else { None }
}

#[cfg(not(target_os = "windows"))]
fn try_windows_ocr_pdf(_path: &Path) -> Option<String> { None }

/// OCR an image file (JPG, PNG, TIFF, BMP) via Windows.Media.Ocr.
#[cfg(target_os = "windows")]
fn try_windows_ocr_image(path: &Path) -> Option<String> {
    use windows::{
        core::HSTRING,
        Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap},
        Media::Ocr::OcrEngine,
        Storage::{FileAccessMode, StorageFile},
    };

    let abs = std::fs::canonicalize(path).ok()?;
    let h   = HSTRING::from(abs.to_str()?);

    let file   = StorageFile::GetFileFromPathAsync(&h).ok()?.get().ok()?;
    let stream = file.OpenAsync(FileAccessMode::Read).ok()?.get().ok()?;

    let decoder = windows::Graphics::Imaging::BitmapDecoder::CreateAsync(&stream)
        .ok()?.get().ok()?;
    let raw    = decoder.GetSoftwareBitmapAsync().ok()?.get().ok()?;
    let bitmap = SoftwareBitmap::Convert(&raw, BitmapPixelFormat::Bgra8).ok()?;

    let engine = OcrEngine::TryCreateFromUserProfileLanguages().ok()?;
    let text = engine.RecognizeAsync(&bitmap).ok()
        .and_then(|op| op.get().ok())
        .and_then(|r| r.Text().ok())
        .map(|h| h.to_string())
        .unwrap_or_default();

    if text.split_whitespace().count() >= 3 { Some(text) } else { None }
}

#[cfg(not(target_os = "windows"))]
fn try_windows_ocr_image(_path: &Path) -> Option<String> { None }

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
