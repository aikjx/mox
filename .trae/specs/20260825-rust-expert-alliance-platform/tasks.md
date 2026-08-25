# 开发专家联盟全维分析 · Rust企业级Platform集成（Tasks）

> **Spec 路径**：`d:\a10\aikjx\gitcode\infotopograph\.trae\specs\20260825-rust-expert-alliance-platform\spec.md`  
> **自然语言**：中文  
> **说明**：按依赖拓扑排序：T1-T5（Rust 核心 xuanji-expert）→ T6-T9（网关 runtime 路由/代理）→ T10-T14（前端 ChatView/MessageBubble/API）→ T15（语音服务集成）→ T16（冒烟+E2E+独立评审）。所有 Rust 新代码必须先写 `#[test]` 再写实现（TDD，避免 HC 退化）。

---

## 任务依赖总览（DAG）

```
T1(P0) alliance 模块骨架 + 常量（HC-2/5/8 参数写死）
  │
  ├─ T2(P0) 意图识别(关键词+spread激活扩散+RRF) —— 依赖 graph-algorithms HC-2
  ├─ T3(P0) 专家组队(14维矩阵+EAF 4.2安全替换)
  ├─ T4(P0) 并行咨询(rayon+60s超时) + 辩论合成(共识≥0.6跳过)
  │
  └─ T5(P0) 质量门禁(A/B/C/D+HC-8公式) + 指标学习(PluginRegistry) + 审计7事件
       │
       ├─ T6(P0) runtime 路由挂载：/ai/engine/alliance/{full,capabilities,report/:id}
       ├─ T7(P0) runtime 语音代理：/voice/* → 3717（degraded 降级链）
       ├─ T8(P0) subservers.rs 注册 AllianceSubserver + VoiceSubserver
       └─ T9(P0) /ai/engine/metrics 扩展：alliance_gate_dist + 阶段延迟直方图
            │
            ├─ T10(P0) frontend API 层：allianceApi + voiceApi 封装
            ├─ T11(P0) ChatView：全维分析 SSE + 5 Chip 流转 + 报告卡片
            ├─ T12(P0) ChatView：麦克风录音按钮 + 小白语音开关 UI
            ├─ T13(P0) MessageBubble 朗读三层回退(TTS引擎→浏览器→禁用)
            └─ T14(P0) Vite dev 代理 /voice/* + /api 统一前缀
                 │
                 └─ T15(P0) xiaobai_voice 默认 TTS=cosyvoice2 + health 端点修正
                      │
                      └─ T16(P0) 全链路冒烟：Rust 测试 + DOM 探针 + Playwright + 独立评审
```

---

## Task 1：Rust 核心 —— `alliance` 模块骨架与 HC 常量锁死

- **Status**: pending
- **Priority**: high
- **Depends On**: （无，必须首个完成）
- **对应 AC**: AC-07, AC-08, AC-09, AC-18, AC-R05
- **覆盖 FR**: FR-CORE-01（部分）, HC-2/HC-8 硬约束常量定义
- **产出文件**:
  - 新增 `platform/services/xuanji-expert/src/alliance/mod.rs`（入口）
  - 新增 `platform/services/xuanji-expert/src/alliance/constants.rs`（HC 锁死常量）
  - 修改 `platform/services/xuanji-expert/src/lib.rs`（pub mod alliance）
- **实现要求（严格）**:
  1. `constants.rs` 必须用 `pub const` 锁死以下值（**禁止配置化**，违反 = HC Blocked）：
     - `SPREAD_METHOD: &str = "spread"`
     - `SPREAD_DAMPING: f64 = 0.85`（HC-2）
     - `SPREAD_ROUNDS: u32 = 30`（HC-2）
     - `RRF_K: u32 = 60`（HC-8 家族）
     - `SPREAD_WEIGHT: f64 = 0.7`
     - `GATE_THRESHOLD_A: f64 = 0.90`
     - `GATE_THRESHOLD_B: f64 = 0.80`
     - `GATE_THRESHOLD_C: f64 = 0.70`
     - `DEBATE_MAX_TOKENS_PER_ROUND: usize = 900`（EAF 4.3）
     - `EXPERT_TIMEOUT_SECS: u64 = 60`（EAF 4.3 超时隔离）
     - `QUALITY_FORMULA: &str = "0.55×Quality + 0.20×Speed + 0.10×TokenEfficiency + 0.15×Stability"`
     - `INTENT_CLASSES: [&str; 7] = ["math", "logic", "knowledge", "code", "chinese", "timeliness", "instruction"]`（HC-9 7 类基准）
  2. `mod.rs` 定义阶段枚举 `AlliancePhase { Intent, Team, Debate, Synthesize, Gate, Learn, Done }` 与事件类型 `AllianceEvent { phase: AlliancePhase, payload: Value, trace_id: Uuid, latency_ms: u64 }`。
  3. `mod.rs` 定义 `AllianceEngine` struct：`pub async fn run_full_analysis(&self, req: AllianceRequest) -> impl Stream<Item = AllianceEvent>`（先占位返回空 SSE，后续 Task2-5 填充）。
  4. `lib.rs` 追加 `pub mod alliance;`。

