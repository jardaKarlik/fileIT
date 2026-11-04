// src-tauri/src/extraction_cache.rs
//
// Persistent cache of text-extraction / OCR results, so a file that was
// already read (and possibly OCR'd) in an earlier scan is not processed again
// in the next scan cycle.
//
// Design
// ──────
// • Keyed by file *content* (SHA-256 of the first 64 KB, as computed by the
//   scanner, plus the exact file size) — not by path. Files that were moved by
//   a restructure, renamed, or copied elsewhere still hit the cache.
// • One small JSON file per entry under `<AppData>/extract_cache/`. Entries are
//   written as soon as a file is extracted, so a crashed or cancelled scan
//   keeps everything processed so far.
// • Only the expensive part (reading text / OCR) is cached. Classification
//   (NER, institution and document-type matching) re-runs on the cached text
//   every scan, so dictionary and CM-registry updates still apply.
// • An entry produced without OCR for a file that needs OCR (scanned PDF,
//   image) is upgraded once when the user scans with OCR enabled. After an OCR
//   attempt the entry is final — OCR is never repeated for the same content.
// • Bump CACHE_VERSION when extraction logic changes in a way that should
//   invalidate old results.

use chrono::{DateTime, Utc};
use log::warn;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const CACHE_VERSION: u32 = 1;
const CACHE_DIR: &str = "extract_cache";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub version: u32,
    pub size_bytes: u64,
    pub sha256: String,
    /// Extracted text (already truncated to the extractor's limit).
    pub text: String,
    /// True if `text` came from OCR.
    pub ocr_used: bool,
    /// "indra" or "windows" when OCR produced the text.
    pub ocr_engine: Option<String>,
    /// True if OCR was run for this content (whether or not it found text).
    pub ocr_attempted: bool,
    /// True if the native text layer was missing/sparse, i.e. OCR could help.
    pub needs_ocr: bool,
    pub extracted_at: DateTime<Utc>,
}

impl CacheEntry {
    /// Can this entry be used for a scan with the given OCR setting?
    pub fn is_reusable(&self, ocr_enabled: bool) -> bool {
        if self.version != CACHE_VERSION {
            return false;
        }
        // Without OCR, any earlier result is at least as good as a new one.
        // With OCR, re-extract only if OCR could help and was never tried.
        !ocr_enabled || !self.needs_ocr || self.ocr_attempted
    }
}

pub struct ExtractionCache {
    dir: PathBuf,
}

impl ExtractionCache {
    pub fn open(app_data_dir: &Path) -> Self {
        let dir = app_data_dir.join(CACHE_DIR);
        if let Err(e) = std::fs::create_dir_all(&dir) {
            warn!("Cannot create extraction cache dir {:?}: {}", dir, e);
        }
        Self { dir }
    }

    fn entry_path(&self, sha256: &str, size_bytes: u64) -> Option<PathBuf> {
        // The hash comes from our own scanner, but validate anyway — it is
        // used as a file name.
        if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        Some(self.dir.join(format!("{}-{}.json", sha256.to_ascii_lowercase(), size_bytes)))
    }

    pub fn get(&self, sha256: &str, size_bytes: u64) -> Option<CacheEntry> {
        let path = self.entry_path(sha256, size_bytes)?;
        let bytes = std::fs::read(&path).ok()?;
        match serde_json::from_slice::<CacheEntry>(&bytes) {
            Ok(e) if e.size_bytes == size_bytes && e.sha256.eq_ignore_ascii_case(sha256) => Some(e),
            Ok(_) => None,
            Err(e) => {
                warn!("Corrupt extraction cache entry {:?}: {}", path, e);
                let _ = std::fs::remove_file(&path);
                None
            }
        }
    }

    pub fn put(&self, entry: &CacheEntry) {
        let Some(path) = self.entry_path(&entry.sha256, entry.size_bytes) else { return };
        let tmp = path.with_extension("json.tmp");
        let result = serde_json::to_vec(entry)
            .map_err(std::io::Error::other)
            .and_then(|bytes| std::fs::write(&tmp, bytes))
            .and_then(|_| std::fs::rename(&tmp, &path));
        if let Err(e) = result {
            warn!("Cannot write extraction cache entry {:?}: {}", path, e);
            let _ = std::fs::remove_file(&tmp);
        }
    }

    /// Delete all cached entries. Returns the number of entries removed.
    pub fn clear(&self) -> std::io::Result<usize> {
        let mut removed = 0;
        for entry in std::fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                std::fs::remove_file(&path)?;
                removed += 1;
            }
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(needs_ocr: bool, ocr_attempted: bool) -> CacheEntry {
        CacheEntry {
            version: CACHE_VERSION,
            size_bytes: 42,
            sha256: "a".repeat(64),
            text: "hello".into(),
            ocr_used: false,
            ocr_engine: None,
            ocr_attempted,
            needs_ocr,
            extracted_at: Utc::now(),
        }
    }

    #[test]
    fn reuse_rules() {
        // Native text was fine → always reuse
        assert!(entry(false, false).is_reusable(true));
        assert!(entry(false, false).is_reusable(false));
        // Scanned doc, OCR never tried → reuse only when OCR is off
        assert!(entry(true, false).is_reusable(false));
        assert!(!entry(true, false).is_reusable(true));
        // Scanned doc, OCR already tried → never OCR again
        assert!(entry(true, true).is_reusable(true));
        // Old cache version → re-extract
        let mut old = entry(false, false);
        old.version = CACHE_VERSION + 1;
        assert!(!old.is_reusable(false));
    }

    #[test]
    fn roundtrip_and_clear() {
        let dir = std::env::temp_dir().join(format!("fileit-cache-test-{}", uuid::Uuid::new_v4()));
        let cache = ExtractionCache::open(&dir);
        let e = entry(true, true);
        cache.put(&e);
        let got = cache.get(&e.sha256, e.size_bytes).expect("entry present");
        assert_eq!(got.text, "hello");
        assert!(cache.get(&e.sha256, 43).is_none(), "size is part of the key");
        assert!(cache.get("not-a-hash", 42).is_none());
        assert_eq!(cache.clear().unwrap(), 1);
        assert!(cache.get(&e.sha256, e.size_bytes).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
