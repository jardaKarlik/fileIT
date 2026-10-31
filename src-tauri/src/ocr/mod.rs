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
use crate::types::{OcrAvailability, OcrBatchResult, OcrBlock, OcrProgress, OcrResult};
use log::{info, warn};
use serde_json::Value as Json;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::time::Duration;

/// Event name the frontend listens on for live batch progress.
pub const PROGRESS_EVENT: &str = "ocr_progress";

const DEFAULT_COMMAND: &str = "indra-ocr";
const DEFAULT_CORES: usize = 4;

/// Wall-clock budget for one CLI invocation. A hung sidecar must not freeze
/// the whole batch; a timeout fails that one file and the batch continues.
const SIDECAR_TIMEOUT: Duration = Duration::from_secs(300);

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
///   <command> <file> --format json --cores 1 [--models <dir>]
///
/// `--cores 1` pins a single OCR worker per invocation: batch-level parallelism
/// is ours (the semaphore), so N concurrent sidecars must not each spin up N
/// threads. Each argument is passed separately — `--format json` split as one
/// string is a classic Windows quoting bug and would fail arg parsing.
///
/// Output is expected to be the Indra JSON on stdout. A non-zero exit,
/// unparseable stdout, or a timeout is an error for this file only.
fn run_sidecar(command: &str, path: &Path) -> Result<OcrResult, String> {
    let output = spawn_sidecar_json(command, path, SIDECAR_TIMEOUT)?;

    let json: Json = serde_json::from_slice(&output)
        .map_err(|e| format!("neplatné JSON z OCR sidecaru: {}", e))?;

    let mut result = indra_json_to_ocr_result(&json, path)?;

    // The sidecar may omit identity fields — fill them from what we know so the
    // cache entry is self-describing.
    if result.path.is_empty() {
        result.path = path.to_string_lossy().to_string();
    }
    if result.sha256.as_deref().map_or(true, |s| s.is_empty()) {
        result.sha256 = hash_file(path);
    }
    if result.confidence_avg.is_none() {
        result.confidence_avg = mean_confidence(&result);
    }

    Ok(result)
}

/// Invoke the sidecar and return its stdout bytes, enforcing a wall-clock
/// timeout. Fully synchronous: it runs inside `spawn_blocking`, so a Tokio
/// timeout cannot be awaited here — we poll `try_wait` against a deadline and
/// kill the child when it expires. Stdout/stderr are drained on reader
/// threads so a chatty child can never deadlock a full pipe.
fn spawn_sidecar_json(
    command: &str,
    path: &Path,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    use std::io::Read;
    use std::time::Instant;

    let mut cmd = Command::new(command);
    // Positional first: `indra-ocr <file> --format json --cores 1`.
    cmd.arg(path);
    cmd.arg("--format");
    cmd.arg("json");
    // One OCR worker per invocation; batch parallelism is the semaphore's job.
    cmd.arg("--cores");
    cmd.arg("1");

    if let Some(models) = env_non_empty("INDRA_OCR_MODELS") {
        cmd.arg("--models").arg(models);
    }

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("sidecar nelze spustit ({}): {}", command, e))?;

    // Drain the pipes on threads — the child can block writing when a pipe
    // fills and nobody reads it.
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let stdout_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut s) = stdout_pipe {
            let _ = s.read_to_end(&mut buf);
        }
        buf
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut s) = stderr_pipe {
            let _ = s.read_to_end(&mut buf);
        }
        buf
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                return Err(format!("sidecar nelze sledovat: {}", e));
            }
        }
    };

    // Join the readers so no bytes are lost when we did get a real exit.
    let stdout = stdout_reader.join().unwrap_or_default();
    let _stderr = stderr_reader.join().unwrap_or_default();

    match status {
        Some(status) if status.success() => Ok(stdout),
        Some(status) => Err(format!(
            "sidecar vrátil chybu ({}): {}",
            status,
            String::from_utf8_lossy(&_stderr).trim()
        )),
        None => Err(format!(
            "sidecar překročil časový limit {} s",
            timeout.as_secs()
        )),
    }
}

// ─────────────────────────── Indra JSON → OcrResult ───────────────────────────

