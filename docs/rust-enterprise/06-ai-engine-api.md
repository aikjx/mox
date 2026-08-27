# 06 · AI 引擎接口规范

> **版本**: v1.0 · **日期**: 2026-08-27
> **实现文件**: `platform/domains/ai/core/mox-ai-intent-core/src/lib.rs`
> **HTTP 适配**: `platform/domains/kg/svc/mox-kg-service-svc/src/http_adapter.rs` (feature=http-adapter)

## 一、AI 引擎架构

```
┌─────────────────────────────────────────────────────┐
│                   AI 引擎 (mox-ai-intent-core)       │
├─────────────────────────────────────────────────────┤
│                                                         │
│  [输入] ──→ 意图分类 (classify_intent)                 │
│              │                                          │
│              ├── 8 类意图识别                           │
│              ├── 置信度计算                             │
│              └── 上下文感知                             │
│                                                         │
│              ↓                                          │
│         能力路由 (score_alliance_candidates)           │
│              │                                          │
│              ├── 7 类基线能力匹配                       │
│              ├── 联盟候选打分                           │
│              └── 最优能力选择                           │
│                                                         │
│              ↓                                          │
│         激活扩散路由 (ActivationDiffusionRouter)       │
│              │                                          │
│              ├── 多跳能力激活                           │
│              ├── 衰减因子控制                           │
│              └── 路由路径追踪                           │
│                                                         │
│              ↓                                          │
│         [输出] 意图响应 + 能力执行 + CEM 指标           │
│                                                         │
└─────────────────────────────────────────────────────┘
```

---

## 二、核心数据结构

### IntentRequest

```rust
pub struct IntentRequest {
    pub input: String,
    pub context: Option<HashMap<String, String>>,
    pub options: IntentOptions,
}

pub struct IntentOptions {
    pub auto_route: bool,
    pub confidence_threshold: f64,
}
```

### IntentResponse

```rust
pub struct IntentResponse {
    pub intent: IntentType,
    pub confidence: f64,
    pub routed_capability: Option<String>,
    pub all_candidates: Vec<CapabilityScore>,
    pub trace_id: String,
}
```

### IntentType（8 类意图）

```rust
pub enum IntentType {
    Requirement,    // 需求输入
    Defect,         // 缺陷报告
    Consultation,   // 咨询问答
    Operation,      // 操作指令
    Approval,       // 审批请求
    Query,          // 查询检索
    Report,         // 报表生成
    Configuration,  // 配置变更
}
```

### Capability（7 类基线能力）

```rust
pub struct Capability {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub version: String,
    pub owners: CapabilityOwners,
}
```

| 能力 ID | 名称 | 域 |
|---|---|---|
| intent_classifier | 意图分类 | ai |
| architecture_analyzer | 架构分析 | enterprise |
| code_reviewer | 代码评审 | development |
| test_generator | 测试生成 | testing |
| doc_writer | 文档撰写 | documentation |
| data_analyzer | 数据分析 | data |
| kg_explorer | 图谱探索 | kg |

---

## 三、核心 API

### 3.1 classify_intent（意图分类）

```rust
pub fn classify_intent(request: &IntentRequest) -> Result<IntentResponse, IntentError>
```

**处理逻辑**:
1. 输入预处理（分词、去噪、归一化）
2. 特征提取（关键词、语义向量、上下文特征）
3. 8 类意图概率计算
4. 取最高概率作为意图，置信度为概率值
5. 若 `auto_route=true` 且置信度 ≥ `confidence_threshold`，自动调用能力路由

**返回**: IntentResponse（含意图类型、置信度、路由结果、所有候选）

---

### 3.2 score_alliance_candidates（联盟候选打分）

```rust
pub fn score_alliance_candidates(
    intent: &IntentType,
    context: &Option<HashMap<String, String>>,
) -> Vec<CapabilityScore>
```

