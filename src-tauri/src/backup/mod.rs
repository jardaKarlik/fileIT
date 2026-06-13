// src-tauri/src/backup/mod.rs
//
// FileIT manifest-based backup.
//
// Instead of copying files into a ZIP, we write a lightweight JSON manifest
// that records every file's original path, planned destination, and SHA-256.
// Since restructure uses fs::rename (move), files are never duplicated —
// restore simply reverses the moves using the manifest.

use crate::types::*;
use anyhow::{Context, Result};
use chrono::Utc;
use log::{info, warn, debug};
use std::path::{Path, PathBuf};

// ─────────────────────────────────────────────────────────────────────────────
// Manifest backup creation
// ─────────────────────────────────────────────────────────────────────────────

/// Create a manifest-only backup: writes a JSON file recording all file paths
/// and metadata so moves can be reversed. No file copying.
/// Returns the path to the created manifest JSON.
pub fn create_backup(
    files: &[ClassifiedFile],
    manifest: &BackupManifest,
    app_data_dir: &Path,
) -> Result<PathBuf> {
    let backup_dir = app_data_dir.join("backups");
    std::fs::create_dir_all(&backup_dir)
        .with_context(|| format!("Creating backup dir: {:?}", backup_dir))?;

    let timestamp = Utc::now().format("%Y-%m-%d_%H%M%S");
    let manifest_name = format!("manifest_{}.json", timestamp);
    let manifest_path = backup_dir.join(&manifest_name);

    info!("[backup] Creating manifest: {:?} for {} files ({} moves)",
        manifest_path, files.len(), manifest.moves.len());

    let json = serde_json::to_string_pretty(manifest)
        .with_context(|| "Serialising manifest")?;
    std::fs::write(&manifest_path, &json)
        .with_context(|| format!("Writing manifest: {:?}", manifest_path))?;

    info!("[backup] Manifest saved: {:?} ({} bytes, {} moves)",
        manifest_path, json.len(), manifest.moves.len());

    Ok(manifest_path)
}

/// Create a standalone manifest listing all scanned files and their metadata.
/// Used from Dashboard "Zálohovat" — records file inventory without moving anything.
pub fn create_standalone_manifest(
    files: &[ClassifiedFile],
    session_id: &str,
    app_data_dir: &Path,
) -> Result<PathBuf> {
    let backup_dir = app_data_dir.join("backups");
    std::fs::create_dir_all(&backup_dir)
        .with_context(|| format!("Creating backup dir: {:?}", backup_dir))?;

    let entries: Vec<ManifestEntry> = files.iter().map(|f| {
        ManifestEntry {
            original_path: f.record.path.clone(),
            new_path: f.record.path.clone(),
            sha256: f.record.sha256.clone().unwrap_or_else(|| "unknown".to_string()),
        }
    }).collect();

    let manifest = BackupManifest::new(session_id, entries);

    let timestamp = Utc::now().format("%Y-%m-%d_%H%M%S");
    let manifest_name = format!("inventory_{}.json", timestamp);
    let manifest_path = backup_dir.join(&manifest_name);

    let json = serde_json::to_string_pretty(&manifest)
        .with_context(|| "Serialising standalone manifest")?;
    std::fs::write(&manifest_path, &json)
        .with_context(|| format!("Writing standalone manifest: {:?}", manifest_path))?;

    info!("[backup] Standalone manifest saved: {:?} ({} files)", manifest_path, files.len());

    Ok(manifest_path)
}

// ─────────────────────────────────────────────────────────────────────────────
// Manifest persistence (for quick access across app restarts)
// ─────────────────────────────────────────────────────────────────────────────

/// Write the manifest to a sidecar JSON file in AppData.
pub fn save_manifest_sidecar(
    manifest: &BackupManifest,
    app_data_dir: &Path,
) -> Result<PathBuf> {
    let sidecar_dir = app_data_dir.join("state");
    std::fs::create_dir_all(&sidecar_dir)?;

    let path = sidecar_dir.join("pending_confirmation.json");
    let json = serde_json::to_string_pretty(manifest)?;
    std::fs::write(&path, &json)?;
    info!("[backup] Manifest sidecar saved: {:?} ({} moves, {} bytes)", path, manifest.moves.len(), json.len());

    Ok(path)
}

