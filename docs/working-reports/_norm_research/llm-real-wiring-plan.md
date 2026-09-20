# 接真 LLM：专家顾问生产实现接入方案（LLM Real Wiring Plan）

- 版本：v1.0 · 2026-09-18
- 状态：**待用户提供模型凭证后实施**（本文件只定方案，未改生产代码）
- 关联：`docs/working-reports/_norm_research/runtime-e2e-acceptance.md` §4（dev 态 llm_calls=0）

## 1. 现状根因（已逐文件核实）

专家联盟专家节点 5ms 跑完、`llm_calls:0`，**不是配置问题，是这块还没从 0 写到 1**：

- 执行契约 `ExpertConsultant` trait 在
  `platform/domains/ai/proto/mox-ai-expert-proto/src/traits.rs:60`，
  唯一方法 `async fn consult(&self, query: &ConsultQuery) -> Result<ConsultReport>`。
- 整个 alliance 域内，该 trait **只有测试用 `MockConsultant` 一个实现**
  （`mox-alliance-executor-core/src/expert_executor.rs:659`，`#[cfg(test)]` 内）；
  **没有任何生产实现**，生产进程注入的必然是桩/内存顾问。
- 旁边的 LLM 路由层 `mox-alliance-scheduler-core/src/llm_router.rs` 已写完整
  （priority/轮询/延迟/成本四种选路 + 熔断恢复 + 从环境变量解析 key），
  但它只负责"选 provider"，**没有任何代码把它接到一个真发 HTTP 请求的顾问上**。

## 2. 数据结构（接 LlmConsultant 必需）

`mox-ai-expert-proto/src/types.rs`：

```text
ConsultQuery { id: String, query: String, ctx: HashMap<String,String> }
ConsultReport { report_id: String, steps: Vec<String>, score: f64, vetoed: bool /* + 其余字段见 types.rs:426 */ }
```

> 另一条线：`mox-ai-alliance-engine/src/debate.rs:58` 有同名 trait `ExpertConsultant`（辩论用，返回 `ExpertOpinion`），与本接线无关，勿混淆。

## 3. 设计：新写 `LlmConsultant`（OpenAI 兼容）

新增文件（建议放 `mox-alliance-executor-core/src/llm_consultant.rs`）：

- `impl ExpertConsultant for LlmConsultant`，`consult()` 内：
  1. 用 `query.query` 作为 user message，`ctx["prefer_expert"]` 拼 system/角色；
  2. reqwest POST `${MOX_LLM_BASE_URL}/chat/completions`，
     `Authorization: Bearer ${MOX_LLM_API_KEY}`，body `{model, messages, temperature}`；
  3. 把模型返回文本包成 `ConsultReport{ report_id: query.id, steps: vec![...], score: 0.8, vetoed:false }`。
- **配置走环境变量，绝不硬编码 key**：
  - `MOX_LLM_BASE_URL`（缺省 `https://api.openai.com/v1`；DeepSeek 填 `https://api.deepseek.com/v1`；Ollama 填 `http://localhost:11434/v1`）
  - `MOX_LLM_API_KEY`（兼容回退 `OPENAI_API_KEY`；Ollama 可空）
  - `MOX_LLM_MODEL`（缺省 `gpt-4o-mini`；DeepSeek 填 `deepseek-chat`；Ollama 填本地模型名）
- **无配置优雅降级**：`MOX_LLM_BASE_URL/MODEL` 都未设时，`consult()` 返回明确标注"未配置模型，走内置模板"的报告（不 panic、不影响现有四进程）；
  一旦配了就真调。

## 4. 生产注入切换点

- 现在：executor-svc 生产 main 注入的是桩顾问。
- 改法：在 executor-svc 组装处按"是否配了 `MOX_LLM_BASE_URL`"二选一——
  配了注入 `Arc::new(LlmConsultant::from_env())`，否则维持现有桩。
  这样零配置行为不变、可灰度，不需要停机改代码。

## 5. 验证计划（拿到 key 后）

1. `cargo test -p mox-alliance-executor-core`：新增 LlmConsultant 单测
   （无配置降级、请求体构造、错误映射；HTTP 用本地 mock server）。
2. 设 `MOX_LLM_*` 后重启 executor:3200，提交一个任务，
   断言 `llm_calls>0`、节点输出为模型真实文本、`score` 合理。

## 6. 待用户拍板（阻塞项）

- 用哪类端点：OpenAI 兼容 / 本地 Ollama / Anthropic（协议不同）。
- 提供：`base_url`、`api_key`（或确认已在环境变量）、`model` 名。
凭证齐了即可按 §3–§5 实施并真跑验证。
