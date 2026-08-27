# MOX 文档索引总图 v1.0

**最后更新：** 2026-08-27
**配套规则：** ADR-006 once-defined（每个概念仅在一份主文档定义，其他文档只引用编号）
**阅读入口顺序建议：** I-1 总纲 → I-2 ADR → I-3 迁移交接 → 按角色选择分册

---

## 0. 文档分层（5 类 × 11 份正本）

```
L0 战略总纲层         1 份  归一化总纲（MOX-Enterprise-Unified-Spec-v2.0.md）
L1 架构决策层         1 份  ADR 决策记录
L2 项目管理层         2 份  12 周交付计划 · 迁移交接清单
L3 运维操作层         5 份  切换 SOP · HA容量 · 信创矩阵 · 运维手册 · 看板JSON
L4 文档索引层         1 份  本文件（DOCUMENT-INDEX.md）
L5 SQL/配置脚本层     1 份  图谱 DDL 脚本
──────────────────────────────────────────────────────────────
合计                  11 份（deploy/docs/ 下 10 + deploy/sql/ 下 1）
```

---

## I. deploy/docs/ 总文档 10 份（阅读入口）

| # | 编号 | 文件名 | 角色 | 核心内容 | 定义的唯一概念 |
|---|---|---|---|---|---|
| 1 | **总纲** | [MOX-Enterprise-Unified-Spec-v2.0.md](./MOX-Enterprise-Unified-Spec-v2.0.md) | 架构师 / 全体开发 | 8 章 + 附录 A-F（代码锚点/子规范/接管说明/迁移矩阵/缺口清单/去重决策） | 6 层架构定义 · T0-T3 阈值 · 三大铁律 · Schema 25 实体/40 关系 · 11 项图谱算法红线 · 28 项验收门禁 |
| 2 | B-09 | [MOX-Architecture-Decision-Records-v1.0.md](./MOX-Architecture-Decision-Records-v1.0.md) | 架构委员会 | ADR-001 ~ ADR-006 + 待办 4 提案 | **不可逆决策**：纯 Rust 技术栈 · 双架构迁入路线 · 分层+跨域API规则 · 单二进制 8080 · 数学红线 · once-defined 文档规则 |
| 3 | B-10 | [MOX-NodeToRust-Migration-Handover-v1.0.md](./MOX-NodeToRust-Migration-Handover-v1.0.md) | 后端 Lead / 新接手开发 | 5 项交接证据 · 32 模块逐行矩阵 · P0-P3 20 项缺口 · Week 1-4 甘特 · 遗留物/回滚策略 | 迁移覆盖度 23% 唯一定义 · P0-1~P3-4 缺口编号 |
| 4 | B-08 | [MOX-Fullstack-Auto-Delivery-Plan-v2.0.md](./MOX-Fullstack-Auto-Delivery-Plan-v2.0.md) | PM / 交付经理 | 12 周全自动开发执行计划 · P0-P12 Gate · 回滚机制 · 报告模板 | W1-W12 阶段周期（唯一排期）· 各 Gate 退出验收 |
| 5 | B-01 | [FS-S3-full-lifecycle-ops-guide.md](./FS-S3-full-lifecycle-ops-guide.md) | SRE / DBA | 图谱 5 步切换 · 云盘 4 步切换 · 9 日上云节奏 · 回滚流程 · F1-F14 Runbook · 巡检 | Key 同构规则（FS↔S3 零改）· 双写对账窗口 ≥7 天 · 一键回滚链 |
| 6 | B-04 | [ha-capacity-tco.md](./ha-capacity-tco.md) | SRE / FinOps / 采购 | T0-T3 多活拓扑 · 容量规划公式 · TCO 计算基线 ¥0.035/GB·月 | T0-T3 阈值数值 |
| 7 | B-05 | [xinchuang-matrix.md](./xinchuang-matrix.md) | 合规 / 基础设施 | 信创 OS/DB/CPU/加密库适配矩阵 · 国密 SM2/SM3/SM4 列表 | 信创兼容白名单（唯一真源） |
| 8 | B-06 | [ops-manual.md](./ops-manual.md) | SRE on-call | 日常巡检脚本 · 备份恢复命令 · 变更窗口日历 | L1-L4 巡检内容 · 备份 RPO/RTO |
| 9 | B-07 | [trace-8stages-dashboard.json](./trace-8stages-dashboard.json) | SRE / 可观测性 | Grafana Dashboard JSON 模板 · 8 阶段端到端链路看板 | 8 个阶段 Trace 指标定义 |
| 10 |（脚本）| [deploy/sql/mox-step1-graph-edges.sql](../sql/mox-step1-graph-edges.sql) | DBA | graph_edges 表 SQLite/PG 双方言幂等 DDL · 4 索引 · tombstone | 图谱关系表 Schema（唯一真源）|

