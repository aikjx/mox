# infotopograph 归一化架构规范 v1.0

> 基于算法验证结果（48 crate / 107 依赖边 / 0 循环 / 1 God Module / 1 层违规 / 24 跨域依赖）
>
> 目标：所有模块与功能明确、独立、关联明确、归一化、企业级

---

## 一、归一化命名规范

### 1.1 crate 命名公式

```
mox-<domain>-<layer>-<role>
```

| 段 | 取值 | 说明 |
|----|------|------|
| `<domain>` | `kg` / `ai` / `flow` / `data` / `cloud` / `voice` / `platform` / `market` | 业务域 |
| `<layer>` | `api` / `svcapi` / `core` / `svc` / `sdk` | 架构层 |
| `<role>` | `storage` / `service` / `engine` / `hub` / `agent` / `registry` | 角色 |

### 1.2 层级定义

| 层级 | 后缀 | 职责 | 依赖方向 |
|------|------|------|----------|
| `api` | `-api` | 对外 DTO + REST/JSON-RPC 接口契约 | 零内部依赖 |
| `svcapi` | `-svcapi` | 服务间 gRPC 契约（.proto + stub） | 仅依赖 api |
| `core` | `-core` | 核心计算/算法引擎，无 IO | 仅依赖 foundation |
| `svc` | `-svc` | 业务服务实现 | 依赖 api + svcapi + core |
| `sdk` | `-sdk` | 客户端 SDK / FFI 绑定 | 依赖 api |

### 1.3 业务域定义

| 域代码 | 域名 | 包含能力 |
|--------|------|----------|
| `kg` | 知识图谱 | 图存储/图服务/图谱Hub/图算法/图流/图元数据 |
| `ai` | AI智能 | AI Agent/AI核心/专家/流程AI/意图 |
| `flow` | 流程自动化 | 算子核心/算子WASM/优化器/PrimiFlow核心/融合 |
| `data` | 数据治理 | 数据平面/ETL/合规/标准/归一化/公式/业务目录 |
| `cloud` | 云存储 | 云盘主/卷/S3/文件器/域抽象 |
| `voice` | 语音 | 小白核心/ASR/意图/算子/桌面/DSP |
| `platform` | 平台基础 | 公共元数据/系统/服务器/runtime/测试框架 |
| `market` | 市场 | 模板市场 |

---

## 二、48 Crate 归一化映射表

### 2.1 平台基础域 (platform)

| 当前名 | 归一化名 | 层 | 职责 | 状态 |
|--------|----------|----|------|------|
| `mox-common-meta` | `mox-platform-foundation` | foundation | 公共元数据/类型定义 | 保留(重命名) |
| `mox-domain-abstractions` | `mox-cloud-foundation` | foundation | 云存储域抽象 | 保留(重命名+移域) |
| `mox-system` | `mox-platform-system-svc` | svc | 用户/角色/权限/菜单/审计 | 保留(重命名) |
| `mox-server` | `mox-platform-gateway-svc` | svc | 单体服务器/网关入口 | 拆分 |
| `runtime` | `mox-platform-runtime-svc` | application | 运行时编排 | God Module 拆分 |
| `mox-t21-harness` | `mox-platform-test-harness` | sdk | 测试框架 | 保留(重命名) |

### 2.2 知识图谱域 (kg)

| 当前名 | 归一化名 | 层 | 职责 | 状态 |
|--------|----------|----|------|------|
| `mox-graph-meta` | `mox-kg-meta-core` | core | 图元数据/类型系统 | 保留 |
| `mox-graph-storage` | `mox-kg-storage-svc` | svc | 自研分布式图存储(RocksDB+Raft) | 保留(核心资产) |
| `mox-graph-service` | `mox-kg-service-svc` | svc | 图查询/遍历/CRUD服务 | 保留 |
| `mox-graph-streams` | `mox-kg-streams-svc` | svc | 图变更流/CDC | 保留 |
| `mox-graph-spark` | `mox-kg-spark-svc` | svc | 图Spark计算 | 评估(是否需要) |
| `graph-algorithms` | `mox-kg-algo-core` | core | 图算法(PageRank/最短路径/社区) | 保留(移core层) |
| `kg-hub` | `mox-kg-hub-svc` | svc | 图谱Hub(本体/推理/摄入/索引/治理) | 保留 |
| `mox-fusion` | `mox-kg-fusion-svc` | svc | 知识融合/实体对齐 | 保留(移kg域) |

### 2.3 AI智能域 (ai)

| 当前名 | 归一化名 | 层 | 职责 | 状态 |
|--------|----------|----|------|------|
| `mox-ai-core` | `mox-ai-core` | core | AI核心类型/接口 | 保留 |
| `mox-intent-core` | `mox-ai-intent-core` | core | 意图识别核心 | 保留(移ai域) |
| `flow-ai` | `mox-ai-flow-svc` | svc | AI流程编排 | 保留(移ai域) |
| `mox-expert` | `mox-ai-expert-svc` | svc | 专家服务/专家注册 | 保留(移ai域) |
| `ai-agent` | `mox-ai-agent-svc` | svc | AI Agent运行时/ReAct循环 | 保留(移ai域) |

