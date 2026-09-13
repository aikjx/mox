# Start the desktop app with the isolated, locally validated neural runtime.
$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $projectRoot "../.."))
$runtimePython = Join-Path $repositoryRoot ".runtime/melody2score-ai-env/Scripts/python.exe"
if (-not (Test-Path -LiteralPath $runtimePython)) {
    throw "AI runtime missing. See requirements-ai.txt and the accuracy report for setup."
}
& $runtimePython -c "import torchcrepe, torchaudio"
if ($LASTEXITCODE -ne 0) { throw "AI runtime dependency check failed." }
& $runtimePython (Join-Path $PSScriptRoot "gui.py")
exit $LASTEXITCODE
