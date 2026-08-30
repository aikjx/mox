// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/mox/mox

//! 性能基准测试 (Performance Benchmark Tests)
//!
//! 测试场景覆盖：
//! - 顶点写入吞吐量：单线程/多线程批量写入
//! - 边写入吞吐量：单线程/多线程批量写入
//! - 点查延迟：P50/P95/P99 延迟分布
//! - 遍历查询性能：1跳/2跳/3跳查询延迟
//! - 图算法性能：PageRank、社区发现、最短路径执行时间
//! - 批量导入性能：万级顶点/边的导入速度
//! - 内存占用：不同数据规模下的内存使用
//!
//! 测试说明：
//! 本测试套件使用 Instant 精确计时 + 百分位统计的方式进行性能基准测试。
//! 所有测试可以通过 `cargo test` 运行。
//! 性能阈值在 debug 和 release 模式下不同，以确保测试在两种模式下都能通过。
//!
//! 运行方式：
//! - cargo test -p mox-kg-storage-svc --test t_perf_bench -- --nocapture
//! - cargo test -p mox-kg-storage-svc --release --test t_perf_bench -- --nocapture

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mox_kg_storage_svc::graph_codec::{self, PropValue};
use mox_kg_storage_svc::storage_api::{Direction, LruCache};
use mox_kg_storage_svc::storage_server::StorageServer;

// ============================================================================
// 性能测试工具函数
// ============================================================================

fn test_addrs() -> Vec<String> {
    vec![
        "127.0.0.1:9401".into(),
        "127.0.0.1:9402".into(),
        "127.0.0.1:9403".into(),
    ]
}

fn new_server(shards: u16) -> StorageServer {
    StorageServer::start_cluster(shards, &test_addrs(), None).expect("start cluster")
}

fn prop(k: &str, v: &str) -> BTreeMap<String, PropValue> {
    let mut m = BTreeMap::new();
    m.insert(k.to_string(), PropValue::from_str(v));
    m
}

fn props(pairs: &[(&str, &str)]) -> BTreeMap<String, PropValue> {
    let mut m = BTreeMap::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), PropValue::from_str(v));
    }
    m
}

/// 百分位统计结果
#[derive(Debug, Clone)]
struct LatencyStats {
    count: usize,
    total: Duration,
    p50: Duration,
    p90: Duration,
    p95: Duration,
    p99: Duration,
    p999: Duration,
    min: Duration,
    max: Duration,
    mean: Duration,
    ops_per_sec: f64,
}

/// 计算延迟统计（P50/P90/P95/P99/P999）
fn compute_latency_stats(latencies: &mut [Duration], total_time: Duration) -> LatencyStats {
    latencies.sort();
    let count = latencies.len();
    let total: Duration = latencies.iter().sum();

    let pct = |p: f64| -> Duration {
        if count == 0 {
            return Duration::ZERO;
        }
        let idx = ((count as f64) * p / 100.0) as usize;
        latencies[idx.min(count.saturating_sub(1))]
    };

    LatencyStats {
        count,
        total,
        p50: pct(50.0),
        p90: pct(90.0),
        p95: pct(95.0),
        p99: pct(99.0),
        p999: pct(99.9),
        min: if count > 0 { latencies[0] } else { Duration::ZERO },
        max: if count > 0 { latencies[count - 1] } else { Duration::ZERO },
        mean: if count > 0 { total / count as u32 } else { Duration::ZERO },
        ops_per_sec: count as f64 / total_time.as_secs_f64(),
    }
}

impl LatencyStats {
    fn print(&self, label: &str) {
        eprintln!(
            "[{}] count={} ops/s={:.0} | mean={:?} p50={:?} p90={:?} p95={:?} p99={:?} p999={:?} | min={:?} max={:?}",
            label,
            self.count,
            self.ops_per_sec,
            self.mean,
            self.p50,
            self.p90,
            self.p95,
            self.p99,
            self.p999,
            self.min,
            self.max
        );
    }
}

/// 吞吐量基准配置
struct ThroughputBench {
    name: &'static str,
    operations: u64,
    warmup: u64,
}

/// 运行吞吐量测试
fn run_throughput_bench<F>(bench: &ThroughputBench, mut op: F) -> (Duration, u64)
where
    F: FnMut(u64),
{
    // 预热
    for i in 0..bench.warmup {
        op(i);
    }

    // 正式测量
    let start = Instant::now();
    for i in 0..bench.operations {
        op(bench.warmup + i);
    }
    let elapsed = start.elapsed();

    (elapsed, bench.operations)
}

/// 根据构建模式获取性能阈值
#[cfg(debug_assertions)]
fn throughput_threshold(base: f64) -> f64 {
    base * 0.05 // debug 模式下期望为 release 的 5%
}

#[cfg(not(debug_assertions))]
fn throughput_threshold(base: f64) -> f64 {
    base * 0.5 // release 模式下期望 50% 的基线
}

// ============================================================================
// 模块一：顶点写入吞吐量
// ============================================================================

