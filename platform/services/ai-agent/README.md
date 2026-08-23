# ai-agent · 璇玑对话驱动智能体

## §1 · 概述
璇玑 RelGraph 平台的 L4Services 级对话驱动开发大脑入口：承上接收前端聊天/工作台交互，承下调 LLM、流程图引擎、工作流引擎、多 Agent 编排器与浏览器自动化能力，把对话/需求归一化为可执行的系统蓝图与流程结果。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L4Services**（领域服务层，12 个 L4 crate 之一）。

```rust
pub const CRATE_ID: &str = "00374bdd-cc60-55bf-8970-a879afbfe443";
pub const ENGINE_NAME: &str = "xuanji::ai_agent";
pub const CRATE_META: xuanji_common_meta::CrateMeta = xuanji_common_meta::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: xuanji_common_meta::AisLayer::L4Services,
    owner: "xuanji-core",
};
```

## §3 · 模块结构 src/* 说明
| 文件 | 职责 |
|------|------|
| `src/lib.rs` | 三常量声明 + 所有子模块 glob 再导出（下游一行 `use ai_agent::*`） |
| `src/conversation.rs` | 对话引擎：会话管理 + 消息历史 + LLM 路由链（真实→规则→离线三级降级） |
| `src/algorithm.rs` | 算法分析器：CEM 多目标 / 意图识别 / 分类辅助 |
| `src/provider.rs` | `trait LLMProvider` + `trait HttpProvider` 接口抽象；多 Provider 注册表 |
| `src/llm_client.rs` | HTTP LLM 客户端（OpenAI/DeepSeek/DashScope 兼容） |
| `src/browser_automation.rs` | 浏览器自动化：URL 解析、动作序列、会话沙箱、结果回传 |
| `src/requirement_compiler.rs` | 需求编译器：自然语言 → `SystemBlueprint`（规则 + LLM 双通道） |
| `src/flow_engine.rs` | 流程图 CRUD + 执行：7 类节点（Start/End/Condition/LLM/Browser/HttpRequest/Template） |
| `src/workflow_engine.rs` | BPMN 风格工作流：串行/并行/分支、变量模板 `${k}` 桥接 |
| `src/dialogue_graph.rs` | 对话→知识图谱写入器：会话/消息/意图节点落 kg-hub 关图闭环 |
| `src/engine/` (5 files) | 单 Agent 引擎：`state_machine.rs` PERCEIVE→PLAN→ACT→OBSERVE→REFLECT；`multi_agent.rs` 多 Agent 编排；`guards.rs` 护栏族 trait；`tools.rs` AgentTool trait；`engine_loop.rs` 主循环 |
| `src/plugin_bus.rs` / `resource_manager.rs` / `types.rs` / `util.rs` | 插件总线、资源容器、共享类型、通用工具函数 |
| `tests/caomei_e2e.rs` | 草莓流程端到端测试（会话→流程→节点→关图全链路） |

## §4 · 关键 Trait & Impl
- **`pub trait LLMProvider`**（`src/provider.rs`）：定义 `async fn chat(&self, req) -> Result<String>`，下游按 Provider 枚举实现 DeepSeek/DashScope/Fallback。
- **`pub trait Guard`**（`src/engine/guards.rs`）：Agent 执行前/后护栏（输入安全检查 / 资源超限 / 输出判重）。
- **`pub struct AIAgent`**：主结构体 13 个 Arc 子系统；`impl AIAgent { chat, configure_llm, compile_requirement_with_llm, create_flow/execute_flow, run_engine_task, spawn_agent, agent_communicate }` 统一对外 API。
- **`ConversationEngine / BrowserAutomationEngine / WorkflowEngine`**：三个子引擎各自独立状态机，AIAgent 调度。

## §5 · 跑单测指引
```bash
cargo test -p ai-agent          # 单元 + 集成
cargo test -p ai-agent caomei_e2e   # 只跑草莓全链路回归
```
断言覆盖：LLM 三级路由（真实/规则/降级模拟）、流程图 7 类节点 `validate_flow`、需求编译规则抽取、多 Agent 三类编排（并行/顺序/通信）、对话→关图幂等写入。

## §6 · 二次开发 / DIP 反转指引
- **新增 LLM Provider**：实现 `trait LLMProvider` → 在 `src/provider.rs` 注册表 `ProviderRegistry::register(Box<dyn LLMProvider>)` 注入（禁止直接改 `llm_client.rs` 硬编码判断）。
- **新增 AgentTool**：实现 `trait AgentTool` → 在 `engine::tools::ToolRegistry` 注册。
- **新增 Guard**：实现 `trait Guard` → 挂入 `engine_loop` 前/后钩子数组（DIP 依赖倒置，不用修改 engine_loop.rs 主体）。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**标准流程**：① 先在 `tests/` 新增失败 RED（如某 Provider 超时重试、某 Workflow 分支变量替换）；② 最小实现（thin wrapper 优先）；③ `cargo test -p ai-agent` 全量 GREEN。
**精度护栏**：`apply_template` 变量替换 SSoT 在 `src/flow_engine.rs`，Workflow 的 `${k}` wrapper 必须 ≤4 行有效代码，禁止独立实现循环/递归（由 `validate_no_duplicate_functions.js` 看门狗拦截）。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-ai-agent
engine id      : engine-rust-ai-agent
code_graph unit: ai-agent
```
`self_sync_rust.js` 变更触发：改 `src/lib.rs` 顶部三常量 / 新增 `pub struct` 或 `pub fn` → 运行 self_sync 自动刷新 atlas_auto_registry 的 keyFeatures、code_graph_bindings 的 topEntities。
