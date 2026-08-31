#Requires -Version 5.1
<#
.SYNOPSIS
  璇玑（infotopograph）一键启动脚本 · Windows PowerShell 版

.DESCRIPTION
  · 自动检测 py/python 解释器
  · 内部调用 scripts/manage.py bootstrap（预检 → 清理残留 → 可选启动服务 → 管理面板）
  · 默认：仅拉起 Web 管理面板，项目服务 (api / frontend) 需在页面上按需 ▶ 启动
  · 提供 start / stop / restart / verify / dashboard 常用动作

.PARAMETER Action
  启动动作：Start（默认）、Stop、Restart、Dashboard、Verify、DryRun

.PARAMETER Strict
  严格模式：任何服务健康检查/端口检查失败即退出非零。

.PARAMETER OpenDashboard
  Bootstrap 或 Dashboard 动作时自动打开浏览器访问管理面板。

.PARAMETER NoBuildRust
  显式跳过 Rust release 构建（即使当前环境有 cargo 也不触发）。

.PARAMETER BuildRust
  显式执行 Rust release 构建（相当于 start.sh 的 --build-rust）。

.PARAMETER WithServices
  显式同步启动所有 auto_start=true 的项目服务（默认 False：仅开管理面板，在页面上按需启停）。
  等价于旧版本的默认启动行为（api + frontend 一起拉起）。

.EXAMPLE
  .\scripts\start.ps1                        # 默认：预检 → 仅启动管理面板（项目服务不自动启）
  .\scripts\start.ps1 Start -WithServices    # 与旧版一致：同步启动 api + frontend + 管理面板
  .\scripts\start.ps1 DryRun -Strict         # 仅预检，严格模式（不实际启动）
  .\scripts\start.ps1 Start -Strict -OpenDashboard
  .\scripts\start.ps1 Stop                   # 按拓扑停止
  .\scripts\start.ps1 Restart -Strict        # 严格重启（已配置服务）
#>

[CmdletBinding()]
param(
  [ValidateSet('Start','Stop','Restart','Dashboard','Verify','DryRun')]
  [string]$Action = 'Start',

  [switch]$Strict,
  [switch]$OpenDashboard,
  [switch]$NoBuildRust,
  [switch]$BuildRust,
  [switch]$WithServices
)

$ErrorActionPreference = 'Stop'

