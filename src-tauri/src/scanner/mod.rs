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
    "XboxGames", "WindowsApps", "boot",
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

/// What a scan found, plus what it had to leave out.
#[derive(Debug, Default)]
pub struct ScanOutcome {
    pub records: Vec<FileRecord>,
    /// Folders the OS refused to open (macOS privacy / TCC prompts denied,
    /// external drives without access, NTFS ACLs). The scan carries on.
    pub denied_folders: Vec<PathBuf>,
    /// Cloud-only placeholders skipped instead of being downloaded.
    pub cloud_only_skipped: usize,
}

/// The logged-in user's home / profile folder.
fn user_home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

/// True for "permission denied" / "operation not permitted" (macOS TCC
/// answers EPERM; both map to io::ErrorKind::PermissionDenied).
fn is_permission_denied(e: &walkdir::Error) -> bool {
    e.io_error()
        .map(|io| io.kind() == std::io::ErrorKind::PermissionDenied)
        .unwrap_or(false)
}

/// Scan roots for files matching the requested categories.
/// Emits "scan_progress" events to the frontend as it goes.
/// Folders that cannot be opened are skipped and reported, never fatal.
pub async fn scan_directories(
    app: &AppHandle,
    roots: Vec<PathBuf>,
    allowed_categories: &[FileCategory],
    extra_skip_dirs: &[PathBuf],
) -> Result<ScanOutcome> {
    let mut records: Vec<FileRecord> = Vec::new();
    let mut hash_map: HashMap<String, PathBuf> = HashMap::new(); // sha256 → first_path
    let mut files_seen = 0usize;
    // Cell: also counted inside the walk's filter closure.
    let cloud_only_skipped = std::cell::Cell::new(0usize);
    let mut denied_folders: Vec<PathBuf> = Vec::new();
    // Only the logged-in user's profile is scanned under <drive>\Users
    // (Windows) or /Users (macOS).
    let user_profile: Option<PathBuf> = user_home_dir();
    // Top-level drive folders recognised as installed software, with reason.
    let mut skipped_software: Vec<(PathBuf, &'static str)> = Vec::new();

    info!("Scanner: {} roots, {} allowed categories ({:?})",
        roots.len(), allowed_categories.len(), allowed_categories);

    // Symlinks and junctions are never followed (they point at folders that
    // are scanned at their real location, or at places we must not enter).
    let mut links_skipped: Vec<PathBuf> = Vec::new();

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
            .filter_entry(|e| {
                if e.depth() == 0 {
                    return true;
                }
                // iCloud "Optimise Mac Storage" stub (".smlouva.pdf.icloud"):
                // the real file is not on this Mac. Never download it.
                if is_icloud_stub_name(&e.file_name().to_string_lossy()) {
                    cloud_only_skipped.set(cloud_only_skipped.get() + 1);
                    return false;
                }
                if should_skip(e.path(), extra_skip_dirs) {
                    return false;
                }
                #[cfg(target_os = "macos")]
                if let Some(reason) = mac_skip_reason(e.path(), user_profile.as_deref()) {
                    debug!("Skip ({}): {:?}", reason, e.path());
                    return false;
                }
                #[cfg(target_os = "macos")]
                if e.file_type().is_dir() && is_dataless_dir(e.path()) {
                    // Cloud folder whose contents are not downloaded: listing
                    // it would make the File Provider fetch it.
                    cloud_only_skipped.set(cloud_only_skipped.get() + 1);
                    debug!("Skip (cloud-only folder): {:?}", e.path());
                    return false;
                }
                if e.file_type().is_dir() {
                    if is_other_user_profile(e.path(), user_profile.as_deref()) {
                        debug!("Skip (other user's profile): {:?}", e.path());
                        return false;
                    }
                    if let Some(reason) = app_folder_reason(e.path(), user_profile.as_deref()) {
                        skipped_software.push((e.path().to_path_buf(), reason));
                        return false;
                    }
                }
                true
            })
        {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    if is_permission_denied(&e) {
                        if let Some(p) = e.path() {
                            denied_folders.push(p.to_path_buf());
                        }
                        warn!("No permission, skipped: {}", e);
                    } else {
                        warn!("WalkDir error: {}", e);
                    }
                    continue;
                }
            };

            // Symlink / junction: skipped, never followed.
            if entry.path_is_symlink() {
                if entry.depth() == 1 {
                    links_skipped.push(entry.path().to_path_buf());
                }
                debug!("Skip (symlink/junction): {:?}", entry.path());
                continue;
            }

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
                cloud_only_skipped.set(cloud_only_skipped.get() + 1);
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

                // NFC: macOS stores accented names decomposed (NFD); everything
            // downstream (NER, registries, folder names) expects NFC.
            let name = crate::norm::nfc(
                &path.file_name().unwrap_or_default().to_string_lossy(),
            );

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

    // Tell the user which top-level folders were treated as installed
    // software, so a work folder skipped by mistake is visible.
    for link in &links_skipped {
        info!("Skipped symlink/junction: {:?}", link);
        emit_progress(
            app,
            files_seen,
            &format!("Přeskočeno (odkaz / symlink): {}", link.display()),
            None,
            false,
        );
    }
    for (folder, reason) in &skipped_software {
        info!("Skipped top-level folder {:?}: {}", folder, reason);
        emit_progress(
            app,
            files_seen,
            &format!("Přeskočeno ({}): {}", reason, folder.display()),
            None,
            false,
        );
    }

    let cloud_only_skipped = cloud_only_skipped.get();
    for folder in &denied_folders {
        info!("Skipped folder without permission: {:?}", folder);
    }
    if !denied_folders.is_empty() {
        emit_progress(
            app,
            files_seen,
            &format!("Přeskočeno (bez oprávnění): {} složek", denied_folders.len()),
            None,
            false,
        );
    }

    emit_progress(
        app,
        files_seen,
        &format!("Nalezeno {} souborů", files_seen),
        None,
        true,
    );

    info!(
        "Scan complete: {} files found across {} roots ({} cloud-only placeholders skipped, {} folders without permission)",
        records.len(), roots.len(), cloud_only_skipped, denied_folders.len()
    );
    Ok(ScanOutcome { records, denied_folders, cloud_only_skipped })
}