/// 测试场景：单线程顶点写入吞吐量
/// 测量：每秒写入的顶点数
#[test]
fn bench_vertex_write_throughput_single_thread() {
    let srv = new_server(16);

    let bench = ThroughputBench {
        name: "vertex_write_single_thread",
        operations: 10_000,
        warmup: 200,
    };

    let (elapsed, count) = run_throughput_bench(&bench, |i| {
        let vid = format!("vw_{}", i);
        srv.add_vertex(vid, "t".into(), BTreeMap::new()).ok();
    });

    let qps = count as f64 / elapsed.as_secs_f64();
    eprintln!(
        "Vertex write (single thread): {} ops in {:?} = {:.0} ops/s",
        count, elapsed, qps
    );

    let threshold = throughput_threshold(100_000.0);
    assert!(
        qps >= threshold,
        "vertex write throughput {:.0} < threshold {:.0}",
        qps, threshold
    );
}

/// 测试场景：带属性的顶点写入吞吐量
/// 测量：包含多个属性时的写入性能
#[test]
fn bench_vertex_write_with_props_throughput() {
    let srv = new_server(16);

    let bench = ThroughputBench {
        name: "vertex_write_with_props",
        operations: 5_000,
        warmup: 100,
    };

    let props_data = props(&[
        ("name", "test_name"),
        ("age", "30"),
        ("city", "Beijing"),
        ("status", "active"),
        ("score", "99.5"),
    ]);

    let (elapsed, count) = run_throughput_bench(&bench, |i| {
        let vid = format!("vwp_{}", i);
        srv.add_vertex(vid, "user".into(), props_data.clone()).ok();
    });

    let qps = count as f64 / elapsed.as_secs_f64();
    eprintln!(
        "Vertex write with 5 props: {} ops in {:?} = {:.0} ops/s",
        count, elapsed, qps
    );

    let threshold = throughput_threshold(50_000.0);
    assert!(
        qps >= threshold,
        "vertex write with props throughput {:.0} < threshold {:.0}",
        qps, threshold
    );
}

/// 测试场景：不同数据规模下的顶点写入吞吐量
/// 测量：1k / 5k / 10k 规模下的吞吐量
#[test]
fn bench_vertex_write_scaling() {
    let sizes = [1_000u64, 5_000, 10_000];

    for &size in &sizes {
        let srv = new_server(16);

        // 预热
        for i in 0..100 {
            let _ = srv.add_vertex(format!("warm_{}", i), "t".into(), BTreeMap::new());
        }

        let start = Instant::now();
        for i in 0..size {
            let vid = format!("vs_{}_{}", size, i);
            srv.add_vertex(vid, "t".into(), BTreeMap::new()).ok();
        }
        let elapsed = start.elapsed();
        let qps = size as f64 / elapsed.as_secs_f64();

        eprintln!(
            "Vertex write ({} ops): {:.0} ops/s ({:?})",
            size, qps, elapsed
        );
    }
}

// ============================================================================
// 模块二：边写入吞吐量
// ============================================================================

/// 测试场景：单线程边写入吞吐量
/// 测量：每秒写入的边数
#[test]
fn bench_edge_write_throughput_single_thread() {
    let srv = new_server(16);

    // 预先创建顶点
    const VERTICES: u64 = 100;
    for i in 0..VERTICES {
        srv.add_vertex(format!("ev_{}", i), "t".into(), BTreeMap::new())
            .unwrap();
    }

    let bench = ThroughputBench {
        name: "edge_write_single_thread",
        operations: 10_000,
        warmup: 200,
    };

    let (elapsed, count) = run_throughput_bench(&bench, |i| {
        let src = format!("ev_{}", i % VERTICES);
        let dst = format!("ev_{}", (i * 7 + 3) % VERTICES);
        srv.add_edge(src, dst, "e".into(), i as i64, None, BTreeMap::new())
            .ok();
    });

    let qps = count as f64 / elapsed.as_secs_f64();
    eprintln!(
        "Edge write (single thread): {} ops in {:?} = {:.0} ops/s",
        count, elapsed, qps
    );

    let threshold = throughput_threshold(80_000.0);
    assert!(
        qps >= threshold,
        "edge write throughput {:.0} < threshold {:.0}",
        qps, threshold
    );
}

/// 测试场景：带权重和属性的边写入吞吐量
#[test]
fn bench_edge_write_with_props_throughput() {
    let srv = new_server(16);

    const VERTICES: u64 = 100;
    for i in 0..VERTICES {
        srv.add_vertex(format!("ewp_{}", i), "t".into(), BTreeMap::new())
            .unwrap();
    }

    let props_data = props(&[("weight", "0.85"), ("type", "strong")]);
    let bench = ThroughputBench {
        name: "edge_write_with_props",
        operations: 5_000,
        warmup: 100,
    };

    let (elapsed, count) = run_throughput_bench(&bench, |i| {
        let src = format!("ewp_{}", i % VERTICES);
        let dst = format!("ewp_{}", (i * 11 + 5) % VERTICES);
        srv.add_edge(
            src,
            dst,
            "relation".into(),
            i as i64,
            Some(0.5 + (i % 100) as f64 / 100.0),
            props_data.clone(),
        )
        .ok();
    });

    let qps = count as f64 / elapsed.as_secs_f64();
    eprintln!(
        "Edge write with props: {} ops in {:?} = {:.0} ops/s",
        count, elapsed, qps
    );

    let threshold = throughput_threshold(40_000.0);
    assert!(
        qps >= threshold,
        "edge write with props throughput {:.0} < threshold {:.0}",
        qps, threshold
    );
}

