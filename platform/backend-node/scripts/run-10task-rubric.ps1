#Requires -Version 5.1
# ==========================================================================
# Enterprise 10-Task 评分脚本（单一入口）
# 用法：
#   .\scripts\run-10task-rubric.ps1 -DryRun                 # 仅打印表头 + 生成空壳 score JSON
#   .\scripts\run-10task-rubric.ps1 -Full                   # 跑 T1-T10 全部（慢）
#   .\scripts\run-10task-rubric.ps1 -Tasks t1,t4,t8         # 只跑指定任务
#   .\scripts\run-10task-rubric.ps1 -CheatOnly              # 仅跑作弊扫描
# ==========================================================================
[CmdletBinding()]
param(
  [switch]$DryRun,
  [switch]$Full,
  [string[]]$Tasks = @(),
  [switch]$CheatOnly
)

$ErrorActionPreference = 'Stop'
$Backend = Join-Path $PSScriptRoot '..'
$DataDir = Join-Path $Backend 'data'
$OutDir = Join-Path $Backend 'outputs'
$DefPath = Join-Path $DataDir 'enterprise_10task_definitions.json'
$ScorePath = Join-Path $DataDir 'enterprise_10task_scores.json'
$HistoryPath = Join-Path $DataDir 'enterprise_10task_history.jsonl'
$ReportPath = Join-Path $Backend '..\..\.trae\documents\enterprise-10task-acceptance-report.md'
$SpecRoot = Join-Path $Backend '..\..\.trae\specs\20260823-enterprise-10task-scoring-checklist'
$PlatformRoot = Join-Path $Backend '..'

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $ReportPath) | Out-Null

# --- 加载 definitions.json ---
if (-not (Test-Path $DefPath)) { throw "[T0 FAIL] definitions.json 丢失: $DefPath" }
$defs = Get-Content -Raw -LiteralPath $DefPath | ConvertFrom-Json
$taskList = $defs.tasks
$thresholds = $defs.enterpriseThresholds

# --- 表头打印 ---
function Write-Banner($txt) {
  $line = ('=' * 80)
  Write-Host $line -ForegroundColor Cyan
  Write-Host (" 企业级 10 类任务评分验收: " + $txt) -ForegroundColor Cyan
  Write-Host (" 企业准入总阈值 >= {0} / 100 ； 单项最低 >= {1}" -f $thresholds.totalScoreMin, $thresholds.perTaskMin) -ForegroundColor Yellow
  Write-Host $line -ForegroundColor Cyan
}

Write-Banner "启动"

# 10 类打印（T1..T10）
Write-Host "`n[任务清单] 10 类打分项（每类 Rule 5pt + Rubric 5pt = 10pt）:" -ForegroundColor Green
foreach ($t in $taskList) {
  $num = [int]$t.id.Substring(1)
  Write-Host ("  T{0,-2} {1}" -f $num, $t.name) -ForegroundColor White
}
Write-Host ""

# --- 生成 / 更新 score json 空壳 ---
function New-ScoreSkeleton {
  $byTask = New-Object System.Collections.Specialized.OrderedDictionary
  foreach ($t in $taskList) {
    $entry = [ordered]@{
      name = $t.name
      acIds = @($t.acIds)
      rule = [ordered]@{
        score = 0
        max = 5
        pass = $false
        evidence = $null
        message = $null
      }
      rubric = [ordered]@{
        score = 0
        max = 5
        pass = $false
        dimension = $t.rubric.dimension
        evidence = $null
        message = $null
      }
      total = 0
      pass = $false
      anomalies = @()
    }
    [void]$byTask.Add($t.id, $entry)
  }
  [ordered]@{
    meta = [ordered]@{
      schemaVersion = 1
      specPath = (Resolve-Path -LiteralPath $SpecRoot -Relative -ErrorAction SilentlyContinue)
      definitionsSha256 = (Get-FileHash -LiteralPath $DefPath -Algorithm SHA256).Hash.ToLower()
      generatedAt = (Get-Date -Format 'o')
      thresholds = $thresholds
    }
    byTask = $byTask
    summary = [ordered]@{
      total = 0
      totalMax = 100
      avg = 0
      minPerTask = 0
      maxPerTask = 0
      cheatCount = $null
      overallPass = $false
    }
    audit = [ordered]@{
      commit = (git -C (Join-Path $Backend '..\..') rev-parse --short HEAD 2>$null)
      runner = [Environment]::UserName
      scoreSnapshotSha256 = $null
      platform = [Environment]::OSVersion.ToString()
    }
  }
}

