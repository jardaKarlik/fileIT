// src-tauri/src/types.rs
//
// Core domain types for FileIT.
// All types are Serde-serialisable so they can cross the IPC bridge to the frontend.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// File scanning
// ─────────────────────────────────────────────────────────────────────────────

/// A raw file found on disk before classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    /// Absolute path to the file.
    pub path: PathBuf,
    /// File name including extension.
    pub name: String,
    /// Extension lowercased (pdf, docx, jpg, …)
    pub extension: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// File system last-modified timestamp.
    pub modified: DateTime<Utc>,
    /// SHA-256 hex digest — used for deduplication detection.
    pub sha256: Option<String>,
}

/// File type categories we care about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileCategory {
    Pdf,
    Word,
    Excel,
    Image,
    Email,
    Text,
    Other,
}

impl FileCategory {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "pdf" => Self::Pdf,
            "doc" | "docx" => Self::Word,
            "xls" | "xlsx" | "csv" => Self::Excel,
            "jpg" | "jpeg" | "png" | "bmp" | "tif" | "tiff" => Self::Image,
            "msg" | "eml" => Self::Email,
            "txt" | "rtf" => Self::Text,
            _ => Self::Other,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Classification
// ─────────────────────────────────────────────────────────────────────────────

/// Classification confidence score 0.0–1.0
pub type Score = f32;

/// Identified customer — extracted from document content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerMatch {
    /// Display name as found in the document.
    pub name: String,
    /// Czech birth number (rodné číslo) if found — primary unique ID.
    pub rodne_cislo: Option<String>,
    /// Business ID (IČO) if found — for legal entities.
    pub ico: Option<String>,
    /// Confidence 0.0–1.0 for this customer identification.
    pub confidence: Score,
}

/// Identified issuing institution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstitutionMatch {
    /// Canonical institution name from the dictionary.
    pub canonical_name: String,
    /// Institution category.
    pub category: InstitutionCategory,
    /// Confidence 0.0–1.0
    pub confidence: Score,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstitutionCategory {
    Bank,
    Insurance,
    Government,
    Utility,
    Telco,
    Legal,
    Other,
}

/// Document type taxonomy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    Contract,
    BankLetter,
    Invoice,
    PaymentDemand,
    TaxReturn,
    UtilityBill,
    InsuranceContract,
    BankStatement,
    AdminDecision,
    Identity,
    LegalLetter,
    Unknown,
}

/// A fully classified file — extends FileRecord with extracted metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedFile {
    pub record: FileRecord,
    pub category: FileCategory,
    /// Primary customer identified in this document.
    pub customer: Option<CustomerMatch>,
    /// All customers referenced (for multi-customer documents).
    pub all_customers: Vec<CustomerMatch>,
    pub institution: Option<InstitutionMatch>,
    pub document_type: DocumentType,
    /// Best-guess document date (from content, not filesystem).
    pub document_date: Option<DateTime<Utc>>,
    /// Overall classification confidence.
    pub confidence: Score,
    /// True if content was extracted via OCR (image/scanned PDF).
    pub ocr_used: bool,
    /// Raw extracted text snippet (first 500 chars) for UI display.
    pub text_preview: Option<String>,
    /// Whether this file appears to be a duplicate of another.
    pub is_duplicate: bool,
    /// Path of the file this duplicates (if is_duplicate).
    pub duplicate_of: Option<PathBuf>,
    /// PDF /Producer string — used for CM pattern grouping in Učebna.
    pub pdf_producer: Option<String>,
    /// PDF /Creator string — used for CM pattern grouping.
    pub pdf_creator: Option<String>,
}

impl ClassifiedFile {
    /// Returns true if confidence is in the "review required" band (0.40–0.84).
    pub fn needs_review(&self) -> bool {
        self.confidence >= 0.40 && self.confidence < 0.85
    }

    /// Grouping key for a given dimension.
    pub fn group_key(&self, dimension: &GroupDimension) -> String {
        match dimension {
            GroupDimension::Customer => self
                .customer
                .as_ref()
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "Neznámý klient".to_string()),
            GroupDimension::Date => self
                .document_date
                .map(|d| d.format("%Y-%m").to_string())
                .unwrap_or_else(|| "Neurčeno".to_string()),
            GroupDimension::Institution => self
                .institution
                .as_ref()
                .map(|i| i.canonical_name.clone())
                .unwrap_or_else(|| "Bez instituce".to_string()),
            GroupDimension::DocumentType => format!("{:?}", self.document_type),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Restructure
// ─────────────────────────────────────────────────────────────────────────────

/// The four grouping dimensions the user can pick from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupDimension {
    Customer,
    Date,
    Institution,
    DocumentType,
}

/// Destination mode chosen by the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DestinationMode {
    /// Files stay on local drive only.
    LocalOnly,
    /// Files written into cloud sync client's watched folder.
    LocalPlusCloud { cloud_name: String },
    /// Files read from source cloud, written to destination cloud.
    CloudToCloud {
        source_cloud: String,
        dest_cloud: String,
    },
}

/// A planned file move — computed during preview, executed on confirm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedMove {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub file_name: String,
}

/// A preview of the restructure — shown to user before execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestructurePreview {
    pub total_files: usize,
    pub total_folders: usize,
    pub total_subfolders: usize,
    pub unknown_count: usize,
    pub duplicate_count: usize,
    pub target_path: PathBuf,
    pub planned_moves: Vec<PlannedMove>,
    /// Collision: target path already has files.
    pub target_has_existing_files: bool,
    pub existing_file_count: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// Backup / Restore
