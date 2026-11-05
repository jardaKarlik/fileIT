// src-tauri/src/restructure/mod.rs
//
// FileIT restructure engine.
//
// Two phases:
//   1. preview()  — computes PlannedMove list from classified files + structure
//                   WITHOUT touching the filesystem
//   2. execute()  — performs the fs::rename moves, creates folders as needed,
//                   emits progress events, records a manifest for restore

use crate::types::*;
use anyhow::{Context, Result};
use chrono::Utc;
use log::{debug, info, warn};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// Preview
// ─────────────────────────────────────────────────────────────────────────────

/// Build a RestructurePreview without touching the filesystem.
/// Called after the user finishes configuring dimensions and destination.
pub fn preview(
    files: &[ClassifiedFile],
    structure: &[GroupDimension],
    target_root: &Path,
) -> Result<RestructurePreview> {
    let mut planned_moves: Vec<PlannedMove> = Vec::new();
    let mut folder_set: HashSet<PathBuf> = HashSet::new();
    let mut subfolder_set: HashSet<PathBuf> = HashSet::new();
    let mut unknown_count = 0usize;
    let mut duplicate_count = 0usize;
    // Destinations already claimed by earlier files in this plan. Without
    // this, two "faktura.pdf" from different folders get the same target and
    // the second move overwrites the first. Lower-cased: NTFS is
    // case-insensitive.
    let mut claimed: HashSet<String> = HashSet::new();

    for file in files {
        if file.is_duplicate {
            duplicate_count += 1;
            // Duplicates still get moved but into a _Duplicates subfolder
        }

        // Build the relative path from the grouping structure
        let mut rel_parts: Vec<String> = Vec::new();
        for (depth, dim) in structure.iter().enumerate() {
            let key = file.group_key(dim);
            if key == "Neznámý klient" || key == "Bez instituce"
                || key == "Neurčený typ" || key == "Neurčeno"
            {
                if depth == 0 {
                    unknown_count += 1;
                }
            }
            rel_parts.push(sanitise_folder_name(&key));
        }

        if rel_parts.is_empty() {
            rel_parts.push("Neroztříděné".to_string());
        }

        // Build target folder path
        let mut folder_path = target_root.to_path_buf();
        for (i, part) in rel_parts.iter().enumerate() {
            folder_path = folder_path.join(part);
            if i == 0 {
                folder_set.insert(folder_path.clone());
            } else {
                subfolder_set.insert(folder_path.clone());
            }
        }

        // Handle filename collisions — append _2, _3 etc.
        // A file already at its target (re-running over an organised folder)
        // keeps its name instead of being renamed to _2.
        let natural = folder_path.join(&file.record.name);
        let dest_file = if same_path(&natural, &file.record.path)
            && claimed.insert(natural.to_string_lossy().to_lowercase())
        {
            natural
        } else {
            unique_filename(&folder_path, &file.record.name, &mut claimed)
        };

        planned_moves.push(PlannedMove {
            source: file.record.path.clone(),
            destination: dest_file,
            file_name: file.record.name.clone(),
        });
    }

    // Check if target already has existing files
    let (target_has_existing_files, existing_file_count) =
        check_target_existing(target_root);

    Ok(RestructurePreview {
        total_files: files.len(),
        total_folders: folder_set.len(),
        total_subfolders: subfolder_set.len(),
        unknown_count,
        duplicate_count,
        target_path: target_root.to_path_buf(),
        planned_moves,
        target_has_existing_files,
        existing_file_count,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Execute
// ─────────────────────────────────────────────────────────────────────────────

/// Execute the planned moves. Emits "restructure_progress" events.
/// Returns (manifest_entries, folders_created, duration_seconds, errors).
pub async fn execute(
    app: &AppHandle,
    planned_moves: &[PlannedMove],
    session_id: &str,
    app_data_dir: &Path,
) -> Result<(Vec<ManifestEntry>, usize, u64, Vec<FileError>)> {
    let start = std::time::Instant::now();
    let total = planned_moves.len();
    let mut manifest_entries: Vec<ManifestEntry> = Vec::new();
    let mut folders_created = 0usize;
    let mut moved = 0usize;
    let mut errors: Vec<FileError> = Vec::new();

    info!("Starting restructure: {} files to move", total);
    let mut journal = Journal::start(app_data_dir, session_id);

    for (idx, plan) in planned_moves.iter().enumerate() {
        let _pct = ((idx as f32 / total as f32) * 100.0) as u32;
        let log_line = format!(
            "→ Přesouváme: {}",
            plan.source.file_name()
                .unwrap_or_default()
                .to_string_lossy()
        );

        emit_progress(app, moved, total, &plan.file_name, &log_line, false, None);

        // Create destination directory if needed. A failure here is a
        // per-file error: aborting would lose the manifest of the files
        // already moved, and with it the ability to restore them.
        let mut dir_error: Option<String> = None;
        if let Some(parent) = plan.destination.parent() {
            if !parent.exists() {
                match std::fs::create_dir_all(parent) {
                    Ok(()) => {
                        folders_created += 1;
                        debug!("Created folder: {:?}", parent);
                    }
                    Err(e) => dir_error = Some(format!("Creating directory {:?}: {}", parent, e)),
                }
            }
        }

        // Perform the move (never overwrites an existing file)
        let outcome = match dir_error {
            Some(e) => Err(anyhow::anyhow!(e)),
            None => move_file_no_overwrite(&plan.source, &plan.destination),
        };
        match outcome {
            Ok((sha256, actual_dest)) => {
                let entry = ManifestEntry {
                    original_path: plan.source.clone(),
                    new_path: actual_dest,
                    sha256,
                };
                journal.record(&entry);
                manifest_entries.push(entry);
                moved += 1;
                debug!("Moved {}/{}: {:?}", moved, total, plan.file_name);
            }
            Err(e) => {
                let error_msg = e.to_string();
                warn!("Failed to move {:?}: {}", plan.source, error_msg);

                let file_error = FileError {
                    timestamp: Utc::now().to_rfc3339(),
                    operation: "file_move".to_string(),
                    source: plan.source.to_string_lossy().to_string(),
                    destination: plan.destination.to_string_lossy().to_string(),
                    error_message: error_msg.clone(),
                };
                errors.push(file_error);

                emit_progress(
                    app, moved, total, &plan.file_name,
                    &format!("⚠ Chyba: {} — přeskakujeme", plan.file_name),
                    false,
                    Some(error_msg),
                );
            }
        }
    }

    let duration = start.elapsed().as_secs();
    emit_progress(app, moved, total, "", "✓ Uspořádání dokončeno", true, None);
    info!("Restructure complete: {} moved, {} errors in {}s", moved, errors.len(), duration);

    // Write structured error log if any errors occurred
    if !errors.is_empty() {
        write_error_log(app_data_dir, session_id, &errors);
    }

    Ok((manifest_entries, folders_created, duration, errors))
}

/// Write structured error log to AppData for audit trail.
fn write_error_log(app_data_dir: &Path, session_id: &str, errors: &[FileError]) {
    let log_dir = app_data_dir.join("error_logs");
    if std::fs::create_dir_all(&log_dir).is_err() {
        return;
    }
    let filename = format!("errors_{}.json", &session_id[..session_id.len().min(36)]);
    let path = log_dir.join(filename);
    if let Ok(json) = serde_json::to_string_pretty(errors) {
        let _ = std::fs::write(&path, json);
        warn!("[restructure] {} file error(s) logged to {:?}", errors.len(), path);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Restore
// ─────────────────────────────────────────────────────────────────────────────

/// Restore all files to their original locations using the manifest.
/// This is the "undo" operation triggered from the Confirmation screen.
pub async fn restore(
    app: &AppHandle,
    manifest: &BackupManifest,
) -> Result<usize> {
    let total = manifest.moves.len();
    let mut restored = 0usize;

    info!("Starting restore: {} files to restore", total);

    for (_idx, entry) in manifest.moves.iter().enumerate() {
        let log_line = format!(
            "↶ Obnovujeme: {}",
            entry.original_path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
        );

        emit_restore_progress(app, restored, total, &log_line, false);

        // Re-create original directory if it no longer exists
        if let Some(parent) = entry.original_path.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    warn!("Cannot recreate {:?} for restore: {}", parent, e);
                    continue;
                }
            }
        }

        // Move back from new_path → original_path. If a different file now
        // sits at the original path, keep both (restored copy gets _2).
        match move_file_no_overwrite(&entry.new_path, &entry.original_path) {
            Ok((_, actual)) => {
                if actual != entry.original_path {
                    warn!("Original path occupied; restored {:?} as {:?}", entry.original_path, actual);
                }
                restored += 1;
            }
            Err(e) => {
                warn!("Failed to restore {:?}: {}", entry.new_path, e);
            }
        }

        // Clean up empty destination directories
        if let Some(parent) = entry.new_path.parent() {
            let _ = remove_if_empty(parent);
        }
    }

    emit_restore_progress(app, restored, total, "✓ Obnova dokončena", true);
    info!("Restore complete: {} files restored", restored);

    Ok(restored)
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Move `src` to `dst`, or to `dst_2`, `dst_3`… if `dst` already exists.
/// std::fs::rename and fs::copy both silently replace an existing file on
/// Windows, so the free name is chosen here first.
/// Returns (hash_before, actual_destination).
fn move_file_no_overwrite(src: &Path, dst: &Path) -> Result<(String, PathBuf)> {
    if same_path(src, dst) {
        // Already in place — nothing to move.
        let hash = crate::scanner::compute_hash_for_manifest(src).unwrap_or_else(|| "unknown".to_string());
        return Ok((hash, dst.to_path_buf()));
    }
    let actual = if dst.exists() {
        let folder = dst.parent().unwrap_or_else(|| Path::new("."));
        let name = dst.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        unique_filename(folder, &name, &mut HashSet::new())
    } else {
        dst.to_path_buf()
    };
    let hash = move_file(src, &actual)?;
    Ok((hash, actual))
}

/// Move a file, computing its SHA-256 digest before and after moving for verification.
/// Uses fs::rename for same-filesystem moves (fast, atomic on most OSes).
/// Falls back to copy+delete for cross-filesystem moves.
/// Callers must ensure `dst` does not exist (see move_file_no_overwrite).
fn move_file(src: &Path, dst: &Path) -> Result<String> {
    // Compute hash before moving (source is still readable here)
    let hash_before = crate::scanner::compute_hash_for_manifest(src)
        .unwrap_or_else(|| "unknown".to_string());

    // Try rename first (fast path)
    if std::fs::rename(src, dst).is_ok() {
        // Verify destination file exists after rename
        if !dst.exists() {
            anyhow::bail!("File move verification failed: destination {:?} does not exist after rename", dst);
        }
        debug!("Move verified via rename: {:?} → {:?}", src, dst);
        return Ok(hash_before);
    }

    // Cross-filesystem fallback: copy then delete
    std::fs::copy(src, dst)
        .with_context(|| format!("Copying {:?} to {:?}", src, dst))?;

    // Verify destination file before deleting source
    if !dst.exists() {
        anyhow::bail!("File move verification failed: destination {:?} does not exist after copy", dst);
    }

    // Optional: verify file size matches (quick sanity check)
    let src_meta = std::fs::metadata(src)?;
    let dst_meta = std::fs::metadata(dst)?;
    if src_meta.len() != dst_meta.len() {
        // Never delete the original when the copy is incomplete.
        let _ = std::fs::remove_file(dst);
        anyhow::bail!(
            "File size mismatch after copy: {} → {} bytes for {:?}; original kept",
            src_meta.len(),
            dst_meta.len(),
            dst
        );
    }

    std::fs::remove_file(src)
        .with_context(|| format!("Removing source {:?} after verified copy", src))?;

    debug!("Move verified via copy+delete: {:?} → {:?}", src, dst);
    Ok(hash_before)
}

/// Sanitise a string for use as a folder name on Windows.
/// Removes characters forbidden in NTFS paths and control characters (\n, \r, \t, etc.).
fn sanitise_folder_name(name: &str) -> String {
    let forbidden: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let sanitised: String = name
        .chars()
        .filter(|c| !c.is_control())  // Strip control characters (\n, \r, \t, etc.)
        .map(|c| if forbidden.contains(&c) { '_' } else { c })
        .collect();

    // Limit to 80 characters. Count chars, not bytes: byte slicing panics
    // in the middle of a Czech letter (č, ř, ů… are 2 bytes in UTF-8).
    let limited: String = sanitised.trim().chars().take(80).collect();

    // Trim trailing dots and spaces (Windows disallows them)
    let trimmed = limited.trim_end_matches(|c| c == '.' || c == ' ').to_string();

    if trimmed.is_empty() {
        return "Neurčeno".to_string();
    }
    // Reserved device names cannot be folder names on Windows.
    let upper = trimmed.to_ascii_uppercase();
    let stem = upper.split('.').next().unwrap_or("");
    let reserved = matches!(stem, "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit());
    if reserved {
        return format!("{}_", trimmed);
    }
    trimmed
}

/// Case-insensitive path equality (NTFS semantics).
fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

// ─────────────────────────────────────────────────────────────────────────────
// Crash-safe journal: every completed move is appended immediately, so a
// crash or power loss mid-run still leaves a record the app can restore from
// (see recover_interrupted_run). Removed once the CompletedRun is saved.
// ─────────────────────────────────────────────────────────────────────────────

pub fn journal_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("state").join("restructure_journal.jsonl")
}

struct Journal {
    file: Option<std::fs::File>,
}

impl Journal {
    fn start(app_data_dir: &Path, session_id: &str) -> Self {
        use std::io::Write;
        let path = journal_path(app_data_dir);
        let file = path
            .parent()
            .map(std::fs::create_dir_all)
            .transpose()
            .ok()
            .and_then(|_| std::fs::File::create(&path).ok())
            .and_then(|mut f| {
                let header = serde_json::json!({ "session_id": session_id });
                writeln!(f, "{}", header).ok()?;
                Some(f)
            });
        if file.is_none() {
            warn!("[restructure] Cannot create journal {:?}; crash recovery unavailable", path);
        }
        Self { file }
    }

    fn record(&mut self, entry: &ManifestEntry) {
        use std::io::Write;
        if let Some(f) = self.file.as_mut() {
            if let Ok(line) = serde_json::to_string(entry) {
                let _ = writeln!(f, "{}", line);
                let _ = f.sync_data();
            }
        }
    }
}

/// Read an interrupted run's journal: (session_id, completed moves).
pub fn read_journal(app_data_dir: &Path) -> Option<(String, Vec<ManifestEntry>)> {
    let raw = std::fs::read_to_string(journal_path(app_data_dir)).ok()?;
    let mut lines = raw.lines();
    let header: serde_json::Value = serde_json::from_str(lines.next()?).ok()?;
    let session_id = header.get("session_id")?.as_str()?.to_string();
    // A torn last line (crash during write) is simply ignored.
    let moves: Vec<ManifestEntry> = lines.filter_map(|l| serde_json::from_str(l).ok()).collect();
    Some((session_id, moves))
}

pub fn delete_journal(app_data_dir: &Path) {
    let _ = std::fs::remove_file(journal_path(app_data_dir));
}

/// Generate a unique destination filename, appending _2, _3, ... if needed.
/// A name is taken if it exists on disk or is already in `claimed` (another
/// file in the same plan); the chosen name is added to `claimed`.
fn unique_filename(folder: &Path, filename: &str, claimed: &mut HashSet<String>) -> PathBuf {
    let mut is_free = |p: &Path| {
        let key = p.to_string_lossy().to_lowercase();
        if p.exists() || claimed.contains(&key) {
            false
        } else {
            claimed.insert(key);
            true
        }
    };

    let path = folder.join(filename);
    if is_free(&path) {
        return path;
    }

    let stem = Path::new(filename)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let ext = Path::new(filename)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    for i in 2..=999 {
        let new_name = format!("{}_{}{}", stem, i, ext);
        let new_path = folder.join(&new_name);
        if is_free(&new_path) {
            return new_path;
        }
    }

    // Fallback: append UUID
    folder.join(format!("{}_{}{}", stem, Uuid::new_v4(), ext))
}

/// Check if the target directory already has files in it.
fn check_target_existing(target: &Path) -> (bool, usize) {
    if !target.exists() {
        return (false, 0);
    }

    let count = walkdir::WalkDir::new(target)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();

    (count > 0, count)
}

/// Remove a directory if it is empty.
fn remove_if_empty(dir: &Path) -> Result<()> {
    if dir.is_dir() {
        let is_empty = std::fs::read_dir(dir)?.next().is_none();
        if is_empty {
            std::fs::remove_dir(dir)?;
        }
    }
    Ok(())
}

fn emit_progress(
    app: &AppHandle,
    files_moved: usize,
    total_files: usize,
    current_file: &str,
    log_line: &str,
    done: bool,
    error: Option<String>,
) {
    let _ = app.emit(
        "restructure_progress",
        RestructureProgress {
            files_moved,
            total_files,
            current_file: current_file.to_string(),
            log_line: log_line.to_string(),
            done,
            error,
        },
    );
}

fn emit_restore_progress(
    app: &AppHandle,
    restored: usize,
    total: usize,
    log_line: &str,
    done: bool,
) {
    let _ = app.emit(
        "restore_progress",
        serde_json::json!({
            "restored": restored,
            "total": total,
            "log_line": log_line,
            "done": done
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_names_are_safe() {
        // Long Czech name: must not panic on a multi-byte boundary.
        let long = "Česká spořitelna ".repeat(10);
        let s = sanitise_folder_name(&long);
        assert!(s.chars().count() <= 80);
        assert_eq!(sanitise_folder_name("a:b/c"), "a_b_c");
        assert_eq!(sanitise_folder_name("  ...  "), "Neurčeno");
        assert_eq!(sanitise_folder_name("CON"), "CON_");
        assert_eq!(sanitise_folder_name("com1"), "com1_");
        assert_eq!(sanitise_folder_name("Composite"), "Composite");
    }

    #[test]
    fn same_name_files_get_distinct_destinations() {
        let dir = std::env::temp_dir().join(format!("fileit-restructure-{}", Uuid::new_v4()));
        let mut claimed = HashSet::new();
        let a = unique_filename(&dir, "faktura.pdf", &mut claimed);
        let b = unique_filename(&dir, "Faktura.pdf", &mut claimed);
        let c = unique_filename(&dir, "faktura.pdf", &mut claimed);
        assert_eq!(a, dir.join("faktura.pdf"));
        assert_eq!(b, dir.join("Faktura_2.pdf"));
        assert_eq!(c, dir.join("faktura_3.pdf"));
    }

    #[test]
    fn journal_roundtrip_ignores_torn_line() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("fileit-journal-{}", Uuid::new_v4()));
        let mut j = Journal::start(&dir, "sess-1");
        let e = ManifestEntry {
            original_path: PathBuf::from(r"D:\Klienti\a.pdf"),
            new_path: PathBuf::from(r"C:\ReOrganized\X\a.pdf"),
            sha256: "abc".into(),
        };
        j.record(&e);
        // Simulate a crash mid-write
        j.file.as_mut().unwrap().write_all(b"{\"original_pa").unwrap();
        drop(j);
        let (sid, moves) = read_journal(&dir).unwrap();
        assert_eq!(sid, "sess-1");
        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].original_path, e.original_path);
        delete_journal(&dir);
        assert!(read_journal(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn already_in_place_is_noop() {
        let dir = std::env::temp_dir().join(format!("fileit-inplace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.pdf");
        std::fs::write(&f, b"x").unwrap();
        let (_, actual) = move_file_no_overwrite(&f, &f).unwrap();
        assert_eq!(actual, f);
        assert!(f.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn move_never_overwrites() {
        let dir = std::env::temp_dir().join(format!("fileit-move-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("src.txt");
        let dst = dir.join("dst.txt");
        std::fs::write(&src, b"new").unwrap();
        std::fs::write(&dst, b"existing").unwrap();
        let (_, actual) = move_file_no_overwrite(&src, &dst).unwrap();
        assert_eq!(actual, dir.join("dst_2.txt"));
        assert_eq!(std::fs::read(&dst).unwrap(), b"existing");
        assert_eq!(std::fs::read(&actual).unwrap(), b"new");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
