# FileIT Project Memory Document

## Project Overview
FileIT is a smart file organizer for Czech financial advisory agents. It's a desktop application built with Tauri (Rust backend) and React/TypeScript frontend that helps users scan, classify, and reorganize their financial documents.

## Current Status (as of 2026-08-11)
- **Version**: v1.0.12 development (package/Cargo metadata still reports 1.0.11 and should be aligned before release)
- **State**: End-of-day checkpoint; core scan/classification changes committed locally and validated in dev mode
- **Latest commits**:
  - `7fd71a3` — v1.0.12: harden classification and incremental scanning
  - `9a77c61` — docs: record v1.0.12 analysis and handoff
- **Remote state**: commits are local only; not pushed from this session
- **Validation**: `cargo check` zero warnings; `cargo test --lib` 13 passed; `npm run build` successful
- **Primary Tech Stack**:
  - Backend: Rust (Tauri 2.0)
  - Frontend: React 18 + TypeScript + Vite
  - State Management: Zustand
  - Build: Tauri CLI + Vite

## Essential Files for Development (Non-Artifact)

To enable quick setup on another workstation with minimal download (~50MB vs 10GB full), transfer only these:

### Core Source Code
```
fileit/
├─ src/
│  ├─ components/          # All React components (Scanning, Dashboard, Restructure, etc.)
│  ├─ store/               # Zustand state management
│  ├─ types/               # TypeScript interfaces
│  ├─ utils/               # Tauri API wrappers
│  ├─ App.tsx
│  ├─ main.tsx
│  └─ styles/
│     └─ global.css
├─ src-tauri/
│  ├─ src/
│  │  ├─ main.rs           # Tauri entry point
│  │  ├─ lib.rs            # Application setup and plugin registration
│  │  ├─ commands/         # IPC command handlers
│  │  ├─ classifier/       # Document classification logic
│  │  ├─ scanner/          # File scanning implementation
│  │  ├─ restructure/      # File reorganization logic
│  │  ├─ backup/           # Backup/restore functionality
│  │  ├─ metadata_reader.rs# Metadata extraction
│  │  ├─ types.rs          # Shared types
│  │  └─ cm_registry.rs    # Customer/Institution matching
│  ├─ resources/           # Embedded data files
│  │  ├─ cm_registry_seed.json
│  │  └─ institution_dictionary.json
│  ├─ tauri.conf.json
│  ├─ Cargo.toml           # Rust dependencies
│  │  └─ build.rs
└─ fileit/
   ├─ package.json         # Node.js dependencies
   ├─ tsconfig.json
   ├─ vite.config.ts
   └─ index.html
```

### Key Rust Dependencies
- `tauri = "2.0"` + plugins (dialog, fs, shell, log)
- `tokio = { version = "1", features = ["full"] }`
- `serde = { version = "1", features = ["derive"] }` + `serde_json = "1"`
- `walkdir = "2"` (file walking)
- `pdf-extract = "0.7"` (PDF text extraction)
- `zip = "2"` + `quick-xml = "0.36"` (DOCX processing)
- `regex = "1"` + `once_cell = "1"` (pattern matching)
- `strsim = "0.11"` (fuzzy string matching for customer names)
- `chrono = { version = "0.4", features = ["serde"] }` (date handling)
- `sha2 = "0.10"` + `hex = "0.4"` (SHA256 for deduplication)
- `uuid = { version = "1", features = ["v4"] }` (session IDs)
- `anyhow = "1"` + `thiserror = "1"` (error handling)
- `reqwest = { version = "0.11", features = ["json", "rustls-tls"], default-features = false }` (HTTP for CM updates)
- `winreg = "0.52"` (Windows registry access for OneDrive detection)

### Key Node.js Dependencies
- `@tauri-apps/api`: "^2.0.0"
- `@tauri-apps/plugin-dialog`: "^2.0.0"
- `@tauri-apps/plugin-fs`: "^2.0.0"
- `@tauri-apps/plugin-shell`: "^2.0.0"
- `react`: "^18.3.1"
- `react-dom`: "^18.3.1"
- `zustand`: "^4.5.2"
- `@fontsource/*`: Font bundles for offline operation
- Dev dependencies: `@tauri-apps/cli`, `@types/react`, `@vitejs/plugin-react`, `typescript`, `vite`

## Current Functionality State

### Implemented Features
1. **File Scanning** (`src/components/Scanning/index.tsx`)
   - Scans user-selected directories for files
   - Shows real-time progress with file count
   - Integrates with Tauri backend for actual scanning
   - Supports development mode simulation

