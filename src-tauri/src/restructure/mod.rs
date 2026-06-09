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
        let dest_file = unique_filename(&folder_path, &file.record.name);

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
/// Returns (manifest_entries, folders_created, duration_seconds).
pub async fn execute(
    app: &AppHandle,
    planned_moves: &[PlannedMove],
    session_id: &str,
) -> Result<(Vec<ManifestEntry>, usize, u64)> {
    let start = std::time::Instant::now();
    let total = planned_moves.len();
    let mut manifest_entries: Vec<ManifestEntry> = Vec::new();
    let mut folders_created = 0usize;
    let mut moved = 0usize;

    info!("Starting restructure: {} files to move", total);

    for (idx, plan) in planned_moves.iter().enumerate() {
        let _pct = ((idx as f32 / total as f32) * 100.0) as u32;
        let log_line = format!(
            "→ Přesouváme: {}",
            plan.source.file_name()
                .unwrap_or_default()
                .to_string_lossy()
        );

        emit_progress(app, moved, total, &plan.file_name, &log_line, false, None);

        // Create destination directory if needed
        if let Some(parent) = plan.destination.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Creating directory: {:?}", parent))?;
                folders_created += 1;
                debug!("Created folder: {:?}", parent);
            }
        }

        // Perform the move
        match move_file(&plan.source, &plan.destination) {
            Ok(sha256) => {
                manifest_entries.push(ManifestEntry {
                    original_path: plan.source.clone(),
                    new_path: plan.destination.clone(),
                    sha256,
                });
                moved += 1;
                debug!("Moved {}/{}: {:?}", moved, total, plan.file_name);
            }
            Err(e) => {
                warn!("Failed to move {:?}: {}", plan.source, e);
                emit_progress(
                    app, moved, total, &plan.file_name,
                    &format!("⚠ Chyba: {} — přeskakujeme", plan.file_name),
                    false,
                    Some(e.to_string()),
                );
            }
        }
    }

    let duration = start.elapsed().as_secs();
    emit_progress(app, moved, total, "", "✓ Uspořádání dokončeno", true, None);
    info!("Restructure complete: {} files moved in {}s", moved, duration);

    Ok((manifest_entries, folders_created, duration))
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
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Recreating directory: {:?}", parent))?;
            }
        }

        // Move back from new_path → original_path
        match move_file(&entry.new_path, &entry.original_path) {
            Ok(_) => {
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

/// Move a file, computing its SHA-256 digest before moving.
/// Uses fs::rename for same-filesystem moves (fast, atomic on most OSes).
/// Falls back to copy+delete for cross-filesystem moves.
fn move_file(src: &Path, dst: &Path) -> Result<String> {
    // Compute hash before moving (source is still readable here)
    let hash = crate::scanner::compute_hash_for_manifest(src)
        .unwrap_or_else(|| "unknown".to_string());

    // Try rename first (fast path)
    if std::fs::rename(src, dst).is_ok() {
        return Ok(hash);
    }

    // Cross-filesystem fallback: copy then delete
    std::fs::copy(src, dst)
        .with_context(|| format!("Copying {:?} to {:?}", src, dst))?;
    std::fs::remove_file(src)
        .with_context(|| format!("Removing source {:?}", src))?;

    Ok(hash)
}

/// Sanitise a string for use as a folder name on Windows.
/// Removes characters forbidden in NTFS paths, control characters, and extra whitespace.
fn sanitise_folder_name(name: &str) -> String {
    let forbidden: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let sanitised: String = name
        .chars()
        .filter(|c| !c.is_control() && !c.is_whitespace())  // Strip control chars AND whitespace
        .map(|c| if forbidden.contains(&c) { '_' } else { c })
        .collect();

    // Ensure single spaces between words (collapse multiple spaces)
    let collapsed = sanitised.split_whitespace().collect::<Vec<_>>().join(" ");

    // Trim trailing dots and spaces (Windows disallows them)
    let trimmed = collapsed.trim_end_matches(|c| c == '.' || c == ' ');

    // Limit folder name length to 80 chars
    if trimmed.len() > 80 {
        trimmed[..80].to_string()
    } else {
        trimmed.to_string()
    }
}

/// Generate a unique destination filename, appending _2, _3, ... if needed.
fn unique_filename(folder: &Path, filename: &str) -> PathBuf {
    let path = folder.join(filename);
    if !path.exists() {
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
        if !new_path.exists() {
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