### TR（本任务自验）

| TR | 类型 | 条件 | 证据来源 |
|---|---|---|---|
| TR-T1-01 | rule | constants.rs 中以上 12 个 const 全部存在且值完全一致 | rg `SPREAD_DAMPING\|GATE_THRESHOLD\|RRF_K\|INTENT_CLASSES` |
| TR-T1-02 | rule | `cargo test -p xuanji-expert --lib alliance::constants --nocapture` 通过（写 1 个断言测试确认硬编码值） | cargo test 输出 |
| TR-T1-03 | rule | `grep -rn "unsafe " platform/services/xuanji-expert/src/alliance/` = 0（AC-18） | grep |
| TR-T1-04 | rubric | pub fn 文档注释率：AC-R05（2=100%，1=≥90%） | 人工审阅 constants.rs/mod.rs 的 pub 项 |

---

## Task 2：Rust 核心 —— 意图识别（双路：关键词 ms 级 + HC-2 激活扩散 + RRF 融合）

- **Status**: pending
- **Priority**: high
- **Depends On**: T1
- **对应 AC**: AC-02, AC-07, AC-08, AC-09
- **覆盖 FR**: FR-CORE-02, HC-2, HC-8 家族, HC-9
- **产出文件**:
  - 新增 `platform/services/xuanji-expert/src/alliance/intent.rs`
  - 修改 `platform/services/xuanji-expert/Cargo.toml`（若需 graph-algorithms feature 激活扩散公开接口）
- **实现要求**:
  1. `fn classify_intent(query: &str, ctx: &ExpertContext) -> IntentResult`：
     - 第一步关键词匹配（ms 级）：对 7 类做正则命中，返回 `keyword_scores: BTreeMap<&str, f64>`。
     - 第二步激活扩散（HC-2 spread）：若 `ctx.graph().is_some()`，把 query 切词作为种子向量，调 `graph_algorithms::activate_spread(graph, seeds, d=0.85, rounds=30)`，返回 `spread_scores: BTreeMap<NodeId, f64>` → 映射到 7 类的 `spread_class_scores`。
     - 第三步 RRF 融合 k=60, spread_weight=0.7：`final_score = (1-sw) * rrf(keyword_ranks, k=60) + sw * rrf(spread_ranks, k=60)`。
     - 若 graph 不可用：降级 spread_scores=空，最终结果等同纯关键词，但必须标记 `degraded=true + degrade_reason="spread_graph_unavailable"`。
  2. 输出 `IntentResult { intent_id, conf, keyword_scores, spread_scores, rrf_scores, degraded, seeds_hit, trace_log }`，trace_log 必须包含字符串 `method=spread, d=0.85, rounds=30`（AC-07）和 `rrf_k=60, spread_weight=0.7`（AC-08）。
  3. TDD 先写 3 个测试：
     - `test_intent_code_query`：输入"帮我写一个 Rust 函数做冒泡排序"→ intent=code, conf≥0.7。
     - `test_intent_degraded`：graph=None 时 degraded=true 不 panic，intent 仍可给出。
     - `test_intent_7_classes`：分别给 7 类典型 query，7 类均有过一次被命中（保证 HC-9 全覆盖）。

### TR

| TR | 类型 | 条件 | 证据来源 |
|---|---|---|---|
| TR-T2-01 | rule | 3 个 TDD 测试全通过（cargo test intent_） | cargo test |
| TR-T2-02 | rule | IntentResult.trace_log 中同时含 HC-2 + HC-8 参数字符串（AC-07/08） | grep trace_log 输出 |
| TR-T2-03 | rule | degraded=true 模式下，函数 0 panic，0 unwrap()（用 assert_no_panic 或 1000 次循环调用） | 压力测试小用例 |

---

## Task 3：Rust 核心 —— 专家组队（14 维矩阵 + EAF 4.2 安全类强制替换）

- **Status**: pending
- **Priority**: high
- **Depends On**: T1, T2
- **对应 AC**: AC-02, AC-04
- **覆盖 FR**: FR-CORE-03
- **产出文件**: 新增 `platform/services/xuanji-expert/src/alliance/team.rs`
- **实现要求**:
  1. 注册表：`build_expert_registry() -> BTreeMap<ExpertId, ExpertMeta>`，其中 ExpertMeta={ dimension: Dimension, supported_classes: [7 类子集], avg_latency_ms, gate_A_rate_30d, priority = dim_priority(dimension) }，14 位专家全部注册（Permission + Security + Resource + Data + Algorithm + Business + Observability + Architecture + SecurityCode + CodeQuality + Performance + Testing + Documentation + Maintainability）。
  2. 组队算法 `optimize_team(intent: &IntentResult, size: usize = 4) -> TeamResult`：
     - 第一步：按 `supported_classes` 命中 intent 打匹配分 × `gate_A_rate_30d` × priority 权重排序。
     - 第二步：**EAF 4.2 安全类强制替换**：若 intent 命中 security/permission/chinese-sensitivity，且候选队末位不是 Security/Permission 专家，强制替换（即使末位分更高）。
     - 第三步：去重（同 dimension 仅保留 Top1），保证队 ≤ size 位。
  3. 输出 `TeamResult { team_ids[], forced_replacements[], reasoning_matrix }`。
  4. TDD 先写：`test_team_security_replace`：构造敏感 query→强制 Security 入队；`test_team_size_four`：队大小 3~5 且无同维度重复。

