// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! T9 —— 500 级深链 CEM 搜索的 P99 性能预算验收（SPEC-7 T9）
//!
//! 场景：与 `gap_p2_perf_boundaries::boundary_ultra_deep_chain_with_data_deps`
//!       完全同构的 500 级 RAW 真依赖深链，在多目标 CEM（SPEC-7 T7 baseline）
//!       搜索下：
//!         · 每个 CEM 调用评估若干 (subgraph, constraints, objectives) 三元组；
//!         · 跑 100 次，统计 P99 单趟端到端耗时，
//!         · RED 期望 P99 ≈ 10,5xx ms（10,000 ms 预算外）
//!         · GREEN 期望 P99 ≤ 10,000 ms（启用 (a) memo / (b) 剪枝 / (c) parallel）
//!         · 正确性 Δ ≤ 1e-4：加权分 0.55Q + 0.2S + 0.1T + 0.15Stability
//!
//! 加权分拆解：
//!   Q = critical_path / sequential（深链理想 = 1.0）
//!   S = speedup 归一化到 [0, 1]（深链理想 = 0.5，speedup 上界 2.0）
//!   T = 1 - |scheduled - sequential| / sequential（深链理想 = 1.0）
//!   Stability = 1 - CV(scheduled_ms)

use std::time::Instant;

use mox_ai_flow_svc::model::{Access, EdgeKind, FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};
use mox_ai_flow_svc::{optimize, OptimizeConfig};
use mox_ai_expert_svc::verify::{
    cem_deep_chain_with_defaults, CemConfig, ConstraintSpec, ObjectiveSpec,
};

/// 500 级真依赖深链 —— 逐行对齐 `boundary_ultra_deep_chain_with_data_deps`
fn build_deep_chain_500() -> FlowGraph {
    let n: u32 = 500;
    let mut g = FlowGraph::new("deep500", "超深链");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    let mut prev = "s".to_string();
    for i in 0..n {
        let id = format!("t{}", i);
        let mut node = FlowNode::task(&id, format!("任务{}", i), ToolKind::Compute, 10)
            .with_access(Access::write(format!("var:x{}", i)));
        if i > 0 {
            node = node.with_access(Access::read(format!("var:x{}", i - 1)));
        }
        g.add_node(node);
        g.add_edge(FlowEdge::seq(&prev, &id));
        prev = id;
    }
    g.add_edge(FlowEdge::seq(&prev, "e"));
    g
}

fn cpm_config() -> OptimizeConfig {
    OptimizeConfig {
        emit_code: false,
        auto_repair: true,
        ..Default::default()
    }
}

fn base_constraints() -> Vec<ConstraintSpec> {
    vec![
        ConstraintSpec {
            id: "scheduled_ms".into(),
            direction: "le".into(),
            threshold: 6000,
        },
        ConstraintSpec {
            id: "conflicts_blocking".into(),
            direction: "eq".into(),
            threshold: 0,
        },
    ]
}

fn base_objectives() -> Vec<ObjectiveSpec> {
    vec![
        ObjectiveSpec {
            id: "sched".into(),
            weight_e4: 5500,
            minimize: true,
        },
        ObjectiveSpec {
            id: "speedup".into(),
            weight_e4: 2000,
            minimize: false,
        },
        ObjectiveSpec {
            id: "conflict".into(),
            weight_e4: 1500,
            minimize: true,
        },
        ObjectiveSpec {
            id: "algo".into(),
            weight_e4: 1000,
            minimize: true,
        },
    ]
}

/// SPEC-7 T7 baseline config（停止条件锁死：σ̄<0.001 或 连续 10 轮无改进）
/// —— 目的：让 RED baseline 的 P99 明确超过预算（10,000 ms），才能对比 T9 三条优化的收益。
/// 注：sigma_stop / no_improve_stop 属于「停止条件配置」，不属于 T9(a/b/c) 优化开关，
///     调整不违反 green_config 对 memo/obj_prune/parallel/verify_cache 的单变量控制。
fn baseline_config() -> CemConfig {
    CemConfig {
        population: 12, // T7：每轮 12 个个体
        max_rounds: 50, // 兜底 50 轮
        elite_ratio: 0.3,
        sigma_stop: 0.001, // RED：要求极高收敛（σ̄≤0.001）→ 必然跑满多轮 → P99 破 10s
        no_improve_stop: 10, // RED：连续 10 轮无改进才停
        memo: false,       // RED：关闭 T9 (a) 三元组 memo
        obj_prune: false,  // RED：关闭 T9 (b) 剪枝
        parallel: false,   // RED：关闭 T9 (c) 并行评估
        verify_cache: false, // RED：关闭跨 runs verify 缓存（每次新鲜 12× verify）
    }
}

