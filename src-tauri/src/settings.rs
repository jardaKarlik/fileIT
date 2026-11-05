// src-tauri/src/settings.rs
//
// FileIT settings file: <app data dir>/settings.json.
//
// The cross-platform way to configure the Indra OCR sidecar. On Windows the
// HKCU\Environment registry fallback still works; macOS has no registry, and
// apps started from Finder / the Dock do not see variables exported in
// ~/.zshrc, so a file is the reliable place.
//
//   macOS:   ~/Library/Application Support/cz.fileit.desktop/settings.json
//            (macOS bundle id, see tauri.macos.conf.json)
//   Windows: %APPDATA%\cz.fileit.app\settings.json
//
// {
//   "indra_ocr_command": "/Users/jana/Library/Python/3.12/bin/indra-ocr",
//   "indra_ocr_models": null,
//   "indra_ocr_cores": null
// }
//
// Lookup order for each value (see ocr::env_non_empty):
//   environment variable → this file → (Windows) HKCU\Environment → default.
// The file is re-read on every lookup, so edits apply without a restart.

use log::{info, warn};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const FILE_NAME: &str = "settings.json";

static SETTINGS_PATH: OnceCell<PathBuf> = OnceCell::new();

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    /// Full path to the indra-ocr binary, or a command name on PATH.
    #[serde(default)]
    pub indra_ocr_command: Option<String>,
    /// Model directory passed to the sidecar as --model-dir.
    #[serde(default)]
    pub indra_ocr_models: Option<String>,
    /// Number of parallel sidecar workers.
    #[serde(default)]
    pub indra_ocr_cores: Option<usize>,
}

impl Settings {
    /// Value for an environment-variable style key, if set and non-empty.
    pub fn value(&self, key: &str) -> Option<String> {
        let v = match key {
            "INDRA_OCR_COMMAND" => self.indra_ocr_command.clone(),
            "INDRA_OCR_MODELS" => self.indra_ocr_models.clone(),
            "INDRA_OCR_CORES" => self.indra_ocr_cores.map(|n| n.to_string()),
            _ => None,
        }?;
        let v = v.trim().to_string();
        if v.is_empty() { None } else { Some(v) }
    }
}

/// Remember where the settings file lives and create an empty template the
/// first time, so users can find and edit it.
pub fn init(app_data_dir: &Path) {
    let path = app_data_dir.join(FILE_NAME);
    if !path.exists() {
        match serde_json::to_string_pretty(&Settings::default()) {
            Ok(json) => match std::fs::write(&path, json) {
                Ok(()) => info!("Created settings file {:?}", path),
                Err(e) => warn!("Cannot create settings file {:?}: {}", path, e),
            },
            Err(e) => warn!("Cannot serialise default settings: {}", e),
        }
    }
    let _ = SETTINGS_PATH.set(path);
}

/// Path of the settings file, once `init` has run.
pub fn path() -> Option<&'static Path> {
    SETTINGS_PATH.get().map(|p| p.as_path())
}

/// Read a settings file. Missing or invalid → defaults (logged).
pub fn load_from(path: &Path) -> Settings {
    match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|e| {
            warn!("Invalid settings file {:?}: {}", path, e);
            Settings::default()
        }),
        Err(_) => Settings::default(),
    }
}

/// Current settings (re-read from disk).
pub fn load() -> Settings {
    path().map(load_from).unwrap_or_default()
}

/// Value for an environment-variable style key from the settings file.
pub fn value(key: &str) -> Option<String> {
    load().value(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_values_and_ignores_blanks() {
        let dir = std::env::temp_dir().join(format!("fileit-settings-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(FILE_NAME);
        std::fs::write(
            &p,
            r#"{"indra_ocr_command": " /opt/homebrew/bin/indra-ocr ", "indra_ocr_models": "  ", "indra_ocr_cores": 2}"#,
        )
        .unwrap();
        let s = load_from(&p);
        assert_eq!(s.value("INDRA_OCR_COMMAND").as_deref(), Some("/opt/homebrew/bin/indra-ocr"));
        assert_eq!(s.value("INDRA_OCR_MODELS"), None);
        assert_eq!(s.value("INDRA_OCR_CORES").as_deref(), Some("2"));
        assert_eq!(s.value("PATH"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_or_broken_file_gives_defaults() {
        let dir = std::env::temp_dir().join(format!("fileit-settings-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(load_from(&dir.join("nope.json")), Settings::default());
        let p = dir.join(FILE_NAME);
        std::fs::write(&p, "{ not json").unwrap();
        assert_eq!(load_from(&p), Settings::default());
        // Unknown keys are tolerated (forward compatible)
        std::fs::write(&p, r#"{"future_option": true}"#).unwrap();
        assert_eq!(load_from(&p), Settings::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
