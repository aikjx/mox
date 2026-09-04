# =============================================================================
# 一键安装 mox_sys 归一化母版（PowerShell / Windows）
# 用法：  .\install.ps1 [-Server 127.0.0.1] [-Port 3306] [-User root] [-Password xxx]
# 说明：  单一权威 DDL 为 mox_sys-universal-template.sql（含全部 56 张表）。
#         本脚本只是把它灌入 MySQL，自动建库 mox_v3 并 USE。
# =============================================================================
param(
  [string]$Server = '127.0.0.1',
  [int]   $Port   = 3306,
  [string]$User   = 'root',
  [string]$Password = ''
)

$sql = Join-Path $PSScriptRoot 'mox_sys-universal-template.sql'
if (-not (Test-Path $sql)) { Write-Error "找不到 $sql"; exit 1 }

$cmd = "mysql -h$Server -P$Port -u$User --default-character-set=utf8mb4"
if ($Password -ne '') { $cmd += " -p$Password" }
$cmd += " < `"$sql`""

Write-Host ">> 执行: $cmd"
cmd /c $cmd
if ($LASTEXITCODE -eq 0) { Write-Host ">> mox_sys 归一化母版安装完成（库 mox_v3）" }
else { Write-Error ">> 安装失败，退出码 $LASTEXITCODE" ; exit $LASTEXITCODE }
