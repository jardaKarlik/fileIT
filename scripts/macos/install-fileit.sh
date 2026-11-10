#!/bin/bash
# install-fileit.sh - one-click FileIT installer for macOS end users.
#
# Downloads FileIT from a cloud link, installs it to /Applications, clears
# the Gatekeeper quarantine flag (the app is ad-hoc signed, not notarized,
# so without this macOS refuses to open it), and optionally installs Indra
# OCR the same way the Windows setup wizard does (isolated environment
# outside the app itself, so app updates/uninstalls never touch it).
#
# Usage:
#   ./install-fileit.sh              install (asks about Indra OCR)
#   ./install-fileit.sh --uninstall  remove FileIT (and optionally Indra OCR)
#   ./install-fileit.sh --no-ocr     install without asking about OCR
#
# Configuration: set FILEIT_URL_ARM64 / FILEIT_URL_X86_64 below to your
# download links (Google Drive share links are accepted and converted
# automatically), or override at run time:
#   FILEIT_URL_ARM64="https://drive.google.com/file/d/XXXX/view" ./install-fileit.sh
#
# Log: ~/Library/Logs/FileIT-Install/install.log

set -u

# ── Configuration ────────────────────────────────────────────────────────
# Fill these in once you have the real links. Apple Silicon is the tested
# target today (per docs/MACOS.md); Intel is left blank until that's verified.
FILEIT_URL_ARM64="${FILEIT_URL_ARM64:-}"
FILEIT_URL_X86_64="${FILEIT_URL_X86_64:-}"

APP_NAME="FileIT.app"
INSTALL_DIR="/Applications"
INDRA_BASE="$HOME/Library/Application Support/FileIT-Indra"
INDRA_VENV="$INDRA_BASE/indra-ocr"
INDRA_EXE="$INDRA_VENV/bin/indra-ocr"
SETTINGS_DIR="$HOME/Library/Application Support/cz.fileit.desktop"
SETTINGS_FILE="$SETTINGS_DIR/settings.json"
LOG_DIR="$HOME/Library/Logs/FileIT-Install"
LOG_FILE="$LOG_DIR/install.log"
WORK_DIR="$(mktemp -d /tmp/fileit-install.XXXXXX)"

UNINSTALL=0
SKIP_OCR_PROMPT=0
FORCE_OCR=""
for arg in "$@"; do
  case "$arg" in
    --uninstall) UNINSTALL=1 ;;
    --no-ocr) SKIP_OCR_PROMPT=1; FORCE_OCR="no" ;;
    --with-ocr) SKIP_OCR_PROMPT=1; FORCE_OCR="yes" ;;
    *) echo "Unknown argument: $arg" >&2; exit 64 ;;
  esac
done

mkdir -p "$LOG_DIR"

log() {
  local line
  line="[$(date '+%Y-%m-%dT%H:%M:%S%z')] $*"
  echo "$line"
  echo "$line" >> "$LOG_FILE"
}

die() {
  log "ERROR: $*"
  cleanup
  exit 1
}

cleanup() {
  [ -n "${MOUNT_POINT:-}" ] && hdiutil detach "$MOUNT_POINT" -quiet 2>/dev/null
  rm -rf "$WORK_DIR" 2>/dev/null
}
trap cleanup EXIT

# ── Admin password, asked once ──────────────────────────────────────────
# Needed for: writing into /Applications on machines where it isn't
# user-writable, and for the Homebrew installer (which elevates itself
# internally when it needs to). Keeping credentials cached avoids a second
# prompt later in the script.
request_admin() {
  log "FileIT installer needs your password once, to install into /Applications."
  sudo -v || die "Admin authorization failed."
  # Keep the sudo timestamp alive for the life of this script.
  ( while true; do sudo -n true; sleep 60; kill -0 "$$" 2>/dev/null || exit; done ) &
  SUDO_KEEPALIVE_PID=$!
}

# ── Google Drive large-file download (bypasses the "can't scan for
#    viruses" interstitial for files over ~25 MB) ─────────────────────────
gdrive_id_from_url() {
  local url="$1"
  if [[ "$url" =~ /d/([a-zA-Z0-9_-]+) ]]; then
    echo "${BASH_REMATCH[1]}"
  elif [[ "$url" =~ id=([a-zA-Z0-9_-]+) ]]; then
    echo "${BASH_REMATCH[1]}"
  else
    echo "$url"   # assume it's already a bare file ID
  fi
}

