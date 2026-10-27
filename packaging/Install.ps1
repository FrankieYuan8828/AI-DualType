# MC-TypeEnglish 安装器（TSF 文本服务版）
#
# 装什么：
#   C:\Program Files\MC-TypeEnglish\
#     mcte_tsf.dll      64 位 TSF 文本服务  → 注册后出现在 Win+Space 语言列表
#     mcte_tsf_x86.dll  32 位 TSF 文本服务  → 让 32 位应用（微信 / 老 Office）也能用
#     mcte.exe          钩子版（免管理员备用方案，默认不启用）
#     data\             词库与本地词表
#     Install.ps1 / Uninstall.ps1
#
# 为什么需要管理员：TSF 输入法必须把 COM 组件和语言配置写进 HKLM，
# 这是 Windows 的硬性要求（微软拼音、Weasel 都一样）。
#
# 用法（双击 install.bat 等价）：
#   powershell -ExecutionPolicy Bypass -File Install.ps1              # 装 TSF 版（推荐）
#   powershell -ExecutionPolicy Bypass -File Install.ps1 -Hook        # 只装免管理员的钩子版
#   powershell -ExecutionPolicy Bypass -File Install.ps1 -DryRun      # 只打印将要做什么

param(
    [switch]$Hook,
    [switch]$NoStart,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'

$AppName      = 'MC-DualType'
$SourceDir    = Split-Path -Parent $MyInvocation.MyCommand.Definition
$InstallDir   = Join-Path $env:ProgramFiles $AppName
# 二进制放进「按内容哈希命名」的子目录：升级时新版本永远写新文件，
# 不会被 explorer / 终端等长驻进程占用的旧 DLL 挡住（这是 TSF 输入法升级的经典坑）。
$Tag          = if (Test-Path (Join-Path $SourceDir 'mcte_tsf.dll')) {
                    (Get-FileHash (Join-Path $SourceDir 'mcte_tsf.dll') -Algorithm SHA256).Hash.Substring(0,8).ToLower()
                } else { 'dev' }
$BinDir       = Join-Path $InstallDir "bin-$Tag"
$DllX64       = Join-Path $BinDir 'mcte_tsf.dll'
$DllX86       = Join-Path $BinDir 'mcte_tsf_x86.dll'
$ExePath      = Join-Path $BinDir 'mcte.exe'
$Regsvr32X64  = Join-Path $env:WINDIR 'System32\regsvr32.exe'
$Regsvr32X86  = Join-Path $env:WINDIR 'SysWOW64\regsvr32.exe'
$StartMenu    = Join-Path $env:ProgramData 'Microsoft\Windows\Start Menu\Programs'
$UninstKey    = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$AppName"
$RunKey       = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$ConfigDir    = Join-Path $env:APPDATA $AppName

function Write-Step([string]$text) { Write-Host "== $text" -ForegroundColor Cyan }
function Write-Action([string]$text) {
    if ($DryRun) { Write-Host "  [dry-run] $text" } else { Write-Host "  $text" }
}

Write-Host "$AppName 安装器" -ForegroundColor Green
if ($DryRun) { Write-Host "（dry-run：只打印动作，不改系统）" -ForegroundColor Yellow }

# ---- 权限检查（TSF 版必须提权）----
$identity  = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
$isAdmin   = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $Hook -and -not $isAdmin -and -not $DryRun) {
    Write-Host "需要管理员权限注册 TSF 输入法，正在请求提权…" -ForegroundColor Yellow
    $argList = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"")
    if ($NoStart) { $argList += '-NoStart' }
    Start-Process powershell -Verb RunAs -ArgumentList $argList
    exit 0
}

# ---- 0. 检测旧版本并自动卸载清理 ----
Write-Step "检测旧版本"
# 1) 检测 MSI 安装残留并静默卸载旧版本
$installedMsi = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*', 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue |
    Where-Object { $_.DisplayName -eq 'MC-TypeEnglish' -and $_.UninstallString -match 'msiexec' }
