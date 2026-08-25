# 璇玑·开发专家联盟 — 架构诊断与 SaaS AI 平台化最优方案 V1.0

> **文档定位**：基于真实源码（2026-08-26 现场取证）的架构评估 + 可落地优化路线图，区别于纯行业最佳实践。
> **取证范围**：ExpertCenterView.vue / ProjectPicker.vue / projectContext.js / types.js / Cargo.toml / .gitignore / projects.js 路由 / platform_config.json / 根目录与数据目录审计。
> **版本**：Mox 3.0 · 企业级

---

## 一、对原分析的事实校准（先纠偏，后诊断）

原分析是"基于目录结构推断 + 行业标准"的草稿级诊断，**6 项问题中 4 项属实但程度不同，2 项存在严重误判**。先澄清事实：

| # | 原诊断 | 真实取证结果 | 判定 |
|---|--------|-------------|------|
| 1 | 根目录严重污染，工程卫生极差 | ✅ 根目录 47 个垃圾文件（*.log / *.txt / NUL-*.d / *.rmeta），总计 ~850KB。但 .gitignore **已经正确写入**了 `*.log`、`graph.json`、`graph.enterprise.json`、`__pycache__`、`my_projects`、`workspace/artifacts`、`*.db` 等规则——问题是**历史遗留文件未清理 + 规则未强制执行**，而非"没有规范"。 | 部分属实，程度中等 |
| 2 | 功能重叠目录并存（projects/my_projects/workspace） | ✅ 三个目录确实都存在，但**语义边界其实清晰**：<br>• `projects/` = 平台级项目案例（melody2score / market-games），已入仓<br>• `my_projects/` = 用户本地 Rust crate 实验区（business-court-docs），.gitignore 已忽略整目录<br>• `workspace/` = AI 自动开发引擎运行时产物（artifacts + screenshots），.gitignore 已忽略子目录<br>**问题不是语义混乱，而是命名不够表达语义，新人读不懂三个目录的区别。** | 属实但根因错判 |
| 3 | 大文件直接入库（graph.json 87MB / graph.enterprise.json 86MB） | ✅ 两个合计 ~173MB 的 JSON 确实在根目录。.gitignore 已写 `graph.json` / `graph.enterprise.json`，但**大概率是 `git add -f` 强加入库**或 ignore 规则晚于提交。实际危害：clone 慢 2-3 分钟、GitHub Release 包膨胀。 | 属实，严重 |
| 4 | Monorepo 结构缺失，多语言边界模糊 | ❌ **严重误判**：`Cargo.toml` workspace 注册了 **42 个 Rust crates**，分层清晰：<br>• `platform/crates/` = 4 个计算核心 + 3 个 FFI 绑定（napi/PyO3）<br>• `platform/services/` = 26 个领域服务（mox-expert / ai-agent / kg-hub / flow-ai …）<br>• `platform/gateway/runtime` = Rust 网关（axum + 多路由域）<br>• `platform/sdk/` = Rust/Node/Python 三语 SDK<br>• `platform/backend-node/` = Node API 层（23 路由域 + JSON Store）<br>• `frontend-ui/` = Vue3 前端（25 视图 + 组件库）<br>**Rust 侧的 Monorepo 管理是项目最强项之一，远超行业平均水平。** 缺失的是：前端/Node/Python 没接入 pnpm/npm workspace，三语构建没统一入口脚本。 | 大误判，需修正 |
| 5 | SaaS 多租户架构无迹可寻 | ✅ **最致命的真实短板**：<br>• `platform_config.json` = 单实例（admin/admin123 硬编码）<br>• 所有后端 JSON Store（projects.json / experts.json / tasks.json）无 `tenant_id` 字段<br>• `projectContext.js` 的 HTTP 注入只挂 `project_id`，无 `tenant_id` / `org_id`<br>• RBAC 仅在 `mox-expert` crate 中有审计+S3 模块，无租户级 RLS<br>• `docs/enterprise/12-RBAC审计全链路闭环验收报告.md` 仅覆盖单租户 | 属实，致命 P0 |
| 6 | 运行时数据与源代码混杂 | ✅ `platform/backend-node/data/` 下 **68 个 JSON/SQLite 文件**（15.4MB），包含：<br>• `alliance_traces.jsonl` = 3.4MB 对话迹线<br>• `ous.db-wal` = 7.3MB SQLite WAL<br>• `ous.db` = 3.3MB<br>• `audit_log.json` / `llm_usage.json` 等运行时数据<br>虽 .gitignore 写了 `*.db`、`backend/data/tasks.json` 等，但漏了大量 JSON（projects.json、experts.json 等是种子数据还是运行时数据边界模糊）。 | 属实，中高 |

---

## 二、开发专家联盟系统现状评估（好的部分要肯定）

在诊断问题前，必须先肯定项目已经做对的事情——这些是**不能动的架构基石**：

### ✅ 已做对的 8 件事（架构红线，优化时禁止破坏）

#### 1. 以项目为根的 5 阶段 φ 生命周期模型
- `types.js` 中 `PROJECT_PHASES` + `NAV_GROUPS` 把 25 个业务模块**按 S1→S5 流水线归档**，`ExpertCenterView.vue` 的黄金比例三栏布局（左 0.382 阶段导航+专家库 / 中 0.618 AI 工作区 / 右 0.382 图谱追踪）是正确的 UX 决策。
- **这是专家联盟区别于通用聊天工具的核心差异化竞争力，优化时必须保留且强化。**

#### 2. 项目上下文单例 + 跨视图联动
`projectContext.js` 实现了：
- provide/inject + 模块级单例双模式（组件内外都能用）
- localStorage 持久化当前项目
- `registerProjectIdGetter()` 把项目 ID 注入所有 HTTP 请求
- `mox:project-updated` / `mox:project-deleted` 自定义事件跨视图刷新
- 自动选项目策略：saved → 第一个 active → 第一个

**这是全维项目化的技术地基，正确。**

#### 3. Rust Workspace 分层模型
42 个 crates 的分层：
```
platform/
├── crates/              # 计算密集核心（公式/归一化/意图/DSP）
│   └── bindings/        # napi-rs + PyO3 FFI 桥
├── services/            # 26 领域服务（业务逻辑单体）
├── gateway/runtime/     # axum 网关（路由/代理/边车）
└── sdk/                 # 三语 SDK（Rust/Node/Python）
```
`workspace.dependencies` 统一版本、`default-members` 避免 napi/PyO3 工具链阻塞日常构建——**这是企业级 Rust 工程的教科书级实践**。

#### 4. 全维资源目录聚合（projects.js /buildCatalog）
`RESOURCE_TYPES` 注册了 18 类资源（模块/MCP工具/插件/Agent/技能/循环体/自动化/工作流/流程图/专家/图谱节点/任务/算子/管线/知识库文档/大模型/服务/商城方案），`buildCatalog()` 实时从各 JSON Store 聚合，**每类资源都标注了前端路由**。这是"一切对象归项目"的正确实现方式。

#### 5. 专家联盟标准化实体定义
`EXPERT_TYPES` 15 种专家（算法/架构/数据/AI/工作流/算子/图谱/安全/性能/可观测/商业/MCP/自动化/需求/融合）+ `AI_EXPERT_PRESETS` 8 种对话预设 + `PROJECT_CATEGORIES` 11 种项目形态——**领域模型完整，直接可做多租户权限矩阵**。

#### 6. 快捷键与命令面板体系
`QUICK_CREATE_COMMANDS`（6 项新建）+ `HOTKEY_GROUPS`（全局/表单两组，Ctrl+K 命令面板 / Ctrl+Shift+N 新任务 / Shift+? 帮助 / Alt+1..9 跳模块）——**符合用户偏好中"键盘优先"的硬约束**。

#### 7. 服务管理器 + 健康检查体系
`platform_config.json` 定义了 api/frontend/xiaobai_voice 三个服务的端口、启动命令、依赖顺序（frontend depends_on api）、health_check 路径、restart_delay、auto_start——**单实例部署已有服务编排雏形**。

#### 8. 企业级文档体系已建立
`docs/enterprise/` 下 25+ 文档已覆盖：需求/架构/设计/业务流程/迭代路线/RBAC/禁伪代码/测试验证/竞品对比/AI引擎评测/三联盟模式/全文档归一化总控卡——**PMF 已有纸面闭环，缺的是把纸面变成代码架构。**

---

## 三、真实架构问题清单（按 P0-P4 分级 + 可验证证据）

### 🔴 P0：SaaS 化致命缺陷（不解决无法对外提供服务）

#### P0-1：零多租户隔离，数据层单实例全局共享
**证据**：
- `platform_config.json` 无租户配置，admin 密码硬编码 `"admin123"`
- `projects.json` / `experts.json` / `tasks.json` 等 68 个数据文件无 `tenant_id` / `org_id` 字段
- `projects.js` 路由 CRUD 不校验租户归属，任何人能读/改任何项目
- `projectContext.js` HTTP 头只注入 `project_id`，无 `tenant_id`

**风险**：一旦部署成多用户，A 客户能看/改 B 客户的所有数据。GDPR/等保 2.0 直接不合格。

#### P0-2：AI 调用链路零计量 + 零成本归因
**证据**：
- `llm_usage.json` = 184 字节（空壳），无 Token 消耗、请求数、延迟、失败率统计
- `platform/backend-node/src/llm-gateway.js` 若存在，未见计量中间件挂载
- 无 per-tenant / per-project / per-user 的用量维度

**风险**：上 SaaS 后不知道哪个客户花了多少钱，亏损无法定位；无配额（Quota）系统，单个用户能把 API Key 刷爆。

#### P0-3：敏感信息暴露风险
**证据**：
- `.gitignore` 虽排除了 `.env` 和 `backend/data/llm_config.json`，但 `platform_config.json` 中 `"admin": {"password": "admin123"}` 是**明文入仓**
- `docs/.trae/specs/` 可能含内部方案，但这是小问题

**风险**：默认凭据被公网扫到直接接管后台。

---

### 🟠 P1：工程卫生 & 仓库健康（影响开发体验 + CI/CD 稳定性）

#### P1-1：根目录历史垃圾文件堆积（47 个，~850KB）
**证据**：
- 40 个 `*.log`：`green_e.log` / `red_o.log` / `ck_e.log` / `meta3.log` 等 CI 门禁日志
- 5 个 `*.txt`：`members.txt` / `green_log.txt`(107KB) / `red_log.txt`(258KB) / `logs_d1d6.txt` / `green_exit.txt`
- 2 个构建残渣：`NUL-9abbf07bbedbe4ee.d` / `libmox_standards-*.rmeta`

**影响**：新成员 `git clone` 后看到根目录像垃圾场，`.log` 规则在 gitignore 中，说明是**未跟踪的本地文件**（非入仓问题，这点原分析误判了——但本地存在仍影响 grep/find 结果）。

#### P1-2：两个 87MB graph.json 大概率违规入仓
**证据**：LS 工具直接看到两个文件在根目录；.gitignore 写了排除规则但文件仍存在——两种可能：
1. `git add -f graph.json` 强制入仓
2. ignore 规则添加前已提交，需 `git rm --cached` 清除历史

