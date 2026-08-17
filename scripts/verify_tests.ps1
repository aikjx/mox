# 算子统一系统 (OUS) —— 全量功能验证测试脚本 (PowerShell)
# 用法（仓库根目录）：
#   pwsh ./verify_tests.ps1           # 仅单元/集成测试
#   pwsh ./verify_tests.ps1 -E2E      # 额外启动 runtime 并探测 HTTP 端点
param([switch]$E2E)

$ErrorActionPreference = "Stop"
$ROOT = Split-Path -Parent $MyInvocation.MyCommand.Definition
Push-Location $ROOT

function Pass { param($m) Write-Host "[PASS] $m" -ForegroundColor Green }
function Fail { param($m) Write-Host "[FAIL] $m" -ForegroundColor Red; exit 1 }
function Step { param($m) Write-Host "`n>>> $m" -ForegroundColor Yellow }

Write-Host "============================================================"
Write-Host " OUS 全量验证  @ $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
Write-Host "============================================================"

# 1. 编译（含测试目标）
Step "[1/3] 编译 workspace + tests ..."
cargo test --workspace --no-run *> build_tests.log
if ($LASTEXITCODE -ne 0) { Fail "编译失败，详见 build_tests.log" }

# 2. 运行所有单元测试 / 集成测试
Step "[2/3] 运行 cargo test --workspace ..."
cargo test --workspace *> test_report.log
if ($LASTEXITCODE -ne 0) { Fail "存在失败的测试，详见 test_report.log" }
$total = (Select-String -Path test_report.log -Pattern "test result: ok\. (\d+) passed" |
          ForEach-Object { [int]$_.Matches.Groups[1].Value } | Measure-Object -Sum).Sum
Pass "全部单元测试通过，合计 $total 个用例 (0 failed)"

# 3. 端到端 API 冒烟（可选）
if ($E2E) {
    Step "[3/3] 端到端 API 冒烟 ..."
    $PORT = 3998
    $env:RUST_LOG = "warn"
    Start-Process -NoNewWindow cargo -ArgumentList "run","-p","runtime","--","--port",$PORT -RedirectStandardOutput runtime_smoke.out -RedirectStandardError runtime_smoke.err
    # 等待端口就绪
    $ready = $false
    for ($i = 0; $i -lt 30; $i++) {
        try { $r = Invoke-WebRequest -Uri "http://127.0.0.1:$PORT/" -UseBasicParsing -TimeoutSec 2; if ($r.StatusCode -eq 200) { $ready = $true; break } } catch {}
        Start-Sleep -Seconds 1
    }
    if (-not $ready) { Fail "runtime 未在 $PORT 就绪" }
    function Check($path) {
        try { $c = (Invoke-WebRequest -Uri "http://127.0.0.1:$PORT$path" -UseBasicParsing -TimeoutSec 5).StatusCode
              if ($c -eq 200) { Pass "GET $path -> $c" } else { Fail "GET $path -> $c" } }
        catch { Fail "GET $path -> $_" }
    }
    Check "/"; Check "/api/operators"; Check "/api/graph"
    try { $c = (Invoke-WebRequest -Uri "http://127.0.0.1:$PORT/api/ai/chat" -Method POST `
            -Body '{"session_id":"v1","message":"列出所有算子"}' -ContentType "application/json" `
            -UseBasicParsing -TimeoutSec 8).StatusCode
          if ($c -eq 200) { Pass "POST /api/ai/chat -> $c" } else { Fail "POST /api/ai/chat -> $c" } }
    catch { Fail "POST /api/ai/chat -> $_" }
    Get-Process operator-server -ErrorAction SilentlyContinue | Stop-Process -Force
    Pass "端到端 API 冒烟全部通过"
} else {
    Write-Host ">>> [3/3] 跳过端到端 (使用 -E2E 启用)" -ForegroundColor Yellow
}

Write-Host "============================================================"
Write-Host " [OK] OUS 验证完成" -ForegroundColor Green
Write-Host "============================================================"
Pop-Location
