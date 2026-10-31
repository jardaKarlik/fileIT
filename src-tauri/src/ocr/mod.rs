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
//   INDRA_OCR_MODELS   model directory, passed through as --model-dir
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
            "Příkaz „{}“ nebyl nalezen v PATH. Nainstalujte Indra OCR sidecar (např. `pip install indra-ocr`), nebo nastavte INDRA_OCR_COMMAND na plnou cestu k binárce, např. C:\\Users\\jaros\\AppData\\Local\\Python\\pythoncore-3.14-64\\Scripts\\indra-ocr.exe.",
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

/// Run the sidecar on one file and parse its JSONL output.
///
/// The sidecar is invoked as:
///   <command> <file> --out <tmpdir> --format json --cores 1 [--model-dir <dir>]
///
/// `--cores 1` pins a single OCR worker per invocation: batch-level parallelism
/// is ours (the semaphore), so N concurrent sidecars must not each spin up N
/// threads. Even though the CLI can stream JSONL to stdout, we always route
/// through `--out`: that keeps chatter-to-stderr races and oversized stdout
/// pipes out of the picture and gives us a single well-defined artifact to
/// parse (handling parity with #523 timings: stdin closed expressly after the
/// file has been fully read).
///
/// The CLI writes JSON LINES: one JSON object per page, one per line:
///   {"file": "<input>", "page": 0, "source": "text"|"ocr", …,
///    "lines": [{"box":[x1,y1,x2,y2], "text": "…", "conf": 0.98}]}
/// `--out` is a DIRECTORY; inside it write <input-stem>.jsonl.
fn run_sidecar(command: &str, cache_dir: &Path, path: &Path) -> Result<OcrResult, String> {
    let out_dir = fresh_out_dir(cache_dir, path)?;
    let output = spawn_sidecar_json(command, path, &out_dir, SIDECAR_TIMEOUT);
    let result = output.and_then(|raw| parse_indra_output(&raw, path));
    let _ = std::fs::remove_dir_all(&out_dir);
    result
}

/// A unique directory under `cache_dir` used as the sidecar's `--out` target.
///
/// The CLI takes a directory and writes `<input-stem>.jsonl` inside it. We use
/// a fresh (never pre-existing) directory name so a stale file from a previous
/// run can never be mistaken for this run's output; the caller deletes the
/// whole directory when done.
fn fresh_out_dir(cache_dir: &Path, path: &Path) -> Result<PathBuf, String> {
    // Ensure the cache root exists — on the very first OCR run nothing has
    // created `ocr-cache` yet, and probing a child directory under a missing
    // parent fails with NotFound.
    std::fs::create_dir_all(cache_dir)
        .map_err(|e| format!("OCR výstupní adresář nelze vytvořit: {}", e))?;

    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().replace(['\\', '/', ':', ' '], "_"))
        .unwrap_or_else(|| "input".to_string());

    // A few rounds of (stem-counter, suffix) probing — cheaper and more
    // readable than a random module for something that must just be unique.
    for attempt in 0..64 {
        let candidate = cache_dir.join(format!(
            "indra-out-{}-{}-{:03}",
            stem,
            std::process::id(),
            attempt
        ));
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("OCR výstupní adresář nelze vytvořit: {}", e)),
        }
    }
    Err("nelze vytvořit unikátní OCR výstupní adresář".to_string())
}

/// Text source for classification: OCR cache first, live sidecar on miss.
/// Returns (full_text, is_scan).
pub fn text_for_classification(
    app_data_dir: &Path,
    command: &str,
    path: &Path,
    sha256: &str,
) -> Result<(String, bool), String> {
    if let Some(hit) = ocr_store::read(app_data_dir, sha256) {
        return Ok((hit.full_text, hit.is_scan));
    }
    let cache_dir = ocr_store::cache_dir(app_data_dir);
    let mut result = run_sidecar(command, &cache_dir, path)?;
    if result.sha256.is_none() {
        result.sha256 = Some(sha256.to_string());
    }
    ocr_store::write(app_data_dir, &result);
    Ok((result.full_text, result.is_scan))
}

