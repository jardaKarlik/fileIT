# scripts/preview-scan-roots.ps1
#
# READ-ONLY preview of which folders FileIT v1.1 would scan.
# Mirrors the scanner rules in src-tauri/src/scanner/mod.rs:
#   - every local fixed drive (C:\, D:\, ...)
#   - symlinks / junctions -> SKIP (never followed)
#   - known system / tool folder names, names starting with "." -> SKIP
#   - drive-root names starting with "$" -> SKIP
#   - antivirus vendor names (AVG, ESET, Norton, ...) -> SKIP
#   - drive-root folders owned by SYSTEM / a service (TrustedInstaller) -> SKIP
#   - application / code folders, judged from a sample (3 levels, 3000 entries):
#       code project marker (.git, node_modules, package.json, *.sln, ...) -> SKIP
#       .exe/.dll/.msi/.sys and no documents                              -> SKIP
#       2+ of bin/lib/include/src/sources/share/plugins/runtime directly
#       inside, or 1 of them and no documents                             -> SKIP
#       app/config/code files >= 10 and >= 5x the documents              -> SKIP
#   - under <drive>\Users only the current user's profile; inside it the
#     well-known folders (Documents, Desktop, Downloads, OneDrive, ...) are
#     always scanned and their direct subfolders are judged like above.
#
# Nothing is changed. Run from PowerShell:
#   powershell -ExecutionPolicy Bypass -File .\scripts\preview-scan-roots.ps1
# Options: -ShowOwner (Owner column), -ShowCounts (sample counts),
#          -Deep (also judge subfolders of Documents/Desktop/Downloads/OneDrive),
#          -Csv <path> (export).

param(
    [switch]$ShowOwner,
    [switch]$ShowCounts,
    [switch]$Deep,
    [string]$Csv
)

$skipNames = @(
    'Windows','Program Files','Program Files (x86)','ProgramData','$Recycle.Bin',
    'System Volume Information','Recovery','$SysReset','MSOCache','$WinREAgent',
    'PerfLogs','Intel','AMD','Windows.old','$Windows.~BT','$Windows.~WS','$GetCurrent',
    'Config.Msi','OneDriveTemp','msys64','cygwin64','XboxGames','WindowsApps','boot',
    'AppData','node_modules','target','dist','build','__pycache__','venv',
    'Native Instruments','Samples','Claude','iZotope','Lexicon','League of Legends',
    'Magda','Madga','McAfee','src-tauri'
)

$avVendors = @('avg','avast','eset','norton','symantec','mcafee','kaspersky','bitdefender',
    'avira','sophos','malwarebytes','webroot','comodo','drweb','gdata','fsecure','trendmicro',
    'emsisoft','zonealarm','quickheal','bullguard','totalav','panda','defender')

$userContainers = @('Documents','Desktop','Downloads','Pictures','Music','Videos',
    'OneDrive','Dropbox','Google Drive','iCloudDrive','Box')

$appExt = @('exe','dll','sys','msi','msix','appx','cab','ocx','drv','jar','class','pyc','pyd',
    'so','dylib','lib','o','obj','pdb','node','wasm','pak','asar','json','md','yml','yaml',
    'toml','ini','cfg','conf','config','lock','xaml','manifest','plist','properties','js',
    'mjs','cjs','ts','tsx','jsx','css','scss','less','map','html','htm','vue','svelte','py',
    'rs','go','java','kt','cs','cpp','cc','c','h','hpp','swift','rb','php','sh','ps1','psm1',
    'bat','cmd','lua','sql')
$binExt = @('exe','dll','msi','sys')
$docExt = @('pdf','doc','docx','xls','xlsx','pptx','odt','ods','odp','msg','eml','csv','rtf')
$projectMarkers = @('.git','node_modules','package.json','Cargo.toml','pyproject.toml',
    'requirements.txt','go.mod','pom.xml','build.gradle')
$layoutDirs = @('bin','lib','lib64','include','src','sources','share','plugins','runtime')

function Test-SystemSid([string]$sid) {
    return ($sid -in @('S-1-5-18','S-1-5-19','S-1-5-20')) -or $sid.StartsWith('S-1-5-80-')
}

