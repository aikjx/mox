#!/usr/bin/env bash
# =============================================================================
# verify-deploy.sh — 专家联盟部署后一致性一键校验（Linux k8s 生产版）
#
# 对应文档：docs/expert-alliance/09-deployment-templates.md §六（7 项清单）
# 同一份检查逻辑的 Windows 版见 verify-deploy.ps1。
#
# 用法见同目录 README.md。退出码：
#   0 = 全 PASS（允许 WARN/MANUAL）
#   N = FAIL 计数（1..127）
#
# 只读校验：只发 HTTP GET，不写任何状态文件；token 永不完整打印。
# =============================================================================
set -uo pipefail

# ── 默认值（对齐 09 §2.7 生产档与 §二拓扑）─────────────────────────────────
HOST="127.0.0.1"
GATEWAY_PORT=3080
SCHEDULER_PORT=3100
EXECUTOR_PORT=3200
REGISTRY_PORT=3400
TOKEN="${MOX_INTERNAL_TOKEN:-}"
MODE="dev"                 # dev | production
DATA_DIR=""                # 可选：scheduler/executor 共享 sqlite 目录；给了才做本地文件检查
AUDIT_LOG_PATH=""           # 可选：审计 NDJSON 路径；给了才做本地文件检查
TIMEOUT=5

# ── 结果计数 ────────────────────────────────────────────────────────────────
PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0
MANUAL_COUNT=0

# 中文 UTF-8 输出（生产镜像通常已带 LANG；这里兜底一次）
export LANG="${LANG:-C.UTF-8}"
export LC_ALL="${LC_ALL:-C.UTF-8}"

# ── 工具函数 ────────────────────────────────────────────────────────────────

usage() {
  cat <<'EOF'
专家联盟部署后一致性校验（Linux）

用法: verify-deploy.sh [选项]

  --host HOST              目标主机（默认 127.0.0.1）
  --gateway-port PORT      网关端口（默认 3080）
  --scheduler-port PORT    调度器端口（默认 3100）
  --executor-port PORT     执行器端口（默认 3200）
  --registry-port PORT     注册中心端口（默认 3400）
  --token TOKEN            MOX_INTERNAL_TOKEN（默认读环境变量 MOX_INTERNAL_TOKEN）
  --mode dev|production    校验档位（默认 dev；production 下鉴权/SM4 必查）
  --data-dir DIR           可选：scheduler/executor 共享 sqlite 目录（如 /var/lib/mox）
  --audit-log-path PATH    可选：审计 NDJSON 绝对路径（如 /var/log/mox/experts-audit.ndjson）
  --timeout SEC            单次 HTTP 超时秒数（默认 5）
  -h, --help               显示本帮助

环境变量: MOX_INTERNAL_TOKEN 等价于 --token（CLI 优先于 env）。
EOF
}

# 掩码 token：前 4 后 4，中间 ****
mask_token() {
  local t="$1"
  if [ -z "$t" ]; then
    echo "(未设置)"
  elif [ ${#t} -le 12 ]; then
    echo "****"
  else
    echo "${t:0:4}****${t: -4}"
  fi
}

# record <PASS|FAIL|WARN|MANUAL> <检查项名> <说明>
record() {
  local status="$1" name="$2" msg="$3"
  case "$status" in
    PASS)   PASS_COUNT=$((PASS_COUNT+1)) ;;
    FAIL)   FAIL_COUNT=$((FAIL_COUNT+1)) ;;
    WARN)   WARN_COUNT=$((WARN_COUNT+1)) ;;
    MANUAL) MANUAL_COUNT=$((MANUAL_COUNT+1)) ;;
  esac
  printf '[%-6s] %-28s %s\n' "$status" "$name" "$msg"
}

# http_get <url> [extra-curl-args...]
# 输出 "HTTP_CODE|BODY"（BODY 可能含换行，用 $'\x1f' 分隔更稳）
http_get() {
  local url="$1"; shift || true
  local code body
  body=$(curl -sS -m "$TIMEOUT" -o - -w $'\n%{http_code}' "$@" "$url" 2>/dev/null)
  code=$?
  if [ "$code" -ne 0 ]; then
    echo "000|"
    return
  fi
  # 最后一行是 http_code
  local http_code="${body##*$'\n'}"
  local resp_body="${body%$'\n'*}"
  echo "${http_code}|${resp_body}"
}