**验证命令**：`git ls-files | grep graph.json`——若返回文件名则真入仓，否则只是本地未跟踪文件。

**影响**：真入仓则仓库体积不可逆膨胀，GitHub Release tarball 增加 173MB。

#### P1-3：运行时数据目录边界模糊（种子数据 vs 运行态）
**证据**：
`platform/backend-node/data/` 下 68 个文件分三类：
| 类型 | 示例 | 是否应入仓 |
|------|------|-----------|
| 种子/初始化数据 | `experts.json` / `projects.json` / `operators.json` / `llm_routing.json` | 是（作为默认模板） |
| 运行时用户数据 | `tasks.json` / `audit_log.json` / `dialogue_sessions.json` / `alliance_traces.jsonl` | 否（用户生成） |
| 数据库/索引 | `ous.db` / `ous.db-wal` / `ous.db-shm` | 否（SQLite 运行时） |

但 .gitignore 只排除了 `*.db*`、`dialogue_sessions.json`、`logs.json`、`tasks.json`、`automation.json`、`workflows.json`——其余 50+ 文件**入仓状态不明确**。

**影响**：CI 测试会覆盖种子数据；用户数据意外入仓导致隐私泄露；部署时不知道哪些目录需要挂载 volume。

#### P1-4：三语构建无统一入口脚本
**证据**：
- Rust：`cargo check` / `cargo build`（但 `default-members` 只编 4 核心，全量需 `cargo check --workspace`）
- Node 后端：`cd platform/backend-node && npm install && node src/api-server.js`
- 前端：`cd frontend-ui && pnpm install && npm run dev`
- Python xiaobai_voice：`cd projects/xiaobai_voice && python -m xiaobai_voice serve`
- 无顶层 `Makefile` / `justfile` / `scripts/*.ps1` 统一封装

**影响**：新成员环境搭建看 4 份 README，容易漏步骤；CI/CD pipeline 要维护 4 套构建逻辑。

---

### 🟡 P2：业务架构可扩展性（现在能用，规模上来会崩）

#### P2-1：JSON Store 到多租户 DB 的迁移路径缺失
**证据**：`platform/backend-node/src/lib/json-store.js`（假设存在，从 projects.js 的 `readJSON/writeJSON` 推断）是本地文件读写模式——单实例好用，但：
- 多进程（pm2 cluster）并发写会丢数据（文件锁不可靠）
- 多租户隔离无法靠文件系统实现（不能每租户一个 JSON）
- 无事务、无索引、无复杂查询（JOIN/聚合）

**风险**：租户 > 50 或 QPS > 100 时读写冲突、数据损坏。

#### P2-2：AI 编排层是否独立不透明
**证据**：
- `platform/backend-node/src/ai-engine.js` / `llm-gateway.js` / `ai-engine-core.js` 三个文件共存——是否有清晰分层：
  ```
  前端请求 → API 路由 → [配额/计量中间件] → AI 编排层 → [模型路由/语义缓存/Guardrails] → Provider API
  ```
  还是直接在 chat.js 中 `fetch(openai)`？

- 从 `llm_routing.json` 和 `llm_config.json` 的存在推测**有模型路由雏形**，但需确认：计量是否在编排层内注入、Prompt 是否模板化、多 Agent 工作流是否复用同一链路。

**风险**：AI 调用散落在各业务路由，计费/审计/降级无法收口。

#### P2-3：插件体系 Manifest 标准化缺失
**证据**：`plugins.json` = 676 字节（3-5 个插件规模），未见标准 Manifest 字段（permissions / resources / endpoints / version / author）。`docs/modules/market-module.md` 可能有定义，但**代码层未强制执行 Manifest 校验**。

**风险**：第三方插件能读任意文件、调任意 AI，无沙箱 + 无权限声明 = 供应链攻击入口。

#### P2-4：projects / my_projects / workspace 命名可读性差
**证据**：三个目录功能正确但名字看不出区别：
| 目录名 | 实际语义 | 建议新名（语义直白） |
|--------|---------|-------------------|
| `projects/` | 平台官方示例项目（melody2score 等） | `examples/` 或 `showcase-projects/` |
| `my_projects/` | 用户本地 Rust crate 开发实验区（.gitignore 忽略） | `local-dev/` 或 `sandbox/` |
| `workspace/` | AI 自动开发引擎运行时产物（.gitignore 忽略子目录） | `runtime-workspace/` 或 `ai-outputs/` |

**影响**：新成员困惑 10-30 分钟，误以为 projects/ 是自己放项目的地方。

---

### 🟢 P3：用户体验 & 可观测性（可用但不够好）

#### P3-1：项目选择器未按租户/组织过滤
`ProjectPicker.vue` 直接展示全量 `projectList`，多租户后会看到其他客户项目——虽然后端会修，但前端也需接入 `tenant-scoped` API。

#### P3-2：无全链路 Trace 可视化
`deploy/docs/trace-8stages-dashboard.json` 存在，说明 Grafana dashboard 模板已有，但：
- OpenTelemetry SDK 是否接入 Node / Rust / Python 三语？
- Trace ID 是否从前端 → API → AI 编排 → Provider 全链路透传？
- 用户侧能否看到"这个 AI 回答为什么慢"的火焰图？

#### P3-3：专家智能匹配算法未见实现
`ExpertCenterView.vue` 有 `smartMode` 开关和 `alliance_intent_priors.json`（1KB）+ `alliance_learned_skills.json`（14KB），但匹配算法的权重/召回策略未见显式定义——是关键词匹配还是图上激活扩散（project_memory 里要求的个性化 PageRank spread d=0.85）？

---

### 🔵 P4：性能 & 成本（规模上来之前不用急）

#### P4-1：graph.json 作为单文件图谱存储的瓶颈
~87MB JSON = 约 5-10 万个节点/边，`JSON.parse` 一次约 100-300ms，全量读写会卡顿。多租户后每租户一个图，需要迁移到 `Neo4j` / `NebulaGraph` 或至少 SQLite + 图索引。

#### P4-2：Rust 42 crates 冷编译时间
首次 `cargo build --workspace` 约 15-30 分钟（取决于 CPU），CI 需要引入 `sccache` + Docker 缓存层。

---

## 四、开发专家联盟系统 — 架构优势的独特性（和竞品对比的护城河）

在做优化方案前，必须明确**哪些是璇玑的"本命能力"，不能为了 SaaS 化而削弱**：

| 维度 | 璇玑现状 | 普通 AI 平台 | 差异化护城河 |
|------|---------|-------------|------------|
| 项目-阶段模型 | 5 阶段 φ 流水线 + 25 模块按阶段归档 | 纯聊天 / 纯任务看板 | 需求→图谱→设计→开发→发布的全链路可视化，每阶段有独立的专家和算子 |
| 全维资源聚合 | 18 类资源挂到项目，每类带前端跳转路由 | 项目只关联文档/任务 | "一个项目页看全所有资产"——专家、算子、Agent、工作流、知识库、图谱节点都在一个视图里联动 |
| Rust 计算核心 | 4 个计算 crate（公式/归一化/意图/DSP）+ 26 领域服务 | 纯 Node/Python | 图算法、公式精度、音频 DSP 性能是 Python 的 10-100x；金融/医疗级精度需求硬约束 |
| 三联盟模式 | docs/18 号文档定义了"算法+架构+商业"三联盟协作 | 单角色 AI 助手 | 复杂项目（比如企业信息化系统）需要三类专家背靠背，单 AI 做不到 |
| 关图治理基线 | `.guantu_baseline.json` + CI 门禁 5 套（enterprise-ci / fusion-gate / graph-gate 等） | 无质量门禁 | 每一次变更跑质量闸门，防止图谱退化、算子出错 |
| 本地语音闭环 | xiaobai_voice（Paraformer ASR + CosyVoice2 TTS）端口 3717 | 依赖云 TTS/ASR | 离线可用、零延迟、数据不出境——金融/政企场景硬需求 |

---

## 五、最优优化方案（四阶段路线图 · 附可执行命令）

### 🎯 核心策略

```
保留强架构基石（5阶段模型 / Rust分层 / 全维聚合 / 快捷键）
  → 修补工程卫生（垃圾清理 / 大文件 / 数据边界）
    → 注入多租户 DNA（tenant_id 贯穿 + RLS + 计量）
      → 生长 SaaS 能力（插件Manifest / 可观测 / 计费）
        → 释放生态（模板市场 / Agent市场 / 开放API）
```

**不做的事**：
- ❌ 不重写前端框架（Vue3 + Element Plus 够用）
- ❌ 不推翻 Rust crate 分层（已经是教科书级）
- ❌ 不把 JSON Store 立即全量换 PostgreSQL（先加 tenant_id，再渐进迁）
- ❌ 不上 K8s（单实例 Docker Compose 先跑通 100 租户）

---

### 阶段一：工程卫生大扫除（0.5-1 周 · P1 全清）

**目标**：让 `git status` 干净、根目录清爽、数据目录边界清晰、构建一命令跑通。

#### 1-1 清理本地垃圾文件 + 强化 .gitignore
执行（Windows PowerShell）：
```powershell
# 1. 删除所有 *.log / NUL-*.d / *.rmeta / 杂项临时文件（仅本地，不碰 git 历史）
$root = "d:\a10\aikjx\gitcode\infotopograph"
$garbage = @(
    "*.log", "NUL-*.d", "*.rmeta",
    "green_exit.txt", "red_exit.txt", "green_log.txt", "red_log.txt",
    "logs_d1d6.txt", "members.txt"
)
foreach ($pattern in $garbage) {
    Get-ChildItem $root -File -Filter $pattern | Remove-Item -Force -Verbose
}

# 2. 验证清理效果（应返回 0）
(Get-ChildItem $root -File | Where-Object { $_.Extension -in '.log','.d','.rmeta' -or $_.Name -like '*_log.txt' }).Count
```

强化 `.gitignore` 追加：
```gitignore
# ===== 追加：璇玑专家联盟运行时（防止 P1-3 边界模糊）=====
# 运行时用户态 JSON（除 seed 数据外全部不入库）
platform/backend-node/data/dialogue_sessions.json
platform/backend-node/data/alliance_traces.jsonl
platform/backend-node/data/alliance_learned_skills.json
platform/backend-node/data/audit_log.json
platform/backend-node/data/llm_usage.json
platform/backend-node/data/logs.json
platform/backend-node/data/tasks.json
platform/backend-node/data/automation.json
platform/backend-node/data/workflows.json
platform/backend-node/data/kb_history.json
platform/backend-node/data/enterprise_10task_history.jsonl
platform/backend-node/data/*.partial
# SQLite 全套（WAL/SHM 也必须排除）
platform/backend-node/data/*.db
platform/backend-node/data/*.db-wal
platform/backend-node/data/*.db-shm
# AI 基准评测产物（每次重建）
platform/backend-node/data/ai_benchmark_*
# 根目录杂项临时文件
/*.txt
!README.md
!members.txt  # 如 members.txt 是贡献者名单则保留，否则删除

# ===== 追加：前端构建日志 =====
frontend-ui/build_log*.txt
```

