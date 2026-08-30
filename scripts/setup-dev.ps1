#Requires -Version 5.1
<#
.SYNOPSIS
  璇玑（infotopograph）开发环境一键准备（B-2 / G-4 四件套之一）

.DESCRIPTION
  · 检测 Python / Node / Cargo 工具链是否就绪
  · 安装 Python 依赖（mox-server / mox-store / xiaobai_voice 的 requirements）
  · 安装前端 npm 依赖（frontend-ui）
  · 校验 Rust workspace 可构建（可选）
  · 生成 .env 占位（如缺）

.EXAMPLE
  .\scripts\setup-dev.ps1
  .\scripts\setup-dev.ps1 -SkipFrontend
#>
param(
    [switch]$SkipFrontend,
    [switch]$SkipRust,
    [switch]$NoInstall
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $Root

function Test-Cmd([string]$Name) {
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

Write-Host "=== 璇玑 开发环境准备 ===" -ForegroundColor Cyan
Write-Host "仓库根: $Root`n"

# 1) 工具链检测
$missing = @()
foreach ($c in @("python", "node", "npm")) {
    if (-not (Test-Cmd $c)) { $missing += $c }
}
if (-not $SkipRust) {
    if (-not (Test-Cmd "cargo")) { Write-Host "[WARN] 未检测到 cargo（Rust），跳过 Rust 校验" -ForegroundColor Yellow }
}
if ($missing.Count -gt 0) {
    $joined = $missing -join ", "
    Write-Host "[FAIL] 缺少工具链: $joined" -ForegroundColor Red
    exit 1
}
Write-Host "[OK] python / node / npm 已就绪" -ForegroundColor Green

# 2) Python 依赖（mox-server + mox-store + xiaobai_voice）
if ($NoInstall) {
    Write-Host "[SKIP] -NoInstall，跳过依赖安装" -ForegroundColor Yellow
} else {
    $reqFiles = @(
        "platform/mox-server/requirements.txt",
        "platform/mox-store/requirements.txt",
        "projects/xiaobai_voice/requirements.txt"
    )
    foreach ($rf in $reqFiles) {
        if (Test-Path $rf) {
            Write-Host "  [pip] $rf"
            & python -m pip install -q -r $rf
            if ($LASTEXITCODE -ne 0) { Write-Host "[WARN] $rf 安装非零退出（可能为可选依赖）" -ForegroundColor Yellow }
        } else {
            Write-Host "  [skip] $rf 不存在"
        }
    }
}

# 3) 前端依赖
if (-not $SkipFrontend -and -not $NoInstall) {
    if (Test-Path "frontend-ui/package.json") {
        Write-Host "  [npm] frontend-ui"
        Push-Location "frontend-ui"
        try {
            if (Test-Path "node_modules") {
                Write-Host "        node_modules 已存在，跳过安装"
            } else {
                & npm install --no-audit --no-fund
                if ($LASTEXITCODE -ne 0) { Write-Host "[WARN] npm install 非零退出" -ForegroundColor Yellow }
            }
        } finally { Pop-Location }
    }
} elseif ($SkipFrontend) {
    Write-Host "[SKIP] -SkipFrontend" -ForegroundColor Yellow
}

# 4) Rust workspace 校验
if (-not $SkipRust -and (Test-Cmd "cargo")) {
    Write-Host "  [cargo] 校验 workspace metadata（不做完整构建，省时）"
    & cargo metadata --format-version 1 --no-deps | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[WARN] cargo metadata 失败（workspace 配置可能有问题）" -ForegroundColor Yellow
    } else {
        Write-Host "[OK] Rust workspace 元数据解析通过" -ForegroundColor Green
    }
}

# 5) 环境变量占位
if (-not (Test-Path ".env")) {
    if (-not (Test-Path ".env.example")) {
        Write-Host "[INFO] 无 .env.example，跳过 .env 生成" -ForegroundColor DarkGray
    } else {
        Copy-Item ".env.example" ".env"
        Write-Host "[OK] 已从 .env.example 生成 .env（请按需填写密钥）" -ForegroundColor Green
    }
}

Write-Host "`n=== 准备完成 ===" -ForegroundColor Cyan
Write-Host "下一步:  ./scripts/check-all.ps1  （一键构建+测试+clippy）"
Write-Host "        ./scripts/start-all.ps1  （启动全部服务 + 管理面板）"