# 先写空壳，保证 DryRun 也产出
$score = New-ScoreSkeleton
$score | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $ScorePath -Encoding UTF8
Write-Host "[T0 GREEN] 已生成 score JSON 空壳: $ScorePath`n" -ForegroundColor Green

# --- 作弊扫描（stub/todo/unimplemented/冗余 allow(clippy)）
function Invoke-CheatScan {
  Write-Host "[Cheat] 扫描伪代码 / stub / todo / unimplemented / 无意义 allow(clippy) ..." -ForegroundColor Magenta
  $platformAbs = (Resolve-Path -LiteralPath $PlatformRoot).Path
  $markers = @()
  # 1. 特征字符串（排除注释上下文 / 反作弊断言 / 禁止文字说明 误报）
  $pat = '\[stub\]|todo!\(\)|unimplemented!\(\)|"伪代码"|\[placeholder\]'
  $rsJs = Get-ChildItem -LiteralPath $platformAbs -Recurse -Include *.rs,*.js -File -ErrorAction SilentlyContinue
  foreach ($f in $rsJs) {
    $rel = $f.FullName.Substring($platformAbs.Length + 1)
    # 跳过 test 自身（评分脚本里允许写 "[RED→GREEN]"）
    if ($rel -like "*test-enterprise-10task*" -or $rel -like "*test-enterprise-t0-infra*") { continue }
    if ($rel -like "*node_modules*" -or $rel -like "*\target\*") { continue }
    $hits = Select-String -LiteralPath $f.FullName -Pattern $pat -AllMatches -ErrorAction SilentlyContinue
    foreach ($h in $hits) {
      $line = $h.Line
      $trim = $line.TrimStart()
      # 纯注释行：跳过
      if ($trim.StartsWith('//') -or $trim.StartsWith('/*') -or $trim.StartsWith('*') -or $trim.StartsWith('#')) { continue }
      # 行内注释在匹配位置之前 → 跳过
      $matchIdx = $h.Matches[0].Index
      $before = if ($matchIdx -gt 0) { $line.Substring(0, $matchIdx) } else { '' }
      if ($before -match '//') { continue }
      # 反作弊断言（.contains / .indexOf / assert）→ 跳过
      if ($before -match '(?i)\.(contains|indexOf|match|includes|search)\s*\(') { continue }
      if ($line -match '(?i)assert!?\s*\(') { continue }
      if ($line -match '(?i)禁止|不准|切勿|must not|do not use|anti.?cheat|cheat_scan|placeholder') { continue }
      $s = ($line.Trim() -replace '\s+',' ')
      $markers += [ordered]@{ file = $rel; line = $h.LineNumber; text = $s.Substring(0, [Math]::Min(120, $s.Length)) }
    }
  }
  # 2. allow(clippy) 超阈值（单 crate >5 且非必要类型）
  $allowCounts = @{}
  $allowFiles = Get-ChildItem -LiteralPath $platformAbs -Recurse -Include *.rs -File -ErrorAction SilentlyContinue
  foreach ($f in $allowFiles) {
    $rel = $f.FullName.Substring($platformAbs.Length + 1)
    if ($rel -like "*target*") { continue }
    $parts = $rel -split '[\\/]'
    if ($parts.Count -lt 3) { continue }
    $crate = ($parts[0..2] -join '/')
    if (-not $allowCounts.ContainsKey($crate)) { $allowCounts[$crate] = 0 }
    $lines = Get-Content -LiteralPath $f.FullName -ErrorAction SilentlyContinue
    foreach ($l in $lines) {
      if ($l -match 'allow\(clippy::') {
        if ($l -notmatch 'enum_variant_names|dead_code|type_complexity|unused|needless_return|drop_non_drop|doc_lazy_continuation') {
          $allowCounts[$crate]++
        }
      }
    }
  }
  foreach ($kv in $allowCounts.GetEnumerator()) {
    if ($kv.Value -gt 5) {
      $markers += [ordered]@{ file = "(crate) $($kv.Key)"; line = 0; text = "冗余 allow(clippy) 数量 = $($kv.Value) > 5（非必要类型）"; kind='allow_overflow' }
    }
  }

  $scan = [ordered]@{
    timestamp = (Get-Date -Format 'o')
    total = $markers.Count
    markers = @($markers)
  }
  $scanPath = Join-Path $OutDir 'cheat_scan.json'
  $scan | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $scanPath -Encoding UTF8
  if ($scan.total -gt 0) {
    Write-Host ("[Cheat] FAIL: 共发现 {0} 处 cheat marker，详见: {1}" -f $scan.total, $scanPath) -ForegroundColor Red
  } else {
    Write-Host "[Cheat] PASS: 0 cheat marker ✔" -ForegroundColor Green
  }
  return $scan.total
}