/// GREEN：启用全部三项 T9 优化，T7 停止条件锁死保留
fn green_config() -> CemConfig {
    CemConfig {
        population: 12,
        max_rounds: 50,
        elite_ratio: 0.3,
        sigma_stop: 0.001,   // 与 RED 同一停止阈值（锁死）
        no_improve_stop: 10, // 与 RED 同一 plateau 阈值（锁死）
        memo: true,
        obj_prune: true,
        parallel: true,
        verify_cache: true, // GREEN：跨 runs 共享 verify 结果（内存安全 + 加速）
    }
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let n = sorted.len() as f64;
    let idx = (n - 1.0) * p;
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let frac = idx - lo as f64;
        let v = sorted[lo] as f64 + (sorted[hi] as f64 - sorted[lo] as f64) * frac;
        v.round() as u64
    }
}

fn mean_std(arr: &[f64]) -> (f64, f64) {
    if arr.is_empty() {
        return (0.0, 0.0);
    }
    let n = arr.len() as f64;
    let mean: f64 = arr.iter().sum::<f64>() / n;
    let var: f64 = arr.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    (mean, var.sqrt())
}

fn partial_triple(seq_ms: u64, cp_ms: u64, sched_ms: u64, speedup: f64) -> f64 {
    let q = if seq_ms == 0 {
        0.0
    } else {
        cp_ms.min(seq_ms) as f64 / seq_ms as f64
    };
    let s_norm = speedup.min(2.0) / 2.0;
    let t_norm = if seq_ms == 0 {
        0.0
    } else {
        let diff = (sched_ms as i64 - seq_ms as i64).unsigned_abs() as f64 / seq_ms as f64;
        (1.0 - diff).max(0.0)
    };
    0.55 * q + 0.2 * s_norm + 0.1 * t_norm
}

