# 专家联盟（Expert Alliance）文档统一索引

> 本索引是专家联盟文档体系的唯一导航入口，回答三件事：**每份文档是什么、按什么顺序读、互相冲突时以谁为准。**
>
> 维护原则：一个事实一个权威源。下表「权威等级」「冲突裁决」列即为权威链。
> 最近核对：**2026-10-01（A1 收官）**。现状事实以 `CURRENT-ARCHITECTURE.md` V1.1 为最终裁决。
>
> 收官新增能力（已核到代码 `文件:行号` + 真实测试数，详见 08 §十三）：
> **N4 图谱节点级 CRUD**（6 写端点 + RBAC `graph.mutate`）· **T2 图 RAG**（`POST /api/expert-graph/rag/expand`，内存态加权多跳）· **T3 MCP Server**（`platform/domains/alliance/mcp/mox-alliance-mcp-server`，**stdio 自实现 JSON-RPC 2.0，非 HTTP**，3 工具 expert_search/optimal_team/graph_expand，经 `MOX_INTERNAL_TOKEN` 调网关读面）· **U1 GraphCanvas 画布** · **U2 MatchExplainPanel 匹配透明化**（健康权重 0.15→**0.05** 纠错）。
>
> **收官测试总量**（均读自报告）：gateway alliance **97** · 三 svc（scheduler 23 + executor 15 + registry 28）**66** · scheduler-core **115** · scheduler-svc+http-sdk **38** · mox-alliance-mcp-server **5** · 前端 vitest **78 文件 / 1044**。
> **40 项矩阵最终态**（08 §十三 全维度功能总表）：**✅闭环 38 · 🟡部分 14 · 🔴残留 0**。

---

## 一、文档总表（编号 00–28，闭环）

