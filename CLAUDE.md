# CLAUDE.md

> 本文件是 AI 编码上下文规范（项目「关图 / 璇玑」算子统一全维治理系统）。深度架构、需求、验收、路线图见 `docs/enterprise/00-INDEX.md` 与 `docs/full-dimensional/`，本文件只规定**编码与架构约束**，不重复文档集内容。

## 项目概览

关图（算子统一全维分析系统）是一套「需求 → 归一化 IR → 双璇玑十四维诊断 → 裁决 → flow-ai 求解 → ⛨验证网关 → 治理 8 闸门 → 出码/出图」的 AI 自动化中枢，面向企业级全维治理与合规出码。后端为 Rust（Axum + Tokio）模块化单体（6层8域DDD矩阵 · 73 crate workspace），前端为 Vue3 管理控制台。

技术栈：
- 后端：Rust edition 2021，Axum（HTTP）、Tokio（异步）、Serde、Anyhow/Thiserror；`cargo` 工作区（73 个 crate · 6层8域DDD矩阵）。
- 前端：Vue 3.4 + Vite 5 + Element Plus 2.4 + vue-router 4.3 + Axios 1.6 + ECharts 5.4 + three / 3d-force-graph；包管理 npm。
- 部署：后端 `mox-platform-gateway-svc` crate 聚合为单一 `operator-server` 二进制（默认 `:3001`，可由 `--port` 覆盖）；`platform/backend-node/`（零依赖 Node）作为边缘入口占 `:3000`，托管 `frontend-ui/dist` 并将 `/api` 反向代理到 Rust；前端 Vite 代理 `/api` → `http://localhost:3000`（即 Node 边缘入口）。Node 不再实现任何领域逻辑，出码统一经 Rust ⛨验证网关 + 治理 8 闸门。
- 架构模型 v2.0：6层8域DDD矩阵 — L0 Foundation（横切基础）/ L1 Gateway（网关）/ L2 Core（8域领域模型）/ L3 Svc（8域应用服务）/ L4 Sdk（8域对外类型）/ L5 Api（8域域间契约，规划中）。8域 = ai / cloud / data / flow / kg / market / platform / voice。旧 `platform/services/` 15-crate扁平模型已废弃，旧→新映射见 `docs/enterprise/ARCHITECTURE-MIGRATION.md`。

## 根目录总览

> AI 操作前必读：明确文件该放到哪里，不要在根目录随意新建散落的目录/文件。

```
infotopograph/
├── platform/          # 后端 Rust 代码（6层8域DDD矩阵 · 模块化单体）
├── frontend-ui/       # 前端 Vue3 代码（用户端 + 管理控制台）
├── docs/              # 文档：架构设计、需求规格、ADR、工作汇报等
│   ├── enterprise/    # 企业级需求/架构/设计文档（最权威）
│   ├── architecture/  # 架构设计文档
│   ├── working-reports/ # 工作汇报/周报/专项报告
│   └── ...            # 其他领域文档
├── reports/           # 产出物：HTML报告、Markdown报告、数据文件
│   ├── html/          # HTML 可视化报告（各报告子目录 + 公共 _shared/）
│   ├── markdown/      # Markdown 格式报告
│   ├── data/          # 报告的数据文件（JSON、日志等）
│   └── _shared/       # HTML 报告共享资源（字体、echarts、mermaid）
├── prototypes/        # HTML 原型/演示项目（非生产代码）
│   ├── _shared/       # 原型共享资源（字体、echarts、mermaid）
│   └── <各原型项目>/
├── deploy/            # 部署配置（helm、docker、nginx、systemd、sql）
├── data/              # 运行时数据（cache、storage、uploads、exports）
├── plugins/           # 插件（extensions、scripts、wasm）
├── projects/          # 子项目/实验性项目
├── log/               # 日志文件
├── .runtime/          # 运行时脚本
├── .github/           # CI/CD 工作流
├── CLAUDE.md          # ← 你正在读的 AI 编码上下文规范
├── README.md          # 项目说明
├── ARCHITECTURE.md    # 架构总览
└── Cargo.toml         # Rust workspace 根
```

