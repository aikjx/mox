// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! T8 · DIP 反转验证（mox-expert 对外抽象 trait + 下游只依赖 trait）
//!
//! 10 个 TR：
//! - tr_08_01_hermes_only_use_traits：静态扫描 hermes-flow-bridge/src 所有 .rs，
//!   凡 `use mox_ai_expert_svc::X` 形式只允许 X ∈ {expert_traits, types, domain}。
//! - tr_08_02_catalog_only_use_traits：同上对 business-catalog/src。
//! - tr_08_03_mock_consultant：最小 MockExpert impl ExpertConsultant trait。
//! - tr_08_04_build_and_unit_test：3 crate `cargo test --lib` exit 0。
//! - tr_08_05_mock_govern_expert_pass：MockGovernExpert 走 GovernExpert trait 返回 Pass。
//! - tr_08_06_mock_govern_expert_block：forced_level=Block → GovernVerdict.level=Block（DIP 可替换性）。
//! - tr_08_07_minimal_context_impl_govern_context：MinimalGovernContext 实现 GovernContext
//!   trait（不依赖 concrete context.rs 结构）。
//! - tr_08_08_govern_trait_object_boxable：Arc<dyn GovernExpert> + &dyn GovernContext 组合调用。
//! - tr_08_09_alliance_orchestrator_trait_is_object_safe：default_orchestrator 返回
//!   Arc<dyn AllianceOrchestrator> 可调用 route()（trait object 安全 + 可构造）。
//! - tr_08_10_registry_trait_object_safe_operations：default_registry() 可 list / find
//!   （Arc<dyn ExpertRegistry> 路径，不出现 concrete RegistryImpl）。

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use mox_ai_expert_svc::domain::{
    GovernContext, GovernExpert, GovernLevel, GovernVerdict, MinimalGovernContext, MockGovernExpert,
};
use mox_ai_expert_svc::expert_traits::{AllianceOrchestrator, ExpertConsultant, ExpertRegistry};
use mox_ai_expert_svc::types::{ConsultQuery, ConsultReport, TaskSpec};

// ============================================================================
// 工具：递归枚举 .rs 文件 + 定位工作区根 + use 违规扫描
// ============================================================================

fn list_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                out.push(path);
            }
        }
    }
    walk(dir, &mut out);
    out.sort();
    out
}

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .unwrap() // services
        .parent()
        .unwrap() // platform
        .parent()
        .unwrap() // infotopograph (workspace root)
        .to_path_buf()
}

fn scan_mox_use_violations(file: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => return vec![format!("无法读取文件 {}: {}", file.display(), e)],
    };
    let mut violations = Vec::new();
    let allowed = ["expert_traits", "types", "domain"];
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if !(trimmed.starts_with("use ")
            || trimmed.starts_with("pub use ")
            || trimmed.starts_with("pub(crate) use "))
        {
            continue;
        }
        let rest_after_keyword = match trimmed.find("mox_ai_expert_svc::") {
            Some(p) => &trimmed[p..],
            None => continue,
        };
        let after = &rest_after_keyword["mox_ai_expert_svc::".len()..];
        let end = after
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        let mod_name = &after[..end];
        if !allowed.contains(&mod_name) {
            violations.push(format!(
                "{}:{} 非白名单模块 use mox_ai_expert_svc::{} (期望 expert_traits/types/domain) —— 行内容：{}",
                file.display(),
                idx + 1,
                mod_name,
                line.trim()
            ));
        }
    }
    violations
}

// ============================================================================
// tr_08_01 / tr_08_02：静态扫描（允许 domain 模块）
// ============================================================================

#[test]
fn tr_08_01_hermes_only_use_traits() {
    let ws = workspace_root();
    let dir = ws.join("platform/services/hermes-flow-bridge/src");
    assert!(dir.is_dir(), "找不到 hermes src: {}", dir.display());
    let files = list_rs_files(&dir);
    assert!(!files.is_empty(), "hermes src 目录至少应有一个 .rs 文件");

    let mut all_violations: Vec<String> = Vec::new();
    for f in &files {
        all_violations.extend(scan_mox_use_violations(f));
    }
    assert!(
        all_violations.is_empty(),
        "hermes-flow-bridge 存在 DIP 违规 use 语句：\n{}",
        all_violations.join("\n")
    );
}