# 从 JSON 字符串里取顶层标量（粗解析，不依赖 jq）
# json_get <json> <key>
json_get() {
  local body="$1" key="$2"
  # 匹配 "key":"value" 或 "key":number/true/false
  echo "$body" | grep -oE "\"${key}\"[[:space:]]*:[[:space:]]*(\"[^\"]*\"|[0-9]+(\\.[0-9]+)?|true|false|null)" \
    | head -n1 | sed -E "s/.*:[[:space:]]*//"
}

# ── 参数解析 ────────────────────────────────────────────────────────────────
while [ $# -gt 0 ]; do
  case "$1" in
    --host)            HOST="$2"; shift 2 ;;
    --gateway-port)    GATEWAY_PORT="$2"; shift 2 ;;
    --scheduler-port)  SCHEDULER_PORT="$2"; shift 2 ;;
    --executor-port)   EXECUTOR_PORT="$2"; shift 2 ;;
    --registry-port)   REGISTRY_PORT="$2"; shift 2 ;;
    --token)           TOKEN="$2"; shift 2 ;;
    --mode)            MODE="$2"; shift 2 ;;
    --data-dir)        DATA_DIR="$2"; shift 2 ;;
    --audit-log-path)  AUDIT_LOG_PATH="$2"; shift 2 ;;
    --timeout)         TIMEOUT="$2"; shift 2 ;;
    -h|--help)         usage; exit 0 ;;
    *) echo "未知参数: $1" >&2; usage; exit 2 ;;
  esac
done

# 统一主机协议前缀
SCHEME="http"
GW_URL="${SCHEME}://${HOST}:${GATEWAY_PORT}"
SCHED_URL="${SCHEME}://${HOST}:${SCHEDULER_PORT}"
EXEC_URL="${SCHEME}://${HOST}:${EXECUTOR_PORT}"
REG_URL="${SCHEME}://${HOST}:${REGISTRY_PORT}"

echo "============================================================"
echo " 专家联盟部署后一致性校验"
echo " 目标: ${HOST}  gateway=${GATEWAY_PORT} scheduler=${SCHEDULER_PORT}"
echo "        executor=${EXECUTOR_PORT} registry=${REGISTRY_PORT}"
echo " 档位: ${MODE}   token=$(mask_token "$TOKEN")"
echo "============================================================"

# ── 检查 1：四进程 /health 探活（09 §6.1）──────────────────────────────────
echo
echo "── [1/7] 进程存活 /health（09 §6.1）─────────────────────────"

# 1.1 gateway
resp=$(http_get "${GW_URL}/health")
code="${resp%%|*}"; body="${resp#*|}"
if [ "$code" = "200" ]; then
  deps_sched=$(json_get "$body" "scheduler")
  deps_exec=$(json_get "$body" "executor")
  record PASS "1.gateway-health" "200 ok; dependencies.scheduler=${deps_sched:-?}, executor=${deps_exec:-?}"
else
  record FAIL "1.gateway-health" "HTTP ${code}（期望 200）；见 09 §6.1 进程存活"
fi

# 1.2 scheduler
resp=$(http_get "${SCHED_URL}/health")
code="${resp%%|*}"; body="${resp#*|}"
if [ "$code" = "200" ]; then
  sdep=$(json_get "$body" "executor")
  record PASS "1.scheduler-health" "200; dependencies.executor=${sdep:-?}"
else
  record FAIL "1.scheduler-health" "HTTP ${code}（期望 200）；见 09 §6.1"
fi

# 1.3 executor
resp=$(http_get "${EXEC_URL}/health")
code="${resp%%|*}"; body="${resp#*|}"
if [ "$code" = "200" ]; then
  ready=$(json_get "$body" "execution_ready")
  emode=$(json_get "$body" "execution_mode")
  record PASS "1.executor-health" "200; execution_ready=${ready:-?}, execution_mode=${emode:-?}"
