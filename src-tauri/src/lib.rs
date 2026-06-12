// src-tauri/src/lib.rs
//
// FileIT Tauri application library entry point.
// Initialises all plugins, loads resources, registers IPC commands,
// and manages the AppState.

mod backup;
mod classifier;
mod cm_registry;
mod commands;
mod metadata_reader;
mod ocr;
mod restructure;
mod scanner;
mod types;

use commands::AppState;
use std::sync::{Mutex, RwLock};
use tauri::Manager;

const CM_SEED_JSON: &str = include_str!("../resources/cm_registry_seed.json");

pub fn run() {
    // Load the institution dictionary — bundled as a resource
    let dictionary_json = include_str!("../resources/institution_dictionary.json");

    let classifier = classifier::Classifier::new(dictionary_json)
        .expect("Failed to initialise classifier — check institution_dictionary.json");

    tauri::Builder::default()
        // ── Plugins ──────────────────────────────────────────────────────────
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Debug)
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir { file_name: Some("fileit".to_string()) },
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .build(),
        )
        // ── App state ─────────────────────────────────────────────────────────
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Could not resolve AppData directory");

            std::fs::create_dir_all(&app_data_dir)
                .expect("Could not create AppData directory");

            // Run startup cleanup (expired backups, auto-confirm checks)
            let _ = backup::cleanup_expired_backups(&app_data_dir);

            // Load CM registry: prefer cached version in AppData, fall back to seed
            let cache_path = app_data_dir.join("cm_registry.json");
            let registry = if cache_path.exists() {
                cm_registry::CmRegistry::load_from_file(&cache_path)
                    .unwrap_or_else(|e| {
                        log::warn!("Failed to load cached CM registry ({}), using seed", e);
                        serde_json::from_str(CM_SEED_JSON)
                            .expect("CM seed JSON must be valid")
                    })
            } else {
                let seed: cm_registry::CmRegistry = serde_json::from_str(CM_SEED_JSON)
                    .expect("CM seed JSON must be valid");
                // Write seed to cache so future runs can update it
                let _ = seed.save_to_file(&cache_path);
                seed
            };

            log::info!(
                "CM registry: version {}, {} fingerprints loaded",
                registry.version,
                registry.fingerprints.len()
            );

            app.manage(AppState {
                session: Mutex::new(types::SessionState::new()),
                classifier,
                app_data_dir,
                cm_registry: RwLock::new(registry),
            });

            Ok(())
        })
        // ── IPC commands ──────────────────────────────────────────────────────
        .invoke_handler(tauri::generate_handler![
            commands::app_init,
            commands::detect_clouds,
            commands::start_scan,
            commands::build_preview,
            commands::run_restructure,
            commands::create_standalone_backup,
            commands::confirm_restructure,
            commands::restore_restructure,
            commands::pick_folder,
            commands::open_in_explorer,
            commands::get_onedrive_path,
            commands::check_cm_update,
            commands::group_unclassified_patterns,
            commands::submit_teaching,
            commands::upload_to_cm,
        ])
        .run(tauri::generate_context!())
        .expect("Error running FileIT application");
}