#### 1-2 排查 & 清除两个 87MB graph.json 的 git 历史
```powershell
# 第一步：确认是否真入仓
cd d:\a10\aikjx\gitcode\infotopograph
git ls-files | Select-String "graph.json|graph.enterprise.json"

# 如有输出（=真入仓），执行：
# 1. 从 git 索引移除（保留本地文件）
git rm --cached graph.json graph.enterprise.json

# 2. 改写历史，永久从所有 commit 删除（大杀器，确认没人并行开发再跑）
# 推荐工具：git-filter-repo（比 BFG 快且安全）
# pip install git-filter-repo
git filter-repo --path graph.json --path graph.enterprise.json --invert-paths --force

# 3. 验证：两个文件应不再出现在 git ls-files
# 4. 最后需要 git push --force（⚠ 通知所有协作者 rebase）
```

**如果没入仓**（git ls-files 无输出），只需确认 .gitignore 规则生效即可。

#### 1-3 重命名三个混淆目录（语义直白化）
选一个周末，所有分支合并后执行：
```powershell
cd d:\a10\aikjx\gitcode\infotopograph

# 1. projects/ → showcase-projects/（平台官方示例）
git mv projects showcase-projects

# 2. my_projects/ → local-dev/（用户本地实验区，已在 .gitignore 中忽略整目录）
#    注意：这个目录已经是 ignore 的，git mv 不会追踪
Rename-Item -Path my_projects -NewName local-dev

# 3. workspace/ → ai-outputs/（AI 自动开发产物）
git mv workspace ai-outputs

# 4. 同步更新所有硬编码路径（grep 后逐个改）：
#    - platform_config.json 中 "xiaobai_voice" 的 cwd: "projects/xiaobai_voice"
#      → 改成 "showcase-projects/xiaobai_voice"
#    - Cargo.toml workspace members 中如有 projects/xxx 引用
#    - frontend-ui 中 import / router 路径
#    - .github/workflows/*.yml 中的路径
```

同步更新 `.gitignore` 中旧路径引用：
```gitignore
# 删除旧
my_projects
workspace/artifacts/
workspace/screenshots/
# 换成新
local-dev/
ai-outputs/artifacts/
ai-outputs/screenshots/
```

#### 1-4 统一三语构建入口（顶层 scripts 目录）
创建 `scripts/` 目录 + 4 个脚本：
```
scripts/
├── setup-dev.ps1      # Windows 一键环境搭建（Node/Python/Rust 版本校验 + npm install）
├── start-all.ps1      # 按 service-manager 依赖顺序启动 api → frontend → xiaobai_voice
├── stop-all.ps1       # 关所有服务（读 .runtime/*.pid 杀进程）
├── check-all.ps1      # cargo check --workspace + npm(lint/test) + pytest
└── README.md          # 脚本使用说明
```

`scripts/start-all.ps1` 核心逻辑（伪代码，要和 `platform_config.json` 的 services 定义对齐，避免双源真相）：
```powershell
# 从 platform_config.json 读 services 定义（单一真相源）
$config = Get-Content "$PSScriptRoot/../platform_config.json" | ConvertFrom-Json
$ordered = $config.services.PSObject.Properties.Value | Sort-Object startup_order_hint

foreach ($svc in $ordered) {
    Write-Host "▶ 启动 $($svc.name)（端口 $($svc.port)）"
    # Start-Process -WorkingDirectory $svc.cwd -ArgumentList $svc.args ...
    # 存 PID 到 .runtime/<svc_name>.pid
}
```

---

### 阶段二：注入多租户 DNA（2-4 周 · P0 全清 + P2-1/P2-2）

**目标**：tenant_id 贯穿数据层→API 层→前端；计量系统上线；默认凭据改环境变量。

#### 2-1 租户数据模型（四层结构 + JSON Store 临时方案）
在不立即换 PostgreSQL 的前提下，先给 JSON Store 每一条记录加 `tenant_id` / `org_id`：

**领域模型定义**（写进 `frontend-ui/src/types.js` 作为单源）：
```js
// 璇玑多租户四层身份模型
// Tenant(租户) → Org(组织/部门) → User(用户) → Project(项目)
//    1           : N           : N        : N
export const TENANT_MODEL = {
  levels: [
    { key: 'tenant', label: '租户',   idField: 'tenant_id', foreign: null },
    { key: 'org',    label: '组织',   idField: 'org_id',    foreign: 'tenant_id' },
    { key: 'user',   label: '用户',   idField: 'user_id',   foreign: 'org_id' },
    { key: 'project',label: '项目',   idField: 'project_id',foreign: 'user_id' }
  ],
  // 每级隔离策略（SaaS 起步用共享库 + 逻辑隔离，企业版可切独立 Schema）
  isolation: {
    data:   'logical-tenant-id',   // 所有表加 tenant_id，查询时强制过滤
    api:    'subdomain-or-header', // api.tenantA.mox.cn 或 X-Tenant-Id header
    quota:  'per-tenant',          // 配额按租户计，超配额返回 429
    ui:     'workspace-switcher',  // 顶栏 ProjectPicker 升级为「租户→项目」二级选择
  }
}
```

**迁移步骤**（JSON Store 版，为后续换 PostgreSQL 留一致接口）：
1. `platform/backend-node/src/lib/json-store.js` 新增 `withTenant(tenantId)` 方法——所有 readJSON/writeJSON 先按 tenantId 过滤
2. 写一次性迁移脚本：`platform/backend-node/src/migrate-tenant.js`，给所有旧数据补 `tenant_id = 'default'` + `org_id = 'default'`
3. `projects.js` / `chat.js` / `tasks.js` 等 23 路由域**强制校验 `req.tenantId`**（无则 401），禁止无租户上下文的 API 调用
4. 登录后签发 JWT，payload 含 `{ tenant_id, org_id, user_id, role }`

#### 2-2 `projectContext.js` 升级：项目选择器 → 租户+项目二级选择器
`ProjectPicker.vue` 改造：
```
原来：[项目下拉] ─── 选项目 → 注入 HTTP X-Project-Id
新：  [租户切换 Chip] [项目下拉]
         │                │
         └── 注入 HTTP X-Tenant-Id (强制)
                          └── 注入 HTTP X-Project-Id
```

关键改动：
- `useProject()` 新增 `currentTenant` / `tenantList` / `setCurrentTenant()`
- localStorage 存 `mox.currentTenant.v1` 和 `mox.currentProject.v1`
- HTTP 拦截器注入两个 header（`X-Tenant-Id` + `X-Project-Id`），后端中间件从 header 读 + 从 JWT 二次校验一致性（防越权）

#### 2-3 修复 P0-3：默认凭据改环境变量
`platform_config.json` 改成：
```json
{
  "admin": {
    "username": "${MOX_ADMIN_USER}",
    "password": "${MOX_ADMIN_PASSWORD_HASH}"
  }
}
```
启动时 `service-manager.js` 做 env 替换，未设置时**禁止启动**并报错：
```
> 环境变量 MOX_ADMIN_PASSWORD_HASH 未设置。
> 生成命令：node -e "console.log(require('crypto').scryptSync('你的密码','mox-salt',64).toString('hex'))"
```

#### 2-4 AI 计量系统 MVP（与 P0-2 对齐）
在 `llm-gateway.js` 加 4 个中间件，形成标准链路：
```
[HTTP 请求进入]
    → ① AuthMiddleware：JWT → 提取 tenant_id/org_id/user_id/project_id
    → ② QuotaMiddleware：查 redis（或 JSON Store 临时版）→ 租户本月 Token 超限？→ 429
    → ③ Orchestrator：Prompt 模板渲染 → 模型路由 → 语义缓存命中？
    → ④ MeteringMiddleware：记录 {tenant, project, user, model, prompt_tokens, completion_tokens, latency_ms, status}
    → [返回响应]
```

计量记录存 `platform/backend-node/data/llm_usage.jsonl`（append-only JSONL，后续换 ClickHouse），提供两个 API：
- `GET /metering/usage?range=this_month` → 用量仪表盘
- `GET /metering/cost?breakdown=tenant` → 成本归因（按 `0.03$/1K prompt + 0.06$/1K completion` 估算）

---

### 阶段三：SaaS 能力补齐（4-8 周 · 剩余 P2/P3）

#### 3-1 JSON Store → PostgreSQL 渐进迁移（P2-1）
两步走，不一次性全迁：
1. **第一步（第 4 周）**：计量+审计+用户系统迁 PostgreSQL（RLS 启用）
   - 表：`tenants` / `orgs` / `users` / `roles` / `llm_usage` / `audit_log` / `login_sessions`
   - 启用 PostgreSQL Row Level Security：每表加 `tenant_id`，`ALTER TABLE ... ENABLE ROW LEVEL SECURITY`，建 policy `USING (tenant_id = current_setting('app.current_tenant'))`
   - 业务表（projects/experts/tasks 等）先留 JSON

2. **第二步（第 6-8 周）**：核心业务表按使用频率逐个迁
   - 优先级：`tasks`(高频写) → `projects` → `experts` → `kb_documents` → `flows`/`workflows` → `graph_nodes/edges`
   - 每迁一张表，保留双写 3 天回滚窗口

#### 3-2 插件 Manifest 标准化 + 权限校验（P2-3）
每个插件目录必须有 `plugin.yaml`：
```yaml
manifest_version: 1
id: knowledge-graph-builder
name: 知识图谱构建器
version: 1.2.0
author: infotopograph
description: 从 PDF/Word 自动构建实体关系图
permissions:                     # 插件能做什么，安装时用户要勾选
  - graph:write                  # 写图谱
  - storage:read:kb_documents    # 只读知识库文档
  - ai:invoke:chat               # 调 AI 聊天（限制非嵌入模型）
resources:                       # 资源配额上限
  memory_mb: 512
  cpu_cores: 1
  ai_tokens_per_day: 100000
endpoints:                       # 对外暴露的 API（平台自动加租户鉴权）
  - path: /build-from-doc
    method: POST
    auth: required
    rate_limit: 10/min
lifecycle:
  install: "npm install"
  start:   "node index.js"
  stop:    "SIGTERM"
```

平台侧改动：
- `plugins.js` 路由安装时解析 Manifest + 弹用户确认对话框（列出权限）
- 插件运行时跑在单独的 `worker_threads` 子进程，RPC 调用过权限检查中间件
- 审计日志记录每次插件跨边界调用（谁调了哪个权限 API）

#### 3-3 OpenTelemetry 三语接入 + 统一 Dashboard（P3-2）
```
前端(Vue)        → @opentelemetry/web → Zipkin 协议上报
├─ UserInteraction (点击/路由)
└─ Fetch (自动注入 traceparent header)

Node 后端        → @opentelemetry/node + auto-instrumentations(express,http,pg)
├─ AI 调用 span (model, tokens, latency)
└─ DB 查询 span (table, rows, duration)

Rust crates      → tracing + tracing-opentelemetry
├─ mox-expert/pipeline.rs (每个阶段一个 span)
└─ 算子执行 (operator_id, input_shape, duration)

Python xiaobai   → opentelemetry-python
└─ TTS/ASR 推理 span (model_name, audio_ms, latency)

→ 统一汇到 Jaeger / Grafana Tempo
→ 配 deploy/docs/trace-8stages-dashboard.json 到 Grafana
```