download_file() {
  local url="$1" out="$2"
  if [[ "$url" != *drive.google.com* ]]; then
    log "Downloading $out ..."
    curl -fL --progress-bar "$url" -o "$out" || die "Download failed: $url"
    return
  fi

  local id cookie_jar confirm
  id="$(gdrive_id_from_url "$url")"
  cookie_jar="$WORK_DIR/gdrive-cookies.txt"
  log "Downloading from Google Drive (file id $id) ..."

  confirm="$(curl -sc "$cookie_jar" "https://drive.google.com/uc?export=download&id=${id}" \
    | grep -o 'confirm=[a-zA-Z0-9_-]*' | head -1 | sed 's/confirm=//')"

  if [ -n "$confirm" ]; then
    curl -fLb "$cookie_jar" \
      "https://drive.google.com/uc?export=download&confirm=${confirm}&id=${id}" \
      -o "$out" --progress-bar
  else
    curl -fLb "$cookie_jar" "https://drive.google.com/uc?export=download&id=${id}" \
      -o "$out" --progress-bar
  fi

  if [ ! -s "$out" ] || [ "$(stat -f%z "$out" 2>/dev/null || echo 0)" -lt 1000000 ]; then
    die "Google Drive download looks wrong (too small or missing). The link may need updating, or Drive's download-confirmation flow may have changed. File saved at: $out"
  fi
}

# ── Install ────────────────────────────────────────────────────────────
do_install() {
  log "=== FileIT installer starting ==="
  request_admin

  local arch url dmg
  arch="$(uname -m)"
  case "$arch" in
    arm64) url="$FILEIT_URL_ARM64" ;;
    x86_64) url="$FILEIT_URL_X86_64" ;;
    *) die "Unsupported architecture: $arch" ;;
  esac
  [ -n "$url" ] || die "No download URL configured for $arch. Set FILEIT_URL_ARM64 / FILEIT_URL_X86_64 at the top of this script."

  dmg="$WORK_DIR/FileIT.dmg"
  download_file "$url" "$dmg"

  log "Mounting disk image..."
  local attach_out
  attach_out="$(hdiutil attach "$dmg" -nobrowse -quiet 2>&1)" || die "Could not mount the downloaded disk image: $attach_out"
  MOUNT_POINT="$(echo "$attach_out" | grep -o '/Volumes/.*' | tail -1)"
  [ -d "$MOUNT_POINT" ] || die "Mounted image but could not find its volume path."

  local src_app
  src_app="$(find "$MOUNT_POINT" -maxdepth 1 -iname '*.app' | head -1)"
  [ -n "$src_app" ] || die "No .app bundle found inside the downloaded disk image."

  log "Installing to $INSTALL_DIR/$APP_NAME ..."
  if [ -d "$INSTALL_DIR/$APP_NAME" ]; then
    rm -rf "$INSTALL_DIR/$APP_NAME" 2>/dev/null || sudo rm -rf "$INSTALL_DIR/$APP_NAME" || die "Could not remove the previous version."
  fi
  ditto "$src_app" "$INSTALL_DIR/$APP_NAME" 2>/dev/null || sudo ditto "$src_app" "$INSTALL_DIR/$APP_NAME" || die "Could not copy FileIT into $INSTALL_DIR."

  log "Clearing the Gatekeeper quarantine flag (the app is ad-hoc signed, not notarized)..."
  xattr -dr com.apple.quarantine "$INSTALL_DIR/$APP_NAME" 2>/dev/null
  sudo xattr -dr com.apple.quarantine "$INSTALL_DIR/$APP_NAME" 2>/dev/null

  hdiutil detach "$MOUNT_POINT" -quiet 2>/dev/null
  MOUNT_POINT=""

  log "FileIT installed."

  maybe_install_indra

  log "=== Done. Launching FileIT. ==="
  open -a "$INSTALL_DIR/$APP_NAME" 2>/dev/null
}

# ── Indra OCR (optional, mirrors the Windows installer's prompt) ─────────
maybe_install_indra() {
  if [ -x "$INDRA_EXE" ]; then
    log "Indra OCR already installed: $INDRA_EXE"
    write_indra_setting
    return
  fi

  local answer="$FORCE_OCR"
  if [ "$SKIP_OCR_PROMPT" -eq 0 ]; then
    echo
    read -r -p "Install Indra OCR for reading scanned documents? Without it, FileIT sorts scanned PDFs/photos by filename and folder only. This downloads about 230 MB and takes a few minutes. [y/N] " reply
    case "$reply" in
      [yY]*) answer="yes" ;;
      *) answer="no" ;;
    esac
  fi
  [ "$answer" = "yes" ] || { log "Skipping Indra OCR."; return; }

  install_indra
}

find_python() {
  local candidates=(python3.13 python3.12 python3.11 python3.10 python3)
  for c in "${candidates[@]}"; do
    if command -v "$c" >/dev/null 2>&1; then
      local v
      v="$("$c" -c 'import sys; print("%d.%d" % sys.version_info[:2])' 2>/dev/null)"
      if [[ "$v" =~ ^3\.([0-9]+)$ ]] && [ "${BASH_REMATCH[1]}" -ge 10 ]; then
        command -v "$c"
        return 0
      fi
    fi
  done
  return 1
}