/// iCloud Drive placeholder written by older macOS versions instead of the
/// file: ".<name>.icloud".
fn is_icloud_stub_name(name: &str) -> bool {
    name.starts_with('.') && name.len() > ".icloud".len() + 1 && name.ends_with(".icloud")
}

/// macOS st_flags bit for "dataless" files and folders: the content lives
/// in the cloud (iCloud Drive, OneDrive / Google Drive / Dropbox via File
/// Provider) and reading it would download it.
#[cfg(target_os = "macos")]
const SF_DATALESS: u32 = 0x4000_0000;

#[cfg(target_os = "macos")]
fn is_dataless_dir(path: &Path) -> bool {
    use std::os::macos::fs::MetadataExt;
    std::fs::symlink_metadata(path)
        .map(|m| m.st_flags() & SF_DATALESS != 0)
        .unwrap_or(false)
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

#[cfg(target_os = "macos")]
fn is_cloud_placeholder(metadata: &std::fs::Metadata) -> bool {
    use std::os::macos::fs::MetadataExt;
    metadata.st_flags() & SF_DATALESS != 0
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
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

/// Default scan roots on macOS: the home folder, plus cloud folders that
/// live inside ~/Library (which is otherwise skipped): File Provider
/// folders in ~/Library/CloudStorage and iCloud Drive.
#[cfg(target_os = "macos")]
pub fn default_scan_roots() -> Vec<PathBuf> {
    match user_home_dir() {
        Some(home) => mac_default_roots(&home),
        None => vec![PathBuf::from("/")],
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn default_scan_roots() -> Vec<PathBuf> {
    vec![user_home_dir().unwrap_or_else(|| PathBuf::from("/"))]
}

// ─────────────────────────────────────────────────────────────────────────────
// macOS layout. Pure path logic, compiled everywhere so it is unit-tested on
// every CI platform; only called on macOS.
// ─────────────────────────────────────────────────────────────────────────────

/// System locations directly on the boot volume. Never user documents.
const MAC_SYSTEM_DIRS: &[&str] = &[
    "/System", "/Applications", "/Library", "/private", "/usr", "/bin", "/sbin",
    "/opt", "/cores", "/dev", "/etc", "/var", "/tmp", "/Network",
];

/// Folders directly in the home folder that hold app data, not documents.
/// (~/Library also holds CloudStorage and iCloud Drive; those are scanned
/// as their own roots.)
const MAC_HOME_SKIP: &[&str] = &["Library", "Applications", ".Trash"];

/// Package / bundle folders: apps, photo and music libraries, Xcode
/// projects, VM images. Finder shows them as single files.
const MAC_BUNDLE_SUFFIXES: &[&str] = &[
    ".app", ".appex", ".bundle", ".framework", ".plugin", ".kext", ".prefpane",
    ".photoslibrary", ".photolibrary", ".migratedphotolibrary", ".aplibrary",
    ".musiclibrary", ".tvlibrary", ".imovielibrary", ".theater", ".fcpbundle",
    ".logicx", ".band", ".xcodeproj", ".xcworkspace", ".xcarchive", ".xcassets",
    ".lrdata", ".lrlibrary", ".vmwarevm", ".pvm", ".utm", ".sparsebundle",
];

/// Why a path is skipped on macOS, or None.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn mac_skip_reason(path: &Path, home: Option<&Path>) -> Option<&'static str> {
    if MAC_SYSTEM_DIRS.iter().any(|d| path == Path::new(d)) {
        return Some("systémová složka macOS");
    }
    if let (Some(home), Some(parent), Some(name)) = (home, path.parent(), path.file_name()) {
        if parent == home && MAC_HOME_SKIP.iter().any(|s| name.to_string_lossy().eq_ignore_ascii_case(s)) {
            return Some("data aplikací");
        }
    }
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    if MAC_BUNDLE_SUFFIXES.iter().any(|s| name.ends_with(s)) {
        return Some("balíček aplikace / knihovna");
    }
    None
}

/// Cloud folders inside ~/Library that macOS sync apps use, with a display
/// name: ~/Library/CloudStorage/<Provider-Account> and iCloud Drive.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn mac_cloud_dirs(home: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let storage = home.join("Library").join("CloudStorage");
    if let Ok(rd) = std::fs::read_dir(&storage) {
        let mut dirs: Vec<PathBuf> = rd
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.path())
            .collect();
        dirs.sort();
        for dir in dirs {
            let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if name.starts_with('.') {
                continue;
            }
            out.push((mac_cloud_display_name(&name), dir));
        }
    }
    let icloud = home.join("Library").join("Mobile Documents").join("com~apple~CloudDocs");
    if icloud.is_dir() {
        out.push(("iCloud Drive".to_string(), icloud));
    }
    out
}

/// "OneDrive-Personal" → "OneDrive", "OneDrive-Firma" → "OneDrive - Firma",
/// "GoogleDrive-jana@x.cz" → "Google Drive (jana@x.cz)", "Dropbox" → "Dropbox".
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn mac_cloud_display_name(dir_name: &str) -> String {
    let name = crate::norm::nfc(dir_name);
    if let Some(rest) = name.strip_prefix("OneDrive") {
        let rest = rest.trim_start_matches('-');
        return if rest.is_empty() || rest.eq_ignore_ascii_case("Personal") {
            "OneDrive".to_string()
        } else {
            format!("OneDrive - {}", rest)
        };
    }
    if let Some(rest) = name.strip_prefix("GoogleDrive") {
        let rest = rest.trim_start_matches('-');
        return if rest.is_empty() { "Google Drive".to_string() } else { format!("Google Drive ({})", rest) };
    }
    if name.starts_with("Dropbox") {
        return "Dropbox".to_string();
    }
    if let Some(rest) = name.strip_prefix("Box") {
        if rest.is_empty() || rest.starts_with('-') {
            return "Box".to_string();
        }
    }
    name
}

/// Home + cloud folders, as scanned by default on macOS.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn mac_default_roots(home: &Path) -> Vec<PathBuf> {
    let mut roots = vec![home.to_path_buf()];
    roots.extend(mac_cloud_dirs(home).into_iter().map(|(_, p)| p));
    roots
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

/// `<drive>\\Users\\<name>` that is not the logged-in user's profile
/// (other accounts, Public, Default). Case-insensitive.
fn is_other_user_profile(path: &Path, current_profile: Option<&Path>) -> bool {
    let parent = match path.parent() {
        Some(p) => p,
        None => return false,
    };
    let parent_is_users = parent
        .file_name()
        .map(|n| n.to_string_lossy().eq_ignore_ascii_case("Users"))
        .unwrap_or(false);
    // "Users" must sit directly on the drive root.
    let users_on_drive_root = parent.parent().map(|g| g.parent().is_none()).unwrap_or(false);
    if !(parent_is_users && users_on_drive_root) {
        return false;
    }
    match current_profile {
        Some(profile) => {
            path.to_string_lossy().to_lowercase() != profile.to_string_lossy().to_lowercase()
        }
        // Unknown profile: do not guess, scan everything.
        None => false,
    }
}

/// Profile folders that are always walked (only their subfolders are
/// judged): judging e.g. Documents as a whole could skip all of it because
/// of a few code projects inside.
const USER_CONTAINERS: &[&str] = &[
    "Documents", "Desktop", "Downloads", "Pictures", "Music", "Videos",
    "OneDrive", "Dropbox", "Google Drive", "iCloudDrive", "Box",
];

/// Antivirus / security vendors: their folders hold quarantine, sandboxes
/// and logs, never user documents. Matched as words in the folder name.
const AV_VENDORS: &[&str] = &[
    "avg", "avast", "eset", "norton", "symantec", "mcafee", "kaspersky",
    "bitdefender", "avira", "sophos", "malwarebytes", "webroot", "comodo",
    "drweb", "gdata", "fsecure", "trendmicro", "emsisoft", "zonealarm",
    "quickheal", "bullguard", "totalav", "panda", "defender",
];

fn is_av_vendor_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    let squashed: String = lower.chars().filter(|c| c.is_alphanumeric()).collect();
    // "F-Secure" → "fsecure", "Trend Micro" → "trendmicro"
    if ["fsecure", "trendmicro", "gdata", "drweb"].iter().any(|v| squashed.contains(v)) {
        return true;
    }
    lower
        .split(|c: char| !c.is_alphanumeric())
        .any(|tok| AV_VENDORS.contains(&tok))
}

