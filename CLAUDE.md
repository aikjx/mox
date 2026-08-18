# CLAUDE.md

> 本文件是 AI 编码上下文规范（项目「关图 / 璇玑」算子统一全维治理系统）。深度架构、需求、验收、路线图见 `docs/enterprise/00-INDEX.md` 与 `docs/full-dimensional/`，本文件只规定**编码与架构约束**，不重复文档集内容。

## 项目概览

关图（算子统一全维分析系统）是一套「需求 → 归一化 IR → 双璇玑十四维诊断 → 裁决 → flow-ai 求解 → ⛨验证网关 → 治理 8 闸门 → 出码/出图」的 AI 自动化中枢，面向企业级全维治理与合规出码。后端为 Rust（Axum + Tokio）单体聚合服务，前端为 Vue3 管理控制台。

技术栈：
- 后端：Rust edition 2021，Axum（HTTP）、Tokio（异步）、Serde、Anyhow/Thiserror；`cargo` 工作区（15 个 crate）。
- 前端：Vue 3.4 + Vite 5 + Element Plus 2.4 + vue-router 4.3 + Axios 1.6 + ECharts 5.4 + three / 3d-force-graph；包管理 npm。
- 部署：后端 `runtime` crate 聚合为单一 `operator-server` 二进制（默认 `:3001`，可由 `--port` 覆盖）；`backend/`（零依赖 Node）作为边缘入口占 `:3000`，托管 `frontend/dist` 并将 `/api` 反向代理到 Rust；前端 Vite 代理 `/api` → `http://localhost:3000`（即 Node 边缘入口）。Node 不再实现任何领域逻辑，出码统一经 Rust ⛨验证网关 + 治理 8 闸门。

## 目录结构与模块职责

> 写职责，不贴完整文件树。后端按「分层」理解，前端按「功能切片」理解。

### 后端（`crates/`，Spring-Boot 风格分层）

- `runtime/`：**控制层（Controller）**。Axum `Router` 聚合四套子服务；`routes/`（路由表）、`handlers/`（处理器）、`api_standard.rs`（统一 REST 错误体）、`rbac_middleware.rs`（鉴权中间件）、`openapi.rs`（OpenAPI/Swagger）、`subservers.rs`（服务聚合边界）。**这是唯一允许直接碰 HTTP 的层**。
- `xuanji-system/`：**服务层 + 仓储层**。业务编排（`services.rs`/`orchestrator.rs`）、领域错误（`error.rs`/加密 `crypto.rs`/事件 `event.rs`/限流 `ratelimit.rs`/指标 `metrics.rs`）、RBAC（`rbac.rs`）；持久化在 `repo/`（mysql/postgres/sqlite/schema）与 `store.rs`。
- `xuanji-expert/`：**璇玑治理与验证引擎**。`verify/`（⛨验证网关 S6/G2 五项数学/语义检查）、`rbac/`（策略与判定）、`audit/`（nats/rabbitmq/s3/syslog 多汇，已脱敏）、`sensitivity.rs`（敏感拦截）、`tenant_policy.rs`（租户分层：default / gov）、`govern.rs`/`reconcile.rs`。
- `primiflow-fusion/`：**治理 8 闸门融合层（GR-STD）**。`unified.rs::full_gate` 实现 G1–G8、`sixdim.rs`（六维）、`ptdoc.rs`（归一化文档）、`platform.rs`（注册/落库）、`server.rs`（对外 REST，离线可 `oneshot` 驱动）。
- `primiflow/`：六维溯源拓扑引擎——`executor.rs`/`runner.rs`/`server.rs`/`persistence.rs`（固化）。
- `flow-ai/`：需求→代码自动生成——`codegen.rs`/`pipeline.rs`/`topology.rs`/`schedule.rs`/`conflict.rs`。
- `ai-agent/`：对话/工作流/LLM 编排——`conversation.rs`/`workflow_engine.rs`/`llm_client.rs`/`dialogue_graph.rs`/`requirement_compiler.rs`。
- `kg-hub/`：知识图谱中枢——`ingest.rs`/`ontology.rs`/`reason.rs`/`loop_engine.rs`/`govern.rs`（含 `api.rs`）。
- `operator-core/`：算子引擎内核——`engine.rs`/`conservation.rs`（守恒）/`monad.rs`/`resource.rs`/`state.rs`。
- `hermes-flow-bridge/`：外部流程集成桥——`bridge.rs`/`router.rs`/`integration/`（外部适配）。
- 其余：`business-catalog`（螺旋业务目录）、`template-market`（模板市场）、`optimizer`（优化器）、`operator-graph`/`operator-wasm`（算子图 / WASM 适配）。

### 前端（`frontend/src/`）

- `api/`：统一 fetcher（`index.js`，Axios 实例 + 请求/响应拦截器：注入 Bearer 令牌、剥离响应包裹、全局错误提示）。**组件禁止直接 `fetch`/`axios`**。
- `views/`：业务页面（按业务域组织，等价 feature-sliced 的 feature）。
- `components/`：通用 UI 组件，无业务逻辑，纯展示/交互。
- `router/`：vue-router 路由表。
- `styles/`：设计 token / 全局样式（禁止硬编码颜色尺寸）。
- `types.js`：前后端契约类型（FlowGraph / GovernanceReport / 算子 / 插件 等）。

## 编码规范

- 命名：Rust 遵循语言惯例（snake_case 函数/变量、PascalCase 类型、SCREAMING_SNAKE 常量，crate 名 kebab-case）；前端遵循项目既有（组件 PascalCase、组合式函数 camelCase 以 `use` 开头）。**不要擅自改写已有代码风格**。
- Rust 类型：优先具体类型；错误统一用 `anyhow::Result` 或领域 `Error`（`xuanji-system::error`）。生产路径禁止裸 `unwrap()`（测试可用）。
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
- 出码/发布必经：`/xuanji/publish` 必须先后经过 `xuanji-expert::verify`（⛨网关）与 `primiflow-fusion::full_gate`（治理 8 闸门），不得绕过。

## 禁止清单（Do NOT，最重要）

- 不要修改数据库 schema / 历史迁移（`xuanji-system/src/repo/schema.rs`、market 迁移文件），除非得到明确指令。
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
- 重点回归面：`xuanji-expert`（verify 5 检查）、`primiflow-fusion`（full_gate G1–G8）、`xuanji-system`（repo 多方言）。
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
- `cargo run -p runtime`：启动聚合后端（默认 `:3001`，含四套子服务；边缘入口 `backend/` 占 `:3000` 并反代 `/api` 至此）
- `cd frontend && npm run dev`：前端开发（Vite，`/api` 代理到 `:3000`）
- `cd frontend && npm run build`：前端生产构建 → `dist/`

### 构建 / 校验
- `cargo build --workspace`：全量构建（沙箱须 `run_in_background`）
- `cargo clippy --workspace --all-targets`：静态检查（目标零告警）
- `cargo test --workspace`：全量测试（背景运行，退出码 0 = 全绿）

### 测试（按 crate）
- `cargo test -p xuanji-expert`：验证网关与敏感拦截
- `cargo test -p primiflow-fusion`：治理 8 闸门
- `cargo test -p xuanji-system`：成员/任务/RBAC/审计 + 多方言 repo

## 上下文别名（项目黑话翻译）

- **璇玑 / ⛨验证网关**：`xuanji-expert::verify` 的五项数学/语义检查（S6/G2），出码前必过。
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
