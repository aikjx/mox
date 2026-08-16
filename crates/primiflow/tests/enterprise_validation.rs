//! PrimiFlow 企业级分步验证套件（L1 引擎内核 → L2 闭环集成 → L3 端到端 → L4 文档自生成）
//!
//! 运行：`cargo test -p primiflow --test enterprise_validation`
//!
//! 设计目标：每一步验证都可独立定位失败，覆盖「一定要可以运行」的全部质量闸门。

use flow_ai::primitive::{DeliveryPolicy, KnowledgeBase, PrimitiveState, PrimiEngine, ResourceBudget};
use primiflow::assoc::AssocGraph;
use primiflow::{enterprise_specs, run_all, run_pipeline};
use std::collections::HashMap;

/// 归一化容许误差（与引擎内部一致）
const EPS: f64 = 1e-6;

fn temp_out(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("primiflow_test").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn engine() -> PrimiEngine {
    PrimiEngine::new(
        10.0,
        KnowledgeBase::new(),
        ResourceBudget {
            total_ms: 2000,
            per_pool: HashMap::new(),
        },
    )
}

// ─────────────────────────────────────────────────────────────
// L1 · 引擎内核不变量（守恒公理 / 策略偏置 / 正则化降维）
// ─────────────────────────────────────────────────────────────

#[test]
fn l1_conservation_holds_for_all_policies() {
    for p in [
        DeliveryPolicy::Urgent,
        DeliveryPolicy::Balanced,
        DeliveryPolicy::Exploratory,
    ] {
        let s = PrimitiveState::from_policy(10.0, p, 0.0);
        assert!(
            s.is_conserved(EPS),
            "{:?} 违反守恒公理 C²=κ²+τ² (残差 {:.2e})",
            p,
            s.conservation_residual()
        );
    }
}

#[test]
fn l1_policy_bias_direction() {
    let urgent = PrimitiveState::from_policy(10.0, DeliveryPolicy::Urgent, 0.0);
    let explore = PrimitiveState::from_policy(10.0, DeliveryPolicy::Exploratory, 0.0);
    let balanced = PrimitiveState::from_policy(10.0, DeliveryPolicy::Balanced, 0.0);

    assert!(urgent.reuse_bias() > urgent.explore_bias(), "紧急交付应复用优先");
    assert!(explore.explore_bias() > explore.reuse_bias(), "探索研发应探索优先");
    assert!((balanced.kappa - balanced.tau).abs() < 1e-9, "均衡应 κ=τ");
}

#[test]
fn l1_reuse_pressure_raises_kappa() {
    // 知识库复用压力越大，κ 越高（贴近历史成熟链路）
    let low = DeliveryPolicy::Balanced.apply(10.0, 0.0);
    let high = DeliveryPolicy::Balanced.apply(10.0, 0.9);
    assert!(high.kappa > low.kappa, "高复用压力应抬高 κ");
}

// ─────────────────────────────────────────────────────────────
// L2 · 闭环集成：单需求跑通 + 复用累积 + 超预算正则化
// ─────────────────────────────────────────────────────────────

#[test]
fn l2_balanced_pipeline_all_green() {
    // 充足预算下，均衡正常需求无需 ℛ̂ 正则化即可通过三道闸门
    let mut e = PrimiEngine::new(
        10.0,
        KnowledgeBase::new(),
        ResourceBudget {
            total_ms: 10_000,
            per_pool: HashMap::new(),
        },
    );
    let mut master = AssocGraph::new();
    let out = temp_out("l2_balanced");

    let spec = &enterprise_specs()[0]; // r1 电商月度经营分析报告（均衡）
    let req = spec.requirement();
    let rep = run_pipeline(&mut e, &req, spec.policy, &mut master, &out).unwrap();

    assert!(rep.all_ok(), "分步验证未全绿:\n{rep}");
    assert!(rep.conserved, "守恒未满足");
    assert!(rep.acyclic, "涌现拓扑应无环");
    assert!(!rep.regularized, "预算充足时均衡正常需求不应触发正则化");
    assert!(rep.q_after > rep.q_before, "成功回灌应注入拓扑荷 Q");
}

#[test]
fn l2_assets_accumulate_across_requests() {
    let mut e = engine();
    let mut master = AssocGraph::new();
    let out = temp_out("l2_reuse");
    let specs = enterprise_specs();

    // r1 成功注库
    let r1 = specs[0].requirement();
    let rep1 = run_pipeline(&mut e, &r1, specs[0].policy, &mut master, &out).unwrap();
    assert!(rep1.all_ok(), "r1 分步验证未全绿:\n{rep1}");

    // r2 在共享引擎上运行（知识库已沉淀 r1 的拓扑资产）
    let r2 = specs[1].requirement();
    let rep2 = run_pipeline(&mut e, &r2, specs[1].policy, &mut master, &out).unwrap();
    assert!(rep2.all_ok(), "r2 分步验证未全绿:\n{rep2}");

    // 成功回灌后应沉淀拓扑资产并累计拓扑荷 Q（κ 复用资产的真正来源）
    assert!(!e.kb.stored.is_empty(), "知识库应已固化拓扑模板");
    assert!(e.state.q > 0.0, "成功回灌应累计拓扑荷 Q");
    assert!(e.kb.stored.len() >= 2, "两个需求应各自沉淀资产");
}

#[test]
fn l2_overbudget_triggers_regularize() {
    let mut e = engine(); // 预算 2000ms
    let mut master = AssocGraph::new();
    let out = temp_out("l2_reg");

    // r4 子任务总算力 2600ms > 2000ms → 必触发 ℛ̂ 正则化裁剪
    let spec = &enterprise_specs()[3];
    let req = spec.requirement();
    let rep = run_pipeline(&mut e, &req, spec.policy, &mut master, &out).unwrap();

    assert!(rep.all_ok(), "超预算需求经正则化后仍应全绿:\n{rep}");
    assert!(rep.regularized, "超预算需求应触发 ℛ̂ 正则化");
    assert!(rep.acyclic, "正则化后拓扑必须无环");
}

// ─────────────────────────────────────────────────────────────
// L3 · 端到端：整组需求跑通并产出全部落地产物
// ─────────────────────────────────────────────────────────────

#[test]
fn l3_e2e_runs_and_generates_artifacts() {
    let mut e = engine();
    let out = temp_out("l3_e2e");
    let specs = enterprise_specs();

    let reports = run_all(&mut e, &specs, &out).unwrap();
    assert_eq!(reports.len(), specs.len());

    for rep in &reports {
        assert!(rep.all_ok(), "存在未通过的需求:\n{rep}");
    }

    // 文档自生成产物非空
    for f in ["graph.mmd", "trace_matrix.md", "ddl.sql", "schema.rs", "mod.rs"] {
        let p = out.join(f);
        assert!(p.exists(), "缺少产物 {f}");
        let len = std::fs::metadata(&p).unwrap().len();
        assert!(len > 0, "产物 {f} 为空");
    }

    // 代码骨架模块数 = 全部子任务数（3+3+4+5 = 15）
    let code_count = std::fs::read_dir(&out)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.ends_with(".rs") && n.starts_with("c_")
        })
        .count();
    assert!(code_count >= 15, "代码骨架模块不足，实际 {code_count}");

    // 涌现 DAG 可视化张数 = 需求数
    let topo_count = std::fs::read_dir(&out)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().into_owned().starts_with("topo_"))
        .count();
    assert_eq!(topo_count, specs.len(), "涌现 DAG 可视化张数不符");

    // 知识库已沉淀资产（κ 复用基础）
    assert!(!e.kb.stored.is_empty(), "知识库应已固化至少一个拓扑模板");
}

