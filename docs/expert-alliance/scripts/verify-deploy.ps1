<#
.SYNOPSIS
    专家联盟部署后一致性一键校验（Windows / 内网试点版）。

.DESCRIPTION
    与 verify-deploy.sh 同一份检查逻辑，覆盖 09-deployment-templates.md §六 的 7 项清单。
    只读校验：只发 HTTP GET，不写任何状态文件；token 永不完整打印。

.PARAMETER Host
    目标主机（默认 127.0.0.1）。

.PARAMETER GatewayPort
    网关端口（默认 3080）。

.PARAMETER SchedulerPort
    调度器端口（默认 3100）。

.PARAMETER ExecutorPort
    执行器端口（默认 3200）。

.PARAMETER RegistryPort
    注册中心端口（默认 3400）。

.PARAMETER Token
    MOX_INTERNAL_TOKEN（默认读环境变量 MOX_INTERNAL_TOKEN；CLI 优先于 env）。

.PARAMETER Mode
    dev | production（默认 dev；production 下鉴权/SM4 必查）。

.PARAMETER DataDir
    可选：scheduler/executor 共享 sqlite 目录（如 D:\mox\data）。给了才做本地文件检查。

.PARAMETER AuditLogPath
    可选：审计 NDJSON 绝对路径（如 D:\mox\logs\experts-audit.ndjson）。给了才做本地文件检查。

.PARAMETER Timeout
    单次 HTTP 超时秒数（默认 5）。

.EXAMPLE
    # dev 档（本机联调）
    .\verify-deploy.ps1

.EXAMPLE
    # production 档（内网试点，带 token）
    .\verify-deploy.ps1 -Host 10.0.0.8 -Mode production -Token $env:MOX_INTERNAL_TOKEN -DataDir D:\mox\data -AuditLogPath D:\mox\logs\experts-audit.ndjson
#>

[CmdletBinding()]
param(
    [string]$Host = "127.0.0.1",
    [int]$GatewayPort = 3080,
    [int]$SchedulerPort = 3100,
    [int]$ExecutorPort = 3200,
    [int]$RegistryPort = 3400,
    [string]$Token = "",
    [ValidateSet("dev","production")]
    [string]$Mode = "dev",
    [string]$DataDir = "",
    [string]$AuditLogPath = "",
    [int]$Timeout = 5
)

# ── UTF-8 输出（PS 5.1 默认 GBK，必须显式切）─────────────────────────────
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}
$OutputEncoding = [System.Text.Encoding]::UTF8
$ProgressPreference = "SilentlyContinue"

# ── 环境变量兜底 ────────────────────────────────────────────────────────────
if ([string]::IsNullOrEmpty($Token) -and $env:MOX_INTERNAL_TOKEN) {
    $Token = $env:MOX_INTERNAL_TOKEN
}

# ── 结果计数 ────────────────────────────────────────────────────────────────
$script:PASS = 0
$script:FAIL = 0
$script:WARN = 0
$script:MANUAL = 0

function Mask-Token([string]$t) {
    if ([string]::IsNullOrEmpty($t)) { return "(未设置)" }
    if ($t.Length -le 12) { return "****" }
    return $t.Substring(0,4) + "****" + $t.Substring($t.Length - 4)
}

function Write-Result {
    param([string]$Status, [string]$Name, [string]$Msg)
    switch ($Status) {
        "PASS"   { $script:PASS++ }
        "FAIL"   { $script:FAIL++ }
        "WARN"   { $script:WARN++ }
        "MANUAL" { $script:MANUAL++ }
    }
    "{0,-7} {1,-28} {2}" -f "[$Status]", $Name, $Msg | Write-Host
}

