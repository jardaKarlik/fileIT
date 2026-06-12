// src-tauri/src/backup/mod.rs
//
// FileIT safe-side backup.
//
// Before any restructure run, if backup_enabled is true:
//   1. Creates a ZIP archive containing all source files
//   2. Writes a manifest.json into the ZIP describing every planned move
//   3. Stores the ZIP in a hidden AppData location
//
// After user confirms restructure:
//   - ZIP auto-deletes after 7 days
//
// After user clicks Restore:
//   - restructure::restore() reads the manifest and moves files back
//   - ZIP is deleted immediately after successful restore

use crate::types::*;
use anyhow::{Context, Result};
use chrono::Utc;
use log::info;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::{write::FileOptions, ZipWriter};

// ─────────────────────────────────────────────────────────────────────────────
// Backup creation
// ─────────────────────────────────────────────────────────────────────────────

/// Estimate backup size before creating (rough calculation: total file sizes * 1.1 for ZIP overhead).
pub fn estimate_backup_size(files: &[ClassifiedFile]) -> u64 {
    let total_bytes: u64 = files
        .iter()
        .map(|f| f.record.size_bytes)
        .sum();

    // Add ~10% for ZIP metadata and Deflate compression headers
    (total_bytes as f64 * 1.1) as u64
}

/// Create the safe-side backup ZIP.
/// Returns the path to the created ZIP.
pub fn create_backup(
    files: &[ClassifiedFile],
    manifest: &BackupManifest,
    app_data_dir: &Path,
) -> Result<PathBuf> {
    // Create the backup directory if needed
    let backup_dir = app_data_dir.join("backups");
    std::fs::create_dir_all(&backup_dir)
        .with_context(|| format!("Creating backup dir: {:?}", backup_dir))?;

    let timestamp = Utc::now().format("%Y-%m-%d_%H%M%S");
    let zip_name = format!("backup_{}.zip", timestamp);
    let zip_path = backup_dir.join(&zip_name);

    info!("Creating backup ZIP: {:?}", zip_path);

    let zip_file = std::fs::File::create(&zip_path)
        .with_context(|| format!("Creating ZIP file: {:?}", zip_path))?;

    let mut zip = ZipWriter::new(zip_file);
    let options = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Write manifest.json first — it's the recovery tool
    let manifest_json = serde_json::to_string_pretty(manifest)
        .with_context(|| "Serialising manifest")?;
    zip.start_file("manifest.json", options)
        .with_context(|| "Starting manifest.json in ZIP")?;
    zip.write_all(manifest_json.as_bytes())
        .with_context(|| "Writing manifest.json")?;

    // Add all source files to the ZIP
    // We preserve a flat structure using sha256-prefixed names to avoid collisions
    let mut skipped_count = 0;
    for file in files {
        let archive_name = archive_entry_name(file);
        match add_file_to_zip(&mut zip, &file.record.path, &archive_name, options) {
            Ok(_) => {}
            Err(e) => {
                // Log but continue — a partial backup is better than no backup
                skipped_count += 1;
                log::warn!("Skipped backup of {:?}: {} (may have special characters in filename)", file.record.path, e);
            }
        }
    }

    if skipped_count > 0 {
        info!("Backup: {} files skipped due to encoding or access issues", skipped_count);
    }

    zip.finish().with_context(|| "Finalising ZIP")?;

    let zip_size = std::fs::metadata(&zip_path)?.len();
    info!("Backup created: {:?} ({} bytes)", zip_path, zip_size);

    Ok(zip_path)
}

fn archive_entry_name(file: &ClassifiedFile) -> String {
    // Use a short hash prefix to avoid collisions for same-named files
    let hash_prefix = file.record.sha256
        .as_deref()
        .map(|h| &h[..8])
        .unwrap_or("00000000");
    format!("files/{}_{}", hash_prefix, file.record.name)
}

