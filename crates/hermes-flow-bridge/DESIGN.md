# hermes-flow-bridge · 设计文档（路线 X：零侵入插件注入）

> 设计原则（用户指令）：先设计、再开发，跟文档一步步落实。所有操作明确、所有代码明确。
> 目标：把已验证的 `flow-ai`（流程图优化内核）+ `expert-alliance`（七专家+治理+算法验证网关）
> 注入到 **Hermes Agent Ultra**（Nous Research 的 Rust 重写版，MIT fork，84k 行 / 23 crate），
> 作为「流程图统一需求源 + 关系网调度先验」，全程**零侵入**改写 Hermes 内核。

---

## 一、为什么是路线 X（零侵入）

- Hermes `hermes-agent` 是 84k 行工业级代码；本地 `runtime`/`ai-agent` 已有 34 个预存编译错误。
  改写它的 `agent_loop.rs` 主循环（路线 Y）编译风险高、回归面大。
- Hermes 自带 **plugin 中间件系统**（`crates/hermes-agent/src/plugins.rs`），提供：
  - `ToolRequestMiddleware`：工具执行**前**截获 `tool_name` + `args`，可改写参数或加来源注解。
  - `ToolExecutionMiddleware`：工具执行**时**包装，可拦截/改写结果。
  - `Plugin` trait + `plugin.yaml`：声明式注册，无需改主循环。
- 因此「流程拓扑调度器」以 **plugin 身份**注入，Hermes 主循环完全不动。

## 二、真实接入点（已查 Hermes 源码，签名精确）

```rust
// crates/hermes-agent/src/plugins.rs （节选，真实类型）
pub struct ToolRequestMiddlewareContext {
    pub tool_name: String,
    pub args: Value,            // 可改写
    pub original_args: Value,
    pub turn: u32,
}
pub struct ToolRequestMiddlewareUpdate {
    pub args: Value,
    pub source: Option<String>,   // 标记"来自流程图复用模板"
    pub reason: Option<String>,
}
pub type ToolRequestMiddleware =
    Arc<dyn Fn(&ToolRequestMiddlewareContext) -> Option<ToolRequestMiddlewareUpdate> + Send + Sync>;

pub struct ToolExecutionMiddlewareContext {
    pub tool_name: String,
    pub tool_call_id: String,
    pub args: Value,
    pub original_args: Value,
    pub turn: u32,
}
pub type ToolExecutionMiddleware = Arc<
    dyn Fn(&ToolExecutionMiddlewareContext, &mut dyn FnMut(Option<Value>) -> ToolResult) -> ToolResult
        + Send + Sync>;

pub trait Plugin: Send + Sync {
    fn register(&self, ctx: &mut PluginContext);  // ctx.on_tool_request(mw); ctx.on_tool_execution(mw);
}
```

**关键约束**：两个中间件都是**同步闭包**。不能在闭包内跑 `alliance_optimize`（async + 重计算），
也不能阻塞 agent loop。→ 解法：优化内核作为**独立常驻服务**（`expert-alliance` 已有 HTTP 服务），
插件只发**轻量请求**（跨进程 channel / 本地 HTTP `127.0.0.1:8080/api/optimize`），
重计算在异步侧完成，中间件拿到结果后只做**注解 / 路由 / 拦截标记**。

## 三、架构总图（全链路）

```
┌────────────────────────────────────────────────────────────────┐
│ Hermes Agent（宿主，零改动）                                       │
│   AgentLoop(ReAct) ──► [ToolRequestMiddleware] ──► 工具执行        │
│                        [ToolExecutionMiddleware]                │
└──────────┬───────────────────────┬─────────────────────────────┘
           │ 同步闭包只做轻量动作    │
           ▼                        ▼
┌──────────────────────┐  ┌──────────────────────────────────────┐
│ hermes-flow-bridge   │  │ expert-alliance 常驻服务(已验证)         │
│ (新增, 薄适配 crate)  │  │  POST /api/optimize → VizBundle        │
│  ├ normalize.rs      │  │   ├ flow-ai: 并行/CPM/RCPSP/Dijkstra    │
│  │  ToolCall↔FlowNode │  │   ├ expert-alliance: 七专家+治理        │
│  ├ recorder.rs       │──┤   └ verify(): 算法否决(最高权限)         │
│  │  累积会话流程图     │  │                                        │
│  ├ router.rs         │  │  GET  /api/health                       │
│  │  复用最短路径点亮   │  │  POST /api/verify (CLI 同款)            │
│  └ plugin.rs         │  └──────────────────────────────────────┘
│     实现 Plugin trait │
└──────────────────────┘
```