/// Is `path` at a level where applications are usually installed or
/// unpacked: directly on a drive, directly in the user profile, or one
/// level inside a profile folder (Documents\\Zoom, C:\\Users\\x\\anaconda3, …)?
fn is_app_check_level(path: &Path, user_profile: Option<&Path>) -> bool {
    let parent = match path.parent() {
        Some(p) => p,
        None => return false,
    };
    if parent.parent().is_none() {
        // Directly on a drive root; Users is handled per profile.
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        return !name.eq_ignore_ascii_case("Users");
    }
    let Some(profile) = user_profile else { return false };
    let same = |a: &Path, b: &Path| a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy());
    if same(parent, profile) {
        // C:\\Users\\x\\<name>: well-known user folders are always walked.
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        return !USER_CONTAINERS.iter().any(|c| name.eq_ignore_ascii_case(c) || name.starts_with("OneDrive"));
    }
    // C:\\Users\\x\\Documents\\<name>
    parent.parent().map(|g| same(g, profile)).unwrap_or(false)
}

/// Why a folder is treated as system, antivirus or application content and
/// skipped, or None if it is scanned. Only applied at app-check levels.
fn app_folder_reason(path: &Path, user_profile: Option<&Path>) -> Option<&'static str> {
    if !is_app_check_level(path, user_profile) {
        return None;
    }
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let at_drive_root = path.parent().map(|p| p.parent().is_none()).unwrap_or(false);
    if at_drive_root && name.starts_with('$') {
        return Some("systémová složka");
    }
    if is_av_vendor_name(&name) {
        return Some("antivirus");
    }
    if at_drive_root && folder_owner_sid(path).map(|sid| is_system_sid(&sid)).unwrap_or(false) {
        return Some("vlastník systém");
    }
    sample_folder(path).verdict()
}

