# docs/architecture/ 文档景观盘点 — 权威分级 + 代码漂移清单

> **产出日期**：2026-09-16
> **盘点范围**：`docs/architecture/` 全部 66 个 `.md`（含 meta/ microservices/ rust-enterprise/ ai/ graph/ plugin/ full-dimensional/ assets/）
> **性质**：L7 过程证据（只读盘点，未移动/重命名/删除任何文件）
> **事实基准**（编排者核对，判定漂移的标尺）：
> - 网关唯一入口 = **3080**（8080 为历史旧值）
> - 企业默认 = **四进程**（网关3080 / 编排3001 / 联盟调度3100 / 联盟执行3200）
> - workspace = **143 crate / 12 业务域**（kg/ai/flow/data/cloud/voice/market/alliance/kb/base/project/platform）
> - 文档结构权威 = `docs/ARCHITECTURE-OF-DOCS.md`（L0~L8）；端口权威 = `docs/api/PORT-REGISTRY.md`

---

## 一、文档全量清单与职责归类（66 个 .md）

### 1.1 权威总览类（声称"唯一权威/顶层总览"的文档）

| # | 文件 | 自称身份 | 日期/版本 | 实际状态判定 |
|---|------|----------|-----------|-------------|
| 1 | `architecture.md` | **"本文档为 MOX 平台唯一权威架构规范，统摄所有模块"**（L4/L540） | v3.0, 2026-08-29 | **过期权威**：整篇描述旧 Python/FastAPI 栈（端口 8600/8601、DSQL 引擎、3 前端），与当前 Rust workspace 完全不符 |
| 2 | `SYSTEM-OVERVIEW.md` | "MOX 系统的顶层总览"（L4） | 基准 2026-09-07 | **事实最接近现状**：143 crates ✓ / 12 域 ✓ / 223 API ✓，但端口与进程数漂移（见漂移清单） |
| 3 | `NORMALIZED_ARCHITECTURE.md` | "归一化架构规范 v1.0" | 基准"48 crate / 107 依赖边" | **基线过期**：仍基于 48 crate / 8 域快照，域列表缺 alliance/kb/base/project |
| 4 | `OPTIMAL_ARCHITECTURE.md` | "企业级最优架构总纲 v2.0" | — | **多版本并行**：6层8域，含 JSONPort[:8080] |
| 5 | `ARCHITECTURE_DESIGN_v3.1.md` | "全维架构设计文档 v3.1.0" | 2026-09-05 | **多版本并行**：8101/8102/8103/8104 四服务独立部署模型，与当前网关3080单体+渐进拆分模型不同 |
| 6 | `MOX-UNIFIED-ENTERPRISE-BASELINE-v1.0.md` | "已落地基线，作为新系统开发的唯一扩展入口" | v1.0 | **多版本并行**：6层归一化但域列表仅 6 个（kg/ai/flow/data/cloud/alliance），未含 kb/base/project/platform/market/voice |
| 7 | `ARCHITECTURE-ENTERPRISE.md` | "璇玑 RelGraph OUS 企业级架构文档 v1.0.0" | 2026-09-04 | **多版本并行**：1000+ 行全栈设计，三端融合视角 |
| 8 | `rust-enterprise/README.md` | "纯 Rust 企业级架构迁移的权威单源（SSOT）" | v1.0, 2026-08-27 | **过期权威**：8 业务域 / 60+ crates / 31 域路由 |

### 1.2 子目录入口与规范文档（非"总览"，按职责定位）

| 子目录 | 文件 | 职责 | 漂移状态 |
|--------|------|------|----------|
| `meta/` | `00-COSMIC-META-ARCHITECTURE.md` | COSMIC 元架构总纲（9 能力域/5 扩展点） | 概念层，无数值漂移 |
| `meta/` | `02-EXPERT-ALLIANCE-ARCHITECTURE.md` | 专家联盟元架构版（7 服务+1 Sidecar） | 概念层 |
| `meta/` | `03-DATABASE-DESIGN-SPEC.md` | 数据库设计规范 | — |
| `meta/` | `04-EXPERT-ALLIANCE-v3-MODULAR.md` | 专家联盟 v3 模块化（God Module 拆分） | "60+文件"指单 crate 文件数，非 workspace crate 数，**不计漂移** |
| `microservices/` | `00~06` + `DOMAIN-DEPLOYMENT.md` + `README.md` | 微服务独立部署 12 铁律/36 服务边界 | 规划/前瞻层，8域为设计目标态 |
| `rust-enterprise/` | `01~08` + `README.md` | Rust 企业级 6 层架构详解 | **系统性 8域/60+crate 漂移**（见清单） |
| `ai/` | `README.md` | AI 架构层说明 | 入口导航 |
| `graph/` | `README.md` + `requests/README.md` | 关图产物与需求基线 | 产物目录 |
| `plugin/` | `PLUGIN-ARCHITECTURE.md` / `VSCODE-COMPATIBILITY.md` / `VSCODE-API-STATUS.md` / `README.md` | VSCode 插件架构 | 独立子系统 |
| `full-dimensional/` | `00-README.md` / `mox-requirement-baseline.md` / `guantu-skeleton.md` / `GOVERNANCE_CONSOLE_API_READY_20260816.md` | 全维 TraceMatrix 与需求基线 | 需求基线层 |
| `assets/` | `README.md` + JSON 基准数据 | 结构化产物 | 数据文件 |

