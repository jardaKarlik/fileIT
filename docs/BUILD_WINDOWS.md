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
src-tauri\target\release\bundle\nsis\FileIT_1.1.0_x64-setup.exe   <- distribute this one
src-tauri\target\release\bundle\msi\FileIT_1.1.0_x64_cs-CZ.msi
src-tauri\target\release\fileit.exe
```

The **setup.exe** (NSIS) includes the Indra OCR step; the MSI does not
(use it for managed / silent deployments).

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

The **setup.exe** asks after copying the files: "Nainstalovat Indra OCR…?".
On *Yes* it runs `resources\install-indra.ps1`, which:

1. finds Python 3.10+ (or installs Python 3.12 for the current user via winget),
2. creates `%LOCALAPPDATA%\FileIT-Indra\indra-ocr` (outside the app folder) and installs `indra-ocr` into it,
3. sets the user variable `INDRA_OCR_COMMAND` (FileIT reads it),
4. pre-downloads the OCR models.

About 230 MB is downloaded; log: `%LOCALAPPDATA%\FileIT-Indra\indra-install.log`.
Silent installs (`/S`) and updates skip the step. To run it later:

```
powershell -ExecutionPolicy Bypass -File "%LOCALAPPDATA%\FileIT\resources\install-indra.ps1"
```

(the path is the FileIT install folder). The uninstaller offers to remove the
Indra environment too (never during updates).

MSI installs / manual setup: `pip install indra-ocr` and, if `indra-ocr` is not
on PATH, set `INDRA_OCR_COMMAND` to the full path of `indra-ocr.exe`. Without
Indra, FileIT still runs; PDFs and images are classified from names only.