/// Convert an Indra CLI JSON document into our `OcrResult`.
///
/// The mapping is deliberately tolerant: Indra emits `pages[].texts[].lines[]`
/// (and optional `blocks[]` / layout labels). Everything optional-degrades —
/// a missing field must produce a valid `OcrResult`, never a parse failure.
///
/// Shape handled:
///   { "pages": [ { "texts": [ { "lines": [ { "text", "confidence",
///       "bbox": [x0,y0,x1,y1] } ] } ] } ], "language"?, … }
///
/// `is_scan` is true when the sidecar reports image-based recognition and
/// false when it read a digital text layer (we pass the flag through when the
/// sidecar provides it, else default true — OCR output implies scanning work).
fn indra_json_to_ocr_result(json: &Json, path: &Path) -> Result<OcrResult, String> {
    let path_str = path.to_string_lossy().to_string();

    let mut full_text = String::new();
    let mut blocks: Vec<OcrBlock> = Vec::new();
    let mut page_count: u32 = 0;
    let mut language: Option<String> = None;

    let pages = json
        .get("pages")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();

    if pages.is_empty() {
        // Some CLI builds emit a flat shape: { "text": "...", "lines": [...] }
        let flat = json.get("text").and_then(Json::as_str);
        if let Some(t) = flat {
            full_text = t.to_string();
        }
        language = json
            .get("language")
            .or_else(|| json.get("lang"))
            .and_then(Json::as_str)
            .map(str::to_string);
        page_count = 1;
    }

    for (page_idx, page) in pages.iter().enumerate() {
        let page_no = (page_idx + 1) as u32;
        page_count = page_count.max(page_no);

        if language.is_none() {
            language = page
                .get("language")
                .or_else(|| page.get("lang"))
                .and_then(Json::as_str)
                .map(str::to_string);
        }

        // Collect text chunks: texts[].lines[] and any bare text fields.
        for text_obj in page
            .get("texts")
            .and_then(Json::as_array)
            .map(|a| a.as_slice())
            .unwrap_or(&[])
        {
            if let Some(lines) = text_obj.get("lines").and_then(Json::as_array) {
                for line in lines {
                    let text = line
                        .get("text")
                        .and_then(Json::as_str)
                        .unwrap_or("")
                        .to_string();
                    let confidence = line
                        .get("confidence")
                        .and_then(|c| c.as_f64())
                        .map(|c| clamp_unit(c as f32));
                    let box_ = line
                        .get("bbox")
                        .or_else(|| line.get("box"))
                        .and_then(bbox_to_f32_4);

                    blocks.push(OcrBlock {
                        page: page_no,
                        label: "line".to_string(),
                        text,
                        confidence,
                        box_,
                    });
                }
                continue;
            }

            // No lines: fall back to the text's own strings/lines.
            if let Some(t) = text_obj.get("text").and_then(Json::as_str) {
                full_text.push_str(t);
                full_text.push(' ');
                blocks.push(OcrBlock {
                    page: page_no,
                    label: "text".to_string(),
                    text: t.to_string(),
                    confidence: None,
                    box_: None,
                });
            }
        }

        // Layout blocks (headings, tables) dumped onto the page object itself.
        if let Some(raw_blocks) = page.get("blocks").and_then(Json::as_array) {
            for rb in raw_blocks {
                let label = rb
                    .get("label")
                    .or_else(|| rb.get("type"))
                    .and_then(Json::as_str)
                    .unwrap_or("block")
                    .to_string();
                let text = rb
                    .get("text")
                    .and_then(Json::as_str)
                    .unwrap_or("")
                    .to_string();
                let confidence = rb
                    .get("confidence")
                    .and_then(Json::as_f64)
                    .map(|c| clamp_unit(c as f32));
                let box_ = rb
                    .get("bbox")
                    .or_else(|| rb.get("box"))
                    .and_then(bbox_to_f32_4);

                blocks.push(OcrBlock {
                    page: page_no,
                    label,
                    text,
                    confidence,
                    box_,
                });
            }
        }

        // Append all page text, in reading order: lines first, then fallback texts.
        // We do a second pass so full_text stays coherent when both shapes exist.
        let line_text = page_text(page);
        if !line_text.is_empty() {
            full_text.push_str(&line_text);
            full_text.push('\n');
        }
    }

    if full_text.is_empty() && blocks.is_empty() {
        return Err("sidecar vrátil prázdný výsledek".to_string());
    }

    let is_scan = json
        .get("is_scan")
        .and_then(Json::as_bool)
        .unwrap_or(true);

    Ok(OcrResult {
        path: path_str,
        sha256: None,
        full_text: full_text.trim().to_string(),
        language,
        confidence_avg: None,
        page_count: page_count.max(1),
        is_scan,
        layout_blocks: blocks,
    })
}

