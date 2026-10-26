# 把编译产物组装成可发布包：dist\MC-TypeEnglish
#
# 产出：
#   mcte.exe               钩子版（免管理员，备用）
#   mcte_tsf.dll           64 位 TSF 文本服务
#   mcte_tsf_x86.dll       32 位 TSF 文本服务（给 32 位应用用）
#   data\dict.txt          词库
#   data\glossary.tsv      本地多语词表
#   Install.ps1 / Uninstall.ps1 / install.bat / README.md
#
# 用法： powershell -ExecutionPolicy Bypass -File tools\stage_dist.ps1

param(
    [string]$Dist = "dist\MC-TypeEnglish"
)

# cargo 往 stderr 写进度，PowerShell 会当成 error record；用 Continue 并自己查退出码
$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

Write-Host "== 1/3 编译（release）=="
cargo build --release -p mcte -p mcte-tsf | Out-Host
if ($LASTEXITCODE -ne 0) { throw "cargo build (x64) 失败" }
cargo build --release -p mcte-tsf --target i686-pc-windows-msvc --lib | Out-Host
if ($LASTEXITCODE -ne 0) { throw "cargo build (x86) 失败" }
$ErrorActionPreference = 'Stop'

Write-Host "== 1.5/3 脚本语法与编码检查 =="
foreach ($script in @('Install.ps1', 'Uninstall.ps1')) {
    $path = Join-Path $Dist $script
    if (-not (Test-Path $path)) { continue }
    $errors = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path $path).Path, [ref]$null, [ref]$errors)
    if ($errors.Count -gt 0) {
        throw "$script 语法错误: $($errors[0].Message) @line $($errors[0].Extent.StartLineNumber)"
    }
    Write-Host "  $script 语法 OK"
}

$targets = @(
    @{ From = 'target\release\mcte.exe';                              To = 'mcte.exe' },
    @{ From = 'target\release\mcte_tsf.dll';                          To = 'mcte_tsf.dll' },
    @{ From = 'target\i686-pc-windows-msvc\release\mcte_tsf.dll';     To = 'mcte_tsf_x86.dll' },
    @{ From = 'target\release\mcte-tsf-selftest.exe';                 To = 'tools\mcte-tsf-selftest.exe' },
    @{ From = 'data\dict.txt';                                        To = 'data\dict.txt' },
    @{ From = 'data\glossary.tsv';                                    To = 'data\glossary.tsv' },
    @{ From = 'data\dict_en.tsv';                                     To = 'data\dict_en.tsv' },
    @{ From = 'data\en2zh.tsv';                                       To = 'data\en2zh.tsv' },
    @{ From = 'data\mcte.ico';                                        To = 'mcte.ico' },
    @{ From = 'packaging\Install.ps1';                          To = 'Install.ps1' },
    @{ From = 'packaging\Uninstall.ps1';                        To = 'Uninstall.ps1' },
    @{ From = 'packaging\install.bat';                          To = 'install.bat' },
    @{ From = 'README.md';                                            To = 'README.md' },
    @{ From = 'LICENSE';                                              To = 'LICENSE' }
)

Write-Host "== 2/3 复制产物 =="
foreach ($item in $targets) {
    if (-not (Test-Path $item.From)) { throw "缺少构建产物: $($item.From)" }
    $dest = Join-Path $Dist $item.To
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $dest) | Out-Null
    Copy-Item $item.From $dest -Force
    Write-Host ("  {0,-26} <- {1}" -f $item.To, $item.From)
}

# PowerShell 5.1 需要 UTF-8 BOM，否则中文会读成乱码
foreach ($script in @('Install.ps1', 'Uninstall.ps1')) {
    $path = Join-Path $Dist $script
    $text = [IO.File]::ReadAllText($path, [Text.Encoding]::UTF8)
    [IO.File]::WriteAllText($path, $text, (New-Object Text.UTF8Encoding($true)))
}

Write-Host "== 3/3 清单 =="
Get-ChildItem $Dist -Recurse -File |
    Sort-Object FullName |
    ForEach-Object {
        "{0,-32} {1,9} bytes  sha256={2}" -f
            $_.FullName.Replace((Resolve-Path $Dist).Path + '\', ''),
            $_.Length,
            (Get-FileHash $_.FullName -Algorithm SHA256).Hash.Substring(0, 16)
    }
