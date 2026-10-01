# 专家联盟（Expert Alliance）文档统一索引

> 本索引是专家联盟文档体系的唯一导航入口。它回答三个问题：**每份文档是什么、按什么顺序读、互相冲突时以谁为准。**
>
> 维护原则：一个事实一个权威源。下表的「权威等级」「冲突裁决」列即为权威链。
> 最近导航核对：2026-10-01。近期能力变更以 16 总账及具体代码证据补充，旧快照不自动更新。现状事实以 `CURRENT-ARCHITECTURE.md` V1.1（2026-09-24）为最终裁决。

---

## 一、文档总表

| 编号 | 文件 | 标题 | 权威等级 | 定位 | 阅读顺序建议 | 冲突时以谁为准 |
|---|---|---|---|---|---|---|
| 00 | `CURRENT-ARCHITECTURE.md` | 专家联盟当前实现架构（唯一权威）V1.1 | 🟢 权威 | 现状代码事实：四进程、16 crates、网关 11 文件 9924 行、7 模式、SM4、10:1:1、HA 选主 | **第 0 读（裁决基准）** | 现状以此为最终裁决 |
| 01 | `08-normalized-architecture.md` | 企业级模块化归一化架构 | 🟢 权威 | 三目录（platform/frontend-ui/docs）对齐总图：40 项矩阵、缺口清单、演进路线、排障 | **第 1 读（导航总图）** | 现状仍回落 CURRENT V1.1；本文负责归一化导航 |
| 02 | `09-deployment-templates.md` | 企业级生产部署配置模板 V1.0 | 🟢 权威 | 两档配置、compose/k8s、52 个 MOX_* 读取点、部署后检查清单 | 部署 / 上生产时读 | 开关语义以代码 `文件:行号` 为准；端口以 PORT-REGISTRY |
| 03 | `01-prd.md` | 产品需求 PRD V1.0 | 🟡 目标态 | 业务目标与角色（含 F-04 WS、F-02 matcher/状态过滤等已过期写法） | 第 2 读：了解"要做什么"，**勿当现状** | 与现状冲突 → CURRENT V1.1 |
| 04 | `02-architecture.md` | 总体架构 V1.0 | 🟡 目标态 | 分层依赖、技术选型（接入层写 REST/WS，默认链画成跨服务） | 第 3 读 | CURRENT V1.1 |
| 05 | `03-business-flow.md` | 业务流程 V1.0 | 🟡 目标态 | 六步主流程、注册/探活闭环（跨服务、状态过滤写法已过期） | 第 4 读 | CURRENT V1.1 |
| 06 | `04-state-machine.md` | 状态机 / 业务规则 V1.0 | 🟡 目标态 | 任务/节点/专家状态机（枚举值已过期） | 第 5 读 | 08 §八 统一枚举字典 |
| 07 | `05-data-model.md` | 数据模型 / 字典 V1.0 | 🟡 目标态 | 表结构字典（含 plans/tasks 表幻觉、FusionStrategy 9 值误写） | 第 6 读 | 08 §六 存储三栏对账 |
| 08 | `06-api-spec.md` | API 规范 V1.0 | 🟡 目标态 | 接口清单（含 /ws/v1 幻影、漏 /api/alliance/* 一族） | 第 7 读 | CURRENT §6.1（去重路径口径） |
| 09 | `07-deployment.md` | 部署运维 V1.0 | 📄 参考 | 早期启动步骤 + 极简 compose（漏 MOX_* env、无 k8s/检查清单） | 仅了解"怎么跑起来" | 生产以 `09-deployment-templates` 为准 |
| 10 | `index.html` | 可视化导航页 | 📄 参考 | 旧版图形化导航 | 浏览器快速浏览 | — |
| 11 | `expert-alliance-whitepaper.html` | 归一化架构白皮书（自包含单文件） | 📄 参考 | 可视化综合呈现：四层架构、40 项矩阵、治理时间线、部署开关、路线排障 | 想快速建立全貌时读 | 数字仍回落到源文档，不替代权威 |
| 12 | `_verification/docs-verification-report.md` | 文档体系三方一致性核验 | 📁 核验记录 | docs↔code↔architecture 对账：三类硬冲突、端口漏登、baseline.sql 性质 | 想知道"为什么有冲突"时读 | 佐证文档冲突，不另立现状 |
| 13 | `platform/.../_verification/backend-verification-report.md` | 后端代码事实核验 | 📁 核验记录 | 30 项摘要核验（28 ✅ / 1 ❌ / 1 ⚠️），N1–N11 代码证据 | 后端事实溯源 | 代码事实以此报告佐证 |
| 14 | `frontend-ui/.../_verification/frontend-verification-report.md` | 前端代码事实核验 | 📁 核验记录 | 模块契约纪律（0 假端点、11 孤儿路由）、G1–G10 缺口 | 前端事实溯源 | 代码事实以此报告佐证 |
| 15 | `platform/.../_verification/backend-fix-report.md` | 后端高严重度缺口修复记录 | 📁 核验记录 | N1/N2/N3/N6/N9 修复、鉴权链路端到端、cargo test 85 / 79 | 看缺口怎么被修掉 | — |
| 16 | `frontend-ui/.../_verification/frontend-fix-report.md` | 前端缺口修复记录 | 📁 核验记录 | G1/G2/G3 + 30+ 调用点收敛、vitest 646 → 980/981 | 看缺口怎么被修掉 | — |
| 17 | `scripts/` | 部署后一致性一键校验脚本（sh/ps1/README） | 🛠️ 工具 | 把 09 §六 7 项清单落成可执行脚本：四进程 /health、鉴权 200/401、SM4 信封、HA 选主、审计 NDJSON | 上生产 / CI 接入时跑 | 检查逻辑以 09 §六 为权威；脚本本身不另立事实 |
| 18 | `10-enterprise-maturity-review.md` | 企业级成熟度逐功能评审 | 📄 评审结论 | 40 项矩阵逐项 7 维度评审：✅22 / 🟡14 / 🔴4，TOP 差距与 P0 阻断项 | 想上生产 / 立项评估时读 | 现状与缺口以 CURRENT V1.1 + 08 矩阵为准；评审结论不另立事实 |
| 19 | `11-competitive-benchmark.md` | 竞品对标（外部核证） | 📄 外部研究 | 四类 10 竞品（CrewAI/AutoGen/LangGraph、Dify/Coze/n8n/Airflow、市场、企业平台）135 处来源 URL 核证 | 想对标"最好用"时读 | 竞品事实以来源 URL 为准；我方结论回落 CURRENT/08/09 |
| 20 | `12-innovation-roadmap.md` | 创新升级规划 | 📄 规划 | 17 张创新卡片（新技术/新UI/新架构/新模式）+ P0 前置 + 三阶段里程碑 | 立项 / 版本规划时读 | 创新建议基于 10/11 结论，不改变现状事实 |
| 21 | `13-end-to-end-business-flow.md` | 端到端全业务流程明确化 | 🟢 权威 | 主流程 8 步逐步详解（输入/输出/服务/状态/异常）+ 附加流程 + 状态机总图 + 本地/远程差异 | 想弄清"任务怎么流转"时读 | 流程细节以 CURRENT V1.1 + PORT-REGISTRY 为准；本文是整合视图 |
| 22 | `14-enterprise-permission-model.md` | 企业级权限模型 | 🟢 权威 | 权限 5 层（认证/路由/按钮/审计/资源租户）现状矩阵：现行 5 角色 + 目标态 6 角色参考；权限矩阵总表 5 栏 | 想弄清"谁能做什么"时读 | 角色/审计事实以代码文件:行号为准；39 号 6 角色为历史快照参考 |
| 23 | `15-product-spec-standard.md` | 企业级最好用产品规范标准 | 📄 规范基线 | 六大维度 27 条规范（易用/可观测/安全/性能/可运维/生态）+ P0 必达标 11 条 QA checklist | 验收/QA/立项评审时读 | 规范定义"达标长什么样"，不改变现状事实 |
| 24 | `16-decision-and-state-ledger.md` | 决策与状态总账 | 🟢 记录权威 | 缺口/决策/状态全量总账：N/G/D/P0/T/U/A/M 逐条 + 12 条设计取舍 + 冲突裁决链 + 00-24 索引锚点 | 任何"这步为什么这么做/那个缺口现在啥状态"时读 | 本账是记录视图，事实仍以各源文档为准 |

| 25 | `17-docs-architecture-and-flow-atlas.md` | 全 docs 架构与业务流程关联图谱 | 📄 关联视图 | 架构家族、流程、冲突与逐层分析；全目录生成证据 | 建立全域地图时读 | 不替代各主题现状源 |
| 26 | `18-modular-product-design.md` | 模块化产品设计与突破路线 | 🟡 目标设计 | 模块责任、契约、质量场景、任务依赖 | 实施前评审时读 | 目标数值非实测，状态仍查16 |

> 路径简写：`platform/.../_verification/` = `platform/domains/alliance/_verification/`；`frontend-ui/.../_verification/` = `frontend-ui/src/modules/expert-alliance/_verification/`。

---

## 二、推荐阅读顺序（一条线读完）

1. **`CURRENT-ARCHITECTURE.md`（V1.1）** —— 先建立"现在到底是什么"的基准，后文一切冲突都拿它裁决。
2. **`08-normalized-architecture.md`** —— 用它当导航总图，看 40 项矩阵、缺口清单与演进路线。
3. **`01-prd → 02 → 03 → 04 → 05 → 06`（目标态串读）** —— 理解设计意图；**逐条对照 CURRENT V1.1**，遇到 WS / plans 表 / 状态枚举矛盾时以 V1.1 为准。
4. **`09-deployment-templates.md`** —— 真正要上生产时读；`07-deployment` 仅作"跑起来"的早期参考。
5. **三份 `*-verification-report` + 两份 `*-fix-report`** —— 需要溯源"这个结论哪来的、那个缺口怎么修的"时再读。
6. **可视化入口**：`index.html` 与 `expert-alliance-whitepaper.html` 用于快速建立全貌，不替代权威文档。
7. **收官四件套（10/11/12/13）**：想评估"够不够企业级"读 `10`；想对标"最好的竞品"读 `11`；想规划下一步读 `12`；想弄清任务端到端怎么流转读 `13`。这四份是评审/研究/规划/流程视图，不改变现状裁决链（仍以 CURRENT V1.1 为基准）。
8. **最终收尾三件（14/15/16）**：想弄清"谁能做什么"读 `14`；想验收"最好用达标了没"读 `15`；想回溯"任何一步为什么这么做、缺口什么状态"读 `16`。

---

## 三、冲突裁决规则（必须遵守）

| 争议点 | 唯一裁决源 | 说明 |
|---|---|---|
| **现状长什么样**（进程、行数、模式数、存储、枚举） | `CURRENT-ARCHITECTURE.md` **V1.1** | 01–07 均为 V1.0 目标态，文首未标"已被取代"，**不得直接当现状引用** |
| **端口号** | `docs/api/PORT-REGISTRY.md` V1.2 | 生产四端口 3080/3100/3200/3400；旁挂 3300/3210；gRPC 50051 联盟未使用 |
| **部署 / 运行开关（MOX_*）取值** | `09-deployment-templates.md`（语义再落到代码 `文件:行号`） | 07 漏列 env，08 仅 12 变量，09 核对 52 个读取点最全 |
| **数据库 / 持久化** | `docs/database/DATABASE-ARCHITECTURE.md`（as-built）+ 08 §六 三栏对账 | baseline.sql 是 MySQL 8.3 **目标模板**（ea_* 11 表），不是 PG 现状；现状是嵌入式 SQLite |
| **代码事实 / 缺口证据** | 三处 `_verification/` 核验报告（文件:行号） | 文档结论可争议，代码行号不争议 |
| **传输加密语义** | `docs/api/API-CRYPTO-TRANSPORT.md` | `MOX_API_CRYPTO=sm4` 一键开关 |

---

## 四、历史 / 被取代文档（勿作现行引用）

- **`docs/enterprise/39-*.md`（开发专家联盟架构诊断与 SaaS 化方案）**：带「历史快照说明（2026-09-13）」，是 Rust 统一前 **Node 时代**的诊断，引用了已删除的 `platform/backend-node/`、`mox-expert` crate 与已迁移的网关端口 :8080。**定性为历史诊断快照，与现行 Rust 实现是"被取代"关系，禁止并列引用。** 现行架构见 `CURRENT-ARCHITECTURE.md` V1.1。
- **`_archive/v1/`、`_archive/v2/`、`_archive/v3/`**：历史版本归档，勿引用。
- **01–07 中的三类已知幻觉**（引用前必查）：
  1. `/ws/v1/*` WebSocket 进度推送 —— 全 crate `WebSocketUpgrade` 零命中，实时性仅 SSE `GET /api/alliance/tasks/:id/logs/stream`；
  2. Task/Node/Plan 落 `experts_db.rs` SQLite —— 实读该文件仅 7 表，无 plans/tasks/nodes 表，任务在 scheduler/executor 侧、plans 进程内；
  3. 专家状态枚举 —— 代码实测为 `online/busy/offline/away`（登记值非探活），非 01/03/04 的 `active/inactive`。

---

*本索引随 `expert-alliance-whitepaper.html` 一并交付。新增文档时请在此表追加一行并标注权威等级。*

## 五、全域整理交付入口

[全域图谱](docs/expert-alliance/17-docs-architecture-and-flow-atlas.md#scope) → [模块化设计](docs/expert-alliance/18-modular-product-design.md#goals) → [ADR-17](docs/enterprise/45-专家联盟模块化归一与证据治理-ADR-17.md#decision)。完整资料盘点位于 reports/markdown/docs-architecture-corpus.md；本文新增25–26仅为导航，不改变原24项权威职责。

## 六、逐目录低代码设计

[19联盟配方](docs/expert-alliance/19-lowcode-dynamic-configuration.md#recipe)为目标配置设计（导航编号27）；[LC-STD-001](docs/standards/lowcode-dynamic-configuration.md#scope)是配置语义源；[目录矩阵](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#directories)规定逐层设计职责；[ADR-18](docs/enterprise/46-低代码全维配置控制与运行边界-ADR-18.md#decision)状态Proposed。配置编译/发布/快照未由本文宣称已落地。

## 七、业务流程归一化实现

[28 · 20统一计划与执行](docs/expert-alliance/20-normalized-plan-execution.md#scope)：已实现计划校验、任务绑定、恢复入口验证和确定性拓扑；完整部署与配置发布另行验收。