### 2.4 流程自动化域 (flow)

| 当前名 | 归一化名 | 层 | 职责 | 状态 |
|--------|----------|----|------|------|
| `operator-core` | `mox-flow-operator-core` | core | 算子核心/算子接口 | 保留(移core层) |
| `operator-wasm` | `mox-flow-operator-wasm-svc` | svc | WASM算子运行时 | 保留 |
| `optimizer` | `mox-flow-optimizer-core` | core | 流程优化器/DAG优化 | 保留(移core层) |
| `primiflow-core` | `mox-flow-primiflow-svc` | svc | PrimiFlow核心引擎 | 保留 |
| `primiflow-fusion` | `mox-flow-fusion-svc` | svc | 流程融合/数据流融合 | 保留 |
| `hermes-flow-bridge` | `mox-flow-bridge-svc` | svc | 流程桥接/外部系统对接 | 保留 |

### 2.5 数据治理域 (data)

| 当前名 | 归一化名 | 层 | 职责 | 状态 |
|--------|----------|----|------|------|
| `mox-formulas-core` | `mox-data-formula-core` | core | 公式引擎核心 | 保留(移core层) |
| `mox-norm-core` | `mox-data-norm-core` | core | 数据归一化核心 | 保留(移core层) |
| `mox-standards` | `mox-data-standards-svc` | svc | 数据标准/规范 | 保留(移data域) |
| `mox-data-plane` | `mox-data-plane-svc` | svc | 数据平面/数据接入 | 保留 |
| `mox-etl-wasm` | `mox-data-etl-svc` | svc | ETL WASM运行时 | 保留 |
| `mox-compliance` | `mox-data-compliance-svc` | svc | 合规/审计/数据治理 | 保留 |
| `business-catalog` | `mox-data-catalog-svc` | svc | 业务目录/数据目录 | 保留(移data域) |

### 2.6 云存储域 (cloud)

| 当前名 | 归一化名 | 层 | 职责 | 状态 |
|--------|----------|----|------|------|
| `mox-cloud-drive-master` | `mox-cloud-master-svc` | svc | 云盘主控/元数据管理 | 保留 |
| `mox-cloud-drive-volume` | `mox-cloud-volume-svc` | svc | 云盘卷/块存储 | 保留 |
| `mox-cloud-drive-s3` | `mox-cloud-s3-svc` | svc | S3兼容对象存储 | 保留 |
| `mox-cloud-drive-filer` | `mox-cloud-filer-svc` | svc | 文件器/文件管理 | 保留 |

### 2.7 语音域 (voice)

| 当前名 | 归一化名 | 层 | 职责 | 状态 |
|--------|----------|----|------|------|
| `xiaobai-dsp` | `mox-voice-dsp-core` | core | 数字信号处理核心 | 保留(移core层) |
| `xiaobai-core` | `mox-voice-core-svc` | svc | 语音核心/会话管理 | 保留 |
| `xiaobai-asr` | `mox-voice-asr-svc` | svc | 语音识别 | 保留 |
| `xiaobai-intent` | `mox-voice-intent-svc` | svc | 语音意图理解 | 保留 |
| `xiaobai-operators` | `mox-voice-operator-svc` | svc | 语音算子/桌面操作 | 保留 |
| `xiaobai-desktop` | `mox-voice-desktop-app` | application | 桌面客户端 | 保留 |
| `xiaobai-dsp-py` | `mox-voice-dsp-py` | sdk | Python DSP绑定 | 保留 |

### 2.8 市场域 + SDK域

| 当前名 | 归一化名 | 域 | 层 | 状态 |
|--------|----------|----|----|------|
| `template-market` | `mox-market-template-svc` | market | svc | 保留 |
| `mox-sdk-cloud` | `mox-cloud-sdk` | cloud | sdk | 保留 |
| `mox-sdk-graph` | `mox-kg-sdk` | kg | sdk | 保留 |
| `mox-formulas-native` | `mox-data-formula-native` | data | sdk | 保留 |
| `mox-norm-intent-native` | `mox-data-norm-intent-native` | data | sdk | 保留 |

### 2.9 待拆分 God Module

| 当前名 | 问题 | 拆分方案 |
|--------|------|----------|
| `runtime` | 扇出15，God Module | 拆为 `mox-platform-orchestrator-svc`(编排) + 各业务域自行启动 |
| `mox-server` | 扇出7，单体入口 | 拆为 `mox-platform-gateway-svc`(网关) + 各服务独立部署 |

---

## 三、功能边界归一化

### 3.1 核心原则

1. **单一职责**：每个crate只负责一个明确的功能域
2. **零循环依赖**：已验证，保持
3. **层依赖单向**：foundation ← core ← engine ← svc ← application
4. **跨域通过平台层**：业务域之间不直接依赖，通过 platform 层的事件/接口中转
5. **接口先行**：每个svc必须有对应的api + svcapi层

### 3.2 功能边界矩阵（核心服务）