用户侧在「系统监控」页看自己项目的 AI 调用 Trace 火焰图（脱敏后）。

#### 3-4 专家匹配算法落地：激活扩散（project_memory 硬约束）
`alliance_intent_priors.json` + `alliance_learned_skills.json` + `expert_capability_graph.json` 三张图，在匹配时跑：
> 个性化 PageRank 特例（method=spread, d=0.85, 30 轮收敛）——来自 project_memory 硬约束

实现伪代码（放在 Rust `mox-intent-core`，调 JS 用 FFI 绑定）：
```rust
// platform/crates/mox-intent-core/src/lib.rs
pub fn spread_activation(
    expert_graph: &Graph,   // 专家-能力-历史项目图
    seed_nodes: &[NodeId],  // 用户输入的关键词/需求描述命中的节点
    damping: f64 = 0.85,    // d = 0.85
    max_iter: usize = 30,   // 30 轮收敛
) -> Vec<(ExpertId, f64)> { /* 返回按激活分排序的候选专家 */ }
```

前端 `smartMode` 开关打开时调 `/experts/match?intent=...`，结果注入专家列表顶部。

---

### 阶段四：生态建设与规模化（8 周+ · P4 + 商业化功能）

#### 4-1 模板市场 + 行业解决方案
把 `docs/enterprise/` 中已纸面定义的三联盟模式、5 阶段流程做成可下载的模板包：
- 模板格式：`.moxt` zip（project.json + phase_defs + 默认专家池 + 工作流骨架 + 种子算子）
- 市场发布流程：上传 → Manifest 校验 → 沙箱安装测试 → 人工审核 → 上架
- 收费模式：免费模板 / 付费模板 / 企业定制（抽成 30%）

#### 4-2 开放 API + Webhook + SDK 完善
`platform/sdk/` 三语 SDK 目前是 `.gitkeep` 空壳，补齐：
- Node SDK：`@infotopograph/sdk`（覆盖 projects/experts/tasks/graph/ai 域）
- Python SDK：`infotopograph`（pip）
- Rust SDK：已注册 crates，补文档和示例

Webhook：
- `POST /webhooks` 注册：`on: project.phase.changed | expert.matched | ai.response.generated | task.completed`
- 签名：`X-Mox-Signature: HMAC-SHA256(secret, payload)`（对齐 `mox-standards` crate 的 SigV4 实现）

#### 4-3 部署架构升级：Docker Compose → K8s（有 100+ 租户再做）
```yaml
# deploy/docker-compose.yml（MVP，100 租户内够用）
services:
  postgres:           # 业务 + RLS
    image: postgres:16
    volumes: [mox-pg:/var/lib/postgresql/data]
  redis:              # 配额 + 语义缓存
    image: redis:7
  gateway-rust:       # platform/gateway/runtime
    build: { context: ., dockerfile: platform/gateway/runtime/Dockerfile }
    ports: ["80:3000"]
    depends_on: [postgres, redis]
  api-node:           # platform/backend-node
    build: { context: ., dockerfile: platform/backend-node/Dockerfile }
    deploy: { replicas: 2 }
  frontend:           # nginx 托管静态
    image: nginx:alpine
  otel-collector:     # Trace 聚合
    image: otel/opentelemetry-collector
  jaeger:             # Trace UI
    image: jaegertracing/all-in-one
  grafana:            # Metrics + Dashboard
    image: grafana/grafana
    volumes: [./deploy/docs/trace-8stages-dashboard.json:/etc/grafana/dashboards]
```

1000+ 租户再迁 K8s + Istio，用 `deploy/helm/mox/` 已有 Chart 扩展。

#### 4-4 计费系统（对接 Stripe / 国内支付）
基于阶段二的计量系统 MVP，增加：
- 4 档套餐：Free / Pro / Team / Enterprise
- 计费维度：Token 用量 × 模型单价 + 活跃席位 × 席位单价 + 存储 GB × 存储单价
- 账单页：PDF 发票下载、消费明细 CSV、成本趋势图

---

## 六、璇玑·专家联盟 SaaS 化终态架构图（七层模型 · 和原分析对齐 + 项目特色）

```
┌──────────────────────────────────────────────────────────────────────────┐
│ L7 · 应用层（以 5 阶段 φ 模型组织 25 模块）                               │
│  ┌───────────────────────────────────────────────────────────────────┐   │
│  │ S1需求 · 编译/AI助手X/专家联盟/知识库/大模型配置                    │   │
│  │ S2图谱 · 璇玑图谱/全维融合/算子引擎/企业图谱/V2编排引擎              │   │
│  │ S3设计 · 工作流/AI自动化/AI插件/MCP兼容/算子商城                   │   │
│  │ S4开发 · 算法实验室/无穷维优化/BotCenter/浏览器自动化               │   │
│  │ S5发布 · 系统监控/API文档/门户大厅/企业管理                         │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│  ← 顶栏：租户切换 → 项目选择 → 全局命令(Ctrl+K) → 快捷新建(6项) →      │
├──────────────────────────────────────────────────────────────────────────┤
│ L6 · 插件 & 模板运行时                                                   │
│  Manifest 校验 → Worker 沙箱隔离 → 权限 RPC → 生命周期 → 版本回滚       │
│  + 模板市场：.moxt 包 → 种子项目 → 专家池 → 工作流骨架                  │
├──────────────────────────────────────────────────────────────────────────┤
│ L5 · AI 编排层（璇玑大脑，统一入口不可绕过）                              │
│  Auth→Quota→Prompt模板→模型路由→语义缓存→Guardrails→Agent编排→RAG      │
│  ↓ 每个请求落盘：llm_usage (tenant, project, user, tokens, cost, trace) │
│  ← 实现 4 个硬约束：CEM 优化 / 加权评分(0.55Q+0.20S+0.10E+0.15Sta)     │
│    / POST /ai/engine/process 统一意图路由                                │
│    / POST /ai/engine/analyze  显式能力执行                              │
│    / GET  /ai/engine/capabilities 能力矩阵自描述                        │
│    / GET  /ai/engine/metrics 成功率/降级率/延迟指标                    │
├──────────────────────────────────────────────────────────────────────────┤
│ L4 · 业务服务层（Rust 26 services + Node 23 routes 共治）                │
│  专家联盟(mox-expert) · 意图(mox-intent-core) · 公式(mox-formulas-core) │
│  归一化(mox-norm-core) · DSP(xiaobai-dsp) · 图谱(kg-hub) · 流程(flow-ai)│
│  全维资源聚合 → 项目 CRUD → 5 阶段追踪 → 任务协作 → 权限 RBAC          │
├──────────────────────────────────────────────────────────────────────────┤
│ L3 · 平台能力层（SaaS 底座，和原分析对齐）                                │
│  租户/组织管理 ← PostgreSQL RLS → 用量计量/配额/计费 ← 审计日志        │
│  SSO/SAML(OIDC) ← SigV4 签名(mox-standards) → 通知(Webhook+飞书/邮件) │
├──────────────────────────────────────────────────────────────────────────┤
│ L2 · 数据层（多隔离级别可选）                                             │
│  PostgreSQL(RLS) │ pgvector │ NebulaGraph/Neo4j │ S3 对象存储           │
│  Redis(配额/缓存) │ Pulsar/Kafka(事件) │ ClickHouse(用量分析)           │
│  ← 启动：共享库+RLS → 500租户：共享库独立Schema → 大客户：独立DB       │
├──────────────────────────────────────────────────────────────────────────┤
│ L1 · 基础设施层（灰度可控）                                               │
│  Docker Compose → K8s + Istio → ArgoCD(GitOps) → OpenTelemetry 三件套  │
│  KMS(密钥) ← 新实例健康100%再切流量 → 旧实例延迟关停（用户硬约束）      │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## 七、验收清单（每阶段完成的可验证标准）

### 阶段一验收（工程卫生）
- [ ] `git status` 无未跟踪文件（除 local-dev/ / ai-outputs/ 两个 ignore 目录）
- [ ] `git ls-files | Select-String graph.json` → 空输出
- [ ] `ls root` 无 `*.log` / `*.d` / `*.rmeta`
- [ ] 新人 `.\scripts\setup-dev.ps1 ; .\scripts\check-all.ps1` → 全绿
- [ ] 三个目录重命名完成，所有路径 grep 无残留旧名

### 阶段二验收（多租户 DNA）
- [ ] 创建两个租户（A/B），A 管理员调 `/projects` 看不到 B 的项目（Postman 可验证）
- [ ] 前端 `ProjectPicker.vue` 显示「当前租户：XX」Chip，切换后列表立即刷新
- [ ] AI 对话 10 次后，`llm_usage.jsonl` 有 10 条记录含 tenant_id + token 用量
- [ ] 未设 `MOX_ADMIN_PASSWORD_HASH` 时启动服务报错退出
- [ ] 所有 23 路由域单测都含"无 tenantId → 401"用例

### 阶段三验收（SaaS 能力）
- [ ] PostgreSQL 中 6 张核心表启用 RLS，用两个租户角色登录查同一张表看到行数不同
- [ ] 安装插件时弹窗列出 Manifest 声明的权限，未勾选的权限 API 返回 403
- [ ] Grafana 打开 trace-8stages-dashboard.json → 有数据，点任一 Trace 能看到 Node→Rust→Python 全链路
- [ ] 智能匹配开关打开时，输入「构建金融风控知识图谱」→ 召回「图谱专家+算法专家+安全专家」前三名（激活扩散算法单测收敛 30 轮 σ̄<0.06）

### 阶段四验收（规模化）
- [ ] `docker compose up` → 浏览器开 `http://localhost` → 能走完注册→创建项目→S1→S5 全流程
- [ ] Node SDK `npm install @infotopograph/sdk` → 5 行代码完成创建项目+派任务
- [ ] 模拟 100 并发 × 1000 请求 → API 95 线延迟 < 500ms，错误率 < 0.1%
- [ ] 新实例启动健康检查 100% 通过后，流量从旧实例 10%→50%→100% 渐进切换，旧实例等待 300s 无请求后关停（满足用户「优雅流量切换」硬约束）

---

## 八、和原分析方案的差异对比（为什么本方案更好）

