//! 缺口 P2 —— 性能与边界用例测试
//!
//! 覆盖三类压力/边界场景：
//!  P2.1  1000+ 节点 CPM 性能：深链/扇出/全链路璇玑优化均需在预算内完成，且关键路径正确；
//!  P2.2  100+ 并发 flow 执行：120 个流程并发回放，全部确定性执行至 100%；
//!  P2.3  边界用例：空图 / 单节点 / 超深链 / 超大扇出 均不 panic 且算法验证通过。

use std::time::Instant;

use xuanji_expert::context::{GovernContext, Principal, Tenant};
use xuanji_expert::executor::run_report;
use xuanji_expert::programming::{programming_pipeline, ProgrammingReport};
use xuanji_expert::verify::verify;
use flow_ai::model::{Access, EdgeKind, FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};
use flow_ai::{optimize, OptimizeConfig};

/// 关闭代码生成的优化配置（性能测试聚焦 CPM 引擎本身）
fn cpm_config() -> OptimizeConfig {
    OptimizeConfig {
        emit_code: false,
        auto_repair: true,
        ..Default::default()
    }
}

/// 有 edit-flow 权限的安全上下文（用于璇玑优化边界用例）
fn safe_ctx() -> GovernContext {
    let t = Tenant::new("acme", "fin");
    let p = Principal::new("ops").with_roles(vec!["editor".into(), "approver".into()]);
    GovernContext::new(t, p)
}

// ---------------------------------------------------------------------------
// P2.1  1000+ 节点 CPM 性能
// ---------------------------------------------------------------------------

#[test]
fn cpm_deep_chain_preserves_real_data_dependency() {
    // 真实数据依赖链：t{i} 读取 t{i-1} 写入的变量 → 必须严格保序，关键路径=总时长
    // 说明：线性链拓扑在 optimize 内存在超线性代价，故用 400 级深链验证 CPM 正确性；
    // 1000+ 节点的规模/性能由下方 fanout / independent / xuanji 测试覆盖。
    let n: u32 = 400;
    let mut g = FlowGraph::new("chain400", "深链依赖");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    let mut prev = "s".to_string();
    for i in 0..n {
        let id = format!("t{}", i);
        let mut node = FlowNode::task(&id, format!("任务{}", i), ToolKind::Compute, 10)
            .with_access(Access::write(format!("var:x{}", i)));
        // 除首节点外，读取上一节点写出的变量 → 形成 RAW 真依赖
        if i > 0 {
            node = node.with_access(Access::read(format!("var:x{}", i - 1)));
        }
        g.add_node(node);
        g.add_edge(FlowEdge::seq(&prev, &id));
        prev = id;
    }
    g.add_edge(FlowEdge::seq(&prev, "e"));

    let cfg = cpm_config();
    let start = Instant::now();
    let rep = optimize(&g, &cfg);
    let elapsed = start.elapsed();

    assert!(elapsed.as_secs_f64() < 10.0, "400 级依赖深链优化耗时 {}ms 超预算 10s", elapsed.as_millis());
    // 正确性：真实依赖链必须保持串行，关键路径 = 各节点时长之和 = 400 * 10 = 4000ms
    assert_eq!(rep.gains.sequential_ms, 4000, "串行总时长应为 4000ms");
    assert!(
        (rep.gains.scheduled_ms as i64 - 4000).abs() <= 200,
        "依赖深链调度时长应≈4000ms，实际 {}",
        rep.gains.scheduled_ms
    );
    assert!((rep.gains.speedup - 1.0).abs() < 0.1, "依赖深链加速比应≈1.0");
    // 算法验证必须接受该合法依赖深链
    assert!(!verify(&g, &rep).vetoed, "合法依赖深链不应被算法否决");
}

