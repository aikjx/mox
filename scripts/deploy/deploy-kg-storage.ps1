# ================================================================
# 璇玑 KG Storage 快速部署脚本
# 模块: mox-kg-storage-svc (rust-rocksdb 优化版)
# 用法: .\deploy-kg-storage.ps1 [-Mode release|release-fast|dev] [-Features persist-rocksdb]
# ================================================================

param(
    [ValidateSet("release", "release-fast", "dev")]
    [string]$Mode = "release-fast",
    
    [switch]$Features = $true,
    
    [switch]$RunTests = $false,
    
    [string]$DataPath = ".\data\kg-storage",
    
    [int]$BlockCacheMB = 512,
    
    [string]$Shards = "0,1,2,3"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = "D:\a10\aikjx\gitcode\infotopograph"
$CrateName = "mox-kg-storage-svc"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host " 璇玑 KG Storage 快速部署" -ForegroundColor Cyan
Write-Host " 模块: $CrateName" -ForegroundColor Cyan
Write-Host " 模式: $Mode" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# Step 1: 环境检查
Write-Host "`n[1/5] 环境检查..." -ForegroundColor Yellow
$rustc = rustc --version 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Error "Rust未安装，请先安装: https://rustup.rs"
    exit 1
}
Write-Host "  Rust: $rustc" -ForegroundColor Green

$cargo = cargo --version 2>&1
Write-Host "  Cargo: $cargo" -ForegroundColor Green

# 检查libclang (rocksdb编译需要)
if ($Features) {
    $clang = clang --version 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  警告: 未检测到clang，rocksdb编译可能失败" -ForegroundColor Red
        Write-Host "  Windows安装: winget install LLVM.LLVM" -ForegroundColor Yellow
    } else {
        Write-Host "  Clang: $($clang.Split([Environment]::NewLine)[0])" -ForegroundColor Green
    }
}

# Step 2: 创建数据目录
Write-Host "`n[2/5] 创建数据目录..." -ForegroundColor Yellow
if (-not (Test-Path $DataPath)) {
    New-Item -ItemType Directory -Path $DataPath -Force | Out-Null
    Write-Host "  创建: $DataPath" -ForegroundColor Green
} else {
    Write-Host "  已存在: $DataPath" -ForegroundColor Green
}

# Step 3: 编译
Write-Host "`n[3/5] 编译 ($Mode)..." -ForegroundColor Yellow
Set-Location $ProjectRoot

$buildArgs = @("build", "-p", $CrateName)
if ($Mode -eq "release") {
    $buildArgs += "--release"
} elseif ($Mode -eq "release-fast") {
    $buildArgs += "--profile"
    $buildArgs += "release-fast"
}
if ($Features) {
    $buildArgs += "--features"
    $buildArgs += "persist-rocksdb"
}

Write-Host "  命令: cargo $($buildArgs -join ' ')" -ForegroundColor Gray
$buildStart = Get-Date
cargo @buildArgs
if ($LASTEXITCODE -ne 0) {
    Write-Error "编译失败！"
    exit 1
}
$buildDuration = (Get-Date) - $buildStart
Write-Host "  编译成功，耗时: $($buildDuration.TotalSeconds.ToString('F1'))s" -ForegroundColor Green

# Step 4: 测试 (可选)
if ($RunTests) {
    Write-Host "`n[4/5] 运行测试..." -ForegroundColor Yellow
    $testArgs = @("test", "-p", $CrateName)
    if ($Features) {
        $testArgs += "--features"
        $testArgs += "persist-rocksdb"
    }
    cargo @testArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Error "测试失败！"
        exit 1
    }
    Write-Host "  测试通过" -ForegroundColor Green
} else {
    Write-Host "`n[4/5] 跳过测试 (使用 -RunTests 启用)" -ForegroundColor Gray
}

# Step 5: 输出部署信息
Write-Host "`n[5/5] 部署信息" -ForegroundColor Yellow
$targetDir = if ($Mode -eq "dev") { "debug" } else { $Mode }
$binaryPath = Join-Path $ProjectRoot "target\$targetDir\$CrateName.exe"

if (Test-Path $binaryPath) {
    $fileSize = (Get-Item $binaryPath).Length / 1MB
    Write-Host "  二进制: $binaryPath" -ForegroundColor Green
    Write-Host "  大小: $($fileSize.ToString('F1')) MB" -ForegroundColor Green
} else {
    Write-Host "  二进制未找到 (可能是lib类型crate)" -ForegroundColor Yellow
}

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host " 部署完成！" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan

Write-Host "`n运行环境变量:" -ForegroundColor White
Write-Host "  MOX_ROCKSDB_BLOCK_CACHE_MB=$BlockCacheMB  (Block cache大小)" -ForegroundColor Gray
Write-Host "  MOX_ROCKSDB_PATH=$DataPath  (数据存储路径)" -ForegroundColor Gray
Write-Host "  MOX_ROCKSDB_SHARDS=$Shards  (分片ID)" -ForegroundColor Gray

Write-Host "`n性能优化已启用:" -ForegroundColor White
Write-Host "  [x] Release LTO fat + codegen-units=1" -ForegroundColor Green
Write-Host "  [x] 512MB Block Cache + 索引/过滤器缓存" -ForegroundColor Green
Write-Host "  [x] Bloom Filter (10位/key)" -ForegroundColor Green
Write-Host "  [x] 两级索引 (TwoLevelIndexSearch)" -ForegroundColor Green
Write-Host "  [x] 分层压缩 (L0/L1无压缩, L2-L5 LZ4, L6 Zstd)" -ForegroundColor Green
Write-Host "  [x] Prefix Extractor (8字节固定前缀)" -ForegroundColor Green
Write-Host "  [x] CF Handle 缓存 (消除重复查找)" -ForegroundColor Green
Write-Host "  [x] WriteOptions 全局复用" -ForegroundColor Green
Write-Host "  [x] MultiGet 批量查询 API" -ForegroundColor Green
Write-Host "  [x] seek_prefix 优化 (iterate_upper_bound + prefix_same_as_start)" -ForegroundColor Green
Write-Host "  [x] scan_cf 预读优化 (256KB)" -ForegroundColor Green
Write-Host "  [x] 自动并行Compaction (按CPU核心数)" -ForegroundColor Green

Write-Host "`n详细优化报告: docs\architecture\ROCKSDB-PERFORMANCE-OPTIMIZATION.md" -ForegroundColor Cyan