### TR

| TR | 类型 | 条件 | 证据来源 |
|---|---|---|---|
| TR-T3-01 | rule | 2 TDD tests 通过 | cargo test team_ |
| TR-T3-02 | rule | `/ai/engine/alliance/capabilities` 返回的专家条目数 = 14（AC-04 扩展） | 之后在 T6 验证；本任务先测 registry 长度 14 |

---

## Task 4：Rust 核心 —— 并行咨询 + 辩论合成（EAF 4.3 + 4.4）

- **Status**: pending
- **Priority**: high
- **Depends On**: T1-T3
- **对应 AC**: AC-02, AC-06
- **覆盖 FR**: FR-CORE-04, FR-CORE-05
- **产出文件**:
  - 新增 `platform/services/xuanji-expert/src/alliance/debate.rs`
- **实现要求**:
  1. `async fn parallel_consult(ctx: &ExpertContext, team: &[Box<dyn Expert>]) -> Vec<ExpertOpinion>`：
     - 内部 `expert::dispatch(ctx, team)`（rayon 并行已存在）。
     - 包裹 `tokio::time::timeout(Duration::from_secs(EXPERT_TIMEOUT_SECS=60))`；超时专家的 Opinion 返回 `skipped=true skip_reason="timeout_60s"`。
  2. `fn synthesize(opinions: Vec<ExpertOpinion>, enable_llm_debate: bool) -> SynthesisResult`：
     - 共识度：对同 dimension 的约束/风险计算 Jaccard 相似度；若 consensus≥0.6，跳过辩论直接合成结构化结果（EAF 4.4）。
     - 若 consensus<0.6：若 enable_llm_debate 为 true（默认 false，Q2 暂定方案），降级占位标记 `debate: {mode: "llm_placeholder"}`；否则走"维度加权投票"（按 dim_priority × score 加权）。
     - 900 tok/轮上限：若未来启用 LLM 辩论必须裁剪 tokens≤900/轮。
  3. 输出 `SynthesisResult { consensus_score, divergence_list[{dimension, expert_a_op, expert_b_op}], final_plan: ReconciledPlan, debate_mode }`。
  4. TDD：`test_consult_timeout_fallback`（模拟超时，断言 skipped）；`test_synthesize_high_consensus_skip`（构造高共识 opinions→debate 被跳过）。

### TR

| TR | 类型 | 条件 | 证据来源 |
|---|---|---|---|
| TR-T4-01 | rule | 2 TDD tests 通过 | cargo test debate_ |
| TR-T4-02 | rule | synthesize 的 consensus_score 计算严格在 0..1 区间，无 NaN | 单元断言 assert!(!consensus.is_nan()) |

---

## Task 5：Rust 核心 —— 质量门禁（HC-8 公式）+ 指标学习 + 7 阶段审计事件

- **Status**: pending
- **Priority**: high
- **Depends On**: T1-T4
- **对应 AC**: AC-02, AC-06, AC-09, AC-16, AC-R01
- **覆盖 FR**: FR-CORE-06, FR-CORE-07, FR-CORE-08, FR-CORE-09
- **产出文件**:
  - 新增 `platform/services/xuanji-expert/src/alliance/gate.rs`
  - 新增 `platform/services/xuanji-expert/src/alliance/learn.rs`
  - 新增 `platform/services/xuanji-expert/src/alliance/pipeline.rs`（把 T1-T4+本任务串成全 6 阶段管线，输出完整 SSE Stream）
- **实现要求**:
  1. `gate.rs` 质量门禁（FR-CORE-06）：
     - `fn quality_gate(syn: &SynthesisResult, ctx: &ExpertContext) -> GateResult`：
       - 计算 `Quality = consensus + avg_score/2`；`Speed = 1 - normalize(total_phase_latency_ms, 0..5000)`；`TokenEfficiency = 1 - normalize(tokens_used_if_any, 0..9000)`；`Stability = 1 - normalize(variance_of_3_runs_if_available, 0..1)`。
       - 综合分 = `0.55*Q + 0.20*S + 0.10*T + 0.15*Stab`（HC-8，四舍五入到 6 位小数）。
       - 按阈值 A≥0.9 / B≥0.8 / C≥0.7 / D<0.7 给等级。
       - C 级重跑：本函数若入参 `retry_on_c=true` 且等级=C，返回 `retried=true`，内部换策略（如组队时 size=5 vs size=4）重跑 T2-T4，取两次更优。
       - **输出解释必须包含 QUALITY_FORMULA 原文**（AC-09：`"0.55×Quality + 0.20×Speed + 0.10×TokenEfficiency + 0.15×Stability"`）。
  2. `learn.rs` 指标学习（FR-CORE-07）：
     - 若 `gate_level ∈ {A, B} 且 consensus ≥ 0.95`，把 `(query_pattern, intent, team, plan_hash)` 写入 `HarnessCtx` 的 skill_registry（复用 harness.rs 已实现 PluginRegistry）。
     - 写入语义缓存：`(intent_hash, trimmed_answer)` → 下次同 hash 直接命中。
     - 7 类基准都至少有一个默认 seeded skill（空分析模式可 0 AI 运行）。
  3. `pipeline.rs` 把 6 阶段串成完整管线（FR-CORE-01 落地）：
     - `run_full_analysis` 的 SSE Stream：按序 emit phase=intent→team→debate→synthesize→gate→learn→done，每次附 latency_ms + trace_id。
     - 每个 phase emit 前调 `audit::emit_event("alliance.0{1..7}.{phase_name}", payload_signed_with_hmac)`（审计 7 条独立事件 FR-CORE-09）。
     - 最终 phase=done 返回 `{report: ReconcileReport, gate: GateResult, trace_id}`。
  4. TDD 先写：`test_gate_C_retries`（构造 C 级入参，断言 retried=true）；`test_audit_7_events`（跑 1 次管线 → 审计接收器收到 7 条事件）；`test_formula_in_explanation`（grep explanation 含 HC-8 公式原文）。