# ------- 路径与彩色输出工具 -------
$RepoRoot = Split-Path -Parent $PSScriptRoot   # repo 根 = scripts/..
Push-Location $RepoRoot
try {
  $esc = [char]27
  $C = @{
    R = "$esc[0;31m";  G = "$esc[0;32m";  Y = "$esc[1;33m"
    B = "$esc[0;34m";  C = "$esc[0;36m";  _ = "$esc[0m"
  }
  function Write-OK([string]$m)    { Write-Host (" {0}✔{1} {2}"   -f $C.G,$C._,$m) -ForegroundColor Green }
  function Write-Warn([string]$m)  { Write-Host (" {0}⚠{1} {2}"   -f $C.Y,$C._,$m) -ForegroundColor Yellow }
  function Write-Fail([string]$m)  { Write-Host (" {0}✗{1} {2}"   -f $C.R,$C._,$m) -ForegroundColor Red }
  function Write-Head([string]$m)  { Write-Host ("`n{0}==== {1} ===={2}" -f $C.C,$m,$C._) }

  $banner = @"

$($C.C)============================================================
  璇玑 Mox · 全维数字孪生中台  一键启动（Windows PowerShell）
  仓库根：$($C.B)$RepoRoot$($C._)
============================================================$($C._)
"@
  Write-Host $banner

  # ------- 解析 Python 解释器（优先 py launcher） -------
  function Get-PythonBin {
    $candidates = @(
      @{ Bin = 'py';    Args = @('-3') },
      @{ Bin = 'python'; Args = @() },
      @{ Bin = 'python3'; Args = @() }
    )
    foreach ($c in $candidates) {
      $cmd = Get-Command $c.Bin -ErrorAction SilentlyContinue
      if (-not $cmd) { continue }
      try {
        # 注意：PS5 向原生进程传参时会剥离内嵌双引号，故探针代码务必避免使用 `"..."` 字符串字面量。
        # 采用 print(sys.version_info[0]) — 无内嵌引号；预期 stdout 为 '3'
        $probeArgs = @($c.Args) + @('-c', 'import sys;print(sys.version_info[0])')
        $out = & $c.Bin @probeArgs 2>&1
        $versionChar = ("$out").Trim()
        if ($LASTEXITCODE -eq 0 -and $versionChar -eq '3') {
          return [pscustomobject]@{ Bin = $cmd.Source; ArgsPrefix = $c.Args }
        }
      } catch {}
    }
    return $null
  }

  $PY = Get-PythonBin
  if (-not $PY) {
    Write-Fail "未找到可用的 Python 解释器（py -3 / python / python3）。请先安装 Python ≥ 3.10，并勾选 Add to PATH。"
    exit 127
  }
  Write-Host "  解释器: $($C.B)$($PY.Bin) $($PY.ArgsPrefix -join ' ')$($C._)"

  function Invoke-Manage([string[]]$ManageArgs) {
    $all = @($PY.ArgsPrefix) + @("$RepoRoot\scripts\manage.py") + @($ManageArgs)
    Write-Host "  $($C.C)→$($C._) & $($PY.Bin) $($all -join ' ')"
    & $PY.Bin @all
    return $LASTEXITCODE
  }

  # ------- 辅助：dump logs -------
  function Show-LastLogs {
    param([int]$Lines = 30)
    $logDir = Join-Path $RepoRoot '.logs'
    if (-not (Test-Path $logDir)) { return }
    Write-Warn "失败后各服务最近 $Lines 行日志："
    Get-ChildItem $logDir -Filter *.log | Sort-Object Name | ForEach-Object {
      Write-Host "`n===== $($_.Name) =====" -ForegroundColor Cyan
      Get-Content $_.FullName -Tail $Lines
    }
  }

  # ------- 可选：公理验证 -------
  if ($Action -eq 'Verify') {
    Write-Head '🧮 verify：六大公理数学自洽性验证'
    $rc = Invoke-Manage @('verify')
    if ($rc -eq 0) { Write-OK '公理验证通过' } else { Write-Warn '存在警告（不影响服务启动）' }
    exit $rc
  }

  # ------- 可选：Rust release 构建 -------
  if ($BuildRust -and -not $NoBuildRust) {
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
      Write-Head '🦀 构建 Rust workspace（release）...'
      cargo build --release
      if ($LASTEXITCODE -ne 0) { Write-Fail "Rust 构建失败 exit=$LASTEXITCODE"; exit 11 }
      Write-OK 'Rust 构建完成'
    } else {
      Write-Warn '未检测到 cargo，跳过 Rust 构建'
    }
  }

  # ------- 动作分发 -------
  switch ($Action) {
    'Stop' {
      Write-Head '按拓扑停止全部服务（--force）'
      $rc = Invoke-Manage @('stop','all','--force')
      exit $rc
    }
    'Restart' {
      Write-Head '严格模式重启 auto_start 服务'
      $restartArgs = @('restart','all')
      if ($Strict) { $restartArgs += '--strict' }
      $rc = Invoke-Manage $restartArgs
      if ($rc -ne 0) { Write-Fail "重启失败 exit=$rc"; Show-LastLogs; exit $rc }
      Write-OK '重启完成'
      $null = Invoke-Manage @('list')
      exit 0
    }
    'Dashboard' {
      Write-Head '启动管理面板（单独前台挂起）'
      $dashArgs = @('dashboard')
      if (-not $OpenDashboard) { $dashArgs += '--no-browser' }
      $rc = Invoke-Manage $dashArgs
      exit $rc
    }
    'DryRun' {
      Write-Head 'DRY-RUN：仅预检（不启动任何进程）'
      $argsA = @('bootstrap','--dry-run')
      if ($Strict) { $argsA += '--strict' }
      $rc = Invoke-Manage $argsA
      exit $rc
    }
    default {  # Start
      Write-Head '一键启动 bootstrap（默认：仅面板 → 页面上按需启动服务）'
      $boot = @('bootstrap')
      if ($Strict) { $boot += '--strict' }
      if ($WithServices) { $boot += '--with-services' }
      if ($OpenDashboard) {
        $boot += '--with-dashboard'
      } else {
        $boot += '--with-dashboard'
        $boot += '--no-browser'
      }
      $rc = Invoke-Manage $boot
      if ($rc -ne 0) {
        Write-Fail "bootstrap 失败 exit=$rc"
        $null = Invoke-Manage @('status')
        Show-LastLogs
        exit $rc
      }
      # 访问小贴士（端口一律以 platform_config.json 为单一事实源，禁止硬编码漂移）
      try {
        $cfg = Get-Content (Join-Path $RepoRoot 'platform_config.json') -Raw -Encoding UTF8 | ConvertFrom-Json
        $dashPort = [int]$cfg.dashboard_port
        $apiPort = [int]$cfg.services.api.port
        $fePort  = [int]$cfg.services.frontend.port
      } catch { $dashPort = 3999; $apiPort = 8080; $fePort = 3020 }
      Write-Host @"

$($C.G)============================================================$($C._)
  启动完成（管理面板挂起 / 后台运行）。常用操作：
   · Dashboard: $($C.B)http://localhost:$dashPort/$($C._)   → 登录后点 ▶ 启动所有 （api:$apiPort / frontend:$fePort）
   · API      : $($C.B)http://localhost:$apiPort/health$($C._)   （需在管理面板启动 api 服务后可用）
   · Frontend : $($C.B)http://localhost:$fePort/$($C._)         （需在管理面板启动 frontend 服务后可用）
   · 停止所有 : $($C.C).\scripts\start.ps1 Stop$($C._)
   · 运维 CLI : $($C.C)& $($PY.Bin) scripts\manage.py list|status|logs|stop$($C._)
   · 旧行为启动（脚本同步启动所有服务）: $($C.C).\scripts\start.ps1 Start -WithServices$($C._)
$($C.G)============================================================$($C._)
"@
      if ($OpenDashboard -eq $false) {
        Write-Host "提示：已在后台启动管理面板，请手动访问 http://localhost:$dashPort/ ；需要前台挂起/自动开浏览器，请加 -OpenDashboard 开关（或 Dashboard 动作）。"
      }
      exit 0
    }
  }
} finally {
  Pop-Location
}
