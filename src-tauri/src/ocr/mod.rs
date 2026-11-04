// src-tauri/src/ocr/mod.rs
//
// Indra OCR integration (https://github.com/datalab-to/indra).
//
// Indra OCR is a small CPU-only OCR model distributed as a Python package
// (`pip install indra-ocr`) with an `indra-ocr` command-line entry point.
// FileIT calls it as an external process so the Rust binary stays free of
// Python / ONNX Runtime dependencies.
//
// The executable is resolved in this order:
//   1. FILEIT_INDRA_OCR environment variable (full path to the executable)
//   2. A sidecar next to fileit.exe (`indra-ocr.exe` / `indra-ocr`), which is
//      where Tauri places `bundle.externalBin` binaries in an installed build
//   3. `indra-ocr` on PATH
//
// Optional: FILEIT_INDRA_MODEL_DIR points Indra at pre-downloaded models so
// the first run does not need network access (passed as `--model-dir`).

use log::{info, warn};
use once_cell::sync::OnceCell;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

/// Pages read per document — matches the Windows OCR fallback. The first
/// pages carry the institution, customer and document type.
const MAX_PAGES: u32 = 5;

/// Hard limit for a single Indra invocation (model load + up to MAX_PAGES).
const OCR_TIMEOUT: Duration = Duration::from_secs(120);

/// Availability of the Indra OCR engine, reported to the UI.
#[derive(Debug, Clone, Serialize)]
pub struct OcrStatus {
    /// True if an `indra-ocr` executable was found and answered `--help`.
    pub indra_available: bool,
    /// Resolved executable path, if found.
    pub indra_path: Option<String>,
    /// True if the Windows built-in OCR fallback can be used.
    pub windows_fallback: bool,
}

static INDRA_EXE: OnceCell<Option<PathBuf>> = OnceCell::new();

/// Locate the Indra OCR executable (cached after the first lookup).
pub fn indra_executable() -> Option<PathBuf> {
    INDRA_EXE.get_or_init(resolve_indra_executable).clone()
}

fn resolve_indra_executable() -> Option<PathBuf> {
    let exe_name = if cfg!(windows) { "indra-ocr.exe" } else { "indra-ocr" };

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("FILEIT_INDRA_OCR") {
        if !p.trim().is_empty() {
            candidates.push(PathBuf::from(p.trim()));
        }
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        candidates.push(dir.join(exe_name));
        candidates.push(dir.join("indra-ocr").join(exe_name));
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            candidates.push(dir.join(exe_name));
        }
    }

    for c in candidates {
        if c.is_file() && probe(&c) {
            info!("Indra OCR found at {:?}", c);
            return Some(c);
        }
    }
    info!("Indra OCR not found — OCR will fall back to Windows OCR");
    None
}

/// Check that the executable actually starts.
fn probe(exe: &Path) -> bool {
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--help").stdout(Stdio::null()).stderr(Stdio::null());
    hide_console_std(&mut cmd);
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

pub fn status() -> OcrStatus {
    let exe = indra_executable();
    OcrStatus {
        indra_available: exe.is_some(),
        indra_path: exe.map(|p| p.to_string_lossy().into_owned()),
        windows_fallback: cfg!(target_os = "windows"),
    }
}

/// Run Indra OCR on a PDF or image and return plain text (one line per
/// detected line). Returns None if Indra is unavailable, fails, times out
/// or produces (almost) no text.
pub async fn indra_ocr(path: &Path) -> Option<String> {
    let exe = indra_executable()?;

    let cores = std::thread::available_parallelism()
        .map(|n| n.get().min(4))
        .unwrap_or(2);

    let mut cmd = tokio::process::Command::new(&exe);
    cmd.arg(path)
        .args(["--format", "text"])
        .args(["--max-pages", &MAX_PAGES.to_string()])
        .args(["--cores", &cores.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Ok(model_dir) = std::env::var("FILEIT_INDRA_MODEL_DIR") {
        if !model_dir.trim().is_empty() {
            cmd.args(["--model-dir", model_dir.trim()]);
        }
    }
    // Indra is a Python program — force UTF-8 so Czech diacritics survive.
    cmd.env("PYTHONIOENCODING", "utf-8").env("PYTHONUTF8", "1");
    hide_console_tokio(&mut cmd);

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            warn!("Indra OCR failed to start for {:?}: {}", path, e);
            return None;
        }
    };

    let output = match tokio::time::timeout(OCR_TIMEOUT, child.wait_with_output()).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => {
            warn!("Indra OCR I/O error for {:?}: {}", path, e);
            return None;
        }
        Err(_) => {
            // kill_on_drop terminates the process
            warn!("Indra OCR timed out after {:?} for {:?}", OCR_TIMEOUT, path);
            return None;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(
            "Indra OCR exited with {} for {:?}: {}",
            output.status,
            path,
            stderr.lines().last().unwrap_or("")
        );
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.split_whitespace().count() >= 3 {
        Some(text)
    } else {
        None
    }
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
fn hide_console_std(cmd: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(CREATE_NO_WINDOW);
}
#[cfg(not(windows))]
fn hide_console_std(_cmd: &mut std::process::Command) {}

#[cfg(windows)]
fn hide_console_tokio(cmd: &mut tokio::process::Command) {
    cmd.creation_flags(CREATE_NO_WINDOW);
}
#[cfg(not(windows))]
fn hide_console_tokio(_cmd: &mut tokio::process::Command) {}