**打分维度**:
| 维度 | 权重 | 说明 |
|---|---|---|
| 语义匹配度 | 0.4 | 输入与能力描述的语义相似度 |
| 域相关性 | 0.3 | 意图域与能力域的匹配程度 |
| 历史成功率 | 0.2 | 该能力处理同类意图的历史表现 |
| 上下文适配 | 0.1 | 当前上下文对能力的适配度 |

**返回**: 按分数降序排列的 CapabilityScore 列表

```rust
pub struct CapabilityScore {
    pub capability: String,
    pub score: f64,
    pub dimensions: ScoreDimensions,
}
```

---

### 3.3 ActivationDiffusionRouter（激活扩散路由）

```rust
pub struct ActivationDiffusionRouter {
    capabilities: HashMap<String, Capability>,
    activation_graph: HashMap<String, Vec<(String, f64)>>,
}

impl ActivationDiffusionRouter {
    pub fn new() -> Self
    pub fn register_capability(&mut self, cap: Capability)
    pub fn route(&self, start: &str, steps: usize, decay: f64) -> RouteResult
}
```

**算法**: 激活扩散（Activation Spread）
- 从起始能力出发，沿能力关联图扩散激活值
- 每跳衰减因子 $decay$（默认 0.85）
- 经过 $steps$ 跳后，激活值最高的节点为最终路由目标

**应用场景**: 多步推理、跨域能力编排、隐式需求发现

---

## 四、HTTP API 规范

**基础路径**: `/ai/engine`

### 4.1 POST /process（意图处理 + 自动路由）

**请求**:
```json
{
  "input": "帮我分析一下这个项目的架构是否合理",
  "context": {
    "project_id": "proj-123",
    "user_id": "user-456",
    "domain": "enterprise"
  },
  "options": {
    "auto_route": true,
    "confidence_threshold": 0.6
  }
}
```

**响应 200**:
```json
{
  "ok": true,
  "data": {
    "intent": "consultation",
    "confidence": 0.87,
    "routed_capability": "architecture_analyzer",
    "all_candidates": [
      {
        "capability": "architecture_analyzer",
        "score": 0.87,
        "dimensions": {
          "semantic": 0.92,
          "domain": 0.85,
          "history": 0.80,
          "context": 0.90
        }
      },
      {
        "capability": "code_reviewer",
        "score": 0.45,
        "dimensions": {
          "semantic": 0.50,
          "domain": 0.40,
          "history": 0.45,
          "context": 0.45
        }
      }
    ],
    "trace_id": "trace-7f3a2b1c"
  }
}
```

**错误响应 400**:
```json
{
  "ok": false,
  "code": "AI4001",
  "message": "输入不能为空",
  "trace_id": "trace-..."
}
```

---

### 4.2 POST /analyze（显式能力执行）

**请求**:
```json
{
  "capability": "architecture_analyzer",
  "input": "系统采用微服务架构，共12个服务...",
  "params": {
    "depth": "full",
    "output_format": "markdown",
    "checklist": ["分层合理性", "域边界清晰度", "依赖方向", "可观测性"]
  }
}
```

**响应 200**:
```json
{
  "ok": true,
  "data": {
    "capability": "architecture_analyzer",
    "result": "# 架构分析报告\n\n## 一、分层合理性...",
    "scores": {
      "completeness": 0.85,
      "correctness": 0.92,
      "consistency": 0.78,
      "performance": 0.70,
      "security": 0.88,
      "maintainability": 0.82
    },
    "cem_score": 0.825,
    "trace_id": "trace-9d4e5f6a"
  }
}
```

**CEM 加权公式**:
$$CEM = 0.4 \times C_{completeness} + 0.4 \times C_{correctness} + 0.2 \times C_{consistency}$$

> 性能/安全/可维护性为辅助参考维度，不纳入 CEM 主指标计算。

---

### 4.3 GET /capabilities（能力矩阵自描述）