function Test-AvName([string]$name) {
    $lower = $name.ToLower()
    $squashed = ($lower -replace '[^a-z0-9]', '')
    foreach ($v in @('fsecure','trendmicro','gdata','drweb')) { if ($squashed.Contains($v)) { return $true } }
    foreach ($tok in ($lower -split '[^a-z0-9]+')) { if ($avVendors -contains $tok) { return $true } }
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

# Sample a folder 3 levels deep, at most 3000 entries, not following links.
function Get-Sample([string]$path) {
    $s = [pscustomobject]@{ App = 0; Bin = 0; Doc = 0; Layout = 0; Marker = $false }
    $seen = 0
    $queue = New-Object System.Collections.Queue
    $queue.Enqueue(@($path, 0))
    while ($queue.Count -gt 0 -and $seen -lt 3000) {
        $item = $queue.Dequeue(); $dir = $item[0]; $depth = $item[1]
        foreach ($c in (Get-ChildItem -LiteralPath $dir -Force -ErrorAction SilentlyContinue)) {
            $seen++
            if ($seen -gt 3000) { break }
            $name = $c.Name
            if (($projectMarkers -contains $name) -or $name.ToLower().EndsWith('.sln')) { $s.Marker = $true; return $s }
            if ($c.PSIsContainer) {
                if ($depth -eq 0 -and ($layoutDirs -contains $name)) { $s.Layout++ }
                if (-not $c.LinkType -and ($depth + 1) -lt 3) { $queue.Enqueue(@($c.FullName, ($depth + 1))) }
                continue
            }
            $ext = $c.Extension.TrimStart('.').ToLower()
            if ($binExt -contains $ext) { $s.Bin++ }
            if ($appExt -contains $ext) { $s.App++ } elseif ($docExt -contains $ext) { $s.Doc++ }
        }
    }
    return $s
}

function Get-SampleVerdict($s) {
    if ($s.Marker) { return 'code project' }
    if ($s.Bin -gt 0 -and $s.Doc -eq 0) { return 'installed program' }
    if ($s.Layout -ge 2 -or ($s.Layout -eq 1 -and $s.Doc -eq 0)) { return 'program layout (bin/lib/src)' }
    if ($s.App -ge 10 -and $s.App -ge 5 * [Math]::Max($s.Doc, 1)) { return 'application / config files' }
    return $null
}

$rows = New-Object System.Collections.ArrayList

function Add-Row($verdict, $folder, $reason, $owner, $sample) {
    $counts = if ($sample) { "app=$($sample.App) bin=$($sample.Bin) doc=$($sample.Doc) layout=$($sample.Layout)" } else { '' }
    [void]$rows.Add([pscustomobject]@{ Verdict=$verdict; Folder=$folder; Reason=$reason; Owner=$owner; Sample=$counts })
}

# Judge one folder at an app-check level.
function Test-Folder($item, [bool]$atDriveRoot) {
    $name = $item.Name
    $owner = if ($ShowOwner) { (Get-OwnerInfo $item.FullName).Name } else { '' }
    if ($item.LinkType) {
        Add-Row 'SKIP' "$($item.FullName) -> $(@($item.Target)[0])" "$($item.LinkType) (links are not followed)" $owner $null; return
    }
    if ($name.StartsWith('.'))                   { Add-Row 'SKIP' $item.FullName 'hidden folder' $owner $null; return }
    if ($atDriveRoot -and $name.StartsWith('$')) { Add-Row 'SKIP' $item.FullName 'system folder ($)' $owner $null; return }
    if ($skipNames -contains $name)              { Add-Row 'SKIP' $item.FullName 'system/tool folder name' $owner $null; return }
    if (Test-AvName $name)                       { Add-Row 'SKIP' $item.FullName 'antivirus' $owner $null; return }
    if ($atDriveRoot) {
        $o = Get-OwnerInfo $item.FullName
        if ($o.Sid -and (Test-SystemSid $o.Sid)) { Add-Row 'SKIP' $item.FullName 'owned by system' $o.Name $null; return }
    }
    $sample = Get-Sample $item.FullName
    $v = Get-SampleVerdict $sample
    if ($v) { Add-Row 'SKIP' $item.FullName $v $owner $sample } else { Add-Row 'SCAN' $item.FullName 'user folder' $owner $sample }
}

$userProfile = $env:USERPROFILE
$drives = Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=3' | Select-Object -ExpandProperty DeviceID
Write-Host "Local fixed drives: $($drives -join ', ')" -ForegroundColor Cyan
Write-Host "Current user profile: $userProfile" -ForegroundColor Cyan
Write-Host "Sampling folders (can take a minute)...`n" -ForegroundColor Cyan

foreach ($d in $drives) {
    foreach ($item in (Get-ChildItem -LiteralPath "$d\" -Directory -Force -ErrorAction SilentlyContinue)) {
        if ($item.Name -ieq 'Users') {
            Add-Row 'SCAN' $item.FullName 'only your profile inside' '' $null
            foreach ($u in (Get-ChildItem -LiteralPath $item.FullName -Directory -Force -ErrorAction SilentlyContinue)) {
                if ($u.FullName -ine $userProfile) { Add-Row 'SKIP' $u.FullName 'other user / Public / Default' '' $null; continue }
                Add-Row 'SCAN' $u.FullName 'your profile' '' $null
                foreach ($p in (Get-ChildItem -LiteralPath $u.FullName -Directory -Force -ErrorAction SilentlyContinue)) {
                    $isContainer = ($userContainers -contains $p.Name) -or $p.Name.StartsWith('OneDrive')
                    if ($isContainer -and -not $p.LinkType) {
                        Add-Row 'SCAN' $p.FullName 'user folder (always scanned)' '' $null
                        if ($Deep) {
                            foreach ($c in (Get-ChildItem -LiteralPath $p.FullName -Directory -Force -ErrorAction SilentlyContinue)) { Test-Folder $c $false }
                        }
                    } else {
                        Test-Folder $p $false
                    }
                }
            }
            continue
        }
        Test-Folder $item $true
    }
}

$cols = @('Verdict','Folder','Reason')
if ($ShowOwner)  { $cols += 'Owner' }
if ($ShowCounts) { $cols += 'Sample' }

Write-Host "=== Folders FileIT WILL scan ===" -ForegroundColor Green
$rows | Where-Object Verdict -eq 'SCAN' | Format-Table $cols -AutoSize -Wrap

Write-Host "=== Folders FileIT will SKIP ===" -ForegroundColor Yellow
$rows | Where-Object Verdict -eq 'SKIP' | Format-Table $cols -AutoSize -Wrap

if ($Csv) {
    $rows | Export-Csv -LiteralPath $Csv -NoTypeInformation -Encoding UTF8
    Write-Host "Saved to $Csv"
}

Write-Host "Check: folders YOU created with documents -> 'WILL scan'; programs, code and Windows -> 'SKIP'." -ForegroundColor Cyan
Write-Host "Note: subfolders named like the skip list (e.g. 'build', 'Samples') are skipped at any depth too."
