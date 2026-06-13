// src-tauri/src/scanner/mod.rs
//
// Async filesystem scanner for FileIT.
//
// Responsibilities:
//   1. Walk the requested root directories
//   2. Filter by allowed file categories
//   3. Compute SHA-256 digest for deduplication detection
//   4. Emit progress events to the frontend via Tauri event channel
//   5. Return a Vec<FileRecord> for the classifier to process

use crate::types::{FileCategory, FileRecord, ScanProgress};
use anyhow::Result;
use log::{debug, info, warn};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

// Extensions we will process — mapped to FileCategory
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "pdf", "doc", "docx", "xls", "xlsx", "csv",
    "jpg", "jpeg", "png", "bmp", "tif", "tiff",
    "msg", "eml", "txt", "rtf",
];

// Directories to always skip — system, hidden, OEM restore partitions
const SKIP_DIRS: &[&str] = &[
    "Windows", "Program Files", "Program Files (x86)",
    "ProgramData", "AppData", "$Recycle.Bin", "System Volume Information",
    "Recovery", "RECOVERY", "$SysReset", "MSOCache",
    "node_modules", ".git", ".svn",
];

/// Public hash helper used by the restructure module before moves.
pub fn compute_hash_for_manifest(path: &Path) -> Option<String> {
    compute_partial_hash(path, 65_536)
}

/// Scan roots for files matching the requested categories.
/// Emits "scan_progress" events to the frontend as it goes.
pub async fn scan_directories(
    app: &AppHandle,
    roots: Vec<PathBuf>,
    allowed_categories: &[FileCategory],
    extra_skip_dirs: &[PathBuf],
) -> Result<Vec<FileRecord>> {
    let mut records: Vec<FileRecord> = Vec::new();
    let mut hash_map: HashMap<String, PathBuf> = HashMap::new(); // sha256 → first_path
    let mut files_seen = 0usize;

    info!("Scanner: {} roots, {} allowed categories ({:?})",
        roots.len(), allowed_categories.len(), allowed_categories);

    for root in &roots {
        info!("Scanning root: {:?} (exists={})", root, root.exists());
        if !root.exists() {
            warn!("Scan root does not exist: {:?}", root);
            continue;
        }

        emit_progress(app, files_seen, &format!("Prohlížíme {:?}…", root), false);

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !should_skip(e.path(), extra_skip_dirs))
        {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("WalkDir error: {}", e);
                    continue;
                }
            };

            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path().to_path_buf();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();

            // Check if this extension is in the allowed categories
            let category = FileCategory::from_extension(&ext);
            if !allowed_categories.is_empty() && !allowed_categories.contains(&category) {
                debug!("Skip (category): {:?}", path.file_name().unwrap_or_default());
                continue;
            }

            // Get metadata
            let metadata = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(e) => {
                    warn!("Cannot stat {:?}: {}", path, e);
                    continue;
                }
            };

            let size_bytes = metadata.len();
            let modified = metadata
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .into();

            // Compute SHA-256 for dedup (only first 64KB for speed)
            let sha256 = compute_partial_hash(&path, 65_536);

            // Mark duplicates
            let is_dup = if let Some(ref hash) = sha256 {
                if let Some(_existing) = hash_map.get(hash) {
                    true
                } else {
                    hash_map.insert(hash.clone(), path.clone());
                    false
                }
            } else {
                false
            };

            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            info!("Found: {:?}", name);
            files_seen += 1;

            // Emit progress every 10 files to avoid flooding the IPC bridge
            if files_seen % 10 == 0 {
                emit_progress(
                    app,
                    files_seen,
                    &format!("Čteme: {}", name),
                    false,
                );
            }

            let record = FileRecord {
                path,
                name,
                extension: ext,
                size_bytes,
                modified,
                sha256,
            };

            records.push(record);

            debug!("Found file {} ({} bytes, dup={})", files_seen, size_bytes, is_dup);
        }
    }

    // Mark duplicates in the records
    mark_duplicates(&mut records);

    emit_progress(
        app,
        files_seen,
        &format!("Nalezeno {} souborů", files_seen),
        true,
    );

    info!("Scan complete: {} files found across {} roots", records.len(), roots.len());
    Ok(records)
}