/// Load the pending confirmation manifest from sidecar, if it exists.
pub fn load_manifest_sidecar(app_data_dir: &Path) -> Option<BackupManifest> {
    let path = app_data_dir.join("state").join("pending_confirmation.json");
    if !path.exists() {
        debug!("[backup] No manifest sidecar at {:?}", path);
        return None;
    }

    let json = std::fs::read_to_string(&path).ok()?;
    let manifest: BackupManifest = serde_json::from_str(&json).ok()?;
    info!("[backup] Loaded manifest sidecar: {} moves, session {}", manifest.moves.len(), manifest.session_id);
    Some(manifest)
}

/// Delete the sidecar manifest — called after user confirms or restores.
pub fn delete_manifest_sidecar(app_data_dir: &Path) -> Result<()> {
    let path = app_data_dir.join("state").join("pending_confirmation.json");
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Completed run persistence
// ─────────────────────────────────────────────────────────────────────────────

/// Save the CompletedRun to disk so it survives app restart.
pub fn save_completed_run(run: &CompletedRun, app_data_dir: &Path) -> Result<()> {
    let path = app_data_dir.join("state").join("last_run.json");
    std::fs::create_dir_all(path.parent().unwrap())?;
    let json = serde_json::to_string_pretty(run)?;
    std::fs::write(&path, &json)?;
    info!("[backup] CompletedRun saved: session={}, {} files moved, status={:?}, manifest={:?}",
        run.session_id, run.files_moved, run.confirmation_status, run.backup_manifest_path);
    Ok(())
}

/// Load the last CompletedRun from disk.
pub fn load_completed_run(app_data_dir: &Path) -> Option<CompletedRun> {
    let path = app_data_dir.join("state").join("last_run.json");
    if !path.exists() {
        debug!("[backup] No completed run at {:?}", path);
        return None;
    }
    let json = std::fs::read_to_string(&path).ok()?;
    let run: CompletedRun = serde_json::from_str(&json).ok()?;
    info!("[backup] Loaded CompletedRun: session={}, {} files, status={:?}",
        run.session_id, run.files_moved, run.confirmation_status);
    Some(run)
}

/// Delete the completed run record — called after auto-confirm or restore.
pub fn delete_completed_run(app_data_dir: &Path) -> Result<()> {
    let path = app_data_dir.join("state").join("last_run.json");
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Cleanup
// ─────────────────────────────────────────────────────────────────────────────

/// Delete a backup manifest file.
pub fn delete_backup_manifest(manifest_path: &Path) -> Result<()> {
    if manifest_path.exists() {
        std::fs::remove_file(manifest_path)?;
        info!("Deleted backup manifest: {:?}", manifest_path);
    }
    Ok(())
}

/// Check for any backup manifests past their delete deadline and remove them.
pub fn cleanup_expired_backups(app_data_dir: &Path) -> Result<()> {
    let backup_dir = app_data_dir.join("backups");
    if !backup_dir.exists() {
        return Ok(());
    }

    let now = Utc::now();

    for entry in std::fs::read_dir(&backup_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            // Also clean up any legacy ZIP files
            if path.extension().and_then(|e| e.to_str()) == Some("zip") {
                let _ = std::fs::remove_file(&path);
                info!("Removed legacy ZIP backup: {:?}", path);
            }
            continue;
        }

        // If the manifest is older than 37 days, safe to remove
        if let Ok(metadata) = std::fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                let age = now - chrono::DateTime::<Utc>::from(modified);
                if age.num_days() > 37 {
                    let _ = std::fs::remove_file(&path);
                    info!("Expired backup manifest removed: {:?}", path);
                }
            }
        }
    }

    Ok(())
}