# HTTP GET：把 body 写临时文件，stdout 只回 http_code；失败 code=000
function Invoke-HttpGet {
    param(
        [string]$Url,
        [hashtable]$Headers = @{}
    )
    $tmp = Join-Path $env:TEMP ("verify-deploy-" + [Guid]::NewGuid().ToString("N") + ".body")
    try {
        $argList = @("-sS", "-m", $Timeout, "-o", $tmp, "-w", "%{http_code}")
        foreach ($k in $Headers.Keys) {
            $argList += @("-H", "$k`: $($Headers[$k])")
        }
        $argList += $Url
        $code = & curl.exe @argList 2>$null
        if ($LASTEXITCODE -ne 0) {
            return [pscustomobject]@{ Code = "000"; Body = "" }
        }
        $body = ""
        if (Test-Path $tmp) {
            # 按 UTF-8 读，避免中文乱码
            $body = [System.IO.File]::ReadAllText($tmp, [System.Text.Encoding]::UTF8)
        }
        return [pscustomobject]@{ Code = "$code"; Body = $body }
    } finally {
        if (Test-Path $tmp) { Remove-Item $tmp -Force -ErrorAction SilentlyContinue }
    }
}

# 从 JSON 字符串取顶层字段（ConvertFrom-Json 解析失败时返回 $null）
function Get-JsonField {
    param([string]$Body, [string]$Field)
    if ([string]::IsNullOrEmpty($Body)) { return $null }
    try {
        $obj = $Body | ConvertFrom-Json -ErrorAction Stop
        # 顶层字段
        if ($obj.PSObject.Properties.Name -contains $Field) {
            $v = $obj.$Field
            if ($v -is [string]) { return "`"$v`"" }
            return "$v"
        }
        # 嵌套一层：data.<Field>
        if ($obj.data -and $obj.data.PSObject.Properties.Name -contains $Field) {
            $v = $obj.data.$Field
            if ($v -is [string]) { return "`"$v`"" }
            return "$v"
        }
    } catch { return $null }
    return $null
}

# ── 组装 URL ────────────────────────────────────────────────────────────────
$GwUrl   = "http://${Host}:${GatewayPort}"
$SchedUrl = "http://${Host}:${SchedulerPort}"
$ExecUrl = "http://${Host}:${ExecutorPort}"
$RegUrl  = "http://${Host}:${RegistryPort}"

Write-Host "============================================================"
Write-Host " 专家联盟部署后一致性校验 (Windows)"
Write-Host (" 目标: {0}  gateway={1} scheduler={2}" -f $Host, $GatewayPort, $SchedulerPort)
Write-Host ("        executor={0} registry={1}" -f $ExecutorPort, $RegistryPort)
Write-Host (" 档位: {0}   token={1}" -f $Mode, (Mask-Token $Token))
Write-Host "============================================================"

# ── 检查 1：四进程 /health 探活（09 §6.1）──────────────────────────────────
Write-Host ""
Write-Host "── [1/7] 进程存活 /health（09 §6.1）─────────────────────────"

$r = Invoke-HttpGet "$GwUrl/health"
if ($r.Code -eq "200") {
    $ds = Get-JsonField $r.Body "scheduler"
    $de = Get-JsonField $r.Body "executor"
    Write-Result PASS "1.gateway-health" "200 ok; dependencies.scheduler=$ds, executor=$de"
} else {
    Write-Result FAIL "1.gateway-health" "HTTP $($r.Code)（期望 200）；见 09 §6.1 进程存活"
}

$r = Invoke-HttpGet "$SchedUrl/health"
if ($r.Code -eq "200") {
    $se = Get-JsonField $r.Body "executor"
    Write-Result PASS "1.scheduler-health" "200; dependencies.executor=$se"
} else {
    Write-Result FAIL "1.scheduler-health" "HTTP $($r.Code)（期望 200）；见 09 §6.1"
}

$r = Invoke-HttpGet "$ExecUrl/health"
if ($r.Code -eq "200") {
    $er = Get-JsonField $r.Body "execution_ready"
    $em = Get-JsonField $r.Body "execution_mode"
    Write-Result PASS "1.executor-health" "200; execution_ready=$er, execution_mode=$em"
} else {
    Write-Result FAIL "1.executor-health" "HTTP $($r.Code)（期望 200）；见 09 §6.1"
}

$r = Invoke-HttpGet "$RegUrl/health"
if ($r.Code -eq "200") {
    $r2 = Invoke-HttpGet "$RegUrl/api/registry/health"
    if ($r2.Code -eq "200") {
        $tot = Get-JsonField $r2.Body "instances_total"
        $act = Get-JsonField $r2.Body "instances_active"
        Write-Result PASS "1.registry-health" "200; instances_total=$tot, active=$act"
    } else {
        Write-Result WARN "1.registry-health" "/health=200 但 /api/registry/health=$($r2.Code)；见 09 §6.1"
    }
} else {
    Write-Result FAIL "1.registry-health" "HTTP $($r.Code)（期望 200）；见 09 §6.1"
}

# ── 检查 2：内部鉴权链路（09 §6.4）─────────────────────────────────────────
Write-Host ""
Write-Host "── [2/7] 内部鉴权链路 200/401（09 §6.4）─────────────────────"

if ($Mode -eq "production" -and -not [string]::IsNullOrEmpty($Token)) {
    $authHeaders = @{ "Authorization" = "Bearer $Token" }
    $authOk = $true

    $r = Invoke-HttpGet "$SchedUrl/tasks" -Headers $authHeaders
    if ($r.Code -eq "401" -or $r.Code -eq "000") {
        Write-Result FAIL "2.scheduler-with-token" "带 token 访问 /tasks 返回 $($r.Code)（期望非 401）；token 可能与下游不一致；见 09 §七.1"
        $authOk = $false
    }

    $fakeUuid = "00000000-0000-0000-0000-000000000000"
    $r = Invoke-HttpGet "$ExecUrl/tasks/$fakeUuid/status" -Headers $authHeaders
    if ($r.Code -eq "401" -or $r.Code -eq "000") {
        Write-Result FAIL "2.executor-with-token" "带 token 访问 /tasks/<uuid>/status 返回 $($r.Code)（期望非 401）；见 09 §七.1"
        $authOk = $false
    }

    $r = Invoke-HttpGet "$RegUrl/api/registry/experts" -Headers $authHeaders
    if ($r.Code -eq "401" -or $r.Code -eq "000") {
        Write-Result FAIL "2.registry-with-token" "带 token 访问 /api/registry/experts 返回 $($r.Code)（期望非 401）；见 09 §七.1"
        $authOk = $false
    }
    if ($authOk) { Write-Result PASS "2.with-token" "带 MOX_INTERNAL_TOKEN 访问三 svc 非公开路径均非 401" }

    # 不带 token 期望 401
    $noauthOk = $true
    $targets = @(
        @{ name = "scheduler"; url = "$SchedUrl/tasks" },
        @{ name = "executor";  url = "$ExecUrl/tasks/$fakeUuid/status" },
        @{ name = "registry";  url = "$RegUrl/api/registry/experts" }
    )
    foreach ($t in $targets) {
        $r = Invoke-HttpGet $t.url
        if ($r.Code -ne "401") {
            Write-Result FAIL "2.noauth-$($t.name)" "不带 token 访问 $($t.url) 返回 $($r.Code)（期望 401）；internal_auth_layer 未生效；见 09 §七.6"
            $noauthOk = $false
        }
    }
    if ($noauthOk) { Write-Result PASS "2.noauth" "不带 token 直连三 svc 均被 401 拦截（internal_auth_layer 生效）" }
} else {
    Write-Result WARN "2.auth-chain" "dev 档或未提供 MOX_INTERNAL_TOKEN：下游 internal_auth_layer 放行（符合 09 §2.8 dev 档）；生产请加 -Mode production -Token <值>"
}

# ── 检查 3：SM4 传输加密信封（09 §6.5）────────────────────────────────────
Write-Host ""
Write-Host "── [3/7] SM4 信封协商（09 §6.5）────────────────────────────"

$r = Invoke-HttpGet "$GwUrl/health" -Headers @{ "x-mox-crypto" = "sm4-gcm+gzip" }
$isEnvelope = $false
$hasAllFields = $false
if ($r.Code -eq "200" -and -not [string]::IsNullOrEmpty($r.Body)) {
    try {
        $obj = $r.Body | ConvertFrom-Json -ErrorAction Stop
        # 裸 DTO 整体加密：顶层有 crypto；或统一信封 data.crypto
        $crypto = $null
        if ($obj.PSObject.Properties.Name -contains "crypto") { $crypto = $obj.crypto }
        elseif ($obj.data -and $obj.data.PSObject.Properties.Name -contains "crypto") { $crypto = $obj.data.crypto }
        if ($crypto) {
            $isEnvelope = $true
            $names = $crypto.PSObject.Properties.Name
            if ($names -contains "alg" -and $crypto.alg -eq "SM4-GCM" -and
                $names -contains "nonce" -and $names -contains "ct" -and $names -contains "tag") {
                $hasAllFields = $true
            }
        }
    } catch { }
}
if ($isEnvelope -and $hasAllFields) {
    Write-Result PASS "3.sm4-envelope" "响应为 SM4-GCM 信封（alg/nonce/ct/tag 齐全）"
} elseif ($isEnvelope) {
    Write-Result WARN "3.sm4-envelope" "响应含 crypto 字段但缺 alg/nonce/ct/tag；见 09 §6.5"
} else {
    if ($Mode -eq "production") {
        Write-Result FAIL "3.sm4-envelope" "带协商头响应仍为明文（未加密）；MOX_API_CRYPTO 未设为 sm4；见 09 §七.2"
    } else {
        Write-Result WARN "3.sm4-envelope" "带协商头响应为明文（dev 档默认 MOX_API_CRYPTO=off）；生产必开 sm4；见 09 §2.7"
    }
}

# ── 检查 4：存储模式（09 §6.6）────────────────────────────────────────────
Write-Host ""
Write-Host "── [4/7] sqlite 存储与 WAL（09 §6.6）───────────────────────"

if (-not [string]::IsNullOrEmpty($DataDir)) {
    $db = Join-Path $DataDir "alliance_tasks.db"
    $wal = "$db-wal"
    if (Test-Path $db) {
        if (Test-Path $wal) {
            Write-Result PASS "4.sqlite-wal" "找到 $db 与 WAL 副产物 $wal"
        } else {
            Write-Result WARN "4.sqlite-wal" "找到 $db 但无 -wal 文件（可能 checkpoint 后已合并；持续写入时应存在）"
        }
    } else {
        Write-Result FAIL "4.sqlite-wal" "未找到 $db；检查 MOX_ALLIANCE_STORAGE_MODE=sqlite 与共享卷挂载；见 09 §2.3"
    }
    Write-Result MANUAL "4.user_version" "代码未暴露 user_version（08 缺口 N3）；请手动执行: sqlite3 `"$db`" 'PRAGMA user_version;'"
} else {
    Write-Result MANUAL "4.sqlite-wal" "未提供 -DataDir，无法在外部校验容器/进程内 sqlite 文件；在目标机执行: ls D:\mox\data\alliance_tasks.db*（见 09 §6.6）"
    Write-Result MANUAL "4.user_version" "同上；手动 sqlite3 /var/lib/mox/alliance_tasks.db 'PRAGMA user_version;'"
}

# ── 检查 5：HA 选主（09 §6.3）─────────────────────────────────────────────
Write-Host ""
Write-Host "── [5/7] HA 选主 /leadership（09 §6.3）─────────────────────"

$r = Invoke-HttpGet "$SchedUrl/leadership"
if ($r.Code -ne "200") {
    Write-Result FAIL "5.leadership" "HTTP $($r.Code)（期望 200）；见 09 §6.3"
} else {
    $haEnabled = $false; $isLeader = $false
    try {
        $obj = $r.Body | ConvertFrom-Json -ErrorAction Stop
        if ($obj.PSObject.Properties.Name -contains "ha_enabled") { $haEnabled = [bool]$obj.ha_enabled }
        if ($obj.PSObject.Properties.Name -contains "is_leader") { $isLeader = [bool]$obj.is_leader }
    } catch { }
    if ($haEnabled) {
        if ($isLeader) {
            Write-Result PASS "5.leadership" "HA 已启用，本副本 is_leader=true（多副本时应恰好一个 leader；见 09 §6.3）"
        } else {
            Write-Result PASS "5.leadership" "HA 已启用，本副本 is_leader=false（follower 副本，符合多副本选主）"
        }
    } else {
        Write-Result WARN "5.leadership" "ha_enabled=false：HA 未开启（单副本预期行为，非故障）；如需多副本，见 09 §2.3"
    }
}

# ── 检查 6：审计日志（09 §6.7）─────────────────────────────────────────────
Write-Host ""
Write-Host "── [6/7] 审计 NDJSON（09 §6.7）─────────────────────────────"

if (-not [string]::IsNullOrEmpty($AuditLogPath)) {
    if (Test-Path $AuditLogPath) {
        $firstLine = $null
        # 读首条非空行（按 UTF-8）
        $lines = [System.IO.File]::ReadAllLines($AuditLogPath, [System.Text.Encoding]::UTF8)
        foreach ($ln in $lines) {
            if (-not [string]::IsNullOrWhiteSpace($ln)) { $firstLine = $ln; break }
        }
        if ([string]::IsNullOrEmpty($firstLine)) {
            Write-Result WARN "6.audit-file" "文件存在但为空：尚未触发任何审计事件（登录/任务操作后应有新行）"
        } elseif ($firstLine -match '"actor"') {
            Write-Result PASS "6.audit-file" "文件存在，首行含 actor 字段（NDJSON 结构正常）"
        } else {
            Write-Result FAIL "6.audit-file" "文件存在但首行未含 actor 字段；审计链路异常；见 09 §6.7"
        }
    } else {
        Write-Result FAIL "6.audit-file" "未找到 $AuditLogPath；检查 MOX_AUDIT_LOG_PATH / MOX_AUDIT_SINK；见 09 §2.1"
    }
    Write-Result MANUAL "6.audit-hmac" "HMAC 哈希链重算需用代码内 verify_chain()（mox-audit）；脚本不重算；见 09 §七.5"
} else {
    Write-Result MANUAL "6.audit-file" "未提供 -AuditLogPath；在目标机执行: Get-Content D:\mox\logs\experts-audit.ndjson -Tail 5（见 09 §6.7）"
    Write-Result MANUAL "6.audit-hmac" "同上；HMAC 校验用 mox-audit verify_chain()"
}

# ── 检查 7：登记状态 ≠ 探活（09 §七.4）────────────────────────────────────
Write-Host ""
Write-Host "── [7/7] 健康语义提示（09 §七.4）──────────────────────────"

$r = Invoke-HttpGet "$GwUrl/health"
if ($r.Code -eq "200") {
    $ds = Get-JsonField $r.Body "scheduler"
    $de = Get-JsonField $r.Body "executor"
    Write-Result PASS "7.health-semantics" "gateway /health dependencies: scheduler=$ds, executor=$de（探活结果）"
} else {
    Write-Result WARN "7.health-semantics" "gateway /health=$($r.Code)，无法读取依赖探活；见 09 §七.4"
}
Write-Result WARN "7.health-semantics" "注意：availability.status(online/busy/offline) 是登记值，不是探活结果；真探活看 registry probe（MOX_ALLIANCE_REGISTRY_PROBE_ENABLED=1）；见 09 §七.4"

# ── 汇总 ────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "============================================================"
$total = $script:PASS + $script:FAIL + $script:WARN + $script:MANUAL
Write-Host (" 汇总: PASS={0}  FAIL={1}  WARN={2}  MANUAL={3}  / 共 {4}" -f $script:PASS, $script:FAIL, $script:WARN, $script:MANUAL, $total)
if ($script:FAIL -eq 0) {
    Write-Host (" VERIFY: PASS ({0}/{1} 项硬指标通过；WARN/MANUAL 需人工跟进)" -f $script:PASS, $total)
    Write-Host "============================================================"
    exit 0
} else {
    Write-Host " VERIFY: FAIL ($($script:FAIL) 项未通过；请按上述 FAIL 行的修复提示处理)"
    Write-Host "============================================================"
    $code = [Math]::Min($script:FAIL, 127)
    exit $code
}
