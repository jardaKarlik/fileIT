// src-tauri/src/ocr/mod.rs
//
// Indra OCR sidecar integration.
//
// FileIT never shells out to OCR on its own: text extraction during a scan
// stays cheap and offline. This module is the single entry point for the
// *explicit* "Spustit OCR" action in the Dashboard.
//
// Design rules:
//   1. The sidecar is optional. When it is missing we return a graceful
//      "unavailable" response and the UI says "OCR není nainstalován".
//      Nothing here panics on a missing binary.
//   2. Results are content-hashed and cached (see ocr_store.rs), so a re-run
//      only pays for files that are new or changed.
//   3. Progress is emitted so the UI can show a live counter even when the
//      batch is long.
//
// Environment variables:
//   INDRA_OCR_COMMAND  sidecar binary, default "indra-ocr"
//   INDRA_OCR_MODELS   model directory, passed through as --models
//   INDRA_OCR_CORES    worker count, default 4

use crate::ocr_store;
use crate::types::{OcrAvailability, OcrBatchResult, OcrProgress, OcrResult};
use log::{info, warn};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// Event name the frontend listens on for live batch progress.
pub const PROGRESS_EVENT: &str = "ocr_progress";

const DEFAULT_COMMAND: &str = "indra-ocr";
const DEFAULT_CORES: usize = 4;

fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.trim().is_empty())
}

fn ocr_command() -> String {
    env_non_empty("INDRA_OCR_COMMAND").unwrap_or_else(|| DEFAULT_COMMAND.to_string())
}

fn worker_count() -> usize {
    env_non_empty("INDRA_OCR_CORES")
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_CORES)
        .min(16)
}

/// A command is "installed" when it resolves on PATH (or is an absolute path
/// that exists). We deliberately do not probe with --help: some builds of the
/// sidecar block on stdin when invoked with no arguments.
fn command_exists(command: &str) -> bool {
    let p = Path::new(command);

    // Absolute or relative path — check the file directly.
    if p.components().count() > 1 || command.contains('/') || command.contains('\\') {
        return p.is_file();
    }

    // Bare name — search PATH the same way the OS would.
    if let Some(paths) = env_non_empty("PATH") {
        for dir in std::env::split_paths(&paths) {
            for ext in ["", ".exe", ".cmd", ".bat"] {
                let candidate = dir.join(format!("{}{}", command, ext));
                if candidate.is_file() {
                    return true;
                }
            }
        }
    }
    false
}

/// Capability probe for the UI. Never fails — always returns a usable answer.
pub fn check(app_data_dir: &Path) -> OcrAvailability {
    let command = ocr_command();
    let installed = command_exists(&command);

    let message = if installed {
        None
    } else {
        Some(format!(
            "Příkaz „{}“ nebyl nalezen v PATH. Nainstalujte Indra OCR sidecar, nebo nastavte INDRA_OCR_COMMAND.",
            command
        ))
    };

    OcrAvailability {
        installed,
        command: if installed { Some(command) } else { None },
        models_dir: env_non_empty("INDRA_OCR_MODELS"),
        message,
        cached_count: ocr_store::count(app_data_dir),
    }
}

/// SHA-256 of a file, computed streaming. Returns None when the file cannot be
/// read (deleted, cloud placeholder, permissions) so the caller can fail just
/// that one entry instead of the whole batch.
fn hash_file(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];

    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(_) => return None,
        }
    }

    Some(hex::encode(hasher.finalize()))
}

