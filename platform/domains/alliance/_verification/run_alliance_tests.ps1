# 联盟域后端全量测试批跑（顺序执行，避免 cargo 锁竞争）
$ErrorActionPreference = "Continue"
Set-Location D:\a10\aikjx\gitcode\infotopograph
$out = "platform\domains\alliance\_verification\fv_other_crates.log"
"" | Out-File -FilePath $out -Encoding utf8

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
  Add-Content -Path $out -Value "`n==================== CARGO TEST -p $c ====================" -Encoding utf8
  cargo test -p $c *>> $out
  Add-Content -Path $out -Value "---- EXIT($c)=$LASTEXITCODE ----" -Encoding utf8
}
Write-Host "ALL DONE"