/// SYSTEM, LocalService, NetworkService and NT SERVICE\\* (TrustedInstaller).
/// BUILTIN\\Administrators is deliberately NOT included: folders a person
/// creates with an elevated tool or `mklink` get that owner too.
fn is_system_sid(sid: &str) -> bool {
    matches!(sid, "S-1-5-18" | "S-1-5-19" | "S-1-5-20") || sid.starts_with("S-1-5-80-")
}

/// Extensions that belong to programs, code and configuration.
const APP_EXTENSIONS: &[&str] = &[
    // binaries / installers
    "exe", "dll", "sys", "msi", "msix", "appx", "cab", "ocx", "drv", "jar", "class",
    "pyc", "pyd", "so", "dylib", "lib", "o", "obj", "pdb", "node", "wasm", "pak", "asar",
    // config / metadata
    "json", "md", "yml", "yaml", "toml", "ini", "cfg", "conf", "config", "lock",
    "xaml", "manifest", "plist", "properties",
    // source code / web assets
    "js", "mjs", "cjs", "ts", "tsx", "jsx", "css", "scss", "less", "map", "html", "htm",
    "vue", "svelte", "py", "rs", "go", "java", "kt", "cs", "cpp", "cc", "c", "h", "hpp",
    "swift", "rb", "php", "sh", "ps1", "psm1", "bat", "cmd", "lua", "sql",
];

