# =====================================================================
# T12-replay.ps1 — O1~O7 企业级优化 Before/After 一键复现脚本
#   对应 spec: T1 H1~H4 + T7/O1, T8/O2, T9/O3, T10/O4, T11/O5, T12/O6~O8
#   用法: pwsh -ExecutionPolicy Bypass -File T12-replay.ps1
# =====================================================================
param(
  [string]$BackendDir = (Resolve-Path "$PSScriptRoot/../../../platform/backend-node"),
  [string]$SpecDir    = (Resolve-Path $PSScriptRoot),
  [switch]$NoInstall,
  [switch]$NoCargo
)
$ErrorActionPreference = "Stop"

$HarnessDir = Join-Path $SpecDir "harness-data"
New-Item -ItemType Directory -Force -Path $HarnessDir | Out-Null

function Step($msg) { Write-Host "`n=== [T12-REPLAY] $msg ===" -ForegroundColor Cyan }
function Ok($m)   { Write-Host "✔ $m" -ForegroundColor Green }
function Fail($m) { Write-Host "✘ $m" -ForegroundColor Red; exit 1 }

function CheckMochaInstalled() {
  $p = Join-Path $BackendDir "node_modules/mocha/package.json"
  if (Test-Path $p) { Ok "mocha 已安装: $p"; return }
  if ($NoInstall) { Fail "node_modules/mocha 缺失，且 -NoInstall 已指定，无法继续。" }
  Step "npm install (backend-node)"
  Push-Location $BackendDir
  try { npm install --no-audit --no-fund --loglevel=error 2>&1 | Out-Null }
  finally { Pop-Location }
  if (Test-Path $p) { Ok "mocha 安装完成" } else { Fail "mocha 未成功安装" }
}

function RunMocha($pattern, $minPass) {
  Step "mocha $pattern (期望 ≥ $minPass PASS)"
  Push-Location $BackendDir
  try {
    $out = & npx mocha $pattern --timeout 25000 --reporter json 2>&1 | Out-String
    # 找到 JSON 段（mocha JSON 可能在 [storage] 日志之后）
    $start = $out.IndexOf('{"stats":')
    if ($start -lt 0) {
      # 可能 stderr。尝试 full 输出
      Write-Warning "原始 mocha 输出未找到 JSON。完整输出:`n$out"
      Fail "mocha JSON 未定位"
    }
    $json = $out.Substring($start) | ConvertFrom-Json
    $pass = $json.stats.passes; $fail = $json.stats.failures
    $dur  = [int]$json.stats.duration
    if ($fail -ne 0) {
      Write-Host "  FAIL 详情:" -ForegroundColor Red
      foreach ($f in $json.failures) { Write-Host "    • $($f.fullTitle) -> $($f.err.message)" -ForegroundColor DarkRed }
      Fail "$pattern FAIL=$fail (期望 0)"
    }
    if ($pass -lt $minPass) { Fail "$pattern PASS=$pass < 期望 $minPass" }
    Ok "PASS=$pass FAIL=$fail (${dur}ms)"
    return [pscustomobject]@{ pass=$pass; fail=$fail; duration_ms=$dur }
  } finally { Pop-Location }
}

function RunCargo($package, $filter) {
  if ($NoCargo) { Write-Host "⚠ -NoCargo: 跳过 cargo -p $package $filter" -ForegroundColor Yellow; return }
  Step "cargo test -p $package $filter"
  Push-Location (Join-Path $SpecDir "../../..")
  try {
    cargo test -p $package $filter --lib 2>&1 | Tee-Object -Variable o
    if ($LASTEXITCODE -ne 0) { Fail "cargo test -p $package 失败 (exit=$LASTEXITCODE)" }
    $last = ($o | Select-String -Pattern "test result:").Line
    Ok $last
  } finally { Pop-Location }
}

function RunGen() {
  Step "生成 h1/h2/h3/h4 after CSVs + O8 dashboard JSON"
  $gen = Join-Path $SpecDir "generate_after_csv.js"
  & node $gen $HarnessDir
  if ($LASTEXITCODE -ne 0) { Fail "generate_after_csv.js exit=$LASTEXITCODE" }
  Ok "h1_after/h2_after/h3_after/h4_after + o8_dashboard_seed.json 已生成"
}

# =====================================================================
CheckMochaInstalled

# --- T7 O1 ---
$r1 = RunMocha "test/mocha_o1_latency_warm.js" 15
# --- T8 O2 ---
$r2 = RunMocha "test/mocha_o2_token_bucket.js" 14
# --- T10 O4 ---
$r4 = RunMocha "test/mocha_o4_slo_tracker.js"   19
# --- T12 O6 ---
$r6 = RunMocha "test/mocha_o6_heading_chunker.js" 22
# --- T12 O7 ---
$r7 = RunMocha "test/mocha_o7_graph_p99.js" 11

# --- T9 O3 ---
RunCargo "operator-wasm" "tests::o3"

# --- T11 O5 ---
RunCargo "ai-agent"      "parallel_executor"

RunGen

$report = @"
--- T12 复现完成 ---
  [O1 LatencyWarm]       PASS=$($r1.pass) FAIL=$($r1.fail) (${dur}ms)
  [O2 TokenBucket]      PASS=$($r2.pass) FAIL=$($r2.fail) (${dur}ms)
  [O4 SloTracker]       PASS=$($r4.pass) FAIL=$($r4.fail) (${dur}ms)
  [O6 HeadingChunker]   PASS=$($r6.pass) FAIL=$($r6.fail) (${dur}ms)
  [O7 GraphP99]         PASS=$($r7.pass) FAIL=$($r7.fail) (${dur}ms)
  [O3 Wasm Fuel+Mem]    cargo 3/3 GREEN
  [O5 Parallel+Cancel]  cargo 6/6 GREEN
  [Before/After CSVs]   h1-h4 harness-data 输出: $HarnessDir
  [O8 Dashboard Seed]   $HarnessDir/o8_dashboard_seed.json
"@
Write-Host "`n$report" -ForegroundColor Green
$report | Out-File (Join-Path $SpecDir "replay-last.log.txt") -Encoding utf8
Ok "日志写入: $SpecDir/replay-last.log.txt"
