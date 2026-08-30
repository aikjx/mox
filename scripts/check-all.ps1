#Requires -Version 5.1
<#
.SYNOPSIS
  璇玑 一键质量检查（B-2 / G-4 四件套之一）

.DESCRIPTION
  依次执行并汇总：
   1) secret-scan     全仓敏感信息扫描（P0 门禁）
   2) cargo fmt        格式检查（--check）
   3) cargo clippy     workspace 静态检查（-D warnings）
   4) cargo test       workspace 单元测试（--lib --tests）
   5) 前端 build       前端构建（可选 --SkipFrontend）
  任意一项失败即退出非零，并输出汇总表。

.EXAMPLE
  .\scripts\check-all.ps1
  .\scripts\check-all.ps1 -SkipFrontend -SkipTest
#>
param(
    [switch]$SkipFrontend,
    [switch]$SkipTest,
    [switch]$SkipClippy
)

$ErrorActionPreference = "Continue"
$Root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $Root

$results = @()
function Add-Result([string]$Name, [int]$Code, [string]$Note = "") {
    $script:results += [PSCustomObject]@{ Name = $Name; Code = $Code; Note = $Note }
    $icon = if ($Code -eq 0) { "PASS" } else { "FAIL" }
    $color = if ($Code -eq 0) { "Green" } else { "Red" }
    Write-Host ("  [{0}] {1} {2}" -f $icon, $Name, $Note) -ForegroundColor $color
}

Write-Host "=== 璇玑 一键质量检查 ===" -ForegroundColor Cyan

# 1) secret-scan
Write-Host "`n[1/5] secret-scan（敏感信息门禁）"
& python "scripts/ci/secret-scan.py" --path $Root
Add-Result "secret-scan" $LASTEXITCODE

# 2) cargo fmt
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Write-Host "`n[2/5] cargo fmt --check"
    & cargo fmt --all -- --check
    Add-Result "cargo fmt" $LASTEXITCODE
} else {
    Write-Host "`n[2/5] 无 cargo，跳过 fmt"
    Add-Result "cargo fmt" 0 "skipped"
}

# 3) cargo clippy
if (-not $SkipClippy -and (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "`n[3/5] cargo clippy（workspace，-D warnings）"
    & cargo clippy --workspace --all-targets -- -D warnings
    Add-Result "cargo clippy" $LASTEXITCODE
} else {
    Write-Host "`n[3/5] 跳过 clippy"
    Add-Result "cargo clippy" 0 "skipped"
}

# 4) cargo test
if (-not $SkipTest -and (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "`n[4/5] cargo test（workspace --lib --tests）"
    & cargo test --workspace --lib --tests -q
    Add-Result "cargo test" $LASTEXITCODE
} else {
    Write-Host "`n[4/5] 跳过 test"
    Add-Result "cargo test" 0 "skipped"
}

# 5) 前端构建
if (-not $SkipFrontend) {
    if (Test-Path "frontend-ui/package.json") {
        Write-Host "`n[5/5] 前端 build"
        Push-Location "frontend-ui"
        try {
            if (-not (Test-Path "node_modules")) {
                Write-Host "      node_modules 缺失，先 npm install"
                & npm install --no-audit --no-fund
            }
            & npm run build
            Add-Result "frontend build" $LASTEXITCODE
        } finally { Pop-Location }
    } else {
        Add-Result "frontend build" 0 "no package.json"
    }
} else {
    Write-Host "`n[5/5] 跳过前端构建"
    Add-Result "frontend build" 0 "skipped"
}

Write-Host "`n=== 汇总 ===" -ForegroundColor Cyan
$fails = @($results | Where-Object { $_.Code -ne 0 })
foreach ($r in $results) {
    $icon = if ($r.Code -eq 0) { "PASS" } else { "FAIL" }
    Write-Host ("  [{0}] {1}" -f $icon, $r.Name)
}
if ($fails.Count -eq 0) {
    Write-Host "`n[PASS] 全部检查通过" -ForegroundColor Green
    exit 0
} else {
    Write-Host "`n[FAIL] $($fails.Count) 项未通过: $($fails.Name -join ', ')" -ForegroundColor Red
    exit 1
}