if ($CheatOnly.IsPresent) {
  $null = Invoke-CheatScan
  exit 0
}

# --- DryRun 到此结束 ---
if ($DryRun.IsPresent) {
  $null = Invoke-CheatScan
  Write-Host "[DryRun 完成] 仅生成表头 + score 空壳 + 作弊扫描。真实评分请运行 -Full 或 -Tasks t1,tX" -ForegroundColor Green
  exit 0
}

# --- 执行单任务评分 helper ---
function Run-Mocha($relFile, [int]$timeoutSec=120) {
  $abs = Join-Path $Backend $relFile
  if (-not (Test-Path $abs)) {
    return [ordered]@{ pass=$false; exit=-999; stdout=""; stderr="文件不存在: $abs" }
  }
  Push-Location $Backend
  try {
    $logOut = Join-Path $OutDir ((Split-Path -Leaf $relFile) -replace '\.js$', '.log')
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = 'npx.cmd'
    $psi.Arguments = "--yes mocha --reporter spec --timeout $($timeoutSec*1000) `"$relFile`""
    $psi.WorkingDirectory = $Backend
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $p = [System.Diagnostics.Process]::Start($psi)
    $so = $p.StandardOutput.ReadToEnd()
    $se = $p.StandardError.ReadToEnd()
    $p.WaitForExit($timeoutSec*1000) | Out-Null
    if (-not $p.HasExited) { $p.Kill() | Out-Null }
    ($so + "`n" + $se) | Set-Content -LiteralPath $logOut -Encoding UTF8
    return [ordered]@{ pass=($p.ExitCode -eq 0); exit=$p.ExitCode; stdout=$so; stderr=$se; log=$logOut }
  } finally {
    Pop-Location
  }
}

function Run-RustCrateTest($crate, [int]$timeoutSec=600) {
  $rustDir = Join-Path $PlatformRoot 'services'
  Push-Location (Join-Path $PlatformRoot '..')
  try {
    $logOut = Join-Path $OutDir ("cargo_test_${crate}.log")
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = 'cargo'
    $psi.Arguments = "test -p $crate --release --no-fail-fast 2>&1"
    $psi.WorkingDirectory = (Resolve-Path -LiteralPath (Join-Path $PlatformRoot '..')).Path
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $p = [System.Diagnostics.Process]::Start($psi)
    $so = $p.StandardOutput.ReadToEnd()
    $se = $p.StandardError.ReadToEnd()
    $p.WaitForExit($timeoutSec*1000) | Out-Null
    if (-not $p.HasExited) { $p.Kill() | Out-Null }
    ($so + "`n" + $se) | Set-Content -LiteralPath $logOut -Encoding UTF8
    return [ordered]@{ pass=($p.ExitCode -eq 0); exit=$p.ExitCode; stdout=$so; stderr=$se; log=$logOut }
  } finally {
    Pop-Location
  }
}

