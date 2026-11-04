// src-tauri/src/cnb_registry.rs
//! CNB (Czech National Bank) Registry for Institution Detection
//!
//! Loads the CNB JERRS registry (embedded at compile time via `include_str!`)
//! and provides lookups for detecting financial institutions from file paths.
//!
//! The full registry (~50k institutions) is embedded in the binary and indexed
//! in memory at startup: by ICO, by exact lower-cased name, and by tokenized
//! keyword, so real-world folder names (e.g. `\Kooperativa\2024\`) can match
//! against legal names (e.g. `Kooperativa pojišťovna, a.s.`).

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// A single institution from the CNB registry.
#[derive(Debug, Clone, Deserialize)]
pub struct Institution {
    pub ico: String,
    pub name: String,
}

/// Registry build statistics — surfaced in the startup log for diagnostics.
#[derive(Debug, Default, Deserialize, Clone)]
pub struct RegistryStats {
    pub files_processed: usize,
    pub files_timeout: usize,
    pub files_error: usize,
    pub total_records: usize,
    pub total_skipped: usize,
}

/// The complete CNB registry structure (parsed from the embedded JSON).
#[derive(Debug, Deserialize)]
struct Registry {
    version: String,
    generated: String,
    source: String,
    schema_version: u32,
    institution_count: usize,
    stats: RegistryStats,
    institutions: Vec<Institution>,
}

/// CNB Registry Manager — provides fast institution lookups.
#[derive(Debug, Clone)]
pub struct CnbRegistry {
    by_ico: Arc<HashMap<String, Institution>>,
    by_name_lower: Arc<HashMap<String, Institution>>,
    /// keyword token -> first institution bearing that token
    token_index: Arc<HashMap<String, Institution>>,
    count: usize,
}

/// Generic legal/descriptor tokens and ultra-common words that are not
/// distinctive enough to identify an institution on their own. Kept short to
/// avoid over-filtering real matches; the token matcher also enforces a
/// minimum token length as a secondary guard.
const GENERIC_TOKENS: &[&str] = &[
    // Legal / corporate-designation suffixes & ultra-common descriptors (len>=3 enforced too)
    "as", "sro", "spol", "spolecnost", "společnost", "vign", "holding",
    "investicni", "investiční", "organizacni", "organizační", "pobocka",
    "pobočka", "sluzby", "slużby", "vyrobni", "vyrobní", "obchodni", "obchodní",
    "a.s", "s.r.o", "v.o.s", "o.p.s", "k.s", "bank", "banka", "adresa", "do",
    // OS / shell / doc-folder words that appear in real paths but never identify an institution
    "users", "user", "documents", "document", "downloads", "download", "desktop",
    "onedrive", "dropbox", "appdata", "temp", "roaming", "local", "windows",
    "system32", "program", "files", "config", "public", "shared", "default",
    // Generic document/title words: never use these as institution evidence.
    "system", "service", "services", "financial", "finance", "strategy",
    "design", "report", "reports", "final", "addendum", "analysis", "process",
    "project", "plan", "annual", "book", "books", "document", "documents",
    "letter", "statement", "maps", "consulting", "risk", "insurance",
];

impl CnbRegistry {
    /// Load the CNB registry from the embedded JSON resource.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let json_str = include_str!("../resources/cnb_registry.json");
        let registry: Registry = serde_json::from_str(json_str)?;

        let mut by_ico = HashMap::with_capacity(registry.institutions.len());
        let mut by_name_lower = HashMap::with_capacity(registry.institutions.len());
        let mut token_index: HashMap<String, Institution> = HashMap::new();

        for inst in &registry.institutions {
            by_ico.insert(inst.ico.clone(), inst.clone());
            by_name_lower.insert(inst.name.to_lowercase(), inst.clone());
            for tok in tokenize(&inst.name) {
                token_index.entry(tok).or_insert_with(|| inst.clone());
            }
        }

        log::info!(
            "CNB Registry loaded: {} institutions (v{}, schema v{}, source {}, generated {})",
            registry.institution_count,
            registry.version,
            registry.schema_version,
            registry.source,
            registry.generated
        );
        log::info!(
            "CNB Registry build stats: {} files, {} records, {} skipped, {} err, {} timeout",
            registry.stats.files_processed,
            registry.stats.total_records,
            registry.stats.total_skipped,
            registry.stats.files_error,
            registry.stats.files_timeout
        );

