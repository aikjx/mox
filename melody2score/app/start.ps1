# -*- coding: utf-8 -*-
# 启动企业级可视化转谱界面
# 用法：
#   powershell -ExecutionPolicy Bypass -File app/start.ps1          # 桌面 GUI（默认）
#   powershell -ExecutionPolicy Bypass -File app/start.ps1 -Mode web # 浏览器界面
$ErrorActionPreference = "Stop"
param(
    [string]$Mode = "gui"   # gui = 桌面窗口；web = 浏览器界面
)
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
if ($Mode -eq "web") {
    Write-Host "[melody2score] 启动 Web 界面 ..."
    Write-Host "[melody2score] 浏览器打开:  http://127.0.0.1:8012"
    python app/webui.py
} else {
    Write-Host "[melody2score] 启动桌面 GUI ..."
    python app/gui.py
}
