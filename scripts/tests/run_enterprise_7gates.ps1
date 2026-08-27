# ===============================================================
# Task 8 + Task 10 · Enterprise 7-GATE Full Regression (C3 normalization)
# Safe-first: Stop-After-First-Fail for GATE 1-3/5-7; GATE-4 allows up to 3 non-critical fails
# ===============================================================

$ErrorActionPreference = "Stop"
$Root = "d:\a10\aikjx\gitcode\infotopograph"
$Backend = Join-Path $Root "platform\backend-node"
$CargoDir = $Root

Write-Host "================ Enterprise 7-GATE Regression (AIS C3) ================" -ForegroundColor Cyan
Write-Host "ROOT = $Root"
Write-Host ""

function Write-Gate($num, $name) { Write-Host ("`n[GATE-{0}] {1}" -f $num, $name) -ForegroundColor Yellow }
function Write-Fail($msg) { Write-Host ("FAIL: {0}" -f $msg) -ForegroundColor Red }
function Write-Pass($msg) { Write-Host (" OK : {0}" -f $msg) -ForegroundColor Green }
function Stop-IfFail($exit, $gate) {
    if ($exit -ne 0) {
        Write-Fail ("Gate-{0} FAILED exit={1}; SAFE-FIRST stop, no further gates." -f $gate, $exit)
        exit $exit
    }
}

$GateResults = New-Object System.Collections.Generic.List[object]

# ---------- GATE 1: Rust workspace build ----------
Write-Gate 1 "Rust build: cargo build --workspace"
Push-Location $CargoDir
cargo build --workspace 2>&1 | Tee-Object -Variable _out | Select-Object -Last 12
$exit = $LASTEXITCODE
Pop-Location
Stop-IfFail $exit 1
$GateResults.Add([pscustomobject]@{ n=1; name="rust_build"; exit=0; note="workspace build ok" })
Write-Pass "Gate-1 rust build OK"

# ---------- GATE 2: Rust Clippy -D warnings ----------
Write-Gate 2 "Rust clippy --workspace --all-targets -- -D warnings"
Push-Location $CargoDir
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | Tee-Object -Variable _out | Select-Object -Last 15
$exit = $LASTEXITCODE
Pop-Location
Stop-IfFail $exit 2
$GateResults.Add([pscustomobject]@{ n=2; name="rust_clippy_D_warnings"; exit=0; note="clippy green (non-error)" })
Write-Pass "Gate-2 clippy OK"

# ---------- GATE 3: Rust workspace tests ----------
Write-Gate 3 "Rust cargo test --workspace (includes DIP orchestrator, ai-agent, primiflow etc.)"
Push-Location $CargoDir
cargo test --workspace 2>&1 | Tee-Object -Variable _out | Select-Object -Last 20
$exit = $LASTEXITCODE
Pop-Location
Stop-IfFail $exit 3
$GateResults.Add([pscustomobject]@{ n=3; name="cargo_test_workspace"; exit=0; note="all test result: ok" })
Write-Pass "Gate-3 rust tests OK"

# ---------- GATE 4: Node 7-test matrix ----------
Write-Gate 4 "Node tests (7 baseline suites): project-atlas / bindings / unified / graph / mcp / atlas-flows / formulas"
Push-Location $Backend
$NodeTests = @(
    "test-project-atlas.js",
    "test/rust_crate_bindings_e2e.js",
    "test/test-unified-data-compat.js",
    "test/test-graph-search-rerank.js",
    "test/test-mcp-protocol.js",
    "test/test-atlas-flows.js",
    "test/test-graph-formulas.js"
)
foreach ($t in $NodeTests) {
    Write-Host ("    -> node {0}" -f $t)
    & node $t 2>&1 | Tee-Object -Variable _out_t | Select-Object -Last 4
    $exit = $LASTEXITCODE
    $GateResults.Add([pscustomobject]@{ n=4; name=("node/{0}" -f $t); exit=$exit; note=(if ($exit -eq 0) { "ok" } else { "FAIL exit={0}" -f $exit }) })
    if ($exit -eq 0) { Write-Pass ("sub {0}" -f $t) } else { Write-Fail ("sub {0} exit={1}" -f $t, $exit) }
}
Pop-Location
$FailCnt = @($GateResults | Where-Object { $_.n -eq 4 -and $_.exit -ne 0 }).Count
if ($FailCnt -gt 3) {
    Write-Fail ("GATE-4 exceeded tolerance of 3 non-critical failures (actual {0})" -f $FailCnt)
    Stop-IfFail 1 4
} else {
    Write-Host ("    [summary] GATE-4 failures = {0}/{1} (<= 3 tolerated, continue gates)" -f $FailCnt, $NodeTests.Count) -ForegroundColor Yellow
}

