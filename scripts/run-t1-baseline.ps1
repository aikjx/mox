param(
  [string]$Root = 'd:\a10\aikjx\gitcode\infotopograph',
  [int]$TimeoutSecPerCmd = 1200
)
$ErrorActionPreference = 'Stop'
$report = Join-Path $Root 'target\enterprise-baseline-report.txt'
New-Item -ItemType Directory -Force -Path (Split-Path $report) | Out-Null

$enc = [System.Text.UTF8Encoding]::new($false)
function W($text) { [System.IO.File]::AppendAllLines($report, [string[]]$text, $enc) }

W @(
  "=== 璇玑 RelGraph · 企业级验收基线盘点 T1 · v3 ===",
  ("Started: {0}" -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss')),
  ("ROOT: {0}" -f $Root),
  ""
)

function Resolve-Exe($name){
  if(-not $name){ return $null }
  if([IO.File]::Exists($name)){ return $name }
  $cmd = (Get-Command ("{0}.cmd" -f $name) -ErrorAction SilentlyContinue)
  if($cmd){ return $cmd.Source }
  $exe = (Get-Command $name -ErrorAction SilentlyContinue)
  if($exe){ return $exe.Source }
  return $name
}

function Run-Cmd($name, $file, [string[]]$argList, $cdTo){
  $realFile = Resolve-Exe $file
  W @("", ("----- {0} -----" -f $name), ("EXE: {0} (resolved from {1}) ; ARGS: {2} ; CWD: {3}" -f $realFile,$file,($argList -join ' '),$cdTo))
  $sw=[Diagnostics.Stopwatch]::StartNew()
  $ec=0
  $lines_out = [System.Collections.Generic.List[string]]::new()
  try {
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $realFile
    foreach($a in $argList){ [void]$psi.ArgumentList.Add($a) }
    $psi.WorkingDirectory = $cdTo
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $psi.StandardOutputEncoding = $enc
    $psi.StandardErrorEncoding = $enc
    $p = [System.Diagnostics.Process]::Start($psi)
    if($null -eq $p){ throw "Process Start returned null" }
    $outTask = $p.StandardOutput.ReadToEndAsync()
    $errTask = $p.StandardError.ReadToEndAsync()
    $finished = $p.WaitForExit($TimeoutSecPerCmd * 1000)
    if(-not $finished){ try { $p.Kill() } catch {} ; [void]$p.WaitForExit(5000) }
    $stdout = $outTask.Result; $stderr = $errTask.Result
    $ec = $p.ExitCode
    $combined = @(($stdout -split "`r?`n"); ($stderr -split "`r?`n"))
    foreach($l in $combined){ if(-not [string]::IsNullOrWhiteSpace($l)){ [void]$lines_out.Add($l) } }
  } catch {
    [void]$lines_out.Add(("EXCEPTION: {0}" -f $_.Exception.ToString()))
    $ec = 1337
  }
  $sw.Stop()
  W @(("EXIT_CODE: {0} ; ELAPSED_MS: {1}" -f $ec, $sw.ElapsedMilliseconds))
  if($lines_out.Count -gt 60){
    $skip = $lines_out.Count - 60
    W @(("... trimmed first {0} lines, showing last 60 ..." -f $skip))
    $tail = $lines_out.GetRange($skip, 60)
  } else {
    $tail = $lines_out
  }
  W $tail
  return [pscustomobject]@{ Name=$name; ExitCode=$ec; ElapsedMs=$sw.ElapsedMilliseconds }
}

$runs = @(
  @{ N='1_cargo_check_workspace_all_targets'; F='cargo'; A=@('check','--workspace','--all-targets','--message-format=short'); C=$Root },
  @{ N='2_cargo_clippy_Dwarnings_16platform'; F='cargo'; A=@('clippy','-p','graph-algorithms','-p','mox-expert','-p','primiflow-fusion','-p','runtime','-p','mox-system','-p','operator-core','-p','operator-wasm','-p','optimizer','-p','flow-ai','-p','hermes-flow-bridge','-p','business-catalog','-p','ai-agent','-p','template-market','-p','primiflow-core','-p','kg-hub','-p','mox-common-meta','--all-targets','--','-D','warnings'); C=$Root },
  @{ N='3_cargo_test_5core'; F='cargo'; A=@('test','-p','graph-algorithms','-p','mox-expert','-p','primiflow-fusion','-p','runtime','-p','mox-system','--lib','--tests','-q','--','--test-threads=4'); C=$Root },
  @{ N='4_reconcile_7x8'; F='node'; A=@('platform/services/graph-algorithms/scripts/reconcile_7x8.js'); C=$Root },
  @{ N='5_backend_node_mocha'; F='npm'; A=@('exec','--','mocha','test','--timeout','25000','--reporter','min'); C=(Join-Path $Root 'platform\backend-node') },
  @{ N='6_frontend_pnpm_build'; F='pnpm'; A=@('build'); C=(Join-Path $Root 'frontend-ui') }
)

$results = New-Object System.Collections.Generic.List[object]
foreach($r in $runs){
  $results.Add( (Run-Cmd -name $r.N -file $r.F -argList $r.A -cdTo $r.C) )
}

W @("", "=== Summary ===")
foreach($r in $results){
  W @(("  {0} -> EXIT={1} ms={2}" -f $r.Name, $r.ExitCode, $r.ElapsedMs))
}
W @("", ("Finished: {0}" -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss')))

Write-Host ("REPORT={0}" -f $report)
Write-Host "--- tail 60 ---"
Get-Content -LiteralPath $report -Tail 60