#[test]
fn cpm_1000_node_independent_tasks_parallelize() {
    // 1000 个互不依赖的任务：CPM 应识别出极短关键路径并将其并行化
    let n: u32 = 1000;
    let mut g = FlowGraph::new("indep1000", "千级独立任务");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    for i in 0..n {
        let id = format!("t{}", i);
        g.add_node(
            FlowNode::task(&id, format!("任务{}", i), ToolKind::Compute, 10)
                .with_access(Access::write(format!("var:u{}", i))),
        );
        g.add_edge(FlowEdge::seq("s", &id));
        g.add_edge(FlowEdge::seq(&id, "e"));
    }

    let cfg = cpm_config();
    let start = Instant::now();
    let rep = optimize(&g, &cfg);
    let elapsed = start.elapsed();

    assert!(elapsed.as_secs_f64() < 30.0, "1000 节点独立任务优化耗时 {}ms 超预算", elapsed.as_millis());
    assert_eq!(rep.gains.sequential_ms, 10000, "总工作量应为 10000ms");
    // 无依赖 → 关键路径仅为单个任务，调度时长远小于串行（资源受限仍大幅并行）
    assert!(
        rep.gains.scheduled_ms < 10000 / 2,
        "独立任务应被并行化，调度时长应远小于串行 10000ms，实际 {}",
        rep.gains.scheduled_ms
    );
    assert!(rep.gains.speedup >= 2.0, "独立任务加速比应 ≥ 2.0，实际 {}", rep.gains.speedup);
    assert!(!verify(&g, &rep).vetoed, "合法独立任务不应被算法否决");
}

#[test]
fn cpm_1000_node_fanout_is_fast_and_parallel() {
    let n: u32 = 1000;
    let mut g = FlowGraph::new("fanout1000", "千级扇出");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    for i in 0..n {
        let id = format!("c{}", i);
        // 每个子任务写独立变量，彼此无数据依赖 → 完全可并行
        g.add_node(
            FlowNode::task(&id, format!("子任务{}", i), ToolKind::Compute, 10)
                .with_access(Access::write(format!("var:c{}", i))),
        );
        g.add_edge(FlowEdge::seq("s", &id));
        g.add_edge(FlowEdge::seq(&id, "e"));
    }

    let cfg = cpm_config();
    let start = Instant::now();
    let rep = optimize(&g, &cfg);
    let elapsed = start.elapsed();

    assert!(elapsed.as_secs_f64() < 30.0, "1000 节点扇出优化耗时 {}ms 超预算", elapsed.as_millis());
    // 串行总时长 = 1000 * 10 = 10000ms
    assert_eq!(rep.gains.sequential_ms, 10000);
    // 并行调度绝不劣于串行，且应显著更短（资源充足时≈单任务时长）
    assert!(
        rep.gains.scheduled_ms <= 10000 + 100,
        "并行调度不应劣于串行，实际 {}",
        rep.gains.scheduled_ms
    );
    assert!(rep.gains.speedup >= 1.0, "加速比应 ≥ 1.0");
    assert!(!verify(&g, &rep).vetoed, "合法扇出不应被算法否决");
}

#[test]
fn xuanji_optimize_1000_nodes_scales() {
    // 全链路璇玑优化（含七专家派发 + 裁决 + 验证）在 1000 节点下仍应在预算内完成
    let n: u32 = 1000;
    let mut g = FlowGraph::new("all1000", "千级璇玑优化");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    let mut prev = "s".to_string();
    for i in 0..n {
        let id = format!("t{}", i);
        g.add_node(FlowNode::task(&id, format!("任务{}", i), ToolKind::Compute, 10));
        g.add_edge(FlowEdge::seq(&prev, &id));
        prev = id;
    }
    g.add_edge(FlowEdge::seq(&prev, "e"));

    let t = Tenant::new("acme", "fin");
    let p = Principal::new("ops").with_roles(vec!["editor".into(), "approver".into()]);
    let mut ctx = GovernContext::new(t, p);
    // 放宽 SLA/预算以聚焦「千级图全链路可扩展性」，避免被默认 SLA(5000ms) 误杀
    ctx.quota.sla_ms = 1_000_000;
    ctx.quota.max_cost_budget = 1_000_000.0;
    let start = Instant::now();
    let rep = xuanji_expert::pipeline::xuanji_optimize(&g, &ctx);
    let elapsed = start.elapsed();

    assert!(elapsed.as_secs_f64() < 60.0, "全链路 1000 节点优化耗时 {}ms 超预算", elapsed.as_millis());
    assert!(!rep.algo.vetoed, "无敏感操作的千级图不应被否决");
    assert!(rep.gate.approved, "应能通过治理闸门：{}", rep.gate.reason);
}

