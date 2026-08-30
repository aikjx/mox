// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use std::sync::Arc;

use mox_ai_expert_svc::types::{ConsultQuery, ConsultReport};

use crate::business::register_business_experts;
use crate::flows::all_businesses;
use crate::topology::build_topology;

#[test]
fn catalog_builds_all_businesses() {
    let biz = all_businesses();
    assert_eq!(biz.len(), 6, "架构内置业务数应为 6");
    for b in &biz {
        let g = (b.build)();
        assert!(!g.nodes.is_empty(), "{} 应有节点", b.id);
        assert!(g.topo_order().is_ok(), "{} 应为 DAG", b.id);
    }
}

#[test]
fn topology_has_six_dimension_entities() {
    let topo = build_topology();
    let kinds: std::collections::HashSet<_> = topo
        .entities
        .iter()
        .map(|e| format!("{:?}", e.kind))
        .collect();
    for k in ["Model", "Tool", "Skill", "Memory", "Rule", "FlowNode"] {
        assert!(kinds.iter().any(|x| x == k), "关系网应含 {} 维", k);
    }
}

// —— DIP 证据：业务 optimize() 可换 MockExpert 运行（不依赖 mox-expert concrete）——
#[test]
fn business_optimize_uses_mock_consultant_via_trait() {
    use async_trait::async_trait;
    struct MockAlwaysApproved;
    #[async_trait]
    impl mox_ai_expert_svc::expert_traits::ExpertConsultant for MockAlwaysApproved {
        async fn consult(
            &self,
            _q: &ConsultQuery,
        ) -> mox_ai_expert_svc::types::Result<ConsultReport> {
            unreachable!("sync 路径不进入 async consult")
        }
        fn consult_blocking(
            &self,
            q: &ConsultQuery,
        ) -> mox_ai_expert_svc::types::Result<ConsultReport> {
            Ok(ConsultReport {
                report_id: q.id.clone(),
                steps: vec!["[Mock] 已批准（无璇玑引擎）".into()],
                score: 0.85,
                vetoed: false,
                reason: None,
            })
        }
    }
    let biz = &all_businesses()[0]; // gov-pii
    let rep = biz.optimize_with(Arc::new(MockAlwaysApproved));
    assert_eq!(rep.report_id, biz.id);
    assert!((rep.score - 0.85).abs() < 1e-9);
    assert!(!rep.vetoed);
}

#[tokio::test]
async fn register_business_experts_runs_via_registry_trait() {
    // DIP 证据：生产路径 register_business_experts 只依赖 Arc<dyn ExpertRegistry>，
    // 使用默认注册表工厂（default_registry），不出现任何 concrete struct 名字。
    let reg = mox_ai_expert_svc::expert_traits::default_registry();
    register_business_experts(reg.clone()).await.unwrap();
    let all = reg.list(Some("gov")).await.unwrap();
    assert!(!all.is_empty(), "应注册 gov 领域专家");
    assert!(
        reg.find("biz-gov-pii").await.unwrap().is_some(),
        "应注册 gov-pii 的领域专家"
    );
}