if ($installedMsi) {
    Write-Host "  检测到已安装的旧 MSI 版本，正在自动先执行静默卸载..." -ForegroundColor Yellow
    foreach ($app in $installedMsi) {
        $uninstCode = if ($app.PSChildName -match '^\{.*\}$') { $app.PSChildName } else { $app.UninstallString -replace '.*\{([0-9a-fA-F-]+)\}.*', '{$1}' }
        if ($uninstCode -match '^\{.*\}$') {
            Write-Host "  卸载旧 MSI: $uninstCode"
            $p = Start-Process msiexec.exe -ArgumentList '/x', $uninstCode, '/qn' -Wait -PassThru
            Write-Host "  旧 MSI 卸载完成，退出码: $($p.ExitCode)"
        }
    }
}

# 2) 检测已有旧 TSF DLL 并主动反注册
try {
    $oldClsid = (Get-ItemProperty 'HKLM:\SOFTWARE\Classes\CLSID\{7C3E0C11-5F6A-4E52-9B2C-A1D47B2E9F31}\InProcServer32' -ErrorAction SilentlyContinue).'(default)'
    if ($oldClsid -and (Test-Path $oldClsid)) {
        Write-Host "  反注册旧版本 TSF DLL: $oldClsid" -ForegroundColor Yellow
        Start-Process (Join-Path $env:WINDIR 'System32\regsvr32.exe') -ArgumentList @('/s', '/u', "`"$oldClsid`"") -Wait
    }
} catch {}

# ---- 1. 停掉旧进程 ----
Write-Step "停止旧实例并刷新系统输入法"
Get-Process mcte -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Get-Process ctfmon -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500

# ---- 2. 复制文件 ----
Write-Step "复制到 $BinDir"
if (-not $DryRun) {
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $InstallDir 'data') | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $BinDir 'tools') | Out-Null
    # 目标已存在且内容相同就跳过：bin-<hash> 里的 DLL 常被 explorer 占着，
    # 内容没变时不该因为这步失败（这是重装失败的真实原因）。
    foreach ($name in 'mcte.exe', 'mcte_tsf.dll', 'mcte_tsf_x86.dll', 'README.md', 'LICENSE') {
        $from = Join-Path $SourceDir $name
        if (-not (Test-Path $from)) { continue }
        $to = Join-Path $BinDir $name
        if ((Test-Path $to) -and
            ((Get-FileHash $from -Algorithm SHA256).Hash -eq (Get-FileHash $to -Algorithm SHA256).Hash)) {
            continue
        }
        try {
            Copy-Item $from $to -Force -ErrorAction Stop
        } catch {
            Write-Host "  跳过 $name（目标被占用，且内容一致）: $($_.Exception.Message)" -ForegroundColor Yellow
        }
    }
    # 命令行工具（--probe / --selftest / --learn）和钩子版都从根目录这份跑，
    # 免得用户在一堆 bin-<hash> 目录里找 mcte.exe。
    foreach ($name in 'mcte.exe', 'README.md', 'LICENSE') {
        $from = Join-Path $SourceDir $name
        $to = Join-Path $InstallDir $name
        if (-not (Test-Path $from)) { continue }
        if ((Test-Path $to) -and
            ((Get-FileHash $from -Algorithm SHA256).Hash -eq (Get-FileHash $to -Algorithm SHA256).Hash)) {
            continue
        }
        try { Copy-Item $from $to -Force -ErrorAction Stop }
        catch { Write-Host "  跳过 $name（被占用）" -ForegroundColor Yellow }
    }
    # 词库 + 中英对照 + 英中反查，全部平铺到 <InstallDir>\data\
    foreach ($file in (Get-ChildItem (Join-Path $SourceDir 'data') -File)) {
        Copy-Item $file.FullName (Join-Path $InstallDir ("data\" + $file.Name)) -Force
    }
    if (Test-Path (Join-Path $SourceDir 'mcte.ico')) {
        Copy-Item (Join-Path $SourceDir 'mcte.ico') (Join-Path $InstallDir 'mcte.ico') -Force
    }
    $selftest = Join-Path $SourceDir 'tools\mcte-tsf-selftest.exe'
    if (Test-Path $selftest) { Copy-Item $selftest (Join-Path $BinDir 'tools\mcte-tsf-selftest.exe') -Force }
    Copy-Item (Join-Path $SourceDir 'Uninstall.ps1') (Join-Path $InstallDir 'Uninstall.ps1') -Force
    Set-Content -Path (Join-Path $InstallDir 'current.txt') -Value $Tag -Encoding ascii
}
Write-Action "mcte_tsf.dll / mcte_tsf_x86.dll / mcte.exe / data\* 已就位（bin-$Tag）"

# ---- 3. 默认配置（保留用户已有配置）----
Write-Step "用户配置"
if (-not $DryRun) {
    New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null
    $configPath = Join-Path $ConfigDir 'config.json'
    if (-not (Test-Path $configPath)) {
        @'
{
  "targets": ["de", "en", "ja"],
  "start_in_ascii_mode": false,
  "toggle_hotkey": "Ctrl+Alt+P",
  "max_parents": 9,
  "max_children": 3,
  "prefix_search": true,
  "learn_enabled": true,
  "ai_spell_fallback": true,
  "ai": {
    "enabled": false,
    "host": "api.deepseek.com",
    "path": "/v1/chat/completions",
    "model": "deepseek-chat",
    "api_key": "",
    "temperature": 0.2,
    "timeout_secs": 8,
    "max_parents": 2
  }
}
'@ | Set-Content -Path $configPath -Encoding utf8
        Write-Action "写入默认配置 $configPath"
    } else {
        Write-Action "保留已有配置 $configPath"
    }
}

# ---- 3.5 清理旧版本残留（只删旧 DLL 与旧版本目录，不动根目录的 mcte.exe）----
Write-Step "清理旧版本残留"
if (-not $DryRun) {
    Get-ChildItem $InstallDir -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match '^mcte_tsf.*\.dll$' } |
        Remove-Item -Force -ErrorAction SilentlyContinue
    Remove-Item (Join-Path $InstallDir 'tools') -Recurse -Force -ErrorAction SilentlyContinue
    Get-ChildItem $InstallDir -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -like 'bin-*' -and $_.Name -ne "bin-$Tag" } |
        ForEach-Object {
            Remove-Item $_.FullName -Recurse -Force -ErrorAction SilentlyContinue
            if (Test-Path $_.FullName) { Write-Host "  旧版本 $($_.Name) 仍被占用，下次重装/重启后可清掉" -ForegroundColor Yellow }
        }
}
Write-Action "已清理旧版本（当前 bin-$Tag）"
# ---- 4. 注册 ----
if ($Hook) {
    Write-Step "钩子版：写用户级自启动（无需管理员）"
    if (-not $DryRun) {
        New-Item -Path $RunKey -Force | Out-Null
        Set-ItemProperty -Path $RunKey -Name $AppName -Value "`"$ExePath`""
    }
    Write-Action "HKCU\...\Run\$AppName -> $ExePath"
} else {
    Write-Step "注册 64 位 TSF 文本服务"
    Write-Action "$Regsvr32X64 /s `"$DllX64`""
    if (-not $DryRun) {
        $p = Start-Process $Regsvr32X64 -ArgumentList '/s', "`"$DllX64`"" -Wait -PassThru
        if ($p.ExitCode -ne 0) { throw "regsvr32 (x64) 失败，退出码 $($p.ExitCode)。请确认安装目录与 DLL 完整。" }
        Start-Sleep -Milliseconds 300
    }

    Write-Step "注册 32 位 TSF 文本服务（供 32 位应用使用）"
    if (Test-Path $DllX86) {
        Write-Action "$Regsvr32X86 /s `"$DllX86`""
        if (-not $DryRun) {
            $p = Start-Process $Regsvr32X86 -ArgumentList '/s', "`"$DllX86`"" -Wait -PassThru
            if ($p.ExitCode -ne 0) { Write-Host "  32 位注册失败（32 位应用里可能用不了），继续。" -ForegroundColor Yellow }
            Start-Sleep -Milliseconds 300
        }
    } else {
        Write-Host "  未找到 mcte_tsf_x86.dll，跳过（32 位应用不可用）" -ForegroundColor Yellow
    }
}