/// Compute SHA-256 of the first `max_bytes` of a file.
/// Returns None if the file can't be read.
fn compute_partial_hash(path: &Path, max_bytes: usize) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; max_bytes.min(65_536)];

    let n = file.read(&mut buffer).ok()?;
    hasher.update(&buffer[..n]);
    let result = hasher.finalize();
    Some(hex::encode(result))
}

/// Mark duplicate files in the records list.
/// A file is a duplicate if another file with the same SHA-256 was seen earlier.
fn mark_duplicates(records: &mut Vec<FileRecord>) {
    // We already detect dups during scanning; this is a second pass
    // to ensure the Vec accurately reflects duplicates even if
    // scan order was non-deterministic.
    let mut seen: HashMap<String, usize> = HashMap::new(); // hash → first index

    // Collect hashes first to avoid borrow issues
    let hashes: Vec<Option<String>> = records.iter()
        .map(|r| r.sha256.clone())
        .collect();

    // Note: duplicate marking is done at the ClassifiedFile level
    // The FileRecord just carries the hash — ClassifiedFile.is_duplicate is set
    // during the classifier pass when we see the same hash twice.
    // This function is a no-op for now but kept as an extension point.
    let _ = seen;
    let _ = hashes;
}

/// Returns true if this path should be skipped during scanning.
fn should_skip(path: &Path, extra_skip_dirs: &[PathBuf]) -> bool {
    // Check component names against the skip list
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        // Skip hidden files/dirs (dot-prefix)
        if name.starts_with('.') {
            return true;
        }
        // Skip known system directories
        if SKIP_DIRS.contains(&name) {
            return true;
        }
    }

    // Check user-specified extra skip paths
    for skip in extra_skip_dirs {
        if path.starts_with(skip) {
            return true;
        }
    }

    false
}

/// Emit a scan_progress event to the frontend.
fn emit_progress(app: &AppHandle, files_found: usize, message: &str, done: bool) {
    let _ = app.emit(
        "scan_progress",
        ScanProgress {
            files_found,
            current_path: message.to_string(),
            message: message.to_string(),
            done,
        },
    );
}

/// Detect installed cloud sync clients by checking for their well-known
/// local sync directories on Windows.
pub fn detect_cloud_clients() -> Vec<crate::types::DetectedCloud> {
    let mut clouds = Vec::new();

    #[cfg(target_os = "windows")]
    {
        use std::path::PathBuf;

        let user_profile = std::env::var("USERPROFILE")
            .unwrap_or_else(|_| "C:\\Users\\User".to_string());

        let candidates = vec![
            ("OneDrive", format!("{}\\OneDrive", user_profile)),
            ("OneDrive - Business", format!("{}\\OneDrive - Business", user_profile)),
            ("Dropbox", format!("{}\\Dropbox", user_profile)),
            ("Google Drive", format!("{}\\Google Drive", user_profile)),
            ("Google Drive (G:)", "G:\\My Drive".to_string()),
            ("Box", format!("{}\\Box", user_profile)),
            ("Box Sync", format!("{}\\Box Sync", user_profile)),
        ];

        for (name, path_str) in candidates {
            let path = PathBuf::from(&path_str);
            if path.exists() && path.is_dir() {
                // Check if the sync client process is running (heuristic)
                let is_running = is_sync_client_running(name);
                clouds.push(crate::types::DetectedCloud {
                    name: name.to_string(),
                    root_path: path,
                    is_running,
                });
            }
        }
    }

    clouds
}

#[cfg(target_os = "windows")]
fn is_sync_client_running(cloud_name: &str) -> bool {
    // Check for process by querying tasklist — a lightweight heuristic.
    // In production this could use winapi to enumerate processes.
    let process_name = match cloud_name {
        n if n.contains("OneDrive") => "OneDrive.exe",
        n if n.contains("Dropbox") => "Dropbox.exe",
        n if n.contains("Google Drive") => "googledrivesync.exe",
        n if n.contains("Box") => "Box.exe",
        _ => return false,
    };

    std::process::Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {}", process_name), "/NH"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(process_name))
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn is_sync_client_running(_cloud_name: &str) -> bool {
    false
}