| 维度 | 原分析（草稿级） | 本方案（基于源码取证） |
|------|----------------|-------------------|
| 对现有架构的判断 | "Monorepo 结构缺失" → 全盘否定 | "Rust 分层是教科书级" → 保留，只补 Node/前端 workspace + 统一脚本 |
| 重叠目录处理 | 抽象说"合并重叠目录" → 风险不可控 | 三个目录实际语义不同，只**重命名表达语义**，保留原有内容不折腾 |
| 大文件处理 | 笼统说"用 Git LFS 或对象存储" | 先 `git ls-files` 验证是否真入仓 → 分情况（本地未跟踪 vs git rm --cached vs filter-repo），附可执行命令 |
| 数据库迁移 | "一上来 PostgreSQL + RLS"，23 路由域全改风险极高 | **两步渐进迁**：先迁计量/审计/用户（6张表），再按频率 6-8 周逐个迁业务表，双写回滚窗口 |
| 多租户模型 | 只给了"三选一隔离策略"表格 | 给出四层身份模型（Tenant→Org→User→Project）+ JSON Store 临时过渡方案 + projectContext.js 具体改造点 + JWT 校验链 |
| AI 编排层 | 只给了通用流程图 | 直接对齐 project_memory 硬约束：4 个统一入口路由 + CEM 优化算法 + 多目标加权评分公式 + σ̄<0.06 停止阈值 |
| 专家匹配 | 没提 | 强制用 project_memory 要求的**激活扩散（spread d=0.85, 30轮收敛）**，放 Rust mox-intent-core 保证性能 |
| 插件体系 | 通用 Manifest 模板 | 和现有 676B 的 plugins.json 对齐 + 和 mox-standards SigV4 签名复用 + Worker 沙箱的具体实现方向 |
| 部署切换 | 没提 | 对齐用户"新服务启动成功后优雅切流量再关旧服务"硬约束，写进阶段四验收标准 |
| 验收标准 | 无 | 四阶段 26 条可执行可验证检查项（Postman/脚本/命令行可直接跑） |

---

## 九、下一步行动建议（本周可开工的 3 件事）

| # | 任务 | 负责人建议 | 耗时 | 产出物 |
|---|------|-----------|------|--------|
| 1 | **阶段一 1-1 + 1-2**：垃圾清理 + graph.json git 历史排查 | DevOps | 半天 | 干净的 git status + 仓库体积减小报告 |
| 2 | **阶段二 2-3**：默认 admin 密码改环境变量 + 启动时校验 | 后端 | 2 小时 | 无硬编码凭据 + 文档说明（配合 `.env.example`） |
| 3 | **阶段二 2-1 模型定义**：写 `TENANT_MODEL` 进 types.js + 评审会 | 架构师 + 产品 | 1 天 | 四层身份模型终稿 + 各路由域改造任务拆分清单 |

> **风险提示**：`git filter-repo` 改写历史后所有协作者必须删仓库重 clone，建议周五晚执行，周一前通知完。

---

## 十、文档变更记录

| 版本 | 日期 | 变更人 | 变更内容 |
|------|------|-------|---------|
| V1.0 | 2026-08-26 | Mox 架构组 | 首版：源码取证（47 垃圾文件/173MB 大图/42 Rust crates/68 数据文件）+ 四阶段路线图 + 26 条验收项 |



全维 Rust 后端，这是正确的技术方向。让我先摸清 infotopograph 现有的 Rust 代码结构，再给出精准的全维优化方案。infotopograph 已经是一个大型 Rust Cargo Workspace（50+ crate），但缺少 gRPC 和多租户。让我深入 platform 目录摸清模块边界。让我快速查看几个核心模块的内部结构和依赖。现在我对 infotopograph 的 Rust 架构有了完整画像。以下是全维 Rust 后端的优化方案。

---

# infotopograph 全维 Rust 后端优化方案

## 一、现状诊断

### 1.1 已有基础（非常扎实）

infotopograph 已经是一个 **50+ crate 的 Cargo Workspace**，版本 3.0.0-ai-powered，技术栈先进且完整：

| 层级 | 已有能力 | 技术选型 |
|------|----------|----------|
| 异步运行时 | tokio full | ✅ 已有 |
| Web 框架 | axum 0.7 + tower | ✅ 已有 |
| 数据库 | sqlx 0.8 + sea-query + rusqlite | ✅ 已有 |
| WASM 插件 | wasmer 4.2 + operator-wasm + mox-etl-wasm | ✅ 已有 |
| 图计算 | petgraph + graph-algorithms + kg-hub + mox-graph-* (6模块) | ✅ 已有 |
| AI 能力 | ai-agent + flow-ai + mox-ai-core + mox-expert | ✅ 已有 |
| 云存储 | mox-cloud-drive-* (4模块，含S3) + mox-standards(国密SM2/3/4) | ✅ 已有 |
| 流程引擎 | primiflow-core/fusion + hermes-flow-bridge | ✅ 已有 |
| 网关 | gateway/runtime (ai_router + rbac_middleware + openapi + sidecar) | ✅ 已有 |
| 可观测性 | prometheus + tracing + mox-server/o11y.rs | ✅ 部分 |
| 数学/并行 | nalgebra + ndarray + rayon + hashbrown + aho-corasick | ✅ 已有 |
| FFI 绑定 | napi (Node.js) + pyo3 (Python) | ✅ 已有 |
| CLI | clap 4 | ✅ 已有 |

### 1.2 核心缺失（五大短板）

| 短板 | 现状 | 影响 |
|------|------|------|
| **无 gRPC/RPC** | 整个 workspace 无 tonic/prost，服务间只能 REST 或单体函数调用 | 无法微服务化，AI 流式生成不优雅 |
| **无多租户** | 无 tenant 模块、拦截器、数据隔离 | 无法做 SaaS |
| **模块命名混乱** | mox-* 前缀与非 mox-*（ai-agent/flow-ai/kg-hub/operator-core）混用 | 领域边界不清，维护困难 |
| **单体倾向** | mox-server 是 single-binary，36个服务编译进一个二进制 | 无法独立扩缩容，启动慢 |
| **可观测性不完整** | 有 prometheus 但无 OpenTelemetry tracing，无日志聚合 | 分布式排查困难 |

### 1.3 服务 crate 清单（36个，需重组）

```
已有服务（命名混乱）：
├── mox-* 前缀（25个）：mox-ai-core, mox-cloud-drive-{filer,master,s3,volume},
│   mox-common-meta, mox-compliance, mox-data-plane, mox-domain-abstractions,
│   mox-etl-wasm, mox-expert, mox-fusion, mox-graph-{meta,service,spark,storage,streams},
│   mox-server, mox-standards, mox-system, mox-t21-harness
├── 无前缀（11个）：ai-agent, business-catalog, flow-ai, graph-algorithms,
│   hermes-flow-bridge, kg-hub, operator-core, operator-wasm, optimizer,
│   primiflow-core, primiflow-fusion, template-market
```

---

## 二、目标架构：全 Rust 微服务 SaaS AI 平台

### 2.1 设计原则

1. **全 Rust 后端**：所有服务、网关、SDK、工具链均用 Rust 编写，Python 仅作为 AI 模型推理的 sidecar（通过 gRPC 调用）
2. **gRPC 优先**：服务间一律 tonic gRPC，对外通过网关暴露 REST + gRPC-Web + WebSocket
3. **多租户原生**：tenant_id 从网关到数据库贯穿全链路
4. **领域驱动分层**：清晰的领域边界，统一命名规范
5. **可独立部署**：每个服务可独立编译、独立部署、独立扩缩容
6. **内核自研**：AI 编排、图谱引擎、插件运行时、多租户内核全部自研

### 2.2 目录结构重组（标准 monorepo）

```
infotopograph/
├── Cargo.toml                          # Workspace 根
├── .gitignore                          # 完善（排除 target/ *.log 大文件）
│
├── crates/                             # ★ 核心库层（原 platform/crates/）
│   ├── mox-formulas-core/             # 公式计算核心
│   ├── mox-norm-core/                 # 归一化核心
│   ├── mox-intent-core/               # 意图识别核心
│   ├── xiaobai-dsp/                   # 语音 DSP
│   └── bindings/                       # FFI 绑定
│       ├── mox-formulas-native/       # napi
│       ├── mox-norm-intent-native/    # napi
│       └── xiaobai-dsp-py/            # pyo3
│
├── libs/                               # ★ 共享库层（原 platform/shared/ + 新增）
│   ├── mox-common/                     # 通用工具（原 mox-common-meta 扩展）
│   ├── mox-domain/                     # 领域模型（原 mox-domain-abstractions）
│   ├── mox-standards/                 # 标准/国密（已有）
│   ├── mox-rpc/                        # ★ RPC 核心（tonic 拦截器、公共 proto）
│   ├── mox-tenant/                     # ★ 多租户核心（租户上下文、拦截器、RLS）
│   ├── mox-auth/                       # ★ 认证授权（JWT、SSO、RBAC，从 gateway 抽取）
│   ├── mox-config/                     # ★ 配置中心客户端
│   ├── mox-discovery/                  # ★ 服务注册发现客户端
│   ├── mox-o11y/                       # ★ 可观测性（otel + tracing + prometheus）
│   ├── mox-cache/                      # ★ 缓存抽象（redis + 内存）
│   ├── mox-mq/                         # ★ 消息队列抽象（nats/kafka）
│   └── mox-db/                         # 数据库访问（sqlx 封装，从各服务抽取）
│
├── services/                           # ★ 业务服务层（原 platform/services/，重组命名）
│   ├── gateway/                        # API 网关（原 platform/gateway/runtime/）
│   ├── system/                         # 系统管理（原 mox-system）
│   ├── tenant/                         # ★ 租户管理（新增）
│   ├── auth/                           # 认证服务（从 gateway/system 抽取）
│   ├── ai-orchestrator/                # AI 编排（原 flow-ai + mox-ai-core 合并）
│   ├── agent/                          # AI Agent（原 ai-agent）
│   ├── expert/                         # 专家系统（原 mox-expert）
│   ├── graph-core/                     # 图谱核心（原 kg-hub + mox-graph-service 合并）
│   ├── graph-storage/                  # 图谱存储（原 mox-graph-storage）
│   ├── graph-algorithms/               # 图算法（原 graph-algorithms + mox-graph-spark）
│   ├── graph-streams/                  # 图流处理（原 mox-graph-streams）
│   ├── graph-meta/                     # 图谱元数据（原 mox-graph-meta）
│   ├── operator-core/                  # 算子核心（已有）
│   ├── operator-wasm/                  # WASM 算子（已有）
│   ├── etl/                            # ETL（原 mox-etl-wasm）
│   ├── data-plane/                     # 数据平面（原 mox-data-plane）
│   ├── optimizer/                      # 优化器（已有）
│   ├── flow-core/                      # 流程核心（原 primiflow-core）
│   ├── flow-fusion/                    # 流程融合（原 primiflow-fusion + hermes-flow-bridge）
│   ├── cloud-drive/                    # 云盘（原 mox-cloud-drive-* 4模块合并）
│   ├── compliance/                     # 合规审计（原 mox-compliance）
│   ├── fusion/                         # 融合引擎（原 mox-fusion）
│   ├── business-catalog/               # 业务目录（已有）
│   ├── template-market/                # 模板市场（已有）
│   ├── metering/                       # ★ 计量计费（新增）
│   └── notification/                   # ★ 通知服务（新增）
│
├── sdk/                                # SDK 层
│   └── rust/                           # Rust SDK（已有 mox-sdk-cloud + mox-sdk-graph）
│
├── proto/                              # ★ Protocol Buffers 定义（新增）
│   ├── common/                         # 公共消息（RequestMeta, PageRequest, Error）
│   ├── tenant/                         # 租户服务
│   ├── auth/                           # 认证服务
│   ├── system/                         # 系统管理
│   ├── ai/                             # AI 编排（含流式 GenerateStream）
│   ├── agent/                          # Agent 服务
│   ├── graph/                          # 图谱服务
│   ├── operator/                       # 算子服务
│   ├── storage/                        # 存储服务
│   └── metering/                       # 计量服务
│
├── deploy/                             # 部署配置
│   ├── k8s/                            # Kubernetes manifests
│   ├── docker/                         # Dockerfile（每服务一个）
│   └── helm/                           # Helm charts
│
├── scripts/                            # 构建/运维脚本
│
├── docs/                               # 文档
│
└── frontend-ui/                        # 前端（保持不变）
```