# ---- 4.4 把手动启用到「当前用户的 Win+Space 列表」里 ----
# RegisterProfile 只写机器级 HKLM；Win+Space 轮的是 HKCU 的用户列表。
# DllRegisterServer 里已调 EnableLanguageProfile，这里再兜一层：
# 直接补 HKCU 的 tips 值（Windows 自己就是这么存的：<langid>:<clsid>{<profile>} = 1）。
Write-Step "把输入法加入当前用户的输入法列表"
$tips = "0804:{7C3E0C11-5F6A-4E52-9B2C-A1D47B2E9F31}{B3D2F0A4-6C81-4E17-9E45-2D8A9C6B4F72}"
$userLangKey = 'HKCU:\Control Panel\International\User Profile\zh-Hans-CN'
if (-not $DryRun) {
    if (Test-Path $userLangKey) {
        # Windows 的存法：值名就是 tips 字符串，值是 DWORD 1
        $existing = (Get-Item $userLangKey).Property |
                    Where-Object { $_ -like '*{7C3E0C11-5F6A-4E52-9B2C-A1D47B2E9F31}*' }
        if (-not $existing) {
            # 顺手清掉早期版本写反的 "1" / "2" 之类的垃圾值
            foreach ($bad in (Get-Item $userLangKey).Property) {
                if ($bad -match '^\d+$') { Remove-ItemProperty -Path $userLangKey -Name $bad -ErrorAction SilentlyContinue }
            }
            New-ItemProperty -Path $userLangKey -Name $tips -Value 1 -PropertyType DWord -Force | Out-Null
            Write-Host "  已加入输入法列表：$tips"
        } else {
            Write-Host "  输入法列表里已有：$existing"
        }
    } else {
        Write-Host "  未找到中文(简体)语言项，请先在「设置 → 时间和语言 → 语言」里添加中文" -ForegroundColor Yellow
    }
    # 重启 ctfmon 让语言栏/输入法列表立即刷新（不必重启电脑）
    Get-Process ctfmon -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 300
    Start-Process (Join-Path $env:WINDIR 'System32\ctfmon.exe') -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 800
}
Write-Action "用户输入法列表已更新（重启 ctfmon）"