### TR

| TR | 类型 | 条件 | 证据来源 |
|---|---|---|---|
| TR-T5-01 | rule | 3 个 TDD tests + pipeline 集成 test 全部通过 | cargo test alliance_ --nocapture |
| TR-T5-02 | rule | GateResult.explanation 字符串严格含 HC-8 公式原文（AC-09） | test 断言 contains |
| TR-T5-03 | rule | 审计事件 7 条齐全（alliance.01 ~ alliance.07，AC-16） | MockAuditSink 7 条命中 |
| TR-T5-04 | rubric | 报告结构：AC-R01（2=14维+公式+Mermaid+导出齐全；阈值≥1） | 人工审阅 ReconcileReport 结构体输出 JSON |

---

## Task 6：Platform 网关 —— 路由挂载 /ai/engine/alliance/{full,capabilities,report}

- **Status**: pending
- **Priority**: high
- **Depends On**: T1-T5（Rust 核心 ready）
- **对应 AC**: AC-03, AC-04, AC-05, AC-15
- **覆盖 FR**: FR-GW-01/02/03
- **产出文件**:
  - 修改 `platform/gateway/runtime/src/routes/ai_engine.rs`（追加 alliance 静态长路径）
  - 修改 `platform/gateway/runtime/src/handlers/ai_engine.rs`（新增 handler + AiEngineState 字段扩展）
- **实现要求**:
  1. AiEngineState 新增字段：`alliance_engine: Arc<xuanji_expert::alliance::AllianceEngine>`、`alliance_metrics: Arc<AllianceMetrics>`（AtomicU64 包装：runs, gate_A, gate_B, gate_C, gate_D, …各阶段延迟）。
  2. 路由注册：
     ```rust
     Router::new()
       .route("/alliance/full", post(handler_alliance_full))
       .route("/alliance/capabilities", get(handler_alliance_capabilities))
       .route("/alliance/report/:trace_id", get(handler_alliance_report))
       // 已有 4 端点保留在原位置（静态优先 AC-10，alliance/full 更长，路由表顺序不影响 AC-10 语义）
     ```
  3. `handler_alliance_full`：解析 JSON 请求（query, session_id, context, options），若未授权 RBAC 401（FR-GW-06），否则调 `engine.run_full_analysis(req)` 并以 SSE `text/event-stream` 流式返回。
  4. `handler_alliance_capabilities`：注册 14 专家条目（AC-04）。
  5. `handler_alliance_report`：幂等查询 trace_id（未命中 404）。
  6. 集成测试 TDD：`test_alliance_full_sse` 抓 7 个事件；`test_alliance_unauthorized_401`。

### TR

| TR | 类型 | 条件 | 证据来源 |
|---|---|---|---|
| TR-T6-01 | rule | cargo test -p runtime ai_engine_alliance_ ≥15 通过（AC-03 基线） | cargo test |
| TR-T6-02 | rule | curl POST /alliance/full 后事件顺序严格：intent→team→debate→synthesize→gate→learn→done（AC-05） | 手动 curl + sse-cat |
| TR-T6-03 | rule | 无 token → 401；错误 token → 403；合法 token → 200（AC-15） | curl 3 组 |

---

## Task 7：Platform 网关 —— /voice/* 代理 + degraded 降级

- **Status**: pending
- **Priority**: high
- **Depends On**: T6 框架
- **对应 AC**: AC-10
- **覆盖 FR**: FR-GW-04
- **产出文件**:
  - 修改 `platform/gateway/runtime/src/routes/ai_engine.rs`（或新增 `routes/voice.rs` 并引入 subservers）
  - 修改 `platform/gateway/runtime/src/handlers/` 新增 `voice_proxy.rs`
