// src-tauri/src/ocr_store.rs
//
// Disk cache for OCR results produced by the Indra sidecar.
//
// Layout: <app_data_dir>/ocr-cache/<sha256>.json
//
// The cache is keyed by content hash, not by path: the same document copied to
// three folders is OCR'd once, and moving a file does not invalidate its result.
// A cache hit therefore costs zero sidecar invocations.
//
// Every failure here is non-fatal. A missing, unreadable or corrupt entry is
// reported as "no cached result" so the caller simply re-runs the sidecar.

use crate::types::OcrResult;
use log::{debug, warn};
use std::fs;
use std::path::{Path, PathBuf};

/// Sub-directory of AppData holding the per-hash JSON entries.
pub const CACHE_DIR_NAME: &str = "ocr-cache";

/// Root of the OCR cache. Created lazily on first write.
pub fn cache_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(CACHE_DIR_NAME)
}

/// Cache file path for a content hash. The hash is already hex, so it is safe
/// to use directly as a file name on every platform we target.
pub fn cache_path_for_hash(app_data_dir: &Path, sha256: &str) -> PathBuf {
    cache_dir(app_data_dir).join(format!("{}.json", sha256))
}

/// Read a cached result. Returns Ok(None) on any miss — absent file, unreadable
/// file, or JSON that no longer matches the current `OcrResult` shape.
pub fn read(app_data_dir: &Path, sha256: &str) -> Option<OcrResult> {
    let path = cache_path_for_hash(app_data_dir, sha256);
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return None,
    };
    match serde_json::from_str::<OcrResult>(&raw) {
        Ok(result) => {
            debug!("OCR cache hit: {}", path.display());
            Some(result)
        }
        Err(e) => {
            // A stale entry written by an older schema is simply re-computed.
            debug!("OCR cache entry unreadable ({}), will re-run: {}", path.display(), e);
            None
        }
    }
}

/// Write a result to the cache. Failures are logged and swallowed — the batch
/// result is still correct, the next run just costs one extra sidecar call.
pub fn write(app_data_dir: &Path, result: &OcrResult) {
    let sha = match result.sha256.as_deref() {
        Some(s) if !s.is_empty() => s,
        _ => {
            warn!("OCR result without sha256 not cached: {}", result.path);
            return;
        }
    };

    let dir = cache_dir(app_data_dir);
    if let Err(e) = fs::create_dir_all(&dir) {
        warn!("Cannot create OCR cache dir {:?}: {}", dir, e);
        return;
    }

    let path = cache_path_for_hash(app_data_dir, sha);
    match serde_json::to_string_pretty(result) {
        Ok(json) => {
            if let Err(e) = fs::write(&path, json) {
                warn!("Cannot write OCR cache entry {:?}: {}", path, e);
            }
        }
        Err(e) => warn!("Cannot serialise OCR result for {:?}: {}", path, e),
    }
}

/// Number of entries currently cached. Used by the Dashboard status row.
/// Unreadable or non-JSON files are ignored rather than counted.
pub fn count(app_data_dir: &Path) -> u32 {
    let dir = cache_dir(app_data_dir);
    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .count() as u32
}
