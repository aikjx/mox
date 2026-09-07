#Requires -Version 5.1
<#
.SYNOPSIS
  MOX 企业级五进程一键启动（编排器 + 知识库 + 联盟调度/执行 + 网关）

.DESCRIPTION
  基于 2026-09-07 实测通过的全链路启动命令（见 docs/API-REGISTRY.md 4.1）：
  1) mox-alliance-scheduler  :3100   2) mox-alliance-executor :3200
  3) mox-kb-server           :8104   4) operator-server        :3001
  5) mox-server（网关）       :8080
  幂等：已监听端口自动跳过；缺失二进制给出明确提示。

.EXAMPLE
  .\scripts\start-mox-enterprise.ps1
  .\scripts\start-mox-enterprise.ps1 -Token my-token
#>
param(
    [string]$Token = "dev-secret-token",
    [switch]$NoWait
)

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root
$DebugBin = Join-Path $Root "target\debug"
$LogDir = Join-Path $Root "target\.logs\enterprise"
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

function Test-Port([int]$port) {
    return [bool](Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue)
}

function Start-One {
    param($Name, [int]$Port, [string[]]$ArgList, [hashtable]$Env)
    if (Test-Port $Port) {
        Write-Host "  [$Port] $Name 已在运行，跳过" -ForegroundColor Yellow
        return
    }
    $exe = Join-Path $DebugBin "$Name.exe"
    if (-not (Test-Path $exe)) {
        Write-Host "  [缺失] $Name.exe 未构建" -ForegroundColor Red
        return
    }
    foreach ($k in $Env.Keys) { Set-Item -Path "Env:$k" -Value $Env[$k] }
    $o = Join-Path $LogDir "$Name.out"
    $e = Join-Path $LogDir "$Name.err"
    Start-Process -FilePath $exe -ArgumentList $ArgList -RedirectStandardOutput $o -RedirectStandardError $e -WindowStyle Hidden
    Write-Host "  [$Port] $Name 启动中…" -ForegroundColor Green
}

Write-Host "=== MOX 企业级五进程启动 ===" -ForegroundColor Cyan

Start-One -Name "mox-alliance-scheduler" -Port 3100 -ArgList @("--port","3100") -Env @{}
Start-One -Name "mox-alliance-executor" -Port 3200 -ArgList @("--port","3200") -Env @{}
Start-One -Name "mox-kb-server" -Port 8104 -ArgList @("--port","8104") -Env @{}

$opEnv = @{ OUS_ENABLE_MOX_SYSTEM = "0"; OUS_API_TOKEN = $Token }
Start-One -Name "operator-server" -Port 3001 -ArgList @("--port","3001") -Env $opEnv

$gwEnv = @{
    MOX_ALLIANCE_SCHEDULER_URL = "http://127.0.0.1:3100"
    MOX_ALLIANCE_EXECUTOR_URL = "http://127.0.0.1:3200"
}
Start-One -Name "mox-server" -Port 8080 -ArgList @("--port","8080") -Env $gwEnv

if (-not $NoWait) {
    Write-Host "等待就绪…"
    Start-Sleep -Seconds 8
    Write-Host ""
    Write-Host "=== 端口状态 ===" -ForegroundColor Cyan
    foreach ($p in 3100, 3200, 8104, 3001, 8080) {
        if (Test-Port $p) {
            $proc = Get-NetTCPConnection -LocalPort $p -State Listen
            Write-Host "  [$p] UP pid=$($proc[0].OwningProcess)" -ForegroundColor Green
        } else {
            Write-Host "  [$p] DOWN（查看 $LogDir 下对应 .err）" -ForegroundColor Red
        }
    }
}
Write-Host ""
Write-Host "验证: curl http://127.0.0.1:8080/api/v1/status -H 'Authorization: Bearer $Token'" -ForegroundColor DarkGray
