# 一次性把乱七八糟的旧安装清干净，然后静默装 1.0.1。
# 需要管理员：由外层用 RunAs 调起。
$ErrorActionPreference = 'Continue'
$log = 'C:\MC-TypeEnglish\dist\msi-cleanup.log'
function W([string]$t) { Add-Content -Path $log -Value ("[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $t) -Encoding UTF8 }
Set-Content -Path $log -Value '开始清理' -Encoding UTF8

# 1) 停掉卡住的 msiexec 与输入法进程
Get-Process mcte -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Get-Process msiexec -ErrorAction SilentlyContinue | Where-Object { $_.Id -ne $PID } | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2
W '已停止 mcte / msiexec'

# 2) 卸载所有 MC-TypeEnglish 的 MSI 产品
$products = Get-ItemProperty 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue |
    Where-Object { $_.DisplayName -like 'MC-TypeEnglish*' -and $_.PSChildName -match '^\{' }
foreach ($p in $products) {
    W "卸载产品 $($p.PSChildName) ($($p.DisplayVersion))"
    $proc = Start-Process msiexec.exe -ArgumentList '/x', $p.PSChildName, '/qn' -Wait -PassThru
    W "  退出码 $($proc.ExitCode)"
}

# 3) 删掉脚本安装器留下的痕迹
$uninst = 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\MC-TypeEnglish'
if (Test-Path $uninst) { Remove-Item $uninst -Recurse -Force -ErrorAction SilentlyContinue; W '已删除脚本安装器的卸载项' }
Remove-Item 'C:\Program Files\MC-TypeEnglish' -Recurse -Force -ErrorAction SilentlyContinue
W '已删除旧安装目录'

# 4) 装 1.0.1
W '开始安装 1.0.1'
$proc = Start-Process msiexec.exe -ArgumentList '/i', 'C:\MC-TypeEnglish\dist\MC-TypeEnglish-1.0.1.msi', '/qn', '/l*v', 'C:\MC-TypeEnglish\dist\msi-final.log' -Wait -PassThru
W "安装退出码 $($proc.ExitCode)"

# 5) 结果
$exe = 'C:\Program Files\MC-TypeEnglish\mcte.exe'
if (Test-Path $exe) {
    W ("安装后 mcte.exe sha256 = " + (Get-FileHash $exe -Algorithm SHA256).Hash.Substring(0,16))
} else {
    W '安装后找不到 mcte.exe'
}
Get-ItemProperty 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue |
    Where-Object { $_.DisplayName -like 'MC-TypeEnglish*' } |
    ForEach-Object { W ("已注册产品: {0} {1}" -f $_.DisplayName, $_.DisplayVersion) }
Start-Sleep -Seconds 2
$m = Get-Process mcte -ErrorAction SilentlyContinue
if ($m) { W ("mcte 运行中 pid=" + $m.Id) } else { W 'mcte 未运行' }
W '完成'
