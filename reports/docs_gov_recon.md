# 文档治理侦察报告（docs_gov_recon）

> 侦察日期：2026-09-27　·　性质：**只读侦察**，未修改仓库任何文件
> 权威依据：`docs/ARCHITECTURE-OF-DOCS.md`（DOC-GOV-ARC-V1.0 / v1.1）
> 范围：`docs/expert-alliance/`、`docs/normalization/`、`docs/API-REGISTRY.md`、`docs/api/PORT-REGISTRY.md`、`docs/modules/CODE-CATALOG.md`、四个门禁脚本、`docs/working-reports/`

---

## 1. L0–L8 速查

铁律：**一个主题一个目录**、**一个事实一个权威源**、**路径即契约**（移动=迁移走 §6 流程）。

| 层 | 目录 | 职责 | 权威等级 | 报告落点？ |
|---|---|---|:--:|---|
| **L0 入口** | `README.md` · `docs-hub/` | 分层导航、交互式文档中心 | 🟢 导航 | 否 |
| **L1 概览** | `CORE-CAPABILITIES.md` · `ROADMAP-DOMAINS.md` · `API-REGISTRY.md` | 能力/路线图/接口总账 | 🟢 权威 | 否（`API-REGISTRY.md` 脚本生成，禁手改） |
| **L2 架构** | `architecture/` | 架构事实（总览/元架构/微服务/Rust/前端/图谱） | 🟢 权威 | 否 |
| **L3 域与模块** | `modules/` · `expert-alliance/` | 业务域与模块说明、流程、产品手册 | 🟢/🟡 | 否（新域文档须登记层 README） |
| **L4 接口与数据** | `api/` · `database/` | REST/TCP/端口契约 + DDL | 🟢 权威 | 否（端口事实=PORT-REGISTRY.md） |
| **L5 规范与治理** | `standards/` · `normalization/` | 编码/流程/归一化标准 + BP/API/ARC/TPL/VAL 索引 | 🟢 权威 | 否 |
| **L6 企业与规格** | `enterprise/` · `specifications/` | 需求→架构→设计→交付+ADR；规格任务包 | 🟢 权威 | 否 |
| **L7 报告与验证** | `working-reports/` | **过程报告/验证证据/审计/基准** | 🟡 证据（非权威） | ✅ **新增报告落这里** |
| **L8 归档** | `_archive/`（含 `expert-alliance/_archive/v1-v3`） | 历史快照，只读，禁止作为权威引用 | 🔴 只读 | 否 |

权威标记：🟢 以此为准；🟡 过程/证据；🔴 归档只读。
报告类命名（§2.2）：`<YYYYMMDD>_<slug>_<type>.md` 或 `<slug>-<YYYYMMDD>.md`；类型取 `plan`/`report`/`verification_report`/`benchmark`/`governance`。

---

## 2. expert-alliance 现有文档清单与缺口

### 2.1 现役文档（`docs/expert-alliance/`，7 份编号 + 1 份权威）

