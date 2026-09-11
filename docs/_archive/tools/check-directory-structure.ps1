# MOX Directory Structure Check Script
# Used for CI/CD or local verification that directory structure follows guidelines

param(
    [string]$RepoRoot = (Get-Location).Path
)

$ErrorActionPreference = "Stop"

Write-Host "=== MOX Directory Structure Check ===" -ForegroundColor Cyan
Write-Host "Repo root: $RepoRoot"
Write-Host ""

$passed = 0
$failed = 0
$warnings = 0

function Check-Exists {
    param([string]$Path, [string]$Description, [string]$Level = "Error")
    $fullPath = Join-Path $RepoRoot $Path
    if (Test-Path $fullPath) {
        Write-Host "  [OK] $Description" -ForegroundColor Green
        $script:passed++
        return $true
    } else {
        if ($Level -eq "Warning") {
            Write-Host "  [WARN] $Description (missing)" -ForegroundColor Yellow
            $script:warnings++
        } else {
            Write-Host "  [FAIL] $Description (missing)" -ForegroundColor Red
            $script:failed++
        }
        return $false
    }
}

function Check-NotExists {
    param([string]$Path, [string]$Description, [string]$Level = "Error")
    $fullPath = Join-Path $RepoRoot $Path
    if (Test-Path $fullPath) {
        if ($Level -eq "Warning") {
            Write-Host "  [WARN] $Description (exists, should be cleaned)" -ForegroundColor Yellow
            $script:warnings++
        } else {
            Write-Host "  [FAIL] $Description (should not exist)" -ForegroundColor Red
            $script:failed++
        }
        return $false
    } else {
        Write-Host "  [OK] $Description" -ForegroundColor Green
        $script:passed++
        return $true
    }
}

# Core source directories
Write-Host "[Core Source Directories]" -ForegroundColor White
Check-Exists "platform" "platform/ - Core backend source"
Check-Exists "platform/domains" "platform/domains/ - Domain layer"
Check-Exists "platform/foundation" "platform/foundation/ - Foundation layer"
Check-Exists "platform/gateway" "platform/gateway/ - Gateway layer"
Check-Exists "frontend-ui" "frontend-ui/ - Frontend project"
Check-Exists "Cargo.toml" "Cargo.toml - Rust workspace config"

Write-Host ""
Write-Host "[Documentation System]" -ForegroundColor White
Check-Exists "docs" "docs/ - Documentation center"
Check-Exists "docs/architecture" "docs/architecture/ - Architecture docs"
Check-Exists "docs/enterprise" "docs/enterprise/ - Enterprise docs"
Check-Exists "docs/specifications" "docs/specifications/ - Specifications"
Check-Exists "deploy/docs" "deploy/docs/ - Deployment docs"

Write-Host ""
Write-Host "[Deployment & Config]" -ForegroundColor White
Check-Exists "deploy" "deploy/ - Deployment configs"
Check-Exists "deploy/config" "deploy/config/ - App configs"
Check-Exists "docker-compose.yml" "docker-compose.yml - All-in-one deploy"

Write-Host ""
Write-Host "[Prototypes & Tools]" -ForegroundColor White
Check-Exists "prototypes" "prototypes/ - HTML prototype projects" "Warning"
Check-Exists "tools" "tools/ - Dev tool scripts" "Warning"

Write-Host ""
Write-Host "[Migrated Directories (should be gone from root)]" -ForegroundColor White
Check-NotExists "chat-project-generator" "chat-project-generator/ (moved to prototypes/)"
Check-NotExists "data-vis" "data-vis/ (moved to prototypes/)"
Check-NotExists "expert-alliance-cyber" "expert-alliance-cyber/ (moved to prototypes/)"
Check-NotExists "expert-alliance-design" "expert-alliance-design/ (moved to prototypes/)"
Check-NotExists "kg-workflow-guide" "kg-workflow-guide/ (moved to prototypes/)"
Check-NotExists "mox-enterprise-optimization" "mox-enterprise-optimization/ (moved to prototypes/)"
Check-NotExists "config" "config/ (moved to deploy/config/)" "Warning"
Check-NotExists "platform/mox-server" "platform/mox-server/ (moved to legacy/)"
Check-NotExists "platform/backend-rust" "platform/backend-rust/ (moved to legacy/, target/ may remain)" "Warning"
Check-NotExists "platform/mox-store" "platform/mox-store/ (moved to legacy/)"

Write-Host ""
Write-Host "[Runtime Files (should NOT be tracked)]" -ForegroundColor White
$trackedLogs = git ls-files "*.log" 2>$null
if ($trackedLogs) {
    Write-Host "  [FAIL] Tracked .log files found: $($trackedLogs.Count)" -ForegroundColor Red
    $script:failed++
} else {
    Write-Host "  [OK] No tracked .log files" -ForegroundColor Green
    $script:passed++
}

$trackedBak = git ls-files "*.bak" 2>$null
if ($trackedBak) {
    Write-Host "  [FAIL] Tracked .bak files found: $($trackedBak.Count)" -ForegroundColor Red
    $script:failed++
} else {
    Write-Host "  [OK] No tracked .bak files" -ForegroundColor Green
    $script:passed++
}

Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host "  Passed:   $passed" -ForegroundColor Green
Write-Host "  Warnings: $warnings" -ForegroundColor Yellow
Write-Host "  Failed:   $failed" -ForegroundColor Red

if ($failed -gt 0) {
    Write-Host ""
    Write-Host "FAILED: Directory structure check did not pass." -ForegroundColor Red
    exit 1
} else {
    Write-Host ""
    Write-Host "PASSED: Directory structure check is clean!" -ForegroundColor Green
    exit 0
}
