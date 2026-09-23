#Requires -Version 5.1
<#
.SYNOPSIS
  璇玑 一键启动全部服务 + 管理面板（B-2 / G-4 四件套之一）

.DESCRIPTION
  复用 scripts/server-manage.py bootstrap：预检 → 清理残留 → 启动 auto_start 服务 →
  拉起 Web 管理面板。等价于 scripts/deploy/start.ps1 Start。

.EXAMPLE
  .\scripts\start-all.ps1
  .\scripts\start-all.ps1 -Services   # 同时启动项目服务（api/frontend）
#>
param(
    [switch]$Services,
    [switch]$NoBrowser,
    [switch]$Strict
)

$Root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $Root

Write-Host "=== 璇玑 一键启动 ===" -ForegroundColor Cyan
$args = @("scripts/server-manage.py", "bootstrap")
if ($Services) { $args += "--with-services" }
if ($NoBrowser) { $args += "--no-browser" }
if ($Strict) { $args += "--strict" }

& python @args
exit $LASTEXITCODE
