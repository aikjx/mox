// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 四维度综合性能基准测试（Round 7）
//!
//! 维度 1：匹配器 Token 缓存（Before vs After）
//! 维度 2：Fusion 7 策略 × 专家数
//! 维度 3：DAG 执行（串行拓扑 vs 并行就绪检测）
//! 维度 4：端到端（MockExecutor + DagEngineImpl）
//!
//! 运行：cargo test -p mox-alliance-scheduler-core --test bench_alliance -- --nocapture

use std::time::{Duration, Instant};

use mox_alliance_common_proto::{
    AllianceMode, Capability, CollaborationPlan, Expert, ExpertStatus, FusionStrategy, Node,
    NodeStatus, Task, TaskStatus,
};
use mox_alliance_core::dag;
use mox_alliance_core::fusion::FusionEngine;
use mox_alliance_executor_proto::DagEngine;
use mox_alliance_scheduler_core::matching::{
    description_overlap, tokenize, ExpertTokenCache,
};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

// ─── 统计辅助函数 ───────────────────────────────────────────────────────────

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p / 100.0 * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn std_dev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let m = mean(values);
    let variance = values.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    variance.sqrt()
}

fn micros(d: Duration) -> f64 {
    d.as_secs_f64() * 1_000_000.0
}

fn millis(d: Duration) -> f64 {
    d.as_secs_f64() * 1_000.0
}

// ─── 合成数据生成 ───────────────────────────────────────────────────────────

const DOMAIN_POOL: &[&str] = &[
    "code", "mathematics", "medical", "finance", "creative",
    "vision", "translation", "research", "architecture", "law",
];

const CAPABILITY_POOL: &[&str] = &[
    "代码生成", "算法分析", "数学证明", "疾病诊断", "财务分析",
    "文案创作", "图像识别", "中英翻译", "文献综述", "系统架构",
    "合同审查", "数据建模", "性能优化", "测试用例", "API设计",
];

fn make_expert(id: usize, desc_len: usize) -> Expert {
    let domain = DOMAIN_POOL[id % DOMAIN_POOL.len()];
    let cap1 = CAPABILITY_POOL[id % CAPABILITY_POOL.len()];
    let cap2 = CAPABILITY_POOL[(id + 3) % CAPABILITY_POOL.len()];

    // 生成指定长度的中文描述（按字符数）
    let base_desc = format!(
        "专业的{domain}领域专家，擅长{cap1}与{cap2}。具备丰富的项目经验和深厚的理论功底，能够独立完成复杂任务并提供高质量的解决方案。"
    );
    let mut description = String::with_capacity(desc_len * 3);
    while description.chars().count() < desc_len {
        description.push_str(&base_desc);
    }
    // 按字符边界截断
    let char_count = description.chars().count();
    if char_count > desc_len {
        let byte_pos = description
            .char_indices()
            .nth(desc_len)
            .map(|(pos, _)| pos)
            .unwrap_or(description.len());
        description.truncate(byte_pos);
    }

    let mut e = Expert::new_system(
        format!("专家-{id}-{domain}"),
        description,
    );
    e.expert_id = format!("expert-{id:04}");
    e.domains = vec![domain.to_string(), DOMAIN_POOL[(id + 1) % DOMAIN_POOL.len()].to_string()];
    e.capabilities = vec![
        Capability {
            capability_id: format!("cap-{id}-1"),
            name: cap1.to_string(),
            description: format!("{cap1}能力描述"),
            domain: domain.to_string(),
            version: "1.0.0".to_string(),
        },
        Capability {
            capability_id: format!("cap-{id}-2"),
            name: cap2.to_string(),
            description: format!("{cap2}能力描述"),
            domain: domain.to_string(),
            version: "1.0.0".to_string(),
        },
    ];
    e.status = ExpertStatus::Active;
    e
}

fn make_query_tokens(desc_len: usize) -> Vec<String> {
    let text = match desc_len {
        0..=30 => "用Rust实现一个高性能的Web服务登录接口，要求支持JWT认证和限流",
        31..=150 => "请帮我设计一个分布式微服务架构，需要包含服务注册发现、配置中心、API网关、链路追踪、熔断降级等核心组件。技术栈优先考虑Rust和Go，数据库使用PostgreSQL，缓存使用Redis。需要给出详细的架构图和部署方案。",
        _ => "这是一个复杂的综合任务，需要多位专家协作完成。首先，我们需要对现有系统进行全面的性能分析，找出瓶颈所在。然后，根据分析结果设计优化方案，包括数据库索引优化、缓存策略调整、代码层面的性能改进等。接下来，需要实现这些优化方案并进行充分的测试验证。同时，还需要考虑系统的可扩展性和可维护性，确保优化后的系统能够支撑未来的业务增长。最后，需要编写详细的技术文档和运维手册，方便团队后续的维护和迭代。整个过程需要严格遵循软件工程最佳实践，确保代码质量和系统稳定性。",
    };
    tokenize(text)
}

