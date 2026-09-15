# 04 · 31域路由 API 规范

> **版本**: v1.0 · **日期**: 2026-08-27
> **实现文件**: `platform/gateway/mox-platform-gateway-svc/src/routes.rs`

## 一、路由注册中心设计

### 核心数据结构

```rust
pub struct Domain {
    pub prefix: &'static str,   // 路由前缀，如 "/kg/v1"
    pub name: &'static str,     // 域名，如 "kg/知识图谱"
    pub status: DomainStatus,   // ready / stub / deprecated
    pub owner: &'static str,    // 负责层级，如 "L2"
    pub note: &'static str,     // 备注说明
}
```

### 注册机制

`build_gateway_router()` 函数自动完成：
1. 遍历 `DOMAINS` 常量数组（31 个域声明）
2. 对 `status = "ready"` 的域，merge 其真实 Router
3. 对 `status = "stub"` 的域，注入统一 404 响应：
   ```json
   {
     "ok": false,
     "stub": true,
     "redirect_to": "/domains",
     "message": "此域为路由桩，待实现"
   }
   ```
4. 挂载 `/health` 健康检查和 `/domains` 域发现端点

---

## 二、31 域完整清单

### 已就绪域（status = ready）

| # | 前缀 | 域名 | 负责层 | 说明 |
|---|---|---|---|---|
| 1 | `/kg/v1` | kg/知识图谱 | L2 | 6接口真实桥接（邻域/路径/中心性/社区/统计/最短路径） |
| 2 | `/ai/engine` | ai/AI引擎 | L3 | 4接口（process/analyze/capabilities/metrics） |

### 路由桩域（status = stub）

| # | 前缀 | 域名 | 负责层 | 说明 |
|---|---|---|---|---|
| 3 | `/iam/v1` | iam/身份认证 | L0 | 用户/角色/权限/令牌 |
| 4 | `/cloud/v1` | cloud/云存储主服务 | L4 | 桶/对象/元数据 |
| 5 | `/cloud/s3` | cloud/S3兼容 | L4 | S3 API 兼容层 |
| 6 | `/cloud/volume` | cloud/卷管理 | L4 | 块存储卷 |
| 7 | `/data/v1` | data/数据平面 | L4 | 数据读写/查询 |
| 8 | `/data/etl` | data/ETL | L4 | 抽取/转换/加载 |
| 9 | `/data/compliance` | data/数据合规 | L4 | 脱敏/审计/分级 |
| 10 | `/data/catalog` | data/数据目录 | L4 | 元数据/血缘/分类 |
| 11 | `/flow/v1` | flow/流程引擎 | L3 | 流程定义/执行/监控 |
| 12 | `/flow/wasm` | flow/WASM算子 | L3 | WASM 插件执行 |
| 13 | `/flow/primiflow` | flow/PrimiFlow | L3 | 原语流 |
| 14 | `/voice/v1` | voice/语音核心 | L3 | 语音通用接口 |
| 15 | `/voice/asr` | voice/语音识别 | L3 | ASR 转写 |
| 16 | `/voice/tts` | voice/语音合成 | L3 | TTS 合成 |
| 17 | `/voice/intent` | voice/语音意图 | L3 | 语音指令理解 |
| 18 | `/market/v1` | market/插件市场 | L8 | 插件注册/分发/计费 |
| 19 | `/market/template` | market/模板市场 | L8 | 模板管理 |
| 20 | `/streams/v1` | streams/数据流 | L3 | 事件总线/CDC/流处理 |
| 21 | `/enterprise/v1` | enterprise/企业编排 | L2 | 项目/设计/评审/部署/运维/复盘 |
| 22 | `/orchestrator/v1` | orchestrator/通用编排 | L2 | 任务调度/工作流 |
| 23 | `/test-harness/v1` | test/测试编排 | L2 | 集成测试/验证/冒烟 |
| 24 | `/platform/v1` | platform/平台管理 | L2 | 系统配置/租户/监控 |
| 25 | `/platform/iam` | platform/IAM | L2 | 身份认证管理 |
| 26 | `/platform/meta` | platform/元数据 | L2 | 元数据管理 |
| 27 | `/platform/datastore` | platform/数据存储 | L2 | 存储抽象层 |
| 28 | `/observability/v1` | observability/可观测性 | L5 | 日志/指标/追踪 |
| 29 | `/metrics` | metrics/指标导出 | L5 | Prometheus 指标 |
| 30 | `/health` | health/健康检查 | L5 | liveness/readiness |
| 31 | `/domains` | domains/域发现 | L1 | 路由注册中心自描述 |

---

## 三、KG 域 6 接口详细规范（已就绪）

**基础路径**: `/kg/v1`

### 3.1 邻域子图查询

```
GET /kg/v1/neighborhood?center={node_id}&depth={k}&limit={n}
```

