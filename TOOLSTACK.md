# FileIT Toolstack & Environment

## Required Tools (Verified 2026-06-09)

### Rust
- **Language**: Rust Edition 2021
- **Version**: v1.95.0 (minimum 1.77)
- **Install**: https://rustup.rs
- **Tauri CLI**: `cargo install tauri-cli --version ^2.0.0`
  - Current: v2.10.1 ✓

### Node.js & npm
- **Node.js**: v24.14.0 (minimum v18 LTS)
- **npm**: v11.9.0
- **Install**: https://nodejs.org (LTS recommended)

### Windows-Specific
- **WebView2 Runtime**: Required for Tauri
  - Usually pre-installed on Windows 10/11
  - Fallback: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

### Build System
- **Frontend Build**: Vite 5.4.0 + TypeScript 5.5.3
- **Backend Build**: Tauri 2.0 + Cargo
- **Package Manager**: npm (monorepo-aware)

## Development Environment Setup

```bash
# 1. Install Rust (one-time)
rustup update stable

# 2. Install Tauri CLI (one-time)
cargo install tauri-cli --version ^2.0.0

# 3. Navigate to project
cd fileIT/fileit

# 4. Install Node dependencies
npm install

# 5. Install Rust dependencies
cd src-tauri
cargo fetch

cd ..
```

## Running the Project

### UI-only (mocked Tauri API)
```bash
npm run dev
# Opens Vite dev server on http://localhost:1420
```

### Full app (Rust + React)
```bash
npm run tauri dev
# Launches Tauri window with real Rust backend
```

### Production Build
```bash
npm run tauri build
# Creates installer in: src-tauri/target/release/bundle/
```

## Key Dependencies

### Rust (Tauri Backend)
- **File ops**: walkdir, tokio (async)
- **Document parsing**: pdf-extract, zip, quick-xml
- **Text processing**: regex, strsim (fuzzy matching)
- **Data**: serde_json, chrono
- **Security**: sha2 (deduplication), uuid
- **Platform**: winreg (Windows registry)
- **HTTP**: reqwest (with rustls-tls, no OpenSSL)

### Node.js (React Frontend)
- **UI**: React 18, Zustand (state management)
- **Build**: Vite, TypeScript
- **Fonts**: @fontsource (offline-capable)
- **Tauri IPC**: @tauri-apps/api, plugins (dialog, fs, shell)

## Version Alignment

| Component | Current | Status |
|-----------|---------|--------|
| tauri.conf.json | 1.0.8 | ✓ Source of truth |
| Cargo.toml | 1.0.8 | ✓ Aligned |
| package.json | 1.0.0 | ⚠️ Needs sync to 1.0.8 |

## Architecture

```
┌─ React (Vite)           →  IPC Invoke/Listen  →  Tauri Backend (Rust) ┐
│  ├─ Components                                    ├─ Scanner           │
│  ├─ UI State (Zustand)         ↔ File I/O        ├─ Classifier        │
│  └─ Tauri API wrappers  ←─ Real backend ops ←    ├─ Restructure       │
└─────────────────────────────────────────────────────────────────────┘
```

## Common Issues

- **WebView2 Missing**: Install from Microsoft (Windows only)
- **Cargo lock failures**: `cargo update` or delete `Cargo.lock` in src-tauri
- **Port 1420 already in use**: Change `devUrl` in `tauri.conf.json`
- **PDF extract panics**: Handled gracefully (malformed PDFs skip)
- **Large scans (>10k files)**: May need pagination (known issue)

## Support & Sync

- **Primary dev**: C:\_dev\fileIT\fileit
- **Backup versions**: C:\_dev\fileIT\1.0.2 (v1.0.6)
- **Git remote**: To be configured
- **Documentation**: See PROJECT_MEMORY.md