### 1.3 根目录编号系列与专题文档

| 文件 | 职责 | 漂移状态 |
|------|------|----------|
| `README.md` | L2 层入口/文档导航 | **第九节速查卡 8域漂移** |
| `BUSINESS-FLOWS.md` | 业务处理流程（请求怎么走） | **:8080 漂移**（L10） |
| `MODULARITY.md` | 模块化模式分析 | **:8080 漂移**（L15）；143 crates ✓ / 12 域 ✓ |
| `operations-manual.md` | 操作手册 | **整篇旧 Python 栈**（8600/8601/8080），未标注废弃 |
| `deployment-guide.md` | 部署指南 | **整篇旧 Python 栈**（8600），未标注废弃 |
| `04-error-code-reference.md` | 错误码参考 | 仅 ISO 8601 时间格式，**非端口漂移** |
| `05-normalization-checklist.md` | 归一化检查清单 | — |
| `06-rpc-integration-guide.md` | RPC 对接手册 | — |
| `07-KG-DYNAMIC-SQL-ARCHITECTURE.md` | KG 动态 SQL | — |
| `08-FULL-DIMENSION-LOWCODE-ARCHITECTURE.md` | 低代码九层架构 | — |
| `09-ROCKSDB-PERFORMANCE-OPTIMIZATION.md` | RocksDB 性能 | — |
| `10-DSQL-CORE-FULL-DIMENSIONAL-VALIDATION.md` | DSQL 验证 | — |
| `11-ENTERPRISE-WEBSITE-LOWCODE-IMPLEMENTATION.md` | 企业门户低代码 | — |
| `12-MOX-RUNTIME-ENGINE-DELIVERY.md` | 运行时引擎交付 | L34 `python run.py 8600` 旧栈残留 |
| `13-PLATFORM-CODEBASE-GUIDE.md` | 代码库开发指南 | — |
| `14-REPOSITORY-FULL-MAP.md` | 仓库全地图 | **73 crate / 8域 / 8600/8601 多重漂移** |
| `AI-UNIFIED-OPTIMIZATION-PLAN.md` | AI 优化计划 | — |
| `ARCHITECTURE_OPTIMIZATION.md` | 架构优化记录 | grep 无 8080 命中 |
| `ARCHITECTURE_SAAS_PRIVATE.md` | SaaS/私有化部署 | **:8080 漂移**（L17 vite 代理） |
| `APP-store-architecture.md` / `app-store-architecture.md` | 应用商店架构 | — |
| `data-exchange-spec.md` | MXDEF 数据交换 | 仅 ISO 8601，**非漂移** |
| `DOMAIN_FIRST_LAYOUT.md` | 领域优先布局 | — |
| `MOX-MODULE-COMPOSITION-v1.md` | 模块组合装配 | — |

---

## 二、五份核心文档声称的架构事实对比

| 文档 | 端口 | 进程数 | crate 数 | 域数 | 分层 | 数据基准 |
|------|------|--------|----------|------|------|----------|
| **SYSTEM-OVERVIEW.md** | **:8080** ❌ | **五进程** ❌ | 143 ✓ | 12 ✓（L35-37） | 6 层 ✓ | 2026-09-07 |
| **NORMALIZED_ARCHITECTURE.md** | 未明确 | 未明确 | **48** ❌ | **8** ❌（L19/L217） | 6 层+8域 ❌ | "48 crate/107 依赖边" |
| **BUSINESS-FLOWS.md** | **:8080** ❌ | 未直接计数（列 4 独立进程+KB） | 未提及 | 46 域描述符 | 中间件链 4 层 | 2026-09-07 |
| **architecture.md** | **:8600/:8601** ❌（旧 Python） | 未提及 | 未提及 | **8 大 domain** ❌（L95） | 3 层数据架构 | v3.0, 2026-08-29 |
| **rust-enterprise/01-architecture-overview.md** | 未明确（mermaid 中 JSONPort[:8080]） | 未提及 | **60+** ❌ | **8** ❌（L8/L89/L200） | L0-L5 六层 | 2026-08-27 |

