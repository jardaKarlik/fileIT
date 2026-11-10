# FileIT on macOS

Status: **porting phase.** The app builds and runs in dev mode and as an
unsigned (ad-hoc signed) local `.app` / `.dmg`. Apple Silicon first; Intel
Macs compile but are not tested yet. No Developer ID, no notarization.

What differs from Windows:

| | Windows | macOS |
|---|---|---|
| Default scan | all local fixed drives | home folder + cloud folders (see below) |
| Skipped | Windows, Program Files, AppData, … | /System, /Applications, /Library, /private, ~/Library, ~/Applications, ~/.Trash, `*.app`, `*.photoslibrary`, `*.musiclibrary`, `*.xcodeproj`, VM images, node_modules, hidden files … |
| Cloud folders | ~\OneDrive, ~\Dropbox, ~\Google Drive, ~\Box | ~/Library/CloudStorage/OneDrive-\*, GoogleDrive-\*, Dropbox, Box-\*; iCloud Drive (~/Library/Mobile Documents/com~apple~CloudDocs) |
| Cloud-only files | skipped (Files On-Demand attributes) | skipped (dataless files/folders, `.name.icloud` stubs), never downloaded |
| Default "organised" folder | `C:\ReOrganized` | `~/FileIT/Organized` |
| "Open folder" | Explorer | Finder (`open -R`) |
| App data | `%APPDATA%\cz.fileit.app` | `~/Library/Application Support/cz.fileit.desktop` |
| Logs | `%LOCALAPPDATA%\cz.fileit.app\logs\fileit.log` | `~/Library/Logs/cz.fileit.desktop/fileit.log` |
| Indra OCR config | env vars, HKCU\Environment, settings file | env vars, settings file |

The macOS bundle identifier is `cz.fileit.desktop` (set in
`src-tauri/tauri.macos.conf.json`). Windows keeps `cz.fileit.app`, so
existing Windows app data is untouched. (Tauri warns about identifiers that
end in `.app` on macOS because that is the bundle extension.)

File names: macOS often stores accented names decomposed (NFD). FileIT
compares, matches and keys everything in NFC (`src-tauri/src/norm.rs`), so
`Dvořák.pdf` from a Mac folder matches `Dvořák` from document text, and
`Faktura.pdf` / `faktura.pdf` / an NFD spelling are one name when planning
moves (APFS is case- and normalisation-insensitive by default).

---

## 0. End-user install (one-click, no Xcode/Rust/Node needed)

For a user who just wants to run FileIT — not build it — use
`scripts/macos/install-fileit.sh`. It mirrors the Windows NSIS installer's
behaviour (`src-tauri/windows/installer-hooks.nsh` +
`src-tauri/resources/install-indra.ps1`): it is not a dev-environment setup,
it is the distribution/installer story for macOS until a signed `.pkg`
exists.

What it does:
1. Asks for the admin password once (`sudo -v`), needed to install into
   `/Applications` and, if used, for Homebrew's own installer.
2. Downloads the right `.dmg` for the Mac's architecture (Apple Silicon or
   Intel) from a configured URL — Google Drive share links are accepted and
   converted automatically (handles the "can't scan this file for viruses"
   large-file confirmation page).
3. Mounts it, copies `FileIT.app` into `/Applications`, then clears the
   Gatekeeper quarantine flag (`xattr -dr com.apple.quarantine`) — required
   because the app is ad-hoc signed, not notarized (see section 4).
4. Asks whether to install Indra OCR (default no, same as Windows): finds or
   installs Python 3.10+ (via Homebrew if needed), creates an isolated
   virtualenv at `~/Library/Application Support/FileIT-Indra/indra-ocr` (kept
   outside the app bundle, like Windows' `%LOCALAPPDATA%\FileIT-Indra`, so
   app updates/uninstalls never touch it), `pip install`s `indra-ocr`, writes
   `indra_ocr_command` into FileIT's `settings.json`, then pre-downloads the
   OCR models with a probe image.
5. Launches FileIT.

