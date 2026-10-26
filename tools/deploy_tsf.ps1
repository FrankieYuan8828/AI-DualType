# 开发迭代用：把最新 TSF DLL 装成「带哈希后缀」的新文件名，并把 CLSID 指过去。
#
# 为什么不直接覆盖 mcte_tsf.dll：explorer.exe / 终端等长驻进程会一直映射着它，
# 文件被占用时覆盖会失败。带哈希后缀 + 改注册表可以在不重启 explorer 的前提下
# 让**新进程**加载到新版本（正式安装仍然用固定文件名，见 packaging\Install.ps1）。
#
# 用法： powershell -ExecutionPolicy Bypass -File tools\deploy_tsf.ps1

param(
    [string]$InstallDir = "$env:ProgramFiles\MC-TypeEnglish"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

$identity  = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host "需要管理员权限，正在请求提权…" -ForegroundColor Yellow
    Start-Process powershell -Verb RunAs -ArgumentList @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $PSCommandPath
    ) -Wait
    Write-Host "提权进程结束，校验部署结果：" -ForegroundColor Yellow
    $clsid = '{7C3E0C11-5F6A-4E52-9B2C-A1D47B2E9F31}'
    $registered = (Get-ItemProperty "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32").'(default)'
    Write-Host "  当前 InprocServer32 = $registered"
    if (Test-Path $registered) {
        Write-Host ("  文件 sha256={0}" -f (Get-FileHash $registered -Algorithm SHA256).Hash.Substring(0,16))
    }
    exit 0
}

$src64 = Join-Path $root 'target\release\mcte_tsf.dll'
$src86 = Join-Path $root 'target\i686-pc-windows-msvc\release\mcte_tsf.dll'
foreach ($src in @($src64, $src86)) {
    if (-not (Test-Path $src)) { throw "缺少构建产物: $src" }
}

$hash64 = (Get-FileHash $src64 -Algorithm SHA256).Hash
$suffix = $hash64.Substring(0, 8).ToLower()
$dst64 = Join-Path $InstallDir "mcte_tsf_$suffix.dll"
$dst86 = Join-Path $InstallDir "mcte_tsf_x86_$suffix.dll"

Copy-Item $src64 $dst64 -Force
Copy-Item $src86 $dst86 -Force
Write-Host ("  x64 -> {0} (sha {1})" -f (Split-Path -Leaf $dst64), $suffix)
Write-Host ("  x86 -> {0}" -f (Split-Path -Leaf $dst86))

$clsidKey = 'HKLM:\SOFTWARE\Classes\CLSID\{7C3E0C11-5F6A-4E52-9B2C-A1D47B2E9F31}\InprocServer32'
Set-ItemProperty -Path $clsidKey -Name '(default)' -Value $dst64
Write-Host "  InprocServer32 -> $dst64"

$clsidKey32 = 'HKLM:\SOFTWARE\Classes\WOW6432Node\CLSID\{7C3E0C11-5F6A-4E52-9B2C-A1D47B2E9F31}\InprocServer32'
if (Test-Path $clsidKey32) {
    Set-ItemProperty -Path $clsidKey32 -Name '(default)' -Value $dst86
    Write-Host "  WOW6432Node\InprocServer32 -> $dst86"
}

Write-Host "部署完成（新开的程序会用新版本；explorer 等长驻进程仍用旧版本，重启后一致）。" -ForegroundColor Green
Write-Host "提示：安装目录里会积累带哈希后缀的旧 DLL，正式发版前用 Install.ps1 重装即可清掉。" -ForegroundColor Yellow