> **要点**：SYSTEM-OVERVIEW.md 是唯一一份 crate 数（143）和域数（12）均正确的总览文档，但端口和进程数仍停留在旧值。其余四份均存在 2~3 项关键数值漂移。

---

## 三、代码漂移清单（文件:行号 + 错误原文 + 应为值）

### 3.1 端口漂移：`:8080` → 应为 `:3080`

| # | 文件:行号 | 原文片段 | 应为 | 备注 |
|---|-----------|----------|------|------|
| D1 | `SYSTEM-OVERVIEW.md:30` | `→ gateway（唯一入口 :8080）` | `:3080` | 当前状态声明 |
| D2 | `SYSTEM-OVERVIEW.md:81` | `\| **mox-server** \| :8080 \| 网关唯一入口…` | `:3080` | 进程表 |
| D3 | `BUSINESS-FLOWS.md:10` | `客户端 ──▶ :8080 网关（唯一入口）` | `:3080` | ASCII 流程图 |
| D4 | `MODULARITY.md:15` | `网关 :8080 进程内装配 21 个路由单元` | `:3080` | 进程面盘点 |
| D5 | `OPTIMAL_ARCHITECTURE.md:314` | `Route -->\|JSON-RPC\| JSONPort[:8080]` | `:3080` | mermaid 图 |
| D6 | `operations-manual.md:56` | `# 浏览器访问 http://localhost:8080` | `:3080` | 旧 Python 栈文档 |
| D7 | `ARCHITECTURE_SAAS_PRIVATE.md:17` | `Vite /voice 代理 → :8080（网关 /voice/** → … → :30010）` | `:3080` | vite.config 注释 |
| D8 | `rust-enterprise/06-ai-engine-api.md:440` | `curl -X POST http://localhost:8080/ai/engine/process` | `:3080` | curl 示例 |
| D9 | `rust-enterprise/06-ai-engine-api.md:445` | `curl http://localhost:8080/ai/engine/capabilities` | `:3080` | curl 示例 |
| D10 | `rust-enterprise/06-ai-engine-api.md:448` | `curl http://localhost:8080/ai/engine/metrics` | `:3080` | curl 示例 |

> **历史对照可保留项**：本次扫描未发现任何 8080 出现在"修复前后对照表"上下文中。`ARCHITECTURE_OPTIMIZATION.md` 中 grep 8080 零命中。上述 10 条均为当前状态声明，**全部待修**。
>
> 附注：`rust-enterprise/README.md:86` 路线图 R9 写"Rust Gateway 端口 8080 全面接管"——这是 2026-08-27 时点的迁移计划描述，属历史路线记录，可在该行加注"（端口后调整为 3080）"而非强制改写。

### 3.2 进程数漂移：`五进程` → 应为 `四进程`

| # | 文件:行号 | 原文片段 | 应为 |
|---|-----------|----------|------|
| D11 | `SYSTEM-OVERVIEW.md:77` | `## 4. 运行架构（五进程企业级部署）` | `四进程企业级部署` |

> 说明：SYSTEM-OVERVIEW.md L79-85 的进程表列了 5 行（mox-server:8080 / operator-server:3001 / mox-kb-server:8104 / scheduler:3100 / executor:3200）。企业默认四进程指网关+编排+调度+执行；KB:8104 为独立知识库进程，不纳入"企业四进程"基线口径。标题"五进程"应改为"四进程"并在表注说明 KB 为独立进程。

### 3.3 Crate 数漂移：`48 crate` / `60+` / `73 crate` → 应为 `143`