else
  record FAIL "1.executor-health" "HTTP ${code}（期望 200）；见 09 §6.1"
fi

# 1.4 registry（/health 返回纯文本 "ok"，非 JSON；另探 /api/registry/health 拿 JSON 计数）
resp=$(http_get "${REG_URL}/health")
code="${resp%%|*}"; body="${resp#*|}"
if [ "$code" = "200" ]; then
  resp2=$(http_get "${REG_URL}/api/registry/health")
  code2="${resp2%%|*}"; body2="${resp2#*|}"
  if [ "$code2" = "200" ]; then
    total=$(json_get "$body2" "instances_total")
    active=$(json_get "$body2" "instances_active")
    record PASS "1.registry-health" "200; instances_total=${total:-?}, active=${active:-?}"
  else
    record WARN "1.registry-health" "/health=200 但 /api/registry/health=${code2}；见 09 §6.1"
  fi
else
  record FAIL "1.registry-health" "HTTP ${code}（期望 200）；见 09 §6.1"
fi

# ── 检查 2：内部鉴权链路（09 §6.4）─────────────────────────────────────────
echo
echo "── [2/7] 内部鉴权链路 200/401（09 §6.4）─────────────────────"

if [ "$MODE" = "production" ] && [ -n "$TOKEN" ]; then
  # 2.1 带 token 访问三 svc 非公开路径，期望非 401
  auth_ok=1
  resp=$(http_get "${SCHED_URL}/tasks" -H "Authorization: Bearer ${TOKEN}")
  code="${resp%%|*}"
  if [ "$code" = "401" ] || [ "$code" = "000" ]; then
    record FAIL "2.scheduler-with-token" "带 token 访问 /tasks 返回 ${code}（期望非 401）；token 可能与下游不一致；见 09 §七.1"
    auth_ok=0
  fi

  resp=$(http_get "${EXEC_URL}/tasks/00000000-0000-0000-0000-000000000000/status" -H "Authorization: Bearer ${TOKEN}")
  code="${resp%%|*}"
  if [ "$code" = "401" ] || [ "$code" = "000" ]; then
    record FAIL "2.executor-with-token" "带 token 访问 /tasks/<uuid>/status 返回 ${code}（期望非 401）；见 09 §七.1"
    auth_ok=0
  fi

  resp=$(http_get "${REG_URL}/api/registry/experts" -H "Authorization: Bearer ${TOKEN}")
  code="${resp%%|*}"
  if [ "$code" = "401" ] || [ "$code" = "000" ]; then
    record FAIL "2.registry-with-token" "带 token 访问 /api/registry/experts 返回 ${code}（期望非 401）；见 09 §七.1"
    auth_ok=0
  fi
  [ "$auth_ok" = "1" ] && record PASS "2.with-token" "带 MOX_INTERNAL_TOKEN 访问三 svc 非公开路径均非 401"

  # 2.2 不带 token，期望 401
  noauth_ok=1
  for target in "scheduler:${SCHED_URL}/tasks" "executor:${EXEC_URL}/tasks/00000000-0000-0000-0000-000000000000/status" "registry:${REG_URL}/api/registry/experts"; do
    name="${target%%:*}"; url="${target#*:}"
    resp=$(http_get "$url")
    code="${resp%%|*}"
    if [ "$code" != "401" ]; then
      record FAIL "2.noauth-${name}" "不带 token 访问 ${url} 返回 ${code}（期望 401）；internal_auth_layer 未生效；见 09 §七.6"
      noauth_ok=0
    fi
  done
  [ "$noauth_ok" = "1" ] && record PASS "2.noauth" "不带 token 直连三 svc 均被 401 拦截（internal_auth_layer 生效）"
else
  record WARN "2.auth-chain" "dev 档或未提供 MOX_INTERNAL_TOKEN：下游 internal_auth_layer 放行（符合 09 §2.8 dev 档）；生产请加 --mode production --token <值>"
fi

# ── 检查 3：SM4 传输加密信封（09 §6.5）────────────────────────────────────
echo
echo "── [3/7] SM4 信封协商（09 §6.5）────────────────────────────"

