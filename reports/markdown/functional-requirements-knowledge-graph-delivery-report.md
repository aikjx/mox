# 功能需求知识图谱（FR-KG）交付报告

> **日期**
>
> ：2026-09-02
> **任务**
>
> ：所有功能需求先上知识图谱 — 将 MOX / 璇玑 平台全量功能需求建模并录入平台自研知识图谱（KG），使 KG 成为功能需求的权威单一事实源（SSOT），后续一切开发以 KG 为驱动。
> **关联报告**
>
> ：
>
> [production-hardening-delivery-report.md](./production-hardening-delivery-report.md)
>
> （生产化收口主报告）、
>
> [api-backend-gap-audit.md](./api-backend-gap-audit.md)
>
> 、
>
> [frontend-views-gap-audit.md](./frontend-views-gap-audit.md)
>
> 、
>
> [database-normalization-plan.md](./database-normalization-plan.md)



***

## 一、交付总览



| 指标           | 数值                |
| ------------ | ----------------- |
| **录入功能需求节点** | 687               |
| **录入功能需求关系** | 776               |
| **图谱最终总节点**  | 702（含 15 个系统架构节点） |
| **图谱最终总关系**  | 793（含 17 条系统架构边）  |
| **覆盖业务域**    | 19 个              |
| **录入失败**     | 0                 |
| **验证通过**     | 9/9 + 独立复核        |
| **幂等性**      | 重复执行 0 新增 ✓       |



***

## 二、KG 领域模型定义

### 2.1 实体类型（node\_type）



| node\_type         | 说明        | ID 前缀    | 数量  |
| ------------------ | --------- | -------- | --- |
| `domain`           | 业务域       | `dom_`   | 19  |
| `module`           | 功能模块      | `mod_`   | 74  |
| `feature`          | 功能点（具体需求） | `feat_`  | 74  |
| `frontend_page`    | 前端页面 / 视图 | `page_`  | 49  |
| `api_function`     | 前端 API 函数 | `apifn_` | 353 |
| `backend_endpoint` | 后端端点      | `ep_`    | 92  |
| `gap`              | 缺口 / 缺失项  | `gap_`   | 20  |
| `pending_decision` | 归一化待决策项   | `dec_`   | 6   |

### 2.2 节点 properties 字段规范

所有节点通过 `properties`（JSON 对象）存储元数据：



* **通用字段**：`domain`（所属业务域 ID）、`description`（描述）

* **feature 节点**：`status`（implemented / frontend\_only / backend\_only / partial / planned / gap）、`priority`（P0 / P1 / P2 / P3）、`missing_backend`（bool）

* **api\_function 节点**：`file_path`（所在 API 文件）、`function_name`、`http_method`、`target_path`（调用的 URL 路径）

* **backend\_endpoint 节点**：`http_method`、`path`、`handler`、`source`（orchestrator / gateway / primiflow）

* **frontend\_page 节点**：`file_path`、`route_path`

* **gap 节点**：`severity`（P0 / P1 / P2）、`status`（open / resolved）、`affected_count`

* **pending\_decision 节点**：`status`（pending）、`decision_options`

### 2.3 关系类型（relation\_type）



| relation\_type | source → target                   | 数量  | weight | 说明           |
| -------------- | --------------------------------- | --- | ------ | ------------ |
| `belongs_to`   | module → domain, feature → module | 148 | 0.9    | 归属关系         |
| `has_function` | feature → api\_function           | 353 | 0.8    | 功能点包含 API 函数 |
| `has_endpoint` | feature → backend\_endpoint       | 92  | 0.8    | 功能点有后端端点     |
| `calls`        | api\_function → backend\_endpoint | 87  | 0.85   | API 函数调用端点   |
| `implements`   | frontend\_page → feature          | 77  | 0.7    | 页面实现功能点      |
| `blocked_by`   | feature → gap                     | 18  | 0.95   | 功能点被缺口阻塞     |
| `related_gap`  | pending\_decision → gap           | 1   | 0.6    | 待决策关联缺口      |



***

## 三、录入规模与覆盖

### 3.1 按业务域统计（feature 节点，共 74 个）