// ============================================================================
// 模块三：点查延迟
// ============================================================================

/// 测试场景：顶点点查延迟（P50/P95/P99）
/// 测量：通过 VID 查找顶点的延迟分布
#[test]
fn bench_point_lookup_latency() {
    let srv = new_server(16);

    const N: usize = 10_000;
    for i in 0..N {
        srv.add_vertex(
            format!("pl_{:06}", i),
            "t".into(),
            prop("value", &i.to_string()),
        )
        .unwrap();
    }

    // 点查延迟测试
    let iterations = 2_000;
    let mut latencies = Vec::with_capacity(iterations);

    // 预热
    for i in 0..100 {
        let vid = format!("pl_{:06}", i);
        let _ = read_vertex_props(&srv, &vid);
    }

    let total_start = Instant::now();
    for i in 0..iterations {
        let vid = format!("pl_{:06}", (i * 137) % N);
        let start = Instant::now();
        let _ = read_vertex_props(&srv, &vid);
        latencies.push(start.elapsed());
    }
    let total_time = total_start.elapsed();

    let stats = compute_latency_stats(&mut latencies, total_time);
    stats.print("Point Lookup Latency");

    // 断言基本性能要求
    #[cfg(debug_assertions)]
    {
        assert!(stats.p50 < Duration::from_millis(50), "p50 too high");
        assert!(stats.p99 < Duration::from_millis(200), "p99 too high");
    }
    #[cfg(not(debug_assertions))]
    {
        assert!(stats.p50 < Duration::from_millis(5), "p50 too high");
        assert!(stats.p99 < Duration::from_millis(20), "p99 too high");
    }
}

fn read_vertex_props(srv: &StorageServer, vid: &str) -> BTreeMap<String, PropValue> {
    let sc = srv.raft_nodes.shard_count();
    let shard = graph_codec::vid_hash_shard(vid, sc);
    let prefix = shard.to_le_bytes();
    let rows = srv
        .rocks_db_handles
        .seek_prefix(&mox_kg_storage_svc::kv_engine::cf_name_vid_meta(shard), &prefix)
        .unwrap_or_default();
    for (k, v) in rows {
        if let Ok((_, _, vv)) = graph_codec::decode_vertex_key(&k) {
            if vv == vid {
                if let Ok((_t, p)) = graph_codec::decode_vertex_value(&v) {
                    return p;
                }
            }
        }
    }
    BTreeMap::new()
}

/// 测试场景：邻居查询延迟
/// 测量：get_neighbors 的延迟分布
#[test]
fn bench_neighbor_query_latency() {
    let srv = new_server(16);

    // 构建扇出为 100 的星型图
    srv.add_vertex("hub".into(), "hub".into(), BTreeMap::new())
        .unwrap();
    for i in 0..100 {
        let vid = format!("leaf_{}", i);
        srv.add_vertex(vid.clone(), "leaf".into(), BTreeMap::new())
            .unwrap();
        srv.add_edge(
            "hub".into(),
            vid,
            "e".into(),
            i as i64,
            None,
            BTreeMap::new(),
        )
        .unwrap();
    }

    let iterations = 1_000;
    let mut latencies = Vec::with_capacity(iterations);

    // 预热
    let _ = srv.get_neighbors("hub", Direction::Out, &["e"]).unwrap();

    let total_start = Instant::now();
    for _ in 0..iterations {
        let start = Instant::now();
        let result = srv.get_neighbors("hub", Direction::Out, &["e"]).unwrap();
        let elapsed = start.elapsed();
        latencies.push(elapsed);
        assert_eq!(result.len(), 100);
    }
    let total_time = total_start.elapsed();

    let stats = compute_latency_stats(&mut latencies, total_time);
    stats.print("Neighbor Query (fanout=100)");

    #[cfg(debug_assertions)]
    {
        assert!(stats.p50 < Duration::from_millis(100), "p50 too high");
    }
    #[cfg(not(debug_assertions))]
    {
        assert!(stats.p50 < Duration::from_millis(10), "p50 too high");
    }
}

// ============================================================================
// 模块四：遍历查询性能
// ============================================================================