/// Read and validate the JSONL the sidecar wrote into the out dir.
fn read_sidecar_output(out_dir: &Path, path: &Path) -> Result<Vec<u8>, String> {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| "vstupní soubor bez názvu".to_string())?;
    let jsonl = out_dir.join(format!("{}.jsonl", stem));

    if !jsonl.is_file() {
        return Err(format!(
            "sidecar nevytvořil výstupní soubor {}",
            jsonl.display()
        ));
    }
    std::fs::read(&jsonl).map_err(|e| format!("výstup sidecaru nelze přečíst: {}", e))
}

/// Invoke the sidecar, enforce a wall-clock timeout, then read the JSONL it
/// wrote into `out_dir`. Fully synchronous: it runs inside `spawn_blocking`,
/// so a Tokio timeout cannot be awaited here — we poll `try_wait` against a
/// deadline and kill the child when it expires. Stdout/stderr are drained on
/// reader threads so a chatty child can never deadlock a full pipe.
fn spawn_sidecar_json(
    command: &str,
    path: &Path,
    out_dir: &Path,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    use std::io::Read;
    use std::time::Instant;

    let mut cmd = Command::new(command);
    // Positional first: `indra-ocr <file> --out <dir> --format json --cores 1`.
    cmd.arg(path);
    cmd.arg("--out").arg(out_dir);
    cmd.arg("--format");
    cmd.arg("json");
    // One OCR worker per invocation; batch parallelism is the semaphore's job.
    cmd.arg("--cores");
    cmd.arg("1");

    if let Some(models) = env_non_empty("INDRA_OCR_MODELS") {
        cmd.arg("--model-dir").arg(models);
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
    let _stdout = stdout_reader.join().unwrap_or_default();
    let _stderr = stderr_reader.join().unwrap_or_default();

    match status {
        Some(status) if status.success() => read_sidecar_output(out_dir, path),
        Some(status) => {
            let _ = is_directory_empty(out_dir); // best-effort cleanup guard
            Err(format!(
                "sidecar vrátil chybu ({}): {}",
                status,
                String::from_utf8_lossy(&_stderr).trim()
            ))
        }
        None => Err(format!(
            "sidecar překročil časový limit {} s",
            timeout.as_secs()
        )),
    }
}

/// Trivial emptiness probe — mirrors the intent of the old stdout-only reader,
/// kept so the failure path stays symmetrical with success.
fn is_directory_empty(dir: &Path) -> std::io::Result<bool> {
    Ok(dir.read_dir()?.next().is_none())
}

// ─────────────────────────── Indra JSONL → OcrResult ───────────────────────────

/// Parse the sidecar's raw output bytes as JSON LINES and dispatch by shape.
///
/// Verified LIVE with indra-ocr 1.1.2 (smoke test on a real 2-page PDF, both
/// --out mode and stdout mode, text-layer and image-only pages):
///   • The CLI emits one JSON object PER PAGE, one per line.
///   • Each line's keys: file, page (0-based), source ("text" when read from
///     the digital text layer, "ocr" when OCR'd), rotation, skew, vertical,
///     note, lines [{box:[x1,y1,x2,y2], text, conf}], markdown (string with
///     real layout incl. table HTML), blocks [{label, text}], timings.
///   • There is NO language field anywhere.
///
/// For backward compatibility a single object carrying a legacy `pages[]`
/// array or a flat `text` field routes to `indra_json_to_ocr_result`.
fn parse_indra_output(raw: &[u8], path: &Path) -> Result<OcrResult, String> {
    let text = String::from_utf8_lossy(raw);

    let mut pages: Vec<Json> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<Json>(line) {
            Ok(v) => pages.push(v),
            Err(e) => return Err(format!("neplatné JSON na řádku: {}", e)),
        }
    }

    if pages.is_empty() {
        return Err("sidecar vrátil prázdný výstup".to_string());
    }

    // Legacy single-document shape (a pages[] array or flat {text: ...}).
    if pages.len() == 1 && (pages[0].get("pages").is_some() || pages[0].get("text").is_some()) {
        let mut result = indra_json_to_ocr_result(&pages[0], path)?;
        if result.sha256.as_deref().map_or(true, |s| s.is_empty()) {
            result.sha256 = hash_file(path);
        }
        if result.confidence_avg.is_none() {
            result.confidence_avg = mean_confidence(&result);
        }
        return Ok(result);
    }

    // Real shape: one JSON object per page per line.
    let mut result = indra_jsonl_to_ocr_result(&pages, path)?;
    if result.sha256.as_deref().map_or(true, |s| s.is_empty()) {
        result.sha256 = hash_file(path);
    }
    Ok(result)
}

