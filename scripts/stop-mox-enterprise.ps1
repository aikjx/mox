#Requires -Version 5.1
<#
.SYNOPSIS
  MOX 企业级五进程一键停止（与 start-mox-enterprise.ps1 配套）

.DESCRIPTION
  停止编排器 3001 / 知识库 8104 / 联盟调度 3100 / 执行 3200 / 网关 8080。
  不影响前端 3020 / primiflow 8000 / melody2score 8012 等独立运行服务。

.EXAMPLE
  .\scripts\stop-mox-enterprise.ps1
#>

$Ports = 3001, 8104, 3100, 3200, 8080
Write-Host "=== MOX 企业级五进程停止 ===" -ForegroundColor Cyan
foreach ($port in $Ports) {
    $c = Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue
    if ($c) {
        $procId = $c[0].OwningProcess
        Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
        Write-Host "  [$port] 已停止 pid=$procId" -ForegroundColor Green
    } else {
        Write-Host "  [$port] 未运行" -ForegroundColor DarkGray
    }
}
Start-Sleep -Seconds 2
Write-Host ""
Write-Host "剩余监听（应只剩 3020/8000/8012 等独立服务）："
foreach ($port in 3020, 8000, 8012) {
    $c = Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue
    if ($c) { Write-Host "  $port : UP" } else { Write-Host "  $port : down" }
}
