# MC-TypeEnglish 卸载器（需管理员：要反注册 TSF 组件）
#
# 移除：TSF 注册（64/32 位）/ 进程 / 自启动项 / 开始菜单快捷方式 / 卸载项 / 程序目录
# 保留：%APPDATA%\MC-TypeEnglish（配置、用户词库、日志）—— 重装可继续用

param([switch]$DryRun)

$ErrorActionPreference = 'SilentlyContinue'
$AppName    = 'MC-TypeEnglish'
$InstallDir = Join-Path $env:ProgramFiles $AppName
$clsidPath  = 'HKLM:\SOFTWARE\Classes\CLSID\{7C3E0C11-5F6A-4E52-9B2C-A1D47B2E9F31}\InprocServer32'
$DllX64     = (Get-ItemProperty $clsidPath -ErrorAction SilentlyContinue).'(default)'
if (-not $DllX64) { $DllX64 = Join-Path $InstallDir 'mcte_tsf.dll' }
$DllX86     = Join-Path (Split-Path -Parent $DllX64) 'mcte_tsf_x86.dll'
$Regsvr32X64 = Join-Path $env:WINDIR 'System32\regsvr32.exe'
$Regsvr32X86 = Join-Path $env:WINDIR 'SysWOW64\regsvr32.exe'
$StartMenu  = Join-Path $env:ProgramData 'Microsoft\Windows\Start Menu\Programs'
$UninstKey  = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$AppName"
$RunKey     = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'

function Step([string]$t) { Write-Host "== $t" -ForegroundColor Cyan }

$identity  = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator) -and -not $DryRun) {
    Write-Host "卸载需要管理员权限，正在请求提权…" -ForegroundColor Yellow
    Start-Process powershell -Verb RunAs -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File',"`"$PSCommandPath`"")
    exit 0
}

Step "停止进程并刷新系统输入法"
if (-not $DryRun) {
    Get-Process mcte -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Get-Process ctfmon -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 400
}

Step "反注册 64 位 TSF"
if (-not $DryRun -and (Test-Path $DllX64)) { Start-Process $Regsvr32X64 -ArgumentList '/s','/u',"`"$DllX64`"" -Wait }

Step "反注册 32 位 TSF"
if (-not $DryRun -and (Test-Path $DllX86)) { Start-Process $Regsvr32X86 -ArgumentList '/s','/u',"`"$DllX86`"" -Wait }

Step "清理自启动 / 快捷方式 / 卸载项"
if (-not $DryRun) {
    Remove-ItemProperty -Path $RunKey -Name $AppName -ErrorAction SilentlyContinue
    Remove-Item (Join-Path $StartMenu "$AppName 配置文件夹.lnk") -Force -ErrorAction SilentlyContinue
    Remove-Item $UninstKey -Recurse -Force -ErrorAction SilentlyContinue
    # 清理所有残留的 MSI 注册
    $installedMsi = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*', 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue |
        Where-Object { $_.DisplayName -eq 'MC-TypeEnglish' -and $_.UninstallString -match 'msiexec' }
    foreach ($app in $installedMsi) {
        $uninstCode = if ($app.PSChildName -match '^\{.*\}$') { $app.PSChildName } else { $app.UninstallString -replace '.*\{([0-9a-fA-F-]+)\}.*', '{$1}' }
        if ($uninstCode -match '^\{.*\}$') {
            Start-Process msiexec.exe -ArgumentList '/x', $uninstCode, '/qn' -Wait
        }
    }
}

Step "删除程序目录"
if (-not $DryRun) { 
    Remove-Item $InstallDir -Recurse -Force -ErrorAction SilentlyContinue 
    Start-Process (Join-Path $env:WINDIR 'System32\ctfmon.exe') -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "已卸载。配置与用户词库保留在 $env:APPDATA\$AppName（重装可继续用；彻底清除请手动删该目录）。" -ForegroundColor Green
Write-Host "提示：已经打开的窗口如果还加载着输入法，建议重启一次资源管理器或重新登录。" -ForegroundColor Yellow