# ---- 5. 开始菜单 / 卸载项 ----
Write-Step "开始菜单与卸载项"
if (-not $DryRun) {
    New-Item -ItemType Directory -Force -Path $StartMenu | Out-Null
    $shell = New-Object -ComObject WScript.Shell
    $lnk = $shell.CreateShortcut((Join-Path $StartMenu "$AppName 配置文件夹.lnk"))
    $lnk.TargetPath = 'explorer.exe'
    $lnk.Arguments = "`"$ConfigDir`""
    $lnk.Description = 'MC-TypeEnglish 配置、用户词库与日志'
    $lnk.Save()

    $toolLnk = $shell.CreateShortcut((Join-Path $StartMenu "$AppName 工具（自检·诊断）.lnk"))
    $toolLnk.TargetPath = 'powershell.exe'
    $toolLnk.Arguments = "-NoExit -NoProfile -Command `"cd '$InstallDir'; Write-Host 'MC-TypeEnglish 工具：'; Get-ChildItem; Write-Host ''; Write-Host 'mcte.exe --probe nihao   # 看候选'; Write-Host 'mcte.exe --selftest      # 自学习自检'; Write-Host 'mcte.exe                 # 钩子版（免管理员备用）'`""
    $toolLnk.Description = 'MC-TypeEnglish 命令行工具与诊断'
    $toolLnk.Save()

    # 终端专用：只在终端窗口里接管按键（终端宿主不加载 TSF 组件）
    $termLnk = $shell.CreateShortcut((Join-Path $StartMenu "$AppName 终端模式.lnk"))
    $termLnk.TargetPath = $ExePath
    $termLnk.Arguments = '--terminal-only'
    $termLnk.WorkingDirectory = $InstallDir
    $termLnk.Description = '只在 Windows Terminal / conhost 里接管输入（其它窗口交给 TSF 版）'
    $termLnk.Save()

    New-Item -Path $UninstKey -Force | Out-Null
    $version = '1.0.0'
    $exeInfo = Join-Path $InstallDir 'mcte.exe'
    if (Test-Path $exeInfo) {
        $ver = (Get-Item $exeInfo).VersionInfo.FileVersion
        if ($ver) { $version = $ver }
    }
    Set-ItemProperty -Path $UninstKey -Name 'DisplayName'     -Value $AppName
    Set-ItemProperty -Path $UninstKey -Name 'DisplayVersion'  -Value $version
    Set-ItemProperty -Path $UninstKey -Name 'Publisher'       -Value 'MC-TypeEnglish'
    Set-ItemProperty -Path $UninstKey -Name 'InstallLocation' -Value $InstallDir
    Set-ItemProperty -Path $UninstKey -Name 'DisplayIcon'     -Value (Join-Path $InstallDir 'mcte_tsf.dll')
    Set-ItemProperty -Path $UninstKey -Name 'NoModify'        -Value 1 -Type DWord
    Set-ItemProperty -Path $UninstKey -Name 'NoRepair'        -Value 1 -Type DWord
    Set-ItemProperty -Path $UninstKey -Name 'UninstallString' -Value "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$(Join-Path $InstallDir 'Uninstall.ps1')`""
}
Write-Action "开始菜单快捷方式 + 卸载项已写入"