**放置规则（AI 必须遵守）：**
- HTML 可视化报告 → `reports/html/<报告名>/`，共享资源用 `reports/_shared/`
- Markdown 报告 → `reports/markdown/`，报告数据 → `reports/data/`
- HTML 原型/演示 → `prototypes/<项目名>/`，共享资源用 `prototypes/_shared/`
- 架构/需求/设计文档 → `docs/` 下对应子目录
- 生产代码 → `platform/`（后端）或 `frontend-ui/`（前端）
- **禁止在根目录新建散落的报告/原型目录**

## 目录结构与模块职责

> 写职责，不贴完整文件树。后端按「分层」理解，前端按「功能切片」理解。

### 后端（`platform/`，6层8域DDD矩阵 · 模块化单体）

**横切层（L0 Foundation + L1 Gateway + Framework）：**
- `foundation/mox-platform-foundation/`：平台基础库 — 通用类型、错误处理（thiserror/anyhow统一）、配置、工具函数、tracing初始化。
- `foundation/mox-cloud-foundation/`：云基础设施基础库 — 云存储抽象、卷管理、S3适配、文件器接口。
- `gateway/mox-platform-gateway-svc/`：**控制层（Controller）**。Axum `Router` 按域挂载子路由；`routes/`（路由表）、`handlers/`（处理器薄层）、`middleware/`（RBAC鉴权/限流/CORS/日志）、`ws/`（WebSocket）、`openapi.rs`（OpenAPI/Swagger）。**这是唯一允许直接碰 HTTP 的层；仅做路由+横切中间件，业务聚合下沉到各域svc层**。
- `framework/`（mox-framework）：插件框架层 — 扩展点定义、插件注册（库）。

**8域 × L2 Core（领域模型 · 纯业务逻辑 · 无I/O依赖）：**
- `domains/ai/core/`：`mox-ai-core`（AI统一内核/LLM客户端抽象）、`mox-ai-intent-core`（意图识别/A5激活扩散路由核心）
- `domains/kg/core/`：`mox-kg-algo-core`（八大算法A1~A8：CNM/Brandes/Harmonic/PageRank/激活扩散/RRF/CEM/CPM）、`mox-kg-meta-core`（本体/Schema/14节点族×19边族）
- `domains/flow/core/`：`mox-flow-operator-core`（算子代数/守恒律/范畴论/单子/Registry）、`mox-flow-optimizer-core`（CPM关键路径/RCPSP资源约束/CEM交叉熵）
- `domains/data/core/`：`mox-data-formula-core`（高精度公式引擎）、`mox-data-norm-core`（归一化IR/六维绑定）、`mox-data-standards-core`（数据标准/Schema）
- `domains/platform/core/`：`mox-platform-system-core`（成员/任务/权限/通信领域模型/Store接口/EventBus/RBAC）、`mox-platform-iam-core`（身份认证/令牌/访问控制）、`mox-platform-meta-core`（AisLayer枚举/CrateMeta/all_crate_metas）、`mox-platform-datastore-core`（多后端SQLite/PG/MySQL抽象/方言归一化/迁移）、`mox-platform-orchestrator-core`（DAG编排/事件反应器/鉴权闸门require()）
- `domains/voice/core/`：`mox-voice-dsp-core`（响度归一/软限幅/Aho-Corasick热词/SIMD f32x4）

