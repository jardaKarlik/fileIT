// src-tauri/src/commands/mod.rs
//
// Tauri IPC command handlers.
// Every function here is callable from the React frontend via:
//   invoke('command_name', { ...args })
//
// All commands are async. Heavy work is spawned onto Tokio task pool.
// SessionState is held in a Tauri-managed Mutex.

use crate::backup;
use crate::classifier::{Classifier, ClassificationHints};
use crate::cm_registry::CmRegistry;
use crate::metadata_reader;
use crate::scanner;
use crate::restructure;
use crate::types::*;
use anyhow::Result;
use chrono::Utc;
use log::{info, warn};
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};
use tauri::{AppHandle, Emitter, State};

/// Global session state — wrapped in Mutex for thread-safe access.
pub struct AppState {
    pub session: Mutex<SessionState>,
    pub classifier: Classifier,
    pub app_data_dir: PathBuf,
    pub cm_registry: RwLock<CmRegistry>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Setup & app lifecycle
// ─────────────────────────────────────────────────────────────────────────────

/// Called on app start. Returns any pending confirmation from a previous run.
#[tauri::command]
pub async fn app_init(
    state: State<'_, AppState>,
) -> Result<Option<CompletedRun>, String> {
    let run = backup::load_completed_run(&state.app_data_dir);

    // Check if auto-confirm deadline has passed
    if let Some(ref r) = run {
        if Utc::now() > r.auto_confirm_at
            && r.confirmation_status == ConfirmationStatus::Pending
        {
            let mut mut_run = r.clone();
            mut_run.confirmation_status = ConfirmationStatus::AutoConfirmed;
            let _ = backup::save_completed_run(&mut_run, &state.app_data_dir);
            let _ = backup::cleanup_expired_backups(&state.app_data_dir);
            return Ok(None);
        }
    }

    Ok(run)
}

/// Detect installed cloud sync clients.
#[tauri::command]
pub async fn detect_clouds() -> Vec<DetectedCloud> {
    scanner::detect_cloud_clients()
}

// ─────────────────────────────────────────────────────────────────────────────
// Scanning
// ─────────────────────────────────────────────────────────────────────────────

/// Start a scan. Emits "scan_progress" and "classify_progress" events.
/// When done, returns a ScanResult.
#[tauri::command]
pub async fn start_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ScanRequest,
) -> Result<ScanResult, String> {
    let mut categories: Vec<FileCategory> = request
        .file_categories
        .iter()
        .filter_map(|s| match s.as_str() {
            "pdf" => Some(FileCategory::Pdf),
            "word" => Some(FileCategory::Word),
            "excel" => Some(FileCategory::Excel),
            "image" => Some(FileCategory::Image),
            "email" => Some(FileCategory::Email),
            "text" => Some(FileCategory::Text),
            "other" => Some(FileCategory::Other),
            _ => None,
        })
        .collect();

    if categories.is_empty() {
        info!("No categories specified — scanning all supported types");
        categories = vec![
            FileCategory::Pdf,
            FileCategory::Word,
            FileCategory::Excel,
            FileCategory::Image,
            FileCategory::Email,
            FileCategory::Text,
        ];
    }

    {
        let mut session = state.session.lock().map_err(|e| e.to_string())?;
        *session = SessionState::new();
        session.scan_roots = request.roots.clone();
        session.file_categories = categories.clone();
    }

    let scan_roots: Vec<PathBuf> = if !request.roots.is_empty() {
        request.roots.clone()
    } else {
        // Scan the entire system drive so we catch documents everywhere —
        // not just USERPROFILE. The scanner's SKIP_DIRS list keeps us out
        // of Windows, Program Files, and other non-document directories.
        let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
        let root = PathBuf::from(format!("{}\\", drive));
        info!("No scan roots provided — scanning entire drive: {:?}", root);
        vec![root]
    };

    let records = scanner::scan_directories(&app, scan_roots, &categories, &[])
        .await
        .map_err(|e| e.to_string())?;

    let mut classified: Vec<ClassifiedFile> = Vec::new();
    let total = records.len();
    let mut seen_hashes: std::collections::HashMap<String, std::path::PathBuf> =
        std::collections::HashMap::new();

    for (i, record) in records.into_iter().enumerate() {
        let _ = app.emit(
            "classify_progress",
            serde_json::json!({
                "done": i,
                "total": total,
                "current": record.name
            }),
        );

        // ── Stage 1: PDF metadata fast-read + CM registry match ──────────────
        let hints = if record.extension == "pdf" {
            // spawn_blocking + 2s timeout: prevents cloud-only PDFs (OneDrive placeholders)
            // from hanging the classify loop indefinitely with blocking file I/O.
            let path_clone = record.path.clone();
            let meta = match tokio::time::timeout(
                std::time::Duration::from_secs(2),
                tokio::task::spawn_blocking(move || metadata_reader::read_pdf_metadata(&path_clone)),
            )
            .await
            {
                Ok(Ok(m)) => m,
                _ => metadata_reader::PdfMetadata::default(),
            };

            // Try CM registry first (may have richer / more up-to-date fingerprints)
            let cm_result = state.cm_registry.read().ok().and_then(|registry| {
                registry.match_file(&meta, &record.name, record.size_bytes)
            });

            let (institution_name, confidence_boost, skip_extraction) = if let Some(cm) = cm_result {
                info!(
                    "CM match for {:?}: {} (score={:.2}, skip={})",
                    record.name, cm.institution, cm.score, cm.skip_extraction
                );
                (Some(cm.institution), cm.confidence_boost, cm.skip_extraction)
            } else {
                // Fall back to hardcoded fingerprints
                (meta.institution_hint, meta.confidence_boost, meta.skip_extraction)
            };

            Some(ClassificationHints {
                institution_name,
                confidence_boost,
                skip_extraction,
                pdf_producer: meta.producer,
                pdf_creator: meta.creator,
            })
        } else {
            None
        };
        // ─────────────────────────────────────────────────────────────────────

        let sha = record.sha256.clone();
        let mut cf = state.classifier.classify_with_hints(record, hints).await;

        if let Some(ref hash) = sha {
            if let Some(first_path) = seen_hashes.get(hash) {
                cf.is_duplicate = true;
                cf.duplicate_of = Some(first_path.clone());
            } else {
                seen_hashes.insert(hash.clone(), cf.record.path.clone());
            }
        }

        classified.push(cf);
    }

    let _ = app.emit(
        "classify_progress",
        serde_json::json!({
            "done": total,
            "total": total,
            "current": ""
        }),
    );

    let customers_found = classified.iter()
        .filter_map(|f| f.customer.as_ref())
        .map(|c| c.name.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();

    let duplicates_found = classified.iter().filter(|f| f.is_duplicate).count();
    let total_size: u64 = classified.iter().map(|f| f.record.size_bytes).sum();

    let result = ScanResult {
        total_files: classified.len(),
        total_size_bytes: total_size,
        customers_found,
        duplicates_found,
        files: classified.clone(),
    };

    {
        let mut session = state.session.lock().map_err(|e| e.to_string())?;
        session.classified_files = classified;
    }

    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// Restructure
// ─────────────────────────────────────────────────────────────────────────────

/// Build a restructure preview. No filesystem changes.
#[tauri::command]
pub async fn build_preview(
    state: State<'_, AppState>,
    request: PreviewRequest,
) -> Result<RestructurePreview, String> {
    let structure: Vec<GroupDimension> = request
        .structure
        .iter()
        .filter_map(|s| match s.as_str() {
            "customer" => Some(GroupDimension::Customer),
            "date" => Some(GroupDimension::Date),
            "inst" => Some(GroupDimension::Institution),
            "doc" => Some(GroupDimension::DocumentType),
            _ => None,
        })
        .collect();

    let preview = {
        let session = state.session.lock().map_err(|e| e.to_string())?;

        if session.classified_files.is_empty() {
            return Err("No classified files in session. Run a scan first.".to_string());
        }

        restructure::preview(
            &session.classified_files,
            &structure,
            &request.target_path,
        )
        .map_err(|e| e.to_string())?
    };

    {
        let mut session = state.session.lock().map_err(|e| e.to_string())?;
        session.preview = Some(preview.clone());
    }

    Ok(preview)
}

/// Execute the restructure. Emits "restructure_progress" events.
#[tauri::command]
pub async fn run_restructure(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RunRestructureRequest,
) -> Result<RestructureResult, String> {
    let (planned_moves, session_id, classified_files) = {
        let session = state.session.lock().map_err(|e| e.to_string())?;
        let preview = session.preview.as_ref()
            .ok_or("No preview computed. Call build_preview first.")?;
        info!("[run_restructure] Session {} — {} files, backup_enabled={}",
            session.id, session.classified_files.len(), request.backup_enabled);
        (
            preview.planned_moves.clone(),
            session.id.clone(),
            session.classified_files.clone(),
        )
    };

    // Step 1: Create backup BEFORE restructure — files are still at original paths
    let pre_manifest_entries: Vec<ManifestEntry> = planned_moves.iter().map(|m| {
        ManifestEntry {
            original_path: m.source.clone(),
            new_path: m.destination.clone(),
            sha256: "pending".to_string(),
        }
    }).collect();

    let total_files = planned_moves.len();

    let backup_manifest_path = if request.backup_enabled {
        info!("[run_restructure] Creating pre-restructure manifest for {} files", classified_files.len());
        let _ = app.emit("restructure_progress", RestructureProgress {
            files_moved: 0, total_files,
            current_file: "Vytváříme manifest zálohy…".to_string(),
            log_line: "→ Vytváříme manifest zálohy…".to_string(),
            done: false, error: None,
        });
        let pre_manifest = BackupManifest::new(&session_id, pre_manifest_entries);
        match backup::create_backup(&classified_files, &pre_manifest, &state.app_data_dir) {
            Ok(path) => {
                info!("[run_restructure] Backup manifest created: {:?}", path);
                let _ = app.emit("restructure_progress", RestructureProgress {
                    files_moved: 0, total_files,
                    current_file: "Manifest vytvořen".to_string(),
                    log_line: "✓ Manifest zálohy vytvořen".to_string(),
                    done: false, error: None,
                });
                Some(path)
            }
            Err(e) => {
                warn!("[run_restructure] Manifest creation failed (continuing without): {}", e);
                let _ = app.emit("restructure_progress", RestructureProgress {
                    files_moved: 0, total_files,
                    current_file: "Manifest se nepodařil — pokračujeme".to_string(),
                    log_line: format!("⚠ Manifest selhal: {} — pokračujeme bez zálohy", e),
                    done: false, error: None,
                });
                None
            }
        }
    } else {
        info!("[run_restructure] Backup disabled by user");
        None
    };

    // Step 2: Execute the restructure (moves files from source → destination)
    info!("[run_restructure] Starting restructure: {} planned moves", planned_moves.len());
    let (manifest_entries, folders_created, duration_secs, move_errors) =
        restructure::execute(&app, &planned_moves, &session_id, &state.app_data_dir)
            .await
            .map_err(|e| e.to_string())?;

    let files_moved = manifest_entries.len();
    let failed_count = move_errors.len();
    let unknown_count = planned_moves
        .iter()
        .filter(|m| m.destination.to_string_lossy().contains("Neznámý klient"))
        .count();
    info!("[run_restructure] Restructure done: {} moved, {} failed, {} folders, {} unknown",
        files_moved, failed_count, folders_created, unknown_count);

    // Step 3: Save manifest sidecar (with actual SHA-256 hashes from the move)
    let manifest = BackupManifest::new(&session_id, manifest_entries);
    let _ = backup::save_manifest_sidecar(&manifest, &state.app_data_dir);
    info!("[run_restructure] Manifest sidecar saved");

    // Step 4: Persist CompletedRun for restore/confirm
    let completed_run = {
        let session = state.session.lock().map_err(|e| e.to_string())?;
        let target = session.preview
            .as_ref()
            .map(|p| p.target_path.clone())
            .unwrap_or_default();

        CompletedRun {
            session_id: session_id.clone(),
            completed_at: Utc::now(),
            duration_seconds: duration_secs,
            files_moved,
            folders_created,
            unknown_count,
            target_path: target,
            backup_manifest_path: backup_manifest_path.clone().unwrap_or_default(),
            manifest,
            confirmation_status: ConfirmationStatus::Pending,
            auto_confirm_at: Utc::now() + chrono::Duration::days(30),
            backup_delete_at: None,
        }
    };

    let _ = backup::save_completed_run(&completed_run, &state.app_data_dir);
    info!("[run_restructure] CompletedRun saved to last_run.json");

    let target_path_str = completed_run.target_path.to_string_lossy().to_string();

    Ok(RestructureResult {
        session_id,
        files_moved,
        folders_created,
        unknown_count,
        duration_seconds: duration_secs,
        backup_manifest_path: backup_manifest_path.map(|p| p.to_string_lossy().to_string()),
        target_path: target_path_str,
        failed_count,
        errors: move_errors,
    })
}

/// Standalone backup — creates a JSON manifest of all currently classified files
/// without restructuring anything. Called from Dashboard "Zálohovat".
#[tauri::command]
pub async fn create_standalone_backup(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let (classified_files, session_id) = {
        let session = state.session.lock().map_err(|e| e.to_string())?;
        if session.classified_files.is_empty() {
            return Err("Nejsou žádné naskenované soubory. Spusťte nejprve skenování.".to_string());
        }
        (session.classified_files.clone(), session.id.clone())
    };

    match backup::create_standalone_manifest(&classified_files, &session_id, &state.app_data_dir) {
        Ok(path) => Ok(path.to_string_lossy().to_string()),
        Err(e) => Err(format!("Záloha selhala: {}", e)),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Confirmation / Restore
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn confirm_restructure(
    state: State<'_, AppState>,
) -> Result<(), String> {
    info!("[confirm] User confirming restructure");
    let mut run = backup::load_completed_run(&state.app_data_dir)
        .ok_or("No completed run found")?;

    run.confirmation_status = ConfirmationStatus::Confirmed;
    run.backup_delete_at = Some(Utc::now() + chrono::Duration::days(7));
    info!("[confirm] Status → Confirmed, ZIP delete scheduled for +7 days");

    backup::save_completed_run(&run, &state.app_data_dir)
        .map_err(|e| e.to_string())?;

    backup::delete_manifest_sidecar(&state.app_data_dir)
        .map_err(|e| e.to_string())?;

    info!("[confirm] Done — sidecar deleted");
    Ok(())
}

#[tauri::command]
pub async fn restore_restructure(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    info!("[restore] User initiating restore");
    let run = backup::load_completed_run(&state.app_data_dir)
        .ok_or("No completed run found")?;
    info!("[restore] Restoring session {} — {} files to move back", run.session_id, run.manifest.moves.len());

    let restored = restructure::restore(&app, &run.manifest)
        .await
        .map_err(|e| e.to_string())?;
    info!("[restore] Restore complete: {}/{} files restored", restored, run.manifest.moves.len());

    if run.backup_manifest_path.as_os_str().len() > 0 {
        info!("[restore] Deleting backup manifest: {:?}", run.backup_manifest_path);
        let _ = backup::delete_backup_manifest(&run.backup_manifest_path);
    }
    let _ = backup::delete_completed_run(&state.app_data_dir);
    let _ = backup::delete_manifest_sidecar(&state.app_data_dir);
    info!("[restore] Cleanup done — run and sidecar deleted");

    Ok(restored)
}

// ─────────────────────────────────────────────────────────────────────────────
// Utility commands
// ─────────────────────────────────────────────────────────────────────────────

/// Detect the OneDrive sync root on Windows.
/// Priority: HKCU registry "UserFolder" → OneDrive/OneDriveConsumer env vars → ~/OneDrive
/// Appends \Documents if that subfolder exists, otherwise returns the root.
/// Returns None when not on Windows or OneDrive is not installed.
#[tauri::command]
pub fn get_onedrive_path() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        use std::path::Path;

        // Priority 1: Registry HKCU\Software\Microsoft\OneDrive "UserFolder"
        let registry_path: Option<String> = (|| -> Option<String> {
            use winreg::enums::*;
            use winreg::RegKey;
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let key = hkcu.open_subkey(r"Software\Microsoft\OneDrive").ok()?;
            key.get_value("UserFolder").ok()
        })();

        // Priority 2: Environment variables
        let env_path: Option<String> = std::env::var("OneDrive")
            .ok()
            .or_else(|| std::env::var("OneDriveConsumer").ok());

        // Priority 3: ~/OneDrive
        let home_path: Option<String> = std::env::var("USERPROFILE")
            .ok()
            .map(|p| format!("{}\\OneDrive", p))
            .filter(|p| Path::new(p).exists());

        let base = registry_path.or(env_path).or(home_path)?;
        let base_path = Path::new(&base);

        // Prefer a \Documents subfolder if it exists
        let docs = base_path.join("Documents");
        if docs.is_dir() {
            Some(format!("{}\\reOrganized", docs.display()))
        } else {
            Some(format!("{}\\reOrganized", base_path.display()))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .file()
        .blocking_pick_folder()
        .map(|p| p.to_string())
}

#[tauri::command]
pub async fn open_in_explorer(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// CM registry — version check + download
// ─────────────────────────────────────────────────────────────────────────────

const CM_REGISTRY_URL: &str = "https://cm.fileit.cz/registry/latest.json";
const CM_CONTRIBUTE_URL: &str = "https://cm.fileit.cz/contribute";

/// Check for CM registry updates on launch. Silently falls back if offline.
#[tauri::command]
pub async fn check_cm_update(
    state: State<'_, AppState>,
) -> Result<CmUpdateResult, String> {
    let local_version = state.cm_registry
        .read()
        .map(|r| r.version.clone())
        .unwrap_or_else(|_| "unknown".to_string());

    let app_data_dir = state.app_data_dir.clone();
    let cache_path = app_data_dir.join("cm_registry.json");

    // Attempt version check with 5s timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    match client.get(CM_REGISTRY_URL).send().await {
        Err(e) => {
            info!("CM update check failed (offline?): {}", e);
            let count = state.cm_registry.read()
                .map(|r| r.fingerprints.len() as u32)
                .unwrap_or(0);
            return Ok(CmUpdateResult {
                updated: false,
                version: local_version,
                fingerprint_count: count,
            });
        }
        Ok(resp) => {
            if !resp.status().is_success() {
                let count = state.cm_registry.read()
                    .map(|r| r.fingerprints.len() as u32)
                    .unwrap_or(0);
                return Ok(CmUpdateResult {
                    updated: false,
                    version: local_version,
                    fingerprint_count: count,
                });
            }

            let remote: CmRegistry = match resp.json().await {
                Ok(r) => r,
                Err(e) => {
                    warn!("CM registry JSON parse failed: {}", e);
                    let count = state.cm_registry.read()
                        .map(|r| r.fingerprints.len() as u32)
                        .unwrap_or(0);
                    return Ok(CmUpdateResult {
                        updated: false,
                        version: local_version,
                        fingerprint_count: count,
                    });
                }
            };

            if remote.version <= local_version {
                let count = state.cm_registry.read()
                    .map(|r| r.fingerprints.len() as u32)
                    .unwrap_or(0);
                return Ok(CmUpdateResult {
                    updated: false,
                    version: local_version,
                    fingerprint_count: count,
                });
            }

            // Newer version — apply and persist
            let new_version = remote.version.clone();
            let new_count = remote.fingerprints.len() as u32;

            if let Ok(mut registry) = state.cm_registry.write() {
                *registry = remote;
            }

            if let Ok(registry) = state.cm_registry.read() {
                let _ = registry.save_to_file(&cache_path);
            }

            info!("CM registry updated: version {}, {} fingerprints", new_version, new_count);

            Ok(CmUpdateResult {
                updated: true,
                version: new_version,
                fingerprint_count: new_count,
            })
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Učebna — unclassified pattern grouping and teaching
// ─────────────────────────────────────────────────────────────────────────────

/// Group unclassified files from the last scan into pattern cards for Učebna.
/// Files are never exposed — only aggregate signals are returned.
#[tauri::command]
pub async fn group_unclassified_patterns(
    state: State<'_, AppState>,
) -> Result<Vec<UnclassifiedPattern>, String> {
    let classified_files = {
        let session = state.session.lock().map_err(|e| e.to_string())?;
        session.classified_files.clone()
    };

    // Unclassified = no institution identified OR low confidence
    let unclassified: Vec<&ClassifiedFile> = classified_files.iter()
        .filter(|f| f.institution.is_none() || f.confidence < 0.65)
        .collect();

    // Group by: pdf_producer → size_bucket (when producer absent)
    let mut groups: std::collections::HashMap<String, Vec<&ClassifiedFile>> =
        std::collections::HashMap::new();

    for cf in &unclassified {
        let key = cf.pdf_producer.clone()
            .or_else(|| cf.pdf_creator.clone())
            .unwrap_or_else(|| {
                format!("size:{}", metadata_reader::size_to_bucket(cf.record.size_bytes))
            });
        groups.entry(key).or_default().push(cf);
    }

    // Build pattern list, most frequent first, max 50
    let mut patterns: Vec<UnclassifiedPattern> = groups.into_iter()
        .map(|(key, files)| {
            let first = files[0];
            let mut signals = Vec::new();

            if let Some(ref prod) = first.pdf_producer {
                signals.push(PatternSignal {
                    label: format!("Producent: {}", prod),
                    strength: "strong".to_string(),
                });
            }
            if let Some(ref cre) = first.pdf_creator {
                signals.push(PatternSignal {
                    label: format!("Creator: {}", cre),
                    strength: "medium".to_string(),
                });
            }
            signals.push(PatternSignal {
                label: format!("Velikost: {}", metadata_reader::size_to_bucket(first.record.size_bytes)),
                strength: "weak".to_string(),
            });

            let pattern_id = format!("p_{}", sanitize_pattern_key(&key));
            let size_bucket = Some(metadata_reader::size_to_bucket(first.record.size_bytes));

            UnclassifiedPattern {
                pattern_id: pattern_id.clone(),
                file_count: files.len(),
                signals,
                thumbnail_colors: vec![],
                pdf_producer: first.pdf_producer.clone(),
                pdf_creator: first.pdf_creator.clone(),
                size_bucket,
            }
        })
        .collect();

    patterns.sort_by(|a, b| b.file_count.cmp(&a.file_count));
    patterns.truncate(50);

    // Persist pattern_data in session for submit_teaching lookups
    {
        let mut session = state.session.lock().map_err(|e| e.to_string())?;
        session.pattern_data.clear();
        for pat in &patterns {
            session.pattern_data.insert(
                pat.pattern_id.clone(),
                GroupedPatternData {
                    pdf_producer: pat.pdf_producer.clone(),
                    pdf_creator: pat.pdf_creator.clone(),
                    color_buckets: pat.thumbnail_colors.clone(),
                },
            );
        }
    }

    Ok(patterns)
}

/// Record a user teaching decision. Adds fingerprint to local CM registry.
/// Does NOT upload to cloud — that requires a separate explicit upload_to_cm call.
#[tauri::command]
pub async fn submit_teaching(
    pattern_id: String,
    institution: String,
    doc_type: String,
    state: State<'_, AppState>,
) -> Result<TeachingResult, String> {
    let pattern_data = {
        let session = state.session.lock().map_err(|e| e.to_string())?;
        session.pattern_data.get(&pattern_id).cloned()
    };

    let data = pattern_data.ok_or_else(|| format!("Unknown pattern_id: {}", pattern_id))?;

    let cm_payload = CmPayload {
        pdf_producer: data.pdf_producer.clone(),
        pdf_creator: data.pdf_creator.clone(),
        color_buckets: data.color_buckets.clone(),
        logo_phash: None,
        institution: institution.clone(),
        doc_type: doc_type.clone(),
    };

    // Build a new fingerprint and add to local CM registry
    let mut producer_signals = Vec::new();
    if let Some(ref prod) = data.pdf_producer {
        producer_signals.push(prod.clone());
    }
    let mut creator_signals = Vec::new();
    if let Some(ref cre) = data.pdf_creator {
        creator_signals.push(cre.clone());
    }

    let fingerprint = crate::cm_registry::InstitutionFingerprint {
        institution: institution.clone(),
        ico: None,
        signals: crate::cm_registry::FingerprintSignals {
            pdf_producer_contains: producer_signals,
            pdf_creator_contains: creator_signals,
            filename_patterns: vec![],
            color_buckets: data.color_buckets.clone(),
            logo_phash: None,
            page_count_range: None,
            size_bucket: None,
        },
        category: doc_type.clone(),
        confidence_boost: 0.45,
        skip_extraction: false,
        contributed_by: 1,
        last_updated: chrono::Utc::now().format("%Y-%m-%d").to_string(),
    };

    let mut local_added = false;
    if let Ok(mut registry) = state.cm_registry.write() {
        registry.add_fingerprint(fingerprint);
        let cache_path = state.app_data_dir.join("cm_registry.json");
        let _ = registry.save_to_file(&cache_path);
        local_added = true;
    }

    info!("Teaching recorded: institution={}, doc_type={}", institution, doc_type);

    Ok(TeachingResult {
        local_fingerprint_added: local_added,
        cm_payload,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// CM contribution upload
// ─────────────────────────────────────────────────────────────────────────────

/// Upload teaching payloads to CM cloud. Requires explicit user action.
/// On failure, queues to cm_pending.json for retry on next launch.
#[tauri::command]
pub async fn upload_to_cm(
    payloads: Vec<CmPayload>,
    state: State<'_, AppState>,
) -> Result<UploadResult, String> {
    if payloads.is_empty() {
        return Ok(UploadResult { uploaded: 0, queued: 0 });
    }

    let app_version = env!("CARGO_PKG_VERSION");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    match client
        .post(CM_CONTRIBUTE_URL)
        .header("X-FileIT-Version", app_version)
        .json(&payloads)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let count = payloads.len() as u32;
            increment_contribution_counter(&state.app_data_dir, count);
            info!("CM contribution: {} fingerprints uploaded", count);
            Ok(UploadResult { uploaded: count, queued: 0 })
        }
        Ok(resp) => {
            warn!("CM upload returned status: {}", resp.status());
            queue_pending_payloads(&state.app_data_dir, &payloads);
            Ok(UploadResult { uploaded: 0, queued: payloads.len() as u32 })
        }
        Err(e) => {
            warn!("CM upload failed: {}", e);
            queue_pending_payloads(&state.app_data_dir, &payloads);
            Ok(UploadResult { uploaded: 0, queued: payloads.len() as u32 })
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Private helpers
// ─────────────────────────────────────────────────────────────────────────────

fn sanitize_pattern_key(key: &str) -> String {
    key.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
        .take(64)
        .collect()
}

fn increment_contribution_counter(app_data_dir: &PathBuf, delta: u32) {
    let prefs_path = app_data_dir.join("user_prefs.json");
    let mut prefs: serde_json::Value = std::fs::read_to_string(&prefs_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let current = prefs.get("cm_contributions").and_then(|v| v.as_u64()).unwrap_or(0);
    prefs["cm_contributions"] = serde_json::json!(current + delta as u64);

    let _ = std::fs::write(&prefs_path, serde_json::to_string_pretty(&prefs).unwrap_or_default());
}

fn queue_pending_payloads(app_data_dir: &PathBuf, payloads: &[CmPayload]) {
    let pending_path = app_data_dir.join("cm_pending.json");
    let mut existing: Vec<CmPayload> = std::fs::read_to_string(&pending_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    existing.extend_from_slice(payloads);
    let _ = std::fs::write(
        &pending_path,
        serde_json::to_string_pretty(&existing).unwrap_or_default(),
    );
}