fn make_dag_nodes(count: usize) -> Vec<Node> {
    let task_id = Uuid::new_v4();
    let mut nodes = Vec::with_capacity(count);
    for i in 0..count {
        // 每个节点依赖 0-3 个前面的节点（确保无环）
        let mut deps = Vec::new();
        if i > 0 {
            let num_deps = (i % 3).min(i);
            for d in 0..num_deps {
                let dep_idx = i - 1 - d;
                deps.push(format!("node-{dep_idx:03}"));
            }
        }
        nodes.push(Node {
            node_id: format!("node-{i:03}"),
            task_id,
            expert_id: format!("expert-{i:03}"),
            module_id: None,
            name: format!("Node {i}"),
            description: None,
            status: NodeStatus::Pending,
            retry_count: 0,
            dependencies: deps,
            input_refs: vec![],
            output_ref: None,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            error_message: None,
        });
    }
    nodes
}

fn make_fusion_values(count: usize) -> Vec<(f64, f64)> {
    (0..count)
        .map(|i| {
            let score = 50.0 + (i as f64 * 7.3) % 50.0;
            let conf = 0.3 + (i as f64 * 0.17) % 0.7;
            (score, conf)
        })
        .collect()
}

// ─── 维度 1：匹配器基准 ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct MatcherResult {
    experts: usize,
    desc_len: &'static str,
    p50_us: f64,
    p95_us: f64,
    p99_us: f64,
    ops_per_sec: f64,
}

fn run_matcher_bench(
    experts: &[Expert],
    query_tokens: &[String],
    use_cache: bool,
    iterations: usize,
) -> MatcherResult {
    let cache = ExpertTokenCache::new();
    if use_cache {
        // 预热缓存
        for e in experts {
            cache.get_or_compute(e);
        }
    }

    let mut latencies: Vec<f64> = Vec::with_capacity(iterations);
    let total_start = Instant::now();

    for _ in 0..iterations {
        let iter_start = Instant::now();
        for e in experts {
            let cache_ref = if use_cache { Some(&cache) } else { None };
            let _ = description_overlap(e, query_tokens, cache_ref);
        }
        latencies.push(micros(iter_start.elapsed()));
    }

    let total_elapsed = total_start.elapsed();
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let total_ops = iterations * experts.len();
    let ops_per_sec = total_ops as f64 / total_elapsed.as_secs_f64();

    MatcherResult {
        experts: experts.len(),
        desc_len: "", // filled by caller
        p50_us: percentile(&latencies, 50.0),
        p95_us: percentile(&latencies, 95.0),
        p99_us: percentile(&latencies, 99.0),
        ops_per_sec,
    }
}

// ─── 维度 2：Fusion 基准 ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct FusionResult {
    strategy: &'static str,
    experts: usize,
    avg_us: f64,
    std_us: f64,
    result_std: f64,
}

fn run_fusion_bench(
    strategy: FusionStrategy,
    strategy_name: &'static str,
    expert_count: usize,
    iterations: usize,
) -> FusionResult {
    let engine = FusionEngine::from_strategy(strategy);
    let values = make_fusion_values(expert_count);

    let mut latencies: Vec<f64> = Vec::with_capacity(iterations);
    let mut results: Vec<f64> = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();
        if let Ok(r) = engine.fuse_scalar(&values) {
            results.push(r);
        }
        latencies.push(micros(start.elapsed()));
    }

    FusionResult {
        strategy: strategy_name,
        experts: expert_count,
        avg_us: mean(&latencies),
        std_us: std_dev(&latencies),
        result_std: std_dev(&results),
    }
}

// ─── 维度 3：DAG 基准 ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct DagResult {
    nodes: usize,
    mode: &'static str,
    avg_us: f64,
}