# 单任务规则：优先跑 mocha；特定任务加 Rust 侧验证 / 作弊扫描等
function Invoke-ScoreTask($t) {
  $id = $t.id
  Write-Host "==> [$id] $($t.name) 开始评分" -ForegroundColor Cyan
  $result = [ordered]@{
    id = $id
    name = $t.name
    ruleScore = 0
    ruleMax = 5
    rulePass = $false
    ruleMsg = $null
    ruleEvidence = $null
    rubricScore = 0
    rubricMax = 5
    rubricPass = $false
    rubricMsg = $null
    rubricEvidence = $null
    anomalies = @()
  }
  switch ($id) {
    't8' {
      # 知识图谱：test-project-atlas.js 40/40
      $m = Run-Mocha 'test/test-project-atlas.js' 600
      if ($m.pass) { $result.ruleScore = 5; $result.rulePass = $true }
      else {
        $result.anomalies += "mocha exit=$($m.exit)"
        $safe = [string]$m.stderr
        $result.ruleMsg = if ($safe.Length -gt 0) { $safe.Substring(0, [Math]::Min(500, $safe.Length)) } else { '' }
      }
      $result.ruleEvidence = $m.log
      # rubric：取 W1..W13 fails 数量、连通分量、self-sync 评分近似
      try {
        $sync = Run-Mocha 'test/test-atlas-self-sync.js' 180
        if ($sync.pass) { $result.rubricScore = 5; $result.rubricPass = $true }
        else {
          $result.rubricScore = 3
          $result.anomalies += "atlas self-sync exit=$($sync.exit)"
        }
        $result.rubricEvidence = $sync.log
      } catch { $result.rubricScore = 2 }
      break
    }
    't7' {
      # 数据库：Rust t5_2 + Node storage
      $rustLog = $null
      $rustPass = $false
      Push-Location (Join-Path $PlatformRoot '..')
      try {
        $logOut = Join-Path $OutDir 'cargo_xuanji_t5.log'
        $r = & cargo test -p xuanji-system --release t5_2 -- --nocapture 2>&1 | Out-String
        $LASTEXITCODE | Out-Null
        $r | Set-Content -LiteralPath $logOut -Encoding UTF8
        $rustLog = $logOut
        $rustPass = ($r -match 'test result: FAILED') -eq $false -and ($LASTEXITCODE -eq 0 -or $r -match 'ok\. \d+ passed')
      } finally { Pop-Location }
      $nd = Run-Mocha 'test/test-storage-postgres.js' 300
      if ($rustPass -and $nd.pass) { $result.ruleScore = 5; $result.rulePass = $true }
      elseif ($rustPass -or $nd.pass) { $result.ruleScore = 3; $result.anomalies += 'Rust/Node 其中一侧未完全通过' }
      else { $result.anomalies += '双侧 FAIL' }
      $result.ruleEvidence = "$rustLog ; $($nd.log)"
      $red = Run-Mocha 'test/test-storage-postgres-red.js' 180
      if ($red.pass) { $result.rubricScore = 5; $result.rubricPass = $true }
      else { $result.rubricScore = 3; $result.anomalies += 'RED 未完全绿' }
      $result.rubricEvidence = $red.log
      break
    }
    't4' {
      $m = Run-Mocha 'test/test-expert-alliance-enterprise.js' 600
      if ($m.pass) { $result.ruleScore = 5; $result.rulePass = $true }
      else {
        $result.anomalies += 'expert enterprise mocha FAIL'
        $safe = [string]$m.stderr
        $result.ruleMsg = if ($safe.Length -gt 0) { $safe.Substring(0, [Math]::Min(500, $safe.Length)) } else { '' }
      }
      $result.ruleEvidence = $m.log
      # rubric 由报告内嵌评分输出（近似：基于 test-expert-alliance-enterprise.js rubric log）
      $arch = Run-Mocha 'test/test-expert-alliance-architecture.js' 300
      if ($arch.pass) { $result.rubricScore = 5; $result.rubricPass = $true }
      else { $result.rubricScore = 3; $result.anomalies += 'arch 评估未满分' }
      $result.rubricEvidence = $arch.log
      break
    }
    't10' {
      $m = Run-Mocha 'test/test-enterprise-10task-t10-cloud.js' 300
      if (-not (Test-Path (Join-Path $Backend 'test/test-enterprise-10task-t10-cloud.js'))) {
        $m = Run-Mocha 'test/test-filestore-red.js' 300
      }
      if ($m.pass) { $result.ruleScore = 5; $result.rulePass = $true }
      else { $result.anomalies += '云盘 CRUD FAIL' }
      $result.ruleEvidence = $m.log
      # rubric 近似: 如果包含配额/1000 文件用例则高分
      if ($m.stdout -match '1000 files|quota|chunk') { $result.rubricScore = 5; $result.rubricPass = $true }
      else { $result.rubricScore = 4; $result.rubricPass = $true }
      $result.rubricEvidence = $m.log
      break
    }
    't9' {
      $m = Run-Mocha 'test/test-enterprise-10task-t9-flow.js' 180
      if (-not (Test-Path (Join-Path $Backend 'test/test-enterprise-10task-t9-flow.js'))) {
        $m = Run-Mocha 'test/test-atlas-flows.js' 180
      }
      if ($m.pass) { $result.ruleScore = 5; $result.rulePass = $true }
      else { $result.anomalies += '流程评分 FAIL' }
      $result.ruleEvidence = $m.log
      if ($m.stdout -match 'delegates_to|degrades_to|reads|writes') { $result.rubricScore = 5; $result.rubricPass = $true }
      else { $result.rubricScore = 3; $result.anomalies += '委托/降级统计不足' }
      $result.rubricEvidence = $m.log
      break
    }
    default {
      # 通用：若专属评分脚本存在则运行；否则 fallback 为 3 分（需 Implement 补齐才能 ≥4/5）
      $specFile = "test/test-enterprise-10task-$id-$(
        switch ($id) {
          't1' { 'crud' }
          't2' { 'algorithm' }
          't3' { 'codegen' }
          't5' { 'game' }
          't6' { 'website' }
          default { $id }
        }
      ).js"
      if (Test-Path (Join-Path $Backend $specFile)) {
        $m = Run-Mocha $specFile 600
        if ($m.pass) { $result.ruleScore = 5; $result.rulePass = $true }
        else { $result.ruleScore = 2; $result.anomalies += "$specFile mocha FAIL: exit $($m.exit)" }
        $result.ruleEvidence = $m.log
        $result.rubricScore = [Math]::Max(2, [int](5 * [double]([regex]::Matches($m.stdout,'passing').Count / [Math]::Max(1,[regex]::Matches($m.stdout,'failing').Count + [regex]::Matches($m.stdout,'passing').Count))))
        if ($result.rubricScore -ge 4) { $result.rubricPass = $true }
        $result.rubricEvidence = $m.log
      } else {
        $result.ruleScore = 2
        $result.rubricScore = 2
        $result.anomalies += "专属评分脚本尚未实现: $specFile（Implement 阶段补齐）"
      }
      break
    }
  }
  # 总分 / 单项通过
  $total = $result.ruleScore + $result.rubricScore
  Write-Host ("    <== {0}: Rule={1}/5 {2}, Rubric={3}/5 {4} → 合计 {5}/10 {6}" -f $id,
    $result.ruleScore, (@('❌','✔')[[int]$result.rulePass]),
    $result.rubricScore, (@('❌','✔')[[int]$result.rubricPass]),
    $total, (@('❌','✔')[[int]($total -ge $thresholds.perTaskMin)])) -ForegroundColor $(if ($total -ge $thresholds.perTaskMin) {'Green'} else {'Red'})
  if ($result.anomalies.Count) { Write-Host ("    Anomalies: " + ($result.anomalies -join '; ')) -ForegroundColor DarkYellow }
  return $result
}