**参数**:
| 参数 | 类型 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| center | string | 是 | - | 中心节点 ID |
| depth | int | 否 | 1 | 跳数深度（1-3） |
| limit | int | 否 | 100 | 最大返回节点数 |

**响应**:
```json
{
  "ok": true,
  "data": {
    "nodes": [{"id": "P0", "label": "需求输入", "type": "phase"}],
    "edges": [{"source": "P0", "target": "P1", "label": "flows_to"}],
    "meta": {
      "hops": 1,
      "excluded": 0,
      "center": "P0"
    }
  }
}
```

**实现**: `KnowledgeGraph::neighborhood_subgraph(center, depth, limit)`

---

### 3.2 路径查找（K 条简单路径）

```
GET /kg/v1/path?src={source}&dst={target}&k={num_paths}
```

**参数**:
| 参数 | 类型 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| src | string | 是 | - | 源节点 ID |
| dst | string | 是 | - | 目标节点 ID |
| k | int | 否 | 3 | 返回路径数量（Yen's 算法） |

**响应**:
```json
{
  "ok": true,
  "data": {
    "paths": [
      {"nodes": ["P0", "P1", "P2"], "weight": 2.0},
      {"nodes": ["P0", "P5", "P2"], "weight": 3.0}
    ],
    "total_weight": 5.0,
    "avg_hops": 2.5,
    "k_requested": 3,
    "k_returned": 2
  }
}
```

**实现**: `KnowledgeGraph::find_paths(src, dst, k)`（Yen's 算法）

---

### 3.3 最短路径

```
GET /kg/v1/shortest-path?src={source}&dst={target}
```

**响应**:
```json
{
  "ok": true,
  "data": {
    "path": ["P0", "P1", "P2", "P3"],
    "distance": 3,
    "found": true
  }
}
```

**实现**: `KnowledgeGraph::shortest_path(src, dst)`（BFS 无权最短路）

---

### 3.4 中心性计算

```
GET /kg/v1/centrality?metric={metric}&top={n}
```

**参数**:
| 参数 | 类型 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| metric | string | 否 | all | pagerank / betweenness / harmonic / degree / all |
| top | int | 否 | 10 | 返回前 N 个节点 |

**响应**:
```json
{
  "ok": true,
  "data": {
    "metric": "all",
    "results": {
      "pagerank": [{"node": "P2", "score": 0.156}],
      "betweenness": [{"node": "P5", "score": 0.089}],
      "harmonic": [{"node": "P3", "score": 0.734}],
      "degree": [{"node": "P0", "score": 8}]
    },
    "formulas": {
      "pagerank": "PR(v) = (1-d)/N + d * Σ PR(u)/L(u)",
      "betweenness": "BC(v) = Σ σ(s→t|v) / σ(s→t)",
      "harmonic": "HC(v) = 1/(n-1) * Σ 1/d(v,u)",
      "degree": "DC(v) = deg(v) / (n-1)"
    }
  }
}
```

**实现**: `KnowledgeGraph::centrality_metrics()` + `CentralityMetrics` 结构体

---

### 3.5 社区检测（CNM）

```
GET /kg/v1/communities?algorithm=cnm
```

**响应**:
```json
{
  "ok": true,
  "data": {
    "algorithm": "cnm",
    "modularity": 0.523,
    "communities": [
      {"id": 0, "nodes": ["P0", "P1", "P2"], "size": 3},
      {"id": 1, "nodes": ["P5", "P6", "P7"], "size": 3}
    ],
    "note": "CNM 模块度贪心凝聚（项目红线：禁用 LPA）"
  }
}
```

**实现**: `KnowledgeGraph::detect_communities()`（CNM 模块度贪心凝聚）

---

### 3.6 图统计

```
GET /kg/v1/stats
```

**响应**:
```json
{
  "ok": true,
  "data": {
    "node_count": 25,
    "edge_count": 40,
    "density": 0.0667,
    "density_interpretation": "稀疏图（density < 0.2）：节点间连接松散，适合邻域扩展查询",
    "avg_degree": 3.2,
    "clustering_coefficient": 0.345,
    "connected_components": 1,
    "centrality_formulas": {
      "pagerank": {"tex": "PR(v) = \\frac{1-d}{N} + d\\sum_{u \\in In(v)}\\frac{PR(u)}{L(u)}", "intuition": "随机游走稳态概率"},
      "betweenness": {"tex": "BC(v) = \\sum_{s \\neq v \\neq t}\\frac{\\sigma(s\\to t|v)}{\\sigma(s\\to t)}", "intuition": "最短路径必经点比例"},
      "harmonic": {"tex": "HC(v) = \\frac{1}{n-1}\\sum_{u \\neq v}\\frac{1}{d(v,u)}", "intuition": "到所有节点距离倒数均值"},
      "degree": {"tex": "DC(v) = \\frac{deg(v)}{n-1}", "intuition": "直接连接数归一化"}
    }
  }
}
```