| 编号 | 文件 | 标题 | 权威等级 | 定位 | 阅读顺序 | 冲突时以谁为准 |
|---|---|---|---|---|---|---|
| 00 | `CURRENT-ARCHITECTURE.md` | 当前实现架构（唯一权威）V1.1 | 🟢 权威 | 现状代码事实：四进程、16 crates、网关 11 文件 9924 行、7 模式、SM4、10:1:1、HA 选主 | **第 0 读（裁决基准）** | 现状以此为最终裁决 |
| 01 | `08-normalized-architecture.md` | 企业级模块化归一化架构（含 **§十三 全维度功能总表**） | 🟢 权威 | 三目录对齐总图：40 项矩阵、缺口清单、演进路线、排障；§十三 为 2026-10-01 收官总表（N4/T2/T3/U1/U2 逐条核到行号与测试数） | **第 1 读（导航总图）** | 现状仍回落 CURRENT V1.1；本文负责归一化导航与收官统计 |
| 02 | `09-deployment-templates.md` | 企业级生产部署配置模板 V1.0 | 🟢 权威 | 两档配置、compose/k8s、52 个 MOX_* 读取点、部署后检查清单 | 部署 / 上生产时读 | 开关语义以代码 `文件:行号` 为准；端口以 PORT-REGISTRY |
| 03 | `01-prd.md` | 产品需求 PRD V1.0 | 🟡 目标态 | 业务目标与角色（含 F-04 WS、F-02 matcher/状态过滤等已过期写法） | 第 2 读 | 与现状冲突 → CURRENT V1.1 |
| 04 | `02-architecture.md` | 总体架构 V1.0 | 🟡 目标态 | 分层依赖、技术选型（接入层写 REST/WS，默认链画成跨服务） | 第 3 读 | CURRENT V1.1 |
| 05 | `03-business-flow.md` | 业务流程 V1.0 | 🟡 目标态 | 六步主流程、注册/探活闭环 | 第 4 读 | CURRENT V1.1 |
| 06 | `04-state-machine.md` | 状态机 / 业务规则 V1.0 | 🟡 目标态 | 任务/节点/专家状态机（枚举已过期） | 第 5 读 | 08 §八 统一枚举字典 |
| 07 | `05-data-model.md` | 数据模型 / 字典 V1.0 | 🟡 目标态 | 表结构字典（含 plans/tasks 表幻觉、FusionStrategy 9 值误写） | 第 6 读 | 08 §六 存储三栏对账 |
| 08 | `06-api-spec.md` | API 规范 V1.0 | 🟡 目标态 | 接口清单（含 /ws/v1 幻影、漏 /api/alliance/* 一族） | 第 7 读 | CURRENT §6.1 |
| 09 | `07-deployment.md` | 部署运维 V1.0 | 📄 参考 | 早期启动步骤 + 极简 compose（漏 MOX_* env、无 k8s/检查清单） | 仅了解"怎么跑起来" | 生产以 09 号为准 |
| 10 | `10-enterprise-maturity-review.md` | 企业级成熟度逐功能评审 | 🟡 评审意见 | 逐功能成熟度打分（不修改现状事实） | 想找"还差什么"时读 | 现状以 CURRENT V1.1 |
| 11 | `11-competitive-benchmark.md` | 竞品对标 | 🟡 外部研究稿 | 竞品事实均带来源 URL；我方事实引自内部权威 | 战略对标时读 | 我方事实回落内部权威 |
| 12 | `12-innovation-roadmap.md` | 创新升级规划（新技术/新UI/新架构/新模式） | 🟡 规划意见 | T1–T5/U1–U4/A1–A4/M1–M4 任务池（部分已落地，见 16 号台账） | 看"下一步做什么" | 未动工项不属现状 |
| 13 | `13-end-to-end-business-flow.md` | 端到端全业务流程 | 🟡 目标态 | **§1.3 全链路总图 · §4.6 画布（U1）· §4.7 MCP（T3）** | 想串起"一个请求怎么跑完全链路"时读 | 已实现部分回落 CURRENT/08 §十三 |
| 14 | `14-enterprise-permission-model.md` | 企业级权限模型 | 🟡 目标态 | 五角色 + RBAC 目标模型（现状见 experts_rbac.rs） | 权限设计溯源 | 现状以代码 + 08 §13.5 |
| 15 | `15-product-spec-standard.md` | 企业级最好用产品规范标准 | 🟡 验收基线 | 产品验收基线（不改变现状事实） | 验收对照 | 现状以 CURRENT + 09 代码事实 |
| 16 | `16-decision-and-state-ledger.md` | 决策与状态总账 | 📁 核验记录 | **主线时间线**：R1–R4 修复轮 + 09-30 N4/N7/N8/RBAC + **10-01 T2/T3/U1/U2** 逐条状态 | 想查"每个决策/状态哪一步落地"时读 | 状态结论回落 CURRENT/报告 |
| 17 | `17-docs-architecture-and-flow-atlas.md` | 全 docs 架构与业务流程关联图谱 | 📄 参考 | 归一化导航与分析视图（含 #conflicts 冲突边界） | 文档间关系速查 | 不替代各主题权威 |
| 18 | `18-modular-product-design.md` | 模块化产品设计与分层突破路线 | 🟡 目标设计 | EA-W01–10 任务 + EA-Q01–16 质量场景（未实施项不属现状） | 路线拆解 | 未实施项非现状 |
| 19 | `19-lowcode-dynamic-configuration.md` | 低代码配方与全维动态配置 | 📄 参考 | 低代码目录矩阵 / 配方卡（document_ready） | 低代码场景 | 不新增运行时能力 |
| 20 | `20-normalized-plan-execution.md` | 业务计划与执行流程归一化 | 🟡 目标态 | 计划值类型校验、执行/恢复绑定、七模式经测试执行器验证（部分已实施） | 计划/执行语义 | release 快照/真实模型未标完成 |
| 21 | `index.html` | 可视化导航页 | 📄 参考 | 旧版图形化导航 | 浏览器快速浏览 | — |
| 22 | `expert-alliance-whitepaper.html` | 归一化架构白皮书（自包含单文件） | 📄 参考 | 可视化综合呈现：四层架构、52 项矩阵、治理时间线、部署开关、路线排障 | 5 分钟建立全貌 | 数字仍回落源文档，不替代权威 |
| 23 | `README.md` | 文档入口 | 📄 参考 | docs/expert-alliance 入口说明 | 首次进入 | — |
| 24 | `_verification/docs-verification-report.md` | 文档体系三方一致性核验 | 📁 核验记录 | docs↔code↔architecture 对账：三类硬冲突、端口漏登、baseline.sql 性质 | 想知道"为什么有冲突"时读 | 佐证文档冲突，不另立现状 |
| 25 | `platform/.../_verification/backend-verification-report.md` | 后端代码事实核验 | 📁 核验记录 | 30 项摘要核验（28✅/1❌/1⚠️），N1–N11 代码证据 | 后端事实溯源 | 代码事实以此报告佐证 |
| 26 | `frontend-ui/.../_verification/frontend-verification-report.md` | 前端代码事实核验 | 📁 核验记录 | 模块契约纪律（0 假端点、11 孤儿路由）、G1–G10 缺口 | 前端事实溯源 | 代码事实以此报告佐证 |
| 27 | `platform/.../_verification/backend-fix-report.md` | 后端高严重度缺口修复记录 | 📁 核验记录 | N1/N2/N3/N6/N9 修复、鉴权链路端到端、cargo test 85/79 | 看缺口怎么被修掉 | — |
| 28 | `frontend-ui/.../_verification/frontend-fix-report.md` | 前端缺口修复记录 | 📁 核验记录 | G1/G2/G3 + 30+ 调用点收敛、vitest 646 → 980/981（收官 1044） | 看缺口怎么被修掉 | — |

> 路径简写：`platform/.../_verification/` = `platform/domains/alliance/_verification/`；`frontend-ui/.../_verification/` = `frontend-ui/src/modules/expert-alliance/_verification/`。
> MCP 服务器入口：`platform/domains/alliance/mcp/mox-alliance-mcp-server/`（**stdio 自实现 JSON-RPC 2.0，Content-Length 帧；非 HTTP 路由、不进 actuator ROUTES**）。

---

## 二、推荐阅读顺序（一条线读完）

1. **`CURRENT-ARCHITECTURE.md`（V1.1）** —— 先建立"现在到底是什么"的基准。
2. **`08-normalized-architecture.md`（重点 §十三 收官总表）** —— 导航总图 + 收官最终态（✅38/🟡14/🔴0）。
3. **`16-decision-and-state-ledger.md` §五** —— 看主线时间线 R1→R4→10-01（T2/T3/U1/U2）怎么一步步闭环。
4. **`01-prd → 02 → 03 → 04 → 05 → 06`（目标态串读）** —— 理解设计意图；**逐条对照 CURRENT V1.1**。
5. **`13-end-to-end-business-flow.md`（§1.3 全链路 / §4.6 画布 / §4.7 MCP）** —— 串起端到端业务流。
6. **`09-deployment-templates.md`** —— 真正上生产时读；`07-deployment` 仅作"跑起来"参考。
7. **三份 `*-verification-report` + 两份 `*-fix-report`** —— 代码事实溯源。
8. **可视化入口**：`index.html` 与 `expert-alliance-whitepaper.html`，不替代权威文档。

---

## 三、冲突裁决规则（必须遵守）

| 争议点 | 唯一裁决源 | 说明 |
|---|---|---|
| **现状长什么样**（进程、行数、模式数、存储、枚举） | `CURRENT-ARCHITECTURE.md` **V1.1** | 01–08、10–20 多为目标态/规划/评审，文首未标"已被取代"，**不得直接当现状引用** |
| **端口号** | `docs/api/PORT-REGISTRY.md` V1.2 | 生产四端口 3080/3100/3200/3400；旁挂 3300/3210；gRPC 50051 联盟未使用 |
| **部署 / 运行开关（MOX_*）取值** | `09-deployment-templates.md`（语义再落到代码 `文件:行号`） | 07 漏列 env，08 仅 12 变量，09 核对 52 个读取点最全 |
| **数据库 / 持久化** | `docs/database/DATABASE-ARCHITECTURE.md`（as-built）+ 08 §六 三栏对账 | baseline.sql 是 MySQL 8.3 **目标模板**（ea_* 11 表），不是 PG 现状；现状是嵌入式 SQLite |
| **代码事实 / 缺口证据** | 三处 `_verification/` 核验报告（文件:行号） | 文档结论可争议，代码行号不争议 |
| **传输加密语义** | `docs/api/API-CRYPTO-TRANSPORT.md` | `MOX_API_CRYPTO=sm4` 一键开关 |
| **匹配健康权重口径** | 08 §13.4 / modular_matcher.rs:286 | **0.05**（旧文档 0.15 已纠错，以 0.05 为准） |

---

## 四、历史 / 被取代文档（勿作现行引用）

- **`08-function-matrix.md`（2026-09-27 初版矩阵）**：已被 **`08-normalized-architecture.md` §十三 全维度功能总表**取代，仅作历史对照。
- **`docs/enterprise/39-*.md`（开发专家联盟架构诊断与 SaaS 化方案）**：带「历史快照说明（2026-09-13）」，是 Rust 统一前 **Node 时代**的诊断，引用已删除的 `platform/backend-node/`、`mox-expert` crate 与已迁移的网关端口 :8080。**定性为历史诊断快照，与现行 Rust 实现是"被取代"关系，禁止并列引用。** 现行架构见 `CURRENT-ARCHITECTURE.md` V1.1。
- **`_archive/v1/`、`_archive/v2/`、`_archive/v3/`**：历史版本归档，勿引用。
- **01–07 中的三类已知幻觉**（引用前必查）：
  1. `/ws/v1/*` WebSocket 进度推送 —— 全 crate `WebSocketUpgrade` 零命中，实时性仅 SSE；文档侧已补记（#25/D3），代码无 WS。
  2. Task/Node/Plan 落 `experts_db.rs` SQLite —— 实读该文件仅 7 表，无 plans/tasks/nodes 表，任务在 scheduler/executor 侧、plans 进程内。
  3. 专家状态枚举 —— 代码实测为 `online/busy/offline/away`（登记值非探活），非 01/03/04 的 `active/inactive`。

---

*本索引随 `expert-alliance-whitepaper.html` 一并维护。新增文档请在此表追加一行并标注权威等级；收官后状态以 08 §十三 与 16 号台账时间线为准。*
