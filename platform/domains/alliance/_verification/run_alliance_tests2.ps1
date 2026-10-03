# 联盟域后端全量测试批跑（顺序执行；用 cmd 重定向避免 PS UTF-16 问题）
Set-Location D:\a10\aikjx\gitcode\infotopograph
$out = "platform\domains\alliance\_verification\fv_other_crates2.log"
"" | Out-File -FilePath $out -Encoding ascii

$crates = @(
  "mox-alliance-scheduler-core",
  "mox-alliance-executor-core",
  "mox-alliance-registry-core",
  "mox-alliance-scheduler-svc",
  "mox-alliance-executor-svc",
  "mox-alliance-registry-svc",
  "mox-alliance-mcp-server",
  "mox-alliance-http-sdk"
)

foreach ($c in $crates) {
  Add-Content -Path $out -Value "`n==== CARGO TEST -p $c ====" -Encoding ascii
  cmd /c "cargo test -p $c >> `"$out`" 2>&1"
  Add-Content -Path $out -Value "---- EXIT($c)=$LASTEXITCODE ----" -Encoding ascii
}
Write-Host "ALL DONE"
