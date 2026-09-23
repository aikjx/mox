[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$manifest = Join-Path $workspace 'target/alliance-local/processes.json'
if (-not (Test-Path -LiteralPath $manifest)) { Write-Output 'No local alliance launch manifest.'; return }
foreach ($entry in (Get-Content -LiteralPath $manifest | ConvertFrom-Json)) {
    $process = Get-Process -Id $entry.id -ErrorAction SilentlyContinue
    if (-not $process) { continue }
    $expected = [IO.Path]::GetFullPath($entry.binary)
    $allowedDirectory = [IO.Path]::GetFullPath((Join-Path $workspace 'target/debug')) + [IO.Path]::DirectorySeparatorChar
    $sameStart = [Math]::Abs(($process.StartTime.ToUniversalTime() - ([datetime]$entry.startedAt).ToUniversalTime()).TotalSeconds) -lt 1
    if (-not $expected.StartsWith($allowedDirectory, [StringComparison]::OrdinalIgnoreCase) -or $process.Path -ne $expected -or -not $sameStart) {
        throw "Process identity no longer matches the launch manifest: $($entry.id). Nothing else will be stopped."
    }
    Stop-Process -Id $entry.id
    Write-Output "Stopped local alliance process $($entry.id)."
}