**8域 × L3 Svc（应用服务 · HTTP handler/业务编排/DB repo/外部API client）：**
- `domains/ai/svc/`：`mox-ai-agent-svc`（对话/浏览器自动化/MultiAgent/ProviderRegistry/A7 CEM）、`mox-ai-expert-svc`（⛨璇玑14专家/归一化IR/裁决/验证5项/审计三汇/RBAC/租户分层）、`mox-ai-flow-svc`（流程AI 9模块/代码生成）
- `domains/kg/svc/`：`mox-kg-storage-svc`、`mox-kg-service-svc`、`mox-kg-streams-svc`、`mox-kg-spark-svc`、`mox-kg-hub-svc`（混合索引+URN+8段5连接器）、`mox-kg-fusion-svc`（RRF融合/实体对齐）
- `domains/flow/svc/`：`mox-flow-operator-wasm-svc`（WASM沙箱/wasmer/热加载）、`mox-flow-primiflow-svc`（解析/代码生成/8类骨架）、`mox-flow-fusion-svc`（六维融合/守恒闸门/Registry）、`mox-flow-bridge-svc`（Hermes桥接/normalize/recorder）
- `domains/data/svc/`：`mox-data-plane-svc`、`mox-data-etl-svc`、`mox-data-compliance-svc`（PII检测/脱敏）、`mox-data-catalog-svc`（6预置FlowGraph）
- `domains/platform/svc/`：`mox-platform-enterprise-svc`（企业服务/成员任务权限通信编排/多后端）、`mox-platform-orchestrator-svc`（编排器服务）
- `domains/cloud/svc/`：`mox-cloud-master-svc`、`mox-cloud-volume-svc`、`mox-cloud-s3-svc`、`mox-cloud-filer-svc`
- `domains/market/svc/`：`mox-market-template-svc`（发布/加载/评分/Fork/2种子）
- `domains/voice/svc/`：`mox-voice-core-svc`、`mox-voice-asr-svc`、`mox-voice-intent-svc`、`mox-voice-operator-svc`、`mox-voice-desktop-app`（**独立产品形态·全局热键/BallWidget/键鼠自动化**）

**8域 × L4 Sdk（对外类型 · FFI绑定 · 客户端库）：**
- `domains/kg/sdk/mox-kg-sdk`、`domains/cloud/sdk/mox-cloud-sdk`、`domains/platform/sdk/mox-platform-test-harness`
- `domains/data/sdk/mox-data-formula-native`（napi-rs Node.js FFI）、`mox-data-norm-intent-native`（napi-rs）
- `domains/voice/sdk/mox-voice-dsp-py`（PyO3 abi3-py39 Python绑定）

**8域 × L5 Api（域间契约 · 规划中 · 0 crate · Phase 3填充）：**
- 每个域的 `api/` 和 `svcapi/` 目录已创建但零crate。设计意图：定义域间通信的trait/interface/DTO，实现依赖倒置。Phase 3优先填充kg/ai/flow三个核心域。

**跨域依赖规则（强制 · ADR-09）：**
1. svc层只能依赖同域core + 其他域的core/sdk/api（**禁止直接依赖其他域的svc**）
2. core层只能依赖foundation + 其他域的core/sdk（**禁止依赖任何svc**）
3. sdk层只能依赖同域core的类型定义
4. api层只能依赖core的类型，定义trait供svc实现
5. 所有跨域调用必须通过api层trait（依赖倒置），Phase 3完成后强制执行arch test

### 前端（`frontend-ui/src/`，用户端 + 系统管理区）

**用户端 (`frontend-ui/`)：**
- `api/`：统一 fetcher（`index.js`，Axios 实例 + 请求/响应拦截器：注入 Bearer 令牌、剥离响应包裹、全局错误提示）。**组件禁止直接 `fetch`/`axios`**。
- `views/`：业务页面（按业务域组织，等价 feature-sliced 的 feature）。
- `components/`：通用 UI 组件，无业务逻辑，纯展示/交互。
- `router/`：vue-router 路由表。
- `styles/`：设计 token / 全局样式（禁止硬编码颜色尺寸）。
- `types.js`：前后端契约类型（FlowGraph / GovernanceReport / 算子 / 插件 等）。