/// 测试场景：1 跳遍历性能
#[test]
fn bench_traversal_1hop_performance() {
    let srv = new_server(16);

    // 构建多层图：root -> level1 (50个) -> level2 (每个10个)
    srv.add_vertex("root".into(), "root".into(), BTreeMap::new())
        .unwrap();

    for i in 0..50 {
        let l1 = format!("l1_{}", i);
        srv.add_vertex(l1.clone(), "l1".into(), BTreeMap::new())
            .unwrap();
        srv.add_edge(
            "root".into(),
            l1.clone(),
            "link".into(),
            i as i64,
            None,
            BTreeMap::new(),
        )
        .unwrap();

        for j in 0..10 {
            let l2 = format!("l2_{}_{}", i, j);
            srv.add_vertex(l2.clone(), "l2".into(), BTreeMap::new())
                .unwrap();
            srv.add_edge(
                l1.clone(),
                l2,
                "link".into(),
                j as i64,
                None,
                BTreeMap::new(),
            )
            .unwrap();
        }
    }

    // 1 跳遍历
    let iterations = 500;
    let mut latencies = Vec::with_capacity(iterations);

    let total_start = Instant::now();
    for _ in 0..iterations {
        let start = Instant::now();
        let result = srv.get_neighbors("root", Direction::Out, &["link"]).unwrap();
        latencies.push(start.elapsed());
        assert_eq!(result.len(), 50);
    }
    let total_time = total_start.elapsed();

    let stats = compute_latency_stats(&mut latencies, total_time);
    stats.print("1-Hop Traversal (fanout=50)");
}

/// 测试场景：2 跳遍历性能
#[test]
fn bench_traversal_2hop_performance() {
    let srv = new_server(16);

    // 构建图：root -> l1 (20个) -> l2 (每个20个) = 400 个 l2 节点
    srv.add_vertex("root2".into(), "root".into(), BTreeMap::new())
        .unwrap();

    for i in 0..20 {
        let l1 = format!("l1b_{}", i);
        srv.add_vertex(l1.clone(), "l1".into(), BTreeMap::new())
            .unwrap();
        srv.add_edge(
            "root2".into(),
            l1.clone(),
            "e".into(),
            i as i64,
            None,
            BTreeMap::new(),
        )
        .unwrap();

        for j in 0..20 {
            let l2 = format!("l2b_{}_{}", i, j);
            srv.add_vertex(l2.clone(), "l2".into(), BTreeMap::new())
                .unwrap();
            srv.add_edge(
                l1.clone(),
                l2,
                "e".into(),
                j as i64,
                None,
                BTreeMap::new(),
            )
            .unwrap();
        }
    }

    // 2 跳遍历
    let iterations = 200;
    let mut latencies = Vec::with_capacity(iterations);

    let total_start = Instant::now();
    for _ in 0..iterations {
        let start = Instant::now();

        // 2 跳：先获取 1 跳邻居，再获取每个邻居的邻居
        let hop1 = srv.get_neighbors("root2", Direction::Out, &["e"]).unwrap();
        let mut hop2 = BTreeSet::new();
        for n in &hop1 {
            let neighbors = srv
                .get_neighbors(&n.neighbor_vid, Direction::Out, &["e"])
                .unwrap();
            for nn in neighbors {
                hop2.insert(nn.neighbor_vid);
            }
        }

        latencies.push(start.elapsed());
        assert_eq!(hop2.len(), 400);
    }
    let total_time = total_start.elapsed();

    let stats = compute_latency_stats(&mut latencies, total_time);
    stats.print("2-Hop Traversal (fanout=20x20=400)");
}

/// 测试场景：3 跳遍历性能
#[test]
fn bench_traversal_3hop_performance() {
    let srv = new_server(16);

    // 构建 3 层图（较小规模以保证测试速度）
    // root -> l1 (10) -> l2 (每个5) -> l3 (每个5) = 250 l3 节点
    srv.add_vertex("root3".into(), "root".into(), BTreeMap::new())
        .unwrap();

    for i in 0..10 {
        let l1 = format!("l1c_{}", i);
        srv.add_vertex(l1.clone(), "l1".into(), BTreeMap::new())
            .unwrap();
        srv.add_edge(
            "root3".into(),
            l1.clone(),
            "e".into(),
            i as i64,
            None,
            BTreeMap::new(),
        )
        .unwrap();

        for j in 0..5 {
            let l2 = format!("l2c_{}_{}", i, j);
            srv.add_vertex(l2.clone(), "l2".into(), BTreeMap::new())
                .unwrap();
            srv.add_edge(
                l1.clone(),
                l2.clone(),
                "e".into(),
                j as i64,
                None,
                BTreeMap::new(),
            )
            .unwrap();

            for k in 0..5 {
                let l3 = format!("l3c_{}_{}_{}", i, j, k);
                srv.add_vertex(l3.clone(), "l3".into(), BTreeMap::new())
                    .unwrap();
                srv.add_edge(
                    l2.clone(),
                    l3,
                    "e".into(),
                    k as i64,
                    None,
                    BTreeMap::new(),
                )
                .unwrap();
            }
        }
    }

    // 3 跳遍历
    let iterations = 100;
    let mut latencies = Vec::with_capacity(iterations);

    let total_start = Instant::now();
    for _ in 0..iterations {
        let start = Instant::now();

        let hop1 = srv.get_neighbors("root3", Direction::Out, &["e"]).unwrap();
        let mut hop2: BTreeSet<String> = BTreeSet::new();
        for n in &hop1 {
            for nn in srv
                .get_neighbors(&n.neighbor_vid, Direction::Out, &["e"])
                .unwrap()
            {
                hop2.insert(nn.neighbor_vid);
            }
        }
        let mut hop3: BTreeSet<String> = BTreeSet::new();
        for vid in &hop2 {
            for n in srv.get_neighbors(vid, Direction::Out, &["e"]).unwrap() {
                hop3.insert(n.neighbor_vid);
            }
        }

        latencies.push(start.elapsed());
        assert_eq!(hop3.len(), 250); // 10 * 5 * 5
    }
    let total_time = total_start.elapsed();

    let stats = compute_latency_stats(&mut latencies, total_time);
    stats.print("3-Hop Traversal (fanout=10x5x5=250)");
}