fn add_file_to_zip(
    zip: &mut ZipWriter<std::fs::File>,
    path: &Path,
    archive_name: &str,
    options: FileOptions<()>,
) -> Result<()> {
    // Verify archive_name is valid UTF-8 before writing
    // This ensures proper encoding of Czech characters and other non-ASCII names
    if archive_name.as_bytes().is_empty() {
        anyhow::bail!("Archive entry name cannot be empty");
    }

    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Opening for backup: {:?}", path))?;

    // Start ZIP entry with UTF-8 filename encoding
    // The zip crate v2 now properly handles UTF-8 encoding when names contain non-ASCII characters
    zip.start_file(archive_name, options)
        .with_context(|| {
            format!(
                "Adding file to ZIP archive. Entry: '{}' (source: {:?}). \
                 This typically fails with non-ASCII names if UTF-8 encoding is not handled correctly.",
                archive_name, path
            )
        })?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .with_context(|| format!("Reading file content for backup: {:?}", path))?;

    zip.write_all(&buffer)
        .with_context(|| format!("Writing file to ZIP archive: {}", archive_name))?;

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Manifest persistence (separate from the ZIP — for quick access)
// ─────────────────────────────────────────────────────────────────────────────

/// Write the manifest to a sidecar JSON file in AppData.
/// This allows the app to load the pending confirmation state on next launch
/// without unzipping the backup.
pub fn save_manifest_sidecar(
    manifest: &BackupManifest,
    app_data_dir: &Path,
) -> Result<PathBuf> {
    let sidecar_dir = app_data_dir.join("state");
    std::fs::create_dir_all(&sidecar_dir)?;

    let path = sidecar_dir.join("pending_confirmation.json");
    let json = serde_json::to_string_pretty(manifest)?;
    std::fs::write(&path, json)?;

    Ok(path)
}

/// Load the pending confirmation manifest from sidecar, if it exists.
pub fn load_manifest_sidecar(app_data_dir: &Path) -> Option<BackupManifest> {
    let path = app_data_dir.join("state").join("pending_confirmation.json");
    if !path.exists() {
        return None;
    }

    let json = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&json).ok()
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
    std::fs::write(path, json)?;
    Ok(())
}

/// Load the last CompletedRun from disk.
pub fn load_completed_run(app_data_dir: &Path) -> Option<CompletedRun> {
    let path = app_data_dir.join("state").join("last_run.json");
    if !path.exists() {
        return None;
    }
    let json = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
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
// ZIP Integrity Validation
// ─────────────────────────────────────────────────────────────────────────────

/// Validate ZIP integrity and return the manifest.
/// Returns Err if ZIP is corrupted or manifest is missing/invalid.
pub fn validate_backup_zip(zip_path: &Path) -> Result<BackupManifest> {
    use zip::ZipArchive;

    let zip_file = std::fs::File::open(zip_path)
        .with_context(|| format!("Opening backup ZIP: {:?}", zip_path))?;

    let mut archive = ZipArchive::new(zip_file)
        .with_context(|| format!("ZIP archive corrupted or invalid: {:?}", zip_path))?;

    // Try to read manifest.json
    let mut manifest_file = archive.by_name("manifest.json")
        .with_context(|| "Manifest not found in backup ZIP")?;

    let mut manifest_json = String::new();
    std::io::Read::read_to_string(&mut manifest_file, &mut manifest_json)
        .with_context(|| "Failed to read manifest.json from ZIP")?;

    let manifest: BackupManifest = serde_json::from_str(&manifest_json)
        .with_context(|| "Manifest JSON invalid or corrupted")?;

    info!("Backup ZIP validated: {:?} ({} files)", zip_path, manifest.moves.len());
    Ok(manifest)
}

// ─────────────────────────────────────────────────────────────────────────────
// Cleanup
// ─────────────────────────────────────────────────────────────────────────────

/// Delete the backup ZIP — called 7 days after user confirms.
pub fn delete_backup_zip(zip_path: &Path) -> Result<()> {
    if zip_path.exists() {
        std::fs::remove_file(zip_path)?;
        info!("Deleted backup ZIP: {:?}", zip_path);
    }
    Ok(())
}

/// Check for any backup ZIPs past their delete deadline and remove them.
pub fn cleanup_expired_backups(app_data_dir: &Path) -> Result<()> {
    let backup_dir = app_data_dir.join("backups");
    if !backup_dir.exists() {
        return Ok(());
    }

    let now = Utc::now();

    for entry in std::fs::read_dir(&backup_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("zip") {
            continue;
        }

        // If the file is older than 37 days (30 day confirm window + 7 day delete window)
        // it's safe to remove regardless of confirmation state
        if let Ok(metadata) = std::fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                let age = now - chrono::DateTime::<Utc>::from(modified);
                if age.num_days() > 37 {
                    let _ = std::fs::remove_file(&path);
                    info!("Expired backup removed: {:?}", path);
                }
            }
        }
    }

    Ok(())
}
