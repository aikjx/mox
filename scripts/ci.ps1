# 算子统一系统（OUS）一键全自动化：构建 + 测试 + 启动 + 端到端健康检查
# 用法：powershell -ExecutionPolicy Bypass -File scripts/ci.ps1
$ErrorActionPreference = 'Stop'
$ROOT = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $ROOT

function Step($msg) { Write-Host "`n===== $msg =====" -ForegroundColor Cyan }

# 1) 清理可能占用 target exe 的残留服务器进程（避免 cargo 无法覆盖）
Step '清理残留进程'
Get-Process -Name 'operator-server' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

# 2) 全量构建
Step 'cargo build --workspace'
cargo build --workspace 2>&1 | Select-Object -Last 5

# 3) 全量测试
Step 'cargo test --workspace'
cargo test --workspace 2>&1 | Select-String -Pattern 'test result' | ForEach-Object { Write-Host $_.Line }

# 4) 前端构建
Step 'npm run build (frontend)'
Push-Location frontend
if (-not (Test-Path node_modules)) { npm install }
npm run build 2>&1 | Select-Object -Last 6
Pop-Location

# 5) 启动服务器并做端到端健康检查（含专家联盟双联盟十四维 API）
Step '端到端健康检查 /api/alliance/*'
$job = Start-Process -FilePath 'cargo' -ArgumentList 'run','-p','runtime' -PassThru -RedirectStandardOutput 'ci_server.out' -RedirectStandardError 'ci_server.err'
try {
    $ready = $false
    for ($i = 0; $i -lt 60; $i++) {
        try {
            $r = Invoke-WebRequest -Uri 'http://localhost:3000/api/health' -UseBasicParsing -TimeoutSec 1
            if ($r.StatusCode -eq 200) { $ready = $true; break }
        } catch { Start-Sleep -Seconds 1 }
    }
    if (-not $ready) { throw '服务器未在 60s 内就绪' }
    Write-Host 'health: 200 OK'

    $h = Invoke-RestMethod -Uri 'http://localhost:3000/api/alliance/health' -Method Get
    Write-Host ('alliance model: ' + $h.alliance + ' | 维度数: ' + $h.dimensions.Count)

    $g = Invoke-RestMethod -Uri 'http://localhost:3000/api/alliance/optimize' -Method Post `
        -ContentType 'application/json' `
        -Body '{"flow":{"nodes":[{"id":"n1","type":"input"}],"edges":[]}}'
    Write-Host ('governance expert_scores: ' + $g.expert_scores.Count + ' | 闸门通过: ' + $g.gate.approved)
    Write-Host '端到端全维度治理验证通过' -ForegroundColor Green
} finally {
    Stop-Process -Id $job.Id -Force -ErrorAction SilentlyContinue
}

Step '全部完成 ✅'