- **实现要求**:
  1. Router 新增前缀 `/voice`，4 端点代理：
     - `GET /voice/health` → `GET http://127.0.0.1:3717/voice/health`；3s 超时，超时返回 `{ok:false, degraded:true, msg:"xiaobai service not running"}`。
     - `POST /voice/asr/full`（multipart）→ 透传 audio 字段。
     - `GET /voice/tts/stream?text=...` → 流式透传 audio/wav。
     - `GET /voice/ws/asr/stream` → WebSocket 代理（用 tokio_tungstenite + hyper upgrade，若实现复杂可先占位 degraded 返回 ws 地址让前端直连）。
  2. 配置可注入 `VOICE_PROXY_TARGET` 环境变量（默认 http://127.0.0.1:3717）。
  3. 测试：关闭 3717 时访问 /voice/health → HTTP 200 + degraded=true（不是 502，AC-10）。

### TR

| TR | 类型 | 条件 | 证据来源 |
|---|---|---|---|
| TR-T7-01 | rule | 3717 关闭时：curl GET /voice/health 返回 200，body.degraded=true（AC-10） | curl 手动 |
| TR-T7-02 | rule | 3717 打开时：透传 xiaobai_voice 返回的 asr.tts.engines 数组不丢失字段 | 前后对比 JSON diff |

---

## Task 8：subservers.rs 注册 + Task 9：metrics 扩展

> T8 + T9 可与 T6/T7 并行（依赖 T5 完成即可，不依赖 T6/T7 内部）

### Task 8：subservers.rs 注册 AllianceSubserver + VoiceSubserver

- **Status**: pending
- **Priority**: high
- **Depends On**: T5
- **对应 AC**: AC-22
- **覆盖 FR**: FR-GW-07（§六 HC-2 跨 crate 引用唯一通道）
- **产出文件**: 修改 `platform/gateway/runtime/src/subservers.rs`
- **实现要求**:
  1. 新增 `pub struct AllianceSubserver` 与 `pub struct VoiceSubserver`，实现统一 Subserver trait（若已存在 trait 则按现有模式）。
  2. 挂载前缀常量：`const ALLIANCE_PREFIX: &str = "/ai/engine/alliance"; const VOICE_PREFIX: &str = "/voice";`
  3. RBAC 边界登记：前缀 → `required_permission = "platform.alliance.use"` / `"platform.voice.use"`（用于 rbac_middleware.rs 批量检查）。

### Task 9：/ai/engine/metrics 扩展 alliance 统计字段

- **Status**: pending
- **Priority**: high
- **Depends On**: T5
- **对应 AC**: AC-17
- **覆盖 FR**: FR-GW-05
- **产出文件**: 修改 `handlers/ai_engine.rs::metrics_handler`
- **实现要求**: 输出 JSON 新增：
  ```json
  "alliance_total_runs": 1234,
  "alliance_gate_dist": {"A": 900, "B": 210, "C": 104, "D": 20},
  "alliance_phase_p95_ms": {"intent": 120, "team": 80, "debate": 1500, "synthesize": 300, "gate": 60, "learn": 40},
  "intent_distribution": {"math":100,"logic":100,"knowledge":200,"code":400,"chinese":200,"timeliness":100,"instruction":134},
  "learned_skills_count": 42
  ```
  所有值来自 AiEngineState.alliance_metrics 的 AtomicU64 快照（保证并发安全，不需要互斥锁）。

### TR（T8+T9 合并）

| TR | 类型 | 条件 | 证据来源 |
|---|---|---|---|
| TR-T8-01 | rule | subservers.rs 含 AllianceSubserver + VoiceSubserver（AC-22） | grep 两个名字存在 |
| TR-T9-01 | rule | metrics JSON 的 alliance_gate_dist 含 A/B/C/D 四个键且值≥0（AC-17） | curl /metrics + JSON schema 断言 |

---

## Task 10：前端 API 层统一封装（allianceApi + voiceApi）

- **Status**: pending
- **Priority**: high
- **Depends On**: T6/T7（路由 ready，后端面契约锁定）
- **对应 AC**: AC-FE-API 结构
- **覆盖 FR**: FR-FE-06
- **产出文件**:
  - 修改 `frontend-ui/src/api/index.js`（追加 allianceApi + voiceApi 导出）
  - 修改 `frontend-ui/vite.config.js`（dev proxy：/api/voice → http://localhost:3717）
- **实现要求**:
  1. `allianceApi.fullAnalysisSse({ query, sessionId, context, options }, onPhase, onDone, onError)`：用 `new EventSource('/api/ai/engine/alliance/full?query=...')` 或 fetch SSE 手动解析（若 query 过长用 POST 流：POST 方式 `fetch` + reader read line）。
  2. `allianceApi.capabilities() / allianceApi.report(traceId)`。
  3. `voiceApi.health() / voiceApi.asrFull(File or Blob) / voiceApi.ttsStream(text, {voice})`（ttsStream 返回 audio 元素可用的 blob URL）。
  4. 所有方法使用 `BASE = import.meta.env.VITE_API_BASE || '/api'` 前缀，禁止硬编码 localhost。
  5. vite.config.js `server.proxy` 新增：
     ```js
     '/api/voice': {
       target: 'http://localhost:3717',
       changeOrigin: true,
       rewrite: (p) => p.replace(/^\/api\/voice/, '/voice'),
       ws: true
     }
     ```
  6. Vitest 单测：`allianceApi` 至少 1 个 mock fetch 验证 URL 拼接正确。