| 域         | feature 数 | 主要状态                                |
| --------- | --------- | ----------------------------------- |
| system    | 11        | 网关原生 IAM 全覆盖                        |
| ai        | 9         | 对话 / 算法已实现，全维分析 / 无穷优化前端\_only      |
| expert    | 7         | **全域前端\_only**，50+ 函数无后端            |
| platform  | 6         | 网关 / 子服务 / KGv1/AIEngine 已实现        |
| workflow  | 6         | 流程图 / 插件 / MCP / 浏览器已实现，自动化 partial |
| graph     | 5         | 核心图谱已实现（17 端点），AI 图谱增强缺失            |
| kb        | 5         | **全域前端\_only**，21 函数无后端             |
| alliance  | 5         | 独立 fetch 客户端，Vite 代理未配置             |
| llm       | 4         | 旧接口已实现，新 /api/llm/\* 缺失             |
| security  | 3         | 已实现（RBAC + 审计 + HITL）               |
| project   | 3         | 资源已实现，任务全域缺失                        |
| operators | 2         | 算子已实现，商城 partial                    |
| mox       | 2         | 已实现                                 |
| melody    | 1         | 独立 FastAPI :8012 未被网关代理             |
| storage   | 1         | 仅 legacy 存在                         |
| caomei    | 1         | 已实现                                 |
| voice     | 1         | 网关代理 :30010，partial                 |
| admin     | 1         | 15 面板已实现                            |
| misc      | 1         | 登录 / 门户 / 大厅 / 403 已实现              |

### 3.2 功能点状态分布（74 个 feature）



| 状态                              | 数量 | 占比    |
| ------------------------------- | -- | ----- |
| implemented（前后端均已实现）            | 36 | 48.6% |
| frontend\_only（前端有页面 / 函数，后端缺失） | 30 | 40.5% |
| partial（部分实现）                   | 8  | 10.8% |

> **口径说明**
>
> ：feature 级 implemented 表示该功能点的前后端均已实现。审计口径 API 对接率～85% 是按 348 个前端 API 函数中有后端支撑的比例统计，两者口径不同。详见 
>
> [production-hardening-delivery-report.md](./production-hardening-delivery-report.md)
>
> 。



***

## 四、缺口与待决策如何在 KG 中体现

### 4.1 缺口（gap 节点，共 20 个）

每个缺口以独立 `gap` 节点存在，包含 `severity`（P0/P1/P2）、`status`（open）、`affected_count`、`description`。被缺口阻塞的功能点通过 `blocked_by` 关系指向对应 gap 节点（共 18 条 blocked\_by 边）。

**P0 缺口（7 项，阻断生产上线）**：



| gap ID                       | 描述                                                | 影响域      |
| ---------------------------- | ------------------------------------------------- | -------- |
| `gap_experts_api`            | 后端缺失 /api/experts/\*，前端约 50 个函数无后端支撑              | expert   |
| `gap_tasks_api`              | 后端缺失 /api/tasks/\*，9 个函数无后端                       | project  |
| `gap_kb_api`                 | 后端缺失 /api/kb/\*，21 个函数无后端                         | kb       |
| `gap_llm_api`                | 后端缺失 /api/llm/\*，17 个函数无后端，仅有旧 /api/ai/llm/config | llm      |
| `gap_full_analysis_api`      | 全维分析 6 个函数无后端                                     | ai       |
| `gap_alliance_experts_proxy` | alliance.js 直接 fetch /experts/\*，Vite 未配置代理       | alliance |
| `gap_frontend_mock_data`     | Dashboard/ExpertWorkspaceView 仍有硬编码假数据            | frontend |

**P1/P2 缺口（13 项）**：包括联网搜索缺失、无穷维度优化缺失、制品引擎缺失、项目一体化缺失、专家图谱缺失、旋律转谱代理缺失、存储 API 缺失、权限指令零使用、i18n 未建、无障碍未建、联盟引擎缺失、联盟任务代理缺失等。

### 4.2 归一化待决策（pending\_decision 节点，共 6 个）

每项待决策以独立 `pending_decision` 节点存在，`status=pending`，包含 `decision_options` 描述可选方案。**仅建节点，不替用户拍板。**



| decision ID                       | 标题                                                      | 域        |
| --------------------------------- | ------------------------------------------------------- | -------- |
| `dec_python_legacy_migration`     | Python legacy 数据全量迁移到主库（全量 / 仅关键 / 不迁移）                 | data     |
| `dec_orphan_messages_table`       | mox\_meta.db.messages 孤儿表迁移后是否 DROP                     | data     |
| `dec_legacy_user_iam_integration` | Python legacy 用户体系是否对接 Rust IAM                         | security |
| `dec_business_tables_schema`      | 业务表 (products/news/cases) 是否纳入主库统一 schema               | data     |
| `dec_static_sites_merge`          | mox-website/mox-console/mox-store 三个静态 HTML 站是否合并进主 SPA | frontend |
| `dec_legacy_backend_rust_archive` | legacy backend-rust (\~200 端点) 是否移植进编排器后归档              | platform |