**实现**: `KnowledgeGraph::stats()` + `GraphStats` 结构体（含 density_interpretation 和 centrality_formulas）

---

## 四、AI 引擎 4 接口详细规范（已就绪）

**基础路径**: `/ai/engine`

### 4.1 意图处理（自动路由）

```
POST /ai/engine/process
```

**请求体**:
```json
{
  "input": "帮我分析一下这个项目的架构",
  "context": {"project_id": "proj-123", "user_id": "user-456"},
  "options": {"auto_route": true, "confidence_threshold": 0.6}
}
```

**响应**:
```json
{
  "ok": true,
  "data": {
    "intent": "analysis",
    "confidence": 0.87,
    "routed_capability": "architecture_analyzer",
    "all_candidates": [
      {"capability": "architecture_analyzer", "score": 0.87},
      {"capability": "code_reviewer", "score": 0.45}
    ],
    "trace_id": "trace-abc123"
  }
}
```

**实现**: `mox-ai-intent-core::classify_intent()` + `score_alliance_candidates()`

---

### 4.2 显式能力执行

```
POST /ai/engine/analyze
```

**请求体**:
```json
{
  "capability": "architecture_analyzer",
  "input": "...",
  "params": {"depth": "full", "output_format": "markdown"}
}
```

**响应**:
```json
{
  "ok": true,
  "data": {
    "capability": "architecture_analyzer",
    "result": "...",
    "scores": {
      "completeness": 0.85,
      "correctness": 0.92,
      "consistency": 0.78,
      "performance": 0.70,
      "security": 0.88,
      "maintainability": 0.82
    },
    "cem_score": 0.825
  }
}
```

**CEM 加权系数**: 完整性 40% + 正确性 40% + 一致性 20%（性能/安全/可维护性为辅助维度）

---

### 4.3 能力矩阵自描述

```
GET /ai/engine/capabilities
```

**响应**:
```json
{
  "ok": true,
  "data": {
    "capabilities": [
      {"id": "intent_classifier", "name": "意图分类", "domain": "ai", "version": "1.0"},
      {"id": "architecture_analyzer", "name": "架构分析", "domain": "enterprise", "version": "1.0"},
      {"id": "code_reviewer", "name": "代码评审", "domain": "development", "version": "1.0"},
      {"id": "test_generator", "name": "测试生成", "domain": "testing", "version": "1.0"},
      {"id": "doc_writer", "name": "文档撰写", "domain": "documentation", "version": "1.0"},
      {"id": "data_analyzer", "name": "数据分析", "domain": "data", "version": "1.0"},
      {"id": "kg_explorer", "name": "图谱探索", "domain": "kg", "version": "1.0"}
    ],
    "total": 7
  }
}
```

---

### 4.4 引擎指标

```
GET /ai/engine/metrics
```

**响应**:
```json
{
  "ok": true,
  "data": {
    "total_requests": 15234,
    "success_rate": 0.967,
    "degradation_rate": 0.028,
    "avg_latency_ms": 145,
    "p99_latency_ms": 890,
    "intent_accuracy": 0.912,
    "routing_success_rate": 0.945,
    "cem_weights": {
      "completeness": 0.4,
      "correctness": 0.4,
      "consistency": 0.2
    }
  }
}
```

---

## 五、通用端点

### 5.1 健康检查

```
GET /health
```

**响应**:
```json
{
  "ok": true,
  "gateway": "axum",
  "version": "3.0.0-ai-powered",
  "ts": "2026-08-27T10:00:00Z",
  "uptime_seconds": 3600
}
```

### 5.2 域发现

```
GET /domains
```

**响应**: 返回完整 31 域列表（prefix/name/status/owner/note），前端用于路由发现和动态菜单生成。

### 5.3 指标导出

```
GET /metrics
```

Prometheus 文本格式指标导出（由 mox-framework metrics 模块提供）。

---

## 六、错误响应规范

所有接口统一使用 mox-framework 的 `FrameworkError` → `IntoResponse` 转换：

```json
{
  "code": "KG4041",
  "message": "节点不存在: P99",
  "severity": "error",
  "trace_id": "trace-abc123",
  "timestamp": "2026-08-27T10:00:00Z",
  "details": {"node_id": "P99"},
  "reason": "node_not_found"
}
```

**错误码体系**:
| 前缀 | 范围 | 类型 |
|---|---|---|
| 4xxx | 4000-4999 | 客户端错误（参数/验证/NotFound） |
| 5xxx | 5000-5999 | 服务端错误（内部/超时/不可用） |
| 6xxx | 6000-6999 | 网关错误（限流/熔断/认证） |
| 7xxx | 7000-7999 | 存储错误（数据库/对象存储/图谱） |

---

*详见 [05-kg-algorithm-core.md](./05-kg-algorithm-core.md) 获取 KG 算法核心接口详解。*