# 带协商头请求 gateway /health（裸 DTO，会被整体加密为 {"crypto":{...}}）
resp=$(http_get "${GW_URL}/health" -H "x-mox-crypto: sm4-gcm+gzip" -D /tmp/.verify-headers.$$)
code="${resp%%|*}"; body="${resp#*|}"
# 响应头应回带 x-mox-crypto
echo_hdr=$(grep -i '^x-mox-crypto:' /tmp/.verify-headers.$$ 2>/dev/null | tr -d '\r' || true)
rm -f /tmp/.verify-headers.$$

if echo "$body" | grep -qE '"crypto"[[:space:]]*:[[:space:]]*\{'; then
  # 进一步校验信封关键字段
  if echo "$body" | grep -qE '"alg"[[:space:]]*:[[:space:]]*"SM4-GCM"' \
     && echo "$body" | grep -qE '"nonce"[[:space:]]*:' \
     && echo "$body" | grep -qE '"ct"[[:space:]]*:' \
     && echo "$body" | grep -qE '"tag"[[:space:]]*:'; then
    record PASS "3.sm4-envelope" "响应为 SM4-GCM 信封（alg/nonce/ct/tag 齐全）；回带头: ${echo_hdr:-（无）}"
  else
    record WARN "3.sm4-envelope" "响应含 crypto 字段但缺 alg/nonce/ct/tag；见 09 §6.5"
  fi
else
  if [ "$MODE" = "production" ]; then
    record FAIL "3.sm4-envelope" "带协商头响应仍为明文（未加密）；MOX_API_CRYPTO 未设为 sm4；见 09 §七.2"
  else
    record WARN "3.sm4-envelope" "带协商头响应为明文（dev 档默认 MOX_API_CRYPTO=off）；生产必开 sm4；见 09 §2.7"
  fi
fi

# ── 检查 4：存储模式（09 §6.6）────────────────────────────────────────────
echo
echo "── [4/7] sqlite 存储与 WAL（09 §6.6）───────────────────────"

if [ -n "$DATA_DIR" ]; then
  db="${DATA_DIR%/}/alliance_tasks.db"
  wal="${db}-wal"
  if [ -f "$db" ]; then
    if [ -f "$wal" ]; then
      record PASS "4.sqlite-wal" "找到 ${db} 与 WAL 副产物 ${wal}"
    else
      record WARN "4.sqlite-wal" "找到 ${db} 但无 -wal 文件（可能 checkpoint 后已合并；生产持续写入时应存在）"
    fi
  else
    record FAIL "4.sqlite-wal" "未找到 ${db}；检查 MOX_ALLIANCE_STORAGE_MODE=sqlite 与共享卷挂载；见 09 §2.3"
  fi
  # user_version：端点不暴露，提示运维用 sqlite3 直查（不依赖脚本内置 sqlite3 二进制）
  record MANUAL "4.user_version" "代码未暴露 user_version（08 缺口 N3）；请在容器内执行: sqlite3 ${db} 'PRAGMA user_version;'"
else
  record MANUAL "4.sqlite-wal" "未提供 --data-dir，无法在外部校验容器内 sqlite 文件；在 Pod 内执行: ls -l /var/lib/mox/alliance_tasks.db*（见 09 §6.6）"
  record MANUAL "4.user_version" "同上；手动 sqlite3 /var/lib/mox/alliance_tasks.db 'PRAGMA user_version;'"
fi

# ── 检查 5：HA 选主（09 §6.3）─────────────────────────────────────────────
echo
echo "── [5/7] HA 选主 /leadership（09 §6.3）─────────────────────"

resp=$(http_get "${SCHED_URL}/leadership")
code="${resp%%|*}"; body="${resp#*|}"
if [ "$code" != "200" ]; then
  record FAIL "5.leadership" "HTTP ${code}（期望 200）；见 09 §6.3"
