# 联盟域接口—功能对账与细节修复（第 13 轮）

日期：2026-09-15
范围：`platform/domains/alliance` 13 个 crate
目标：回答"所有接口与功能是否一一对应、是否已对接"，并对发现的缺口做最小化修复

---

## 一、对账结论总览

| # | 类别 | 结论 | 状态 |
|---|------|------|------|
| 1 | `TaskScheduler` trait（9 方法）→ 实现 → HTTP 端点 | 一一对应 | 已对接 |
| 2 | `DagEngine` trait（9 方法）→ 实现 → HTTP 端点 | 一一对应 | 已对接 |
| 3 | `ExpertMatcher` trait（4 方法）→ 实现 → HTTP 端点 | 一一对应 | 已对接 |
| 4 | `ExecutorBridge` trait（6 方法）→ 4 套实现 | 一一对应 | 已对接 |
| 5 | 调度器 pause/cancel/resume → 执行器 | 经 Bridge 转发 | 已对接 |
| 6 | `DagEngine::get_fusion_output` 契约 | **缺失** | **已修复** |
| 7 | `FusionOutput` 类型归属 | **定义在实现层** | **已下沉到 proto** |
| 8 | scheduler-core 平行 DAG 引擎（1790 行） | **未接线** | 已标注，待决策 |
| 9 | scheduler-core `FusionOutput` 孤儿类型 | 无外部引用、缺 Serialize | 已标注 |

### 端到端调用链（已验证闭合）

```
用户 → scheduler-svc:3100
  POST /tasks            → TaskScheduler::submit_task
                          → ExpertMatcher::match_experts（含 infer_domains 自动识别）
                          → SimplePlanGenerator::generate
                          → ExecutorBridge::submit_plan ──HTTP──▶ executor-svc:3200
                                                                   POST /internal/executions
                                                                   → DagEngine::start_execution
  POST /tasks/:id        → pause/resume/cancel
                          → ExecutorBridge::{pause,resume,cancel}_task ──▶ executor-svc
  GET  /tasks/:id/nodes  ──proxy──▶ executor-svc GET /tasks/:id/nodes
  GET  /tasks/:id/result ──proxy──▶ executor-svc GET /tasks/:id/result
                                    → DagEngine::get_fusion_output
```

---

## 二、本轮修复

### 修复 1：`DagEngine` 契约缺 `get_fusion_output`（架构层缺陷）

**问题**
`/tasks/:task_id/result` 是执行器对外暴露的公共端点，但底层 `get_fusion_output`
只作为固有方法挂在具体类型 `DagEngineImpl` 上，**不在 `DagEngine` trait 内**。
后果：

- 持有 `Arc<dyn DagEngine>` 的调用方（如 `InProcessExecutorBridge`、测试替身、
  未来任何 trait 对象使用者）**无法**取得融合结论；
- 上层被迫依赖具体类型 `DagEngineImpl`，违反本域明示的 DIP 原则
  （`executor-proto` 模块文档首条即为"依赖倒置"）。

**修复**
把返回类型 `FusionOutput` 下沉到协议层，再把方法纳入契约：

| 文件 | 改动 |
|------|------|
| `proto/mox-alliance-executor-proto/src/dag_engine.rs` | 新增 `pub struct FusionOutput`（Serialize/Deserialize）；trait 增 `async fn get_fusion_output` |
| `proto/mox-alliance-executor-proto/src/lib.rs` | 重导出 `FusionOutput` |
| `core/mox-alliance-executor-core/src/fusion.rs` | 删除本地同名定义，改用协议层类型 |
| `core/mox-alliance-executor-core/src/dag_engine.rs` | 固有方法改 trait impl（转为 async） |
| `core/mox-alliance-executor-core/src/lib.rs` | 转出协议层 `FusionOutput` |
| `svc/mox-alliance-executor-svc/src/routes.rs` | 调用点补 `.await` |
| `svc/mox-alliance-scheduler-svc/tests/http_integration.rs` | 测试替身 `StubEngine` 补实现 |

**为什么必须下沉类型而不是只加方法**
trait 方法的返回类型若定义在实现层，协议层就无法声明该方法，
`Arc<dyn DagEngine>` 依旧取不到——两步必须一起做，缺一则契约不完整。

### 修复 2：scheduler-core 平行 DAG 引擎（1790 行）标注接线状态

**发现**
`scheduler-core` 内存在一套自包含的完整执行实现：

- `src/dag_engine.rs`（981 行，`DagExecutionEngine`）
- `src/fusion.rs`（809 行，`FusionEngine`/`FusionInput`/`FusionOutput`）

两者在 workspace 内**零生产调用方**（仅 `lib.rs` 重导出 + 自测），
与 `mox-alliance-executor-core` 的同名能力功能重叠。
而 `lib.rs` 模块文档把它们列为核心能力，造成"文档宣称有、实际没接"的误导。

**处置（不删，仅标注）**
在三个文件的模块文档顶部加显式"未接线（历史遗留）"说明：

- 说明生产链路一律走 `ExecutorBridge` 委派给 `executor-svc`；
- 提示 `scheduler-core::FusionOutput` 与协议层 `FusionOutput` 字段同名但类型不同，
  且未实现 `Serialize`，不可混用。

**未删除的理由**：删除 1790 行属不可逆破坏性改动，且该实现可能有历史保留意图。
去留应由产品/架构决策，本轮只消除误导。

---

## 三、验证

```
cargo check  —p 联盟域 13 crate --all-targets
  → 0 error / 0 warning

cargo test   —p 联盟域 13 crate
  → 309 passed / 0 failed / 5 ignored
```

编译期已捕获 trait 变更的全部影响面（`StubEngine` 缺实现 → 已补齐），
无遗漏的 `dyn DagEngine` 实现者。

---

## 四、遗留与待决策

| 项 | 说明 | 建议 |
|----|------|------|
| scheduler-core 平行 DAG 引擎 | 1790 行未接线 | 决策：删除 或 正式启用；已标注防误接 |
| scheduler-svc `proxy_to_executor` | 手搓 reqwest 代理，与 `HttpExecutorBridge` 各持一份 HTTP 客户端 | 职责不同（读转发 vs 控制通道），建议保留但统一超时/错误策略 |
| `mox-ai-expert-svc` 约 24 条 warning | 独立于本轮 | 单独一轮清理 |
| gateway 3 处未接线能力 | workspace 历史 / misc CRUD / task_type 映射 | 需产品决策 |

---

## 五、改动文件清单

```
platform/domains/alliance/proto/mox-alliance-executor-proto/src/dag_engine.rs   改
platform/domains/alliance/proto/mox-alliance-executor-proto/src/lib.rs          改
platform/domains/alliance/core/mox-alliance-executor-core/src/fusion.rs         改
platform/domains/alliance/core/mox-alliance-executor-core/src/dag_engine.rs     改
platform/domains/alliance/core/mox-alliance-executor-core/src/lib.rs            改
platform/domains/alliance/core/mox-alliance-scheduler-core/src/lib.rs           改（文档）
platform/domains/alliance/core/mox-alliance-scheduler-core/src/dag_engine.rs    改（文档）
platform/domains/alliance/core/mox-alliance-scheduler-core/src/fusion.rs        改（文档）
platform/domains/alliance/svc/mox-alliance-executor-svc/src/routes.rs           改
platform/domains/alliance/svc/mox-alliance-scheduler-svc/tests/http_integration.rs 改
```
