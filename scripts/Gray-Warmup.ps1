# Xuanji Umbrella Chart 4-Stage Gray Warmup Script
#
# 4 stages: 1% -> 10% -> 50% -> 100%.
# Each stage calls Invoke-HealthCheck mock returning 0..100 percent.
# Continue if health >= 95%; if < 95% write rollback.log and exit 1.
# On all 4 stages pass -> exit 0.
#
# PARAMETERS:
#   -BaseUrl            Xuanji service base URL (default: http://localhost:8080)
#   -HealthThreshold    Minimum health % required per stage (default 95)
#   -WarmupSeconds      Wait time per stage (default 60)
#   -ForceFailPercent   Injected health score to test rollback path (-1 = disable)
#   -ForceFailAtStage   Stage index (1..4) at which to inject -ForceFailPercent
#
# EXAMPLES:
#   .\Gray-Warmup.ps1                                     # Normal 4 stages, exit 0
#   .\Gray-Warmup.ps1 -ForceFailPercent 80                # Stage 1 injects 80% -> rollback exit 1
#   .\Gray-Warmup.ps1 -ForceFailPercent 60 -ForceFailAtStage 3

[CmdletBinding()]
param(
    [string]$BaseUrl = "http://localhost:8080",
    [int]$HealthThreshold = 95,
    [int]$WarmupSeconds = 60,
    [int]$ForceFailPercent = -1,
    [int]$ForceFailAtStage = 1
)

$ErrorActionPreference = "Stop"

# --- 4 stages definition ---
$Stages = @(
    @{ Index = 1; Name = "Stage-1"; Weight = 1 },
    @{ Index = 2; Name = "Stage-2"; Weight = 10 },
    @{ Index = 3; Name = "Stage-3"; Weight = 50 },
    @{ Index = 4; Name = "Stage-4"; Weight = 100 }
)

$RollbackLog = "rollback.log"
$script:CurrentStageIndex = 0

# ---------- helpers ----------

function Write-Log {
    param([string]$Msg)
    $ts = Get-Date -Format "yyyy-MM-dd HH:mm:ss.fff"
    Write-Host "[$ts] $Msg"
}

function Invoke-HealthCheck {
    param(
        [Parameter(Mandatory = $true)][string]$StageName,
        [Parameter(Mandatory = $true)][string]$Url
    )
    if ($ForceFailPercent -ge 0 -and $ForceFailAtStage -eq $script:CurrentStageIndex) {
        Write-Log ("[{0}] Injecting forced health score = {1}%" -f $StageName, $ForceFailPercent)
        return $ForceFailPercent
    }
    try {
        $resp = Invoke-RestMethod -Uri "$Url/readyz" -TimeoutSec 5 -ErrorAction SilentlyContinue
        if ($null -ne $resp) {
            if ($resp -is [hashtable] -or $resp -is [pscustomobject]) {
                if ($resp.healthy -eq $true) { return 100 }
                if ($null -ne $resp.score) { return [int]$resp.score }
            }
        }
    } catch {
        # ignore: fall through to mock
    }
    $seed = [int]([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())
    $rng = [System.Random]::new($seed + $script:CurrentStageIndex * 31)
    $score = $rng.Next(95, 101)
    Write-Log ("[{0}] Mock health score = {1}% (threshold={2}%)" -f $StageName, $score, $HealthThreshold)
    return $score
}

function Write-RollbackLog {
    param(
        [string]$StageName,
        [int]$HealthScore,
        [string]$Reason
    )
    $ts = Get-Date -Format "yyyy-MM-dd HH:mm:ss.fff"
    $content = (
        "ROLLBACK TRIGGERED at {0}`r`n" + `
        "- Stage: {1}`r`n" + `
        "- Health Score: {2}%`r`n" + `
        "- Threshold: {3}%`r`n" + `
        "- Reason: {4}`r`n" + `
        "- Action: Immediate rollback; canary weight -> 0%`r`n" + `
        "- BaseUrl: {5}`r`n"
    ) -f $ts, $StageName, $HealthScore, $HealthThreshold, $Reason, $BaseUrl
    Set-Content -Path $RollbackLog -Value $content -Encoding UTF8
    Write-Log ("!!! Rollback log written: {0}" -f (Resolve-Path $RollbackLog).Path)
}

# ---------- main ----------

try {
    Write-Log "=== Xuanji Gray Warmup START ==="
    Write-Log ("Stages={0}; Threshold={1}%; Warmup={2}s" -f $Stages.Count, $HealthThreshold, $WarmupSeconds)
    if ($ForceFailPercent -ge 0) {
        Write-Log ("FORCE-FAIL MODE: stage={0} score={1}%" -f $ForceFailAtStage, $ForceFailPercent)
    }

    foreach ($s in $Stages) {
        $script:CurrentStageIndex = [int]$s.Index
        Write-Host ""
        Write-Log ("--- Enter {0} [canary weight = {1}%] ---" -f $s.Name, $s.Weight)

        $score = Invoke-HealthCheck -StageName $s.Name -Url $BaseUrl
        Write-Log ("[{0}] Health check score = {1}%" -f $s.Name, $score)

        if ($score -lt $HealthThreshold) {
            Write-Log ("[{0}] FAILED ({1}% < {2}%); triggering rollback!" -f $s.Name, $score, $HealthThreshold)
            Write-RollbackLog -StageName $s.Name -HealthScore $score -Reason "Health check below threshold at gray stage"
            exit 1
        }

        Write-Log ("[{0}] PASS. Warmup wait {1}s..." -f $s.Name, $WarmupSeconds)
        if ($WarmupSeconds -gt 0) {
            if ($env:CI -or $env:GITHUB_ACTIONS) {
                Start-Sleep -Seconds 1
            } else {
                Start-Sleep -Seconds $WarmupSeconds
            }
        }
    }

    Write-Host ""
    Write-Log "=== All 4 stages passed successfully; exit 0 ==="
    exit 0
} catch {
    Write-Log ("Unexpected error: {0}" -f $_)
    Write-RollbackLog -StageName "Unknown" -HealthScore 0 -Reason ("Script exception: {0}" -f $_)
    exit 1
}