| 功能 | 负责crate(归一化) | 不负责 | 依赖 |
|------|-------------------|--------|------|
| 图存储 | `mox-kg-storage-svc` | 图查询/图算法 | foundation |
| 图查询 | `mox-kg-service-svc` | 图存储/图算法 | storage-svc + algo-core |
| 图算法 | `mox-kg-algo-core` | 图存储/图查询 | foundation (纯计算) |
| 图谱Hub | `mox-kg-hub-svc` | 图存储/图算法 | storage-svc + service-svc |
| AI Agent | `mox-ai-agent-svc` | 专家注册/图存储 | expert-svc + kg-sdk + flow-sdk |
| 专家服务 | `mox-ai-expert-svc` | Agent运行时 | foundation |
| 算子核心 | `mox-flow-operator-core` | WASM运行时 | foundation (纯计算) |
| 流程优化 | `mox-flow-optimizer-core` | 流程执行 | operator-core + algo-core |
| 公式引擎 | `mox-data-formula-core` | 数据归一化 | foundation (纯计算) |
| 数据归一化 | `mox-data-norm-core` | 公式计算 | foundation (纯计算) |
| 云盘主控 | `mox-cloud-master-svc` | 块存储/S3 | volume-svc + foundation |
| 语音核心 | `mox-voice-core-svc` | ASR/意图/DSP | asr-svc + intent-svc + dsp-core |

---

## 四、关联关系归一化

### 4.1 允许的依赖方向

```
application → svc → core → foundation
     ↓          ↓
   sdk       svcapi → api
```

### 4.2 禁止的依赖

| 禁止类型 | 说明 | 当前违规 |
|----------|------|----------|
| 循环依赖 | A→B→A | 无 |
| 层违规 | 底层依赖顶层 | ai-agent→template-market |
| 跨域直连 | 业务域A直接依赖业务域B | 24个(需通过platform中转) |
| God Module | 单模块依赖>10 | runtime(15) |

### 4.3 跨域依赖治理方案

当前24个跨域依赖，治理为3类：

**A类：合理跨域（保留）** — 平台型服务依赖多域
- `mox-platform-runtime-svc` → 各域（编排器天然依赖多域）
- `mox-platform-gateway-svc` → 各域（网关天然路由多域）

**B类：通过SDK解耦（改造）** — 改为依赖对方的SDK层
- `ai-agent` → `kg-hub` → 改为 `ai-agent` → `mox-kg-sdk`
- `ai-agent` → `graph-algorithms` → 改为 `ai-agent` → `mox-kg-sdk`
- `xiaobai-core` → `mox-expert` → 改为 `xiaobai-core` → `mox-ai-sdk`
- `kg-hub` → `primiflow-fusion` → 改为事件驱动(NATS)

**C类：通过事件解耦（改造）** — 改为异步事件
- `business-catalog` → `flow-ai` / `mox-expert` → 事件驱动
- `hermes-flow-bridge` → `flow-ai` / `mox-expert` → 事件驱动

---

## 五、最优架构目标态

### 5.1 目标架构（6层 + 8域）

```
L6 Application:  voice-desktop-app / platform-gateway-svc / platform-runtime-svc
L5 Service:      kg(8) ai(5) flow(6) data(7) cloud(5) voice(7) market(1) platform(3)
L4 SDK:          kg-sdk / cloud-sdk / ai-sdk / flow-sdk / native绑定
L3 Core:         kg-algo / ai-intent / flow-operator / flow-optimizer / data-formula / data-norm / voice-dsp / ai-core
L2 Service API:  每个svc对应一个 -svcapi crate (gRPC .proto + stub)
L1 API:          每个svc对应一个 -api crate (DTO + REST/JSON-RPC)
L0 Foundation:   platform-foundation / cloud-foundation
```

### 5.2 目标指标

| 指标 | 当前 | 目标 |
|------|------|------|
| 循环依赖 | 0 | 0 |
| God Module(扇出>10) | 1 | 0 |
| 层违规 | 1 | 0 |
| 跨域直连 | 24 | <5(仅平台层) |
| 三层分离覆盖率 | 0% | 100% |
| 模块独立性均分 | ~75 | >85 |
| core层纯计算率 | ~60% | 100% |

---

## 六、迁移路径（渐进式，不破坏现有功能）

| 阶段 | 周期 | 内容 |
|------|------|------|
| 1 归一化命名 | 1周 | 所有crate重命名为规范格式，保持功能不变 |
| 2 拆分God Module | 2周 | 拆分runtime/mox-server，引入mox-dualrpc通信底座 |
| 3 三层分离 | 4周 | 每个svc拆为-api + -svcapi + -svc，优先核心服务 |
| 4 跨域解耦 | 3周 | B类改SDK依赖，C类改NATS事件驱动 |
| 5 core层纯化 | 2周 | core层移除IO依赖，纯计算可独立测试 |
| 6 企业级加固 | 持续 | mox-framework基础框架 + 架构约束CI测试 + 99.95% SLA |

---

*关联文档：[架构审计报告](../architecture-audit-report.txt) | [算法指标数据](../architecture-metrics.json)*