| 文件 | doc_id | 权威 | 内容 | 是否有空壳/TODO |
|---|---|:--:|---|---|
| `01-prd.md` | EA-PRD-001 | 🟢 | 产品定位、F-01~F-10 功能清单、5 角色权限、关键参数、术语表 | 完整，无 TODO |
| `02-architecture.md` | EA-ARCH-001 | 🟢 | 六层分层（L1 SDK→L6 接入）、16 crate 拓扑、4 进程端口、依赖矩阵、技术选型、Phase1-4 路线 | 完整 |
| `03-business-flow.md` | EA-FLOW-001 | 🟢 | 端到端 6 步、5 维匹配、7 模式 DAG 拓扑、Dynamic 决策短路、DAG 调度循环、6 融合策略路由、异常重试降级、专家注册心跳 | 完整 |
| `04-state-machine.md` | EA-SM-001 | 🟢 | 任务/节点/专家/审批状态机、评分权重表、Dynamic 阈值常量、融合规则、重试降级、一致性规则 | 完整 |
| `05-data-model.md` | EA-DATA-001 | 🟢 | Task/Node/CollaborationPlan/Expert 字段表 + 全部枚举；存储架构 | 完整；但 FusionStrategy 列了 **9 个**枚举值 |
| `06-api-spec.md` | EA-API-001 | 🟢 | 通用约定 + **自称 50 个端点**分 7 组 + 请求/响应示例 + WebSocket + 错误码 | 完整；但与 §2.3 现状有出入 |
| `07-deployment.md` | EA-OPS-001 | 🟡 | 4 进程清单、本地启动脚本、docker-compose 片段、Phase1-4 演进、`/metrics` 指标、配置项 | 完整（🟡 参考级） |
| `CURRENT-ARCHITECTURE.md` | EA-DOC-CURRENT V1.1 | 🟢 **唯一权威** | 代码事实逐条核对：16 crate 拓扑、11 个网关 .rs 文件行数、调度器/执行器/网关模块表、数据流（本地 vs 远程）、存储持久化表、API 计数、两套内置专家、与 v3 差距、关键决策 | 见下方缺口 |

### 2.2 `CURRENT-ARCHITECTURE.md` 的缺口与未核项

- **章节跳号**：§3.3 之后直接跳 §3.6、§3.7——**§3.4、§3.5 缺失**（无标题占位，疑似删改后留洞）。
- **§8 / §9 自述未重核**：frontmatter 明确写「§8/§9 本轮未重核，沿用 V1.0，未重核即不声称已核」。
- **底部 3 条断链**（`check-doc-links.py` 实测确认）：第 327 行的原始链接如下（历史字面记录，不是本报告的导航链接）：

```text
[v3 架构优化设计](v3/README.md)
[专家注册表协议](expert-registry-and-protocol.md)
[知识图谱Schema](knowledge-graph-schema.md)
```

这三个文件实际都在 `_archive/v1/`、`_archive/v3/` 下，相对路径没补 `_archive/` 前缀。
- **§2.1 运行时进程表漏登 registry-svc:3400**（表内 3080 列了两次：gateway + "模块化网关"）。
- 行数表自注「无门禁校验，跨版本必然漂移」，不要拿里面的行数做结论。

### 2.3 01–07（V1.0 设计态）与 CURRENT-ARCHITECTURE（V1.1 实现态）的事实冲突

| 主题 | 01–07 说法 | CURRENT-ARCHITECTURE（实现态权威） | 以谁为准 |
|---|---|---|---|
| WebSocket 进度推送 | `06-api-spec.md` §4 声明 `/ws/v1/experts/tasks/:task_id/progress` | §6.1 V1.1 补记：网关全 crate `WebSocketUpgrade` **零命中**；实时性只有 SSE `GET /api/alliance/tasks/:id/logs/stream` | CURRENT-ARCHITECTURE |
| 端点计数 | 06 自称「50 个端点」 | §6.1：`/api/experts/*` 43 + `/api/alliance/*` 20（去重路径口径）；前端台账 74 是 (path,method) 行口径 | CURRENT-ARCHITECTURE |
| 融合策略数 | 01/03/04 说 **6 种** | 05-data-model 列了 **9 个** FusionStrategy 枚举（voting/weighted/confidence_weighted/concatenation/best_of/stacking/debate/map_reduce/iterative） | **未对齐** |
| 健康检查 | 03/04 把健康当过滤项 | §4.2 补记：`is_healthy` 折成 0.15 权重参与总分，**不是过滤**；不健康专家仍入选，仅排后 | CURRENT-ARCHITECTURE |
| 内置专家 | 01/05 不区分来源 | §7 明确**两套别混**：7.1 调度器侧 11 个 `expert-<domain>` 模块目录；7.2 网关种子 10 位 `exp-*-001` 具名专家；两者 id 零交集 | CURRENT-ARCHITECTURE |
| 条件分支节点 | 02 编排暗示支持条件分支 | §3.7 补记：代码里**没有条件分支/重试回路**，只有固定步骤表 + Kahn 拓扑 | CURRENT-ARCHITECTURE |