### TR

| TR | 类型 | 条件 | 证据来源 |
|---|---|---|---|
| TR-T10-01 | rule | BASE 前缀不写死：grep `localhost` 在 api/index.js 中 = 0（除注释外） | grep |
| TR-T10-02 | rule | vite proxy /api/voice 条目存在（可在 vitest 读 vite config 验证或文件内容 grep） | 断言 |
| TR-T10-03 | rule | Vitest：`npm run test -- api/index.js` 单测 ≥1 通过 | npm run vitest 输出 |

---

## Task 11：ChatView —— 全维分析 φ 真工作 + 5 Chip 流转 + 报告卡片

- **Status**: pending
- **Priority**: high
- **Depends On**: T10
- **对应 AC**: AC-13, AC-14, AC-19, AC-R01, AC-R02
- **覆盖 FR**: FR-FE-01, FR-FE-05, FR-FE-07
- **产出文件**: 修改 `frontend-ui/src/views/ChatView.vue`
- **实现要求**:
  1. `triggerFullAnalysis()`（已有函数名）内部改为调 `allianceApi.fullAnalysisSse`：
     - phase 事件更新 `currentStage` Chip 激活（映射 phase→stage index）。
     - learn 阶段结束后把归一化报告 push 到 messages，类型 = `full_analysis_report`。
     - onError 把错误消息作为 assistant message 显示，Chip 回退到失败态。
  2. 全维分析报告消息卡片模板新增（或在 MessageBubble 中新增 type=full_analysis_report 的渲染分支）：
     - 顶部徽标：Gate A（绿）/ B（青）/ C（黄）/ D（红），显示综合得分。
     - 6 阶段完成列表：intent/team/debate/synthesize/gate/learn 各阶段时间戳 + 延迟 ms。
     - 14 维折叠面板：每维 score 进度条（按 0..1 百分比）+ 风险列表（带 severity 颜色）+ 建议列表。
     - Mermaid 全维流程图（可展开，默认折叠）：展示 intent→team→debate→synth→gate→learn 流 + 各节点附关键参数（HC-2/HC-8）。
     - 底部两按钮：复制报告 MD、导出 JSON。
  3. 空态快捷问法 3×2 Grid 内新增第 7 张卡片"启动全维分析示例"（FR-FE-07），点击触发 query="我想做一个 Rust 企业级服务，请帮我做架构开发专家联盟全维分析"并自动启动全维。
  4. 样式严格遵循：深空色系（global.css 令牌 `--ds-*`）+ 黄金间距 4/6/10/16/26/42（AC-20）。
  5. Playwright 或 DOM 探针：全维分析流程跑完后，断言 5 Chip `.stage-chip.active` 按序出现（AC-14）。

### TR

| TR | 类型 | 条件 | 证据来源 |
|---|---|---|---|
| TR-T11-01 | rule | 报告卡片 DOM 中：`[data-testid=gate-badge]` 存在；14 个折叠面板标题数量=14（AC-13） | Playwright DOM 计数 |
| TR-T11-02 | rule | 5 Chip 流转：录制 `.stage-chip.stage-*.active` 的激活顺序严格与 phase 事件顺序一致（AC-14） | 事件日志对比 DOM class |
| TR-T11-03 | rule | 复制报告按钮能把 Markdown 复制到剪贴板（Playwright 可调用 navigator.clipboard.readText 断言非空） | Playwright 脚本 |
| TR-T11-04 | rubric | SSE UI 流畅：AC-R02（2=≤800ms/阶段；阈值≥1） | Playwright 时间戳采集 |
| TR-T11-05 | rubric | 报告结构质量：AC-R01（2=14维+公式+Mermaid+导出齐全；阈值≥1） | DOM 探针 + 人工 |

---

## Task 12：ChatView —— 麦克风录音 + 小白语音开关 UI

- **Status**: pending
- **Priority**: high
- **Depends On**: T10
- **对应 AC**: AC-11, AC-19, AC-20
- **覆盖 FR**: FR-FE-02, FR-FE-03
- **产出文件**: 修改 `frontend-ui/src/views/ChatView.vue`（输入操作栏新增按钮）
- **实现要求**:
  1. 输入操作栏左起第一个按钮：Mic 麦克风录音（`Microphone` Element Plus 图标）。
     - 三态：idle(灰色) / recording(红色脉冲动画) / processing(旋转加载)。
     - 行为：短按点击切换录音状态（开始→结束录音→ASR）；或长按说话（松手结束）。
     - 录音使用 `navigator.mediaDevices.getUserMedia({audio:true})` + MediaRecorder，格式 audio/webm；录音结束后调用 `voiceApi.asrFull(blob)`，识别结果自动填入输入框（不自动发送，给用户修改机会）。
     - 权限被拒：按钮弹出 ElMessage 提示"请授权麦克风权限"，按钮仍可用但不可点击。
  2. Mic 右侧新增小白语音状态图标：`ChatDotRound` 或自定义小白头像。
     - 三态：offline(灰，3717 未启动) / online(绿，已联通) / error(红，异常)。
     - 启动时调用 `voiceApi.health()` 轮询 5s 更新状态。
     - 点击 offline 图标：打开弹窗"桌面小白 AI 智能助手未启动，是否查看启动指南？"，给出启动命令 `xiaobai run`（或 `cd projects/xiaobai_voice && python -m xiaobai_voice.cli start`）。
  3. 按钮尺寸、边距符合 26px 图标 + 10px 间距（黄金序列）。