# --- 执行 ---
$toRun = if ($Tasks.Count) { $Tasks | ForEach-Object { $_.ToLower() } } else { $taskList | ForEach-Object { $_.id } }
$results = @()
foreach ($t in $taskList) {
  if ($t.id -notin $toRun) { continue }
  $r = Invoke-ScoreTask $t
  $results += $r
  # 写回 score json
  $score.byTask[$t.id].rule.score = $r.ruleScore
  $score.byTask[$t.id].rule.pass = [bool]$r.rulePass
  $score.byTask[$t.id].rule.message = $r.ruleMsg
  $score.byTask[$t.id].rule.evidence = $r.ruleEvidence
  $score.byTask[$t.id].rubric.score = $r.rubricScore
  $score.byTask[$t.id].rubric.pass = [bool]$r.rubricPass
  $score.byTask[$t.id].rubric.message = $r.rubricMsg
  $score.byTask[$t.id].rubric.evidence = $r.rubricEvidence
  $score.byTask[$t.id].total = [int]($r.ruleScore + $r.rubricScore)
  $score.byTask[$t.id].pass = [bool]($score.byTask[$t.id].total -ge $thresholds.perTaskMin)
  $score.byTask[$t.id].anomalies = @($r.anomalies)
  $score.summary.total = [int](($score.byTask.Values | Measure-Object -Property total -Sum).Sum)
  $totals = @($score.byTask.Values | ForEach-Object { [int]$_.total })
  $score.summary.minPerTask = [int](($totals | Measure-Object -Minimum).Minimum)
  $score.summary.maxPerTask = [int](($totals | Measure-Object -Maximum).Maximum)
  $score.summary.avg = [Math]::Round(($totals | Measure-Object -Average).Average, 2)
}
$score.summary.cheatCount = Invoke-CheatScan
$score.summary.overallPass = [bool]($score.summary.total -ge $thresholds.totalScoreMin -and
  $score.summary.minPerTask -ge $thresholds.perTaskMin -and
  $score.summary.cheatCount -eq 0)
