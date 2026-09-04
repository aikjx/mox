#!/bin/bash
# 算子统一系统 v3.0.0 端到端冒烟测试
# 用法: bash scripts/smoke_test.sh [base_url] [token]
BASE="${1:-http://localhost:3000}"
TOKEN="${2:-dev-secret-token}"
AUTH="Authorization: Bearer $TOKEN"
PASS=0; FAIL=0; FAILED=()

check() {
  local name="$1" method="$2" path="$3" expect="$4" data="$5"
  local out code
  if [ -n "$data" ]; then
    out=$(curl -s -o /tmp/smoke_body.txt -w "%{http_code}" -X "$method" -H "$AUTH" -H "Content-Type: application/json" -d "$data" "$BASE$path" 2>/dev/null)
  else
    out=$(curl -s -o /tmp/smoke_body.txt -w "%{http_code}" -X "$method" -H "$AUTH" "$BASE$path" 2>/dev/null)
  fi
  code="$out"
  if [[ "$code" == "$expect" ]]; then
    PASS=$((PASS+1)); echo "✅ [$code] $method $path  ($name)"
  else
    FAIL=$((FAIL+1)); FAILED+=("$name [$method $path] got $code want $expect")
    echo "❌ [$code] $method $path  ($name) want $expect body=$(head -c 160 /tmp/smoke_body.txt)"
  fi
}

echo "══════════ 算子统一系统 v3.0.0 冒烟测试 ══════════"
echo "Base: $BASE"

# ── 工作台 / 系统 ──
check "健康检查"      GET  /api/health 200
check "系统状态"      GET  /api/status 200
check "完整状态"      GET  /api/status/full 200
check "执行日志"      GET  /api/logs 200
check "插件列表"      GET  /api/plugins 200
check "审计日志"      GET  /api/audit 200

# ── 算子中心 ──
check "算子列表"      GET  /api/operators 200
check "注册算子"      POST /api/operators/register 200 '{"id":"smoke_op","name":"Smoke","operator_type":"function","description":"smoke"}'
check "执行工作流"    POST /api/execute 200 '{"workflow":["identity","relu"],"input":[1,-2,3],"parameters":{}}'

# ── 知识图谱 ──
check "图谱数据"      GET  /api/graph 200
check "图谱统计"      GET  /api/graph/stats 200
check "中心性"        GET  /api/graph/centrality 200
check "社区发现"      GET  /api/graph/communities 200
check "PageRank"      GET  /api/graph/pagerank 200
check "最短路径"      GET  "/api/graph/path?source=relu&target=softmax" 200
check "搜索"          GET  "/api/graph/search?q=relu" 200
check "激活传播"      POST /api/graph/activate 200 '{"start_nodes":["relu"],"iterations":5}'
check "新增节点"      POST /api/graph/node 200 '{"id":"smoke_node","label":"冒烟节点","node_type":"custom"}'
check "新增边"        POST /api/graph/edge 200 '{"source":"relu","target":"smoke_node","weight":0.5,"relation_type":"smoke"}'
check "智能推荐"      POST /api/graph/recommend 200 '{"context_nodes":["relu","softmax"],"limit":5}'
check "自动同步状态"  GET  /api/graph/auto-sync/status 200
check "对话会话"      GET  /api/dialogue/sessions 200
check "图谱导出"      GET  /api/graph/export 200

# ── AI 助手 ──
check "AI 对话"       POST /api/ai/chat 200 '{"session_id":"smoke","message":"你好"}'
check "对话历史"      GET  /api/ai/chat/history/smoke 200
check "算法类型"      GET  /api/ai/algorithm-types 200
check "算法分析"      POST /api/ai/analyze-algorithm 200 '{"code":"def f(x): return x*2","language":"python"}'

# ── 资源管理 ──
check "资源全景"      GET  /api/ai/resources 200
check "资源健康"      GET  /api/ai/resources/health 200

# ── AI 插件 ──
check "插件列表"      GET  /api/ai/plugins 200
check "插件拓扑"      GET  /api/ai/plugins/topology 200
check "插件发消息"    POST /api/ai/plugins/send-message 200 '{"source":"smoke","topic":"test.topic","payload":{"ok":true},"need_response":false}'

# ── 工作流编排 ──
check "工作流模板"    GET  /api/ai/workflows/templates 200
check "工作流列表"    GET  /api/ai/workflows 200
check "工作流实例"    GET  /api/ai/workflows/instances 200
check "流程图列表"    GET  /api/ai/flows 200
check "节点类型"      GET  /api/ai/flows/node-types 200
check "流程图校验"    POST /api/ai/flows/validate 200 '{"flow":{"id":"smoke-flow","name":"冒烟流程","description":"验证用","nodes":[{"id":"n1","node_type":"Start","name":"开始","config":{}},{"id":"n2","node_type":"End","name":"结束","config":{}}],"edges":[{"id":"e1","source":"n1","target":"n2"}],"variables":{},"created_at":"2026-08-19T00:00:00Z","updated_at":"2026-08-19T00:00:00Z"}}'

# ── LLM 配置 ──
check "LLM 配置"      GET  /api/ai/llm/config 200

# ── 浏览器自动化 ──
check "浏览器模板"    GET  /api/ai/browser/templates 200
check "浏览器会话"    GET  /api/ai/browser/sessions 200
check "浏览器任务"    POST /api/ai/browser/execute-task 200 '{"task_id":"web-search","variables":{"query":"smoke"}}'

# ── 算子商城 ──
check "商城列表"      GET  /api/market 200
check "商城随机"      GET  /api/market/random 200
check "商城导出全部"  GET  /api/market/export/all 200

# ── MCP 兼容 ──
check "MCP tools/list" POST /api/mcp 200 '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'

# ── AI 自动化 ──
check "自动化列表"    GET  /api/automation 200
check "需求对话"      POST /api/automation/chat 200 '{"requirement":"做一个冒烟测试资产","tags":["smoke"]}'

# ── 需求编译 ──
check "编译模板"      GET  /api/caomei/templates 200
check "需求编译"      POST /api/caomei/compile 200 '{"requirement":"用户登录后展示积分余额"}'

# ── mox 模块化系统架构融合 / 璇玑 ──
check "璇玑健康"      GET  /api/mox/health 200
check "治理台"        GET  /api/governance/dashboard 200

# ── API 文档 ──
check "OpenAPI YAML"  GET  /api/openapi.yaml 200
check "Swagger UI"    GET  /api/docs 200

echo ""
echo "══════════ 结果汇总 ══════════"
echo "通过: $PASS / $((PASS+FAIL))"
if [ ${#FAILED[@]} -gt 0 ]; then
  echo "失败明细:"
  for f in "${FAILED[@]}"; do echo "  - $f"; done
  exit 1
fi
echo "🎉 全部通过"