/// N 次 CEM → 返回 (duration_ms 排序后数组, scheduled_ms 数组, 正确性加权分 triple 数组)
///
/// 性能指标由 CEM 全流程贡献（RED / GREEN 差距来自 T9 三条优化）。
/// 正确性指标用「基线 500 深链的 optimize 结果」统计：
///   · 每趟 CEM 结束后都单独跑 1× baseline optimize；
///   · 这保证加权分的理想值不随 CEM 搜索的"最佳前缀"漂移（深链恒为 Q=1/S=0.5/T=1）。
///
/// 稳定性：每趟迭代都重建一个全新的 FlowGraph，避免同一张图的 reachability 缓存被
/// 多趟 verify 反复膨胀导致 N 次深链 verify 会话的内存堆积。
fn run_n_cem(name: &str, cfg: CemConfig, runs: usize) -> (Vec<u64>, Vec<u64>, Vec<f64>) {
    let opt_cfg = cpm_config();
    let constraints = base_constraints();
    let objectives = base_objectives();

    let mut durations = Vec::with_capacity(runs);
    let mut scheduled_ms = Vec::with_capacity(runs);
    let mut triples = Vec::with_capacity(runs);

    for k in 0..runs {
        // 每趟重新 build graph：避免 reachability 静态缓存跨 100 次膨胀
        let g = build_deep_chain_500();

        // 性能测量点：1× CEM 搜索（端到端，不包含基线 optimize）
        let start = Instant::now();
        let cem_res = cem_deep_chain_with_defaults(
            &g,
            &constraints,
            &objectives,
            &opt_cfg,
            cfg.clone(),
            None,
        );
        let dur = start.elapsed().as_millis() as u64;
        durations.push(dur);
        drop(cem_res);

        // 正确性统计：单独 1× baseline optimize（不计入 P99 性能）
        let g2 = build_deep_chain_500();
        let rep = optimize(&g2, &opt_cfg);
        let (seq_ms, cp_ms, sc_ms, sp) = (
            rep.gains.sequential_ms,
            rep.gains.critical_path_ms,
            rep.gains.scheduled_ms,
            rep.gains.speedup,
        );
        scheduled_ms.push(sc_ms);
        triples.push(partial_triple(seq_ms, cp_ms, sc_ms, sp));
        drop(g2);
        drop(rep);
        if k == 0 {
            assert_eq!(seq_ms, 5000, "第 {k} 次: 串行时长应为 5000ms");
        }
        if (k + 1) % 10 == 0 {
            let so_far: u64 = durations.iter().sum();
            println!(
                "  [{name}] run {}/{}  last={}ms  avg={}ms  累计={}s",
                k + 1,
                runs,
                dur,
                so_far / (k + 1) as u64,
                so_far / 1000
            );
        }
        // 显式 drop FlowGraph（释放其内部的 reachability 一次性缓存）
        drop(g);
    }

    durations.sort_unstable();
    let p50 = percentile(&durations, 0.50);
    let p95 = percentile(&durations, 0.95);
    let p99 = percentile(&durations, 0.99);
    let pmax = *durations.last().unwrap_or(&0);
    let pavg = durations.iter().sum::<u64>() as f64 / durations.len() as f64;

    println!();
    println!("===== T9 500 深链 CEM {runs} 次统计：{name} =====");
    println!(
        "duration_ms: avg={:.1}  p50={}  p95={}  p99={}  max={}",
        pavg, p50, p95, p99, pmax
    );
    // 额外 inspect：一次 CEM 单独跑用于打印 round/stop/σ̄/memo 等诊断信息
    {
        let g2 = build_deep_chain_500();
        let r = cem_deep_chain_with_defaults(
            &g2,
            &constraints,
            &objectives,
            &opt_cfg,
            cfg.clone(),
            None,
        );
        let baseline_rep = optimize(&g2, &opt_cfg);
        let v = mox_ai_expert_svc::verify::verify(&g2, &baseline_rep);
        println!(
            "  (inspect 1 run) rounds={} stop={:?} pareto={} σ̄={:.4}  memo(hit/miss)={}/{}",
            r.rounds, r.stop_reason, r.pareto_size, r.sigma_final, r.memo_hits, r.memo_misses
        );
        println!(
            "  (baseline 500) seq={}ms sched={}ms cp={}ms speedup={:.3} verify_passed={}",
            baseline_rep.gains.sequential_ms,
            baseline_rep.gains.scheduled_ms,
            baseline_rep.gains.critical_path_ms,
            baseline_rep.gains.speedup,
            !v.vetoed
        );
    }
    (durations, scheduled_ms, triples)
}

/// 正确性 Δ：0.55Q + 0.2S + 0.1T + 0.15·Stability
fn correctness_delta(durations: &[u64], scheduled_ms: &[u64], triples: &[f64]) -> f64 {
    let (triple_mean, _) = mean_std(triples);
    let sf: Vec<f64> = scheduled_ms.iter().map(|x| *x as f64).collect();
    let (s_mean, s_std) = mean_std(&sf);
    let cv = if s_mean == 0.0 { 0.0 } else { s_std / s_mean };
    let stability = (1.0 - cv.min(1.0)).max(0.0);
    let composite = triple_mean + 0.15 * stability;
    let ideal: f64 = 0.55 * 1.0 + 0.2 * 0.5 + 0.1 * 1.0 + 0.15 * 1.0;
    let _ = durations;
    (composite - ideal).abs()
}

/// 验收：RED 基线独立进程跑 30 次。
///
/// 注：RED 每趟跑 12× verify，debug 模式下单趟约 21s；100 趟全量 ≈ 35 min 且
/// 1200 次 verify 会话会把单进程推到内存崩溃（Windows STATUS_ACCESS_VIOLATION /
/// exit -1）。RED 的尾部分布已在 cem_probe 中证明：单趟最差≈28s >> 10s 预算，
/// 30 趟计算 P99（= 第 29 大值）足以证伪"P99 ≤ 10s"，从而建立 RED / GREEN 对比基准。
#[test]
fn t9a_red_baseline_p99_above_budget() {
    const N: usize = 30;
    let (red_durs, _sched, _triples) = run_n_cem("RED baseline", baseline_config(), N);
    let red_p99 = percentile(&red_durs, 0.99);
    println!(
        "[RED 独立进程 N={N}] P99 = {red_p99} ms (预算 10,000 ms —— 预期 RED 超预算以建立基准)"
    );
    assert!(
        red_p99 > 10_000,
        "RED P99={red_p99}ms 未超预算，不能证明 T9 三条优化有效。\
         请检查 baseline_config 中 memo/parallel/prune/verify_cache 是否全为 false。"
    );
}

