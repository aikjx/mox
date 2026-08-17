# -*- coding: utf-8 -*-
# Melody2Score 一键打包脚本（Windows）
# 用法（在 melody2score/ 目录下用 PowerShell 运行）：
#   powershell -ExecutionPolicy Bypass -File build_exe.ps1
#
# 产物：dist/Melody2Score/  —— 一个可直接拷贝到任意 Windows 电脑的绿色文件夹，
#       双击里面的「启动Melody2Score.bat」即可运行，无需安装 Python 或任何依赖。
$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
Push-Location $root

Write-Host "=================================================="
Write-Host " Melody2Score 一键全维打包（PyInstaller + PyQt5）"
Write-Host "=================================================="

# 1) 确保打包依赖
Write-Host "[1/4] 检查打包依赖 (pyinstaller) ..."
if (-not (python -c "import pyinstaller" 2>$null)) {
    Write-Host "  安装 pyinstaller ..."
    python -m pip install -U pyinstaller
}
# 运行所需依赖（仅在当前环境装，目标电脑不需要）
python -m pip install -q -r requirements.txt pyqt5 2>&1 | Out-Null

# 2) 清理旧产物
Write-Host "[2/4] 清理旧构建 ..."
if (Test-Path build) { Remove-Item -Recurse -Force build }
if (Test-Path dist)  { Remove-Item -Recurse -Force dist }

# 3) 执行 PyInstaller（onedir 模式）
Write-Host "[3/4] 运行 PyInstaller（torch 较大，请耐心等待，约 5-15 分钟）..."
python -m PyInstaller build_exe.spec --noconfirm --clean
if ($LASTEXITCODE -ne 0) {
    Write-Error "PyInstaller 打包失败，请查看上方报错。"
    exit 1
}

# 4) 生成一键启动脚本 + 说明
$dist = Join-Path $root "dist\Melody2Score"
if (-not (Test-Path $dist)) {
    Write-Error "未找到产物目录 $dist"
    exit 1
}

$bat = @'
@echo off
chcp 65001 >nul
REM Melody2Score 一键启动（绿色版，无需安装 Python）
setlocal
cd /d "%~dp0"
REM 确保工作目录在分发根（app/Melody2Score.exe 的上级）
if exist "app\Melody2Score.exe" (
    start "" "app\Melody2Score.exe"
) else (
    echo 未找到 app\Melody2Score.exe，请确认解压完整。
    pause
)
'@
Set-Content -Path (Join-Path $dist "启动Melody2Score.bat") -Value $bat -Encoding ASCII

$readme = @'
# Melody2Score 绿色版（开箱即用）

本文件夹是一个**独立的桌面应用**，已把 Python 运行环境、全部依赖（PyTorch CPU / librosa / PyQt5 等）和内置样例音频全部打包进来。

## 在其他电脑运行
1. 把整个 `Melody2Score` 文件夹拷贝到目标 Windows 电脑（无需安装 Python）。
2. 双击 `启动Melody2Score.bat` 即可打开「哼唱旋律转谱」桌面程序。
   - 也可直接双击 `app\Melody2Score.exe`。

## 功能
- 选择音频文件（wav/mp3/flac/ogg/m4a）→ 实时转简谱 + 五线谱 + 音高轮廓。
- 内置 144 个经典旋律样例，一键识别。
- 麦克风实时录音识别（需目标电脑有麦克风及声卡驱动）。
- 一键保存 Markdown 报告到 `app\exports\`。

## 说明
- 首次启动稍慢（torch 在磁盘上解压载入），属正常现象。
- 文件夹不要拆分移动，`audio\` 与 `app\` 需保持相对结构。
- 若被杀软误报，请加入白名单（本程序不含任何网络/上传行为）。
'@
Set-Content -Path (Join-Path $dist "README.txt") -Value $readme -Encoding UTF8

Write-Host "[4/4] 完成！"
Write-Host "产物目录：$dist"
Write-Host "分发方式：把 $dist 整个文件夹压缩发给对方，解压后双击 启动Melody2Score.bat 即可。"
Pop-Location