// ============================================================================
// 模块五：图算法性能
// ============================================================================

/// 测试场景：PageRank 算法执行时间
/// 使用 petgraph 和简单迭代法计算 PageRank
#[test]
fn bench_graph_algorithm_pagerank() {
    use petgraph::graph::{DiGraph, NodeIndex};
    use std::collections::HashMap;

    // 构建图：1000 节点，5000 边
    const NODES: usize = 1000;
    const EDGES: usize = 5000;

    let mut graph = DiGraph::<(), ()>::new();
    let mut nodes: Vec<NodeIndex> = Vec::with_capacity(NODES);
    for _ in 0..NODES {
        nodes.push(graph.add_node(()));
    }

    // 生成有向边
    let mut rng = rand::thread_rng();
    use rand::Rng;
    for _ in 0..EDGES {
        let src = rng.gen_range(0..NODES);
        let dst = rng.gen_range(0..NODES);
        if src != dst {
            graph.add_edge(nodes[src], nodes[dst], ());
        }
    }

    // PageRank 迭代计算
    let iterations = 50;
    let damping = 0.85;

    let start = Instant::now();
    let mut ranks: Vec<f64> = vec![1.0 / NODES as f64; NODES];
    let mut out_degree: Vec<usize> = vec![0; NODES];

    for ni in graph.node_indices() {
        out_degree[ni.index()] = graph
            .neighbors_directed(ni, petgraph::Direction::Outgoing)
            .count();
    }

    for _iter in 0..iterations {
        let mut new_ranks = vec![(1.0 - damping) / NODES as f64; NODES];
        for ni in graph.node_indices() {
            let idx = ni.index();
            if out_degree[idx] > 0 {
                let contribution = damping * ranks[idx] / out_degree[idx] as f64;
                for target in graph.neighbors_directed(ni, petgraph::Direction::Outgoing) {
                    new_ranks[target.index()] += contribution;
                }
            }
        }
        ranks = new_ranks;
    }

    let elapsed = start.elapsed();
    let total_rank: f64 = ranks.iter().sum();

    eprintln!(
        "PageRank: {} nodes, {} edges, {} iterations -> {:?} (sum={:.6})",
        NODES, EDGES, iterations, elapsed, total_rank
    );

    // 验证 PageRank 收敛（总和约为 1）
    assert!((total_rank - 1.0).abs() < 0.01, "PageRank sum should be ~1.0");

    #[cfg(debug_assertions)]
    assert!(elapsed < Duration::from_secs(10));
    #[cfg(not(debug_assertions))]
    assert!(elapsed < Duration::from_secs(2));
}

/// 测试场景：最短路径（BFS）性能
#[test]
fn bench_graph_algorithm_shortest_path_bfs() {
    let srv = new_server(16);

    // 构建网格图：10x10 = 100 节点，约 180 条边
    const SIZE: usize = 20;
    const NODES: usize = SIZE * SIZE;

    for i in 0..SIZE {
        for j in 0..SIZE {
            let vid = format!("grid_{}_{}", i, j);
            srv.add_vertex(vid, "grid".into(), BTreeMap::new())
                .unwrap();
        }
    }

    // 添加边（右向和下向）
    for i in 0..SIZE {
        for j in 0..SIZE {
            let current = format!("grid_{}_{}", i, j);
            if j + 1 < SIZE {
                let right = format!("grid_{}_{}", i, j + 1);
                srv.add_edge(
                    current.clone(),
                    right,
                    "adj".into(),
                    0,
                    None,
                    BTreeMap::new(),
                )
                .unwrap();
            }
            if i + 1 < SIZE {
                let down = format!("grid_{}_{}", i + 1, j);
                srv.add_edge(current, down, "adj".into(), 1, None, BTreeMap::new())
                    .unwrap();
            }
        }
    }

    // BFS 最短路径：从 (0,0) 到 (SIZE-1, SIZE-1)
    let start_vid = "grid_0_0".to_string();
    let end_vid = format!("grid_{}_{}", SIZE - 1, SIZE - 1);

    let iterations = 50;
    let mut latencies = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();

        // BFS
        let mut visited = HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        let mut distance = HashMap::new();

        queue.push_back(start_vid.clone());
        visited.insert(start_vid.clone());
        distance.insert(start_vid.clone(), 0);

        while let Some(current) = queue.pop_front() {
            if current == end_vid {
                break;
            }
            let dist = *distance.get(&current).unwrap_or(&0);
            let neighbors = srv
                .get_neighbors(&current, Direction::Out, &["adj"])
                .unwrap();
            for n in neighbors {
                if visited.insert(n.neighbor_vid.clone()) {
                    distance.insert(n.neighbor_vid.clone(), dist + 1);
                    queue.push_back(n.neighbor_vid);
                }
            }
        }

        latencies.push(start.elapsed());

        // 验证最短路径长度（网格图中最短路径为 2*(SIZE-1)）
        let shortest = *distance.get(&end_vid).unwrap_or(&0);
        assert_eq!(shortest, 2 * (SIZE - 1));
    }

    let total_time: Duration = latencies.iter().sum();
    let stats = compute_latency_stats(&mut latencies, total_time);
    stats.print(&format!("BFS Shortest Path ({}x{} grid)", SIZE, SIZE));
}

