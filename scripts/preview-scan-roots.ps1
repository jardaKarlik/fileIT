# scripts/preview-scan-roots.ps1
#
# READ-ONLY preview of which top-level folders FileIT v1.1 would scan.
# It mirrors the scanner rules in src-tauri/src/scanner/mod.rs:
#   - every local fixed drive (C:\, D:\, ...)
#   - folders on a drive root owned by SYSTEM / a service (TrustedInstaller)
#     -> SKIP (Administrators does NOT count: mklink / elevated tools give it)
#   - folders that look like a program (.exe/.dll at top or in bin\, and no
#     PDF/Office/e-mail documents at top) -> SKIP
#   - known system / tool folder names, names starting with "." or "$" -> SKIP
#   - symlinks / junctions -> SKIP (never followed)
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
    'Config.Msi','OneDriveTemp','msys64','cygwin64','XboxGames','WindowsApps','AppData','node_modules','target',
    'dist','build','__pycache__','venv','Native Instruments','Samples','Claude',
    'iZotope','Lexicon','League of Legends','Magda','Madga','McAfee','src-tauri'
)

function Test-SystemSid([string]$sid) {
    return ($sid -in @('S-1-5-18','S-1-5-19','S-1-5-20')) -or $sid.StartsWith('S-1-5-80-')
}

$docExt = @('.pdf','.doc','.docx','.xls','.xlsx','.pptx','.msg','.eml')

function Test-ProgramFolder([string]$path) {
    $files = Get-ChildItem -LiteralPath $path -File -Force -ErrorAction SilentlyContinue | Select-Object -First 2000
    if ($files | Where-Object { $docExt -contains $_.Extension.ToLower() }) { return $false }
    if ($files | Where-Object { @('.exe','.dll') -contains $_.Extension.ToLower() }) { return $true }
    $bin = Join-Path $path 'bin'
    if (Test-Path -LiteralPath $bin -PathType Container) {
        $binExe = Get-ChildItem -LiteralPath $bin -File -Force -ErrorAction SilentlyContinue |
            Where-Object { @('.exe','.dll') -contains $_.Extension.ToLower() } | Select-Object -First 1
        if ($binExe) { return $true }
    }
    return $false
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

function Get-Verdict([string]$path, [string]$name, [string]$sid) {
    if ($name.StartsWith('.') -or $name.StartsWith('$')) { return @('SKIP', 'hidden / system folder') }
    if ($skipNames -contains $name)                      { return @('SKIP', 'system/tool folder name') }
    if ($sid -and (Test-SystemSid $sid))                 { return @('SKIP', 'owned by system') }
    if (Test-ProgramFolder $path)                        { return @('SKIP', 'looks like a program (exe/dll)') }
    return @('SCAN', 'user folder')
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

        if ($_.LinkType) {
            # Symlinks / junctions are never followed by FileIT.
            $target = @($_.Target)[0]
            $rows += [pscustomobject]@{ Verdict='SKIP'; Folder="$($_.FullName) -> $target"; Reason="$($_.LinkType) (links are not followed)"; Owner=$owner.Name }
            return
        }

        $v = Get-Verdict $_.FullName $_.Name $owner.Sid
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