/// Best-effort page text: texts[].lines[].text joined with newlines.
fn page_text(page: &Json) -> String {
    let mut out = String::new();
    for text_obj in page
        .get("texts")
        .and_then(Json::as_array)
        .map(|a| a.as_slice())
        .unwrap_or(&[])
    {
        if let Some(lines) = text_obj.get("lines").and_then(Json::as_array) {
            for line in lines {
                if let Some(t) = line.get("text").and_then(Json::as_str) {
                    out.push_str(t);
                    out.push('\n');
                }
            }
        } else if let Some(t) = text_obj.get("text").and_then(Json::as_str) {
            out.push_str(t);
            out.push(' ');
        }
    }
    out.trim_end().to_string()
}

/// Parse an Indra bbox into a 4-tuple, tolerating ints, floats and junk.
fn bbox_to_f32_4(v: &Json) -> Option<[f32; 4]> {
    let arr = v.as_array()?;
    if arr.len() != 4 {
        return None;
    }
    let mut out = [0f32; 4];
    for (slot, item) in out.iter_mut().zip(arr) {
        *slot = item.as_f64()? as f32;
        if !slot.is_finite() {
            return None;
        }
    }
    Some(out)
}

/// Clamp a confidence value into 0..1. The fixture and live engine both emit
/// sane values, but anything outside must be normalised, not believed.
fn clamp_unit(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
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

// ─────────────────────────── batch runner ───────────────────────────

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
    if paths.is_empty() {
        return empty_result();
    }

    let command = ocr_command();
    if !command_exists(&command) {
        info!("OCR batch skipped — sidecar '{}' not installed", command);
        return OcrBatchResult {
            skipped_unavailable: true,
            ..empty_result()
        };
    }

    // De-duplicate while preserving order: the same file selected twice must not
    // be OCR'd twice.
    let mut seen = std::collections::HashSet::new();
    let unique: Vec<PathBuf> = paths
        .into_iter()
        .filter(|p| seen.insert(p.clone()))
        .map(PathBuf::from)
        .collect();

    let state = Arc::new(BatchState {
        done: std::sync::atomic::AtomicU32::new(0),
        failed: std::sync::atomic::AtomicU32::new(0),
        from_cache: std::sync::atomic::AtomicU32::new(0),
        processed: std::sync::atomic::AtomicU32::new(0),
        errors: std::sync::Mutex::new(Vec::new()),
    });

    let total = unique.len() as u32;
    let workers = worker_count();

    info!(
        "OCR batch starting: {} files, {} workers, command '{}'",
        total, workers, command
    );

    // Blocking work is pushed onto the Tokio blocking pool so the async command
    // stays responsive; the semaphore caps real concurrency.
    let semaphore = Arc::new(tokio::sync::Semaphore::new(workers));

    let app_data_dir = app_data_dir.to_path_buf();
    let command = Arc::new(command);

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
        let state = state.clone();

        handles.push(tokio::spawn(async move {
            // Held for the duration of the sidecar call.
            let _permit = permit;

            let path_display = path.to_string_lossy().to_string();

            // (cache_hit, failed, error) triple: exactly one of cache_hit/success
            // applies, and on failure the error text is pushed into the batch.
            let outcome = tokio::task::spawn_blocking({
                let path = path.clone();
                let app_data_dir = app_data_dir.clone();
                let command = command.clone();
                move || -> Result<bool, String> {
                    let sha = hash_file(&path);
                    let cached = sha
                        .as_deref()
                        .and_then(|s| ocr_store::read(&app_data_dir, s));

                    if let Some(_hit) = cached {
                        return Ok(true); // cache hit
                    }

                    let mut result = run_sidecar(&command, &path)?;
                    if result.sha256.is_none() {
                        result.sha256 = hash_file(&path);
                    }
                    ocr_store::write(&app_data_dir, &result);
                    Ok(false) // processed fresh
                }
            })
            .await;

            match outcome {
                Ok(Ok(true)) => {
                    // Cache hit — the sidecar was never invoked.
                    state.from_cache.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                Ok(Ok(false)) => {
                    // Sidecar ran and succeeded.
                    state.processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                Ok(Err(e)) => {
                    warn!("OCR failed for {}: {}", path_display, e);
                    state.failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if let Ok(mut guard) = state.errors.lock() {
                        guard.push(format!("{}: {}", path_display, e));
                    }
                }
                Err(e) => {
                    // Join failure on the blocking task itself.
                    warn!("OCR task failed for {}: {}", path_display, e);
                    state.failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if let Ok(mut guard) = state.errors.lock() {
                        guard.push(format!("{}: {}", path_display, e));
                    }
                }
            }

            let d = state.done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            let f = state.failed.load(std::sync::atomic::Ordering::Relaxed);

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

    let result = OcrBatchResult {
        processed: state.processed.load(std::sync::atomic::Ordering::Relaxed),
        from_cache: state.from_cache.load(std::sync::atomic::Ordering::Relaxed),
        failed: state.failed.load(std::sync::atomic::Ordering::Relaxed),
        skipped_unavailable: false,
        errors: state.errors.lock().map(|g| g.clone()).unwrap_or_default(),
    };

    info!(
        "OCR batch done: {} processed, {} from cache, {} failed",
        result.processed, result.from_cache, result.failed
    );

    result
}

struct BatchState {
    done: std::sync::atomic::AtomicU32,
    failed: std::sync::atomic::AtomicU32,
    from_cache: std::sync::atomic::AtomicU32,
    processed: std::sync::atomic::AtomicU32,
    errors: std::sync::Mutex<Vec<String>>,
}

fn empty_result() -> OcrBatchResult {
    OcrBatchResult {
        processed: 0,
        from_cache: 0,
        failed: 0,
        skipped_unavailable: false,
        errors: Vec::new(),
    }
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

// ─────────────────────────── unit tests ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Fixture: realistic Indra output for one page of a scanned statement.
    #[test]
    fn indra_blocks_convert() {
        let json = json!({
            "pages": [{
                "language": "ces",
                "texts": [{ "lines": [
                    { "text": "VÝPIS Z ÚČTU", "confidence": 0.98, "bbox": [72, 48, 523, 96] },
                    { "text": "真实的不存在行", "confidence": 0.55, "bbox": null }
                ]}]
            }]
        });

        let r = indra_json_to_ocr_result(&json, Path::new("x.pdf")).unwrap();
        assert_eq!(r.page_count, 1);
        assert_eq!(r.language.as_deref(), Some("ces"));
        assert_eq!(r.layout_blocks.len(), 2);
        assert!(r.is_scan);

        let b0 = &r.layout_blocks[0];
        assert_eq!(b0.label, "line");
        assert_eq!(b0.page, 1);
        assert!(b0.confidence.is_some());
        assert!(b0.box_.is_some());

        let b1 = &r.layout_blocks[1];
        assert_eq!(b1.box_, None);
    }

    /// Minimal shape without pages: the flat fallback must still yield a valid
    /// result rather than an error.
    #[test]
    fn empty_pages_fallback() {
        let json = json!({ "text": "hello", "language": "cs" });
        let r = indra_json_to_ocr_result(&json, Path::new("flat.pdf")).unwrap();
        assert_eq!(r.full_text, "hello");
        assert_eq!(r.page_count, 1);
        assert_eq!(r.language.as_deref(), Some("cs"));
    }

    /// A bbox of arbitrary numeric types must normalise or drop cleanly.
    #[test]
    fn bbox_normalization() {
        assert_eq!(
            bbox_to_f32_4(&json!([1, 2, 3, 4])),
            Some([1.0, 2.0, 3.0, 4.0])
        );
        assert_eq!(bbox_to_f32_4(&json!([1.5, 2.5, 3.5, 4.5])), Some([1.5, 2.5, 3.5, 4.5]));
        assert_eq!(bbox_to_f32_4(&json!([1, 2, 3])), None);
        assert_eq!(bbox_to_f32_4(&json!("not an array")), None);
    }

    /// Clamping keeps out-of-range confidences sane.
    #[test]
    fn confidence_clamping() {
        assert_eq!(clamp_unit(1.2), 1.0);
        assert_eq!(clamp_unit(-0.3), 0.0);
        assert_eq!(clamp_unit(0.42), 0.42);
    }

    /// Mean over present confidences only.
    #[test]
    fn mean_confidence_picks_up_present() {
        let r = OcrResult {
            path: "t.pdf".to_string(),
            sha256: None,
            full_text: String::new(),
            language: None,
            confidence_avg: None,
            page_count: 1,
            is_scan: true,
            layout_blocks: vec![
                OcrBlock { page: 1, label: "line".into(), text: "a".into(), confidence: Some(0.8), box_: None },
                OcrBlock { page: 1, label: "line".into(), text: "b".into(), confidence: None, box_: None },
                OcrBlock { page: 1, label: "line".into(), text: "c".into(), confidence: Some(0.4), box_: None },
            ],
        };
        assert_eq!(mean_confidence(&r), Some(0.6));
    }
}