---

## II. crate 级自描述文档索引（45 份，保留不合并）

> 说明：Rust 生态每个 crate 的 README/DESIGN/tasks.md 是 crate 的局部 API 契约，**不定义全局概念**，仅描述 crate 自身用法。任何全局规则请用「详见总纲 §x / ADR-xxx」格式引用，避免 once-defined 违规。以下按 6 层架构分组。

### L0 · foundation 基础层（2 份）

| crate 路径 | README | 内容摘要 |
|---|---|---|
| `platform/foundation/mox-platform-foundation/` | ✅ [README.md](../platform/foundation/mox-platform-foundation/README.md) | 基础类型定义 · 跨域共享数据结构 |
| `platform/foundation/mox-cloud-foundation/` | ✅ [README.md](../platform/foundation/mox-cloud-foundation/README.md) | 云存储通用对象/卷/快照 Trait · STS/SM3/WORM |

### L1 · gateway 网关层（无 README，入口在总纲 §C）

| crate 路径 | README | 关键代码（见总纲附录 A8-A10） |
|---|---|---|
| `platform/gateway/mox-platform-gateway-svc/` | ❌（见总纲附录 C） | [build_gateway_router](../platform/gateway/mox-platform-gateway-svc/src/lib.rs#L23-L51) · [main.rs](../platform/gateway/mox-platform-gateway-svc/src/main.rs) · [routes.rs](../platform/gateway/mox-platform-gateway-svc/src/routes.rs) |

### L2 · api 纯契约层（8 份，全 README ❌ 空壳 trait）

| 域 | crate 路径 |
|---|---|
| AI | `platform/domains/ai/api/` |
| Cloud | `platform/domains/cloud/api/` |
| Data | `platform/domains/data/api/` |
| Flow | `platform/domains/flow/api/` |
| KG | `platform/domains/kg/api/` |
|（8 域中 5 域已建空壳 api crate）| 待新增：Voice / Market / Platform 的 api/ crate |

### L3 · core 算法/领域内核（19 份 · README 覆盖率 63%）

| 域 | crate 路径 | README | 文档内容 |
|---|---|---|---|
| **KG（4/4）** | | | |
| | `platform/domains/kg/core/mox-kg-algo-core/` | ✅ [README.md](../platform/domains/kg/core/mox-kg-algo-core/README.md) | Brandes/harmonic/CNM/PageRank 算法说明 · 11 项数学红线 |
| | `platform/domains/kg/core/mox-kg-meta-core/` | ✅ [README.md](../platform/domains/kg/core/mox-kg-meta-core/README.md) + [tasks.md](../platform/domains/kg/core/mox-kg-meta-core/tasks.md) | 元数据 Schema 管理 · 待办清单 |
| **Platform（7/7）** | | | |
| | `platform/domains/platform/core/mox-platform-iam-core/` | ❌ | IAM 鉴权实现 |
| | `platform/domains/platform/core/mox-platform-meta-core/` | ❌ | Meta 字段槽分配器 |
| | `platform/domains/platform/core/mox-platform-datastore-core/` | ❌ | 通用数据存储 + slot 分配 |
| | `platform/domains/platform/core/mox-platform-orchestrator-core/` | ❌ | 编排核心 |
| | `platform/domains/platform/core/mox-platform-operator-core/` | ❌ | Operator 核心 |
| | `platform/domains/platform/core/mox-platform-system-core/` | ✅ [README.md](../platform/domains/platform/core/mox-platform-system-core/README.md) | 系统配置 · 路径管理 |
| | `platform/domains/platform/core/mox-connector-core/` | ❌ | 连接器 |
| | `platform/domains/platform/core/mox-plugin-core/` | ❌ | 插件系统 |
| | `platform/domains/platform/core/mox-enterprise-core/` | ❌ | 企业核心 |
| **AI（2/2）** | | | |
| | `platform/domains/ai/core/mox-ai-core/` | ❌（代码有 4 Provider Traits：anthropic/openai/qwen/dto） | LLM Provider 抽象 · Chat/Graph/Reasoning/Registry/Router |
| | `platform/domains/ai/core/mox-ai-intent-core/` | ❌（但 ADR-005 + 总纲附录 A19 引用） | 意图分类 + 专家打分 |
| **Flow（2/2）** | | | |
| | `platform/domains/flow/core/mox-flow-operator-core/` | ✅ [README.md](../platform/domains/flow/core/mox-flow-operator-core/README.md) | Operator 分类/守恒/核/扩展 |
| | `platform/domains/flow/core/mox-flow-optimizer-core/` | ✅ [README.md](../platform/domains/flow/core/mox-flow-optimizer-core/README.md) | 优化器（CEM 算法待实现） |
| **Data（3/3）** | | | |
| | `platform/domains/data/core/mox-data-formula-core/` | ❌ | 公式库（中心性/社区/PageRank/stats） |
| | `platform/domains/data/core/mox-data-norm-core/` | ❌ | 规范化（去重/合并/规则） |
| | `platform/domains/data/core/mox-data-standards-core/` | ✅ [README.md](../platform/domains/data/core/mox-data-standards-core/README.md) + [tasks.md](../platform/domains/data/core/mox-data-standards-core/tasks.md) | 标准库 CRC32C/FIPS_HMAC/SigV4/SM2/SM3/SM4/STS_SM2/RFC5424 |
| **Voice（1/1）** | | | |
| | `platform/domains/voice/core/mox-voice-dsp-core/` | ❌ | 数字信号处理核心 |
| **L3 core README 覆盖率** | | **12/19 = 63%** | 待补：Platform 5 + AI 2 + Data 2 + Voice 1 |

### L4 · svc 业务服务层（29 份 · README 覆盖率 83%）

| 域 | crate 路径 | README / 额外文档 | 内容摘要 |
|---|---|---|---|
| **KG（6/6）** | | | |
| | `platform/domains/kg/svc/mox-kg-algo-core/` → 见 L3 | ✅ | （归入 core）|
| | `platform/domains/kg/svc/mox-kg-storage-svc/` | ✅ [README.md](../platform/domains/kg/svc/mox-kg-storage-svc/README.md) | 图谱存储 SQLite/PG/Memory/Dual 多引擎 |
| | `platform/domains/kg/svc/mox-kg-service-svc/` | ❌（**含 http_adapter.rs 关键代码**，见总纲附录 A11） | KG/AI HTTP 适配层（唯一已就绪的业务 HTTP 层） |
| | `platform/domains/kg/svc/mox-kg-streams-svc/` | ❌ | 图谱流处理 |
| | `platform/domains/kg/svc/mox-kg-spark-svc/` | ❌ | Spark 对接 |
| | `platform/domains/kg/svc/mox-kg-fusion-svc/` | ❌（代码有 canonical 融合） | 实体融合/别名 |
| | `platform/domains/kg/svc/mox-kg-hub-svc/` | ✅ [README.md](../platform/domains/kg/svc/mox-kg-hub-svc/README.md) | KG Hub 中枢 |
| **AI（3/3）** | | | |
| | `platform/domains/ai/svc/mox-ai-agent-svc/` | ✅ [README.md](../platform/domains/ai/svc/mox-ai-agent-svc/README.md) | Agent 引擎：多 agent/状态机/工具/对话图/工作流/caomei_e2e 测试 |
| | `platform/domains/ai/svc/mox-ai-expert-svc/` | ✅ [README.md](../platform/domains/ai/svc/mox-ai-expert-svc/README.md) + [DESIGN.md](../platform/domains/ai/svc/mox-ai-expert-svc/DESIGN.md) + [DESIGN_STAGE2.md](../platform/domains/ai/svc/mox-ai-expert-svc/DESIGN_STAGE2.md) | **最复杂大模块**（80% 已实现）：Alliance/Audit/Domain/Experts/FlowLoader/RBAC/Verify 7 模块 · 专家评分 + 辩论合成 + CEM 优化 + 9 份 tests/benches |
| | `platform/domains/ai/svc/mox-ai-flow-svc/` | ✅ [README.md](../platform/domains/ai/svc/mox-ai-flow-svc/README.md) + [artifact.md](../platform/domains/ai/svc/mox-ai-flow-svc/src/bin/flowopt.rs.artifact.md) | 流程自动化：代码生成/冲突/关键路径/数据流/管道/调度/拓扑 + flowopt bin |
| **Cloud（4/4）** | | | |
| | `platform/domains/cloud/svc/mox-cloud-master-svc/` | ✅ [README.md](../platform/domains/cloud/svc/mox-cloud-master-svc/README.md) | 云盘 Master：卷分配/副本/快照 |
| | `platform/domains/cloud/svc/mox-cloud-s3-svc/` | ❌（S3 Server/sigv4/acl/cors/版本/lifecycle/glacier/mpu 全实现） | S3 兼容协议层 |
| | `platform/domains/cloud/svc/mox-cloud-volume-svc/` | ✅ [README.md](../platform/domains/cloud/svc/mox-cloud-volume-svc/README.md) | 卷存储：Chunk 重建/GF256/ReedSolomon/Manifest/指标 |
| | `platform/domains/cloud/svc/mox-cloud-filer-svc/` | ❌ | POSIX Filer：FUSE 客户端 + Meta SQLite/Redis/PG 分布式 |
| **Data（4/4）** | | | |
| | `platform/domains/data/svc/mox-data-catalog-svc/` | ✅ [README.md](../platform/domains/data/svc/mox-data-catalog-svc/README.md) | 数据目录 + 螺旋式索引 + catalog bin |
| | `platform/domains/data/svc/mox-data-compliance-svc/` | ❌ | 合规：审计记录 + Legal Hold + 密级 |
| | `platform/domains/data/svc/mox-data-etl-svc/` | ❌ | ETL 管道 · ABI 绑定 |
| | `platform/domains/data/svc/mox-data-plane-svc/` | ❌ | 数据平面：FSHC/多部分/监听/挂载 |
| **Flow（5/5）** | | | |
| | `platform/domains/flow/svc/mox-flow-operator-core/` → L3 | ✅ | （归入 core）|
| | `platform/domains/flow/svc/mox-flow-bridge-svc/` | ✅ [README.md](../platform/domains/flow/svc/mox-flow-bridge-svc/README.md) + [DESIGN.md](../platform/domains/flow/svc/mox-flow-bridge-svc/DESIGN.md) | Bridge：Hermes mini / 直播 / 规范化 / 插件 / 记录 + plugin.yaml + tests |
| | `platform/domains/flow/svc/mox-flow-fusion-svc/` | ✅ [README.md](../platform/domains/flow/svc/mox-flow-fusion-svc/README.md) + 11 份 [fusion_docs/](../platform/domains/flow/svc/mox-flow-fusion-svc/data/fusion_docs/INDEX.md) | Fusion：注册中心 + PTDoc + 六维 + 统一化 + 11 份设计文档 PT-DOC-01~10 + INDEX + Dockerfile |
| | `platform/domains/flow/svc/mox-flow-operator-wasm-svc/` | ✅ [README.md](../platform/domains/flow/svc/mox-flow-operator-wasm-svc/README.md) | WASM 沙箱 Operator |
| | `platform/domains/flow/svc/mox-flow-primiflow-svc/` | ✅ [README.md](../platform/domains/flow/svc/mox-flow-primiflow-svc/README.md) + [trace_matrix.md](../platform/domains/flow/svc/mox-flow-primiflow-svc/src/gen/trace_matrix.md) + [examples/out](../platform/domains/flow/svc/mox-flow-primiflow-svc/examples/out/trace_matrix.md) | Primiflow：14 个 examples + gen 代码 + DDL/Schema 生成 + Graph HTML/MMD |
| **Voice（5/5）** | | | |
| | `platform/domains/voice/svc/mox-voice-core-svc/` | ❌ | 核心 |
| | `platform/domains/voice/svc/mox-voice-asr-svc/` | ❌ | 语音识别 |
| | `platform/domains/voice/svc/mox-voice-intent-svc/` | ❌ | 语音意图 |
| | `platform/domains/voice/svc/mox-voice-operator-svc/` | ❌ | Operator |
| | `platform/domains/voice/svc/mox-voice-desktop-app/` | ❌ | 桌面应用（独立，非网关） |
| **Market（1/1）** | | | |
| | `platform/domains/market/svc/mox-market-template-svc/` | ✅ [README.md](../platform/domains/market/svc/mox-market-template-svc/README.md) | 模板市场（空壳，待 P2 实现） |
| **Platform（2/2）** | | | |
| | `platform/domains/platform/svc/mox-platform-enterprise-svc/` | ❌（**唯一生产 JWT + 动态实体 CRUD 已跑通的 3002 服务**，见总纲附录 A18） | 企业：IAM/租户/配额/JWT · 10 接口冒烟通过 3002 |
| | `platform/domains/platform/svc/mox-platform-orchestrator-svc/` | ✅ [README.md](../platform/domains/platform/svc/mox-platform-orchestrator-svc/README.md) | 编排服务 |
| **L4 svc README 覆盖率** | | **24/29 = 83%** | 待补：KG service/streams/spark/fusion + Cloud S3/Filer + Data 3 + Voice 5 = 11 份（其中 mox-kg-service-svc 因 HTTP 关键代码位置特殊需特别加 README 指出 http_adapter.rs） |

### L5 · sdk FFI 绑定层（2 份 · 全 ❌）

| crate 路径 |
|---|
| `platform/domains/cloud/sdk/mox-cloud-sdk/`（有 examples + tests，无 README）|
| `platform/domains/data/sdk/mox-data-formula-native/` |
| `platform/domains/data/sdk/mox-data-norm-intent-native/` |

### 特殊 · arch-test 架构守护（1 份 README ❌，见总纲附录 A14）

| crate 路径 | 测试数 | 覆盖的不变量 |
|---|---|---|
| `platform/arch-test/` | **7 tests** | 分层依赖 · 跨域 API 纯净 · 循环依赖 · L2 API 纯净 · 架构-数据分离 · 无硬编码路径 · 插件/三方目录分离 |

---

## III. 架构平行仓库（非 6 层 workspace 成员）

### `platform/backend-rust/`（独立 workspace · ADR-002 迁入计划，进入只读）

| 路径 | 文档 | 状态 |
|---|---|---|
| `platform/backend-rust/src/lib.rs` | ADR-002 映射表 | Q/R/S/T 4 模块导出 |
| `platform/backend-rust/deploy/istio/mox-service-mesh.yaml` | | Service Mesh Istio 配置，迁移目标 → `deploy/helm/mox/templates/istio-gateway.yaml` |
| `platform/backend-rust/tests/`（3 份集成测试）| | 迁入对应 L4 svc 下 tests/ |
| `platform/backend-rust/benches/`（性能基准）| | 迁入 `platform/framework/benches/` |

---

## IV. 部署脚本 & 配置（deploy/ 下非 md）

| 路径 | 用途 |
|---|---|
| `deploy/helm/mox/`（Chart.yaml + 4 模板 + values.yaml） | 主服务 Helm 伞图 |
| `deploy/helm/mox-dr/`（双活 DR：主备 Deployment + HPA + PDB + 区域选择） | 双 Region 灾备 Helm 伞图 |

---

## V. 缺文档待补（下一个 PR 的 TODO）

| 优先级 | 文档 | 目标 |
|---|---|---|
| HIGH | `mox-kg-service-svc/README.md`（当前缺）| 指出 http_adapter.rs 是唯一已生产的 KG/AI HTTP 入口 |
| HIGH | `mox-platform-enterprise-svc/README.md`（当前缺）| 指出 3002 端口 JWT 登录流程 + Week 2 合并入 8080 计划 |
| HIGH | `mox-cloud-s3-svc/README.md`（当前缺）| S3 兼容协议的 endpoints 表 + 与 AWS SDK/MinIO 客户端互通测试 |
| MEDIUM | L3 core 剩下 7 份空 README补全（Platform 5/AI 2）| 每个 core crate 至少 1 页架构说明 |
| MEDIUM | L4 svc 剩下 5 份空 README补全（Data 3 / KG storage+fusion 2）| 每个 svc crate 1 页：接口/依赖/启动命令 |
| LOW | L5 sdk 3 份 README | FFI 绑定用法示例 |

---

> 本索引文件本身遵守 once-defined 原则：**不定义任何概念**，只提供链接；所有「定义内容」链接回 I 部分的 10 份正本和 II 部分的 crate 级 README。
