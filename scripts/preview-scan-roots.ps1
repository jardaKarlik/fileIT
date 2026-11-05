# scripts/preview-scan-roots.ps1
#
# READ-ONLY preview of which top-level folders FileIT v1.1 would scan.
# It mirrors the scanner rules in src-tauri/src/scanner/mod.rs:
#   - every local fixed drive (C:\, D:\, ...)
#   - folders on a drive root owned by SYSTEM / Administrators / a service
#     (TrustedInstaller) are treated as installed software -> SKIP
#   - known system / tool folder names -> SKIP
#   - hidden names starting with "." -> SKIP
#   - under <drive>\Users only the current user's profile -> SCAN
#
# Nothing is changed. Run from PowerShell:
#   powershell -ExecutionPolicy Bypass -File .\scripts\preview-scan-roots.ps1
# Optional: -ShowOwner to add an Owner column, -Csv <path> to export.

param(
    [switch]$ShowOwner,
    [string]$Csv
)

$skipNames = @(
    'Windows','Program Files','Program Files (x86)','ProgramData','$Recycle.Bin',
    'System Volume Information','Recovery','$SysReset','MSOCache','$WinREAgent',
    'PerfLogs','Intel','AMD','Windows.old','$Windows.~BT','$Windows.~WS','$GetCurrent',
    'Config.Msi','OneDriveTemp','msys64','cygwin64','AppData','node_modules','target',
    'dist','build','__pycache__','venv','Native Instruments','Samples','Claude',
    'iZotope','Lexicon','League of Legends','Magda','Madga','McAfee','src-tauri'
)

function Test-SystemSid([string]$sid) {
    return ($sid -in @('S-1-5-18','S-1-5-19','S-1-5-20','S-1-5-32-544')) -or $sid.StartsWith('S-1-5-80-')
}

function Get-OwnerInfo([string]$path) {
    try {
        $acl = Get-Acl -LiteralPath $path -ErrorAction Stop
        $sid = $acl.GetOwner([System.Security.Principal.SecurityIdentifier]).Value
        return [pscustomobject]@{ Sid = $sid; Name = $acl.Owner }
    } catch {
        return [pscustomobject]@{ Sid = ''; Name = '(no access)' }
    }
}

function Get-Verdict([string]$name, [string]$sid) {
    if ($name.StartsWith('.'))                     { return @('SKIP', 'hidden (dot) folder') }
    if ($skipNames -contains $name)                { return @('SKIP', 'system/tool folder name') }
    if ($sid -and (Test-SystemSid $sid))           { return @('SKIP', 'installed software (system-owned)') }
    return @('SCAN', 'owned by a person')
}

$userProfile = $env:USERPROFILE
$rows = @()

$drives = Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=3' | Select-Object -ExpandProperty DeviceID
Write-Host "Local fixed drives: $($drives -join ', ')" -ForegroundColor Cyan
Write-Host "Current user profile: $userProfile`n" -ForegroundColor Cyan

foreach ($d in $drives) {
    $root = "$d\"
    Get-ChildItem -LiteralPath $root -Directory -Force -ErrorAction SilentlyContinue | ForEach-Object {
        $owner = Get-OwnerInfo $_.FullName

        if ($_.Name -ieq 'Users') {
            # The scanner never treats Users as software; it scans only your profile inside it.
            $rows += [pscustomobject]@{ Verdict='SCAN'; Folder=$_.FullName; Reason='only your profile inside'; Owner=$owner.Name }
            Get-ChildItem -LiteralPath $_.FullName -Directory -Force -ErrorAction SilentlyContinue | ForEach-Object {
                $u = Get-OwnerInfo $_.FullName
                if ($_.FullName -ieq $userProfile) {
                    $rows += [pscustomobject]@{ Verdict='SCAN'; Folder=$_.FullName; Reason='your profile (AppData skipped)'; Owner=$u.Name }
                } else {
                    $rows += [pscustomobject]@{ Verdict='SKIP'; Folder=$_.FullName; Reason='other user / Public / Default'; Owner=$u.Name }
                }
            }
            return
        }

        $v = Get-Verdict $_.Name $owner.Sid
        $rows += [pscustomobject]@{ Verdict=$v[0]; Folder=$_.FullName; Reason=$v[1]; Owner=$owner.Name }
    }
}

$cols = if ($ShowOwner) { 'Verdict','Folder','Reason','Owner' } else { 'Verdict','Folder','Reason' }

Write-Host "=== Folders FileIT WILL scan ===" -ForegroundColor Green
$rows | Where-Object Verdict -eq 'SCAN' | Format-Table $cols -AutoSize

Write-Host "=== Folders FileIT will SKIP ===" -ForegroundColor Yellow
$rows | Where-Object Verdict -eq 'SKIP' | Format-Table $cols -AutoSize

if ($Csv) {
    $rows | Export-Csv -LiteralPath $Csv -NoTypeInformation -Encoding UTF8
    Write-Host "Saved to $Csv"
}

Write-Host "Check: every folder YOU created should be under 'WILL scan'; programs and Windows under 'SKIP'." -ForegroundColor Cyan
Write-Host "Note: subfolders named like the skip list (e.g. 'build', 'Samples') are skipped at any depth too."