else
  ha_enabled=$(json_get "$body" "ha_enabled")
  is_leader=$(json_get "$body" "is_leader")
  if echo "$ha_enabled" | grep -qi "true"; then
    if echo "$is_leader" | grep -qi "true"; then
      record PASS "5.leadership" "HA 已启用，本副本 is_leader=true（多副本时应恰好一个 leader；见 09 §6.3）"
    else
      record PASS "5.leadership" "HA 已启用，本副本 is_leader=false（follower 副本，符合多副本选主）"
    fi
  else
    record WARN "5.leadership" "ha_enabled=false：HA 未开启（单副本预期行为，非故障）；如需多副本，见 09 §2.3"
  fi
fi

# ── 检查 6：审计日志（09 §6.7）─────────────────────────────────────────────
echo
echo "── [6/7] 审计 NDJSON（09 §6.7）─────────────────────────────"

if [ -n "$AUDIT_LOG_PATH" ]; then
  if [ -f "$AUDIT_LOG_PATH" ]; then
    # 取首行非空行做结构校验：能解析为 JSON 且含 actor 字段即可
    first_line=$(grep -m1 -E '.' "$AUDIT_LOG_PATH" 2>/dev/null || true)
    if [ -z "$first_line" ]; then
      record WARN "6.audit-file" "文件存在但为空：尚未触发任何审计事件（登录/任务操作后应有新行）"
    elif echo "$first_line" | grep -qE '"actor"'; then
      record PASS "6.audit-file" "文件存在，首行含 actor 字段（NDJSON 结构正常）"
    else
      record FAIL "6.audit-file" "文件存在但首行未含 actor 字段；审计链路异常；见 09 §6.7"
    fi
  else
    record FAIL "6.audit-file" "未找到 ${AUDIT_LOG_PATH}；检查 MOX_AUDIT_LOG_PATH / MOX_AUDIT_SINK；见 09 §2.1"
  fi
  record MANUAL "6.audit-hmac" "HMAC 哈希链重算需用代码内 verify_chain()（mox-audit）；脚本不重算；见 09 §七.5"
else
  record MANUAL "6.audit-file" "未提供 --audit-log-path；在 Pod 内执行: tail -n 5 /var/log/mox/experts-audit.ndjson（见 09 §6.7）"
  record MANUAL "6.audit-hmac" "同上；HMAC 校验用 mox-audit verify_chain()"
fi

# ── 检查 7：登记状态 ≠ 探活（09 §七.4）────────────────────────────────────
echo
echo "── [7/7] 健康语义提示（09 §七.4）──────────────────────────"

resp=$(http_get "${GW_URL}/health")
code="${resp%%|*}"; body="${resp#*|}"
if [ "$code" = "200" ]; then
  ds=$(json_get "$body" "scheduler")
  de=$(json_get "$body" "executor")
  record PASS "7.health-semantics" "gateway /health dependencies: scheduler=${ds:-?}, executor=${de:-?}（探活结果）"
else
  record WARN "7.health-semantics" "gateway /health=${code}，无法读取依赖探活；见 09 §七.4"
fi
record WARN "7.health-semantics" "注意：availability.status(online/busy/offline) 是登记值，不是探活结果；真探活看 registry probe（MOX_ALLIANCE_REGISTRY_PROBE_ENABLED=1）；见 09 §七.4"

# ── 汇总 ────────────────────────────────────────────────────────────────────
echo
echo "============================================================"
total=$((PASS_COUNT + FAIL_COUNT + WARN_COUNT + MANUAL_COUNT))
echo " 汇总: PASS=${PASS_COUNT}  FAIL=${FAIL_COUNT}  WARN=${WARN_COUNT}  MANUAL=${MANUAL_COUNT}  / 共 ${total}"
if [ "$FAIL_COUNT" -eq 0 ]; then
  echo " VERIFY: PASS (${PASS_COUNT}/${total} 项硬指标通过；WARN/MANUAL 需人工跟进)"
  echo "============================================================"
  exit 0
else
  echo " VERIFY: FAIL (${FAIL_COUNT} 项未通过；请按上述 FAIL 行的修复提示处理)"
  echo "============================================================"
  # 退出码 = FAIL 计数，上限 127
  exit $(( FAIL_COUNT > 127 ? 127 : FAIL_COUNT ))
fi