| # | 文件:行号 | 原文片段 | 应为 |
|---|-----------|----------|------|
| D12 | `NORMALIZED_ARCHITECTURE.md:3` | `基于算法验证结果（48 crate / 107 依赖边 / …）` | `143 crates`（需重算依赖边） |
| D13 | `NORMALIZED_ARCHITECTURE.md:48` | `## 二、48 Crate 归一化映射表` | 表标题需重写 |
| D14 | `OPTIMAL_ARCHITECTURE.md:386` | `\| 48 crate 归一化迁移（目录+重命名） \| ✅ 完成 \|` | 历史进度记录，可保留但加注"（已演进至 143 crates）" |
| D15 | `14-REPOSITORY-FULL-MAP.md:20` | `73 crate workspace · 6层8域DDD矩阵` | `143 crate workspace · 6层12域` |
| D16 | `14-REPOSITORY-FULL-MAP.md:268` | `根 workspace（73 crate）` | `143 crate` |
| D17 | `rust-enterprise/01-architecture-overview.md:218` | `workspace 根（60+ members）` | `143 members` |
| D18 | `rust-enterprise/01-architecture-overview.md:238` | `完整 60+ crates 清单` | `143 crates` |
| D19 | `rust-enterprise/03-module-inventory.md:1` | `# 03 · 60+ Crates 模块清单` | `143 Crates` |
| D20 | `rust-enterprise/03-module-inventory.md:201` | `60+ crates 编译零错误` | `143 crates` |
| D21 | `rust-enterprise/07-build-and-test.md:67` | `60+ crates 编译零错误` | `143 crates` |
| D22 | `rust-enterprise/README.md:7` | `覆盖 60+ Rust crates` | `143 Rust crates` |
| D23 | `rust-enterprise/README.md:23` | `60+ Crates 模块清单` | `143 Crates` |
| D24 | `rust-enterprise/README.md:60` | `盘点 60+ crates 现状` | `143 crates` |

### 3.4 域数漂移：`8域` / `八大域` → 应为 `12 域`

| # | 文件:行号 | 原文片段 | 应为 |
|---|-----------|----------|------|
| D25 | `README.md:15` | `Rust 企业级开发指南（6 层 / 8 域）` | `6 层 / 12 域` |
| D26 | `README.md:84` | `6 层架构、8 业务域详解` | `12 业务域` |
| D27 | `README.md:207` | `L3 领域服务层 → 8域 (kg/ai/flow/data/cloud/voice/market/platform)` | `12 域 (kg/ai/flow/data/cloud/voice/market/alliance/kb/base/project/platform)` |
| D28 | `architecture.md:95` | `在现有 8 大 domain 基础上` | `12 业务域` |
| D29 | `NORMALIZED_ARCHITECTURE.md:19` | 域定义表仅列 8 域（kg/ai/flow/data/cloud/voice/platform/market） | 补全 12 域（+alliance/kb/base/project） |
| D30 | `NORMALIZED_ARCHITECTURE.md:217` | `### 5.1 目标架构（6层 + 8域）` | `6层 + 12域` |
| D31 | `OPTIMAL_ARCHITECTURE.md:29` | `## 二、目录结构（6层8域，最优布局）` | `6层12域` |
| D32 | `14-REPOSITORY-FULL-MAP.md:20` | `6层8域DDD矩阵` | `6层12域` |
| D33 | `14-REPOSITORY-FULL-MAP.md:36` | `8 域 api → 各域 svc → core` | `12 域` |
| D34 | `14-REPOSITORY-FULL-MAP.md:68` | `**8 域 DDD 矩阵核心**` | `12 域` |
| D35 | `14-REPOSITORY-FULL-MAP.md:72` | `8 域 api/svcapi 已建目录` | `12 域` |
| D36 | `14-REPOSITORY-FULL-MAP.md:270` | `6层8域 DDD 矩阵` | `6层12域` |
| D37 | `rust-enterprise/01-architecture-overview.md:8` | `8 个业务域（AI/KG/Flow/Cloud/Data/Voice/Market/Streams）` | `12 个业务域` |
| D38 | `rust-enterprise/01-architecture-overview.md:89` | `### L3 · 业务服务层（Service Layer · 8域）` | `12域` |
| D39 | `rust-enterprise/01-architecture-overview.md:200` | `L3 业务服务层 (8域独立 crate)` | `12域` |
| D40 | `rust-enterprise/02-business-flow.md:33` | `P5 编码开发<br/>8域模块化 + 单元测试` | `12域` |
| D41 | `rust-enterprise/03-module-inventory.md:12` | `L4 Core（算法内核） \| 16 \| 8 域核心算法库` | `12 域` |
| D42 | `rust-enterprise/03-module-inventory.md:13` | `L3 Service（业务服务） \| 28 \| 8 域服务层` | `12 域` |
| D43 | `rust-enterprise/07-build-and-test.md:41` | `domains/ # 8 域 (L3+L4)` | `12 域` |
| D44 | `rust-enterprise/08-business-function-relation.md:8` | `## 一、8 域功能关联总图` | `12 域` |
| D45 | `rust-enterprise/08-business-function-relation.md:104` | `Gateway \| 全部 8 域 \| 路由分发` | `12 域` |
| D46 | `rust-enterprise/README.md:7` | `6 层归一化 + 8 业务域` | `12 业务域` |
| D47 | `rust-enterprise/README.md:21` | `8域划分` | `12域` |
| D48 | `rust-enterprise/README.md:28` | `8域关联总图` | `12域` |
| D49 | `rust-enterprise/README.md:43` | `L3 业务服务 8域 Service: AI · KG · Flow · Cloud · Data · Voice · Market · Streams` | `12域` |

