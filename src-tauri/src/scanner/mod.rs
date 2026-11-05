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

// Directories to always skip when scanning.
// Comprehensive list for whole-drive scanning — system, dev tooling,
// package caches, game assets, and runtime data that never contain
// user financial documents.
const SKIP_DIRS: &[&str] = &[
    // Windows system
    "Windows", "Program Files", "Program Files (x86)",
    "ProgramData", "$Recycle.Bin", "System Volume Information",
    "Recovery", "RECOVERY", "$SysReset", "MSOCache",
    "$WinREAgent", "PerfLogs", "Intel", "AMD",
    "Windows.old", "$Windows.~BT", "$Windows.~WS", "$GetCurrent",
    "Config.Msi", "OneDriveTemp", "msys64", "cygwin64",
    // User profile noise
    "AppData", ".cache", ".local", ".config",
    // Dev tooling & package managers
    "node_modules", ".git", ".svn", ".hg",
    ".cargo", ".rustup", ".npm", ".yarn", ".pnpm-store",
    ".nuget", ".gradle", ".m2", ".pub-cache",
    "target", "dist", "build", "__pycache__", ".venv", "venv",
    // IDE / editor data
    ".vscode", ".idea", ".vs",
    // Specific software/content folders requested for exclusion under
    // Documents. Matching is case-insensitive below. Keep this list narrow:
    // ordinary work folders must remain scannable.
    "Native Instruments", "Samples", "Claude", "iZotope", "Lexicon",
    "League of Legends", "Magda", "Madga", "McAfee",
    // Tauri / Electron build output
    "src-tauri",
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
    let mut cloud_only_skipped = 0usize;

    info!("Scanner: {} roots, {} allowed categories ({:?})",
        roots.len(), allowed_categories.len(), allowed_categories);

    for root in &roots {
        info!("Scanning root: {:?} (exists={})", root, root.exists());
        if !root.exists() {
            warn!("Scan root does not exist: {:?}", root);
            continue;
        }

        emit_progress(app, files_seen, &format!("Prohlížíme {:?}…", root), None, false);

        // depth 0 is the root the user chose — never skip it by name.
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !should_skip(e.path(), extra_skip_dirs))
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

            // Office lock/owner files ("~$smlouva.docx") are not documents.
            if entry.file_name().to_string_lossy().starts_with("~$") {
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

            // OneDrive / cloud "online-only" placeholders: reading them would
            // download the whole file. Skip them instead of hydrating.
            if is_cloud_placeholder(&metadata) {
                cloud_only_skipped += 1;
                debug!("Skip (cloud-only placeholder): {:?}", path);
                continue;
            }

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

            debug!("Found: {:?}", name);
            files_seen += 1;

            // Emit progress every 10 files to avoid flooding the IPC bridge
            if files_seen % 10 == 0 {
                emit_progress(
                    app,
                    files_seen,
                    &format!("Čteme: {}", name),
                    Some(&path),
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
        None,
        true,
    );

    info!(
        "Scan complete: {} files found across {} roots ({} cloud-only placeholders skipped)",
        records.len(), roots.len(), cloud_only_skipped
    );
    Ok(records)
}

/// True for cloud-sync placeholders whose content is not on disk
/// (OneDrive "Files On-Demand", etc.).
#[cfg(target_os = "windows")]
fn is_cloud_placeholder(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
    const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
    const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
    metadata.file_attributes()
        & (FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_RECALL_ON_OPEN | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        != 0
}

#[cfg(not(target_os = "windows"))]
fn is_cloud_placeholder(_metadata: &std::fs::Metadata) -> bool {
    false
}

/// Default scan roots: every local fixed drive (C:\, D:\, …). Removable,
/// network and optical drives are left out. Falls back to the system drive.
#[cfg(target_os = "windows")]
pub fn default_scan_roots() -> Vec<PathBuf> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetDriveTypeW(root_path_name: *const u16) -> u32;
    }
    const DRIVE_FIXED: u32 = 3;

    let mut roots = Vec::new();
    for letter in b'A'..=b'Z' {
        let root = format!("{}:\\", letter as char);
        let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: `wide` is a valid NUL-terminated UTF-16 string that outlives the call.
        let kind = unsafe { GetDriveTypeW(wide.as_ptr()) };
        if kind == DRIVE_FIXED {
            roots.push(PathBuf::from(root));
        }
    }
    if roots.is_empty() {
        let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
        roots.push(PathBuf::from(format!("{}\\", drive)));
    }
    roots
}

#[cfg(not(target_os = "windows"))]
pub fn default_scan_roots() -> Vec<PathBuf> {
    vec![std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))]
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
    let seen: HashMap<String, usize> = HashMap::new(); // hash → first index

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
        // Skip known system/software directories. Windows paths are
        // case-insensitive, so normalize both sides before comparing.
        if SKIP_DIRS.iter().any(|skip| name.eq_ignore_ascii_case(skip)) {
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

/// Emit a scan_progress event to the frontend. `current_path` carries the
/// full path of the file just found (empty for status-only events) so the
/// frontend live visualization can build the folder tree while scanning.
fn emit_progress(
    app: &AppHandle,
    files_found: usize,
    message: &str,
    current_path: Option<&Path>,
    done: bool,
) {
    let _ = app.emit(
        "scan_progress",
        ScanProgress {
            files_found,
            current_path: current_path
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
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

#[cfg(test)]
mod tests {
    use super::should_skip;
    use std::path::Path;

    #[test]
    #[cfg(target_os = "windows")] // uses C:\\ paths, which only Windows splits into components
    fn skips_software_folders_case_insensitively() {
        assert!(should_skip(
            Path::new(r"C:\Users\jaros\Documents\Native Instruments"),
            &[]
        ));
        assert!(should_skip(
            Path::new(r"C:\Users\jaros\Documents\mcafee"),
            &[]
        ));
        assert!(should_skip(
            Path::new(r"C:\Users\jaros\Documents\League of Legends"),
            &[]
        ));
    }

    #[test]
    fn keeps_normal_document_folders() {
        assert!(!should_skip(
            Path::new(r"C:\Users\jaros\Documents\Kooperativa\2024"),
            &[]
        ));
        assert!(!should_skip(
            Path::new(r"C:\Users\jaros\Documents\Ableton Projects"),
            &[]
        ));
        assert!(!should_skip(
            Path::new(r"C:\Users\jaros\Documents\Work\Client Files"),
            &[]
        ));
    }
}