2. **Document Classification** (Backend: `src-tauri/src/classifier/`)
   - Extracts text from PDF, DOCX, images (OCR placeholder)
   - Identifies customer names, rodné číslo, IČO
   - Recognizes institutions (banks, utilities, government, etc.)
   - Classifies document types (Contract, Invoice, Bank Statement, etc.)
   - Detects duplicates via SHA256

3. **Restructure Planning** (`src/components/Restructure/index.tsx`)
   - Interactive dimension selection (customer, date, institution, document type)
   - Live preview of folder structure that will be created
   - Destination selection: local only, local+cloud, or cloud-to-cloud
   - Backup enable/disable toggle with warnings

4. **Progress Tracking** (Store: `src/store/index.ts`)
   - Progress percentage tracking
   - Current operation status text
   - Detailed log lines with timestamps
   - Progress reset functionality

5. **Dashboard** (`src/components/Dashboard/index.tsx`)
   - Summary view of scan results
   - File count, total size, customers found, duplicates
   - Charts showing file activity over time and by customer
   - Quick access to Učebna (teaching feature) and restructuring

6. **File Type Selection** (`src/components/FileTypes/index.tsx`)
   - Core file types: PDF, Word, Excel, Images (pre-selected)
   - Optional types: Email, Text
   - One-click start scanning with auto-detected common folders

7. **Confirmation Screen** (`src/components/Confirmation/index.tsx`)
   - Shows restructuring results
   - Allows user to confirm or restore changes
   - Displays backup location and manifest info

8. **Učebna (Teaching)** (`src/components/Ucebna/index.tsx`)

## Setup Instructions for New Workstation