### 2.3 模块命名规范（统一）

**所有 crate 统一使用 `mox-` 前缀**，按层级分类：

| 层级 | 命名模式 | 示例 |
|------|----------|------|
| 核心库 | `mox-{domain}-core` | `mox-formulas-core`, `mox-norm-core` |
| 共享库 | `mox-{capability}` | `mox-rpc`, `mox-tenant`, `mox-auth`, `mox-o11y` |
| 业务服务 | `mox-{domain}-svc` | `mox-ai-svc`, `mox-graph-svc`, `mox-tenant-svc` |
| SDK | `mox-sdk-{target}` | `mox-sdk-rust`, `mox-sdk-python` |
| FFI | `mox-{core}-native` | `mox-formulas-native` |

**重命名映射表**（36个服务 → 统一命名）：

| 原名 | 新名 | 说明 |
|------|------|------|
| mox-server | mox-gateway-svc | 主服务器→网关服务（职责收敛） |
| gateway/runtime | mox-gateway-svc | 合并入网关 |
| mox-system | mox-system-svc | 加 -svc 后缀 |
| ai-agent | mox-agent-svc | 加 mox- 前缀 + -svc |
| flow-ai + mox-ai-core | mox-ai-svc | 合并为 AI 编排服务 |
| mox-expert | mox-expert-svc | 加 -svc |
| kg-hub + mox-graph-service | mox-graph-svc | 合并为图谱核心服务 |
| graph-algorithms + mox-graph-spark | mox-graph-algo-svc | 合并为图算法服务 |
| mox-graph-storage | mox-graph-storage-svc | 加 -svc |
| mox-graph-streams | mox-graph-streams-svc | 加 -svc |
| mox-graph-meta | mox-graph-meta-svc | 加 -svc |
| operator-core | mox-operator-svc | 加 mox- 前缀 + -svc |
| operator-wasm | mox-operator-wasm | 保持（是库不是服务） |
| mox-etl-wasm | mox-etl-svc | 改名，wasm 是实现细节 |
| mox-data-plane | mox-dataplane-svc | 加 -svc |
| optimizer | mox-optimizer-svc | 加 mox- 前缀 + -svc |
| primiflow-core | mox-flow-svc | 改名 + 加 mox- |
| primiflow-fusion + hermes-flow-bridge | mox-flow-fusion-svc | 合并 |
| mox-cloud-drive-{master,volume,s3,filer} | mox-storage-svc | 4模块合并为存储服务 |
| mox-compliance | mox-compliance-svc | 加 -svc |
| mox-fusion | mox-fusion-svc | 加 -svc |
| business-catalog | mox-catalog-svc | 加 mox- 前缀 + -svc |
| template-market | mox-market-svc | 加 mox- 前缀 + -svc |
| mox-common-meta | mox-common | 改名（共享库） |
| mox-domain-abstractions | mox-domain | 改名（共享库） |
| mox-standards | mox-standards | 保持（共享库） |
| mox-t21-harness | mox-test-harness | 改名（测试工具） |
| **新增** | mox-tenant-svc | 租户管理服务 |
| **新增** | mox-auth-svc | 认证服务 |
| **新增** | mox-metering-svc | 计量计费服务 |
| **新增** | mox-notification-svc | 通知服务 |

---

## 三、gRPC 全维接入方案（核心）

### 3.1 技术选型

| 组件 | crate | 版本 | 用途 |
|------|-------|------|------|
| gRPC 运行时 | `tonic` | 0.12 | async gRPC 服务端/客户端 |
| gRPC 构建 | `tonic-build` | 0.12 | 编译 .proto 生成 Rust 代码 |
| Protocol Buffers | `prost` | 0.13 | protobuf 序列化 |
| protobuf 构建 | `prost-build` | 0.13 | proto 编译 |
| 通用类型 | `prost-types` | 0.13 | Timestamp, Duration 等 |
| gRPC-Web | `tonic-web` | 0.12 | 浏览器 gRPC 支持 |
| 反射 | `tonic-reflection` | 0.12 | gRPC Server Reflection（调试用） |
| 健康检查 | `tonic-health` | 0.12 | 标准健康检查协议 |

### 3.2 Workspace 依赖新增

在 `Cargo.toml` 的 `[workspace.dependencies]` 中新增：

```toml
# ===== gRPC / RPC =====
tonic = { version = "0.12", features = ["tls", "prost"] }
tonic-build = "0.12"
tonic-web = "0.12"
tonic-reflection = "0.12"
tonic-health = "0.12"
prost = "0.13"
prost-build = "0.13"
prost-types = "0.13"

# ===== 多租户 =====
# (mox-tenant 作为内部 crate)

# ===== 认证 =====
jsonwebtoken = "9"
# biscuit = "0.6"  # 可选：更安全的 token 格式

# ===== 缓存 =====
redis = { version = "0.27", features = ["tokio-comp", "connection-manager"] }
# 或 fred = "9"（更高性能的 redis 客户端）

# ===== 消息队列 =====
nats = "0.35"  # 轻量、高性能、内置 JetStream 持久化
# 或 rdkafka = "0.36"（Kafka）

# ===== 服务注册发现 =====
etcd-client = { version = "0.12", features = ["tls"] }
# 或直接用 K8s Service（k8s 环境下）

# ===== 可观测性 =====
opentelemetry = { version = "0.27", features = ["trace", "metrics"] }
opentelemetry-otlp = "0.27"
opentelemetry-semantic-conventions = "0.27"
tracing-opentelemetry = "0.28"
tracing-subscriber = { workspace = true }  # 已有，补充 fmt + json feature

# ===== 配置 =====
config = "0.14"  # 分层配置（文件+环境变量+远程）
```

### 3.3 公共 Proto 定义

`proto/common/common.proto`：

```protobuf
syntax = "proto3";
package mox.common;

// 请求元数据：所有 gRPC 请求必须携带
message RequestMeta {
  string tenant_id = 1;    // 租户 ID（必填）
  string user_id = 2;      // 用户 ID
  string trace_id = 3;     // 链路追踪 ID
  string request_id = 4;   // 请求 ID（幂等用）
  map<string, string> headers = 5;
}

// 分页请求
message PageRequest {
  int32 page_num = 1;
  int32 page_size = 2;
  string order_by = 3;
}

// 分页响应
message PageResponse {
  int64 total = 1;
  int32 page_num = 2;
  int32 page_size = 3;
}

// 统一错误码
enum ErrorCode {
  OK = 0;
  UNAUTHENTICATED = 1;
  PERMISSION_DENIED = 2;
  TENANT_NOT_FOUND = 3;
  QUOTA_EXCEEDED = 4;
  INVALID_ARGUMENT = 5;
  NOT_FOUND = 6;
  INTERNAL = 7;
  AI_TIMEOUT = 100;
  AI_RATE_LIMITED = 101;
}
```

### 3.4 AI 流式 Proto（核心场景）

`proto/ai/ai.proto`：

```protobuf
syntax = "proto3";
package mox.ai;

import "common/common.proto";

service AIService {
  // 流式生成（AI 核心场景，token-by-token）
  rpc GenerateStream(GenerateRequest) returns (stream GenerateResponse);

  // 一元生成
  rpc Generate(GenerateRequest) returns (GenerateResponse);

  // 模型路由（选择最优模型）
  rpc RouteModel(RouteRequest) returns (RouteResponse);

  // 嵌入向量
  rpc Embed(EmbedRequest) returns (EmbedResponse);

  // RAG 检索
  rpc Retrieve(RetrieveRequest) returns (RetrieveResponse);
}

message GenerateRequest {
  mox.common.RequestMeta meta = 1;
  string prompt = 2;
  string system_prompt = 3;
  string model = 4;              // 不填则自动路由
  map<string, string> variables = 5;
  double temperature = 6;
  int32 max_tokens = 7;
  repeated Message messages = 8; // 多轮对话
  bool stream = 9;
}

message GenerateResponse {
  string chunk = 1;              // 流式返回的 token 片段
  bool is_end = 2;
  string model = 3;
  int32 prompt_tokens = 4;
  int32 completion_tokens = 5;
  int32 total_tokens = 6;
  mox.common.ErrorCode code = 7;
  string message = 8;
}

message Message {
  string role = 1;    // system/user/assistant/tool
  string content = 2;
}
```

### 3.5 gRPC 拦截器链（复用 axum middleware 思想）

`libs/mox-rpc/src/interceptor/mod.rs`：

```rust
use tonic::{Request, Status, service::Interceptor};

// 拦截器执行顺序（从外到内）
pub struct MoxInterceptorChain {
    tenant: TenantInterceptor,
    auth: AuthInterceptor,
    trace: TraceInterceptor,
    rate_limit: RateLimitInterceptor,
    log: LogInterceptor,
}

impl Interceptor for MoxInterceptorChain {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        // 1. 租户染色：从 metadata 提取 tenant_id，写入 extensions
        req = self.tenant.call(req)?;
        // 2. 认证：JWT 验证，获取用户信息
        req = self.auth.call(req)?;
        // 3. 链路追踪：注入/提取 trace_id
        req = self.trace.call(req)?;
        // 4. 限流配额：按租户/用户检查配额
        req = self.rate_limit.call(req)?;
        // 5. 日志：记录调用元数据
        req = self.log.call(req)?;
        Ok(req)
    }
}
```

### 3.6 服务端模板（每个服务统一模式）

```rust
// services/ai/src/main.rs
use tonic::transport::Server;
use mox_rpc::interceptor::MoxInterceptorChain;
use mox_o11y::init_tracing;
use mox_ai_svc::AIServiceImpl;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 初始化可观测性
    init_tracing("mox-ai-svc").await?;

    // 2. 加载配置
    let config = mox_config::load("mox-ai-svc").await?;

    // 3. 注册服务发现
    let _lease = mox_discovery::register("mox-ai-svc", &config.grpc_addr).await?;

    // 4. 构建拦截器链
    let interceptor = MoxInterceptorChain::new(&config).await?;

    // 5. 启动 gRPC 服务
    let svc = AIServiceImpl::new(&config).await?;
    let ai_svc = ai_proto::ai_service_server::AiServiceServer::with_interceptor(svc, interceptor);

    Server::builder()
        .add_service(tonic_health::server::health_reporter())
        .add_service(tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(ai_proto::FILE_DESCRIPTOR_SET)
            .build()?)
        .add_service(ai_svc)
        .serve(config.grpc_addr.parse()?)
        .await?;

    Ok(())
}
```

### 3.7 客户端调用模板