# ---------- GATE 5: T5 + T6 C3 normalization single-source tests ----------
Write-Gate 5 "C3 normalization tests (T5 graph-single-source 27, T6 intent-single-source 44)"
Push-Location $Backend
& node test/test-graph-formulas-single-source.js 2>&1 | Select-Object -Last 3
$exit = $LASTEXITCODE; Stop-IfFail $exit 5
& node test/test-intent-single-source.js  2>&1 | Select-Object -Last 3
$exit = $LASTEXITCODE; Stop-IfFail $exit 5
Pop-Location
$GateResults.Add([pscustomobject]@{ n=5; name="c3_T5_T6_singlesource"; exit=0; note="graph 27 + intent 44 green" })
Write-Pass "Gate-5 T5+T6 ok"

# ---------- GATE 6: T7 no-duplicate watchdog ----------
Write-Gate 6 "No-duplicate-functions watchdog (C3 anti copy-paste reimplementation)"
Push-Location $Backend
& node scripts/validate_no_duplicate_functions.js 2>&1 | Select-Object -Last 10
$exit = $LASTEXITCODE; Stop-IfFail $exit 6
Pop-Location
$GateResults.Add([pscustomobject]@{ n=6; name="duplicate_funcs_watchdog"; exit=0; note="5 families 21/21" })
Write-Pass "Gate-6 watchdog ok"

# ---------- GATE 7: T12 integration triple (Rust dep / node bindings / T12 algo reconcile) ----------
Write-Gate 7 "T12 Integration triple (Rust deps + bindings e2e + algorithm reconcile)"
Push-Location $Backend
& node scripts/validate_rust_workspace_deps.js 2>&1 | Select-Object -Last 4
$exit = $LASTEXITCODE; Stop-IfFail $exit 7
& node test/rust_crate_bindings_e2e.js 2>&1 | Select-Object -Last 3
$exit = $LASTEXITCODE; Stop-IfFail $exit 7
& node test/test-t12-algorithm-reconcile.js 2>&1 | Select-Object -Last 3
$exit = $LASTEXITCODE; Stop-IfFail $exit 7
Pop-Location
$GateResults.Add([pscustomobject]@{ n=7; name="T12_triple_integration"; exit=0; note="dep+bind+reconcile green" })
Write-Pass "Gate-7 T12 ok"

# ---------- Summary ----------
Write-Host "`n================ Enterprise 7-GATE Final Summary ================" -ForegroundColor Cyan
$FailN = 0
foreach ($g in $GateResults) {
    if ($g.exit -eq 0) { Write-Host ("   OK  G{0,-2} {1,-42} - {2}" -f $g.n, $g.name, $g.note) }
    else          { Write-Host ("  FAIL G{0,-2} {1,-42} - {2}" -f $g.n, $g.name, $g.note) -ForegroundColor Red; $FailN++ }
}
Write-Host ("`nTotal Failures = {0}" -f $FailN) -ForegroundColor $(if ($FailN -eq 0) { "Green" } else { "Red" })
if ($FailN -eq 0) {
    Write-Host "*** ALL 7 GATES PASSED: Full-stack normalization green ***" -ForegroundColor Green
    exit 0
} else {
    Write-Host "Gate failures present; non-zero exit." -ForegroundColor Red
    exit 1
}
