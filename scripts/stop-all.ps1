#Requires -Version 5.1
<#
.SYNOPSIS
  璇玑 一键停止全部服务（B-2 / G-4 四件套之一）

.DESCRIPTION
  复用 scripts/manage.py stop：停止所有已注册服务进程并清理残留。

.EXAMPLE
  .\scripts\stop-all.ps1
  .\scripts\stop-all.ps1 -Force
#>
param(
    [switch]$Force
)

$Root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $Root

Write-Host "=== 璇玑 一键停止 ===" -ForegroundColor Cyan
$args = @("scripts/manage.py", "stop")
if ($Force) { $args += "--force" }

& python @args
exit $LASTEXITCODE