> 01–07 的 frontmatter 是 `2026-09-25` 写的设计态文档；**写新报告时引用"当前实现"事实必须查 CURRENT-ARCHITECTURE.md，不要抄 01–07**。

---

## 3. normalization 与注册表格式（抽样）

- **`docs/normalization/README.md`**（DOC-NORM-HUB-V1.0）：5 类索引枢纽——BP（业务流程）/API（接口契约）/ARC（架构）/VAL（验证）/TPL（模板）。登记规则：`{前缀}-{两位序号}-{中文短名}.md`，放 `business/` 子目录；新文档须在对应 `*-INDEX.md` 登记 + `enterprise/00-INDEX.md` 变更记录留痕。
- **`BP-INDEX.md` 登记行示例**：`| 政务 gov | 事项申报→受理→审批→出证→监管 | TPL-05 工作流 + TPL-02 树表 | mox_sys + iam(sso) + audit |`
- **`API-INDEX.md` 登记行示例**：`| alliance.js | runAllianceFullSSE, getAllianceCapabilities | /api/alliance | 专家联盟 SSE |`（SSoT = `frontend-ui/src/MODULE-MANIFEST.md` §4）
- **`ARC-INDEX.md` 登记行示例**：`| ea | domain | logical | mox_sys, iam | expert_registry, collaboration_dag, case_memory |`（SSoT = `docs/database/mox_sys/module-registry.yml`）
- **`docs/API-REGISTRY.md`**：由 `scripts/doc/gen-api-registry.py` 从 `gateway/src/actuator.rs ROUTES` + `routes.rs DOMAINS` 自动生成，表头即「声明即实现」。当前 236 条路由 / 46 域描述符。**禁手改**；CI 跑「重生成 + diff」门禁，漂移即红。
- **`docs/api/PORT-REGISTRY.md`**（PORT-REGISTRY-001 V1.2）：全仓端口唯一权威。分类 RUNTIME / ALLIANCE / ANCILLARY / LEGACY / DEPRECATED / TEST-ONLY / THIRD-PARTY。关键端口：网关 **3080**、前端 3020、调度 3100、执行 3200、专家桥 3300、注册中心 3400、codeengine 3210。DEPRECATED 禁止复用：**3010 / 3021 / 3717 / 8101–8104 / 8081–8082**。
- **`docs/modules/CODE-CATALOG.md`**：由 `scripts/registry/module_catalog.py` 从 `cargo metadata` 生成，表头自注「请勿手改」。原文件登记行示例（相对路径属于原目录）：

```text
| [mox-alliance-executor-svc](<../../platform/domains/alliance/svc/mox-alliance-executor-svc/Cargo.toml>) | svc | mox-alliance-executor | runtime: ... |
```

---

## 4. 门禁脚本用法（只读阅读，未运行修改类）