/// Files or folders that mark a code project.
const PROJECT_MARKERS: &[&str] = &[
    ".git", "node_modules", "package.json", "Cargo.toml", "pyproject.toml",
    "requirements.txt", "go.mod", "pom.xml", "build.gradle",
]; // plus any *.sln (Visual Studio solution)

/// Sample of a folder's contents (a few levels, capped) used to tell
/// applications and code apart from document folders.
#[derive(Debug, Default)]
struct FolderSample {
    app_files: usize,
    binaries: usize,
    documents: usize,
    project_marker: bool,
    /// Direct subfolders named like an install layout (bin, lib, src, …).
    layout_dirs: usize,
}

/// Subfolder names typical of installed or unpacked software.
const APP_LAYOUT_DIRS: &[&str] = &[
    "bin", "lib", "lib64", "include", "src", "sources", "share", "plugins", "runtime",
];

impl FolderSample {
    fn verdict(&self) -> Option<&'static str> {
        if self.project_marker {
            return Some("vývojový projekt");
        }
        if self.binaries > 0 && self.documents == 0 {
            return Some("nainstalovaný program");
        }
        if self.layout_dirs >= 2 || (self.layout_dirs == 1 && self.documents == 0) {
            return Some("struktura programu (bin/lib/src)");
        }
        if self.app_files >= 10 && self.app_files >= 5 * self.documents.max(1) {
            return Some("aplikace / konfigurace");
        }
        None
    }
}

const SAMPLE_DEPTH: usize = 3;
const SAMPLE_MAX_ENTRIES: usize = 3000;

fn sample_folder(path: &Path) -> FolderSample {
    let mut s = FolderSample::default();
    for e in WalkDir::new(path)
        .min_depth(1)
        .max_depth(SAMPLE_DEPTH)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .take(SAMPLE_MAX_ENTRIES)
    {
        let name = e.file_name().to_string_lossy();
        let is_marker = PROJECT_MARKERS.iter().any(|m| name.eq_ignore_ascii_case(m))
            || name.to_lowercase().ends_with(".sln");
        if is_marker {
            s.project_marker = true;
            break;
        }
        if !e.file_type().is_file() {
            if e.depth() == 1
                && e.file_type().is_dir()
                && APP_LAYOUT_DIRS.iter().any(|d| name.eq_ignore_ascii_case(d))
            {
                s.layout_dirs += 1;
            }
            continue;
        }
        let ext = e.path().extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
        if matches!(ext.as_str(), "exe" | "dll" | "msi" | "sys") {
            s.binaries += 1;
        }
        if APP_EXTENSIONS.contains(&ext.as_str()) {
            s.app_files += 1;
        } else if matches!(
            FileCategory::from_extension(&ext),
            FileCategory::Pdf | FileCategory::Word | FileCategory::Excel
                | FileCategory::PowerPoint | FileCategory::Email | FileCategory::Csv
        ) || ext == "rtf"
        {
            s.documents += 1;
        }
    }
    s
}