***

## 五、前端查看方式

### 5.1 访问入口

打开 `http://localhost:3020/graph`（前端 Vite dev server），左侧面板顶部新增 \*\*"需求图谱过滤"\*\* 区域（默认展开）。

### 5.2 过滤能力



| 过滤维度      | 功能                | 说明                                                              |
| --------- | ----------------- | --------------------------------------------------------------- |
| **视图模式**  | 全部 / 仅功能需求 / 系统架构 | 三档切换，"仅功能需求" 隐藏系统架构节点只显示 8 种功能需求节点                              |
| **业务域过滤** | 下拉选择（全部域 + 19 个域） | 按 `properties.domain` 过滤，支持清空                                   |
| **只看缺口**  | 快捷按钮              | 显示 gap 节点 + 与之有 blocked\_by 关系的 feature 节点                      |
| **只看待决策** | 快捷按钮              | 显示 pending\_decision 节点 + 所有与之相连的节点                             |
| **只看未实现** | 快捷按钮              | 显示 feature 节点且 status ∈ {frontend\_only, partial, planned, gap} |
| **重置**    | 快捷按钮              | 清除所有过滤条件，恢复全图                                                   |
| **过滤统计**  | 实时显示              | "当前显示：N 节点 / M 边"                                               |

### 5.3 节点颜色映射（8 种功能需求类型）



