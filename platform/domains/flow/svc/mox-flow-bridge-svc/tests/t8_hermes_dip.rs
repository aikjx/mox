// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Hermes-flow-bridge DIP 验收：下游（BridgeState / optimize_session）只依赖
//! `mox_ai_expert_svc::expert_traits::ExpertConsultant` trait，不依赖 concrete。
//!
//! tr_h8_01_bridge_state_with_mock：BridgeState 构造时注入 MockHealthy →
//!   optimize_session_with 返回 mock 数据，不触发 mox-expert 引擎。
//! tr_h8_02_gate_veto_flag_propagated：Mock 返回 vetoed=true → GateState 置位。
//! tr_h8_03_default_factory_is_trait_object：default_consultant() 返回的
//!   Arc<dyn ExpertConsultant> 可通过 trait object 调用（无 concrete 名）。
//! tr_h8_04_consultant_swappable_without_recompile：同一 session 先 Healthy 再 Veto
//!   两 consultant 互换证明纯 trait object 驱动。
//! tr_h8_05_no_mox_concrete_in_lib_use：静态扫描 src/，确保 hermes 生产代码
//!   `use mox_ai_expert_svc::X` 仅允许 X ∈ {expert_traits, types, domain}。

use async_trait::async_trait;
use mox_flow_bridge_svc::bridge::optimize_session_with;
use mox_flow_bridge_svc::normalize::ToolCall;
use mox_flow_bridge_svc::state::{BridgeState, GateState};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use mox_ai_expert_svc::expert_traits::ExpertConsultant;
use mox_ai_expert_svc::types::{ConsultQuery, ConsultReport};

fn small_graph() -> (std::sync::Arc<BridgeState>, mox_ai_flow_svc::model::FlowGraph) {
    let st = BridgeState::new();
    st.recorder.record(
        "default",
        &ToolCall {
            tool_name: "db.read".into(),
            args: json!({"q":"select 1"}),
            turn: 1,
        },
    );
    st.recorder.record(
        "default",
        &ToolCall {
            tool_name: "guard.authz".into(),
            args: json!({}),
            turn: 1,
        },
    );
    let g = st.recorder.snapshot("default").unwrap();
    (st, g)
}

// ---- Mock：healthy ----
struct MockHealthy;
#[async_trait]
impl ExpertConsultant for MockHealthy {
    async fn consult(&self, _q: &ConsultQuery) -> mox_ai_expert_svc::types::Result<ConsultReport> {
        unreachable!()
    }
    fn consult_blocking(&self, q: &ConsultQuery) -> mox_ai_expert_svc::types::Result<ConsultReport> {
        Ok(ConsultReport {
            report_id: q.id.clone(),
            steps: vec!["[MockHealthy] DIP".into()],
            score: 0.95,
            vetoed: false,
            reason: None,
        })
    }
}

// ---- Mock：vetoed ----
struct MockVeto;
#[async_trait]
impl ExpertConsultant for MockVeto {
    async fn consult(&self, _q: &ConsultQuery) -> mox_ai_expert_svc::types::Result<ConsultReport> {
        unreachable!()
    }
    fn consult_blocking(&self, q: &ConsultQuery) -> mox_ai_expert_svc::types::Result<ConsultReport> {
        Ok(ConsultReport {
            report_id: q.id.clone(),
            steps: vec!["[MockVeto] blocked".into()],
            score: 0.1,
            vetoed: true,
            reason: Some("policy block".into()),
        })
    }
}

#[test]
fn tr_h8_01_bridge_state_with_mock_uses_trait_object() {
    let st = BridgeState::with_consultant(Arc::new(MockHealthy));
    let g = st.recorder.snapshot("default").unwrap_or_else(|| {
        let tool = ToolCall {
            tool_name: "t".into(),
            args: json!({}),
            turn: 1,
        };
        st.recorder.record("default", &tool);
        st.recorder.snapshot("default").unwrap()
    });
    let rep = optimize_session_with(&g, &st.gate, st.consultant.clone());
    assert!((rep.score - 0.95).abs() < 1e-9, "score={}", rep.score);
    assert!(!st.gate.is_vetoed(), "healthy 不应触发否决");
    assert!(
        rep.steps.iter().any(|s| s.contains("DIP")),
        "mock 特征步骤缺失: {:?}",
        rep.steps
    );
}

#[test]
fn tr_h8_02_gate_veto_flag_propagated_via_trait() {
    let st = BridgeState::with_consultant(Arc::new(MockVeto));
    let (_st2, g) = small_graph();
    let rep = optimize_session_with(&g, &st.gate, st.consultant.clone());
    assert!(rep.vetoed, "mock veto 报告应 vetoed=true");
    assert!(st.gate.is_vetoed(), "GateState 应被置位 vetoed");
    assert_eq!(rep.score, 0.1);
}

#[test]
fn tr_h8_03_default_factory_returns_trait_object() {
    // 直接使用 expert_traits::default_consultant() 工厂，不出现 concrete 名
    let c: Arc<dyn ExpertConsultant> = mox_ai_expert_svc::expert_traits::default_consultant();
    // 可调用 trait 方法（通过 Arc deref），无需 `downcast_ref`
    let q = ConsultQuery {
        id: "q".into(),
        query: String::new(),
        ctx: std::collections::HashMap::new(),
    };
    // consult_blocking 是 trait 默认方法 / impl 覆写（均对象安全）
    let _ = c.consult_blocking(&q);
}

#[test]
fn tr_h8_04_consultant_swappable_without_recompile() {
    let gate = GateState::new();
    let g = mox_ai_flow_svc::model::FlowGraph::new("g", "g");

    // 同一 gate，先用 Healthy consultant（不否决）
    let healthy: Arc<dyn ExpertConsultant> = Arc::new(MockHealthy);
    let r1 = optimize_session_with(&g, &gate, healthy);
    assert!(!r1.vetoed);

    // 再用 Veto consultant（否决），gate 置位
    let veto: Arc<dyn ExpertConsultant> = Arc::new(MockVeto);
    let r2 = optimize_session_with(&g, &gate, veto);
    assert!(r2.vetoed);
    assert!(gate.is_vetoed());
    // 证明同一容器位置可无侵入替换不同实现 → DIP 成立
}

#[test]
fn tr_h8_05_no_mox_concrete_in_lib_use() {
    // 与 t8 测试 tr_08_01 同款扫描，但放在 hermes tests 目录内作为本地自检
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_dir = manifest.join("src");
    assert!(src_dir.is_dir());

    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(&p, out)
            } else if p.extension().map(|e| e == "rs").unwrap_or(false) {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(&src_dir, &mut files);
    assert!(!files.is_empty());

    let allowed = ["expert_traits", "types", "domain"];

    for f in &files {
        let content = std::fs::read_to_string(f).unwrap();
        for (idx, line) in content.lines().enumerate() {
            let t = line.trim_start();
            if !(t.starts_with("use ") || t.starts_with("pub use ")) {
                continue;
            }
            let Some(rest_p) = t.find("mox_ai_expert_svc::") else {
                continue;
            };
            let after = &t[rest_p + "mox_ai_expert_svc::".len()..];
            let end = after
                .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .unwrap_or(after.len());
            let mod_name = &after[..end];
            if !allowed.contains(&mod_name) {
                panic!(
                    "tr_h8_05 FAIL: {}:{} use mox_ai_expert_svc::{}（仅允许 expert_traits/types/domain）;\n原行: {}",
                    f.display(), idx + 1, mod_name, line.trim()
                );
            }
        }
    }
}