## 四、职责划分（全代码明确）

### 4.1 `normalize.rs` — Hermes ToolCall ↔ flow_ai::FlowNode
- `fn to_flow_node(call: &ToolRequestMiddlewareContext) -> FlowNode`
  - `node.id = call.tool_name + "#" + call.turn`（同工具多次调用区分）
  - `node.kind = NodeKind::Tool`
  - `node.tool = ToolKind::from_str(&call.tool_name)`（映射表：file→File, browser→Browser, db→Database, …）
  - `node.duration_ms`：从 `hermes-intelligence` 的历史统计或默认估值
  - `node.tags`：从 `call.args` 提取维度（如含 `pii`/`desensitize` 标记）
- `fn dependency_edges(prev: &[FlowNode], cur: &FlowNode) -> Vec<FlowEdge>`
  - 同回合并发工具 → 无依赖边（并行）
  - 跨回合且 `cur.args` 引用 `prev` 输出字段 → 加 `FlowEdge::seq`（数据依赖）

### 4.2 `recorder.rs` — 跨回合累积会话流程图
- `pub struct SessionFlow { graph: FlowGraph, order: Vec<String> }`
- 线程安全：`Arc<Mutex<HashMap<session_id, SessionFlow>>>`（插件持有单一全局实例）
- 每次 `ToolRequestMiddleware` 触发 → `recorder.record(session_id, node, edges)`
- 工具执行完成（或回合结束）→ 触发**异步** `optimize` 请求（spawn 后台，不阻塞中间件）

### 4.3 `router.rs` — 复用最短路径（跳过完整 ReAct）
- 调用 `expert-alliance` 的 `TopologyGraph::route(query, threshold)` 做 fast-path 点亮
- 命中历史模板 → 在 `ToolRequestMiddlewareUpdate.source` 写 `"flow-template:<id>"`
  Hermes 上游读到该 source 注解即可走轻量执行、跳过完整推理（由 Hermes 侧读取注解实现；bridge 只负责标注）

### 4.4 `plugin.rs` — 实现 Hermes `Plugin` trait
```rust
pub struct FlowBridgePlugin { state: Arc<BridgeState> }
impl Plugin for FlowBridgePlugin {
    fn register(&self, ctx: &mut PluginContext) {
        let st = self.state.clone();
        ctx.on_tool_request(Arc::new(move |c: &ToolRequestMiddlewareContext| {
            // 1) 累积到会话流程图
            let node = normalize::to_flow_node(c);
            st.recorder.lock().record(c, &node);
            // 2) 轻量复用路由（同步，仅查本地缓存/最短路径表）
            if let Some(tpl) = st.router.match_template(&c.tool_name, &c.args) {
                return Some(ToolRequestMiddlewareUpdate {
                    args: c.args.clone(),
                    source: Some(format!("flow-template:{}", tpl)),
                    reason: Some("命中流程图复用模板，跳过完整 ReAct".into()),
                });
            }
            None // 不拦截，交还 Hermes 正常执行
        }));
        // 异步侧：另起后台任务把累积图推给 expert-alliance 服务做 optimize+verify
    }
}
```
- **算法否决拦截**：`ToolExecutionMiddleware` 中，若 `expert-alliance` 服务的 `verify` 结论为
  `algo.vetoed==true`（通过本地共享状态读取，避免同步内网络调用），则：
  ```rust
  if st.gate.vetoed() {
      return ToolResult::error("璇玑验证否决：优化破坏语义依赖，已拦截", None);
  }
  ```
  （veto 状态由后台 optimize 任务写入 `Arc<Mutex<GateState>>`，中间件只同步读）

