#Requires -Version 5
<#
.SYNOPSIS
  T12 一键算法对账脚本（Rust 8/8 + Node 56/56 + 公式 35/35 三重全绿）
.DESCRIPTION
  工作目录: $PSScriptRoot\.. (即 backend-node 根)
  三步串行, 每步 exit 0 才继续; 任意一步失败则整体非 0.
.EXAMPLE
  powershell -ExecutionPolicy Bypass -File .\scripts\run_t12_integration_test.ps1
#>

$ErrorActionPreference = 'Stop'

# ---- Step 0: 切到 backend-node 根 ----
$repoRoot = Join-Path $PSScriptRoot '..'
Set-Location $repoRoot
Write-Host ('=' * 86) -ForegroundColor Cyan
Write-Host "[T12 一键算法对账] 工作目录: $(Get-Location)" -ForegroundColor Cyan
Write-Host ('=' * 86) -ForegroundColor Cyan

$overall = $true

# ==================== Step a) Rust 侧: cargo test -p graph-algorithms --flow_graph::tests ====================
Write-Host ''
Write-Host ('-' * 86) -ForegroundColor DarkYellow
Write-Host '[Step a/3] 运行 Rust 单测: cargo test -p graph-algorithms --flow_graph::tests' -ForegroundColor Yellow
Write-Host ('-' * 86) -ForegroundColor DarkYellow

$rustServices = Join-Path $repoRoot '..\services'
if (Test-Path (Join-Path $rustServices 'Cargo.toml')) {
  Push-Location $rustServices
  cargo test -p graph-algorithms -- flow_graph::tests
  $rustExit = $LASTEXITCODE
  Pop-Location
} else {
  Push-Location $repoRoot
  cargo test -p graph-algorithms -- flow_graph::tests
  $rustExit = $LASTEXITCODE
  Pop-Location
}

if ($rustExit -eq 0) {
  Write-Host '[Step a/3] PASS — Rust 侧 flow_graph 8/8 单测全绿' -ForegroundColor Green
} else {
  Write-Host "[Step a/3] FAIL — Rust 侧 exit code = $rustExit" -ForegroundColor Red
  $overall = $false
}

# ==================== Step b) Node 侧 T12 对账 56/56 ====================
if ($overall) {
  Write-Host ''
  Write-Host ('-' * 86) -ForegroundColor DarkYellow
  Write-Host '[Step b/3] 运行 Node T12 对账: node test/test-t12-algorithm-reconcile.js' -ForegroundColor Yellow
  Write-Host ('-' * 86) -ForegroundColor DarkYellow

  node test/test-t12-algorithm-reconcile.js
  $t12Exit = $LASTEXITCODE

  if ($t12Exit -eq 0) {
    Write-Host '[Step b/3] PASS — Node 侧 T12 对账 56/56 全绿' -ForegroundColor Green
  } else {
    Write-Host "[Step b/3] FAIL — Node T12 exit code = $t12Exit" -ForegroundColor Red
    $overall = $false
  }
} else {
  Write-Host '[Step b/3] SKIP — 因前序步骤失败' -ForegroundColor Gray
}

# ==================== Step c) Node 侧图公式佐证（全绿） ====================
if ($overall) {
  Write-Host ''
  Write-Host ('-' * 86) -ForegroundColor DarkYellow
  Write-Host '[Step c/3] 运行图公式佐证: node test/test-graph-formulas.js' -ForegroundColor Yellow
  Write-Host ('-' * 86) -ForegroundColor DarkYellow

  node test/test-graph-formulas.js
  $formulaExit = $LASTEXITCODE

  if ($formulaExit -eq 0) {
    Write-Host '[Step c/3] PASS — Node 侧图公式 35/35 全绿（对账佐证）' -ForegroundColor Green
  } else {
    Write-Host "[Step c/3] FAIL — 图公式 exit code = $formulaExit" -ForegroundColor Red
    $overall = $false
  }
} else {
  Write-Host '[Step c/3] SKIP — 因前序步骤失败' -ForegroundColor Gray
}

# ==================== 总结 ====================
Write-Host ''
Write-Host ('=' * 86) -ForegroundColor Cyan
if ($overall) {
  Write-Host 'T12 一键算法对账：Rust 8/8 + Node 56/56 + 公式 35/35 = 三重全绿' -ForegroundColor Green
  Write-Host ('=' * 86) -ForegroundColor Cyan
  exit 0
} else {
  Write-Host 'T12 一键算法对账：存在失败步骤，请查看上方日志逐条修复' -ForegroundColor Red
  Write-Host ('=' * 86) -ForegroundColor Cyan
  exit 1
}
