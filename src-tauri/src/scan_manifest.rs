// src-tauri/src/scan_manifest.rs
//
// Scan Manifest Persistence Module
// Saves scan history to enable fast re-scans by detecting file modifications
// and reusing previous classifications for unchanged files.

use crate::types::{ClassifiedFile, FileRecord};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Persistent manifest for a single scan session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanManifest {
    pub scan_id: String,
    pub scanned_at: String,
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub files: Vec<ManifestEntry>,
}

/// Single file entry in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub path: String,
    pub sha256: String,
    pub modified: i64,
    pub size_bytes: u64,
    /// JSON-encoded ClassifiedFile (if classification was successful)
    pub classification: Option<String>,
}

/// Result of comparing current scan with previous manifest
#[derive(Debug)]
pub struct ScanDiff {
    pub unchanged_files: Vec<ManifestEntry>,
    pub modified_files: Vec<PathBuf>,
    pub new_files: Vec<PathBuf>,
    pub deleted_files: Vec<String>,
}

impl ScanManifest {
    /// Create a new manifest from scan results
    pub fn from_scan_results(
        scan_id: &str,
        files: &[ClassifiedFile],
        scan_time: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        let entries: Vec<ManifestEntry> = files
            .iter()
            .map(|cf| {
                let classification_json = serde_json::to_string(cf).ok();
                ManifestEntry {
                    path: cf.record.path.to_string_lossy().to_string(),
                    sha256: cf.record.sha256.clone().unwrap_or_default(),
                    modified: cf.record.modified.timestamp(),
                    size_bytes: cf.record.size_bytes,
                    classification: classification_json,
                }
            })
            .collect();

        ScanManifest {
            scan_id: scan_id.to_string(),
            scanned_at: scan_time.to_rfc3339(),
            total_files: entries.len(),
            total_size_bytes: entries.iter().map(|e| e.size_bytes).sum(),
            files: entries,
        }
    }

    /// Save manifest to disk
    pub fn save(&self, app_data_dir: &Path) -> Result<PathBuf> {
        let manifest_dir = app_data_dir.join("scan_history");
        std::fs::create_dir_all(&manifest_dir)?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("scan_{}_{}.json", self.scan_id, timestamp);
        let manifest_path = manifest_dir.join(filename);

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&manifest_path, json)?;

        log::info!(
            "Saved scan manifest: {} files to {:?}",
            self.total_files,
            manifest_path
        );

        // Update "latest" symlink/copy
        let latest_path = manifest_dir.join("latest.json");
        std::fs::copy(&manifest_path, &latest_path)?;

        Ok(manifest_path)
    }

    /// Load the most recent manifest
    pub fn load_latest(app_data_dir: &Path) -> Option<Self> {
        let latest_path = app_data_dir.join("scan_history").join("latest.json");
        if !latest_path.exists() {
            return None;
        }

        match std::fs::read_to_string(&latest_path) {
            Ok(json) => match serde_json::from_str::<Self>(&json) {
                Ok(manifest) => {
                    log::info!(
                        "Loaded previous scan manifest: {} files from {}",
                        manifest.total_files,
                        manifest.scanned_at
                    );
                    Some(manifest)
                }
                Err(e) => {
                    log::warn!("Failed to parse previous scan manifest: {}", e);
                    None
                }
            },
            Err(e) => {
                log::warn!("Failed to read previous scan manifest: {}", e);
                None
            }
        }
    }

    /// Compare current files with this manifest to detect changes
    pub fn diff(&self, current_files: &[FileRecord]) -> ScanDiff {
        let mut previous_map: HashMap<String, &ManifestEntry> = HashMap::new();
        for entry in &self.files {
            previous_map.insert(entry.path.clone(), entry);
        }

        let mut current_map: HashMap<String, &FileRecord> = HashMap::new();
        for file in current_files {
            current_map.insert(file.path.to_string_lossy().to_string(), file);
        }

        let mut unchanged = Vec::new();
        let mut modified = Vec::new();
        let mut new_files = Vec::new();

        // Check each current file
        for (path, file) in &current_map {
            if let Some(prev_entry) = previous_map.get(path) {
                // File exists in both - check if modified
                let current_sha = file.sha256.as_deref().unwrap_or("");
                let current_modified = file.modified.timestamp();

                if prev_entry.sha256 == current_sha
                    && prev_entry.modified == current_modified
                    && prev_entry.size_bytes == file.size_bytes
                {
                    // File unchanged
                    unchanged.push((*prev_entry).clone());
                } else {
                    // File modified
                    modified.push(PathBuf::from(path));
                }
            } else {
                // New file
                new_files.push(PathBuf::from(path));
            }
        }

        // Find deleted files
        let deleted: Vec<String> = previous_map
            .keys()
            .filter(|path| !current_map.contains_key(*path))
            .cloned()
            .collect();

        ScanDiff {
            unchanged_files: unchanged,
            modified_files: modified,
            new_files,
            deleted_files: deleted,
        }
    }
}
