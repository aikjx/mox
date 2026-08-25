# Run-V21-AllTests.ps1
# ============================================================
# Xuanji v2.1 Validation Harness - Runs T22 / T23 / T24 / T25
# cargo lib tests, aggregates results, and emits summary.
#
# Run me from the repository root:
#   powershell -ExecutionPolicy Bypass -File .\.trae\specs\20260824-v2.1-t22-t23-t24-t25-simd-graph-gm-glacier\Run-V21-AllTests.ps1
#
# Requires: rustup, cargo (stable, ideally nightly for -Z unstable-options)
# Optional: nightly toolchain for --report-dir JSON reports.
# If nightly is unavailable, the harness falls back to text-output
# capture and regex-based counting, which is still deterministic.
# ============================================================

[CmdletBinding()]
param(
    # When true, skip the gm-sm/dual_chain feature phase (T24)
    # to run a minimal "baseline" set that still exits 0.
    [switch]$Baseline,

    # Override the pass-count threshold (NFR7 door).
    [int]$Threshold = 152,

    # Repository root (defaults to the location of this script + 4 levels up).
    [string]$RepoRoot
)

$ErrorActionPreference = 'Continue'

# --------- Encoding helper: write UTF-8 w/o BOM ----------
function Write-Utf8NoBom {
    param(
        [Parameter(Mandatory)] [string]$Path,
        [Parameter(Mandatory)] [string]$Content
    )
    $dir = Split-Path -Parent $Path
    if ($dir -and -not (Test-Path -LiteralPath $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Content, $utf8NoBom)
}

# --------- Resolve repo root ----------
if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))
}
$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
Write-Host "[Run-V21-AllTests] RepoRoot = $RepoRoot" -ForegroundColor Cyan

# --------- Artifact directories ----------
$ProjectsRoot = Join-Path $RepoRoot 'projects'
if (-not (Test-Path -LiteralPath $ProjectsRoot)) {
    New-Item -ItemType Directory -Path $ProjectsRoot -Force | Out-Null
}

$Phases = @(
    @{
        Key        = 'P1-T22'
        Title      = 'T22 - SIMD AVX2 / NEON accelerated erasure coding (xuanji-cloud-drive-volume)'
        Packages   = @('xuanji-cloud-drive-volume')
        Features   = 'simd'
        BaselineSafe = $true
    },
    @{
        Key        = 'P2-T23'
        Title      = 'T23 - Fusion CDC Tag-Graph 20 projection-list + Graph service (xuanji-fusion + xuanji-graph-service)'
        Packages   = @('xuanji-fusion', 'xuanji-graph-service')
        Features   = $null
        BaselineSafe = $true
    },
    @{
        Key        = 'P3-T24'
        Title      = 'T24 - GM-SM 国密 & dual_chain hash-chain compliance (xuanji-standards)'
        Packages   = @('xuanji-standards')
        Features   = 'gm-sm dual_chain'
        BaselineSafe = $false
    },
    @{
        Key        = 'P4-T25'
        Title      = 'T25 - Glacier storage class + S3 gateway (xuanji-cloud-drive-s3)'
        Packages   = @('xuanji-cloud-drive-s3')
        Features   = 'glacier'
        BaselineSafe = $true
    }
)

function Write-Banner {
    param([Parameter(Mandatory)][string]$Text)
    $bar = '=' * ($Text.Length + 4)
    Write-Host ""
    Write-Host $bar -ForegroundColor Magenta
    Write-Host ("| " + $Text + " |") -ForegroundColor Magenta
    Write-Host $bar -ForegroundColor Magenta
    Write-Host ""
}

function New-PhasedArtifactsDir {
    param([Parameter(Mandatory)][string]$PhaseKey)
    $root = Join-Path $ProjectsRoot "$PhaseKey-artifacts"
    $runs = Join-Path $root 'runs'
    $latest = Join-Path $runs 'latest'
    # Idempotent: wipe latest/ then recreate.
    if (Test-Path -LiteralPath $latest) {
        Remove-Item -LiteralPath $latest -Recurse -Force -ErrorAction SilentlyContinue
    }
    New-Item -ItemType Directory -Path $latest -Force | Out-Null
    return $latest
}

