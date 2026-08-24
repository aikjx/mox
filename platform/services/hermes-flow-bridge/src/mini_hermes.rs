//! Mini Hermes：可运行 agent-loop 原型 + LLM 调用计数器。
//!
//! 证明「录制即优化 + 复用路由」能真实削减 LLM 调用次数（用户原方案核心收益：LLM 调用减半）。
//! 在 baseline 模式里每个工具调用都跑一次 LLM ReAct 决策；在 bridge 模式里命中复用模板的工具
//! 段整段跳过 LLM，直接按已知模板回放，LLM 只用于「未知 / 未命中」部分。
//!
//! 该模块不依赖 Hermes 源码，纯本地，可单测、可 CLI 演示（见 bin/bridge_demo.rs）。

use crate::router::FlowTemplate;
use crate::state::BridgeState;
use serde_json::{json, Value};

/// LLM 调用计数（原子，跨回放共享）。
#[derive(Default)]
pub struct LlmTracer {
    pub calls: std::sync::atomic::AtomicU64,
}

impl LlmTracer {
    pub fn new() -> Self {
        Self::default()
    }
    /// 模拟一次 LLM ReAct 决策（计入调用）。返回「下一步工具」由 LLM 给出。
    pub fn decide(&self, _ctx: &str) -> String {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // 真实 Hermes 这里会产出下一工具名 + 参数；原型里由外部 plan 提供，所以只计数。
        String::new()
    }
    pub fn count(&self) -> u64 {
        self.calls.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// 任务计划：一个工具序列（模拟一条可复用业务流程）。
pub fn gov_pii_plan() -> Vec<(&'static str, Value)> {
    vec![
        ("db.read", json!({"query":"select * from citizen_info"})),
        ("guard.desensitize", json!({"var":"citizen"})),
        ("web1.submit", json!({})),
        ("merge.report", json!({})),
    ]
}

/// 把计划注册成可复用模板（首次跑完由璇玑引擎提炼后写入；原型里直接注册）。
pub fn register_gov_template(st: &BridgeState) {
    st.router.register(FlowTemplate {
        id: "gov-pii".into(),
        tool_seq: gov_pii_plan()
            .into_iter()
            .map(|(t, _)| t.to_string())
            .collect(),
    });
}

/// Baseline 执行：每个工具都调一次 LLM 决策（模拟线性 ReAct 主循环）。
pub fn run_baseline(plan: &[(&str, Value)], tracer: &LlmTracer) -> Vec<String> {
    let mut executed = Vec::new();
    for (tool, args) in plan {
        tracer.decide("baseline"); // 每步一次 LLM
        executed.push(format!("{tool} <-llm"));
        let _ = args;
    }
    executed
}

/// Bridge 执行：命中复用模板的前缀段整段跳过 LLM（按已知模板回放）；
/// 未命中部分（未知工具）才调 LLM。
pub fn run_bridge(st: &BridgeState, plan: &[(&str, Value)], tracer: &LlmTracer) -> Vec<String> {
    let tools: Vec<String> = plan.iter().map(|(t, _)| t.to_string()).collect();
    let mut executed = Vec::new();
    let mut i = 0;
    while i < plan.len() {
        let cur: Vec<String> = tools[i..].to_vec();
        // 找一张「是 cur 前缀」的已知模板 → 回放该段，跳过 LLM
        if let Some(tpl_id) = st.router.match_prefix(&cur) {
            if let Some(tpl) = st.router.get_template(&tpl_id) {
                for tool in &tpl.tool_seq {
                    executed.push(format!("{tool} <-replay"));
                }
                executed.push(format!("REPLAY<{tpl_id}>"));
                i += tpl.tool_seq.len();
                continue;
            }
        }
        // 未知工具：调 LLM 决策
        tracer.decide("bridge");
        executed.push(format!("{} <-llm", plan[i].0));
        i += 1;
    }
    executed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_calls_llm_per_step() {
        let tracer = LlmTracer::new();
        run_baseline(&gov_pii_plan(), &tracer);
        assert_eq!(tracer.count(), 4, "baseline 每步一次 LLM = 4 次");
    }

    #[test]
    fn bridge_skips_llm_when_template_known() {
        let st = BridgeState::new();
        register_gov_template(&st);
        let tracer = LlmTracer::new();
        let out = run_bridge(&st, &gov_pii_plan(), &tracer);
        // 整段已知 → 0 次 LLM
        assert_eq!(tracer.count(), 0, "已知流程应 0 次 LLM");
        assert!(out.iter().any(|s| s.contains("REPLAY<gov-pii>")));
        // 每条工具行都走 replay，无 <-llm 行
        let tool_lines: Vec<&String> = out.iter().filter(|s| !s.contains("REPLAY")).collect();
        assert!(!tool_lines.is_empty());
        assert!(tool_lines.iter().all(|s| s.contains("<-replay")));
        assert!(out.iter().all(|s| !s.contains("<-llm")));
    }

    #[test]
    fn bridge_calls_llm_only_for_unknown_tail() {
        let st = BridgeState::new();
        // 只注册前缀模板：db.read→guard
        st.router.register(FlowTemplate {
            id: "partial".into(),
            tool_seq: vec!["db.read".into(), "guard.desensitize".into()],
        });
        let tracer = LlmTracer::new();
        let plan = gov_pii_plan();
        run_bridge(&st, &plan, &tracer);
        // 4 步里前 2 步命中前缀回放（0 LLM），后 2 步未知 → 2 次 LLM
        assert_eq!(tracer.count(), 2, "部分已知：未知尾部 2 步各 1 次 LLM");
    }
}
