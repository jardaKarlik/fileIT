// src-tauri/src/ocr/mod.rs
//
// OCR extraction module for FileIT.
//
// Phase 2 feature (v1.0.9):
// - Tesseract integration for image text extraction
// - Currently stubbed for compatibility
// - Graceful fallback to filename-based classification
//
// TODO: Replace stub with actual tesseract-ocr crate

use log::warn;
use std::path::Path;

/// Extract text from image file using OCR.
/// Currently returns empty string (stub for Phase 2).
///
/// # Arguments
/// * `image_path` - Path to image file (JPG, PNG, TIFF, etc.)
///
/// # Returns
/// Extracted text or empty string if extraction fails
pub fn extract_text_from_image(image_path: &Path) -> String {
    // STUB: Phase 2 will implement real OCR using tesseract-ocr crate
    warn!("OCR stub called for {:?} — Phase 2 will implement real extraction", image_path);
    String::new()
}

/// Extract text from PDF using OCR (images) or direct PDF reading.
/// Currently returns empty string (stub for Phase 2).
pub fn extract_text_from_pdf(_pdf_path: &Path) -> String {
    // STUB: Phase 2 will implement real PDF text extraction
    String::new()
}

/// Preprocess image for better OCR accuracy (rotation, denoise, contrast).
/// Currently no-op (stub for Phase 2).
pub fn preprocess_image_for_ocr(_image_path: &Path) -> Vec<u8> {
    // STUB: Phase 2 will add preprocessing
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_returns_empty() {
        let result = extract_text_from_image(Path::new("dummy.jpg"));
        assert_eq!(result, "");
    }
}
