# API 层归一化审计脚本
# 用法（在 frontend-ui 目录或任意位置执行）：
#   powershell -ExecutionPolicy Bypass -File scripts/api-duplicate-check.ps1
#
# 检查项：
#   1) 文件内重复 export const（ESM 解析期 SyntaxError，如历史 getMenuTree 问题）
#   2) 跨文件重名导出（index.js `export *` 静默歧义风险）
#   3) node --check 全量语法校验（.js）
# 任一失败以非零码退出，便于接入 CI。

param(
    [string]$ApiDir = (Join-Path $PSScriptRoot '..\src\api')
)

$ErrorActionPreference = 'Stop'
$failed = $false

if (-not (Test-Path $ApiDir)) {
    Write-Error "API 目录不存在: $ApiDir"
    exit 2
}

$files = @((Get-ChildItem -Path (Join-Path $ApiDir '*.js') -File) + (Get-ChildItem -Path (Join-Path $ApiDir '*.ts') -File) | Sort-Object Name)
Write-Host "== 审计目录: $ApiDir（$($files.Count) 个模块）==" -ForegroundColor Cyan

# 1) + 2) 导出名收集（同时覆盖 export const / export function / export async function）
$all = @{}   # name -> "file1,file2"
foreach ($f in $files) {
    $content = Get-Content -Path $f.FullName -Raw
    $matches = [regex]::Matches($content, 'export\s+const\s+([A-Za-z0-9_$]+)|export\s+async\s+function\s+([A-Za-z0-9_$]+)|export\s+function\s+([A-Za-z0-9_$]+)')
    $inFile = @{}
    foreach ($m in $matches) {
        $n = $m.Groups[1].Value
        if (-not $n) { $n = $m.Groups[2].Value }
        if (-not $n) { $n = $m.Groups[3].Value }
        if ($inFile.ContainsKey($n)) {
            Write-Host "  [FAIL] 文件内重复声明 $($f.Name) -> $n" -ForegroundColor Red
            $failed = $true
        } else {
            $inFile[$n] = $true
        }
        if ($all.ContainsKey($n)) { $all[$n] += ",$($f.Name)" } else { $all[$n] = $f.Name }
    }
}

foreach ($k in ($all.Keys | Sort-Object)) {
    if ($all[$k].Contains(',')) {
        Write-Host "  [FAIL] 跨文件重名导出 $k <- $($all[$k])（export * 静默歧义）" -ForegroundColor Red
        $failed = $true
    }
}

# 3) node --check
$nodeOk = $true
foreach ($f in @(Get-ChildItem -Path $ApiDir -File -Filter *.js)) {
    $out = node --check $f.FullName 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  [FAIL] 语法错误 $($f.Name): $out" -ForegroundColor Red
        $nodeOk = $false
        $failed = $true
    }
}

if ($failed) {
    Write-Host "== 审计未通过：存在需修复项 ==" -ForegroundColor Red
    exit 1
} else {
    Write-Host "== 审计通过：无重复声明、无跨文件重名、语法全部合法（$($all.Count) 个唯一导出）==" -ForegroundColor Green
    exit 0
}
