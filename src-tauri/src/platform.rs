// src-tauri/src/platform.rs
//
// Small OS-specific helpers shared by commands: the default "organised"
// folder, revealing a path in the file manager, and what the UI needs to
// know about the platform (separator, file-manager name).

use serde::Serialize;
use std::path::{Path, PathBuf};

/// What the frontend needs to render platform-correct paths and wording.
#[derive(Debug, Clone, Serialize)]
pub struct PlatformInfo {
    /// "windows" | "macos" | "linux"
    pub os: &'static str,
    /// Path separator used when the UI joins paths ("\\" or "/").
    pub path_separator: &'static str,
    /// Default target folder for organised files.
    pub default_target_path: String,
    /// Czech name of the system file manager ("Průzkumník", "Finder").
    pub file_manager: &'static str,
    /// Home folder, if known (used for display only).
    pub home_dir: Option<String>,
}

pub fn home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let var = "USERPROFILE";
    #[cfg(not(target_os = "windows"))]
    let var = "HOME";
    std::env::var_os(var).map(PathBuf::from).filter(|p| !p.as_os_str().is_empty())
}

/// Default folder that organised files go to.
/// Windows: C:\ReOrganized (unchanged). macOS / Linux: ~/FileIT/Organized.
pub fn default_target_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(r"C:\ReOrganized")
    }
    #[cfg(not(target_os = "windows"))]
    {
        unix_default_target(home_dir().as_deref())
    }
}

#[cfg_attr(target_os = "windows", allow(dead_code))]
fn unix_default_target(home: Option<&Path>) -> PathBuf {
    home.unwrap_or_else(|| Path::new("/tmp"))
        .join("FileIT")
        .join("Organized")
}

pub fn info() -> PlatformInfo {
    let (os, path_separator, file_manager) = if cfg!(target_os = "windows") {
        ("windows", "\\", "Průzkumník")
    } else if cfg!(target_os = "macos") {
        ("macos", "/", "Finder")
    } else {
        ("linux", "/", "správce souborů")
    };
    PlatformInfo {
        os,
        path_separator,
        default_target_path: default_target_dir().to_string_lossy().into_owned(),
        file_manager,
        home_dir: home_dir().map(|p| p.to_string_lossy().into_owned()),
    }
}

/// Show `path` in the system file manager.
/// Windows: `explorer <path>` (unchanged). macOS: `open -R <path>` (Finder,
/// item selected). Linux: `xdg-open <path>`.
pub fn reveal_in_file_manager(path: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer").arg(path).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg("-R").arg(path).spawn()?;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_default_target_is_under_home() {
        assert_eq!(
            unix_default_target(Some(Path::new("/Users/jana"))),
            PathBuf::from("/Users/jana/FileIT/Organized")
        );
        assert_eq!(unix_default_target(None), PathBuf::from("/tmp/FileIT/Organized"));
    }

    #[test]
    fn info_is_consistent() {
        let i = info();
        assert!(["windows", "macos", "linux"].contains(&i.os));
        assert!(!i.default_target_path.is_empty());
        if i.os == "windows" {
            assert_eq!(i.default_target_path, r"C:\ReOrganized");
        } else {
            assert!(i.default_target_path.ends_with("FileIT/Organized"));
        }
    }
}