#[test]
fn tr_08_02_catalog_only_use_traits() {
    let ws = workspace_root();
    let dir = ws.join("platform/services/business-catalog/src");
    assert!(
        dir.is_dir(),
        "找不到 business-catalog src: {}",
        dir.display()
    );
    let files = list_rs_files(&dir);
    assert!(!files.is_empty(), "catalog src 至少应有一个 .rs 文件");

    let mut all_violations: Vec<String> = Vec::new();
    for f in &files {
        all_violations.extend(scan_mox_use_violations(f));
    }
    assert!(
        all_violations.is_empty(),
        "business-catalog 存在 DIP 违规 use 语句：\n{}",
        all_violations.join("\n")
    );
}

// ============================================================================
// tr_08_03：MockExpert DIP 证据
// ============================================================================

struct MockExpertEmpty;
#[async_trait]
impl ExpertConsultant for MockExpertEmpty {
    async fn consult(&self, _q: &ConsultQuery) -> mox_ai_expert_svc::types::Result<ConsultReport> {
        unreachable!("sync-only mock")
    }
    fn consult_blocking(&self, q: &ConsultQuery) -> mox_ai_expert_svc::types::Result<ConsultReport> {
        Ok(ConsultReport {
            report_id: q.id.clone(),
            steps: vec!["[MockExpertEmpty] 空咨询报告（DIP 证据：无璇玑 concrete）".into()],
            score: 1.0,
            vetoed: false,
            reason: None,
        })
    }
}

#[test]
fn tr_08_03_mock_consultant() {
    let consultant: Arc<dyn ExpertConsultant> = Arc::new(MockExpertEmpty);
    let query = ConsultQuery {
        id: "mock-q".into(),
        query: "Hello".into(),
        ctx: HashMap::new(),
    };
    let rep = consultant
        .consult_blocking(&query)
        .expect("mock consult 必成功");
    assert_eq!(rep.report_id, "mock-q");
    assert!((rep.score - 1.0).abs() < 1e-9);
    assert!(!rep.vetoed);
    assert_eq!(rep.steps.len(), 1);
    assert!(rep.steps[0].contains("DIP 证据"));
    assert!(rep.reason.is_none());

    fn downstream_api(consultant: Arc<dyn ExpertConsultant>) -> ConsultReport {
        let q = ConsultQuery {
            id: "d".into(),
            query: String::new(),
            ctx: HashMap::new(),
        };
        consultant.consult_blocking(&q).unwrap()
    }
    let r2 = downstream_api(consultant.clone());
    assert_eq!(r2.report_id, "d");
    assert!(!r2.vetoed);
}

// ============================================================================
// tr_08_05 / tr_08_06：GovernExpert Mock DIP（Pass / Block）
// ============================================================================

fn simple_graph_ctx_pair() -> (mox_ai_flow_svc::model::FlowGraph, MinimalGovernContext) {
    let g = mox_ai_flow_svc::model::FlowGraph::new("test", "test");
    let ctx = MinimalGovernContext::default();
    (g, ctx)
}

#[test]
fn tr_08_05_mock_govern_expert_pass_via_trait() {
    let expert = MockGovernExpert::default(); // forced_level=Pass
    let (g, ctx) = simple_graph_ctx_pair();
    // 通过 trait object（同步路径，不走 tokio）
    let v: GovernVerdict = GovernExpert::govern_blocking(&expert, &g, &ctx);
    assert_eq!(v.level, GovernLevel::Pass);
    assert!((v.score - 1.0).abs() < 1e-9);
    assert_eq!(v.gate_id, "mock-gate-sync");
    assert!(v.reasons.iter().any(|r| r.contains("DIP 证据")));
}

#[test]
fn tr_08_06_mock_govern_expert_block_swap_proves_replacement() {
    // 同一 GovernExpert trait 位置：替换为 Block 行为 → 返回 Block
    let expert = MockGovernExpert {
        forced_level: GovernLevel::Block,
        fixed_score: 0.0,
    };
    let (g, ctx) = simple_graph_ctx_pair();
    let v = GovernExpert::govern_blocking(&expert, &g, &ctx);
    assert_eq!(v.level, GovernLevel::Block);
    assert!((v.score - 0.0).abs() < 1e-9);
}

// ============================================================================
// tr_08_07：MinimalGovernContext 实现 GovernContext trait（字段独立自洽）
// ============================================================================