| node\_type        | 颜色                |
| ----------------- | ----------------- |
| domain            | indigo (#6366f1)  |
| module            | violet (#8b5cf6)  |
| feature           | emerald (#10b981) |
| frontend\_page    | cyan (#06b6d4)    |
| api\_function     | amber (#f59e0b)   |
| backend\_endpoint | blue (#3b82f6)    |
| gap               | red (#ef4444)     |
| pending\_decision | orange (#f97316)  |

### 5.4 修改文件

仅修改 `frontend-ui/src/views/graph/GraphView.vue`（54KB → 62KB，2150 行），未修改 graph.api.js、路由配置或其他视图文件。现有搜索、布局、样式调节、AI 分析、快捷分析等功能完全保持不变。



***

## 六、验证结果

### 6.1 录入子任务自检（9/9 通过）



| # | 验证项             | 方法                                         | 结果                                    |
| - | --------------- | ------------------------------------------ | ------------------------------------- |
| 1 | 回读验证            | GET /api/graph，统计节点 / 边数                   | 702 = 15 + 687 ✓，793 = 17 + 776 ✓     |
| 2 | 按 node\_type 统计 | 对回读节点分组计数                                  | 全部 8 种类型数量与预期一致 ✓                     |
| 3 | 按域统计            | feature 节点按 properties.domain 分组           | 19 个域数量全部匹配 ✓                         |
| 4 | 缺口 / 待决策验证      | 统计 gap 和 pending\_decision 节点              | gap=20 ✓，pending\_decision=6 ✓        |
| 5 | 实现率验证           | 统计 feature status=implemented 比例           | 36/74=48.6%（口径说明见 3.2）✓               |
| 6 | 邻居查询            | GET /api/graph/neighbors/feat\_graph\_core | 返回 24 个关联节点 ✓                         |
| 7 | stats 验证        | GET /api/graph/stats                       | nodes=702, edges=793, density=0.003 ✓ |
| 8 | 幂等性             | 重复执行录入脚本                                   | 0 新增，全部跳过 ✓                           |
| 9 | 系统架构节点保护        | 检查原有 15 个节点                                | 未被修改 ✓                                |

### 6.2 独立复核（Organizer 执行）



* **GET /api/graph/stats**：确认 nodes=702, edges=793, density=0.003, components=1 ✓

* **GET /api/graph 节点类型分布**：api\_function=353, backend\_endpoint=92, module=74, feature=74, frontend\_page=49, gap=20, domain=19, pending\_decision=6，与录入子任务报告完全一致 ✓

* **feature 按域分布**：19 域全部存在，system=11, ai=9, expert=7 等 ✓

* **gap 节点清单**：20 个 gap 节点 ID 全部可查（gap\_experts\_api, gap\_tasks\_api, gap\_kb\_api 等）✓

* **pending\_decision 节点清单**：6 个待决策节点全部可查 ✓

* **前端文件确认**：GraphView.vue 已修改（62KB，2026-09-02），FEATURE\_NODE\_TYPES、filteredGraphData、需求图谱过滤、只看缺口、只看待决策等关键词均存在 ✓

* **Vite HMR 验证**：JS 模块 200、CSS 模块 200、无编译错误 ✓



***

## 七、持久化与恢复

### 7.1 当前存储机制

平台 KG 为**内存态**（编排器内置 `mox_kg_algo_core::KnowledgeGraph`），通过网关 :8080 暴露 REST API。服务重启后内存数据丢失。

### 7.2 恢复机制

已提供完整的种子数据 + 幂等录入脚本，服务重启后可一键恢复：



```
cd D:\a10\aikjx\gitcode\infotopograph

python platform/domains/kg/seed/ingest\_functional\_requirements.py
```

录入脚本特性：



* **幂等**：先 GET 现有节点，跳过已存在 ID，重复执行不产生重复

* **进度显示**：实时打印录入进度

* **dry-run**：支持 `--dry-run` 参数只统计不实际录入

* **中文编码**：全程 UTF-8

### 7.3 交付物清单



| 文件              | 路径                                                                   | 大小       |
| --------------- | -------------------------------------------------------------------- | -------- |
| 功能需求清单（盘点源数据）   | `platform/domains/kg/seed/functional-requirements-inventory.json`    | 94.5 KB  |
| KG 种子数据（节点 + 边） | `platform/domains/kg/seed/functional-requirements-graph-seed.json`   | 376.3 KB |
| 种子生成脚本          | `platform/domains/kg/seed/generate_seed.py`                          | 15.2 KB  |
| 录入脚本（幂等）        | `platform/domains/kg/seed/ingest_functional_requirements.py`         | 8 KB     |
| 验证脚本            | `platform/domains/kg/seed/verify_ingestion.py`                       | 9.3 KB   |
| 前端过滤适配          | `frontend-ui/src/views/graph/GraphView.vue`                          | 62 KB    |
| 本交付报告           | `reports/functional-requirements-knowledge-graph-delivery-report.md` | —        |



***

## 八、与生产化收口报告的交叉引用

本报告与 [production-hardening-delivery-report.md](./production-hardening-delivery-report.md) 数据口径一致：



| 维度         | 生产化收口报告                                                      | 本 FR-KG 报告                                                                             | 一致性 |
| ---------- | ------------------------------------------------------------ | -------------------------------------------------------------------------------------- | --- |
| API 对接率    | \~85%                                                        | \~85%（348 前端 API 函数口径）                                                                 | ✓   |
| 缺失端点       | /api/experts/*, /api/tasks/*, /api/melody2score/*, /api/kb/* | gap\_experts\_api, gap\_tasks\_api, gap\_melody2score\_api, gap\_kb\_api 等 20 个 gap 节点 | ✓   |
| 6 项归一化待决策  | 列出 6 项                                                       | 6 个 pending\_decision 节点                                                               | ✓   |
| i18n / 无障碍 | 未建                                                           | gap\_i18n, gap\_accessibility 节点                                                       | ✓   |
| 三服务架构      | 网关：8080 + 编排器：3001 + PrimiFlow:8000                          | KG API 通过网关：8080 暴露                                                                    | ✓   |
| 数据库双写      | messages 双写已修复，写入统一到 mox\_business.db                        | 作为已完成事实，不在 gap 中                                                                       | ✓   |



***

## 九、后续建议



1. **KG 持久化**：当前 KG 为内存态，建议后续将种子数据加载机制固化到编排器启动流程中（读取 seed JSON 自动加载），或引入持久化存储层。

2. **图谱 API 收敛**：当前编排器有 `/api/graph/*`（12 端点），网关原生有 `/kg/v1/*`（6 端点），两套图谱 API 并存，建议后续收敛为统一接口。

3. **缺口驱动开发**：后续补功能 / 接口时，以 KG 中的 gap 节点为任务来源，完成后更新对应 feature 节点的 status 和 blocked\_by 关系，实现 KG 驱动的开发闭环。

4. **待决策闭环**：6 项 pending\_decision 节点需用户拍板后更新 status，并在 KG 中记录决策结果。

5. **权限指令落地**：`v-permission` 指令已定义但零使用，建议后续结合 KG 中的 feature 节点优先级逐步落地路由级权限控制。



***

*报告生成时间：2026-09-02 | 数据来源：代码全量扫描 + 四份审计报告 + KG API 回读验证*