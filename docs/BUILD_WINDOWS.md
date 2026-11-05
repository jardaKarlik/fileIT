# Building the FileIT installer on Windows

## Prerequisites (one time)

- Rust (stable, MSVC toolchain) via rustup
- Node.js 18+ (LTS)
- Visual Studio Build Tools with "Desktop development with C++"
- WebView2 runtime (preinstalled on Windows 10/11; the installer also embeds
  the bootstrapper)
- WiX is downloaded automatically by the Tauri CLI on the first MSI build.

## Build

Run each command in PowerShell from the repository root:

```
git fetch origin
git checkout lee/awesome-curie-ek1wko
git pull
npm ci
npm run tauri build
```

`npm ci` installs the exact package versions from `package-lock.json`.
`Cargo.lock` is committed, so the Rust side uses the tested versions too.

Output:

```
src-tauri\target\release\bundle\msi\FileIT_1.1.0_x64_cs-CZ.msi
src-tauri\target\release\fileit.exe
```

A release build takes 5–15 minutes (LTO, size-optimised).

## Common problems

- **"Found version mismatched Tauri packages"** — the Tauri crates in
  `src-tauri/Cargo.toml` and the `@tauri-apps/*` npm packages must share
  major.minor. Bump both sides together.
- **"Your local changes … would be overwritten"** on `git pull` with an empty
  `git diff` — line endings only; `git checkout -- <file>` and pull again.
- **SmartScreen "Windows protected your PC"** — the MSI is not code-signed
  yet. Click *More info → Run anyway*. Signing: set
  `bundle.windows.certificateThumbprint` in `src-tauri/tauri.conf.json`.

## Indra OCR on the target machine

The installer does not include Indra OCR. On each PC that should OCR scans:

```
pip install indra-ocr
```

If `indra-ocr` is not on PATH, set `INDRA_OCR_COMMAND` to the full path of
`indra-ocr.exe`. Without it, FileIT still runs; PDFs and images are classified
from file and folder names only, and the Home screen says OCR is not installed.