# ---- 6. 启动 ----
if (-not $NoStart -and -not $DryRun) {
    if ($Hook) {
        Write-Step "启动钩子版"
        Start-Process -FilePath $ExePath
    } else {
        Write-Step "启动输入法主机"
        # 让 ctfmon 立刻刷新输入法列表，省得用户重登
        Start-Process -FilePath (Join-Path $env:WINDIR 'System32\ctfmon.exe') -ErrorAction SilentlyContinue
        # 终端模式（钩子）也拉起来：它只处理终端窗口，和其它窗口不冲突
        Start-Process -FilePath $ExePath -ArgumentList '--terminal-only' -ErrorAction SilentlyContinue
    }
}

Write-Host ""
Write-Host "安装完成。" -ForegroundColor Green
if ($Hook) {
    Write-Host "  钩子版已装：托盘出现图标，任何输入框打拼音即可（提权窗口不支持）"
} else {
    Write-Host "  用法：按 Win+Space 切到「MC-TypeEnglish」，然后打拼音（如 nihao）"
    Write-Host "        候选窗显示 1 你好 / 1.1 hallo / 1.2 hello / 2 妳好，数字键直接选"
    Write-Host "        提权窗口（管理员权限程序）里同样可用——TSF 是进程内加载"
}
Write-Host "  程序目录 : $InstallDir"
Write-Host "  配置目录 : $ConfigDir"
Write-Host "  卸载     : 设置 → 应用 → 安装的应用 → $AppName"
Write-Host "  自检     : `"$InstallDir\tools\mcte-tsf-selftest.exe`" `"$DllX64`""