/// Run the sidecar on one file and parse its JSON output.
///
/// The sidecar is invoked as:
///   <command> --input <path> --format json [--models <dir>]
///
/// Output is expected to be the `OcrResult` JSON on stdout. A non-zero exit or
/// unparseable stdout is an error for this file only.
fn run_sidecar(command: &str, path: &Path) -> Result<OcrResult, String> {
    let mut cmd = Command::new(command);
    cmd.arg("--input").arg(path).arg("--format json");

    if let Some(models) = env_non_empty("INDRA_OCR_MODELS") {
        cmd.arg("--models").arg(models);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("sidecar nelze spustit ({}): {}", command, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "sidecar vrátil chybu ({}): {}",
            output.status,
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut result: OcrResult = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("neplatné JSON z OCR sidecaru: {}", e))?;

    // The sidecar may omit identity fields — fill them from what we know so the
    // cache entry is self-describing.
    if result.path.is_empty() {
        result.path = path.to_string_lossy().to_string();
    }
    if result.sha256.is_none() {
        result.sha256 = hash_file(path);
    }
    if result.confidence_avg.is_none() {
        result.confidence_avg = mean_confidence(&result);
    }

    Ok(result)
}

/// Average of the per-block confidences that are actually present.
fn mean_confidence(result: &OcrResult) -> Option<f32> {
    let vals: Vec<f32> = result
        .layout_blocks
        .iter()
        .filter_map(|b| b.confidence)
        .collect();

    if vals.is_empty() {
        return None;
    }
    Some(vals.iter().sum::<f32>() / vals.len() as f32)
}

/// Process a batch of files.
///
/// Returns the cache-hit / processed / failed counts. When the sidecar is not
/// installed the whole batch is skipped and `skipped_unavailable` is true —
/// the caller surfaces that instead of treating it as a failure.
pub async fn run_batch(
    app: &AppHandle,
    app_data_dir: &Path,
    paths: Vec<String>,
) -> OcrBatchResult {
    let mut result = OcrBatchResult {
        processed: 0,
        from_cache: 0,
        failed: 0,
        skipped_unavailable: false,
        errors: Vec::new(),
    };

    if paths.is_empty() {
        return result;
    }

    let command = ocr_command();
    if !command_exists(&command) {
        info!("OCR batch skipped — sidecar '{}' not installed", command);
        result.skipped_unavailable = true;
        return result;
    }

    // De-duplicate while preserving order: the same file selected twice must not
    // be OCR'd twice.
    let mut seen = std::collections::HashSet::new();
    let unique: Vec<PathBuf> = paths
        .into_iter()
        .filter(|p| seen.insert(p.clone()))
        .map(PathBuf::from)
        .collect();

    let total = unique.len() as u32;
    let workers = worker_count();

    info!(
        "OCR batch starting: {} files, {} workers, command '{}'",
        total, workers, command
    );

    // Shared counters across the blocking workers.
    let done = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let failed = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let from_cache = Arc::new(std::sync::atomic::AtomicU32::new(0));

    let app_data_dir = app_data_dir.to_path_buf();
    let command = Arc::new(command);

    // Blocking work is pushed onto the Tokio blocking pool so the async command
    // stays responsive; the semaphore caps real concurrency.
    let semaphore = Arc::new(tokio::sync::Semaphore::new(workers));

    let mut handles = Vec::new();

    for path in unique {
        // acquire_owned returns a future; awaiting it inside this async loop
        // gives real backpressure — at most `workers` sidecars at once.
        let permit = match semaphore.clone().acquire_owned().await {
            Ok(p) => p,
            Err(_) => break,
        };

        let app = app.clone();
        let app_data_dir = app_data_dir.clone();
        let command = command.clone();
        let done = done.clone();
        let failed = failed.clone();
        let from_cache = from_cache.clone();

        handles.push(tokio::spawn(async move {
            // Held for the duration of the sidecar call.
            let _permit = permit;

            let path_display = path.to_string_lossy().to_string();

            // Result<OcrResult, String>: Err means "this file failed", so the
            // batch can continue with the next entry.
            let outcome = tokio::task::spawn_blocking(move || -> Result<OcrResult, String> {
                let sha = hash_file(&path);
                let cached = sha
                    .as_deref()
                    .and_then(|s| ocr_store::read(&app_data_dir, s));

                if let Some(hit) = cached {
                    return Ok(hit);
                }

                let result = run_sidecar(&command, &path)?;
                ocr_store::write(&app_data_dir, &result);
                Ok(result)
            })
            .await;

            match outcome {
                // Cache hit — the sidecar was never invoked.
                Ok(Ok(_)) => {
                    from_cache.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                // Sidecar ran and succeeded.
                Ok(Err(_)) => {}                // The closure reported an error for this file.
                Err(e) => {
                    warn!("OCR failed for {}: {}", path_display, e);
                    failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    // Errors are surfaced in the batch result, not the progress
                    // event, so the counter stays cheap to render.
                    result_lock().push(format!("{}: {}", path_display, e));
                }
            }

            let d = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            let f = failed.load(std::sync::atomic::Ordering::Relaxed);

            let _ = app.emit(
                PROGRESS_EVENT,
                OcrProgress {
                    done: d,
                    total,
                    current_path: path_display,
                    failed_count: f,
                },
            );
        }));
    }

    for h in handles {
        let _ = h.await;
    }

    result.processed = done.load(std::sync::atomic::Ordering::Relaxed);
    result.from_cache = from_cache.load(std::sync::atomic::Ordering::Relaxed);
    result.failed = failed.load(std::sync::atomic::Ordering::Relaxed);
    result.errors = result_lock().clone();
    result.errors.sort();

    info!(
        "OCR batch done: {} processed ({} from cache), {} failed",
        result.processed, result.from_cache, result.failed
    );

    result
}

// A tiny global sink for per-file errors. Using a Mutex<Vec<String>> here keeps
// the worker closures simple without threading a channel through every spawn.
// A poisoned lock is recovered rather than propagated — losing the error list
// is far less harmful than panicking the whole command.
fn result_lock() -> std::sync::MutexGuard<'static, Vec<String>> {
    use std::sync::Mutex;
    static CELL: std::sync::OnceLock<Mutex<Vec<String>>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Tauri command — capability probe.
#[tauri::command]
pub async fn check_ocr(
    state: State<'_, crate::commands::AppState>,
) -> Result<OcrAvailability, String> {
    Ok(check(&state.app_data_dir))
}

/// Tauri command — run the batch over the given paths.
#[tauri::command]
pub async fn run_ocr(
    app: AppHandle,
    state: State<'_, crate::commands::AppState>,
    paths: Vec<String>,
) -> Result<OcrBatchResult, String> {
    // Clear any leftovers from a previous run so this batch's error list is exact.
    result_lock().clear();
    Ok(run_batch(&app, &state.app_data_dir, paths).await)
}

/// Tauri command — read a cached result for one path. Ok(None) on a miss.
#[tauri::command]
pub async fn get_ocr(
    state: State<'_, crate::commands::AppState>,
    path: String,
) -> Result<Option<OcrResult>, String> {
    let p = PathBuf::from(&path);
    let sha = tokio::task::spawn_blocking(move || hash_file(&p))
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Soubor nelze přečíst: {}", path))?;

    Ok(ocr_store::read(&state.app_data_dir, &sha))
}