**响应 200**:
```json
{
  "ok": true,
  "data": {
    "capabilities": [
      {
        "id": "intent_classifier",
        "name": "意图分类",
        "domain": "ai",
        "version": "1.0",
        "description": "对用户输入进行8类意图识别，输出意图类型和置信度"
      },
      {
        "id": "architecture_analyzer",
        "name": "架构分析",
        "domain": "enterprise",
        "version": "1.0",
        "description": "分析系统架构的分层合理性、域边界、依赖方向等"
      },
      {
        "id": "code_reviewer",
        "name": "代码评审",
        "domain": "development",
        "version": "1.0",
        "description": "代码质量评审，涵盖安全、性能、可维护性等维度"
      },
      {
        "id": "test_generator",
        "name": "测试生成",
        "domain": "testing",
        "version": "1.0",
        "description": "自动生成单元测试、集成测试用例"
      },
      {
        "id": "doc_writer",
        "name": "文档撰写",
        "domain": "documentation",
        "version": "1.0",
        "description": "自动生成技术文档、API文档、设计文档"
      },
      {
        "id": "data_analyzer",
        "name": "数据分析",
        "domain": "data",
        "version": "1.0",
        "description": "数据分析、统计、可视化建议"
      },
      {
        "id": "kg_explorer",
        "name": "图谱探索",
        "domain": "kg",
        "version": "1.0",
        "description": "知识图谱查询、邻域扩展、路径分析、社区检测"
      }
    ],
    "total": 7
  }
}
```

---

### 4.4 GET /metrics（引擎指标）

**响应 200**:
```json
{
  "ok": true,
  "data": {
    "total_requests": 15234,
    "success_rate": 0.967,
    "degradation_rate": 0.028,
    "avg_latency_ms": 145,
    "p50_latency_ms": 85,
    "p95_latency_ms": 320,
    "p99_latency_ms": 890,
    "intent_accuracy": 0.912,
    "routing_success_rate": 0.945,
    "cem_weights": {
      "completeness": 0.4,
      "correctness": 0.4,
      "consistency": 0.2
    },
    "intent_distribution": {
      "requirement": 0.25,
      "defect": 0.15,
      "consultation": 0.30,
      "operation": 0.10,
      "approval": 0.05,
      "query": 0.10,
      "report": 0.03,
      "configuration": 0.02
    }
  }
}
```

---

## 五、错误码

| 错误码 | HTTP 状态 | 说明 |
|---|---|---|
| AI4001 | 400 | 输入不能为空 |
| AI4002 | 400 | 无效的意图类型 |
| AI4003 | 400 | 置信度阈值超出范围 [0, 1] |
| AI4041 | 404 | 能力不存在 |
| AI4042 | 404 | 无可用候选能力 |
| AI5001 | 500 | 意图分类内部错误 |
| AI5002 | 500 | 能力路由内部错误 |
| AI5003 | 500 | 激活扩散路由失败 |

---

## 六、使用示例

### Rust 直接调用

```rust
use mox_ai_intent_core::{classify_intent, IntentRequest, IntentOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let request = IntentRequest {
        input: "帮我分析一下这个项目的架构".to_string(),
        context: Some([("project_id".to_string(), "proj-123".to_string())].into()),
        options: IntentOptions {
            auto_route: true,
            confidence_threshold: 0.6,
        },
    };

    let response = classify_intent(&request)?;
    println!("意图: {:?}, 置信度: {}", response.intent, response.confidence);
    println!("路由能力: {:?}", response.routed_capability);

    Ok(())
}
```

### HTTP 调用

```bash
# 意图处理
curl -X POST http://localhost:8080/ai/engine/process \
  -H "Content-Type: application/json" \
  -d '{"input":"分析架构","options":{"auto_route":true}}'

# 能力矩阵
curl http://localhost:8080/ai/engine/capabilities

# 引擎指标
curl http://localhost:8080/ai/engine/metrics
```

---

*详见 [04-api-gateway-routes.md](./04-api-gateway-routes.md) 获取完整 31 域 API 规范。*