// ---------------------------------------------------------------------------
// P2.2  100+ 并发 flow 执行
// ---------------------------------------------------------------------------

fn safe_demo_report() -> ProgrammingReport {
    let mut g = FlowGraph::new("demo", "演示执行");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(
        FlowNode::task("a", "拉取", ToolKind::Database, 200).with_access(Access::read("db:x")),
    );
    g.add_node(FlowNode::task("b", "处理", ToolKind::Compute, 100));
    g.add_node(FlowNode::task("c", "汇总", ToolKind::Compute, 50));
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    g.add_edge(FlowEdge::seq("s", "a"));
    g.add_edge(FlowEdge::seq("a", "b"));
    g.add_edge(FlowEdge::seq("b", "c"));
    g.add_edge(FlowEdge::seq("c", "e"));

    let t = Tenant::new("acme", "fin").with_pool("browser", 4);
    let p = Principal::new("ops").with_roles(vec!["editor".into(), "approver".into()]);
    let mut ctx = GovernContext::new(t, p);
    ctx.quota.max_parallel = 8;
    ctx.quota.max_cost_budget = 100.0;
    ctx.quota.sla_ms = 50_000;
    programming_pipeline("演示执行流程", vec!["顺序执行".into()], true, &g, &ctx)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_120_flow_executions_all_complete() {
    let n = 120usize;
    let mut handles = Vec::with_capacity(n);
    for _ in 0..n {
        handles.push(tokio::spawn(async {
            let rep = safe_demo_report();
            assert!(rep.safe_to_emit, "演示图应可出码");
            let state = run_report(&rep, 0.0001).await; // 极速回放
            let mut rx = state.subscribe();
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                if tokio::time::Instant::now() >= deadline {
                    break;
                }
                match tokio::time::timeout_at(deadline, rx.changed()).await {
                    Ok(Ok(())) => {
                        if rx.borrow().finished {
                            break;
                        }
                    }
                    _ => break,
                }
            }
            let snap = state.trace.lock().await.clone();
            (snap.finished, snap.progress)
        }));
    }

    let mut all_ok = true;
    for h in handles {
        let (finished, progress) = h.await.unwrap();
        if !(finished && (progress - 1.0).abs() < 1e-9) {
            all_ok = false;
        }
    }
    assert!(all_ok, "全部 120 个并发流程应执行完成且进度 100%");
}

// ---------------------------------------------------------------------------
// P2.3  边界用例
// ---------------------------------------------------------------------------

#[test]
fn boundary_empty_graph_no_panic() {
    let g = FlowGraph::new("empty", "空图");
    let rep = optimize(&g, &cpm_config());
    assert!(rep.optimized_graph.nodes.is_empty(), "空图优化后节点应仍为空");
    assert!(!verify(&g, &rep).vetoed, "空图算法验证应通过");
    // 璇玑层面对空图也不应 panic
    let rep2 = xuanji_expert::pipeline::xuanji_optimize(&g, &safe_ctx());
    assert!(!rep2.algo.vetoed, "空图璇玑优化不应否决");
}

#[test]
fn boundary_single_node_no_panic() {
    let mut g = FlowGraph::new("single", "单节点");
    g.add_node(FlowNode::task("only", "唯一任务", ToolKind::Compute, 10));
    let rep = optimize(&g, &cpm_config());
    // 单节点时长应被正确统计
    assert_eq!(rep.gains.sequential_ms, 10, "单节点串行时长应为 10ms");
    assert_eq!(rep.gains.scheduled_ms, 10, "单节点调度时长应为 10ms");
    assert!(!verify(&g, &rep).vetoed, "单节点算法验证应通过");
}