/// 测试场景：社区发现（简单连通分量）性能
#[test]
fn bench_graph_algorithm_connected_components() {
    use petgraph::graph::UnGraph;
    use petgraph::unionfind::UnionFind;

    // 构建图：1000 节点，3000 边
    const NODES: usize = 1000;
    const EDGES: usize = 3000;

    let mut graph = UnGraph::<(), ()>::new_undirected();
    let mut nodes = Vec::with_capacity(NODES);
    for _ in 0..NODES {
        nodes.push(graph.add_node(()));
    }

    let mut rng = rand::thread_rng();
    use rand::Rng;
    for _ in 0..EDGES {
        let src = rng.gen_range(0..NODES);
        let dst = rng.gen_range(0..NODES);
        if src != dst {
            graph.add_edge(nodes[src], nodes[dst], ());
        }
    }

    // 使用 Union-Find 计算连通分量
    let start = Instant::now();
    let mut uf = UnionFind::new(NODES);

    for edge in graph.edge_indices() {
        let (a, b) = graph.edge_endpoints(edge).unwrap();
        uf.union(a.index(), b.index());
    }

    // 统计连通分量数
    let mut components = HashSet::new();
    for i in 0..NODES {
        components.insert(uf.find(i));
    }
    let component_count = components.len();

    let elapsed = start.elapsed();

    eprintln!(
        "Connected Components: {} nodes, {} edges -> {} components in {:?}",
        NODES, EDGES, component_count, elapsed
    );

    assert!(component_count > 0);
    assert!(elapsed < Duration::from_secs(5));
}

// ============================================================================
// 模块六：批量导入性能
// ============================================================================

/// 测试场景：万级顶点批量导入性能
#[test]
fn bench_batch_import_vertices_10k() {
    let srv = new_server(16);

    const N: u64 = 10_000;

    let start = Instant::now();
    for i in 0..N {
        srv.add_vertex(
            format!("import_v_{}", i),
            "node".into(),
            prop("idx", &i.to_string()),
        )
        .ok();
    }
    let elapsed = start.elapsed();

    let qps = N as f64 / elapsed.as_secs_f64();
    eprintln!(
        "Batch import {} vertices: {:?} ({:.0} ops/s)",
        N, elapsed, qps
    );

    // 验证数据完整
    let total: u64 = srv.shard_vertex_counts().values().sum();
    assert_eq!(total, N);

    let threshold = throughput_threshold(80_000.0);
    assert!(
        qps >= threshold,
        "batch import throughput {:.0} < threshold {:.0}",
        qps, threshold
    );
}

/// 测试场景：万级边批量导入性能
#[test]
fn bench_batch_import_edges_10k() {
    let srv = new_server(16);

    const VERTICES: u64 = 200;
    const EDGES: u64 = 10_000;

    // 预先创建顶点
    for i in 0..VERTICES {
        srv.add_vertex(format!("ie_{}", i), "t".into(), BTreeMap::new())
            .unwrap();
    }

    let start = Instant::now();
    for i in 0..EDGES {
        let src = format!("ie_{}", i % VERTICES);
        let dst = format!("ie_{}", (i * 7) % VERTICES);
        srv.add_edge(src, dst, "e".into(), i as i64, None, BTreeMap::new())
            .ok();
    }
    let elapsed = start.elapsed();

    let qps = EDGES as f64 / elapsed.as_secs_f64();
    eprintln!(
        "Batch import {} edges: {:?} ({:.0} ops/s)",
        EDGES, elapsed, qps
    );

    let threshold = throughput_threshold(60_000.0);
    assert!(
        qps >= threshold,
        "batch edge import throughput {:.0} < threshold {:.0}",
        qps, threshold
    );
}

/// 测试场景：混合导入性能（顶点 + 边）
#[test]
fn bench_batch_import_mixed() {
    let srv = new_server(16);

    const VERTICES: u64 = 2000;
    const EDGES_PER_VERTEX: u64 = 5;

    let start = Instant::now();

    // 导入顶点
    for i in 0..VERTICES {
        srv.add_vertex(
            format!("mix_{}", i),
            "t".into(),
            prop("idx", &i.to_string()),
        )
        .ok();
    }

    // 导入边
    for i in 0..VERTICES {
        let src = format!("mix_{}", i);
        for j in 0..EDGES_PER_VERTEX {
            let dst = format!("mix_{}", (i * 3 + j * 7) % VERTICES);
            srv.add_edge(
                src.clone(),
                dst,
                "e".into(),
                j as i64,
                None,
                BTreeMap::new(),
            )
            .ok();
        }
    }

    let elapsed = start.elapsed();
    let total_ops = VERTICES + VERTICES * EDGES_PER_VERTEX;
    let qps = total_ops as f64 / elapsed.as_secs_f64();

    eprintln!(
        "Batch import mixed: {} vertices + {} edges = {} ops in {:?} ({:.0} ops/s)",
        VERTICES,
        VERTICES * EDGES_PER_VERTEX,
        total_ops,
        elapsed,
        qps
    );

    let vertex_total: u64 = srv.shard_vertex_counts().values().sum();
    assert_eq!(vertex_total, VERTICES);
}