/// Owner SID of a file or folder as a string ("S-1-5-21-…").
#[cfg(target_os = "windows")]
fn folder_owner_sid(path: &Path) -> Option<String> {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "advapi32")]
    extern "system" {
        fn GetNamedSecurityInfoW(
            object_name: *const u16,
            object_type: u32,
            security_info: u32,
            owner: *mut *mut c_void,
            group: *mut *mut c_void,
            dacl: *mut *mut c_void,
            sacl: *mut *mut c_void,
            descriptor: *mut *mut c_void,
        ) -> u32;
        fn ConvertSidToStringSidW(sid: *mut c_void, string_sid: *mut *mut u16) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn LocalFree(mem: *mut c_void) -> *mut c_void;
    }
    const SE_FILE_OBJECT: u32 = 1;
    const OWNER_SECURITY_INFORMATION: u32 = 1;

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut owner: *mut c_void = std::ptr::null_mut();
    let mut descriptor: *mut c_void = std::ptr::null_mut();
    // SAFETY: valid NUL-terminated path; out-pointers are valid; the
    // descriptor (which owns `owner`) is freed with LocalFree below.
    unsafe {
        let rc = GetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut owner,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut descriptor,
        );
        if rc != 0 || owner.is_null() {
            if !descriptor.is_null() {
                LocalFree(descriptor);
            }
            return None;
        }
        let mut sid_str: *mut u16 = std::ptr::null_mut();
        let ok = ConvertSidToStringSidW(owner, &mut sid_str) != 0 && !sid_str.is_null();
        let result = if ok {
            let mut len = 0usize;
            while *sid_str.add(len) != 0 {
                len += 1;
            }
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(sid_str, len));
            LocalFree(sid_str as *mut c_void);
            Some(s)
        } else {
            None
        };
        LocalFree(descriptor);
        result
    }
}