Run it with `--uninstall` to reverse all of this (asks separately about
removing Indra OCR and about removing FileIT's app data/logs), `--no-ocr` to
skip the OCR step non-interactively, or `--with-ocr` to force it.

Configure the download links before distributing it — either edit
`FILEIT_URL_ARM64` / `FILEIT_URL_X86_64` near the top of the script, or set
them as environment variables when running it:

```bash
FILEIT_URL_ARM64="https://drive.google.com/file/d/<id>/view" ./install-fileit.sh
```

Log: `~/Library/Logs/FileIT-Install/install.log`.

This script only installs the *app*; it does not touch Xcode Command Line
Tools, Rust or Node — those are build-time prerequisites covered in section 1
below, for people building FileIT from source, not for end users running it.

---

## 1. Set up a fresh Mac

```bash
xcode-select --install
```

Rust (stable):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

```bash
source "$HOME/.cargo/env"
```

Node.js 22 (pick one: Homebrew shown here, or the installer from nodejs.org):

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

```bash
brew install node@22
```

Indra OCR sidecar (optional — without it PDFs/images are classified from
their names and paths only, and the Dashboard says "OCR není nainstalován"):

```bash
python3 -m pip install --user indra-ocr
```

`pip install --user` puts it in `~/Library/Python/3.x/bin`, which is not on
the PATH of apps started from Finder. FileIT also looks there (and in
`/opt/homebrew/bin`, `/usr/local/bin`, `~/.local/bin`), but the reliable way
is to give the full path, either as an environment variable (works for
`npm run tauri dev` started from a terminal):

```bash
export INDRA_OCR_COMMAND="$(python3 -m site --user-base)/bin/indra-ocr"
```

or in the FileIT settings file (works for the `.app` too; the file is
created on first start):

```text
~/Library/Application Support/cz.fileit.desktop/settings.json
```

```json
{
  "indra_ocr_command": "/Users/<you>/Library/Python/3.12/bin/indra-ocr",
  "indra_ocr_models": null,
  "indra_ocr_cores": null
}
```

Lookup order: environment variable → settings file → default `indra-ocr`
on PATH. Changes to the file apply without restarting the app.

(Packaged sidecars named `indra-ocr-aarch64-apple-darwin` /
`indra-ocr-x86_64-apple-darwin` come in a later phase.)

## 2. Get the code and run in dev mode

```bash
git clone https://github.com/jardaKarlik/fileIT.git
```

```bash
cd fileIT && git checkout feature/macos-port
```

```bash
npm ci
```

To scan only the sample data instead of the whole home folder, create
`.env.local` in the repo root:

```bash
echo "VITE_FILEIT_SCAN_ROOT=$PWD/sample-vault" > .env.local
```

Start the app (first build takes several minutes):

```bash
npm run tauri dev
```

In dev mode macOS attributes folder access to **Terminal** (or the terminal
app you use), so privacy prompts and Full Disk Access apply to that app,
not to FileIT.

## 3. Build an unsigned .app / .dmg

```bash
npm run tauri build -- --debug
```

Output:

```text
src-tauri/target/debug/bundle/macos/FileIT.app
src-tauri/target/debug/bundle/dmg/FileIT_1.1.0_aarch64.dmg
```

(Drop `--debug` for an optimised build under `target/release/…`.) The
bundle is ad-hoc signed (`signingIdentity: "-"`), which Apple Silicon
requires to run at all; it is not signed with a Developer ID.

`.env.local` is baked in at build time: remove it before building if the
`.app` should scan the home folder.

## 4. Open an unsigned build

A build you made yourself on the same Mac usually opens directly. A `.dmg`
downloaded from CI or copied from another machine is quarantined, and
Gatekeeper says the app "cannot be opened because the developer cannot be
verified" or "is damaged". Either:

- **Finder:** right-click (Control-click) `FileIT.app` → **Open** → **Open**.
  On macOS 15 (Sequoia) and later this may not offer Open: try to open the
  app once, then go to **System Settings → Privacy & Security**, scroll down
  and click **Open Anyway**.
- **Terminal:** remove the quarantine attribute:

```bash
xattr -dr com.apple.quarantine /Applications/FileIT.app
```

## 5. Privacy prompts and Full Disk Access

macOS asks the first time FileIT reads Desktop, Documents, Downloads,
external drives, network drives or cloud (File Provider) folders. Click
**Allow**. If a prompt is denied, or a folder is protected, the scan does
**not** fail: the folder is skipped, and the Dashboard lists the skipped
folders with instructions.

To allow everything at once: **System Settings → Privacy & Security → Full
Disk Access**, turn on FileIT (add it with **+** if it is not listed), then
quit and restart FileIT. For `npm run tauri dev`, give Full Disk Access to
your terminal app instead.

To reset the answers while testing:

```bash
tccutil reset All cz.fileit.desktop
```

---

## 6. Manual test checklist (macOS VM)

Record macOS version, chip (Apple Silicon / Intel) and build (dev / debug
`.app` / `.dmg`) with your results. Logs: `~/Library/Logs/cz.fileit.desktop/fileit.log`
(dev mode: also the terminal output).

### Build and start
- [ ] `npm ci`, `npm run build` and `cargo test` (in `src-tauri`) pass.
- [ ] `npm run tauri dev` opens the FileIT window; no errors in the terminal.
- [ ] `npm run tauri build -- --debug` produces `FileIT.app` and a `.dmg`.
- [ ] The `.dmg` mounts and shows FileIT next to an Applications link; drag-install works.
- [ ] The installed app opens (after right-click → Open / `xattr`, if quarantined).
- [ ] Dock icon and window title show FileIT.
- [ ] `~/Library/Application Support/cz.fileit.desktop/settings.json` exists after the first start.

### Home screen
- [ ] "Co se prohledá" lists **Domovská složka** (your home path), and any cloud folders that exist (e.g. **Cloud OneDrive-Personal**, **iCloud Drive**).
  With `.env.local` set: **Vývojová složka** with the sample-vault path.
- [ ] The scope text describes the home folder (no "C:\Klienti", no "Windows").
- [ ] The Full Disk Access note is shown.
- [ ] The OCR switch reports whether Indra is installed.

### Mode 1 (analysis) on sample-vault
Dev mode with `.env.local` pointing at `sample-vault`, or for the `.app`
copy the folder to `~/Documents/sample-vault` and scan the default roots.
- [ ] Start "Jen analyzovat" → choose file types → scan finishes, no crash.
- [ ] Dashboard: all 15 sample files counted; institutions found for the
      CEZ, Ceska-sporitelna, O2 and VZP folders; `Neidentifikovano` stays unknown.
- [ ] With Indra installed: `real-invoice-test.pdf` gets text (OCR on), the Dashboard shows OCR "Nainstalováno".
- [ ] Without Indra: the scan still completes; PDFs are classified from names/paths.
- [ ] Re-scan ("Znovu prohlédnout") is faster; nothing is re-read (log: "Re-scan diff: N unchanged").
- [ ] Export relace works and the saved file opens.

### Czech names (NFD)
Create decomposed copies (APFS keeps the form you give it):

```bash
mkdir -p ~/Documents/nfd-test && cd ~/Documents/nfd-test && python3 - <<'EOF'
import unicodedata, shutil, os
src = os.path.expanduser("~/fileIT/sample-vault/real-invoice-test.pdf")
for name in ["Faktura Dvořák ČSOB.pdf", "Smlouva Řehoř Kooperativa.pdf", "Výpis Česká spořitelna.pdf"]:
    shutil.copy(src, unicodedata.normalize("NFD", name))
d = unicodedata.normalize("NFD", "Klienti Šťastný")
os.makedirs(d, exist_ok=True)
shutil.copy(src, os.path.join(d, unicodedata.normalize("NFD", "Pojistná smlouva Allianz.pdf")))
print([ (n, unicodedata.is_normalized("NFC", n)) for n in os.listdir(".") ])
EOF
```

- [ ] The script prints `False` (= NFD) for the names.
- [ ] After a scan the names display correctly (no split accents) and ČSOB,
      Česká spořitelna, Kooperativa, Allianz are recognised from the file names.
- [ ] Re-scan reuses these files' results (not shown as new).

### Skips, cloud and permissions
- [ ] With the default roots: nothing from `~/Library`, `/Applications`, `*.app`,
      `Photos Library.photoslibrary`, `.Trash` appears in the results (check the log for "Skip").
- [ ] Deny the Desktop (or Documents) prompt → scan completes; the Dashboard
      shows "N složek se nepodařilo otevřít" with the folder and the Full Disk Access steps.
- [ ] Grant Full Disk Access, restart FileIT, re-scan → the note is gone.
- [ ] External USB drive or disk image (`hdiutil create -size 50m -fs ExFAT -volname TEST test.dmg`, then open it):
      pick it as a scan source → scan completes (prompt allowed or skipped).
- [ ] iCloud Drive with "Optimise Mac Storage": a file removed from the Mac
      (right-click → Remove Download) is **not** downloaded by the scan; the log
      reports cloud-only placeholders skipped.
- [ ] OneDrive / Google Drive (if signed in): the CloudStorage folder is listed on Home;
      online-only files are not downloaded; on the Restructure screen the cloud is offered as a destination.

### Restructure (Mode 2) — on test data only
- [ ] Default target is `~/FileIT/Organized` (shown with `/`, not `\`).
- [ ] Preview → run on the sample-vault copy; files are moved, nothing is overwritten:
      put `Faktura.pdf` and `faktura.pdf` (different content) in two source folders →
      the result has `Faktura.pdf` and `faktura_2.pdf` (or `Faktura_2.pdf`), both contents intact.
- [ ] Confirmation screen: "Otevřete cílovou složku ve Finderu"; "Otevřít cílovou složku" reveals it in Finder.
- [ ] "Vrátit zpět" restores all files to their original paths.

### Indra settings file
- [ ] Unset `INDRA_OCR_COMMAND`, put the full path in `settings.json` → the
      Dashboard shows OCR installed without restarting.
- [ ] With a wrong path, the "not installed" message names the settings file and a macOS example path.

### Windows (regression, on a Windows machine)
- [ ] Home still lists `Disk C:\` (and other fixed drives) and the Windows scope text.
- [ ] Default target `C:\ReOrganized`; "v Průzkumníku"; Explorer opens the folder.
- [ ] Indra via HKCU\Environment still works.