### 4.5 `plugin.yaml` — 声明式注册
```yaml
name: flow-bridge
version: 0.1.0
description: "注入 flow-ai + expert-alliance 流程图/关系网优化内核（璇玑验证）"
author: algorithm-alliance
kind: plugin
dependencies:
  - expert-alliance-server   # 独立常驻服务
```

## 五、数据流（一次完整调用）

1. 用户语音 → Hermes 分类（Hermes 原生）；走对应子流程。
2. 子流程触发工具 `browser.scrape`：
   - `ToolRequestMiddleware` 触发 → `recorder` 把节点加入会话流程图；`router` 查复用模板。
   - 若是政务脱敏场景历史模板命中 → `update.source="flow-template:gov-pii"`，上游走轻量路径。
3. 回合结束 → 后台 spawn `optimize(session_graph)` → `expert-alliance` 服务返回 `VizBundle`：
   - 并行调度（web1/web2 并行）、CPM 关键路径、冲突拓扑（浏览器互斥=1）、`verify()` 验证。
4. 若 `verify.vetoed==true`（理论上不会发生，因 optimize 已保证语义守恒，仅作最高权限兜底）→
   `ToolExecutionMiddleware` 直接拦截该工具执行，返回算法否决错误，审计链记录 `algorithm_veto`。
5. `alliance.html`（复用 expert-alliance 的 Three.js 面板）实时高亮：关键路径金黄、冲突标红、
   复用路径青色点亮、算法验证卡片显示「✔ 全部通过」。

## 六、兼容第三方插件（MCP / Skills / Loops / 多模型）

- **MCP**：Hermes `hermes-mcp` 把第三方工具注册为 `ToolHandler`；bridge 的 `normalize` 把这些工具
  统一映射为 `flow_ai::ToolKind::External` 节点 —— **任何 MCP 工具自动成为流程图节点**，零额外改造。
- **Skills**：`hermes-skills` 提炼的技能写入 `expert-alliance::TopologyGraph`（六维关系网 `Skill` 实体），
  `router` 的最短路径检索即"技能复用"。
- **Loops**：Hermes 子 Agent 委托（`sub_agent_orchestrator`）产生的循环，在 `recorder` 中识别为
  `FlowEdge` 回边，`flow-ai` 的 `critpath` 识别循环/异常/事务；逆向 `codegen` 可还原为 Python `while/for`。
- **多模型**：`expert-alliance` 的 `ModelRouting`（Light/Standard/Heavy）直接复用 Hermes
  `smart_model_routing` 的逐轮 cheap-route 结论，算力分配一致。

## 七、开发步骤（跟文档逐步，先骨架后接入）