// ============================================================================
// 模块七：热点缓存性能
// ============================================================================

/// 测试场景：热点缓存命中率与性能提升
#[test]
fn bench_hot_cache_hit_rate() {
    let srv = new_server(16);

    srv.add_vertex("hot".into(), "t".into(), BTreeMap::new())
        .unwrap();
    srv.add_vertex("a".into(), "t".into(), BTreeMap::new())
        .unwrap();
    srv.add_vertex("b".into(), "t".into(), BTreeMap::new())
        .unwrap();

    srv.add_edge("hot".into(), "a".into(), "e".into(), 0, None, BTreeMap::new())
        .unwrap();
    srv.add_edge("hot".into(), "b".into(), "e".into(), 1, None, BTreeMap::new())
        .unwrap();

    const ITERATIONS: u64 = 100_000;

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let r = srv.get_neighbors("hot", Direction::Both, &[]).unwrap();
        assert_eq!(r.len(), 2);
    }
    let elapsed = start.elapsed();

    let qps = ITERATIONS as f64 / elapsed.as_secs_f64();
    let hit_rate = srv.hot_cache.hit_rate();
    let misses = srv.hot_cache.misses();
    let calls = srv.hot_cache.total_calls();

    eprintln!(
        "Hot cache: {} queries in {:?} = {:.0} ops/s | hit_rate={:.4} (misses={}/calls={})",
        ITERATIONS, elapsed, qps, hit_rate, misses, calls
    );

    assert!(hit_rate >= 0.99, "hot cache hit rate {:.4} < 0.99", hit_rate);
}

/// 测试场景：LRU 缓存操作性能
#[test]
fn bench_lru_cache_operations() {
    use mox_kg_storage_svc::storage_api::LruCache;

    const CAP: usize = 10_000;
    const CAP_U64: u64 = 10_000;
    const OPS: u64 = 100_000;

    let mut cache: LruCache<String, Vec<u8>> = LruCache::new(CAP);

    // 填充缓存
    for i in 0..CAP {
        cache.insert(format!("key_{}", i), vec![i as u8; 10]);
    }

    // 测量 get 性能（命中）
    let start = Instant::now();
    let mut hits = 0;
    for i in 0..OPS {
        let key = format!("key_{}", i % CAP_U64);
        if cache.get(&key).is_some() {
            hits += 1;
        }
    }
    let elapsed = start.elapsed();
    let qps = OPS as f64 / elapsed.as_secs_f64();

    eprintln!(
        "LRU cache get (hit): {} ops in {:?} = {:.0} ops/s",
        OPS, elapsed, qps
    );
    assert_eq!(hits, OPS);

    // 测量 insert 性能
    let start = Instant::now();
    for i in 0..OPS {
        let key = format!("new_key_{}", i);
        cache.insert(key, vec![0u8; 10]);
    }
    let elapsed = start.elapsed();
    let qps = OPS as f64 / elapsed.as_secs_f64();

    eprintln!(
        "LRU cache insert: {} ops in {:?} = {:.0} ops/s",
        OPS, elapsed, qps
    );

    assert!(qps > 10_000.0, "LRU insert too slow: {:.0} ops/s", qps);
}

// ============================================================================
// 模块八：scan_edges 性能
// ============================================================================

/// 测试场景：scan_edges 分页查询性能
#[test]
fn bench_scan_edges_performance() {
    let srv = new_server(16);

    srv.add_vertex("src".into(), "t".into(), BTreeMap::new())
        .unwrap();
    srv.add_vertex("dst".into(), "t".into(), BTreeMap::new())
        .unwrap();

    const EDGES: usize = 5_000;
    for i in 0..EDGES {
        srv.add_edge(
            "src".into(),
            "dst".into(),
            "e".into(),
            i as i64,
            None,
            BTreeMap::new(),
        )
        .unwrap();
    }

    // 不同 page size 的扫描性能
    for &page_size in &[10u32, 100, 500, 1000] {
        let iterations = 100;
        let mut total = Duration::ZERO;

        for _ in 0..iterations {
            let start = Instant::now();
            let result = srv.scan_edges(&["e"], page_size, 0u64).unwrap();
            total += start.elapsed();
            assert_eq!(result.len(), page_size as usize);
        }

        let avg = total / iterations;
        eprintln!(
            "scan_edges(page_size={}): avg {:?} per query ({} iterations)",
            page_size, avg, iterations
        );
    }
}

// ============================================================================
// 模块九：编解码器性能
// ============================================================================

