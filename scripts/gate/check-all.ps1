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
   5) 前端 build       前端构建（可选 --SkipFrontend；pnpm 优先）
   6) 前端模块化门禁    FE-MOD-GOV-V1.0（element-plus 根导入/相对导入/barrel 冲突/命名/vite 配置/深路径）+ 时间/locale 口径闸门（无参 toLocaleString 棘轮/出口写法唯一性/locale pin/扫描集分母）
   7) 端口漂移校验      PORT-REGISTRY 权威源一致性
   8) 其余 CI 门禁      与 ci.yml 同账：两台公式仪器的 --selftest ＋ 前端四闸门的 --check
  段落分母 $Total 只定义一次，段标签一律写 [i/$Total]——同一个数写 N 次就是 N 个第二源。
  任意一项失败即退出非零，并输出汇总表。

.EXAMPLE
  .\scripts\gate\check-all.ps1
  .\scripts\gate\check-all.ps1 -SkipFrontend -SkipTest
#>
param(
    [switch]$SkipFrontend,
    [switch]$SkipTest,
    [switch]$SkipClippy
)

$ErrorActionPreference = "Continue"
# 本仓门禁的判决行含 ⇒／＝ 等字符：GBK 控制台下 print 会抛 UnicodeEncodeError，
# 那时 rc=1 会被读成"有红"，而真凶是仪器自己的输出编码（崩的是仪器不是被测面）。
$env:PYTHONIOENCODING = "utf-8"
$Root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $Root

$results = @()
$Total = 8
function Add-Result([string]$Name, [int]$Code, [string]$Note = "") {
    $script:results += [PSCustomObject]@{ Name = $Name; Code = $Code; Note = $Note }
    $icon = if ($Code -eq 0) { "PASS" } else { "FAIL" }
    $color = if ($Code -eq 0) { "Green" } else { "Red" }
    Write-Host ("  [{0}] {1} {2}" -f $icon, $Name, $Note) -ForegroundColor $color
}

Write-Host "=== 璇玑 一键质量检查 ===" -ForegroundColor Cyan

# 1) secret-scan
Write-Host "`n[1/$Total] secret-scan（敏感信息门禁）"
& python "scripts/ci/secret-scan.py" --path $Root
Add-Result "secret-scan" $LASTEXITCODE

# 2) cargo fmt
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Write-Host "`n[2/$Total] cargo fmt --check"
    & cargo fmt --all -- --check
    Add-Result "cargo fmt" $LASTEXITCODE
} else {
    Write-Host "`n[2/$Total] 无 cargo，跳过 fmt"
    Add-Result "cargo fmt" 0 "skipped"
}

# 3) cargo clippy
if (-not $SkipClippy -and (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "`n[3/$Total] cargo clippy（workspace，-D warnings）"
    & cargo clippy --workspace --all-targets -- -D warnings
    Add-Result "cargo clippy" $LASTEXITCODE
} else {
    Write-Host "`n[3/$Total] 跳过 clippy"
    Add-Result "cargo clippy" 0 "skipped"
}

# 4) cargo test
if (-not $SkipTest -and (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "`n[4/$Total] cargo test（workspace --lib --tests）"
    & cargo test --workspace --lib --tests -q
    Add-Result "cargo test" $LASTEXITCODE
} else {
    Write-Host "`n[4/$Total] 跳过 test"
    Add-Result "cargo test" 0 "skipped"
}

# 5) 前端构建
if (-not $SkipFrontend) {
    if (Test-Path "frontend-ui/package.json") {
        Write-Host "`n[5/$Total] 前端 build（pnpm 优先）"
        Push-Location "frontend-ui"
        try {
            if (-not (Test-Path "node_modules")) {
                Write-Host "      node_modules 缺失，先 pnpm install"
                & pnpm install --no-audit --no-fund
            }
            if (Get-Command pnpm -ErrorAction SilentlyContinue) {
                & pnpm build
            } else {
                & npm run build
            }
            Add-Result "frontend build" $LASTEXITCODE
        } finally { Pop-Location }
    } else {
        Add-Result "frontend build" 0 "no package.json"
    }
} else {
    Write-Host "`n[5/$Total] 跳过前端构建"
    Add-Result "frontend build" 0 "skipped"
}

# 6) 前端模块化门禁（FE-MOD-GOV-V1.0，CI 同源）+ 时间/locale 口径闸门 + 脚本路径解析门禁（F33）
Write-Host "`n[6/$Total] 前端模块化门禁"
& python "scripts/gate/check-frontend-module.py"
Add-Result "frontend-module gates" $LASTEXITCODE
& python "scripts/gate/check-locale-format-outlets.py" --selftest
Add-Result "locale-format outlets gate selftest" $LASTEXITCODE
& python "scripts/gate/check-locale-format-outlets.py"
Add-Result "locale-format outlets gate" $LASTEXITCODE
& python "scripts/gate/check-script-paths.py" --selftest
Add-Result "script-paths gate selftest" $LASTEXITCODE
& python "scripts/gate/check-script-paths.py"
Add-Result "script-paths gate" $LASTEXITCODE

# 7) 端口漂移校验（PORT-REGISTRY 权威源）
Write-Host "`n[7/$Total] 端口漂移校验"
& python "scripts/gate/verify-ports.py"
Add-Result "port drift gate" $LASTEXITCODE

# 8) 其余 CI 门禁（与 ci.yml 同账：本地一键不许是 CI 的子集却自称全量）
Write-Host "`n[8/$Total] 其余 CI 门禁（与 CI 同账）"
& python "scripts/gate/check-doc-formulas.py" --selftest
Add-Result "doc-formulas gate selftest" $LASTEXITCODE
& python "scripts/gate/check-api-surface.py" --selftest
Add-Result "api-surface gate selftest" $LASTEXITCODE
& python "frontend-ui/scripts/gate/check-api-binding-kinds.py" --check
Add-Result "api binding kinds gate" $LASTEXITCODE
& python "frontend-ui/scripts/gate/check-ep-feedback-imports.py" --check
Add-Result "EP feedback imports gate" $LASTEXITCODE
& python "frontend-ui/scripts/gate/check-framework-imports.py" --check
Add-Result "framework imports gate" $LASTEXITCODE
& python "frontend-ui/scripts/gate/check-theme-tokens.py" --check
Add-Result "theme tokens gate" $LASTEXITCODE

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
