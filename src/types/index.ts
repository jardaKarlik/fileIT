// src/types/index.ts
// Mirror of the Rust types that cross the IPC bridge.

export type FileCategory = 'pdf' | 'word' | 'excel' | 'image' | 'email' | 'text' | 'other';

export type DocumentType =
  | 'Contract' | 'BankLetter' | 'Invoice' | 'PaymentDemand'
  | 'TaxReturn' | 'UtilityBill' | 'InsuranceContract' | 'BankStatement'
  | 'AdminDecision' | 'Identity' | 'LegalLetter' | 'Unknown';

export type InstitutionCategory =
  | 'bank' | 'insurance' | 'government' | 'utility' | 'telco' | 'legal' | 'other';

export type GroupDimension = 'customer' | 'date' | 'inst' | 'doc';

export type DestinationMode =
  | { type: 'local_only' }
  | { type: 'local_plus_cloud'; cloud_name: string }
  | { type: 'cloud_to_cloud'; source_cloud: string; dest_cloud: string };

export type ConfirmationStatus =
  | 'pending' | 'confirmed' | 'restored' | 'auto_confirmed';

export interface CustomerMatch {
  name: string;
  rodne_cislo: string | null;
  ico: string | null;
  confidence: number;
}

export interface InstitutionMatch {
  canonical_name: string;
  category: InstitutionCategory;
  confidence: number;
}

export interface FileRecord {
  path: string;
  name: string;
  extension: string;
  size_bytes: number;
  modified: string; // ISO datetime
  sha256: string | null;
}

export interface ClassifiedFile {
  record: FileRecord;
  category: FileCategory;
  customer: CustomerMatch | null;
  all_customers: CustomerMatch[];
  institution: InstitutionMatch | null;
  document_type: DocumentType;
  document_date: string | null;
  confidence: number;
  ocr_used: boolean;
  text_preview: string | null;
  is_duplicate: boolean;
  duplicate_of: string | null;
  pdf_producer: string | null;
  pdf_creator: string | null;
}

export interface PlannedMove {
  source: string;
  destination: string;
  file_name: string;
}

export interface RestructurePreview {
  total_files: number;
  total_folders: number;
  total_subfolders: number;
  unknown_count: number;
  duplicate_count: number;
  target_path: string;
  planned_moves: PlannedMove[];
  target_has_existing_files: boolean;
  existing_file_count: number;
}

export interface ScanResult {
  total_files: number;
  total_size_bytes: number;
  customers_found: number;
  duplicates_found: number;
  files: ClassifiedFile[];
}

export interface FileError {
  timestamp: string;
  operation: string;
  source: string;
  destination: string;
  error_message: string;
}

export interface RestructureResult {
  session_id: string;
  files_moved: number;
  folders_created: number;
  unknown_count: number;
  duration_seconds: number;
  backup_manifest_path: string | null;
  /** Actual destination root used — authoritative source for done screen. */
  target_path: string;
  failed_count: number;
  errors: FileError[];
}

export interface ManifestEntry {
  original_path: string;
  new_path: string;
  sha256: string;
}

export interface BackupManifest {
  id: string;
  created_at: string;
  session_id: string;
  moves: ManifestEntry[];
}

export interface CompletedRun {
  session_id: string;
  completed_at: string;
  duration_seconds: number;
  files_moved: number;
  folders_created: number;
  unknown_count: number;
  target_path: string;
  backup_manifest_path: string;
  manifest: BackupManifest;
  confirmation_status: ConfirmationStatus;
  auto_confirm_at: string;
  backup_delete_at: string | null;
}

export interface DetectedCloud {
  name: string;
  root_path: string;
  is_running: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// CM / Učebna types
// ─────────────────────────────────────────────────────────────────────────────

export interface PatternSignal {
  label: string;
  strength: 'strong' | 'medium' | 'weak';
}

export interface UnclassifiedPattern {
  pattern_id: string;
  file_count: number;
  signals: PatternSignal[];
  thumbnail_colors: string[];
  pdf_producer: string | null;
  pdf_creator: string | null;
  size_bucket: string | null;
}

export interface CmPayload {
  pdf_producer: string | null;
  pdf_creator: string | null;
  color_buckets: string[];
  logo_phash: number | null;
  institution: string;
  doc_type: string;
}

export interface TeachingResult {
  local_fingerprint_added: boolean;
  cm_payload: CmPayload;
}

export interface UploadResult {
  uploaded: number;
  queued: number;
}

export interface CmUpdateResult {
  updated: boolean;
  version: string;
  fingerprint_count: number;
}

// Progress event payloads (received via Tauri event listener)
export interface ScanProgress {
  files_found: number;
  current_path: string;
  message: string;
  done: boolean;
}

export interface ClassifyProgress {
  done: number;
  total: number;
  current: string;
}

export interface RestructureProgress {
  files_moved: number;
  total_files: number;
  current_file: string;
  log_line: string;
  done: boolean;
  error: string | null;
}