#[cfg(not(target_os = "windows"))]
fn folder_owner_sid(_path: &Path) -> Option<String> {
    None
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
/// local sync directories (Windows profile folders, macOS CloudStorage).
pub fn detect_cloud_clients() -> Vec<crate::types::DetectedCloud> {
    // Only Windows and macOS push entries; elsewhere the list stays empty.
    #[cfg_attr(not(any(target_os = "windows", target_os = "macos")), allow(unused_mut))]
    let mut clouds = Vec::new();

    #[cfg(target_os = "macos")]
    if let Some(home) = user_home_dir() {
        let mut found = mac_cloud_dirs(&home);
        // Older Dropbox / Google Drive installs sync straight into the home
        // folder (a symlink to CloudStorage is not a separate folder).
        for (name, dir) in [("Dropbox", "Dropbox"), ("Google Drive", "Google Drive")] {
            let p = home.join(dir);
            let real_dir = std::fs::symlink_metadata(&p).map(|m| m.is_dir()).unwrap_or(false);
            if real_dir {
                found.push((name.to_string(), p));
            }
        }
        for (name, root_path) in found {
            let is_running = is_sync_client_running(&name);
            clouds.push(crate::types::DetectedCloud { name, root_path, is_running });
        }
    }

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

/// macOS: `pgrep -x <process>` (exit status 0 = running).
#[cfg(target_os = "macos")]
fn is_sync_client_running(cloud_name: &str) -> bool {
    let process_name = match cloud_name {
        n if n.contains("OneDrive") => "OneDrive",
        n if n.contains("Dropbox") => "Dropbox",
        n if n.contains("Google Drive") => "Google Drive",
        n if n.starts_with("Box") => "Box",
        // iCloud Drive is synced by the system daemon "bird".
        n if n.contains("iCloud") => "bird",
        _ => return false,
    };
    std::process::Command::new("pgrep")
        .args(["-x", process_name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
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
    fn only_current_user_profile_is_scanned() {
        use super::is_other_user_profile;
        let me = Path::new("/Users/jaros");
        assert!(!is_other_user_profile(Path::new("/Users/jaros"), Some(me)));
        assert!(!is_other_user_profile(Path::new("/Users/JAROS"), Some(me)));
        assert!(is_other_user_profile(Path::new("/Users/Public"), Some(me)));
        assert!(is_other_user_profile(Path::new("/Users/Default"), Some(me)));
        // Not directly on the drive root → unaffected
        assert!(!is_other_user_profile(Path::new("/Data/Users/Klient"), Some(me)));
        // Unknown profile → scan everything
        assert!(!is_other_user_profile(Path::new("/Users/Public"), None));
    }

    #[test]
    fn system_owner_sids() {
        use super::is_system_sid;
        assert!(is_system_sid("S-1-5-18"));
        // Administrators: also the owner of folders made with mklink / elevated tools
        assert!(!is_system_sid("S-1-5-32-544"));
        assert!(is_system_sid("S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464"));
        assert!(!is_system_sid("S-1-5-21-1004336348-1177238915-682003330-1001"));
    }

    fn sample(dir: &Path, files: &[&str]) -> &'static str {
        for f in files {
            let p = dir.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, b"x").unwrap();
        }
        super::sample_folder(dir).verdict().unwrap_or("scan")
    }

    #[test]
    fn app_folder_detection() {
        let base = std::env::temp_dir().join(format!("fileit-app-{}", uuid::Uuid::new_v4()));
        // Program nested two levels deep (C:\\ffmpeg\\ffmpeg-8.1.2-full_build\\bin)
        assert_eq!(sample(&base.join("ffmpeg"), &["ffmpeg-8.1.2-full_build/bin/ffmpeg.exe", "ffmpeg-8.1.2-full_build/README.txt"]), "nainstalovaný program");
        // Code project
        assert_eq!(sample(&base.join("_dev"), &["fileIT/package.json", "fileIT/src/a.ts"]), "vývojový projekt");
        // Config-heavy app data
        let many: Vec<String> = (0..12).map(|i| format!("models/m{}.json", i)).collect();
        let refs: Vec<&str> = many.iter().map(|s| s.as_str()).collect();
        assert_eq!(sample(&base.join("pinokio"), &refs), "aplikace / konfigurace");
        // Work folder with a stray installer and documents → scanned
        assert_eq!(sample(&base.join("temp"), &["setup.exe", "smlouva.pdf", "Klient/faktura.pdf"]), "scan");
        // Documents with a few json exports → scanned
        assert_eq!(sample(&base.join("data"), &["a.json", "b.json", "x.pdf", "y.docx", "z.xlsx"]), "scan");
        // Install layout without binaries in the sample
        assert_eq!(sample(&base.join("tool"), &["bin/readme.txt", "lib/x.txt", "share/y.txt"]), "struktura programu (bin/lib/src)");
        // One "lib" folder next to client documents → scanned
        assert_eq!(sample(&base.join("klient"), &["lib/notes.txt", "smlouva.pdf"]), "scan");
        // Music only → scanned (nothing to read, harmless)
        assert_eq!(sample(&base.join("music"), &["t.mp3"]), "scan");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn antivirus_names() {
        use super::is_av_vendor_name;
        for n in ["AVG sandbox", "$AV_AVG", "ESET", "Norton Security", "F-Secure", "Trend Micro", "Kaspersky Lab"] {
            assert!(is_av_vendor_name(n), "{}", n);
        }
        for n in ["Klienti", "Avgust", "data", "Pandora smlouvy"] {
            assert!(!is_av_vendor_name(n), "{}", n);
        }
    }

    #[test]
    fn app_check_levels() {
        use super::is_app_check_level;
        let me = Path::new("/Users/jaros");
        assert!(is_app_check_level(Path::new("/ffmpeg"), Some(me)));
        assert!(!is_app_check_level(Path::new("/Users"), Some(me)));
        assert!(!is_app_check_level(Path::new("/Users/jaros/Documents"), Some(me)));
        assert!(!is_app_check_level(Path::new("/Users/jaros/OneDrive - Firma"), Some(me)));
        assert!(is_app_check_level(Path::new("/Users/jaros/anaconda3"), Some(me)));
        assert!(is_app_check_level(Path::new("/Users/jaros/Documents/Zoom"), Some(me)));
        assert!(!is_app_check_level(Path::new("/Users/jaros/Documents/Klienti/2024"), Some(me)));
        assert!(!is_app_check_level(Path::new("/data/Klienti"), Some(me)));
    }

    #[test]
    fn mac_skip_rules() {
        use super::mac_skip_reason;
        let home = Path::new("/Users/jana");
        for p in [
            "/System", "/Applications", "/private", "/Library",
            "/Users/jana/Library", "/Users/jana/Applications",
            "/Users/jana/Documents/Nástroje/Kalkulačka.app",
            "/Users/jana/Pictures/Photos Library.photoslibrary",
            "/Users/jana/Music/Music Library.musiclibrary",
            "/Users/jana/Projects/App.xcodeproj",
        ] {
            assert!(mac_skip_reason(Path::new(p), Some(home)).is_some(), "{}", p);
        }
        for p in [
            "/Users/jana/Documents", "/Users/jana/Desktop/Klienti",
            "/Users/jana/Documents/Library",          // only ~/Library is app data
            "/Users/jana/Library/CloudStorage/OneDrive-Personal/Smlouvy", // inside a cloud root
            "/Users/jana/Documents/Smlouvy/Applications", // not the home folder
            "/Volumes/USB/Klienti",
            "/Users/jana/Documents/smlouva.pdf",
        ] {
            assert!(mac_skip_reason(Path::new(p), Some(home)).is_none(), "{}", p);
        }
    }

    #[test]
    fn mac_cloud_names() {
        use super::mac_cloud_display_name as n;
        assert_eq!(n("OneDrive-Personal"), "OneDrive");
        assert_eq!(n("OneDrive-Firma s.r.o"), "OneDrive - Firma s.r.o");
        assert_eq!(n("GoogleDrive-jana@example.cz"), "Google Drive (jana@example.cz)");
        assert_eq!(n("Dropbox"), "Dropbox");
        assert_eq!(n("Dropbox-Osobni"), "Dropbox");
        assert_eq!(n("Box-Box"), "Box");
        assert_eq!(n("pCloud"), "pCloud");
    }

    #[test]
    fn mac_default_roots_include_cloud_folders() {
        let home = std::env::temp_dir().join(format!("fileit-home-{}", uuid::Uuid::new_v4()));
        let storage = home.join("Library/CloudStorage");
        for d in ["OneDrive-Personal", "GoogleDrive-jana@example.cz", ".hidden"] {
            std::fs::create_dir_all(storage.join(d)).unwrap();
        }
        let icloud = home.join("Library/Mobile Documents/com~apple~CloudDocs");
        std::fs::create_dir_all(&icloud).unwrap();
        let roots = super::mac_default_roots(&home);
        assert_eq!(roots[0], home);
        assert!(roots.contains(&storage.join("OneDrive-Personal")));
        assert!(roots.contains(&storage.join("GoogleDrive-jana@example.cz")));
        assert!(roots.contains(&icloud));
        assert!(!roots.iter().any(|r| r.ends_with(".hidden")));
        assert_eq!(roots.len(), 4);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn icloud_stub_names() {
        use super::is_icloud_stub_name;
        assert!(is_icloud_stub_name(".smlouva Dvořák.pdf.icloud"));
        assert!(!is_icloud_stub_name("smlouva.icloud"));
        assert!(!is_icloud_stub_name(".icloud"));
        assert!(!is_icloud_stub_name("smlouva.pdf"));
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