/// Convert real JSONL page objects into our `OcrResult`.
///
/// Mapping contract (per real CLI output, verified against a live smoke test):
///   page_count      := number of JSONL lines (each line = one page)
///   OcrBlock.page   := 1-based (the JSONL `page` field is 0-based; we count
///                      lines, not trusting the CLI's page field to be dense)
///   conf → confidence, clamped into 0..1; missing → None
///   box → box; label → label
///   source "text" → is_scan=false; anything else → is_scan=true
///   confidence_avg := mean of the line confidences (present values only)
///   full_text      := newline-joined line texts in per-page, per-line order
///   language       := None — the JSONL has no language field; never fabricate
///   markdown       := not persisted into blocks (per-page layout already
///                     lives in blocks + lines; storing raw markdown into the
///                     OcrResult type would need a types change owned by
///                     others)
fn indra_jsonl_to_ocr_result(pages: &[Json], path: &Path) -> Result<OcrResult, String> {
    let path_str = path.to_string_lossy().to_string();

    let mut full_text = String::new();
    let mut blocks: Vec<OcrBlock> = Vec::new();
    let mut has_ocr_page = false;

    for (idx, page) in pages.iter().enumerate() {
        let page_no = (idx + 1) as u32; // 1-based output numbering

        // source "text" means digital-text-layer reading; anything else
        // (OCR of a real scan, or missing/unknown) marks the result as scanned.
        if page
            .get("source")
            .and_then(Json::as_str)
            .map_or(true, |s| s != "text")
        {
            has_ocr_page = true;
        }

        // Ordered "lines" of the page. Full text joins them with newlines.
        let mut page_lines: Vec<String> = Vec::new();

        let jsonl_lines = page
            .get("lines")
            .and_then(Json::as_array)
            .cloned()
            .unwrap_or_default();

        for line in &jsonl_lines {
            let text = line
                .get("text")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string();
            if !text.is_empty() {
                page_lines.push(text.clone());
            }
            let confidence = line
                .get("conf")
                .and_then(Json::as_f64)
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

        // Layout blocks (headings, tables) — label is a real layout tag
        // ("Section-Header", "Table", "Text", …) straight from the JSONL.
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
                    .get("conf")
                    .or_else(|| rb.get("confidence"))
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

        // Layout markdown: when present it is the durable human-readable page
        // (tables as HTML etc.). We do NOT append it to full_text — that must
        // stay a newline-joined line transcript — and we do not add markdown
        // text as a block either; both are already represented by blocks/lines.
        // (Storing markdown verbatim would require extending OcrResult, a type
        // owned elsewhere; skipping keeps the shape compatible.)
        let _markdown = page.get("markdown").and_then(Json::as_str);

        if !page_lines.is_empty() {
            full_text.push_str(&page_lines.join("\n"));
            full_text.push('\n');
        }
    }

    if full_text.is_empty() && blocks.is_empty() {
        return Err("sidecar vrátil prázdný výsledek".to_string());
    }

    Ok(OcrResult {
        path: path_str,
        sha256: None,    // filled by caller
        full_text: full_text.trim_end().to_string(),
        language: None, // the JSONL has no language field — never fabricate
        confidence_avg: mean_of_blocks(&blocks),
        page_count: pages.len().max(1) as u32,
        is_scan: has_ocr_page,
        layout_blocks: blocks,
    })
}

/// Convert a legacy Indra CLI JSON document into our `OcrResult`.
///
/// Tolerant mapping kept for backward compatibility with older sidecar builds
/// and the sanity fixture: Indra's earlier shape was `pages[].texts[].lines[]`
/// (plus optional `blocks[]` / layout labels). Everything optional-degrades —
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

/// Mean of the line confidences of a block collection — used by the JSONL
/// path where confidence_avg must be computed on present values only.
fn mean_of_blocks(blocks: &[OcrBlock]) -> Option<f32> {
    let vals: Vec<f32> = blocks.iter().filter_map(|b| b.confidence).collect();
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

                    let cache_dir = ocr_store::cache_dir(&app_data_dir);
                    let mut result = run_sidecar(&command, &cache_dir, &path)?;
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

    /// Fixture: realistic legacy Indra output for one page of a scanned
    /// statement (pre-JSONL shape kept for backward compatibility).
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

    /// Real Indra CLI output (indra-ocr 1.1.2, verified live): JSON LINES,
    /// one object per page, page numbering 0-based, conf + box + label keys.
    fn realistic_jsonl_pages() -> Vec<Json> {
        let mut jsonl = String::new();
        jsonl.push_str(
            &json!({
                "file": "C:\\Klienti\\vypis.pdf", "page": 0, "source": "text",
                "rotation": 0, "skew": 0.0, "vertical": false, "note": "order=model",
                "lines": [
                    {"box": [201.6, 197.0, 625.3, 223.7], "text": "STRANA 1 - VYPIS Z UCTU", "conf": 1.0},
                    {"box": [201.7, 254.6, 305.4, 280.4], "text": "Datum", "conf": 0.42}
                ],
                "markdown": "## STRANA 1 - VYPIS Z UCTU\n\n<table><tr><th>Datum</th></tr></table>",
                "blocks": [
                    {"label": "Section-Header", "bbox": [201.6, 197.0, 625.3, 223.7],
                     "text": "STRANA 1 - VYPIS Z UCTU"},
                    {"label": "Table", "bbox": [201.7, 254.6, 305.4, 280.4], "text": "Datum"}
                ],
                "timings": {"total_ms": 100.0}
            })
            .to_string(),
        );
        jsonl.push('\n');
        jsonl.push_str(
            &json!({
                "file": "C:\\Klienti\\vypis.pdf", "page": 1, "source": "ocr",
                "rotation": 0, "skew": 3.5, "vertical": true,
                "note": "no text layer; order=model",
                "lines": [
                    {"box": [10.0, 10.0, 20.0, 20.0], "text": "STRANA 2", "conf": 1.7},
                    {"box": null, "text": "Bez konfu"}
                ],
                "markdown": "STRANA 2",
                "blocks": [{"label": "Table", "bbox": [1.0, 2.0, 3.0, 4.0], "text": "a\nb"}],
                "timings": {"total_ms": 50.0}
            })
            .to_string(),
        );
        jsonl.push('\n');

        jsonl
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    #[test]
    fn real_jsonl_two_pages() {
        let pages = realistic_jsonl_pages();
        let r = indra_jsonl_to_ocr_result(&pages, Path::new("vypis.pdf")).unwrap();

        // One JSONL line = one page.
        assert_eq!(r.page_count, 2);
        assert_eq!(r.layout_blocks.iter().map(|b| b.page).max(), Some(2));

        // conf → confidence; clamped 0..1; missing → None; box → box.
        let b0 = &r.layout_blocks[0];
        assert_eq!(b0.confidence, Some(1.0));
        assert_eq!(b0.box_, Some([201.6, 197.0, 625.3, 223.7]));
        assert_eq!(b0.page, 1); // first JSONL line → page 1 (1-based)

        let b1 = &r.layout_blocks[1];
        assert_eq!(b1.confidence, Some(0.42));

        // Bloks placement note: first page has 2 lines then 2 blocks, so the
        // page-2 entries start at index 4, not 2.
        let b2 = &r.layout_blocks[4]; // page 2's first line: conf 1.7 → clamp 1.0
        assert_eq!(b2.page, 2);
        assert_eq!(b2.confidence, Some(1.0));

        let b3 = &r.layout_blocks[5]; // no conf key, null box → both None
        assert_eq!(b3.confidence, None);
        assert_eq!(b3.box_, None);
        assert_eq!(b3.page, 2);

        // source "text" on page 1, "ocr" on page 2 → the document is a scan.
        assert!(r.is_scan);

        // full_text: newline-joined line texts in page order.
        assert_eq!(r.full_text, "STRANA 1 - VYPIS Z UCTU\nDatum\nSTRANA 2\nBez konfu");

        // language: the JSONL has no language field — must not be fabricated.
        assert_eq!(r.language, None);

        // confidence_avg: mean of present line confs (line blocks only have
        // conf; the blocks entries in this fixture carry none).
        let expected = (1.0 + 0.42 + 1.0) / 3.0;
        let avg = r.confidence_avg.unwrap();
        assert!((avg - expected).abs() < 1e-6, "avg={} expected={}", avg, expected);

        // layout labels straight from the JSONL.
        let labels: Vec<&str> = r.layout_blocks.iter().map(|b| b.label.as_str()).collect();
        assert!(labels.contains(&"line"), "labels: {:?}", labels);
        assert!(labels.contains(&"Section-Header"), "labels: {:?}", labels);
        assert!(labels.contains(&"Table"), "labels: {:?}", labels);
    }

    /// source "text" → is_scan=false; anything else → true.
    #[test]
    fn jsonl_is_scan_from_source() {
        let mk = |source: &str| {
            let line = json!({
                "file": "f.pdf", "page": 0, "source": source,
                "lines": [{"box": [1, 2, 3, 4], "text": "t", "conf": 0.9}]
            });
            indra_jsonl_to_ocr_result(&[line], Path::new("f.pdf")).unwrap()
        };
        assert!(!mk("text").is_scan);
        assert!(mk("ocr").is_scan);
        assert!(mk("").is_scan);
    }

    /// A missing `source` key must still count as scanned.
    #[test]
    fn jsonl_missing_source_counts_as_scan() {
        let line = json!({
            "file": "f.pdf", "page": 0,
            "lines": [{"box": [1, 2, 3, 4], "text": "t", "conf": 0.9}]
        });
        let r = indra_jsonl_to_ocr_result(&[line], Path::new("f.pdf")).unwrap();
        assert!(r.is_scan);
    }

    /// All-"text" sources across all pages → is_scan=false.
    #[test]
    fn jsonl_all_text_pages_not_scan() {
        let pages = realistic_jsonl_pages()
            .into_iter()
            .map(|mut p| {
                p["source"] = json!("text");
                p
            })
            .collect::<Vec<_>>();
        let r = indra_jsonl_to_ocr_result(&pages, Path::new("vypis.pdf")).unwrap();
        assert!(!r.is_scan);
    }

    /// Out-of-range confidences clamp at both ends of the range.
    #[test]
    fn jsonl_conf_clamping() {
        let line = json!({
            "file": "f.pdf", "page": 0, "source": "ocr",
            "lines": [
                {"box": [1, 2, 3, 4], "text": "a", "conf": -0.5},
                {"box": [1, 2, 3, 4], "text": "b", "conf": 2.0},
                {"box": [1, 2, 3, 4], "text": "c", "conf": 0.5}
            ]
        });
        let r = indra_jsonl_to_ocr_result(&[line], Path::new("f.pdf")).unwrap();
        assert_eq!(r.layout_blocks[0].confidence, Some(0.0));
        assert_eq!(r.layout_blocks[1].confidence, Some(1.0));
        assert_eq!(r.layout_blocks[2].confidence, Some(0.5));
    }

    /// Empty/blank-only output must still error, never silently pass.
    #[test]
    fn jsonl_empty_pages_is_error() {
        let pages: Vec<Json> = Vec::new();
        assert!(indra_jsonl_to_ocr_result(&pages, Path::new("f.pdf")).is_err());
    }

    /// The full pipeline through `parse_indra_output` on raw JSONL bytes.
    #[test]
    fn parse_indra_output_jsonl_round_trip() {
        let pages = realistic_jsonl_pages();
        let mut raw = String::new();
        for p in &pages {
            raw.push_str(&p.to_string());
            raw.push('\n');
        }
        let r = parse_indra_output(raw.as_bytes(), Path::new("vypis.pdf")).unwrap();
        assert_eq!(r.page_count, 2);
        assert_eq!(r.language, None);
        assert!(r.is_scan);
        assert_eq!(r.layout_blocks.len(), 7); // 2+2 lines&blocks pg1, 2+1 pg2
    }

    /// Legacy pages[] document given as a single JSONL line still routes to
    /// the legacy mapper (backward compatibility).
    #[test]
    fn parse_indra_output_legacy_pages_still_works() {
        let legacy = json!({
            "pages": [{
                "language": "ces",
                "texts": [{ "lines": [
                    { "text": "A", "confidence": 0.9, "bbox": [1, 2, 3, 4] }
                ]}]
            }]
        });
        let raw = format!("{}\n", legacy);
        let r = parse_indra_output(raw.as_bytes(), Path::new("x.pdf")).unwrap();
        assert_eq!(r.page_count, 1);
        assert_eq!(r.language.as_deref(), Some("ces"));
        assert_eq!(r.layout_blocks[0].text, "A");
    }
}