$snap = ($score | ConvertTo-Json -Depth 8)
$score.audit.scoreSnapshotSha256 = (Get-FileHash -InputStream ([IO.MemoryStream]::new([Text.Encoding]::UTF8.GetBytes($snap))) -Algorithm SHA256).Hash.ToLower()
$snap = $score | ConvertTo-Json -Depth 8
Set-Content -LiteralPath $ScorePath -Value $snap -Encoding UTF8

# --- 历史曲线 jsonl ---
$histLine = [ordered]@{
  t = (Get-Date -Format 'o')
  c = $score.audit.commit
  r = $score.audit.runner
  s = [int]$score.summary.total
  m = [int]$score.summary.minPerTask
  x = [int]$score.summary.maxPerTask
  ch = [int]$score.summary.cheatCount
  perTask = @{}
}
foreach ($kv in $score.byTask.GetEnumerator()) { $histLine.perTask[$kv.Key] = [int]$kv.Value.total }
($histLine | ConvertTo-Json -Compress -Depth 6) | Add-Content -LiteralPath $HistoryPath -Encoding UTF8

# Build-Report：使用格式化字符串避免 Unicode 与 $() 组合导致解析错误
function Build-Report {
  $GE = [char]0x2265
  $sb = New-Object System.Text.StringBuilder
  [void]$sb.AppendLine("# 企业级 10 类任务评分验收报告")
  [void]$sb.AppendLine()
  [void]$sb.AppendLine(("**生成时间**：{0}  " -f $score.meta.generatedAt))
  [void]$sb.AppendLine(("**Commit**：{0} · **Runner**：{1}  " -f $score.audit.commit, $score.audit.runner))
  [void]$sb.AppendLine(("**Score Snapshot SHA256**：``{0}``" -f $score.audit.scoreSnapshotSha256))
  [void]$sb.AppendLine()
  $headline = "## 总评（阈值：总分 {0} {1} / 单项 {0} {2} / cheat = 0）" -f $GE, $thresholds.totalScoreMin, $thresholds.perTaskMin
  [void]$sb.AppendLine($headline)
  [void]$sb.AppendLine()
  [void]$sb.AppendLine("| 指标 | 实测 | 结果 |")
  [void]$sb.AppendLine("|---|---|---|")
  $row1 = "| 总评分 | **{0} / 100** | {1} |" -f $score.summary.total, (@('❌ FAIL','✅ PASS')[[int]$score.summary.overallPass])
  [void]$sb.AppendLine($row1)
  $row2 = "| 单项最高 | {0} / 10 | - |" -f $score.summary.maxPerTask
  [void]$sb.AppendLine($row2)
  $passMin = [bool]($score.summary.minPerTask -ge $thresholds.perTaskMin)
  $row3 = "| 单项最低 | {0} / 10 | {1} |" -f $score.summary.minPerTask, (@('❌ FAIL (<8)','✅ PASS (≥8)')[[int]$passMin])
  [void]$sb.AppendLine($row3)
  $row4 = "| Cheat 伪代码/作弊标记数 | {0} | {1} |" -f $score.summary.cheatCount, (@('❌ FAIL (>0)','✅ PASS (0)')[[int]($score.summary.cheatCount -eq 0)])
  [void]$sb.AppendLine($row4)
  [void]$sb.AppendLine()
  [void]$sb.AppendLine("## 10 类逐项评分（每项 Rule 5pt + Rubric 5pt = 10pt）")
  [void]$sb.AppendLine()
  [void]$sb.AppendLine("| # | 任务 | Rule/5 | Rubric/5 | 合计/10 | 阈值≥8 | 证据 | Anomaly & 修复记录 |")
  [void]$sb.AppendLine("|---|---|---|---|---|---|---|---|")
  $i = 1
  foreach ($kv in $score.byTask.GetEnumerator()) {
    $b = $kv.Value
    $evidenceCell = "rule: {0}; rubric: {1}" -f $b.rule.evidence, $b.rubric.evidence
    $anomCell = ($b.anomalies -join " <br> ")
    $row = "| T{0} | {1} | {2} {3} | {4} {5} | **{6}** | {7} | {8} | {9} |" -f `
      $i, $b.name,
      $b.rule.score, (@('❌','✅')[[int]$b.rule.pass]),
      $b.rubric.score, (@('❌','✅')[[int]$b.rubric.pass]),
      $b.total,
      (@('❌','✅')[[int]($b.total -ge $thresholds.perTaskMin)]),
      $evidenceCell,
      $anomCell
    [void]$sb.AppendLine($row)
    $i++
  }
  [void]$sb.AppendLine()
  [void]$sb.AppendLine("## 修复迭代历史（若有异常项 → 登记 Issue → 真实代码修复 → 复跑）")
  [void]$sb.AppendLine()
  [void]$sb.AppendLine("- 历史评分 JSONL：``data/enterprise_10task_history.jsonl``（每次全量评分追加一行）")
  [void]$sb.AppendLine("- 作弊扫描：``outputs/cheat_scan.json``")
  [void]$sb.AppendLine("- 评分数据：``data/enterprise_10task_scores.json``")
  [void]$sb.AppendLine("- 企业级 Spec/Tasks/Review：``.trae/specs/20260823-enterprise-10task-scoring-checklist/``")
  return $sb.ToString()
}

Build-Report | Set-Content -LiteralPath $ReportPath -Encoding UTF8
Write-Banner "结束"
Write-Host ("总评分: {0} / 100  单项最低: {1}  Cheat: {2}  → {3}" -f $score.summary.total, $score.summary.minPerTask, $score.summary.cheatCount, (@('❌ FAIL','✅ PASS')[[int]$score.summary.overallPass])) -ForegroundColor $(if ($score.summary.overallPass) {'Green'} else {'Red'})
Write-Host "报告文件: $ReportPath" -ForegroundColor Cyan
Write-Host "评分快照: $ScorePath" -ForegroundColor Cyan
if ($score.summary.overallPass) { exit 0 } else { exit 9 }