#[test]
fn tr_08_07_minimal_context_impl_trait_getters() {
    let ctx = MinimalGovernContext {
        tenant: "t1".into(),
        namespace: "ns1".into(),
        principal: "alice".into(),
        roles: vec!["admin".into(), "approver".into()],
        regulated: true,
        max_parallel: 16,
        cost_budget: 500.0,
        sla_ms: 30_000,
    };
    let d: &dyn GovernContext = &ctx;
    assert_eq!(d.tenant(), "t1");
    assert_eq!(d.namespace(), "ns1");
    assert_eq!(d.principal(), "alice");
    assert_eq!(d.roles(), &["admin".to_string(), "approver".to_string()]);
    assert!(d.is_regulated());
    assert_eq!(d.max_parallel(), 16);
    assert!((d.cost_budget() - 500.0).abs() < 1e-9);
    assert_eq!(d.sla_ms(), 30_000);
}

// ============================================================================
// tr_08_08：GovernExpert + GovernContext trait object 组合可调用
// ============================================================================

#[test]
fn tr_08_08_govern_trait_object_boxable_and_callable() {
    // Arc<dyn GovernExpert> 可接收任意 GovernExpert impl
    let e: Arc<dyn GovernExpert> = Arc::new(MockGovernExpert {
        forced_level: GovernLevel::Warn,
        fixed_score: 0.5,
    });
    let ctx = MinimalGovernContext::default();
    let dyn_ctx: &dyn GovernContext = &ctx;
    let g = mox_ai_flow_svc::model::FlowGraph::new("g", "g");
    // 通过 Arc<dyn GovernExpert> 调用 trait 方法 → 对象安全 + 可运行
    let v = e.govern_blocking(&g, dyn_ctx);
    assert_eq!(v.level, GovernLevel::Warn);
    assert!((v.score - 0.5).abs() < 1e-9);
}

// ============================================================================
// tr_08_09：AllianceOrchestrator trait object safe（工厂可直接构造）
// ============================================================================

#[tokio::test]
async fn tr_08_09_alliance_orchestrator_trait_object_safe() {
    // 默认注册表 + 默认编排器 → trait object
    let reg: Arc<dyn ExpertRegistry> = mox_ai_expert_svc::expert_traits::default_registry();
    let router: Arc<dyn AllianceOrchestrator> =
        mox_ai_expert_svc::expert_traits::default_orchestrator(reg);
    // 空 task，验证 trait object 可调 route（无需 concrete 名）
    let spec = TaskSpec {
        task_id: "s".into(),
        scenario: "default".into(),
        constraints: HashMap::new(),
    };
    // route 可能缺专家返回 Err，但只要不出现 concrete 类型名即合规
    let _ = router.route(&spec).await;
}

// ============================================================================
// tr_08_10：ExpertRegistry trait object 操作
// ============================================================================

#[tokio::test]
async fn tr_08_10_registry_trait_object_safe_operations() {
    let reg: Arc<dyn ExpertRegistry> = mox_ai_expert_svc::expert_traits::default_registry();
    // list(None) 通过 trait object
    let all = reg.list(None).await;
    assert!(all.is_ok(), "应能列出默认注册表（至少内置几个）");
    let all = all.unwrap();
    if !all.is_empty() {
        let first_id = all[0].id.clone();
        let found = reg.find(&first_id).await.unwrap();
        assert!(found.is_some(), "应能通过 trait find 找到刚 list 到的 id");
        assert_eq!(found.unwrap().id, first_id);
    }
}

// ============================================================================
// tr_08_04：3 crate --lib 单元测试 exit 0
// ============================================================================

#[test]
fn tr_08_04_build_and_unit_test() {
    let ws = workspace_root();
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&ws)
        .arg("test")
        .arg("-p")
        .arg("mox-expert")
        .arg("-p")
        .arg("hermes-flow-bridge")
        .arg("-p")
        .arg("business-catalog")
        .arg("--lib")
        .arg("--quiet");

    let status = cmd.status().expect("无法启动 cargo 子进程");

    assert!(
        status.success(),
        "DIP 改造破坏了现有单元测试：cargo test -p mox-expert -p hermes-flow-bridge -p business-catalog --lib 返回 {:?}\n\
         工作目录：{}",
        status.code(),
        ws.display()
    );
}