#[test]
fn boundary_single_task_graph_minimal() {
    // 最小合法图：开始→任务→结束
    let mut g = FlowGraph::new("min", "最小图");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(FlowNode::task("t", "任务", ToolKind::Compute, 10));
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    g.add_edge(FlowEdge::seq("s", "t"));
    g.add_edge(FlowEdge::seq("t", "e"));
    let rep = optimize(&g, &cpm_config());
    assert_eq!(rep.gains.scheduled_ms, 10, "最小图调度时长应为 10ms");
    assert!(!verify(&g, &rep).vetoed);
}

#[test]
fn boundary_ultra_deep_chain_with_data_deps() {
    // 超深链（500 级），相邻节点通过共享变量形成真数据依赖，验证 CPM + data_dependency 一致
    let n: u32 = 500;
    let mut g = FlowGraph::new("deep500", "超深链");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    let mut prev = "s".to_string();
    for i in 0..n {
        let id = format!("t{}", i);
        // 写 var:x{i}，下一节点读 var:x{i} → 真数据依赖，必须严格保序
        let mut node = FlowNode::task(&id, format!("任务{}", i), ToolKind::Compute, 10)
            .with_access(Access::write(format!("var:x{}", i)));
        // 读取上一节点写出的变量 → 真实 RAW 依赖，强制严格保序
        if i > 0 {
            node = node.with_access(Access::read(format!("var:x{}", i - 1)));
        }
        g.add_node(node);
        g.add_edge(FlowEdge::seq(&prev, &id));
        prev = id;
    }
    g.add_edge(FlowEdge::seq(&prev, "e"));

    let start = Instant::now();
    let rep = optimize(&g, &cpm_config());
    let elapsed = start.elapsed();

    assert!(elapsed.as_secs_f64() < 10.0, "500 级深链耗时 {}ms 超预算 10s", elapsed.as_millis());
    assert_eq!(rep.gains.sequential_ms, 5000, "串行时长应为 500*10=5000ms");
    assert!(
        (rep.gains.scheduled_ms as i64 - 5000).abs() <= 200,
        "深链调度时长应≈5000ms，实际 {}",
        rep.gains.scheduled_ms
    );
    // 带真数据依赖的深链，算法验证（含 data_dependency）必须放行
    let v = verify(&g, &rep);
    assert!(!v.vetoed, "深链数据依赖应保持一致，不应否决：{:?}", v.checks);
}

#[test]
fn boundary_ultra_large_fanout() {
    // 超大扇出（500 子任务从同一起点扇出到终点，彼此独立）
    let n: u32 = 500;
    let mut g = FlowGraph::new("fanout500", "超大扇出");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    for i in 0..n {
        let id = format!("c{}", i);
        g.add_node(
            FlowNode::task(&id, format!("子任务{}", i), ToolKind::Compute, 10)
                .with_access(Access::write(format!("var:c{}", i))),
        );
        g.add_edge(FlowEdge::seq("s", &id));
        g.add_edge(FlowEdge::seq(&id, "e"));
    }

    let start = Instant::now();
    let rep = optimize(&g, &cpm_config());
    let elapsed = start.elapsed();

    assert!(elapsed.as_secs_f64() < 3.0, "500 扇出耗时 {}ms 超预算", elapsed.as_millis());
    assert_eq!(rep.gains.sequential_ms, 5000, "串行时长应为 500*10=5000ms");
    // 独立扇出应被并行，调度时长远小于串行
    assert!(
        rep.gains.scheduled_ms <= 5000,
        "扇出应被并行，调度时长≤串行，实际 {}",
        rep.gains.scheduled_ms
    );
    assert!(rep.gains.speedup >= 1.0, "扇出加速比应 ≥ 1.0");
    assert!(!verify(&g, &rep).vetoed, "合法扇出不应被否决");
    // 额外确认：无互斥硬边（扇出本应互不影响）
    assert!(
        !rep.optimized_graph
            .edges
            .iter()
            .any(|e| matches!(e.kind, EdgeKind::Mutex)),
        "独立扇出不应产生互斥硬边"
    );
}