fn run_dag_bench(nodes: &[Node], mode: &'static str, iterations: usize) -> DagResult {
    let mut latencies: Vec<f64> = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();
        match mode {
            "serial" => {
                // 纯串行拓扑排序
                let _ = dag::topological_sort(nodes).unwrap();
            }
            "parallel" => {
                // 并行就绪节点检测：模拟多轮调度，每轮检测就绪节点
                let mut working: Vec<Node> = nodes.to_vec();
                let mut completed = 0usize;
                while completed < working.len() {
                    let ready = dag::find_ready_nodes(&working);
                    if ready.is_empty() {
                        break;
                    }
                    // 标记就绪节点为已完成（模拟并行执行）
                    for rid in &ready {
                        if let Some(n) = working.iter_mut().find(|n| n.node_id == *rid) {
                            n.status = NodeStatus::Completed;
                            completed += 1;
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
        latencies.push(micros(start.elapsed()));
    }

    DagResult {
        nodes: nodes.len(),
        mode,
        avg_us: mean(&latencies),
    }
}

// ─── 维度 4：端到端基准 ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct E2EResult {
    nodes: usize,
    p50_ms: f64,
    p95_ms: f64,
    success_rate: f64,
}

async fn run_e2e_bench(node_count: usize, iterations: usize) -> E2EResult {
    use mox_alliance_executor_core::{DagEngineImpl, MockExecutorConfig, MockNodeExecutor};
    use mox_alliance_executor_proto::{ExecutionOptions, ExecutorConfig};

    let executor_config = ExecutorConfig {
        poll_interval_ms: 1, // 加速测试
        ..Default::default()
    };
    let mock_config = MockExecutorConfig {
        delay_ms: 1, // 极小延迟加速测试
        success_rate: 1.0,
        generate_output: true,
    };
    let executor = Arc::new(MockNodeExecutor::new(mock_config));
    let engine = DagEngineImpl::spawn(executor_config, executor);

    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let mut latencies: Vec<f64> = Vec::with_capacity(iterations);
    let mut successes = 0usize;

    for i in 0..iterations {
        let task_id = Uuid::new_v4();
        let nodes = make_dag_nodes(node_count);
        let plan = CollaborationPlan {
            task_id,
            mode: AllianceMode::Parallel,
            fusion_strategy: FusionStrategy::Weighted,
            nodes: nodes.clone(),
            version: 1,
            created_at: chrono::Utc::now(),
        };
        let task = Task {
            task_id,
            tenant_id,
            user_id,
            title: format!("bench-task-{i}"),
            description: "benchmark".to_string(),
            task_type: "benchmark".to_string(),
            status: TaskStatus::Pending,
            priority: Default::default(),
            progress: 0.0,
            current_node_id: None,
            mode: AllianceMode::Parallel,
            fusion_strategy: FusionStrategy::Weighted,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            duration_ms: None,
            fusion_result: None,
        };
        let options = ExecutionOptions::default();

        let start = Instant::now();
        match engine.start_execution(&task, plan, options).await {
            Ok(_) => {
                // 轮询直到完成或超时
                let mut completed = false;
                for _ in 0..5000 {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    if let Ok(status) = engine.get_execution_status(task_id, tenant_id).await {
                        if status.progress >= 1.0 {
                            completed = true;
                            // 确认最终状态
                            if let Ok(final_status) = engine.get_execution_status(task_id, tenant_id).await {
                                if final_status.completed_nodes == node_count {
                                    successes += 1;
                                }
                            }
                            break;
                        }
                    }
                }
                if !completed {
                    eprintln!("  [WARN] E2E task {task_id} timed out after 5s");
                }
            }
            Err(e) => {
                eprintln!("  [ERROR] E2E start failed: {e}");
            }
        }
        latencies.push(millis(start.elapsed()));
    }

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    E2EResult {
        nodes: node_count,
        p50_ms: percentile(&latencies, 50.0),
        p95_ms: percentile(&latencies, 95.0),
        success_rate: successes as f64 / iterations as f64,
    }
}

// ─── 主测试函数 ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn bench_alliance_round7() {
    println!("=== 三联盟系统四维度性能基准测试 (Round 7) ===");
    println!("开始时间: {:?}", std::time::SystemTime::now());

    let bench_start = Instant::now();

    // ── 环境信息 ──
    let rust_version = "rustc 1.98.0-nightly (54333ff07 2026-05-22)";
    let cpu = std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".to_string());
    let date = "2026-09-01";

    println!("\n环境: Rust={rust_version}, CPU={cpu}, OS=Windows, Date={date}");

    // ═══════════════════════════════════════════════════════════════════════
    // 维度 1：匹配器基准
    // ═══════════════════════════════════════════════════════════════════════
    println!("\n--- 维度 1：匹配器 Token 缓存基准 ---");

    let expert_counts = [10usize, 50, 100, 200];
    let desc_lengths = [
        ("short", 20usize),
        ("medium", 100usize),
        ("long", 500usize),
    ];
    let matcher_iterations = 100;

    let mut before_results: Vec<Value> = Vec::new();
    let mut after_results: Vec<Value> = Vec::new();

    for &num_experts in &expert_counts {
        let experts: Vec<Expert> = (0..num_experts).map(|i| make_expert(i, 100)).collect();
        for (len_name, len_val) in &desc_lengths {
            let query_tokens = make_query_tokens(*len_val);

            // Before: 无缓存
            let mut r = run_matcher_bench(&experts, &query_tokens, false, matcher_iterations);
            r.desc_len = len_name;
            println!(
                "  Before  experts={:>3} len={:<6} p50={:>8.1}us p95={:>8.1}us p99={:>8.1}us ops={:>10.0}/s",
                num_experts, len_name, r.p50_us, r.p95_us, r.p99_us, r.ops_per_sec
            );
            before_results.push(json!({
                "experts": r.experts,
                "desc_len": r.desc_len,
                "p50_us": r.p50_us,
                "p95_us": r.p95_us,
                "p99_us": r.p99_us,
                "ops_per_sec": r.ops_per_sec,
            }));

            // After: 有缓存
            let mut r = run_matcher_bench(&experts, &query_tokens, true, matcher_iterations);
            r.desc_len = len_name;
            println!(
                "  After   experts={:>3} len={:<6} p50={:>8.1}us p95={:>8.1}us p99={:>8.1}us ops={:>10.0}/s",
                num_experts, len_name, r.p50_us, r.p95_us, r.p99_us, r.ops_per_sec
            );
            after_results.push(json!({
                "experts": r.experts,
                "desc_len": r.desc_len,
                "p50_us": r.p50_us,
                "p95_us": r.p95_us,
                "p99_us": r.p99_us,
                "ops_per_sec": r.ops_per_sec,
            }));
        }
    }

    let matcher_data_points = before_results.len() + after_results.len();
    println!(
        "  维度1完成: {} 组合 × {} 迭代 = {} 数据点",
        expert_counts.len() * desc_lengths.len() * 2,
        matcher_iterations,
        matcher_data_points
    );

    // ═══════════════════════════════════════════════════════════════════════
    // 维度 2：Fusion 基准
    // ═══════════════════════════════════════════════════════════════════════
    println!("\n--- 维度 2：Fusion 7 策略基准 ---");

    let strategies: [(FusionStrategy, &'static str); 7] = [
        (FusionStrategy::Weighted, "Weighted"),
        (FusionStrategy::ConfidenceWeighted, "ConfidenceWeighted"),
        (FusionStrategy::BestOf, "BestOf"),
        (FusionStrategy::Stacking, "Stacking"),
        (FusionStrategy::MapReduce, "MapReduce"),
        (FusionStrategy::Iterative, "Iterative"),
        (FusionStrategy::Debate, "Debate"),
    ];
    let fusion_expert_counts = [1usize, 3, 5, 10];
    let fusion_iterations = 1000;

    let mut fusion_results: Vec<Value> = Vec::new();

    for (strategy, name) in &strategies {
        for &num_experts in &fusion_expert_counts {
            let r = run_fusion_bench(*strategy, name, num_experts, fusion_iterations);
            println!(
                "  {:<18} experts={:>2} avg={:>8.3}us std={:>7.3}us result_std={:.6}",
                name, num_experts, r.avg_us, r.std_us, r.result_std
            );
            fusion_results.push(json!({
                "strategy": r.strategy,
                "experts": r.experts,
                "avg_us": r.avg_us,
                "std_us": r.std_us,
                "result_std": r.result_std,
            }));
        }
    }

    println!(
        "  维度2完成: {} 策略 × {} 专家数 × {} 迭代 = {} 数据点",
        strategies.len(),
        fusion_expert_counts.len(),
        fusion_iterations,
        fusion_results.len()
    );

    // ═══════════════════════════════════════════════════════════════════════
    // 维度 3：DAG 基准
    // ═══════════════════════════════════════════════════════════════════════
    println!("\n--- 维度 3：DAG 执行基准 ---");

    let dag_node_counts = [1usize, 5, 10, 20];
    let dag_iterations = 500;

    let mut dag_results: Vec<Value> = Vec::new();

    for &num_nodes in &dag_node_counts {
        let nodes = make_dag_nodes(num_nodes);

        // 串行拓扑排序
        let r = run_dag_bench(&nodes, "serial", dag_iterations);
        println!(
            "  Serial   nodes={:>2} avg={:>8.2}us",
            num_nodes, r.avg_us
        );
        dag_results.push(json!({
            "nodes": r.nodes,
            "mode": r.mode,
            "avg_us": r.avg_us,
        }));

        // 并行就绪检测
        let r = run_dag_bench(&nodes, "parallel", dag_iterations);
        println!(
            "  Parallel nodes={:>2} avg={:>8.2}us",
            num_nodes, r.avg_us
        );
        dag_results.push(json!({
            "nodes": r.nodes,
            "mode": r.mode,
            "avg_us": r.avg_us,
        }));
    }

    println!(
        "  维度3完成: {} 节点数 × 2 模式 × {} 迭代 = {} 数据点",
        dag_node_counts.len(),
        dag_iterations,
        dag_results.len()
    );

    // ═══════════════════════════════════════════════════════════════════════
    // 维度 4：端到端基准
    // ═══════════════════════════════════════════════════════════════════════
    println!("\n--- 维度 4：端到端基准 (MockExecutor + DagEngineImpl) ---");

    let e2e_node_counts = [3usize, 5, 10];
    let e2e_iterations = 20;

    let mut e2e_results: Vec<Value> = Vec::new();
    let e2e_skipped: Option<String> = None;

    for &num_nodes in &e2e_node_counts {
        println!("  运行 nodes={num_nodes} ({e2e_iterations} iterations)...");
        let r = run_e2e_bench(num_nodes, e2e_iterations).await;
        println!(
            "  nodes={:>2} p50={:>8.2}ms p95={:>8.2}ms success={:.0}%",
            num_nodes, r.p50_ms, r.p95_ms, r.success_rate * 100.0
        );
        e2e_results.push(json!({
            "nodes": r.nodes,
            "p50_ms": r.p50_ms,
            "p95_ms": r.p95_ms,
            "success_rate": r.success_rate,
        }));
    }

    if e2e_skipped.is_none() {
        println!(
            "  维度4完成: {} 节点数 × {} 迭代 = {} 数据点",
            e2e_node_counts.len(),
            e2e_iterations,
            e2e_results.len()
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 组装 JSON 并写入文件
    // ═══════════════════════════════════════════════════════════════════════

    let total_elapsed = bench_start.elapsed();

    let mut e2e_json = json!({
        "results": e2e_results,
    });
    if let Some(reason) = e2e_skipped {
        e2e_json["e2e_skipped_reason"] = json!(reason);
    }

    let output = json!({
        "environment": {
            "rust_version": rust_version,
            "os": "Windows",
            "cpu": cpu,
            "date": date,
        },
        "matcher_benchmark": {
            "before_no_cache": before_results,
            "after_with_cache": after_results,
            "iterations_per_combo": matcher_iterations,
            "expert_counts": expert_counts,
            "desc_lengths": ["short", "medium", "long"],
        },
        "fusion_benchmark": {
            "results": fusion_results,
            "iterations_per_combo": fusion_iterations,
            "strategies": ["Weighted", "ConfidenceWeighted", "BestOf", "Stacking", "MapReduce", "Iterative", "Debate"],
            "expert_counts": fusion_expert_counts,
        },
        "dag_benchmark": {
            "results": dag_results,
            "iterations_per_combo": dag_iterations,
            "node_counts": dag_node_counts,
        },
        "e2e_benchmark": e2e_json,
        "summary": {
            "total_duration_sec": total_elapsed.as_secs_f64(),
            "matcher_data_points": matcher_data_points,
            "fusion_data_points": fusion_results.len(),
            "dag_data_points": dag_results.len(),
            "e2e_data_points": e2e_results.len(),
            "total_data_points": matcher_data_points + fusion_results.len() + dag_results.len() + e2e_results.len(),
        },
    });

    let output_path = r"D:\a10\aikjx\gitcode\infotopograph\docs\bench_results_round7.json";
    std::fs::create_dir_all(std::path::Path::new(output_path).parent().unwrap()).unwrap();
    let json_str = serde_json::to_string_pretty(&output).unwrap();
    std::fs::write(output_path, &json_str).unwrap();

    println!("\n=== 基准测试完成 ===");
    println!("总耗时: {:.2}s", total_elapsed.as_secs_f64());
    println!("结果已写入: {output_path}");
    println!(
        "数据点统计: matcher={}, fusion={}, dag={}, e2e={}, total={}",
        matcher_data_points,
        fusion_results.len(),
        dag_results.len(),
        e2e_results.len(),
        matcher_data_points + fusion_results.len() + dag_results.len() + e2e_results.len()
    );
}