### TR

| TR | 类型 | 条件 | 证据来源 |
|---|---|---|---|
| TR-T12-01 | rule | 人工录音 3 秒中文"你好璇玑"→ 输入框填入的文本非空（ASR 服务可用时）；ASR 不可用时返回 degraded=true 并提示"xiaobai 未启动，已使用浏览器录音但未识别，建议手动输入" | 人工验收 + DOM |
| TR-T12-02 | rule | 语音状态图标准确反映 3717 开/关：开=绿，关=灰，health endpoint 500=红 | 手动启停验证 |
| TR-T12-03 | rule | Console 0 error/warning（麦克风权限请求若用户拒绝也不应产生 error 级别 log，应 warning 以内）（AC-19） | Playwright console 采集 |

---

## Task 13：MessageBubble 朗读三层回退 + 进度显示

- **Status**: pending
- **Priority**: high
- **Depends On**: T10
- **对应 AC**: AC-12
- **覆盖 FR**: FR-FE-04
- **产出文件**: 修改 `frontend-ui/src/components/MessageBubble.vue`（已有朗读按钮，逻辑扩展）
- **实现要求**:
  1. 点击"朗读"按钮时：
     - 层 1（优先）：调用 `voiceApi.ttsStream(text)` → 返回 Blob URL → new Audio() 播放；同时按钮显示暂停图标 + 小进度条（从 audio element 取 currentTime/duration）。
     - 层 2（层 1 失败时自动降级）：`window.speechSynthesis.speak(new SpeechSynthesisUtterance(text))`，中文 zh-CN，速率 1.0。
     - 层 3（两层都失败）：按钮变灰，tooltip="当前环境不支持语音朗读"。
  2. 按钮三态：idle / playing（可暂停）/ paused（可继续）；停止时回到 idle。
  3. 朗读时不阻塞其他消息气泡的操作（不同消息实例 audio 独立，点其他朗读自动停止前一个）。
  4. 单元测试（mock Web Speech API + mock voiceApi.ttsStream）：`test_tts_fallback_chain`，验证：
     - ttsStream resolve → 不调用 speechSynthesis。
     - ttsStream reject → 调用 speechSynthesis。
     - 两者都 reject → 按钮 disabled。

### TR

| TR | 类型 | 条件 | 证据来源 |
|---|---|---|---|
| TR-T13-01 | rule | 关闭 3717 → 点朗读 → 浏览器播音（AC-12 降级链工作） | 人工验收 |
| TR-T13-02 | rule | Vitest：fallback chain 3 场景测试通过 | npm run vitest components |
| TR-T13-03 | rule | 同时点两条不同消息朗读 → 先停第一条再播第二条（不混播） | 人工 |

---

## Task 14：Vite 代理前缀统一 + 开发模式一键启动脚本（可选增强）

- **Status**: pending
- **Priority**: medium
- **Depends On**: T10
- **对应 AC**: AC-02（开发体验无退化）
- **覆盖 FR**: FR-FE-06
- **产出文件**:
  - 修改 `frontend-ui/package.json`（可选 scripts 追加 `dev:with-voice`）
  - 修改 `frontend-ui/vite.config.js`（T10 已覆盖代理，本任务核对完整性）
- **实现要求**:
  - 确认所有 `/api/ai/engine/alliance/*`、`/api/voice/*`、`/api/ai/engine/metrics` 都走正确代理。
  - 可选：新增 `dev:with-voice` 脚本：`concurrently "vite" "cd ../projects/xiaobai_voice && python -m xiaobai_voice.cli start --port 3717"`（若未安装 concurrently 可省略纯文档化说明）。

---

## Task 15：xiaobai_voice 默认 TTS cosyvoice2 + health 端点修正

- **Status**: pending
- **Priority**: high
- **Depends On**: （无，与 T10-T14 可并行）
- **对应 AC**: AC-21, AC-10, FR-VOICE-01/03
- **覆盖 FR**: FR-VOICE-01, FR-VOICE-03, FR-VOICE-05（部分）
- **产出文件**:
  - 修改 `projects/xiaobai_voice/xiaobai_voice/config/default_config.yaml`（tts.default_engine 改为 cosyvoice2）
  - 修改 `projects/xiaobai_voice/xiaobai_voice/service/main.py`（health 端点 JSON 对齐 spec：ok + asr + tts + endpoints）
  - 修改 `projects/xiaobai_voice/xiaobai_voice/tts/__init__.py`（build 默认不自动激活 Fish-S2-Pro，除非 license=accepted）