```rust
// 服务 A 调用服务 B
use mox_rpc::client::MoxChannel;

// 从服务发现获取地址，构建带拦截器的 channel
let channel = MoxChannel::builder("mox-graph-svc")
    .with_load_balancer()       // 客户端负载均衡
    .with_retry(3)              // 自动重试
    .with_timeout(Duration::from_secs(30))
    .connect()
    .await?;

let mut client = GraphClient::new(channel);

// 调用时自动注入 tenant_id / trace_id
let resp = client.query_graph(QueryGraphRequest {
    meta: Some(RequestMeta { tenant_id: ctx.tenant_id().into(), .. }),
    query: "MATCH (n) RETURN n LIMIT 10".into(),
}).await?;
```

---

## 四、多租户架构（全链路）

### 4.1 租户上下文传递

```
客户端 → API Gateway（REST/gRPC-Web）
  → 提取 tenant_id（从 subdomain / JWT / header）
  → 注入 gRPC metadata
  → 后端服务（TenantInterceptor 提取 → ThreadLocal / task-local）
  → 数据层（自动追加 tenant_id 条件 + RLS 兜底）
```

### 4.2 数据隔离策略（三档）

| 级别 | 实现 | 适用场景 | Rust 实现 |
|------|------|----------|-----------|
| **共享库 + tenant_id** | 所有表加 tenant_id，sqlx 拦截器自动追加 WHERE tenant_id = ? | 中小企业，默认 | `mox-db` 层封装 sqlx QueryBuilder |
| **共享库 + Schema** | 每租户一个 PostgreSQL Schema，`SET search_path` | 中型企业 | 动态数据源路由 |
| **独立数据库** | 每租户独立 DB 实例 | 大型企业/金融 | `mox-tenant` 动态数据源管理 |

### 4.3 sqlx 租户拦截器

```rust
// libs/mox-db/src/tenant.rs
use sqlx::{Executor, Postgres};
use mox_tenant::TenantContext;

/// 租户感知的查询执行器
pub struct TenantExecutor<'c, E: Executor<'c, Database = Postgres>> {
    inner: E,
    tenant_id: String,
}

impl<'c, E> TenantExecutor<'c, E>
where E: Executor<'c, Database = Postgres>
{
    /// 自动为查询追加 tenant_id 条件
    pub async fn fetch_tenant_scoped<'q, T>(
        &self,
        sql: &'q str,
    ) -> Result<Vec<T>, sqlx::Error>
    where T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'q
    {
        // 解析 SQL AST，在 WHERE 子句追加 AND tenant_id = $1
        let scoped_sql = append_tenant_condition(sql, &self.tenant_id);
        sqlx::query_as(&scoped_sql)
            .bind(&self.tenant_id)
            .fetch_all(self.inner)
            .await
    }
}
```

### 4.4 租户管理服务（mox-tenant-svc）

```protobuf
service TenantService {
  rpc CreateTenant(CreateTenantRequest) returns (Tenant);
  rpc GetTenant(GetTenantRequest) returns (Tenant);
  rpc UpdateTenant(UpdateTenantRequest) returns (Tenant);
  rpc DeleteTenant(DeleteTenantRequest) returns (google.protobuf.Empty);
  rpc ListTenants(ListTenantsRequest) returns (ListTenantsResponse);
  rpc GetQuota(GetQuotaRequest) returns (Quota);
  rpc UpdateQuota(UpdateQuotaRequest) returns (Quota);
  rpc CheckQuota(CheckQuotaRequest) returns (CheckQuotaResponse);
}

message Tenant {
  string id = 1;
  string name = 2;
  string plan = 3;           // free/pro/enterprise
  string status = 4;         // active/suspended/deleted
  string isolation_level = 5; // shared/schema/dedicated
  map<string, string> config = 6;
  google.protobuf.Timestamp created_at = 7;
  google.protobuf.Timestamp expires_at = 8;
}

message Quota {
  int64 max_users = 1;
  int64 max_projects = 2;
  int64 max_storage_gb = 3;
  int64 ai_tokens_per_month = 4;
  int64 ai_concurrent_requests = 5;
  int64 api_calls_per_minute = 6;
}
```

---

## 五、网关增强（mox-gateway-svc）

### 5.1 已有能力（gateway/runtime）

- ai_router（AI 路由）
- rbac_middleware（RBAC 中间件）
- openapi（OpenAPI 规范）
- sidecar（Node.js sidecar 降级）
- market（模板市场路由）
- automation（自动化）

### 5.2 需增强的能力

| 能力 | 实现 | 说明 |
|------|------|------|
| **多协议入口** | axum + tonic-web + tokio-tungstenite | 同一端口支持 REST / gRPC-Web / WebSocket |
| **租户识别** | subdomain / JWT / X-Tenant-Id | 从请求提取 tenant_id |
| **认证** | JWT 验证 + SSO | 统一登录入口 |
| **限流** | 按租户/用户/IP 限流 | 使用 redis 滑动窗口 |
| **路由** | 路径 → gRPC 服务映射 | REST → gRPC 转码 |
| **可观测性** | otel tracing + prometheus metrics | 全链路追踪 |
| **健康检查** | /health + gRPC health | K8s liveness/readiness |

### 5.3 多协议单端口架构

```rust
// services/gateway/src/main.rs
use axum::Router;
use tower::ServiceBuilder;
use tonic_web::GrpcWebLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // REST 路由
    let rest_routes = Router::new()
        .nest("/api/v1", rest_handlers())
        .layer(ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(RateLimitLayer::new())
            .layer(TenantMiddleware::new())
            .layer(AuthMiddleware::new())
        );

    // gRPC-Web 服务（通过 tonic-web 暴露给浏览器）
    let grpc_services = tonic::transport::Server::builder()
        .accept_http1(true)
        .layer(GrpcWebLayer::new())
        .add_service(ai_proto::AiServiceServer::new(ai_client()))
        .add_service(graph_proto::GraphServiceServer::new(graph_client()));

    // 合并到同一 axum 服务
    let app = Router::new()
        .merge(rest_routes)
        .merge(grpc_services.into_router());  // tonic 提供 into_router()

    // WebSocket 升级（axum 原生支持）
    let app = app.route("/ws", axum::routing::get(ws_handler));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

---

## 六、可观测性全维升级

### 6.1 三大支柱

| 支柱 | 技术 | 用途 |
|------|------|------|
| **Logs** | tracing + tracing-subscriber (json) | 结构化日志 |
| **Metrics** | prometheus + autometrics | QPS/延迟/错误率 |
| **Traces** | opentelemetry-otlp → Jaeger/Tempo | 分布式链路追踪 |

### 6.2 统一初始化

```rust
// libs/mox-o11y/src/lib.rs
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub async fn init_tracing(service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 1. OTLP Tracer（导出到 Jaeger/Tempo）
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint("http://otel-collector:4317"))
        .with_trace_config(opentelemetry::sdk::trace::config()
            .with_resource(opentelemetry::sdk::Resource::new(vec![
                opentelemetry::KeyValue::new("service.name", service_name),
                opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            ])))
        .install_batch(opentelemetry::runtime::Tokio)?;

    // 2. tracing 层 + JSON 格式化日志
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    Ok(())
}
```

### 6.3 gRPC 拦截器中的 Trace 传播

```rust
// libs/mox-rpc/src/interceptor/trace.rs
use opentelemetry::propagation::TextMapPropagator;
use opentelemetry::global::get_text_map_propagator;
use tonic::metadata::MetadataMap;

struct MetadataCarrier<'a>(&'a mut MetadataMap);

impl opentelemetry::propagation::Injector for MetadataCarrier<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(key) = tonic::metadata::MetadataKey::from_bytes(key.as_bytes()) {
            self.0.insert(key, value.parse().unwrap());
        }
    }
}

// 客户端：注入 trace context 到 gRPC metadata
pub fn inject_trace_context(req: &mut tonic::Request<()>) {
    get_text_map_propagator(|propagator| {
        let mut carrier = MetadataCarrier(req.metadata_mut());
        propagator.inject_context(&opentelemetry::Context::current(), &mut carrier);
    });
}
```

---

## 七、AI 编排层（Rust 原生）

### 7.1 架构

```
mox-ai-svc（Rust 原生编排）
├── Prompt 管理（版本化、A/B测试）
├── 模型路由（按任务/成本/延迟选择模型）
├── 多 Agent 编排（DAG 工作流，复用 primiflow）
├── RAG 管道（文档切分→向量化→检索→重排序→生成）
├── 语义缓存（相似问题复用回答，降低成本）
├── Guardrails（输出安全校验、事实一致性）
├── 成本追踪（每请求 token 消耗、费用归因到租户）
└── 模型推理 Sidecar（Python，通过 gRPC 调用）
    ├── vLLM / llama.cpp（开源模型本地推理）
    └── OpenAI / Claude / Gemini（外部 API）
```

### 7.2 为什么编排层用 Rust，推理用 Python sidecar

| 层 | 语言 | 原因 |
|----|------|------|
| **编排层** | Rust | 高并发、低延迟、类型安全、流式处理、成本控制 |
| **推理层** | Python sidecar | PyTorch/HF 生态、vLLM、模型微调、GPU 加速 |
| **通信** | gRPC streaming | Rust ↔ Python 流式 token 传输 |

### 7.3 Python 推理 Sidecar 的 gRPC 服务

```python
# ai_inference_sidecar/server.py（Python，仅做推理）
import grpc
from concurrent import futures
import ai_pb2
import ai_pb2_grpc

class InferenceService(ai_pb2_grpc.AIServiceServicer):
    def GenerateStream(self, request, context):
        # 调用 vLLM / OpenAI，流式返回 token
        for chunk in llm.generate_stream(request.prompt):
            yield ai_pb2.GenerateResponse(chunk=chunk, is_end=False)
        yield ai_pb2.GenerateResponse(chunk="", is_end=True)

