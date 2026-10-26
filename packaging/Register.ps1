# MSI 的自定义动作调用它：只做「注册/反注册 TSF 组件」这一件事。
#
# 为什么单独写一份而不是复用 Install.ps1：
#   MSI 已经负责铺文件、写 HKCU 的输入法条目和开机启动项，这里再拷一遍反而会
#   和 MSI 的组件跟踪打架。所以只保留 regsvr32 这一步（MSI 无法原生做 COM 注册）。
#
# 用法：
#   powershell -File Register.ps1 -InstallDir "C:\Program Files\MC-TypeEnglish"
#   powershell -File Register.ps1 -InstallDir "..." -Unregister

param(
    [Parameter(Mandatory = $true)][string]$InstallDir,
    [switch]$Unregister
)

$InstallDir = $InstallDir.Trim('"', "'", '\', ' ')

$ErrorActionPreference = 'Continue'
$log = Join-Path $env:TEMP 'mcte-msi-register.log'

function Write-Log([string]$text) {
    $line = "[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $text
    Add-Content -Path $log -Value $line -Encoding UTF8
    Write-Host $line
}

$dllX64 = Join-Path $InstallDir 'mcte_tsf.dll'
$dllX86 = Join-Path $InstallDir 'mcte_tsf_x86.dll'
$regsvr32X64 = Join-Path $env:WINDIR 'System32\regsvr32.exe'
$regsvr32X86 = Join-Path $env:WINDIR 'SysWOW64\regsvr32.exe'

Write-Log ("{0} 开始：InstallDir={1}" -f $(if ($Unregister) { '反注册' } else { '注册' }), $InstallDir)

# 无论安装还是卸载，均先关闭运行中的 mcte.exe 避免锁定
Get-Process mcte -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

# 如果是注册新版，先检查当前注册表是否残留了不同路径的旧 DLL（比如旧的 bin-xxx 目录）并主动反注册
if (-not $Unregister) {
    try {
        $oldReg = (Get-ItemProperty 'HKLM:\SOFTWARE\Classes\CLSID\{7C3E0C11-5F6A-4E52-9B2C-A1D47B2E9F31}\InProcServer32' -ErrorAction SilentlyContinue).'(default)'
        if ($oldReg -and ($oldReg -ne $dllX64) -and (Test-Path $oldReg)) {
            Write-Log "  发现旧版本 DLL 注册: $oldReg，先执行反注册..."
            Start-Process $regsvr32X64 -ArgumentList @('/s', '/u', "`"$oldReg`"") -Wait
        }
    } catch {}
}

# 停掉 ctfmon 释放各程序里被加载的模块句柄
Get-Process ctfmon -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 300

foreach ($pair in @(@{ Dll = $dllX64; Reg = $regsvr32X64; Name = 'x64' },
                    @{ Dll = $dllX86; Reg = $regsvr32X86; Name = 'x86' })) {
    if (-not (Test-Path $pair.Dll)) {
        Write-Log "  跳过 $($pair.Name)：找不到 $($pair.Dll)"
        continue
    }
    $regArgs = if ($Unregister) { @('/s', '/u', "`"$($pair.Dll)`"") } else { @('/s', "`"$($pair.Dll)`"") }
    $proc = Start-Process $pair.Reg -ArgumentList $regArgs -Wait -PassThru
    Write-Log ("  regsvr32 {0} ({1}) 退出码 {2}" -f $pair.Name, $(if ($Unregister) { '/u' } else { '注册' }), $proc.ExitCode)
    Start-Sleep -Milliseconds 200
}

# ctfmon 会被系统自动拉起，手动再拉一次确保托盘语言栏立即刷新
if (-not $Unregister) {
    Start-Process (Join-Path $env:WINDIR 'System32\ctfmon.exe') -ErrorAction SilentlyContinue
    Write-Log '  已重启 ctfmon'
}
Write-Log '完成'
