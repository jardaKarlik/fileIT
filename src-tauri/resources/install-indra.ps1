# install-indra.ps1 - installs Indra OCR for FileIT (called by the setup wizard).
#
# - Finds Python 3.10+ (or installs Python 3.12 for the current user via winget)
# - Creates an isolated environment %LOCALAPPDATA%\FileIT-Indra\indra-ocr
#   (outside the app install folder %LOCALAPPDATA%\FileIT, so app updates and
#   uninstalls never touch it)
#   and installs the indra-ocr package into it
# - Sets the user variable INDRA_OCR_COMMAND (read by FileIT, also from HKCU)
# - Pre-downloads the OCR models with a tiny test image
#
# Exit codes: 0 ok, 2 no Python, 3 venv failed, 4 pip failed, 5 exe missing.
# Log: %LOCALAPPDATA%\FileIT-Indra\indra-install.log
#
#   powershell -ExecutionPolicy Bypass -File install-indra.ps1            install
#   powershell -ExecutionPolicy Bypass -File install-indra.ps1 -Uninstall remove

param([switch]$Uninstall)

# Native tools (pip, winget) write progress to stderr; never turn that into
# PowerShell errors. Success is judged by $LASTEXITCODE.
$ErrorActionPreference = 'Continue'

$base = Join-Path $env:LOCALAPPDATA 'FileIT-Indra'
$venv = Join-Path $base 'indra-ocr'
$venvPython = Join-Path $venv 'Scripts\python.exe'
$exe = Join-Path $venv 'Scripts\indra-ocr.exe'
$log = Join-Path $base 'indra-install.log'
New-Item -ItemType Directory -Force -Path $base | Out-Null

function Log([string]$msg) {
    $line = '[{0}] {1}' -f (Get-Date -Format 's'), $msg
    Write-Output $line
    Add-Content -LiteralPath $log -Value $line -Encoding UTF8
}

if ($Uninstall) {
    if (Test-Path -LiteralPath $venv) {
        Remove-Item -LiteralPath $venv -Recurse -Force -ErrorAction SilentlyContinue
        Log "Removed $venv"
    }
    if ([Environment]::GetEnvironmentVariable('INDRA_OCR_COMMAND', 'User') -eq $exe) {
        [Environment]::SetEnvironmentVariable('INDRA_OCR_COMMAND', $null, 'User')
        Log 'Removed INDRA_OCR_COMMAND'
    }
    exit 0
}

Log '=== Indra OCR setup for FileIT ==='

# Returns "3.x" for a working Python 3.10+, else $null.
function Get-PyVersion([string]$cmd, [string[]]$prefix) {
    try {
        $out = & $cmd @prefix -c "import sys; print('%d.%d' % sys.version_info[:2])" 2>$null
        if ($LASTEXITCODE -eq 0 -and "$out" -match '^3\.(\d+)$' -and [int]$Matches[1] -ge 10) { return "$out" }
    } catch { }
    return $null
}

function Find-Python {
    $candidates = @(
        @('py', @('-3.12')), @('py', @('-3.13')), @('py', @('-3.11')), @('py', @('-3.10')),
        @('py', @('-3')), @('python', @()), @('python3', @())
    )
    foreach ($c in $candidates) {
        $cmd = Get-Command $c[0] -ErrorAction SilentlyContinue
        if (-not $cmd) { continue }
        # The Microsoft Store alias (WindowsApps\python.exe) only opens the Store.
        if ($cmd.Source -like '*\WindowsApps\*') { continue }
        $v = Get-PyVersion $cmd.Source $c[1]
        if ($v) { return @{ Cmd = $cmd.Source; Prefix = $c[1]; Version = $v } }
    }
    # Fresh per-user installs that are not on PATH in this process yet.
    foreach ($ver in @('312', '313', '311', '310')) {
        $p = Join-Path $env:LOCALAPPDATA "Programs\Python\Python$ver\python.exe"
        if (Test-Path -LiteralPath $p) {
            $v = Get-PyVersion $p @()
            if ($v) { return @{ Cmd = $p; Prefix = @(); Version = $v } }
        }
    }
    return $null
}

function Install-PythonWithWinget {
    $winget = Get-Command winget -ErrorAction SilentlyContinue
    if (-not $winget) { Log 'winget not available'; return }
    Log 'No suitable Python found - installing Python 3.12 for this user via winget...'
    & $winget.Source install -e --id Python.Python.3.12 --scope user --silent `
        --accept-package-agreements --accept-source-agreements --disable-interactivity
    Log "winget exit code: $LASTEXITCODE"
}

if (Test-Path -LiteralPath $exe) {
    Log "Indra OCR already installed: $exe"
} else {
    $py = Find-Python
    if (-not $py) {
        Install-PythonWithWinget
        $py = Find-Python
    }
    if (-not $py) {
        Log 'ERROR: Python 3.10+ not found and could not be installed. Install Python from https://www.python.org/downloads/ and run this again.'
        exit 2
    }
    Log "Using Python $($py.Version): $($py.Cmd) $($py.Prefix -join ' ')"

    Log "Creating environment $venv"
    & $py.Cmd @($py.Prefix) -m venv $venv
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $venvPython)) {
        Log "ERROR: creating the environment failed (exit $LASTEXITCODE)"
        exit 3
    }

    Log 'Installing indra-ocr (about 150 MB download, a few minutes)...'
    & $venvPython -m pip install --disable-pip-version-check --no-input indra-ocr
    if ($LASTEXITCODE -ne 0) {
        Log "ERROR: pip install failed (exit $LASTEXITCODE)"
        exit 4
    }
}

if (-not (Test-Path -LiteralPath $exe)) {
    Log "ERROR: $exe not found after install"
    exit 5
}

[Environment]::SetEnvironmentVariable('INDRA_OCR_COMMAND', $exe, 'User')
Log "INDRA_OCR_COMMAND = $exe"

# Pre-download the models (~76 MB) so the first scan does not wait for them.
# Non-fatal: FileIT downloads them on first use otherwise.
Log 'Downloading OCR models (about 76 MB)...'
$probe = Join-Path $env:TEMP 'fileit-indra-probe.png'
& $venvPython -c "import numpy as np, cv2; img = np.full((80, 360), 255, np.uint8); cv2.putText(img, 'FileIT 2026', (10, 55), cv2.FONT_HERSHEY_SIMPLEX, 1.4, 0, 3); cv2.imwrite(r'$probe', img)"
if (Test-Path -LiteralPath $probe) {
    & $exe $probe --format text | Out-Null
    Log "Model download / test run exit code: $LASTEXITCODE"
    Remove-Item -LiteralPath $probe -Force -ErrorAction SilentlyContinue
}

Log 'Indra OCR is ready.'
exit 0