- [x] **Step 1** 新建 crate `hermes-flow-bridge` + `Cargo.toml`（依赖 flow-ai / expert-alliance / serde_json / tokio）
- [x] **Step 2** `normalize.rs`：ToolCall↔FlowNode 映射（4 单测通过）
- [x] **Step 3** `recorder.rs`：会话流程图累积（Arc<Mutex<HashMap<..>>>，2 单测通过）
- [x] **Step 4** `router.rs`：复用模板匹配（本地缓存版最短路径，2 单测通过）
- [x] **Step 5** `plugin.rs`：实现 Hermes `Plugin` trait + `ToolRequestMiddleware`/`ToolExecutionMiddleware`（含算法否决拦截，2 单测）
- [x] **Step 6** `plugin.yaml`：声明式注册
- [x] **Step 7** `bridge.rs`：后台 optimize 推送 + `verify` 否决拦截（直接复用 `expert_alliance::alliance_optimize`，1 单测）
- [x] **Step 8** 编译 `hermes-flow-bridge`（独立 + `live` feature，0 warning，0 error）+ 单测 14 通过
- [x] **Step 9** 真实 Hermes 适配重构：抽 `hooks.rs`(框架无关单一事实源) + `state.rs`(BridgeState/GateState) + `integration/hermes_shim.rs`(feature=`hermes` 门控，对照真实 `plugins.rs` 字段写的适配模板，标注 3 处集成缝) + `plugin.rs` 重写复用 hooks。bridge 不依赖 Hermes 具体类型编译，做到零侵入；用户侧启用 `hermes` feature 并改 `Cargo.toml` 中 `hermes-agent` 注释依赖为真实 path 即联调。
- [x] **Step 10** 前端实时联动：expert-alliance `server.rs` 新增 `POST /api/ingest` + `GET /api/live`（对实时会话图跑 `alliance_optimize` 返回带高亮 VizBundle）；`frontend/alliance.html` 新增「实时联动」按钮（1.5s 轮询 `/api/live`）。已实测：ingest 接受 FlowGraph → live 返回 gate=Approved、算法验证 5 项、7 专家评分、关键路径/冲突高亮。
- [x] **Step 11** 会话级端到端集成测试 `tests/session_e2e.rs`（6 用例）+ 闭环演示 bin `src/bin/bridge_demo.rs`：模拟 Hermes 多轮工具调用 → 录制 FlowGraph → 复用路由命中 → 后台 `alliance_optimize` → 算法网关 → 否决拦截接线。`cargo run -p hermes-flow-bridge --bin bridge_demo` 实测：speedup 2.60×（省 61.5%），复用模板命中，否决位可强制阻断。
- [x] **Step 12** 「LLM 调用减半」量化原型：`src/mini_hermes.rs`（可单测 mini agent-loop + `LlmTracer` 原子计数）+ `router.match_prefix`（模板是 recent 前缀则回放）。`bridge_demo` 阶段 5 实测：baseline linear ReAct = 4 次 LLM，bridge 复用回放 = 0 次（已知流程 100% 削减）；部分已知流程尾部未知段按步计 LLM。证明用户原方案核心收益「LLM 调用次数减半」可运行、可复现。新增 3 单测全部通过。

## 十、Step 9/10 实测结论（2026-08-02）
- `cargo test -p hermes-flow-bridge`（default + `live`）：14 passed，0 warning，0 error。
- `cargo test -p expert-alliance`：23 lib + 5 integration 通过；`cargo test -p flow-ai`：50 + 1 通过。
- 联盟服务 `alliance serve --port 3079` 实测：
  - `POST /api/ingest` 接受 `{id,name,nodes[{id,name,kind,tool,accesses[{resource,mode}]}],edges[{from,to,kind}]}` → ok
  - `GET /api/live` 跑完整 `alliance_optimize`：gate=Approved，algorithm.vetoed=false（topology/data_dep/conflict 三项阻断检查全过，code_rt 仅非阻断告警），critical_path=2 节点，conflicts=0 阻断，expert_scores 七专家全 1.0/0.8。
- 融合架构闭环：Hermes 工具调用 → bridge 录制为 FlowGraph → 后台调 `alliance_optimize` → `verify()` 算法网关 → 否决时 `ToolExecutionMiddleware` 强制拦截。

## 八、优先级铁律（与 expert-alliance 一致）
```
算法验证网关(数学正确性) > 权限专家 > 安全专家 > 其他专家 > Hermes 原生路由
```
即便 Hermes 原生路由放行，bridge 的 `ToolExecutionMiddleware` 在 `algo.vetoed` 时强制拦截。

## 九、预期收益（融合后）
- Hermes 线性 ReAct → 「图谱优先、流程约束、推理兜底」三层新架构（与用户原方案一致）
- 历史流程复用走最短路径，LLM 调用减半；简单任务分流轻量模型，算力降 35–60%
- 冲突拓扑检测（浏览器互斥/事务/文件锁/脱敏）在工具执行前拦截
- 流程图+关系网为统一需求源，改一处同步代码/记忆/Skill/规则，无数据孤岛
- 全算法经 `verify()` 数学验证，可 CI 证明，任何优化破坏语义即否决