| 脚本 | 用法 | 触发失败条件 | 新报告是否受影响 |
|---|---|---|---|
| `scripts/doc/gen-api-registry.py` | 无 `--check`；运行即**覆写** `docs/API-REGISTRY.md`。CI 跑「生成后 diff」门禁 | 已提交的 API-REGISTRY.md 与 `actuator.rs ROUTES` 重生成结果不一致 | 否（除非你改了路由）。**不要手动跑它来"修"文档** |
| `scripts/gate/verify-ports.py` | `python scripts/gate/verify-ports.py [--json] [--repo <path>]`。扫描根含 `docs/`（仅排除 `docs/_archive/`） | **ERROR**（退出 1）：DEPRECATED 端口仍被活跃代码/配置引用、`platform_config.json` 与注册表不一致、一端口多服务。**WARN**（不致命）：出现未登记端口、DEPRECATED 端口仅出现在文档里 | **会扫到 working-reports/**。新报告里出现未登记端口号→WARN；引用 3010/3717 等历史端口→WARN（文档里是历史证据，不致命）。只引用已登记端口（3080/3100/3200/3400/3020 等）即完全干净 |
| `scripts/gate/check-doc-links.py` | `python scripts/gate/check-doc-links.py [--all] [--repo] [--strict] [--json out.json]`。扫 docs/ 下 .md/.html，默认跳过 `_archive/` | 退出 1 = 存在断链（Markdown `[](...)`、HTML `href/src`、反引号 `docs/...` 路径）。围栏代码块、`${}`、`file://`、`<!-- check-doc-links:ignore-start/end -->` 豁免区不算 | **直接受影响**。新报告里每个 `[](相对链接)` 都必须真实存在；反引号包的 `` `docs/...` `` 路径默认仅 WARN，`--strict` 后算错误 |
| `scripts/registry/module_catalog.py --check` | 对比 `cargo metadata` 重渲染结果与已提交 CODE-CATALOG.md | 不一致即非零退出（提示重新生成） | 否（只读 Cargo.toml + frontend-ui/src + projects/，与 .md 报告无关）。注意：报错文案写的是 `python tools/module_catalog.py`，实际路径是 `scripts/registry/module_catalog.py`（文案过期） |

### 4.1 门禁现状实测（2026-09-27，只读运行）

- `check-doc-links.py` 实跑：**断链 41 条 / 7 文件**（基线 §7.1 声称 2026-09-21 为 0，已漂移）。重灾区：`docs-hub/docs-hub.html`（14）、`CODE-CATALOG.md`（8）、`docs/README.md`（8）、`architecture/README.md`（4）、`CURRENT-ARCHITECTURE.md`（3）——绝大多数是 expert-alliance v1/v2/v3 文档迁入 `_archive/` 后，上层导航页没改相对路径。**这些是既有技术债，新报告不要去碰、也不要新增同类断链**。
- `CODE-CATALOG.md` 实测漂移：磁盘上 alliance 域有 **16 个 crate**（含 `mox-alliance-registry-core/-proto/-svc` 三个），但 CODE-CATALOG.md 里 `mox-alliance-registry-*` **零命中**，且 8 条链接指向已不存在的前端文件（ForgotPassword.vue / Login.vue / alliance.js / auth.js 等）。即 `module_catalog.py --check` 当前大概率是红的——**新报告不要去重生成它**，那是代码侧变更触发的动作。

---

## 5. 新增一份 `docs/working-reports/` 报告的安全步骤清单

1. **放对位置**：直接放 `docs/working-reports/` 根目录；原始测试 stdout 放 `verification/`，一次性审计快照放 `audits/`；不要新开平级目录。
2. **起对名字**：优先 `<YYYYMMDD>_<英文或拼音 slug>_<type>.md`，type ∈ `plan`/`report`/`verification_report`/`benchmark`/`governance`（例：`20260927_expert_alliance_doc_gov_recon_report.md`）。轮次 HTML 用 `<主题>-round<N>-<YYYYMMDD>.html`。
3. **frontmatter 不强制 YAML**：现有报告多数直接 `# 标题` 开头，部分用元信息表或 `>` 引用块写日期/范围/性质。建议加一行 `> 日期 / 范围 / 性质（只读核验，未改源码）`。**不要**在报告里声称 🟢 权威——L7 一律 🟡 证据。
4. **引用要闭环**：
   - 指向仓库内文档用仓根相对路径 `docs/...`（§4.1 铁律；新增必须遵守，旧文件不回改）；
   - 相对链接务必真的存在——写完自查 `python scripts/gate/check-doc-links.py`，**不要再增加新的断链**；
   - 引用 `_archive/` 里的 v1/v2/v3 文档时，明确写「历史设计态，非权威」。
5. **事实单一来源**：写"专家联盟当前实现"时查 `CURRENT-ARCHITECTURE.md`（V1.1），不要照抄 01–07；写端口只引用 PORT-REGISTRY.md 已登记的号；写接口数查 API-REGISTRY.md（236 条口径），不要用 06-api-spec 的"50 个"。
6. **登记到层索引**：在 `docs/working-reports/README.md` 对应分组表（阶段交付 / 基准性能 / 专家联盟专项 / 归一化治理 / 计划调研）加一行 `文件名 | 一句话主题`。
7. **跑校验（只读）**：
   - `python scripts/gate/check-doc-links.py`——确认断链数不增加（基线已 41，不新增即合格）；
   - `python scripts/gate/verify-ports.py`——确认新报告只引用已登记端口，不新增 ERROR；
   - **不要跑** `gen-api-registry.py`（会覆写 API-REGISTRY.md）和 `module_catalog.py`（不带 `--check` 会覆写 CODE-CATALOG.md）。
8. **不要做的事**：不回填 L1–L6 权威文档（那是后续独立 PR）；不重命名/迁移旧报告；不动 `_archive/`；不在报告里"修复"现有 41 条断链。

---

## 6. normalization ↔ expert-alliance 待对齐点

1. **融合策略数量**：01/03/04 说 6 种，05-data-model 枚举列 9 个，CURRENT-ARCHITECTURE §3.7 又说 7 模式。需在 BP-INDEX / ARC-INDEX 定一个口径。
2. **实时性通道**：06-api-spec 写 WebSocket，CURRENT-ARCHITECTURE 证伪为 SSE；API-INDEX §1 写 `alliance.js` 走 SSE 是对的，06 需对齐。
3. **端点计数三套口径**：06 自称 50、CURRENT-ARCHITECTURE 43+20（去重路径）、前端台账 74（path+method 行）、API-INDEX 写"专家联盟 15"。建议在 API-INDEX 补一行口径说明。
4. **两套内置专家**：CURRENT-ARCHITECTURE §7.1/§7.2 已明确切分，01-prd F-06 / 05-data-model Expert 实体没体现，后续权威回填时需同步。
5. **端口覆盖**：07-deployment.md 进程表缺 3300（专家桥）、3210（codeengine）、33080/33100/33200（本地联盟栈）；PORT-REGISTRY 已登。
6. **品牌串残留**：`docs/normalization/README.md` 第 4 行仍有「mox 模块化系统」字样（P0-1 正文替换声称已完成，此处漏网）。
7. **健康字段语义**：03/04 把健康当过滤条件，CURRENT-ARCHITECTURE §4.2 纠正为 0.15 加权项——BP-INDEX §2「专家联盟调度」行措辞需同步。

---

## 7. 红线（不要碰）

- 🟢 权威源**只读**：`docs/API-REGISTRY.md`（脚本生成）、`docs/api/PORT-REGISTRY.md`、`docs/enterprise/GLOSSARY.md`、`docs/expert-alliance/CURRENT-ARCHITECTURE.md`、`docs/modules/CODE-CATALOG.md`（脚本生成）。**除非真有端口/路由变更，否则一个字都不改**。
- 不与既有 🟢 条目冲突；发现冲突先在 `enterprise/22-...权威链单源映射表` 提对齐，不要在报告里另立说法。
- **DEPRECATED 端口禁止复用**：3010 / 3021 / 3717 / 8101–8104 / 8081–8082；新端口必须走 PORT-REGISTRY §5 变更流程。
- 不动 `docs/_archive/`（含 `expert-alliance/_archive/v1-v3`）——只读历史快照。
- 不跑 `gen-api-registry.py` / `module_catalog.py`（无 `--check` 即覆写）；本次侦察已规避。
- 不在 working-report 里"顺手修复"现存 41 条断链或 CODE-CATALOG 漂移——那是独立治理项，本报告只记录。
- 不新开与 L7 平级的报告目录；报告结论固化后应回填 L1–L6，本层只留证据。

---

*侦察完成。未修改仓库任何文件；本文件输出至 `reports/docs_gov_recon.md`。*