### 3.5 旧 Python 栈端口：`:8600` / `:8601`（整篇文档过期）

以下文档整篇描述已归档的 Python/FastAPI 栈（AGENTS.md 已声明"旧 Python 版服务已归档至 platform/legacy/，勿用"），但**未在头部标注废弃**：

| 文件 | 典型行号 | 内容 |
|------|----------|------|
| `operations-manual.md` | L39-43, L56, L63, L242-245, L341, L353, L441, L482-485, L530, L543-544, L573, L607 | 全篇 `python run.py 8600` / `:8601` 商店服务操作 |
| `deployment-guide.md` | L54-57, L110-122, L150-151, L193, L248-252, L322 | 全篇 Docker/systemd/Nginx 配置指向 8600 |
| `architecture.md` | L178, L201, L395, L468, L491-496 | 旧栈架构图 + 目录结构 + API 端口 |
| `12-MOX-RUNTIME-ENGINE-DELIVERY.md` | L34 | `python run.py 8600` |
| `14-REPOSITORY-FULL-MAP.md` | L21, L37, L66, L283 | 仓库地图中仍列 Python mox-server:8600 / mox-store:8601 |

> 这些不是"改一个数字"能修的——整篇文档的技术栈（FastAPI 内置网关、DSQL Jinja2 模板、3 前端 SPA、Python run.py）已被 Rust axum 网关取代。建议头部标注"⚠️ 历史参考：本文档描述旧 Python 栈，已归档至 platform/legacy/，当前实现见 SYSTEM-OVERVIEW.md"。

### 3.6 `:3010`（Node 旧入口）扫描结果

**零命中**。全 `docs/architecture/` 目录无 3010 残留。干净。

---

## 四、归一化建议

### 4.1 谁是归一化唯一权威？

**建议：以 `SYSTEM-OVERVIEW.md` 作为归一化后唯一权威总览，而非 `NORMALIZED_ARCHITECTURE.md`。**

理由：

| 维度 | SYSTEM-OVERVIEW.md | NORMALIZED_ARCHITECTURE.md |
|------|-------------------|---------------------------|
| 数据基准 | 2026-09-07/09-13（最近） | "48 crate / 107 依赖边"（早期快照） |
| crate 数 | **143 ✓** | 48 ❌ |
| 域数 | **12 ✓** | 8 ❌ |
| 分层 | 6 层 ✓ | 6 层 ✓ |
| 端口 | 8080 ❌（仅需改 2 处） | 未提及 |
| 进程数 | 五进程 ❌（仅需改 1 处） | 未提及 |
| 定位 | 顶层总览（架构+能力+运行+治理） | 命名规范+crate 映射表 |

SYSTEM-OVERVIEW.md 只需修 **3 处**（D1/D2 端口 8080→3080、D11 五进程→四进程）即可成为完全准确的当前权威总览。NORMALIZED_ARCHITECTURE.md 则需要从 48 crate/8域 全面重写为 143 crate/12域，且其 crate 映射表本身是"归一化迁移计划"的中间态记录，不适合作为现状权威。

**具体建议**：

1. **修复 `SYSTEM-OVERVIEW.md` 的 3 处漂移**（D1, D2, D11），使其成为准确的"当前架构唯一权威总览"。
2. **将 `NORMALIZED_ARCHITECTURE.md` 重定位**为"架构设计规范/命名约定"参考文档（而非"现状权威"），头部加注"本文档记录归一化迁移设计与命名公式；当前 workspace 实际状态见 SYSTEM-OVERVIEW.md"。其 48 crate 映射表作为历史迁移记录保留。
3. 在 `architecture/README.md` 第二节"架构总览"表中，将 `SYSTEM-OVERVIEW.md` 标注为 **✅ 当前权威总览**。

