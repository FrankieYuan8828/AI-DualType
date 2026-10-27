# 构建 MC-TypeEnglish 的版本化 MSI 安装包与 Setup.exe
#
# 用法： powershell -ExecutionPolicy Bypass -File tools\build_msi.ps1 [-Version 1.0.3]

param(
    [string]$Version = "1.0.3"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$versionDir = Join-Path "dist" "v$Version"
$distFolder = Join-Path $versionDir "MC-TypeEnglish"
$legacyDist = "dist\MC-TypeEnglish"

New-Item -ItemType Directory -Force -Path $distFolder | Out-Null
New-Item -ItemType Directory -Force -Path $legacyDist | Out-Null

# 1. 确保 dist\MC-TypeEnglish 和 dist\v$Version\MC-TypeEnglish 均为最新
Write-Host "== 1/3 同步发布目录 ($versionDir) =="
& powershell -ExecutionPolicy Bypass -File tools\stage_dist.ps1 -Dist $legacyDist
if ($LASTEXITCODE -ne 0) { throw "stage_dist.ps1 (legacy) 失败" }

# 同步复制到独立版本文件夹
Copy-Item "$legacyDist\*" $distFolder -Recurse -Force

# 2. 调用 WiX 构建 MSI
Write-Host "== 2/3 构建 MSI 安装包 =="
$wix = (Get-Command wix -ErrorAction SilentlyContinue).Source
if (-not $wix) {
    $fallback = "$env:USERPROFILE\.dotnet\tools\wix.exe"
    if (Test-Path $fallback) { $wix = $fallback }
    else { throw "未找到 wix.exe，请先安装 WiX CLI (dotnet tool install --global wix)" }
}

$resolvedDist = (Resolve-Path $distFolder).Path
$msiOut = Join-Path $versionDir "MC-TypeToAnything-$Version.msi"

& $wix build -arch x64 `
    -d DistDir="$resolvedDist" `
    -d RepoDir="$root" `
    packaging\mcte.wxs `
    -o "$msiOut"

if ($LASTEXITCODE -ne 0) { throw "WiX build 失败" }

# 3. 构建单文件 exe 安装包
Write-Host "== 3/3 构建 Setup.exe =="
cargo build --release -p mcte-setup
if ($LASTEXITCODE -ne 0) { throw "cargo build mcte-setup 失败" }
$setupExe = Join-Path $versionDir "MC-TypeToAnything-Setup-$Version.exe"
Copy-Item "target\release\mcte-setup.exe" $setupExe -Force
Copy-Item "target\release\mcte-setup.exe" (Join-Path $versionDir "MC-TypeToAnything-Setup.exe") -Force

Write-Host "== 完成 v$Version 发布产物 =="
Get-ChildItem $versionDir -File |
    ForEach-Object { "{0,-38} {1,10} bytes" -f $_.Name, $_.Length }