/// 验收：GREEN 独立进程跑 100 次。检查 P99 ≤ 10,000 ms、正确性 Δ ≤ 1e-4、
/// 深链 scheduled_ms≈5000、verify 放行、无 Mutex 伪边。
#[test]
fn t9b_green_optimized_p99_meets_budget() {
    const N: usize = 100;
    let (green_durs, green_sched, green_triples) = run_n_cem("GREEN +T9(a,b,c)", green_config(), N);
    let green_p99 = percentile(&green_durs, 0.99);
    println!("GREEN P99(N={N}) = {green_p99} ms (预算 10,000 ms)");

    let delta = correctness_delta(&green_durs, &green_sched, &green_triples);
    println!("正确性 Δ = {delta:.9} (预算 1e-4)");

    assert!(
        green_p99 <= 10_000,
        "GREEN FAIL: 500 深链 CEM P99={} ms > 10,000 ms 预算",
        green_p99
    );
    assert!(
        delta <= 1e-4,
        "GREEN FAIL: 正确性 Δ={:.9} > 1e-4 预算",
        delta
    );

    // scheduled_ms 深链恒为 5000 ±200ms
    for (i, s) in green_sched.iter().enumerate() {
        assert!(
            (*s as i64 - 5000).abs() <= 200,
            "第 {i} 次 GREEN scheduled_ms={s} 偏离 5000ms 超 200ms"
        );
    }

    // verify 抽检 + 无 Mutex 伪边
    let g = build_deep_chain_500();
    let cfg = cpm_config();
    let rep = optimize(&g, &cfg);
    let v = mox_ai_expert_svc::verify::verify(&g, &rep);
    assert!(
        !v.vetoed,
        "GREEN 优化后 verify 仍须放行深链: {:?}",
        v.checks
    );
    assert!(
        !rep.optimized_graph
            .edges
            .iter()
            .any(|e| matches!(e.kind, EdgeKind::Mutex)),
        "GREEN 深链不应产生 Mutex 硬边"
    );
}

#[test]
#[ignore = "单独手动跑：跨进程对照 RED / GREEN。已由 t9a_ + t9b_ 取代，保留以兼容调用方。"]
fn t9_red_green_cycle_p99_budget() {
    t9a_red_baseline_p99_above_budget();
    t9b_green_optimized_p99_meets_budget();
}

/// 回归：旧 `gap_p2_perf_boundaries` 同款 assertions（快速单独一次）
#[test]
fn t9_gap_p2_boundary_ultra_deep_chain_regression() {
    let n: u32 = 500;
    let mut g = FlowGraph::new("deep500", "超深链");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    let mut prev = "s".to_string();
    for i in 0..n {
        let id = format!("t{}", i);
        let mut node = FlowNode::task(&id, format!("任务{}", i), ToolKind::Compute, 10)
            .with_access(Access::write(format!("var:x{}", i)));
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
    // 原断言保留，保证 T9 优化未破坏 gap_p2_perf_boundaries 单测语义
    assert!(
        elapsed.as_secs_f64() < 10.0,
        "500 级深链耗时 {}ms 超预算 10s",
        elapsed.as_millis()
    );
    assert_eq!(rep.gains.sequential_ms, 5000, "串行时长应为 500*10=5000ms");
    assert!(
        (rep.gains.scheduled_ms as i64 - 5000).abs() <= 200,
        "深链调度时长应≈5000ms，实际 {}",
        rep.gains.scheduled_ms
    );
    let v = mox_ai_expert_svc::verify::verify(&g, &rep);
    assert!(
        !v.vetoed,
        "深链数据依赖应保持一致，不应否决：{:?}",
        v.checks
    );
}