### Prerequisites
1. **Rust Toolchain**: Install via rustup (https://rustup.rs)
   - Target: `stable` channel
   - Windows MSVC toolchain recommended
2. **Node.js**: v18+ (LTS)
3. **Tauri CLI**: `cargo install tauri-cli --version ^2.0.0`
4. **WebView2 Runtime**: Required for Tauri on Windows (usually pre-installed on Win10/11)

### Setup Steps
1. **Clone repository** (once Git remote is configured)
2. **Install Rust dependencies**:
   ```bash
   cd fileit/src-tauri
   cargo fetch  # Downloads dependencies
   ```
3. **Install Node dependencies**:
   ```bash
   cd ../fileit
   npm install
   ```
4. **Development mode** (without Tauri backend - UI only with mocks):
   ```bash
   npm run dev  # Runs Vite dev server with mocked Tauri API
   ```
5. **Full development mode** (with Tauri backend):
   ```bash
   # In one terminal:
   cd src-tauri
   cargo build

   # In another terminal:
   cd ../fileit
   npm run tauri dev
   ```

### Building for Production
```bash
# From fileit/ directory
npm run tauri build  # Creates final installer in src-tauri/target/release/bundle/
```

## Current Known Issues / TODO Items for v1 Completion

### High Priority
1. **Actual OCR Integration**: Currently mock OCR - need to integrate Tesseract or Windows OCR API
2. **Real Cloud Detection**: Improve OneDrive/Google Drive/Dropbox detection beyond registry checks
3. **Error Handling**: More granular error messages for file access issues, permission problems
4. **Performance Optimization**: Large folder scans (>10k files) may need pagination or worker optimization

### Medium Priority
1. **Učebna Synchronization**: Ensure teaching data properly syncs with central CM service
2. **Backup Validation**: Verify ZIP integrity before trusting restore operations
3. **Cross-platform Testing**: Verify behavior on Windows 10/11 (primary target)
4. **Internationalization**: Czech language is hardcoded - consider making it configurable

### Low Priority / Polish
1. **Animation Refinements**: Smoother transitions between screens
2. **Accessibility Improvements**: Better ARIA labels, keyboard navigation
3. **Dark Mode**: Consider adding dark/light theme toggle
4. **Documentation**: Create user guide and administrator manual

## Git Repository Status
*Note: As of this session, the directory is not yet a Git repository. Next steps should include:*
1. Initialize git repo: `git init`
2. Set up remote origin to GitHub repository
3. Create .gitignore targeting build artifacts, node_modules, target directories
4. Initial commit with current state
5. Push to remote for synchronization with desktop workstation

## Recommended Immediate Actions
1. **Initialize Git Repository** to enable version control and sync with desktop
2. **Create .gitignore** file to exclude:
   - `node_modules/`
   - `src-tauri/target/`
   - `fileit/dist/`
   - `.tmp/`, `.cache/`
   - Built executables and installers
3. **Document Build Process** in README for new contributors
4. **Test End-to-End Flow** on this laptop to confirm v1 readiness
5. **Prepare Release Assets** (icons, version info, release notes)

## Architecture Summary
```
┌─────────────────────┐    IPC Invoke/Listen    ┌────────────────────────────┐
│   React Frontend    │◄───────────────────────►│    Tauri Backend (Rust)    │
│  (Vite + TSX)       │                         │                            │
│  - Scanning UI      │                         │  - Scanner (file walking)  │
│  - Dashboard        │                         │  - Classifier (text/NER)   │
│  - Restructure UI   │                         │  - Restructure (file ops)  │
│  - FileType Select  │                         │  - Backup (ZIP ops)        │
│  - Confirmation     │                         │  - CM Registry (matching)  │
│  - Učebna (teach)   │                         │                            │
└─────────────────────┘                         └────────────────────────────┘
                    ▲                               ▲
                    │                               │
              Dev Mock API                   Actual System Calls
              (when !TAURI)                  (filesystem, winreg, etc.)
```

## Communication Between Sessions
When handing off work between this laptop (latest code, no tools) and the desktop workstation (tools, older code), use this protocol:
1. On the laptop: `git add . && git commit -m "session-summary"` then `git push`
2. On the desktop: `git pull` to get latest changes
3. Before working on desktop: `git status` to ensure clean state
4. After working on desktop: `git add . && git commit -m "session-summary"` then `git push`
5. Pull on the laptop to stay in sync (though you can't run/test there)

This document captures the essential state needed for another workstation with development tools to quickly get up to speed and continue work toward v1 completion.

   - Groups unclassified files by patterns
   - Allows users to teach the system about new document patterns
   - Submits feedback to central Customer Matching (CM) service

### Backend Implementation Status
- **Scanner** (`src-tauri/src/scanner/mod.rs`): File walking and basic metadata extraction
- **Classifier** (`src-tauri/src/classifier/`):
  - Extractor: Text extraction from various formats
  - NER: Named Entity Recognition for customers/institutions
  - Scorer: Confidence scoring for classifications
- **Commands** (`src-tauri/src/commands/mod.rs`): All IPC handlers exposed to frontend
- **Restructure** (`src-tauri/src/restructure/mod.rs`): File moving and folder creation logic
- **Backup** (`src-tauri/src/backup/mod.rs`): ZIP backup creation and restoration
- **Metadata Reader** (`src-tauri/src/metadata_reader.rs`): File metadata extraction
- **CM Registry** (`src-tauri/src/cm_registry.rs`): Customer/institution matching database

## v1.0.12 End-of-Day Memory — 2026-08-11

### Completed
- Built and integrated the local CNB JERRS registry loader and path detector.
- Independently verified the CNB source data: 133 XML files, 127,065 raw cross-file rows, 50,283 unique ICO-keyed records, zero duplicate/collision ICOs.
- Kept `src-tauri/resources/cnb_registry.json` local-only; it is explicitly ignored in `.gitignore` and must never be committed. Regenerate it with `python scripts/build_cnb_registry.py` from `C:\_temp\xml`.
- Added persistent scan manifests under AppData and verified incremental scanning: 5,211 existing files reused and 10 newly created PDFs extracted/classified.
- Narrowed scanner exclusions to the requested folders: Native Instruments, Samples, Claude, iZotope, Lexicon, League of Legends, Magda/Madga, and McAfee; normal work folders remain scannable.
- Hardened PDF classification by disabling unstable in-process Windows PDF OCR fallback. Native extraction remains active; OCR should later be isolated in a helper process before re-enabling.
- Fixed sticky `skip_extraction`: metadata skip and CM skip now merge with logical OR, so a strong metadata rule cannot be overwritten by a weaker CM match.
- Reduced CNB false positives by ignoring generic filename words, matching directory components only, and requiring CNB results to resolve through the curated NER dictionary.
- Removed obsolete compiler dead code and reached a zero-warning Rust check.

### Runtime test result
- Latest incremental scan summary: `5221 total, 5211 reused, 10 extracted, 0 extraction-skipped, 458 duplicates`.
- This confirms new files are detected and processed while unchanged files are reused.
- `extraction-skipped=0` in that test is expected because the 10 generated PDFs were new and did not match a skip rule.

### Known cautions / next session
- Do not run Cargo builds/tests while a live FileIT scan is running; inspect logs only until the scan completes.
- The current AppData runtime log is `C:\Users\jaros\AppData\Local\cz.fileit.app\logs\fileit.log`.
- The current scan manifest is under `C:\Users\jaros\AppData\Roaming\cz.fileit.app\scan_history\`.
- Before release, align version metadata (`package.json`, `Cargo.toml`, `tauri.conf.json`) with v1.0.12.
- Run a fresh modified-file test to confirm `metadata_skip=true` produces `skip_extraction=true` and no extraction-start log for an EPO/OpenPDF document.
- Decide whether to implement isolated helper-process OCR or leave OCR disabled for the stable release.