// ─────────────────────────────────────────────────────────────────────────────

/// Manifest written into the backup ZIP — enables precise restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub session_id: String,
    pub moves: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub original_path: PathBuf,
    pub new_path: PathBuf,
    pub sha256: String,
}

impl BackupManifest {
    pub fn new(session_id: &str, moves: Vec<ManifestEntry>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            session_id: session_id.to_string(),
            moves,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Session state — the root of all in-memory application state
// ─────────────────────────────────────────────────────────────────────────────

/// Confirmation status of a completed restructure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationStatus {
    /// User has not yet checked and confirmed.
    Pending,
    /// User confirmed the restructure is correct.
    Confirmed,
    /// User restored to original state.
    Restored,
    /// Auto-confirmed after 30 days without user action.
    AutoConfirmed,
}

/// The result of a completed restructure run — persisted across launches
/// until the user confirms or restores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedRun {
    pub session_id: String,
    pub completed_at: DateTime<Utc>,
    pub duration_seconds: u64,
    pub files_moved: usize,
    pub folders_created: usize,
    pub unknown_count: usize,
    pub target_path: PathBuf,
    pub backup_zip_path: PathBuf,
    pub manifest: BackupManifest,
    pub confirmation_status: ConfirmationStatus,
    /// Auto-confirm deadline — 30 days after completion.
    pub auto_confirm_at: DateTime<Utc>,
    /// ZIP auto-delete deadline — 7 days after user confirms.
    pub backup_delete_at: Option<DateTime<Utc>>,
}

/// The central in-memory session state.
/// Created fresh on each scan run. Never persisted to disk except for
/// CompletedRun (which uses a small JSON sidecar in AppData).
#[derive(Debug, Default)]
pub struct SessionState {
    pub id: String,
    pub scan_roots: Vec<PathBuf>,
    pub file_categories: Vec<FileCategory>,
    pub scanned_files: Vec<FileRecord>,
    pub classified_files: Vec<ClassifiedFile>,
    pub structure: Vec<GroupDimension>,
    pub destination_mode: Option<DestinationMode>,
    pub target_path: Option<PathBuf>,
    pub preview: Option<RestructurePreview>,
    pub backup_enabled: bool,
    pub last_run: Option<CompletedRun>,
    /// Pattern data backing Učebna cards — keyed by pattern_id.
    pub pattern_data: std::collections::HashMap<String, GroupedPatternData>,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            backup_enabled: true,
            ..Default::default()
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// IPC payloads — request/response types that cross the Tauri bridge
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    pub roots: Vec<PathBuf>,
    pub file_categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub files_found: usize,
    pub current_path: String,
    pub message: String,
    pub done: bool,
}

#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub customers_found: usize,
    pub duplicates_found: usize,
    pub files: Vec<ClassifiedFile>,
}

#[derive(Debug, Deserialize)]
pub struct PreviewRequest {
    pub structure: Vec<String>,
    pub target_path: PathBuf,
    pub destination_mode: String,
}

#[derive(Debug, Deserialize)]
pub struct RunRestructureRequest {
    pub backup_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RestructureProgress {
    pub files_moved: usize,
    pub total_files: usize,
    pub current_file: String,
    pub log_line: String,
    pub done: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RestructureResult {
    pub session_id: String,
    pub files_moved: usize,
    pub folders_created: usize,
    pub unknown_count: usize,
    pub duration_seconds: u64,
    pub backup_zip_path: Option<String>,
    /// The actual destination root used — returned so the frontend never
    /// needs to guess or read from local state.
    pub target_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DetectedCloud {
    pub name: String,
    pub root_path: PathBuf,
    pub is_running: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// CM / Učebna types
// ─────────────────────────────────────────────────────────────────────────────

/// A cluster of unclassified files sharing the same fingerprint signals.
/// Učebna shows one card per pattern — never individual filenames.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnclassifiedPattern {
    pub pattern_id: String,
    pub file_count: usize,
    pub signals: Vec<PatternSignal>,
    pub thumbnail_colors: Vec<String>,
    pub pdf_producer: Option<String>,
    pub pdf_creator: Option<String>,
    pub size_bucket: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternSignal {
    pub label: String,
    pub strength: String, // "strong" | "medium" | "weak"
}

/// Internal session data backing an UnclassifiedPattern.
/// Stored in SessionState so submit_teaching can reconstruct CmPayload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupedPatternData {
    pub pdf_producer: Option<String>,
    pub pdf_creator: Option<String>,
    pub color_buckets: Vec<String>,
}

/// Privacy-safe payload sent to CM cloud contribution endpoint.
/// Contains ONLY technical signals — no filenames, paths, or content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmPayload {
    pub pdf_producer: Option<String>,
    pub pdf_creator: Option<String>,
    pub color_buckets: Vec<String>,
    pub logo_phash: Option<u64>,
    pub institution: String,
    pub doc_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TeachingResult {
    pub local_fingerprint_added: bool,
    pub cm_payload: CmPayload,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResult {
    pub uploaded: u32,
    pub queued: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CmUpdateResult {
    pub updated: bool,
    pub version: String,
    pub fingerprint_count: u32,
}