### 4.2 多版本文档应标注"已被取代/历史参考"的清单

以下文档与 SYSTEM-OVERVIEW.md 功能重叠且数值过期，建议在头部 blockquote 加注：

| 文件 | 当前自称 | 建议标注 |
|------|----------|----------|
| `architecture.md` | "唯一权威架构规范"（v3.0, Python 栈） | ⚠️ **历史参考**：本文档描述旧 Python/FastAPI 栈（端口 8600/8601），当前 Rust workspace 架构见 `SYSTEM-OVERVIEW.md` |
| `NORMALIZED_ARCHITECTURE.md` | "归一化架构规范 v1.0"（48 crate/8域） | ⚠️ **迁移设计记录**：基线为 48 crate/8域早期快照；当前 143 crate/12域状态见 `SYSTEM-OVERVIEW.md` |
| `OPTIMAL_ARCHITECTURE.md` | "最优架构总纲 v2.0"（6层8域） | ⚠️ **历史参考**：8域/60+ crate 规划已演进；当前状态见 `SYSTEM-OVERVIEW.md` |
| `ARCHITECTURE_DESIGN_v3.1.md` | "v3.1 全维架构设计"（8101-8104 四服务） | ⚠️ **历史设计稿**：独立服务端口模型已调整为网关3080+渐进拆分；见 `SYSTEM-OVERVIEW.md` §4 |
| `MOX-UNIFIED-ENTERPRISE-BASELINE-v1.0.md` | "唯一扩展入口"（6层/6域） | ⚠️ **基线参考**：域列表未含 kb/base/project/alliance；当前 12 域见 `SYSTEM-OVERVIEW.md` §2.2 |
| `ARCHITECTURE-ENTERPRISE.md` | "OUS 企业级架构 v1.0.0" | ⚠️ **三端融合视角设计稿**：与 SYSTEM-OVERVIEW.md 互补，不替代之 |
| `rust-enterprise/README.md` | "权威单源 SSOT"（8域/60+ crate） | ⚠️ **2026-08-27 迁移期快照**：8域/60+ crate 已演进为 12域/143 crate；当前总览见 `SYSTEM-OVERVIEW.md` |
| `operations-manual.md` | 操作手册（Python run.py 8600） | ⚠️ **历史参考**：旧 Python 栈操作；当前启动见 `scripts/start-mox-enterprise.ps1` |
| `deployment-guide.md` | 部署指南（Docker 8600） | ⚠️ **历史参考**：旧 Python 栈部署；当前见 `docker-compose.yml` 及 AGENTS.md |

### 4.3 优先级排序

| 优先级 | 动作 | 影响范围 |
|--------|------|----------|
| **P0** | 修 SYSTEM-OVERVIEW.md 3 处漂移（端口×2 + 进程×1） | 确立当前权威总览的准确性 |
| **P0** | 修 README.md 第九节速查卡（D27: 8域→12域） | 入口导航不误导 |
| **P1** | 为 9 篇多版本/旧栈文档头部加"历史参考"标注（见 4.2） | 消除"谁说了算"的歧义 |
| **P1** | 修 BUSINESS-FLOWS.md / MODULARITY.md 的 :8080（D3/D4） | 与 SYSTEM-OVERVIEW 对齐 |
| **P2** | rust-enterprise/ 全组 8域→12域、60+→143 批量替换（D37-D49） | 子目录一致性 |
| **P2** | 14-REPOSITORY-FULL-MAP.md 73 crate→143、8域→12域、8600/8601 标注 legacy（D15/D16/D32-D36） | 仓库地图准确性 |
| **P3** | rust-enterprise/06-ai-engine-api.md curl 示例端口（D8-D10） | API 示例可用性 |

---

## 五、附录：grep 扫描元信息

- 扫描工具：Grep（ripgrep），`docs/architecture/` 递归
- 扫描模式与命中：
  - `:8080` → 10 命中（D1-D10），无历史对照上下文
  - `五进程` → 1 命中（D11）
  - `48 crate|48个|48 个` → 2 命中（D12/D14）
  - `8 域|8域|八大域` → 21 命中（D25-D49，含子目录）
  - `3010` → **0 命中**（干净）
  - `60\+|73 crate|60 crate|143 crate` → 17 命中（143 crate 正确 3 处，60+/73 待修 D15-D24）
  - `8600|8601` → 42 命中（含 ISO 8601 时间格式误命中 4 处，实际端口漂移分布见 3.5）