// ─────────────────────────────────────────────────────────────
// L4 · 文档自生成质量：产物内容正确、可追溯、可编译结构
// ─────────────────────────────────────────────────────────────

#[test]
fn l4_doc_content_quality() {
    let mut e = engine();
    let out = temp_out("l4_doc");
    let specs = enterprise_specs();
    let _ = run_all(&mut e, &specs, &out).unwrap();

    let graph = std::fs::read_to_string(out.join("graph.mmd")).unwrap();
    assert!(graph.contains("flowchart LR"), "Mermaid 图头应为 flowchart LR");
    for s in &specs {
        assert!(graph.contains(&s.name), "可视化图应含需求「{}」", s.name);
    }

    let matrix = std::fs::read_to_string(out.join("trace_matrix.md")).unwrap();
    assert!(matrix.contains("需求"), "溯源矩阵应包含表头");
    // 共享子任务以 ascii key 形式贯穿所有需求（保证生成代码标识符合法）
    assert!(matrix.contains("report"), "溯源矩阵应含共享子任务 report");
    // 一一对应不变量：绑定校验无断裂（run_all 内已断言，这里交叉验证产物存在）
    assert!(matrix.contains("|"), "溯源矩阵应已渲染表格");

    let schema = std::fs::read_to_string(out.join("schema.rs")).unwrap();
    assert!(schema.contains("pub struct"), "schema.rs 应含结构体定义");
    assert!(schema.contains("serde"), "schema.rs 应启用 serde 派生");
    assert!(schema.contains("DateTime<Utc>"), "schema.rs 应含时间字段类型");

    let ddl = std::fs::read_to_string(out.join("ddl.sql")).unwrap();
    assert!(ddl.contains("CREATE TABLE"), "ddl.sql 应含建表语句");
}