install_homebrew_if_missing() {
  if command -v brew >/dev/null 2>&1; then return; fi
  log "Homebrew not found - installing it (this may ask for your password again, that's Homebrew's own installer)..."
  NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)" \
    || die "Homebrew installation failed."
  if [[ "$(uname -m)" == "arm64" ]]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
  else
    eval "$(/usr/local/bin/brew shellenv)"
  fi
}

install_indra() {
  log "Setting up Indra OCR..."
  local py
  py="$(find_python)" || {
    install_homebrew_if_missing
    log "Installing Python via Homebrew..."
    brew install python@3.12 || die "Could not install Python."
    py="$(find_python)" || die "Python 3.10+ still not found after installing it."
  }
  log "Using Python: $py ($("$py" -c 'import sys; print("%d.%d"%sys.version_info[:2])'))"

  mkdir -p "$INDRA_BASE"
  log "Creating environment at $INDRA_VENV ..."
  "$py" -m venv "$INDRA_VENV" || die "Could not create the Python environment."

  log "Installing indra-ocr (about 150 MB, a few minutes)..."
  "$INDRA_VENV/bin/python" -m pip install --disable-pip-version-check --quiet indra-ocr \
    || die "pip install indra-ocr failed."

  [ -x "$INDRA_EXE" ] || die "$INDRA_EXE not found after install."

  write_indra_setting

  log "Downloading OCR models (about 76 MB, pre-warming on a test image)..."
  local probe="$WORK_DIR/probe.png"
  "$INDRA_VENV/bin/python" - "$probe" <<'PYEOF'
import sys
import numpy as np, cv2
img = np.full((80, 360), 255, np.uint8)
cv2.putText(img, "FileIT 2026", (10, 55), cv2.FONT_HERSHEY_SIMPLEX, 1.4, 0, 3)
cv2.imwrite(sys.argv[1], img)
PYEOF
  if [ -f "$probe" ]; then
    "$INDRA_EXE" "$probe" --format text >/dev/null 2>&1
    log "Model download / test run: exit $?"
  fi

  log "Indra OCR is ready."
}

# Writes indra_ocr_command into FileIT's settings.json without clobbering
# other keys the app may have already written there.
write_indra_setting() {
  mkdir -p "$SETTINGS_DIR"
  python3 - "$SETTINGS_FILE" "$INDRA_EXE" <<'PYEOF'
import json, sys, os
path, exe = sys.argv[1], sys.argv[2]
data = {}
if os.path.exists(path):
    try:
        with open(path) as f:
            data = json.load(f)
    except Exception:
        data = {}
data["indra_ocr_command"] = exe
data.setdefault("indra_ocr_models", None)
data.setdefault("indra_ocr_cores", None)
with open(path, "w") as f:
    json.dump(data, f, indent=2)
PYEOF
  log "Wrote indra_ocr_command to $SETTINGS_FILE"
}

# ── Uninstall ──────────────────────────────────────────────────────────
do_uninstall() {
  log "=== FileIT uninstaller starting ==="

  if [ -x "$INDRA_EXE" ]; then
    local reply="n"
    if [ "$SKIP_OCR_PROMPT" -eq 0 ]; then
      read -r -p "Also remove Indra OCR (about 230 MB)? [y/N] " reply
    elif [ "$FORCE_OCR" = "yes" ]; then
      reply="y"
    fi
    case "$reply" in
      [yY]*)
        rm -rf "$INDRA_VENV" 2>/dev/null
        log "Removed $INDRA_VENV"
        if [ -f "$SETTINGS_FILE" ]; then
          python3 - "$SETTINGS_FILE" <<'PYEOF'
import json, sys
path = sys.argv[1]
try:
    with open(path) as f:
        data = json.load(f)
    data.pop("indra_ocr_command", None)
    with open(path, "w") as f:
        json.dump(data, f, indent=2)
except Exception:
    pass
PYEOF
        fi
        ;;
      *) log "Keeping Indra OCR." ;;
    esac
  fi

  read -r -p "Remove FileIT itself from $INSTALL_DIR? [y/N] " reply
  case "$reply" in
    [yY]*)
      request_admin
      rm -rf "$INSTALL_DIR/$APP_NAME" 2>/dev/null || sudo rm -rf "$INSTALL_DIR/$APP_NAME"
      log "Removed $INSTALL_DIR/$APP_NAME"
      read -r -p "Also remove FileIT's settings and logs (Application Support, Logs)? [y/N] " reply2
      case "$reply2" in
        [yY]*)
          rm -rf "$SETTINGS_DIR" "$HOME/Library/Logs/cz.fileit.desktop"
          log "Removed app data and logs."
          ;;
      esac
      ;;
    *) log "Keeping FileIT installed." ;;
  esac

  log "=== Uninstall finished. ==="
}

# ── Entry point ────────────────────────────────────────────────────────
if [ "$UNINSTALL" -eq 1 ]; then
  do_uninstall
else
  do_install
fi