- **实现要求**:
  1. default_config.yaml::tts 段：`default_engine: cosyvoice2`（原 fish_s2_pro 改为第二优先）。
  2. `service/main.py::/voice/health` 返回结构必须：
     ```json
     {"ok": true,
      "asr": {"ready": bool, "model": "paraformer-zh", "backend": "sherpa-onnx"},
      "tts": {"ready": bool,
              "engines": [
                {"name": "cosyvoice2", "available": true, "license": "Apache-2.0"},
                {"name": "fish_s2_pro", "available": false, "license": "Research", "note": "手动 enable"}],
              "active": "cosyvoice2"},
      "endpoints": {"asr_full": "/voice/asr/full", "tts_stream": "/voice/tts/stream", "ws_asr_stream": "/voice/ws/asr/stream"}}
     ```
  3. tts/__init__.py::build 中：若配置 fish_s2_pro 但未设置 env `XIAOBAI_ACCEPT_RESEARCH_LICENSE=1`，返回 engine 不可用（避免 Research License 误激活）。
  4. 冒烟测试：`python -m pytest projects/xiaobai_voice/xiaobai_voice/tests/selftest.py -k health --no-header -q` 通过（或手动启动后 curl 验证 JSON schema）。

### TR

| TR | 类型 | 条件 | 证据来源 |
|---|---|---|---|
| TR-T15-01 | rule | default_config.yaml 中 grep `default_engine: cosyvoice2` 命中（AC-21） | grep |
| TR-T15-02 | rule | 启动后 curl /voice/health 返回 tts.active=cosyvoice2，fish_s2_pro license=Research，note=手动 enable | JSON schema 断言 |
| TR-T15-03 | rule | XIAOBAI_ACCEPT_RESEARCH_LICENSE=0 时，tts engines 中 fish_s2_pro.available=false（安全锁） | env 切换测试 |

---

## Task 16：全链路冒烟 + E2E + 独立评审汇总（Review 前置）

- **Status**: pending
- **Priority**: high
- **Depends On**: 所有 T1-T15
- **对应 AC**: 全部 AC-xx + AC-Rxx
- **覆盖 FR**: 全部
- **产出文件**:
  - `platform/services/xuanji-expert/tests/alliance_e2e.rs`（新增 Rust 集成测试）
  - `.trae/specs/20260825-rust-expert-alliance-platform/review.md`（Review 阶段独立评审员生成，本任务不写）
  - 前端 Playwright 用例追加到 `frontend-ui/tests/10-key-pages@P0.spec.js`（专家联盟全维分析场景）
- **实现要求（冒烟清单）**:
  1. **Rust 层**：`cargo test -p xuanji-expert alliance_ --nocapture` → 全通过；`cargo test -p runtime ai_engine_alliance_ --nocapture` → 全通过；`cargo test --workspace` 基线 passed 数 ≥ 640（AC-R07 ≥ 1）。
  2. **Golang/HTTP 契约层**（curl 或脚本）：
     ```
     (a) POST /api/ai/engine/alliance/full SSE 7 事件齐全且有序（AC-05）
     (b) GET  /api/voice/health degraded=true（停 3717）→ 200（AC-10）
     (c) GET  /api/ai/engine/metrics alliance_gate_dist ABCD 四键齐全（AC-17）
     (d) 401/403/200 RBAC 三件套（AC-15）
     ```
  3. **前端 Playwright 层**：
     - 打开 http://localhost:3021/#/ai → 0 console error/warning（AC-19）。
     - 点击「空态：启动全维分析示例」→ 5 Chip 流转（AC-14）→ 报告卡片出现（AC-13）→ 复制报告按钮成功。
     - 点麦克风按钮 → 录音→ASR→填入输入框（AC-11 或 degraded 提示不崩）。
     - 关闭 3717 → 点 MessageBubble 朗读 → 浏览器 TTS（AC-12）。
  4. **unsafe=0 + HC 常量锁死复查**：grep 代码确认（AC-18/07/08/09）。
  5. **649 passed 基线**：不退化（AC-R07）。

### TR（本任务 = 所有 AC 汇总）

| TR | 类型 | 条件 | 证据来源 |
|---|---|---|---|
| TR-T16-01 | rule | Rust 两个 test 套件 100% 通过 | cargo test 日志 |
| TR-T16-02 | rule | 4 HTTP 契约全满足 | curl 输出文件 |
| TR-T16-03 | rule | Playwright 4 场景全通过 + console 0 error/warning | Playwright report |
| TR-T16-04 | rule | unsafe=0；HC 常量正确（grep 复查） | grep |
| TR-T16-05 | rule | `cargo test --workspace` 最终 passed ≥ 640（AC-R07） | 摘要行 |
| TR-T16-06 | rubric | 所有 rubric 维度打分 ≥ 阈值最低限（AC-R01~AC-R07 每一项都 ≥ 其阈值） | 人工汇总表 |

---

> **版本**：v1.0 · 2026-08-25 · 任务数 16（T1~T16）；其中 P0=15，P1=1；依赖 DAG 6 层。
