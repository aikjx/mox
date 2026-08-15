#!/usr/bin/env bash
# 算子统一系统 (OUS) —— 全量功能验证测试脚本
# 用途：编译 + 运行所有单元测试 + 对 runtime 做端到端 API 冒烟验证
#
# 用法（在仓库根目录执行）：
#   bash verify_tests.sh            # 仅单元/集成测试
#   bash verify_tests.sh --e2e      # 额外启动 runtime 并探测 HTTP 端点
#
# 依赖：cargo (Rust nightly 已验证 1.98)，可选 PowerShell (Windows 端到端探测)

set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'; NC='\033[0m'
pass(){ echo -e "${GREEN}[PASS]${NC} $*"; }
fail(){ echo -e "${RED}[FAIL]${NC} $*"; exit 1; }

echo "============================================================"
echo " OUS 全量验证  @ $(date '+%Y-%m-%d %H:%M:%S')"
echo "============================================================"

# ---------- 1. 编译（含测试目标）----------
echo -e "${YELLOW}[1/3] 编译 workspace + tests ...${NC}"
cargo test --workspace --no-run 2>&1 | tail -5
echo

# ---------- 2. 运行所有单元测试 / 集成测试 ----------
echo -e "${YELLOW}[2/3] 运行 cargo test --workspace ...${NC}"
if cargo test --workspace 2>&1 | tee test_report.log | grep -qE "test result: FAILED|error\["; then
  echo
  grep -E "test result: FAILED|FAILED|panicked|error\[" test_report.log || true
  fail "存在失败的测试，详见 test_report.log"
fi
TOTAL=$(grep -oE "test result: ok\. [0-9]+ passed" test_report.log | awk '{s+=$4} END{print s}')
echo
pass "全部单元测试通过，合计 $TOTAL 个用例 (0 failed)"
echo

# ---------- 3. 端到端 API 冒烟（可选）----------
if [[ "${1:-}" == "--e2e" ]]; then
  echo -e "${YELLOW}[3/3] 端到端 API 冒烟 ...${NC}"
  PORT=3998
  RUST_LOG=warn cargo run -p runtime -- --port "$PORT" >runtime_smoke.out 2>&1 &
  SRV=$!
  # 等待端口就绪
  for i in $(seq 1 30); do
    if command -v curl >/dev/null 2>&1; then
      curl -s -o /dev/null "http://127.0.0.1:$PORT/" && break
    fi
    sleep 1
  done
  check(){ local path="$1"; local code; code=$(curl -s -o /tmp/ous_body -w "%{http_code}" "http://127.0.0.1:$PORT$path");
    if [[ "$code" == "200" ]]; then pass "GET $path -> $code"; else fail "GET $path -> $code"; fi; }
  check "/"
  check "/api/operators"
  check "/api/graph"
  # 对话端点
  code=$(curl -s -o /tmp/ous_chat -w "%{http_code}" -X POST "http://127.0.0.1:$PORT/api/ai/chat" \
         -H "Content-Type: application/json" -d '{"session_id":"v1","message":"列出所有算子"}')
  [[ "$code" == "200" ]] && pass "POST /api/ai/chat -> $code" || fail "POST /api/ai/chat -> $code"
  kill "$SRV" 2>/dev/null || true
  echo
  pass "端到端 API 冒烟全部通过"
else
  echo -e "${YELLOW}[3/3] 跳过端到端 (使用 --e2e 启用) ${NC}"
fi

echo "============================================================"
echo -e "${GREEN} ✓ OUS 验证完成${NC}"
echo "============================================================"