**系统管理区（`frontend-ui/src/views/admin/`，路由 `/admin?tab=`，原 frontend-admin-ui 已裁撤并入）：**
- `panels/AdminOverview.vue`：管理总览（安全状态/存储/模块/LLM 网关概况，真实端点 /security/status、/storage/status、/modules）
- `panels/AdminAccess.vue`：访问凭证管理（API Key 创建/吊销/校验，真实端点 /security/api-keys、/security/validate）
- `panels/AdminAudit.vue`：审计日志（真实端点 /security/audit-log）
- `panels/AdminStorage.vue`：存储提供方切换与模块清单（真实端点 /storage/*、/modules）
- `panels/AdminHitl.vue`：HITL 人机协同审批（WebSocket `/ws/hitl`，经 Vite `/ws` 代理至 Rust 网关，协议见 `utils/hitl-ws.js`）
- 大模型配置与知识库管理复用既有视图：`/llm-config`、`/knowledge-base`

## 编码规范

- 命名：Rust 遵循语言惯例（snake_case 函数/变量、PascalCase 类型、SCREAMING_SNAKE 常量，crate 名 kebab-case）；前端遵循项目既有（组件 PascalCase、组合式函数 camelCase 以 `use` 开头）。**不要擅自改写已有代码风格**。
- Rust 类型：优先具体类型；错误统一用 `anyhow::Result` 或领域 `Error`（`mox-system::error`）。生产路径禁止裸 `unwrap()`（测试可用）。
- `todo!()` / `unimplemented!()`：**严禁进入生产代码路径**。`primiflow` 代码生成模板内故意下发 `todo!()` 骨架属生成器行为，非运行时代码。
- 错误处理：后端统一经 `runtime::api_standard` 转 REST 错误体；前端统一经 `src/api/index.js` 拦截器，不散落 `try/catch`。
- 样式：复用 `src/styles/` 设计 token，禁止硬编码颜色/尺寸。
- 最小改动原则：**只修改必须改动的代码，不顺手重构无关代码**。

## 架构约束

- 分层约束（Spring-Boot 式）：**Controller（`runtime` routes/handlers + 各 crate `server.rs`）→ Service（`services.rs` / `*_engine.rs` 领域逻辑）→ Repository（`repo/` / `store.rs` / `persistence.rs`）**。禁止跨层直接调用（如 handler 直接写 SQL 或碰 DB 连接）。
- 归一化 IR 契约：全链路以 **FlowGraph / PTDoc** 归一化中间表示为唯一契约，禁止在各层各造一套结构体。
- 状态管理（前端）：当前用 Vue Composition API + props / `provide-inject`，**未引入 Pinia**；新增跨页共享状态先抽组合式函数，确需全局再评估引入。
- 数据请求：统一 `src/api/index.js` fetcher，统一处理鉴权（Bearer）与异常。
- 禁止过度抽象：不为一次性使用提前抽象；新增治理闸门走 `primiflow-fusion::unified` 统一登记，不要散落。
- 出码/发布必经：`/mox/publish` 必须先后经过 `mox-expert::verify`（⛨网关）与 `primiflow-fusion::full_gate`（治理 8 闸门），不得绕过。

## 禁止清单（Do NOT，最重要）

- 不要修改数据库 schema / 历史迁移（`mox-system/src/repo/schema.rs`、market 迁移文件），除非得到明确指令。
- 不要升级核心依赖大版本（axum / tokio / element-plus / vue / vite 等）。
- 不要提交密钥、`.env`、token（`.env` 已被 gitignore；`OUS_API_TOKEN` / `VITE_API_TOKEN` 经环境变量注入，禁止写死进仓库）。
- 不要回退别人已写好的业务逻辑（尤其璇玑验证、治理 8 闸门、租户分层）。
- 不要 `git push --force` 到主分支。
- 不要修改 CI / infra / secrets 相关文件，必须先询问确认。
- 不要在日志输出 PII / 敏感信息（`audit/` 已脱敏，新增汇同样处理）。
- 不要新增 `todo!()` / `unimplemented!()` 到生产代码路径。
- 禁止用 `// TODO: 实现…` 注释占位替代真实逻辑（如空函数体、硬编码成功返回）。未接线的预留 API 必须显式 `#![allow(dead_code)]` + 说明注释（见 `docs/enterprise/11` §6ter 审计口径）。
- 不要在 Controller/route 层直接写业务 SQL 或仓储逻辑（越层）。
- 不要绕过 ⛨验证网关与治理 8 闸门直接出码/发布。

## 测试规范

- 修改业务逻辑必须同步更新单元测试。
- 后端框架：原生 `#[test]` / `#[tokio::test]`；命令 `cargo test --workspace`（沙箱中须 `run_in_background`，前台写 `target/` 会被沙箱拦截）。
- 重点回归面：`mox-expert`（verify 5 检查）、`primiflow-fusion`（full_gate G1–G8）、`mox-system`（repo 多方言）。
- 集成范式：各子服务 `server.rs` 提供 `build_router` 供 `tower::ServiceExt::oneshot` 离线驱动（见 `primiflow-fusion/tests/server_test.rs`）。
- 静态分析：`cargo clippy --workspace --all-targets` **零告警已达成（2026-08-17，188 → 0）**。归零过程：先 `cargo clippy --fix` 修风格 lint，再手动修全部正确性/语义 bug（`await_holding_lock` 死锁、`let _ =` 静默吞错、手动 `clamp` 等），保留但未接线的公共 API（`rbac_middleware` 等）以带说明的 `#![allow(dead_code)]` 标注而非删除。**注意**：`--fix` 对「unused `Result`」可能加 `.ok()` / `let _ =` 静默吞错，此类必须手动处理。
- 前端：当前**未引入单测框架**（Vitest/Jest 缺失）。公共组件变更以 `npm run build` 构建校验兜底；建议后续引入 Vitest 再补组件/E2E 测试。

## Git 与工作流

- 只在当前分支工作，不随意切换分支。
- 提交信息遵循约定式提交：`feat/fix/docs/refactor/test/chore(scope): 描述`。
- 大范围修改前先输出变更文件清单，确认后再写代码。
- 涉及用户行为变更（鉴权、RBAC、治理闸门、出码）必须显式标注风险点。

## 常用命令

### 开发
- `cargo run -p mox-platform-gateway-svc`：启动聚合后端（默认 `:3001`，按域挂载子服务；边缘入口 `platform/backend-node/` 占 `:3000` 并反代 `/api` 至此）
- `cd frontend-ui && npm run dev`：用户端前端开发（Vite，`/api` 代理到 `:3000`；系统管理区 `/admin`，HITL `/ws` 代理至 Rust 网关 `:3001`）
- `cd frontend-ui && npm run build`：用户端生产构建 → `dist/`

### 构建 / 校验
- `cargo build --workspace`：全量构建（沙箱须 `run_in_background`）
- `cargo clippy --workspace --all-targets`：静态检查（目标零告警，已达成 188→0）
- `cargo test --workspace`：全量测试（背景运行，退出码 0 = 全绿）

### 测试（按域/crate）
- `cargo test -p mox-ai-expert-svc`：验证网关与敏感拦截（旧 mox-expert）
- `cargo test -p mox-flow-fusion-svc`：治理 8 闸门（旧 primiflow-fusion）
- `cargo test -p mox-platform-enterprise-svc`：成员/任务/RBAC/审计 + 多方言 repo（旧 mox-system）
- `cargo test -p mox-kg-algo-core`：八大算法 A1~A8 对账（旧 graph-algorithms）
- `cargo test -p mox-flow-operator-core`：算子内核/守恒律/范畴论（旧 operator-core）

### 架构一致性校验
- `cargo metadata --format-version 1 | jq '.packages | length'`：验证 workspace crate 数量（应为73）
- 对照 `docs/enterprise/02-architecture.md` §3.2 的 6层8域crate矩阵表，确认路径一致
- 对照 `docs/enterprise/ARCHITECTURE-MIGRATION.md`，确认无旧路径 `platform/services/` 残留

## 上下文别名（项目黑话翻译）

- **璇玑 / ⛨验证网关**：`mox-expert::verify` 的五项数学/语义检查（S6/G2），出码前必过。
- **治理 8 闸门（GR-STD）**：`primiflow-fusion::unified::full_gate` 的 G1–G8，图归一化体检 + 安全/合规/权限门禁；`gov` 租户驱动 I-06 分层。
- **双璇玑十四维**：业务 7 维 + 开发 7 维并行诊断。
- **归一化 IR / PTDoc**：全链路统一中间表示（FlowGraph 归一化文档），各层契约。
- **关图 / 算子统一**：项目总称；`operator-*` 系列为算子引擎内核。
- **融合（fusion）**：`primiflow-fusion` 把多源图融合 + 注册 + 落库 + 闸门。
- **operator-server**：`runtime` 聚合后的单一对外二进制（原四套独立服务收敛）。
- **OUS_API_TOKEN**：后端鉴权令牌（前端经 `Authorization: Bearer` 注入）。

## 风险与边界

- 修改璇玑验证、治理 8 闸门、RBAC、租户策略、审计汇，优先提示爆炸半径（影响全维出码与合规）。
- 修改 `repo/schema` 或迁移，影响持久化与已有数据，必须请求确认。
- 重大变更先输出方案，用户确认后再实现。
- 部署、数据库高危操作，必须请求确认，禁止直接执行。