        Ok(Self {
            by_ico: Arc::new(by_ico),
            by_name_lower: Arc::new(by_name_lower),
            token_index: Arc::new(token_index),
            count: registry.institution_count,
        })
    }

    /// Look up an institution by ICO (Czech organisation ID).
    pub fn lookup_by_ico(&self, ico: &str) -> Option<&Institution> {
        self.by_ico.get(ico)
    }

    /// Look up an institution by exact name (case-insensitive).
    pub fn lookup_by_name(&self, name: &str) -> Option<&Institution> {
        self.by_name_lower.get(&name.to_lowercase())
    }

    /// Detect an institution from file path components.
    /// Returns the best match with a confidence score (0.0-1.0).
    pub fn detect_from_path(&self, path: &Path) -> Option<(Institution, f64)> {
        let components: Vec<String> = path
            .parent()
            .unwrap_or(path)
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .map(|s| s.to_lowercase())
            .collect();

        // Strategy 1: 8-digit ICO embedded in the filename → very high confidence.
        if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
            if let Some(ico) = find_ico_in_filename(filename) {
                if let Some(inst) = self.lookup_by_ico(&ico) {
                    return Some((inst.clone(), 0.95));
                }
            }
        }

        // Strategy 2: a path component that equals an exact registered name.
        for comp in &components {
            if let Some(inst) = self.lookup_by_name(comp) {
                return Some((inst.clone(), 0.90));
            }
        }

        // Strategy 3: tokenized keyword match across directory components only.
        // Never use ordinary filename words as CNB evidence; names like
        // "Financial Report" routinely occur in unrelated documents.
        let mut best: Option<(Institution, f64)> = None;
        for comp in &components {
            for tok in tokenize(comp) {
                if tok.len() < 6 {
                    continue;
                }
                if let Some(inst) = self.token_index.get(&tok) {
                    let score = if tok.len() >= 6 { 0.65 } else { 0.60 };
                    if best.as_ref().map_or(true, |(_, s)| *s < score) {
                        best = Some((inst.clone(), score));
                    }
                }
            }
        }
        best
    }

    /// Total number of institutions in the registry.
    pub fn count(&self) -> usize {
        self.count
    }
}

/// Scan a filename for an 8-digit ICO and return any registered institution.
/// Scan a filename for an 8-digit ICO sequence and return it if found.
fn find_ico_in_filename(filename: &str) -> Option<String> {
    let bytes = filename.as_bytes();
    if bytes.len() < 8 {
        return None;
    }
    for window in bytes.windows(8) {
        if let Ok(s) = std::str::from_utf8(window) {
            if s.chars().all(|c| c.is_ascii_digit()) {
                return Some(s.to_string());
            }
        }
    }
    None
}

/// Split a string into normalized keyword tokens, dropping generic words.
fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .map(str::trim)
        .filter(|t| t.len() >= 3)
        .filter(|t| !GENERIC_TOKENS.contains(t))
        .map(|t| t.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_kooperativa_from_folder_token() {
        let reg = CnbRegistry::new().expect("registry loads");
        let p = Path::new(r"C:\Users\jaros\Kooperativa\2024\pojistka.pdf");
        let (inst, conf) = reg.detect_from_path(p).expect("folder token match");
        assert!(
            inst.name.to_lowercase().contains("kooperativa"),
            "unexpected match: {}",
            inst.name
        );
        assert!(conf >= 0.6);
    }

    #[test]
    fn detects_ico_in_filename() {
        let reg = CnbRegistry::new().expect("registry loads");
        let p = Path::new(r"C:\temp\smlouva_25672720_2026.pdf");
        let (inst, conf) = reg.detect_from_path(p).expect("ICO match");
        assert_eq!(inst.ico, "25672720", "ICO should map to the registered entity");
        assert_eq!(conf, 0.95);
    }

    #[test]
    fn no_match_for_unrelated_app_folder() {
        let reg = CnbRegistry::new().expect("registry loads");
        let p = Path::new(r"C:\Users\jaros\AppData\Roaming\SomeApp\config\cache.log");
        assert!(
            reg.detect_from_path(p).is_none(),
            "should not match an unrelated app folder"
        );
    }

    #[test]
    fn ignores_generic_document_filename_words() {
        let reg = CnbRegistry::new().expect("registry loads");
        for filename in [
            "Sparta Financial Due Diligence IT Addendum Final Report.pdf",
            "1_ITIL3 Service Strategy.pdf",
            "EXIM-IT System_Process_Analysis.docx",
        ] {
            let p = Path::new(r"C:\Users\jaros\Documents\Archiv").join(filename);
            assert!(
                reg.detect_from_path(&p).is_none(),
                "generic filename should not match CNB: {}",
                filename
            );
        }
    }

    #[test]
    fn keeps_real_institution_directory_signal() {
        let reg = CnbRegistry::new().expect("registry loads");
        let p = Path::new(r"C:\Users\jaros\Documents\Kooperativa\2024\pojistka.pdf");
        let (inst, conf) = reg.detect_from_path(p).expect("institution directory match");
        assert!(inst.name.to_lowercase().contains("kooperativa"));
        assert!(conf >= 0.6);
    }
}
