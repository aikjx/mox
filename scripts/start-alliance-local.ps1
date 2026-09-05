[CmdletBinding()]
param(
    [int]$GatewayPort = 33080,
    [int]$SchedulerPort = 33100,
    [int]$ExecutorPort = 33200,
    [switch]$SkipBuild
)
$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$runDirectory = Join-Path $workspace 'target/alliance-local'
New-Item -ItemType Directory -Force -Path $runDirectory | Out-Null
$ports = @($GatewayPort, $SchedulerPort, $ExecutorPort)
if (($ports | Select-Object -Unique).Count -ne 3 -or ($ports | Where-Object { $_ -lt 1024 -or $_ -gt 65535 })) {
    throw 'Choose three distinct ports between 1024 and 65535.'
}
foreach ($port in $ports) {
    if (Get-NetTCPConnection -State Listen -LocalPort $port -ErrorAction SilentlyContinue) {
        throw "Port $port is already in use. Choose another port; existing processes will not be stopped."
    }
}
if (-not $SkipBuild) {
    Push-Location $workspace
    try {
        & cargo build -p mox-alliance-scheduler-svc -p mox-alliance-executor-svc -p mox-platform-gateway-svc --bins
        if ($LASTEXITCODE -ne 0) { throw 'Service build failed.' }
    } finally { Pop-Location }
}

$started = [System.Collections.Generic.List[object]]::new()
function Start-ServiceProcess([string]$binary, [hashtable]$variables) {
    $executable = Join-Path $workspace "target/debug/$binary.exe"
    if (-not (Test-Path -LiteralPath $executable)) { throw "Build missing: $executable" }
    $directory = Join-Path $runDirectory $binary
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
    $original = @{}
    try {
        foreach ($key in $variables.Keys) {
            $original[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
            [Environment]::SetEnvironmentVariable($key, $variables[$key], 'Process')
        }
        $process = Start-Process -FilePath $executable -WorkingDirectory $directory -WindowStyle Hidden -PassThru `
            -RedirectStandardOutput (Join-Path $directory 'stdout.log') -RedirectStandardError (Join-Path $directory 'stderr.log')
        $started.Add([pscustomobject]@{ id = $process.Id; binary = $executable; startedAt = $process.StartTime.ToUniversalTime().ToString('o') })
    } finally {
        foreach ($key in $original.Keys) { [Environment]::SetEnvironmentVariable($key, $original[$key], 'Process') }
    }
}
try {
    Start-ServiceProcess 'mox-alliance-executor' @{
        MOX_ALLIANCE_CONFIG_FILE = (Join-Path $workspace 'config/alliance-executor.yml')
        MOX_ALLIANCE_SERVER_HOST = '127.0.0.1'; MOX_ALLIANCE_SERVER_PORT = "$ExecutorPort"
        MOX_ALLIANCE_EXECUTOR_MODE = 'expert'; EXECUTOR_MODE = 'expert'
    }
    Start-ServiceProcess 'mox-alliance-scheduler' @{
        MOX_ALLIANCE_CONFIG_FILE = (Join-Path $workspace 'config/alliance-scheduler.yml')
        MOX_ALLIANCE_EXPERTS_FILE = (Join-Path $workspace 'config/alliance-experts.yml')
        MOX_ALLIANCE_SERVER_HOST = '127.0.0.1'; MOX_ALLIANCE_SERVER_PORT = "$SchedulerPort"
        MOX_ALLIANCE_EXECUTOR_BRIDGE_BASE_URL = "http://127.0.0.1:$ExecutorPort"
        MOX_ALLIANCE_STORAGE_MODE = 'file'; MOX_ALLIANCE_STORAGE_PATH = (Join-Path $runDirectory 'tasks.json')
    }
    Start-ServiceProcess 'mox-server' @{
        MOX_GATEWAY_HOST = '127.0.0.1'; MOX_GATEWAY_PORT = "$GatewayPort"
        MOX_ALLIANCE_SCHEDULER_URL = "http://127.0.0.1:$SchedulerPort"
        MOX_ALLIANCE_EXECUTOR_URL = "http://127.0.0.1:$ExecutorPort"
        MOX_ALLIANCE_REMOTE_MODE = 'auto'; MOX_DEV_MODE = '1'
    }
    $started | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $runDirectory 'processes.json') -Encoding utf8
    Write-Output "Local expert-mode services started. Gateway: http://127.0.0.1:$GatewayPort"
    Write-Output "Readiness: /api/alliance/runtime. State and logs: $runDirectory"
    Write-Output 'This launcher is for loopback development; it does not configure production tenant authentication or durable executor recovery.'
} catch {
    foreach ($entry in $started) {
        $process = Get-Process -Id $entry.id -ErrorAction SilentlyContinue
        if ($process -and $process.Path -eq $entry.binary) { Stop-Process -Id $entry.id }
    }
    throw
}