function Invoke-CargoTestWithFallback {
    param(
        [Parameter(Mandatory)][string]$Package,
        [string]$Features,
        [Parameter(Mandatory)][string]$OutDir
    )

    $outLog = Join-Path $OutDir "cargo-$Package.log"
    $reportDir = Join-Path $OutDir "report"
    New-Item -ItemType Directory -Path $reportDir -Force | Out-Null

    # 1) Try unstable JSON report first.
    $args = @('test', '-p', $Package, '--lib')
    if (-not [string]::IsNullOrWhiteSpace($Features)) {
        $args += @('--features', $Features)
    }
    $args += @('-Z', 'unstable-options', "--report-dir=$reportDir", '--format', 'json')

    Write-Host "    cargo $($args -join ' ')" -ForegroundColor Gray
    $proc = Start-Process -FilePath cargo -ArgumentList $args `
        -WorkingDirectory $RepoRoot `
        -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput (Join-Path $OutDir "cargo-$Package.stdout.json") `
        -RedirectStandardError (Join-Path $OutDir "cargo-$Package.stderr.log")

    $usedJsonReport = $false
    if ($proc.ExitCode -eq 0 -or $proc.ExitCode -eq 101 -or $proc.ExitCode -eq 1) {
        # Check whether we actually got JSON reports (if cargo didn't support -Z, stderr will say so).
        $jsonCandidates = Get-ChildItem -LiteralPath $reportDir -Recurse -Filter '*.json' -ErrorAction SilentlyContinue
        if ($jsonCandidates -and $jsonCandidates.Count -gt 0) {
            $usedJsonReport = $true
        }
    }

    # 2) Fallback: run again with text output, then parse summary via regex.
    $exitCode = $proc.ExitCode
    if (-not $usedJsonReport) {
        $args2 = @('test', '-p', $Package, '--lib')
        if (-not [string]::IsNullOrWhiteSpace($Features)) {
            $args2 += @('--features', $Features)
        }
        # Capture to log file.
        Write-Host "    [fallback] cargo $($args2 -join ' ')" -ForegroundColor DarkYellow
        $proc2 = Start-Process -FilePath cargo -ArgumentList $args2 `
            -WorkingDirectory $RepoRoot `
            -NoNewWindow -Wait -PassThru `
            -RedirectStandardOutput $outLog `
            -RedirectStandardError (Join-Path $OutDir "cargo-$Package.stderr2.log")
        $exitCode = $proc2.ExitCode
    }

    # 3) Count pass/fail/ignored + extract test names.
    $passed = 0; $failed = 0; $ignored = 0
    $testNames = New-Object System.Collections.Generic.List[string]

    if ($usedJsonReport) {
        foreach ($jf in (Get-ChildItem -LiteralPath $reportDir -Recurse -Filter '*.json')) {
            try {
                $raw = Get-Content -LiteralPath $jf.FullName -Raw -Encoding UTF8
                if ($raw -match '^\s*\[') { $arr = $raw | ConvertFrom-Json } else { $arr = @($raw | ConvertFrom-Json) }
                foreach ($e in $arr) {
                    if ($e.type -eq 'test') {
                        if ($e.name) { [void]$testNames.Add([string]$e.name) }
                        if ($e.event -eq 'ok') { $passed++ }
                        elseif ($e.event -eq 'failed') { $failed++ }
                        elseif ($e.event -eq 'ignored') { $ignored++ }
                    }
                    elseif ($e.type -eq 'suite') {
                        if ($e.event -eq 'ok' -and $e.passed) { $passed += [int]$e.passed }
                        if ($e.event -eq 'failed' -and $e.failed) { $failed += [int]$e.failed }
                        if ($e.ignored) { $ignored += [int]$e.ignored }
                    }
                }
            } catch {
                Write-Warning "  JSON parse failed on $($jf.FullName): $_"
            }
        }
    }

    # Always also parse raw stdout lines to ensure we capture counts robustly.
    $logTargets = @($outLog)
    if (Test-Path -LiteralPath (Join-Path $OutDir "cargo-$Package.stdout.json")) {
        $logTargets += Join-Path $OutDir "cargo-$Package.stdout.json"
    }
    foreach ($log in $logTargets) {
        if (-not (Test-Path -LiteralPath $log)) { continue }
        $lines = Get-Content -LiteralPath $log -Encoding UTF8
        foreach ($ln in $lines) {
            # test <name> ... <ok|FAILED|ignored>
            if ($ln -match '^test (.+?)\s+\.\.\.\s+(ok|FAILED|ignored)') {
                $tn = $Matches[1].Trim()
                if (-not $testNames.Contains($tn)) { [void]$testNames.Add($tn) }
                # if JSON was missing this granular event, count here too.
                if (-not $usedJsonReport) {
                    if ($Matches[2] -eq 'ok') { $passed++ }
                    elseif ($Matches[2] -eq 'FAILED') { $failed++ }
                    else { $ignored++ }
                }
            }
            # test result: ok. X passed; Y failed; Z ignored;
            if ($ln -match '^test result:\s+(ok|FAILED)\.\s+(\d+)\s+passed;\s+(\d+)\s+failed(?:;\s+(\d+)\s+ignored)?') {
                $p = [int]$Matches[2]; $f = [int]$Matches[3]; $ig = 0
                if ($Matches[4]) { $ig = [int]$Matches[4] }
                # Use aggregate if it dominates (avoids over-counting duplicate sources).
                if (($p + $f + $ig) -gt ($passed + $failed + $ignored)) {
                    $passed = $p; $failed = $f; $ignored = $ig
                }
            }
        }
    }

    return [pscustomobject]@{
        Package   = $Package
        Passed    = $passed
        Failed    = $failed
        Ignored   = $ignored
        TestNames = @($testNames)
        ExitCode  = [int]$exitCode
        UsedJson  = [bool]$usedJsonReport
    }
}

# --------- Execute phases ----------
$PhaseReports = New-Object System.Collections.Generic.List[object]
$overallExit = 0

foreach ($Phase in $Phases) {
    $phaseKey = $Phase.Key
    Write-Banner "PHASE $phaseKey  ::  $($Phase.Title)"

    if ($Baseline -and -not $Phase.BaselineSafe) {
        Write-Host "  [BASELINE MODE] Skipping phase $phaseKey (gm-sm/dual_chain feature not built)..." -ForegroundColor DarkGray
        $artifactsDir = New-PhasedArtifactsDir -PhaseKey $phaseKey
        $report = [ordered]@{
            phase     = $phaseKey
            passed    = 0
            failed    = 0
            ignored   = 0
            test_names = @()
            exit_code = 0
            skipped   = $true
            packages  = @($Phase.Packages)
            features  = $Phase.Features
        }
        $PhaseReports.Add([pscustomobject]$report)
        $jsonPath = Join-Path $artifactsDir 'report.json'
        Write-Utf8NoBom -Path $jsonPath -Content (ConvertTo-Json $report -Depth 6)
        continue
    }

    $artifactsDir = New-PhasedArtifactsDir -PhaseKey $phaseKey

    $phasePassed = 0; $phaseFailed = 0; $phaseIgnored = 0
    $phaseTests = New-Object System.Collections.Generic.List[string]
    $phaseExit = 0
    $perPkg = @()

    foreach ($pkg in $Phase.Packages) {
        Write-Host "  -> Package: $pkg  (features='$($Phase.Features)')" -ForegroundColor Cyan
        $res = Invoke-CargoTestWithFallback -Package $pkg -Features $Phase.Features -OutDir $artifactsDir
        $phasePassed += $res.Passed
        $phaseFailed += $res.Failed
        $phaseIgnored += $res.Ignored
        foreach ($tn in $res.TestNames) { if (-not $phaseTests.Contains($tn)) { [void]$phaseTests.Add($tn) } }
        if ($res.ExitCode -ne 0) { $phaseExit = 1 }
        $perPkg += [ordered]@{
            package  = $pkg
            passed   = $res.Passed
            failed   = $res.Failed
            ignored  = $res.Ignored
            exit_code = $res.ExitCode
            used_json_report = $res.UsedJson
        }
        $color = if ($res.ExitCode -eq 0) { 'Green' } else { 'Yellow' }
        Write-Host ("     => passed={0}  failed={1}  ignored={2}  exit={3}" -f $res.Passed, $res.Failed, $res.Ignored, $res.ExitCode) -ForegroundColor $color
    }

    if ($phaseExit -ne 0) { $overallExit = 1 }

    $report = [ordered]@{
        phase      = $phaseKey
        title      = $Phase.Title
        passed     = $phasePassed
        failed     = $phaseFailed
        ignored    = $phaseIgnored
        test_names = @($phaseTests)
        exit_code  = $phaseExit
        packages   = $perPkg
        features   = $Phase.Features
        skipped    = $false
    }
    $PhaseReports.Add([pscustomobject]$report)
    $jsonPath = Join-Path $artifactsDir 'report.json'
    Write-Utf8NoBom -Path $jsonPath -Content (ConvertTo-Json $report -Depth 8)
}

# --------- P5: Summary ----------
Write-Banner "PHASE P5-Summary  ::  Aggregate report generation"

$totalPassed = 0; $totalFailed = 0; $totalIgnored = 0
foreach ($r in $PhaseReports) {
    $totalPassed += [int]$r.passed
    $totalFailed += [int]$r.failed
    $totalIgnored += [int]$r.ignored
}
$ts = (Get-Date).ToUniversalTime().ToString('o')
$thresholdOk = $totalPassed -ge $Threshold

$summaryJson = [ordered]@{
    schema       = 'xuanji-v21-validation-summary@1'
    timestamp_utc = $ts
    threshold    = $Threshold
    threshold_ok = [bool]$thresholdOk
    total_passed = $totalPassed
    total_failed = $totalFailed
    total_ignored = $totalIgnored
    overall_exit_code = $(if ($thresholdOk -and $totalFailed -eq 0) { 0 } elseif (-not $thresholdOk) { 1 } else { 1 })
    per_phase    = @($PhaseReports)
}

$jsonOut = Join-Path $ProjectsRoot 'v21-artifacts-summary.json'
Write-Utf8NoBom -Path $jsonOut -Content (ConvertTo-Json $summaryJson -Depth 10)
Write-Host "  Wrote $jsonOut" -ForegroundColor Cyan

# Markdown summary
$sb = New-Object System.Text.StringBuilder
[void]$sb.AppendLine("# Xuanji v2.1 Test Suite Aggregate Summary")
[void]$sb.AppendLine("")
[void]$sb.AppendLine(("- Generated (UTC): {0}" -f $ts))
[void]$sb.AppendLine(("- Pass threshold (NFR7 door): **{0}**" -f $Threshold))
[void]$sb.AppendLine(("- **Total passed: {0}**" -f $totalPassed))
[void]$sb.AppendLine(("- **Total failed: {0}**" -f $totalFailed))
[void]$sb.AppendLine(("- **Total ignored: {0}**" -f $totalIgnored))
[void]$sb.AppendLine(("- Threshold gate: {1}  ({0} -ge {2})" -f $totalPassed, $(if ($thresholdOk) { 'PASS' } else { 'FAIL' }), $Threshold))
[void]$sb.AppendLine("")
[void]$sb.AppendLine("## Per-Phase")
[void]$sb.AppendLine("")
[void]$sb.AppendLine("| Phase | Package(s) | Features | Passed | Failed | Ignored | Exit |")
[void]$sb.AppendLine("|-------|------------|----------|--------|--------|---------|------|")
foreach ($r in $PhaseReports) {
    $pkgs = @($r.packages.package) -join ', '
    if (-not $pkgs) { $pkgs = @($PhaseReports[0].packages.package) -join ', ' }
    $feats = if ($r.features) { $r.features } else { '(default)' }
    if ($r.skipped) {
        [void]$sb.AppendLine("| $($r.phase) | SKIPPED (baseline) | - | 0 | 0 | 0 | 0 |")
    } else {
        [void]$sb.AppendLine("| $($r.phase) | $pkgs | $feats | $($r.passed) | $($r.failed) | $($r.ignored) | $($r.exit_code) |")
    }
}
[void]$sb.AppendLine("")
[void]$sb.AppendLine("## Detailed counts per package")
[void]$sb.AppendLine("")
foreach ($r in $PhaseReports) {
    if ($r.skipped) { continue }
    [void]$sb.AppendLine("### $($r.phase)")
    foreach ($p in $r.packages) {
        $pkgLine = "- **{0}**: passed={1}, failed={2}, ignored={3}, exit={4}, json_report={5}" -f $p.package,$p.passed,$p.failed,$p.ignored,$p.exit_code,$p.used_json_report
        [void]$sb.AppendLine($pkgLine)
    }
    [void]$sb.AppendLine("")
}

$mdOut = Join-Path $ProjectsRoot 'v21-artifacts-summary.md'
Write-Utf8NoBom -Path $mdOut -Content $sb.ToString()
Write-Host "  Wrote $mdOut" -ForegroundColor Cyan

Write-Banner "OVERALL RESULT"
Write-Host ("  Passed  : {0}" -f $totalPassed) -ForegroundColor Green
Write-Host ("  Failed  : {0}" -f $totalFailed) -ForegroundColor $(if ($totalFailed -gt 0) { 'Red' } else { 'Green' })
Write-Host ("  Threshold gate (passed >= $Threshold) : $(if ($thresholdOk) {'PASS'} else {'FAIL'})" ) -ForegroundColor $(if ($thresholdOk) { 'Green' } else { 'Red' })

if (-not $thresholdOk) {
    Write-Host "  EXIT = 1  (NFR7 door not met)" -ForegroundColor Red
    exit 1
}
if ($overallExit -ne 0 -or $totalFailed -gt 0) {
    Write-Host "  EXIT = 1  (one or more phases had failures)" -ForegroundColor Red
    exit 1
}
Write-Host "  EXIT = 0  (all phases green + door met)" -ForegroundColor Green
exit 0