server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
ai_pb2_grpc.add_AIServiceServicer_to_server(InferenceService(), server)
server.add_insecure_port('[::]:50051')
server.start()
server.wait_for_termination()
```

Rust 编排层通过 tonic 客户端调用这个 Python sidecar：

```rust
// Rust 编排层调用 Python 推理 sidecar
let mut inference_client = InferenceClient::connect("http://localhost:50051").await?;
let stream = inference_client.generate_stream(request).await?.into_inner();
while let Some(chunk) = stream.message().await? {
    // 后处理：Guardrails 校验、成本统计、缓存写入
    tx.send(chunk).await?;
}
```

---

## 八、知识图谱层（Rust 原生）

### 8.1 模块拆分

| 服务 | 职责 | 技术 |
|------|------|------|
| **mox-graph-svc** | 图谱核心 API（CRUD、查询、遍历） | tonic + sqlx |
| **mox-graph-storage-svc** | 存储引擎（邻接表、属性图、持久化） | 自研 + sled/rocksdb |
| **mox-graph-algo-svc** | 图算法（最短路径、社区发现、PageRank、中心性） | petgraph + rayon 并行 |
| **mox-graph-streams-svc** | 图流处理（实时更新、增量计算） | tokio + 差分数据流 |
| **mox-graph-meta-svc** | 图谱元数据（Schema、索引、约束） | sqlx |

### 8.2 存储引擎选型

| 方案 | 优点 | 缺点 | 建议 |
|------|------|------|------|
| **PostgreSQL + 邻接表** | 事务、ACID、生态成熟 | 超大规模图性能一般 | 默认起步 |
| **sled / rocksdb（嵌入式 KV）** | 极高性能、零依赖 | 需自研图查询层 | 大规模时引入 |
| **Neo4j（外部）** | 成熟图数据库、Cypher | Java 依赖、非 Rust 原生 | 企业版可选 |
| **自研 Rust 图引擎** | 完全可控、极致性能 | 开发量大 | 长期目标 |

**建议**：先用 PostgreSQL + sqlx 做存储，算法层用 petgraph + rayon 在内存中计算；超大规模时迁移到自研 KV 存储引擎。

---

## 九、插件体系（WASM，已有基础需完善）

### 9.1 已有基础

- wasmer 4.2 运行时
- operator-wasm（WASM 算子）
- mox-etl-wasm（WASM ETL）

### 9.2 需完善

| 能力 | 实现 | 说明 |
|------|------|------|
| **插件 Manifest** | `plugin.yaml` + WASM custom section | 声明能力、权限、依赖 |
| **权限沙箱** | wasmer capabilities + 资源限制 | 内存/CPU/系统调用限制 |
| **生命周期** | install/enable/disable/upgrade/uninstall | 热加载，无需重启 |
| **插件 API** | WASI + 自定义 host function | 插件通过 RPC 调用平台能力 |
| **插件市场** | mox-market-svc | 上传、审核、发布、计费 |
| **版本管理** | 语义化版本 + 回滚 | 多版本并存 |

### 9.3 插件通过 gRPC 调用平台

插件运行在 WASM 沙箱中，通过 host function 发起 gRPC 调用：

```rust
// 平台侧：为 WASM 插件提供 gRPC 调用 host function
// libs/mox-plugin/src/host.rs
use wasmer::{Function, Store, Value};

pub fn register_grpc_host_functions(store: &mut Store, plugin_tenant: &str) -> Vec<(String, Function)> {
    vec![
        ("grpc_call".to_string(), Function::new_typed(store, |service: String, method: String, payload: Vec<u8>| -> Result<Vec<u8>, String> {
            // 插件只能调用声明过的服务，且自动注入租户上下文
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async move {
                mox_rpc::client::call_plugin(&service, &method, &payload, plugin_tenant).await
            })
        })),
        ("log".to_string(), Function::new_typed(store, |level: i32, msg: String| {
            tracing::event!(match level { 1=>tracing::Level::WARN, 2=>tracing::Level::ERROR, _=>tracing::Level::INFO }, "plugin: {}", msg);
        })),
    ]
}
```

---

## 十、部署架构（Kubernetes 微服务）

### 10.1 服务清单（可独立部署）

```
infotopograph-platform/
├── mox-gateway-svc          # API 网关（2+ 副本，HPA）
├── mox-auth-svc             # 认证服务（2 副本）
├── mox-tenant-svc           # 租户管理（2 副本）
├── mox-system-svc           # 系统管理（2 副本）
├── mox-ai-svc               # AI 编排（2+ 副本，HPA by QPS）
│   └── ai-inference-sidecar # Python 推理（GPU 节点，独立 Deployment）
├── mox-agent-svc            # AI Agent（2 副本）
├── mox-expert-svc           # 专家系统（2 副本）
├── mox-graph-svc            # 图谱核心（2 副本）
├── mox-graph-storage-svc    # 图谱存储（StatefulSet）
├── mox-graph-algo-svc       # 图算法（HPA by CPU）
├── mox-graph-streams-svc    # 图流处理（2 副本）
├── mox-graph-meta-svc       # 图谱元数据（2 副本）
├── mox-operator-svc         # 算子服务（2 副本）
├── mox-etl-svc              # ETL（2 副本）
├── mox-dataplane-svc        # 数据平面（2 副本）
├── mox-optimizer-svc        # 优化器（2 副本）
├── mox-flow-svc             # 流程引擎（2 副本）
├── mox-flow-fusion-svc      # 流程融合（2 副本）
├── mox-storage-svc          # 云存储（2 副本）
├── mox-compliance-svc       # 合规审计（2 副本）
├── mox-fusion-svc           # 融合引擎（2 副本）
├── mox-catalog-svc          # 业务目录（2 副本）
├── mox-market-svc           # 模板市场（2 副本）
├── mox-metering-svc         # 计量计费（2 副本）
└── mox-notification-svc     # 通知服务（2 副本）
```

### 10.2 基础设施

| 组件 | 选型 | 用途 |
|------|------|------|
| 容器编排 | Kubernetes | 服务编排 |
| 服务网格 | Istio / Linkerd | 流量管理、mTLS、可观测性 |
| 服务发现 | K8s Service + CoreDNS | 服务注册发现（k8s 原生，无需额外组件） |
| 配置管理 | K8s ConfigMap + Secret | 配置中心（起步阶段） |
| 关系数据库 | PostgreSQL 16 | 主数据存储（含 RLS） |
| 缓存 | Redis 7 | 缓存、会话、限流 |
| 消息队列 | NATS JetStream | 异步事件、流处理 |
| 对象存储 | MinIO / S3 | 文件、模型、插件包 |
| 向量库 | pgvector / Qdrant | RAG 检索 |
| 可观测性 | OTel Collector + Jaeger + Prometheus + Grafana + Loki | 全栈监控 |
| 日志聚合 | Loki | 日志收集查询 |
| CI/CD | ArgoCD + GitHub Actions | GitOps 部署 |

---

## 十一、分阶段实施路线图

### 阶段一：工程治理 + 命名统一（2 周）

| 任务 | 产出 |
|------|------|
| 清理根目录 | 删除 30+ .log，完善 .gitignore |
| 大文件迁移 | graph.json → Git LFS / 对象存储 |
| 目录重组 | platform/services → services, platform/crates → crates, 新增 libs/ proto/ |
| crate 重命名 | 36个服务统一 mox-*-svc 命名 |
| 合并重叠模块 | flow-ai+mox-ai-core→mox-ai-svc, kg-hub+mox-graph-service→mox-graph-svc 等 |
| Workspace 整理 | 统一版本、依赖、feature flag |

### 阶段二：gRPC 基础设施 + 共享库（4 周）

| 任务 | 产出 |
|------|------|
| 引入 tonic 生态 | workspace.dependencies 新增 tonic/prost/tonic-build |
| proto/ 目录建立 | 公共 proto + 各服务 proto |
| mox-rpc 库 | 拦截器链、客户端封装、错误码、公共类型 |
| mox-o11y 库 | otel tracing + prometheus + 统一初始化 |
| mox-db 库 | sqlx 封装 + 租户感知查询 + 连接池 |
| mox-cache 库 | redis + 内存缓存抽象 |
| mox-mq 库 | nats 封装 |
| 代码生成 | build.rs 自动编译 proto → Rust |
| 网关 gRPC-Web | gateway 支持 gRPC-Web 协议 |

### 阶段三：多租户 + 认证 + 计量（4 周）

| 任务 | 产出 |
|------|------|
| mox-tenant 库 | 租户上下文、拦截器、数据隔离 |
| mox-tenant-svc | 租户 CRUD、配额管理 |
| mox-auth 库 | JWT、SSO、RBAC（从 gateway 抽取） |
| mox-auth-svc | 登录、注册、Token 管理 |
| mox-metering-svc | 用量统计、配额检查、计费数据 |
| 数据库迁移 | 所有表加 tenant_id，启用 RLS |
| 网关租户识别 | subdomain/JWT/header 提取 tenant_id |

### 阶段四：核心服务 gRPC 化（6 周）

| 任务 | 产出 |
|------|------|
| mox-ai-svc gRPC 化 | GenerateStream 流式接口、模型路由、RAG |
| mox-graph-svc gRPC 化 | 图谱 CRUD、查询、遍历 |
| mox-graph-algo-svc gRPC 化 | 图算法接口 |
| mox-system-svc gRPC 化 | 用户/角色/权限 |
| mox-storage-svc gRPC 化 | 文件上传下载（流式） |
| mox-operator-svc gRPC 化 | 算子注册/执行 |
| Python 推理 sidecar | gRPC 服务，对接 vLLM/OpenAI |
| 服务间调用改造 | REST 调用 → gRPC 调用 |

### 阶段五：插件体系 + 生态（4 周）

| 任务 | 产出 |
|------|------|
| 插件 Manifest 规范 | plugin.yaml + WASM custom section |
| 插件沙箱完善 | 权限、资源限制、host function |
| 插件生命周期 | 热加载、版本管理、回滚 |
| mox-market-svc | 插件/模板市场 |
| 插件 SDK | Rust/AssemblyScript 插件开发模板 |
| 示例插件 | 3-5 个官方插件示范 |

### 阶段六：可观测性 + 部署 + 压测（持续）

| 任务 | 产出 |
|------|------|
| 全链路 tracing | 所有服务接入 otel |
| Grafana 仪表盘 | QPS/延迟/错误率/资源/成本 |
| 告警规则 | PagerDuty/飞书告警 |
| K8s manifests | 每服务 Deployment + Service + HPA |
| Helm chart | 一键部署 |
| 压测 | k6/ghz 压测 gRPC，性能基线 |
| 混沌工程 | 故障注入测试 |

---

## 十二、关键技术决策总结

| 决策点 | 选择 | 理由 |
|--------|------|------|
| **后端语言** | 全 Rust | 高性能、内存安全、统一技术栈 |
| **RPC 框架** | tonic (gRPC) | Rust 生态最成熟、流式原生、多语言支持 |
| **Web 框架** | axum 0.7 | 已有、tokio 生态、tower 中间件 |
| **数据库** | PostgreSQL + sqlx | 事务、RLS 多租户、Rust 异步原生 |
| **缓存** | Redis (fred/redis crate) | 成熟、限流/会话/缓存 |
| **消息队列** | NATS JetStream | 轻量、Rust 客户端好、内置持久化 |
| **服务发现** | K8s Service + CoreDNS | 零额外组件、k8s 原生 |
| **配置中心** | K8s ConfigMap（起步）→ etcd（规模化） | 渐进式 |
| **可观测性** | OpenTelemetry + Jaeger + Prometheus + Loki | 行业标准、全链路 |
| **AI 推理** | Python sidecar (vLLM) + Rust 编排 | 生态分工，gRPC 通信 |
| **插件** | WASM (wasmer) | 已有基础、沙箱安全、多语言 |
| **图存储** | PostgreSQL（起步）→ 自研 KV（规模化） | 渐进式 |
| **容器编排** | Kubernetes | 行业标准 |
| **CI/CD** | ArgoCD + GitHub Actions | GitOps |

---

## 十三、一句话总结

**infotopograph 已有非常扎实的 Rust 基础（50+ crate、tokio/axum/sqlx/wasmer/petgraph 全栈就绪），缺的是 gRPC 通信骨架、多租户隔离、统一命名和微服务化。最优路径是：先做工程治理和命名统一，再引入 tonic gRPC 作为服务间通信主干，同步建设 mox-rpc/mox-tenant/mox-o11y 等共享库，然后将 36 个服务逐一 gRPC 化并可独立部署，AI 推理用 Python sidecar 通过 gRPC 对接，最终形成全 Rust 微服务 + 多租户 + 插件化的 SaaS AI 平台。**