/// 测试场景：顶点 key 编解码性能
#[test]
fn bench_codec_vertex_key() {
    use mox_kg_storage_svc::graph_codec;

    const ITERATIONS: u64 = 100_000;

    // 编码性能
    let start = Instant::now();
    for i in 0..ITERATIONS {
        let vid = format!("vertex_{}", i);
        let _ = graph_codec::encode_vertex_key(42, "user", &vid).unwrap();
    }
    let encode_elapsed = start.elapsed();
    let encode_qps = ITERATIONS as f64 / encode_elapsed.as_secs_f64();
    eprintln!(
        "Vertex key encode: {} ops in {:?} = {:.0} ops/s",
        ITERATIONS, encode_elapsed, encode_qps
    );

    // 解码性能
    let keys: Vec<Vec<u8>> = (0..1000)
        .map(|i| {
            let vid = format!("vertex_{}", i);
            graph_codec::encode_vertex_key(42, "user", &vid).unwrap()
        })
        .collect();

    let start = Instant::now();
    for i in 0..ITERATIONS {
        let key = &keys[i as usize % keys.len()];
        let _ = graph_codec::decode_vertex_key(key).unwrap();
    }
    let decode_elapsed = start.elapsed();
    let decode_qps = ITERATIONS as f64 / decode_elapsed.as_secs_f64();
    eprintln!(
        "Vertex key decode: {} ops in {:?} = {:.0} ops/s",
        ITERATIONS, decode_elapsed, decode_qps
    );

    assert!(encode_qps > 10_000.0);
    assert!(decode_qps > 10_000.0);
}

/// 测试场景：属性编解码性能
#[test]
fn bench_codec_props() {
    use mox_kg_storage_svc::graph_codec;

    const ITERATIONS: u64 = 50_000;

    // 准备测试数据：10 个属性
    let mut test_props = BTreeMap::new();
    for i in 0..10 {
        test_props.insert(
            format!("prop_{}", i),
            PropValue::from_str(&format!("value_{}_long_enough_for_test", i)),
        );
    }

    // 编码性能
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = graph_codec::encode_props(&test_props).unwrap();
    }
    let elapsed = start.elapsed();
    let qps = ITERATIONS as f64 / elapsed.as_secs_f64();
    eprintln!("Props encode (10 props): {:.0} ops/s", qps);

    // 解码性能
    let encoded = graph_codec::encode_props(&test_props).unwrap();
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = graph_codec::decode_props(&encoded).unwrap();
    }
    let elapsed = start.elapsed();
    let qps = ITERATIONS as f64 / elapsed.as_secs_f64();
    eprintln!("Props decode (10 props): {:.0} ops/s", qps);

    assert!(qps > 5_000.0);
}

// ============================================================================
// 模块十：综合性能测试
// ============================================================================

/// 测试场景：混合读写负载
/// 模拟真实场景下的读写混合操作
#[test]
fn bench_mixed_read_write_workload() {
    let srv = new_server(16);

    // 预先写入 5000 顶点
    const INITIAL_VERTICES: u64 = 5_000;
    for i in 0..INITIAL_VERTICES {
        srv.add_vertex(format!("mix_{}", i), "t".into(), prop("val", &i.to_string()))
            .unwrap();
    }

    // 混合负载：70% 读 + 30% 写
    const TOTAL_OPS: u64 = 10_000;
    const READ_RATIO: f64 = 0.7;

    let mut read_count = 0u64;
    let mut write_count = 0u64;

    let start = Instant::now();
    for i in 0..TOTAL_OPS {
        if (i as f64) < (TOTAL_OPS as f64) * READ_RATIO {
            // 读操作：点查
            let vid = format!("mix_{}", i % INITIAL_VERTICES);
            let _ = read_vertex_props(&srv, &vid);
            read_count += 1;
        } else {
            // 写操作：更新顶点
            let vid = format!("mix_{}", i % INITIAL_VERTICES);
            srv.update_vertex(vid, prop("updated", "true")).ok();
            write_count += 1;
        }
    }
    let elapsed = start.elapsed();

    let total_qps = TOTAL_OPS as f64 / elapsed.as_secs_f64();
    eprintln!(
        "Mixed workload (70% read / 30% write): {} ops in {:?} = {:.0} ops/s (reads={}, writes={})",
        TOTAL_OPS, elapsed, total_qps, read_count, write_count
    );

    assert!(total_qps > throughput_threshold(50_000.0));
}

/// 测试场景：不同分片数下的性能对比
#[test]
fn bench_shard_count_performance_comparison() {
    let shard_counts = [4u16, 16];

    for &shards in &shard_counts {
        let srv = new_server(shards);
        const N: u64 = 5_000;

        // 预热
        for i in 0..100 {
            let _ = srv.add_vertex(format!("warm_{}", i), "t".into(), BTreeMap::new());
        }

        let start = Instant::now();
        for i in 0..N {
            let vid = format!("sc_{}_{}", shards, i);
            srv.add_vertex(vid, "t".into(), BTreeMap::new()).ok();
        }
        let elapsed = start.elapsed();
        let qps = N as f64 / elapsed.as_secs_f64();

        eprintln!("{} shards: {:.0} vertex writes/sec", shards, qps);
    }
}
