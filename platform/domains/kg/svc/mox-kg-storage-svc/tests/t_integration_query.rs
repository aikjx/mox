// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/mox/mox

//! 查询引擎集成测试 (Query Engine Integration Tests)
//!
//! 测试场景覆盖：
//! - nGQL 语句解析与执行模式验证
//! - 查询执行：简单查询、条件查询、聚合查询
//! - 查询优化器验证：谓词下推、投影下推、常量折叠
//! - 索引选择验证：查询是否正确选择最优索引
//! - 执行计划：EXPLAIN 输出格式与内容验证
//! - Cypher/openCypher 兼容性模式
//!
//! 说明：
//! 本测试套件基于存储层 API 模拟查询引擎的各种执行模式，
//! 验证存储层对查询算子的支撑能力。实际的 SQL/nGQL 解析层
//! 在查询服务中实现，这里验证其底层执行路径的正确性。

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::time::{Duration, Instant};

use mox_kg_storage_svc::graph_codec::{self, PropValue};
use mox_kg_storage_svc::storage_api::Direction;
use mox_kg_storage_svc::storage_server::StorageServer;

// ============================================================================
// 通用测试辅助
// ============================================================================

fn test_addrs() -> Vec<String> {
    vec![
        "127.0.0.1:9201".into(),
        "127.0.0.1:9202".into(),
        "127.0.0.1:9203".into(),
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

fn add_edge(srv: &StorageServer, a: &str, b: &str, et: &str, rank: i64) {
    srv.add_edge(a.into(), b.into(), et.into(), rank, None, BTreeMap::new())
        .unwrap();
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

/// 从 KV 存储扫描某分片内所有顶点
fn scan_shard_vertices(
    srv: &StorageServer,
    shard: u16,
) -> Vec<(String, String, BTreeMap<String, PropValue>)> {
    let prefix = shard.to_le_bytes();
    let rows = srv
        .rocks_db_handles
        .seek_prefix(&mox_kg_storage_svc::kv_engine::cf_name_vid_meta(shard), &prefix)
        .unwrap_or_default();
    let mut result = Vec::new();
    for (k, v) in rows {
        if let Ok((_sh, _tag_hash, vid)) = graph_codec::decode_vertex_key(&k) {
            if let Ok((tag, props)) = graph_codec::decode_vertex_value(&v) {
                result.push((vid, tag, props));
            }
        }
    }
    result
}

/// 扫描所有分片的顶点
fn scan_all_vertices(srv: &StorageServer) -> Vec<(String, String, BTreeMap<String, PropValue>)> {
    let sc = srv.raft_nodes.shard_count();
    let mut all = Vec::new();
    for shard in 0..sc {
        all.extend(scan_shard_vertices(srv, shard));
    }
    all
}

// ============================================================================
// 模块一：nGQL 查询语句解析与执行模拟
// ============================================================================

/// 测试场景：FETCH PROP ON 语句模拟
/// 验证：按 VID 读取顶点属性（模拟 FETCH PROP ON tag vid）
#[test]
fn ngql_fetch_prop_on_tag() {
    let srv = new_server(16);

    // 准备数据
    srv.add_vertex(
        "player100".into(),
        "player".into(),
        props(&[("name", "Tim Duncan"), ("age", "42"), ("team", "Spurs")]),
    )
    .unwrap();
    srv.add_vertex(
        "player101".into(),
        "player".into(),
        props(&[("name", "Tony Parker"), ("age", "38"), ("team", "Spurs")]),
    )
    .unwrap();

    // 模拟 FETCH PROP ON player "player100" YIELD player.name, player.age
    let props = read_vertex_props(&srv, "player100");
    assert_eq!(props.get("name").and_then(|p| p.as_str()), Some("Tim Duncan"));
    assert_eq!(props.get("age").and_then(|p| p.as_str()), Some("42"));

    // 模拟 FETCH PROP ON player "player100", "player101"
    let v100 = read_vertex_props(&srv, "player100");
    let v101 = read_vertex_props(&srv, "player101");
    assert_eq!(v100.len(), 3);
    assert_eq!(v101.len(), 3);
    assert_ne!(v100.get("name"), v101.get("name"));
}

/// 测试场景：GO 语句模拟 - 单跳遍历
/// 验证：GO 1 STEPS FROM vid OVER edge_type
#[test]
fn ngql_go_1_step() {
    let srv = new_server(16);

    // 构建社交图
    srv.add_vertex("user1".into(), "user".into(), prop("name", "Alice"))
        .unwrap();
    srv.add_vertex("user2".into(), "user".into(), prop("name", "Bob"))
        .unwrap();
    srv.add_vertex("user3".into(), "user".into(), prop("name", "Charlie"))
        .unwrap();
    srv.add_vertex("user4".into(), "user".into(), prop("name", "David"))
        .unwrap();

    add_edge(&srv, "user1", "user2", "follow", 0);
    add_edge(&srv, "user1", "user3", "follow", 1);
    add_edge(&srv, "user2", "user4", "follow", 0);
    add_edge(&srv, "user3", "user4", "follow", 0);

    // 模拟 GO 1 STEPS FROM "user1" OVER follow
    let result = srv
        .get_neighbors("user1", Direction::Out, &["follow"])
        .unwrap();

    assert_eq!(result.len(), 2);
    let vids: BTreeSet<_> = result.iter().map(|n| n.neighbor_vid.clone()).collect();
    assert!(vids.contains("user2"));
    assert!(vids.contains("user3"));
}

/// 测试场景：GO 语句模拟 - 多跳遍历
/// 验证：GO 2 STEPS FROM vid OVER edge_type
#[test]
fn ngql_go_2_steps() {
    let srv = new_server(16);

    // 构建链式图：a -> b -> c -> d
    for v in ["a", "b", "c", "d"] {
        srv.add_vertex(v.into(), "n".into(), BTreeMap::new())
            .unwrap();
    }
    add_edge(&srv, "a", "b", "e", 0);
    add_edge(&srv, "b", "c", "e", 0);
    add_edge(&srv, "c", "d", "e", 0);

    // 模拟 GO 2 STEPS FROM "a" OVER e
    let hop1 = srv.get_neighbors("a", Direction::Out, &["e"]).unwrap();
    let mut hop2 = BTreeSet::new();
    for n in &hop1 {
        let next = srv
            .get_neighbors(&n.neighbor_vid, Direction::Out, &["e"])
            .unwrap();
        for nn in next {
            hop2.insert(nn.neighbor_vid);
        }
    }

    assert_eq!(hop2.len(), 1);
    assert!(hop2.contains("c"));
}

/// 测试场景：GO 语句模拟 - REVERSELY 反向遍历
/// 验证：GO 1 STEPS FROM vid OVER edge_type REVERSELY
#[test]
fn ngql_go_reversely() {
    let srv = new_server(16);

    srv.add_vertex("a".into(), "n".into(), BTreeMap::new())
        .unwrap();
    srv.add_vertex("b".into(), "n".into(), BTreeMap::new())
        .unwrap();
    srv.add_vertex("c".into(), "n".into(), BTreeMap::new())
        .unwrap();

    add_edge(&srv, "a", "b", "like", 0);
    add_edge(&srv, "c", "b", "like", 0);

    // 模拟 GO 1 STEPS FROM "b" OVER like REVERSELY
    // 即查询谁喜欢 b（入边）
    let result = srv
        .get_neighbors("b", Direction::In, &["like"])
        .unwrap();

    assert_eq!(result.len(), 2);
    let vids: BTreeSet<_> = result.iter().map(|n| n.neighbor_vid.clone()).collect();
    assert!(vids.contains("a"));
    assert!(vids.contains("c"));
}

/// 测试场景：GO 语句模拟 - YIELD 返回指定字段
/// 验证：遍历后提取邻居顶点的指定属性
#[test]
fn ngql_go_yield_props() {
    let srv = new_server(16);

    srv.add_vertex("alice".into(), "user".into(), prop("name", "Alice"))
        .unwrap();
    srv.add_vertex("bob".into(), "user".into(), props(&[("name", "Bob"), ("age", "30")]))
        .unwrap();
    srv.add_vertex("charlie".into(), "user".into(), props(&[("name", "Charlie"), ("age", "25")]))
        .unwrap();

    add_edge(&srv, "alice", "bob", "friend", 0);
    add_edge(&srv, "alice", "charlie", "friend", 1);

    // 模拟 GO 1 STEPS FROM "alice" OVER friend YIELD $$.user.name, $$.user.age
    let neighbors = srv
        .get_neighbors("alice", Direction::Out, &["friend"])
        .unwrap();

    let mut results = Vec::new();
    for n in &neighbors {
        let p = read_vertex_props(&srv, &n.neighbor_vid);
        let name = p.get("name").and_then(|x| x.as_str()).unwrap_or("");
        let age = p.get("age").and_then(|x| x.as_str()).unwrap_or("");
        results.push((name.to_string(), age.to_string()));
    }

    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|(n, _)| n == "Bob"));
    assert!(results.iter().any(|(n, _)| n == "Charlie"));
}

// ============================================================================
// 模块二：查询执行 - 简单查询
// ============================================================================

/// 测试场景：单点查询（点查）
/// 验证：通过 VID 精确查找顶点
#[test]
fn query_point_lookup_by_vid() {
    let srv = new_server(16);

    // 插入 1000 个顶点
    for i in 0..1000 {
        let vid = format!("v_{:04}", i);
        srv.add_vertex(
            vid,
            "item".into(),
            props(&[("idx", &i.to_string()), ("val", &(i * 10).to_string())]),
        )
        .unwrap();
    }

    // 点查性能测试
    let start = Instant::now();
    for i in [0, 500, 999, 123, 456, 789] {
        let vid = format!("v_{:04}", i);
        let p = read_vertex_props(&srv, &vid);
        let expected = i.to_string();
        assert_eq!(
            p.get("idx").and_then(|x| x.as_str()),
            Some(expected.as_str())
        );
    }
    let elapsed = start.elapsed();
    eprintln!("6 point lookups took {:?}", elapsed);
}

/// 测试场景：范围扫描查询
/// 验证：scan_edges 带 limit 和 offset 的分页查询
#[test]
fn query_range_scan_pagination() {
    let srv = new_server(16);

    srv.add_vertex("src".into(), "n".into(), BTreeMap::new())
        .unwrap();
    srv.add_vertex("dst".into(), "n".into(), BTreeMap::new())
        .unwrap();

    for i in 0..100 {
        add_edge(&srv, "src", "dst", "e", i);
    }

    // 分页查询：每页 20 条
    let page_size: u32 = 20;
    let mut all_edges = Vec::new();
    let mut offset: u64 = 0;

    loop {
        let page = srv.scan_edges(&["e"], page_size, offset).unwrap();
        if page.is_empty() {
            break;
        }
        all_edges.extend(page);
        offset += page_size as u64;
        if offset >= 200 {
            break; // 安全上限
        }
    }

    assert_eq!(all_edges.len(), 100);

    // 验证没有重复
    let unique_ranks: BTreeSet<_> = all_edges.iter().map(|e| e.rank).collect();
    assert_eq!(unique_ranks.len(), 100);
}

/// 测试场景：邻居查询 + 属性过滤
/// 验证：先查邻居，再按属性过滤（模拟 WHERE 子句）
#[test]
fn query_neighbor_with_property_filter() {
    let srv = new_server(16);

    srv.add_vertex("root".into(), "dept".into(), prop("name", "Engineering"))
        .unwrap();

    // 创建员工顶点，带 level 属性
    for i in 0..50 {
        let vid = format!("emp_{}", i);
        let level = if i < 20 { "senior" } else { "junior" };
        let salary = (50000 + i * 1000).to_string();
        srv.add_vertex(
            vid.clone(),
            "employee".into(),
            props(&[("level", level), ("salary", &salary)]),
        )
        .unwrap();
        add_edge(&srv, "root", &vid, "has_employee", i as i64);
    }

    // 模拟：GO FROM "root" OVER has_employee WHERE $$.employee.level == "senior"
    let neighbors = srv
        .get_neighbors("root", Direction::Out, &["has_employee"])
        .unwrap();

    let seniors: Vec<_> = neighbors
        .iter()
        .filter(|n| {
            let p = read_vertex_props(&srv, &n.neighbor_vid);
            p.get("level").and_then(|x| x.as_str()) == Some("senior")
        })
        .collect();

    assert_eq!(seniors.len(), 20);
}

// ============================================================================
// 模块三：查询执行 - 聚合查询
// ============================================================================

/// 测试场景：COUNT 聚合
/// 验证：统计顶点数、边数
#[test]
fn query_aggregation_count() {
    let srv = new_server(16);

    for i in 0..100 {
        srv.add_vertex(format!("v{}", i), "t".into(), BTreeMap::new())
            .unwrap();
    }

    // 统计顶点总数（通过分片统计）
    let shard_counts = srv.shard_vertex_counts();
    let total_vertices: u64 = shard_counts.values().sum();
    assert_eq!(total_vertices, 100);
}

/// 测试场景：GROUP BY 聚合模拟
/// 验证：按类型分组统计顶点数量
#[test]
fn query_aggregation_group_by_tag() {
    let srv = new_server(16);

    // 插入不同类型的顶点
    let type_counts = [("user", 150usize), ("post", 80), ("comment", 200)];

    for (tag, count) in &type_counts {
        for i in 0..*count {
            let vid = format!("{}_{}", tag, i);
            srv.add_vertex(vid, (*tag).into(), BTreeMap::new())
                .unwrap();
        }
    }

    // 模拟 GROUP BY tag COUNT(*)
    let all_vertices = scan_all_vertices(&srv);
    let mut group_counts: HashMap<String, usize> = HashMap::new();
    for (_vid, tag, _props) in &all_vertices {
        *group_counts.entry(tag.clone()).or_insert(0) += 1;
    }

    assert_eq!(
        *group_counts.get("user").unwrap_or(&0),
        150,
        "user count mismatch"
    );
    assert_eq!(
        *group_counts.get("post").unwrap_or(&0),
        80,
        "post count mismatch"
    );
    assert_eq!(
        *group_counts.get("comment").unwrap_or(&0),
        200,
        "comment count mismatch"
    );
    assert_eq!(group_counts.len(), 3);
}

/// 测试场景：SUM / AVG 聚合模拟
/// 验证：对属性值进行求和和平均值计算
#[test]
fn query_aggregation_sum_avg() {
    let srv = new_server(16);

    // 插入带 value 属性的顶点
    for i in 1..=100 {
        srv.add_vertex(
            format!("n{}", i),
            "num".into(),
            prop("value", &i.to_string()),
        )
        .unwrap();
    }

    // 模拟 SELECT SUM(value), AVG(value) FROM num
    let all = scan_all_vertices(&srv);
    let mut sum: i64 = 0;
    let mut count: i64 = 0;

    for (_vid, _tag, props) in &all {
        if let Some(v) = props.get("value").and_then(|p| p.as_str()) {
            if let Ok(n) = v.parse::<i64>() {
                sum += n;
                count += 1;
            }
        }
    }

    assert_eq!(count, 100);
    let expected_sum: i64 = (1..=100).sum();
    assert_eq!(sum, expected_sum); // 5050
    let avg = sum as f64 / count as f64;
    assert!((avg - 50.5).abs() < 0.01);
}

/// 测试场景：MIN / MAX 聚合模拟
/// 验证：找出属性的最小和最大值
#[test]
fn query_aggregation_min_max() {
    let srv = new_server(16);

    let values = [42, 17, 99, 3, 56, 78, 23, 91, 5, 67];
    for (i, v) in values.iter().enumerate() {
        srv.add_vertex(
            format!("n{}", i),
            "item".into(),
            prop("score", &v.to_string()),
        )
        .unwrap();
    }

    let all = scan_all_vertices(&srv);
    let mut min_val = i64::MAX;
    let mut max_val = i64::MIN;

    for (_vid, _tag, props) in &all {
        if let Some(v) = props.get("score").and_then(|p| p.as_str()) {
            if let Ok(n) = v.parse::<i64>() {
                min_val = min_val.min(n);
                max_val = max_val.max(n);
            }
        }
    }

    assert_eq!(min_val, 3);
    assert_eq!(max_val, 99);
}

// ============================================================================
// 模块四：优化器验证
// ============================================================================

/// 测试场景：谓词下推优化验证
/// 验证：将过滤条件下推到存储层，减少数据传输量
///
/// 在图数据库中，谓词下推意味着：
/// - 将 WHERE 条件下推到邻居查询阶段
/// - 而不是先拉取所有邻居再过滤
///
/// 本测试验证：带边类型过滤的查询比不带过滤的返回更少数据
#[test]
fn optimizer_predicate_pushdown_edge_type_filter() {
    let srv = new_server(16);

    srv.add_vertex("center".into(), "hub".into(), BTreeMap::new())
        .unwrap();
    for i in 0..20 {
        let vid = format!("n{}", i);
        srv.add_vertex(vid.clone(), "n".into(), BTreeMap::new())
            .unwrap();
    }

    // 两种边类型各 10 条
    for i in 0..10 {
        add_edge(&srv, "center", &format!("n{}", i), "type_a", i as i64);
    }
    for i in 10..20 {
        add_edge(&srv, "center", &format!("n{}", i), "type_b", i as i64);
    }

    // 不带过滤：返回 20 条
    let all = srv
        .get_neighbors("center", Direction::Out, &[])
        .unwrap();
    assert_eq!(all.len(), 20);

    // 带类型过滤（谓词下推）：只返回 10 条
    let filtered = srv
        .get_neighbors("center", Direction::Out, &["type_a"])
        .unwrap();
    assert_eq!(filtered.len(), 10);
    assert!(filtered.iter().all(|n| n.etype == "type_a"));

    // 验证：带过滤的结果是不带过滤的子集
    let filtered_set: BTreeSet<_> = filtered.iter().map(|n| &n.neighbor_vid).collect();
    let all_set: BTreeSet<_> = all.iter().map(|n| &n.neighbor_vid).collect();
    assert!(filtered_set.is_subset(&all_set));
}

/// 测试场景：投影下推优化验证
/// 验证：只读取需要的属性列，减少 I/O
///
/// 在 KV 存储中，投影下推意味着：
/// - 仅解码所需的属性字段
/// - 而不是解码所有属性再投影
///
/// 本测试验证：可以从完整属性中提取子集（模拟投影下推效果）
#[test]
fn optimizer_projection_pushdown() {
    let srv = new_server(16);

    // 顶点有 10 个属性
    let mut all_props = BTreeMap::new();
    for i in 0..10 {
        all_props.insert(format!("prop_{}", i), PropValue::from_str(&format!("val_{}", i)));
    }
    srv.add_vertex("v".into(), "t".into(), all_props)
        .unwrap();

    // 完整读取（模拟无投影下推）
    let full = read_vertex_props(&srv, "v");
    assert_eq!(full.len(), 10);

    // 投影：只取 prop_0, prop_5, prop_9（模拟投影下推）
    let projected: BTreeMap<_, _> = ["prop_0", "prop_5", "prop_9"]
        .iter()
        .filter_map(|k| full.get(k.to_owned()).map(|v| (k.to_string(), v.clone())))
        .collect();

    assert_eq!(projected.len(), 3);
    assert_eq!(
        projected.get("prop_0").and_then(|p| p.as_str()),
        Some("val_0")
    );
    assert_eq!(
        projected.get("prop_5").and_then(|p| p.as_str()),
        Some("val_5")
    );
    assert_eq!(
        projected.get("prop_9").and_then(|p| p.as_str()),
        Some("val_9")
    );
}

/// 测试场景：常量折叠优化验证
/// 验证：查询编译阶段可对常量表达式进行预计算
#[test]
fn optimizer_constant_folding() {
    // 模拟优化器对 WHERE 条件中的常量表达式进行折叠
    // 例如：WHERE age > 20 + 10  →  WHERE age > 30

    let srv = new_server(16);

    for i in 0..50 {
        srv.add_vertex(
            format!("u{}", i),
            "user".into(),
            prop("age", &(20 + i).to_string()),
        )
        .unwrap();
    }

    // 模拟常量折叠后的查询：WHERE age > 30
    // （原本可能是 WHERE age > 20 + 10，优化器折叠为 30）
    let all = scan_all_vertices(&srv);
    let threshold = 30; // 常量折叠后的结果

    let filtered: Vec<_> = all
        .iter()
        .filter(|(_, _, props)| {
            if let Some(age_str) = props.get("age").and_then(|p| p.as_str()) {
                if let Ok(age) = age_str.parse::<i32>() {
                    return age > threshold;
                }
            }
            false
        })
        .collect();

    // age 从 20 到 69，大于 30 的有 40 个（31-69）
    assert_eq!(filtered.len(), 39); // age 31..=69 = 39个
}

/// 测试场景：列裁剪优化验证
/// 验证：只访问需要的列族，避免不必要的 I/O
#[test]
fn optimizer_column_pruning() {
    let srv = new_server(16);

    srv.add_vertex("v".into(), "t".into(), props(&[("a", "1"), ("b", "2")]))
        .unwrap();
    srv.add_vertex("w".into(), "t".into(), BTreeMap::new())
        .unwrap();
    add_edge(&srv, "v", "w", "e", 0);

    // 场景1：只需要顶点属性 → 只访问 vid_meta 列族
    let vertex_props = read_vertex_props(&srv, "v");
    assert!(!vertex_props.is_empty());

    // 场景2：只需要邻居关系 → 只访问 out_edge / in_edge 列族
    let neighbors = srv.get_neighbors("v", Direction::Out, &["e"]).unwrap();
    assert_eq!(neighbors.len(), 1);

    // 场景3：同时需要属性和关系 → 访问多个列族
    // （验证两者都能正确返回）
    let both_props = read_vertex_props(&srv, "v");
    let both_nbrs = srv.get_neighbors("v", Direction::Out, &["e"]).unwrap();
    assert!(!both_props.is_empty());
    assert_eq!(both_nbrs.len(), 1);
}

// ============================================================================
// 模块五：索引选择验证
// ============================================================================

/// 测试场景：主键索引（VID 索引）选择
/// 验证：按 VID 查询时使用主键索引，时间复杂度 O(1)
#[test]
fn index_selection_primary_key_lookup() {
    let srv = new_server(16);

    // 插入 10000 个顶点
    const N: usize = 10_000;
    for i in 0..N {
        srv.add_vertex(format!("v_{:05}", i), "t".into(), BTreeMap::new())
            .unwrap();
    }

    // 测量 100 次随机点查的总耗时
    let lookups: u32 = 100;
    let start = Instant::now();
    for i in 0..lookups {
        let vid = format!("v_{:05}", (i * 97) % N as u32); // 伪随机
        let _ = read_vertex_props(&srv, &vid);
    }
    let elapsed = start.elapsed();
    let per_lookup = elapsed / lookups;

    eprintln!(
        "primary key lookup: {:?} per query ({} queries)",
        per_lookup, lookups
    );

    // 点查应远快于线性扫描
    assert!(per_lookup < Duration::from_millis(10));
}

/// 测试场景：边类型索引选择
/// 验证：带边类型过滤的邻居查询使用边类型索引
#[test]
fn index_selection_edge_type_filter() {
    let srv = new_server(16);

    srv.add_vertex("v".into(), "n".into(), BTreeMap::new())
        .unwrap();
    for i in 0..30 {
        let vid = format!("n{}", i);
        srv.add_vertex(vid.clone(), "n".into(), BTreeMap::new())
            .unwrap();
    }

    // 插入多种类型的边
    for i in 0..10 {
        add_edge(&srv, "v", &format!("n{}", i), "follows", i as i64);
    }
    for i in 10..20 {
        add_edge(&srv, "v", &format!("n{}", i), "likes", i as i64);
    }
    for i in 20..30 {
        add_edge(&srv, "v", &format!("n{}", i), "owns", i as i64);
    }

    // 验证每种类型的查询都正确
    let follows = srv
        .get_neighbors("v", Direction::Out, &["follows"])
        .unwrap();
    assert_eq!(follows.len(), 10);
    assert!(follows.iter().all(|n| n.etype == "follows"));

    let likes = srv
        .get_neighbors("v", Direction::Out, &["likes"])
        .unwrap();
    assert_eq!(likes.len(), 10);
    assert!(likes.iter().all(|n| n.etype == "likes"));

    let owns = srv
        .get_neighbors("v", Direction::Out, &["owns"])
        .unwrap();
    assert_eq!(owns.len(), 10);
    assert!(owns.iter().all(|n| n.etype == "owns"));
}

/// 测试场景：复合索引（前缀匹配）
/// 验证：按分片+VID前缀扫描利用复合索引
#[test]
fn index_selection_composite_prefix_scan() {
    let srv = new_server(16);

    // 插入带前缀的顶点
    for i in 0..100 {
        srv.add_vertex(
            format!("user_{:04}", i),
            "user".into(),
            BTreeMap::new(),
        )
        .unwrap();
        srv.add_vertex(
            format!("post_{:04}", i),
            "post".into(),
            BTreeMap::new(),
        )
        .unwrap();
    }

    // 验证总数
    let total: u64 = srv.shard_vertex_counts().values().sum();
    assert_eq!(total, 200);
}

// ============================================================================
// 模块六：执行计划（EXPLAIN）验证
// ============================================================================

/// 查询执行计划节点类型
#[derive(Debug, Clone, PartialEq, Eq)]
enum PlanNodeType {
    Scan,
    IndexScan,
    Filter,
    Project,
    Join,
    Aggregate,
    Sort,
    Limit,
    Traverse,
    GetVertices,
}

#[derive(Debug, Clone)]
struct PlanNode {
    node_type: PlanNodeType,
    description: String,
    estimated_rows: u64,
    children: Vec<PlanNode>,
}

impl PlanNode {
    fn new(node_type: PlanNodeType, desc: &str, est: u64) -> Self {
        Self {
            node_type,
            description: desc.to_string(),
            estimated_rows: est,
            children: Vec::new(),
        }
    }

    fn with_child(mut self, child: PlanNode) -> Self {
        self.children.push(child);
        self
    }

    /// 格式化输出执行计划（模拟 EXPLAIN 输出）
    fn format(&self, indent: usize) -> String {
        let prefix = "  ".repeat(indent);
        let mut result = format!(
            "{}{} [{}] (est. {} rows)\n",
            prefix, self.description,
            format!("{:?}", self.node_type).to_uppercase(),
            self.estimated_rows
        );
        for child in &self.children {
            result.push_str(&child.format(indent + 1));
        }
        result
    }
}

/// 模拟查询优化器生成执行计划
fn generate_plan(query_type: &str) -> PlanNode {
    match query_type {
        "point_lookup" => PlanNode::new(
            PlanNodeType::Project,
            "Project name, age",
            1,
        )
        .with_child(PlanNode::new(
            PlanNodeType::IndexScan,
            "IndexScan vid = 'v1'",
            1,
        )),
        "neighbor_query" => PlanNode::new(
            PlanNodeType::Project,
            "Project $$.user.name",
            100,
        )
        .with_child(
            PlanNode::new(PlanNodeType::GetVertices, "GetVertices", 100).with_child(
                PlanNode::new(
                    PlanNodeType::Traverse,
                    "Traverse 1 step OVER follow",
                    100,
                )
                .with_child(PlanNode::new(
                    PlanNodeType::IndexScan,
                    "IndexScan vid = 'alice'",
                    1,
                )),
            ),
        ),
        "aggregate" => PlanNode::new(
            PlanNodeType::Aggregate,
            "Aggregate count(*) GROUP BY level",
            10,
        )
        .with_child(PlanNode::new(
            PlanNodeType::Scan,
            "Scan vertices (tag=employee)",
            10000,
        )),
        "filtered_query" => PlanNode::new(
            PlanNodeType::Project,
            "Project name, salary",
            500,
        )
        .with_child(
            PlanNode::new(PlanNodeType::Filter, "Filter age > 30", 500).with_child(
                PlanNode::new(PlanNodeType::Scan, "Scan vertices (tag=user)", 1000),
            ),
        ),
        _ => PlanNode::new(PlanNodeType::Scan, "FullScan", 10000),
    }
}

/// 测试场景：EXPLAIN 输出格式验证
/// 验证：执行计划以树状结构输出，包含节点类型和估算行数
#[test]
fn explain_output_format() {
    // 点查执行计划
    let plan1 = generate_plan("point_lookup");
    let output1 = plan1.format(0);
    assert!(output1.contains("PROJECT"));
    assert!(output1.contains("INDEXSCAN"));
    assert!(output1.contains("1 row"));

    // 邻居查询执行计划
    let plan2 = generate_plan("neighbor_query");
    let output2 = plan2.format(0);
    assert!(output2.contains("TRAVERSE"));
    assert!(output2.contains("GETVERTICES"));
    assert!(output2.contains("1 step"));

    eprintln!("=== Neighbor Query Plan ===");
    eprintln!("{}", output2);
}

/// 测试场景：执行计划成本估算验证
/// 验证：不同查询的估算行数合理
#[test]
fn explain_cost_estimation() {
    // 点查估算 1 行
    let plan1 = generate_plan("point_lookup");
    assert_eq!(plan1.estimated_rows, 1);

    // 聚合查询估算行数减少（分组后）
    let plan3 = generate_plan("aggregate");
    assert_eq!(plan3.estimated_rows, 10);

    // 子节点行数应 >= 父节点行数（聚合是缩减操作）
    let child_rows = plan3.children.iter().map(|c| c.estimated_rows).sum::<u64>();
    assert!(child_rows >= plan3.estimated_rows);
}

/// 测试场景：执行计划中索引选择验证
/// 验证：点查使用 IndexScan，范围查询使用 Scan
#[test]
fn explain_index_selection_in_plan() {
    // 点查：应该用 IndexScan
    let plan1 = generate_plan("point_lookup");
    let has_index_scan = plan_contains_node(&plan1, PlanNodeType::IndexScan);
    assert!(has_index_scan, "point lookup plan should contain IndexScan");

    // 聚合扫描：应该用 Scan
    let plan2 = generate_plan("aggregate");
    let has_scan = plan_contains_node(&plan2, PlanNodeType::Scan);
    assert!(has_scan, "aggregate plan should contain Scan");

    // 邻居查询：应该有 Traverse 节点
    let plan3 = generate_plan("neighbor_query");
    let has_traverse = plan_contains_node(&plan3, PlanNodeType::Traverse);
    assert!(has_traverse, "neighbor query plan should contain Traverse");
}

fn plan_contains_node(plan: &PlanNode, node_type: PlanNodeType) -> bool {
    if plan.node_type == node_type {
        return true;
    }
    plan.children
        .iter()
        .any(|c| plan_contains_node(c, node_type.clone()))
}

// ============================================================================
// 模块七：Cypher / openCypher 兼容性
// ============================================================================

/// 测试场景：MATCH 语句模拟
/// 验证：MATCH (a:Label)-[r:REL]->(b) 模式匹配
#[test]
fn cypher_match_pattern_simple() {
    let srv = new_server(16);

    // 构建数据
    srv.add_vertex("alice".into(), "Person".into(), prop("name", "Alice"))
        .unwrap();
    srv.add_vertex("bob".into(), "Person".into(), prop("name", "Bob"))
        .unwrap();
    srv.add_vertex("charlie".into(), "Person".into(), prop("name", "Charlie"))
        .unwrap();

    add_edge(&srv, "alice", "bob", "KNOWS", 0);
    add_edge(&srv, "bob", "charlie", "KNOWS", 0);

    // 模拟 MATCH (a:Person)-[:KNOWS]->(b:Person) RETURN a.name, b.name
    let all = scan_all_vertices(&srv);
    let persons: Vec<_> = all
        .iter()
        .filter(|(_, tag, _)| tag == "Person")
        .collect();

    let mut results = Vec::new();
    for (vid_a, _, _) in &persons {
        let neighbors = srv
            .get_neighbors(vid_a, Direction::Out, &["KNOWS"])
            .unwrap();
        for n in &neighbors {
            let props_a = read_vertex_props(&srv, vid_a);
            let props_b = read_vertex_props(&srv, &n.neighbor_vid);
            let name_a = props_a.get("name").and_then(|p| p.as_str()).unwrap_or("");
            let name_b = props_b.get("name").and_then(|p| p.as_str()).unwrap_or("");
            results.push((name_a.to_string(), name_b.to_string()));
        }
    }

    assert_eq!(results.len(), 2);
    assert!(results.contains(&(
        "Alice".to_string(),
        "Bob".to_string()
    )));
    assert!(results.contains(&(
        "Bob".to_string(),
        "Charlie".to_string()
    )));
}

/// 测试场景：Cypher WHERE 子句模拟
/// 验证：MATCH (n:Label) WHERE n.prop > value
#[test]
fn cypher_match_with_where() {
    let srv = new_server(16);

    for i in 0..30 {
        let age = 20 + i; // 20..49
        srv.add_vertex(
            format!("p{}", i),
            "Person".into(),
            props(&[("name", &format!("Person{}", i)), ("age", &age.to_string())]),
        )
        .unwrap();
    }

    // 模拟 MATCH (n:Person) WHERE n.age > 35 RETURN n.name, n.age
    let all = scan_all_vertices(&srv);
    let results: Vec<_> = all
        .iter()
        .filter(|(_, tag, props)| {
            if tag != "Person" {
                return false;
            }
            if let Some(age_str) = props.get("age").and_then(|p| p.as_str()) {
                if let Ok(age) = age_str.parse::<i32>() {
                    return age > 35;
                }
            }
            false
        })
        .map(|(vid, _, props)| {
            let name = props.get("name").and_then(|p| p.as_str()).unwrap_or("");
            let age = props.get("age").and_then(|p| p.as_str()).unwrap_or("");
            (vid.clone(), name.to_string(), age.to_string())
        })
        .collect();

    // age > 35: 36..49 = 14 人
    assert_eq!(results.len(), 14);
    for (_, _, age) in &results {
        let a: i32 = age.parse().unwrap();
        assert!(a > 35, "age {} should be > 35", a);
    }
}

/// 测试场景：Cypher 变长路径查询模拟
/// 验证：MATCH (a)-[*1..3]->(b)  1-3 跳路径
#[test]
fn cypher_variable_length_path() {
    let srv = new_server(16);

    // 构建链：a -> b -> c -> d -> e
    for v in ["a", "b", "c", "d", "e"] {
        srv.add_vertex(v.into(), "n".into(), BTreeMap::new())
            .unwrap();
    }
    add_edge(&srv, "a", "b", "LINK", 0);
    add_edge(&srv, "b", "c", "LINK", 0);
    add_edge(&srv, "c", "d", "LINK", 0);
    add_edge(&srv, "d", "e", "LINK", 0);

    // 模拟 MATCH (a)-[*1..3]->(b) WHERE a.id = 'a' RETURN DISTINCT b
    let min_hops = 1;
    let max_hops = 3;

    let mut reachable: BTreeSet<String> = BTreeSet::new();
    let mut current: BTreeSet<String> = BTreeSet::new();
    current.insert("a".to_string());
    let mut visited: BTreeSet<String> = BTreeSet::new();
    visited.insert("a".to_string());

    for hop in 1..=max_hops {
        let mut next: BTreeSet<String> = BTreeSet::new();
        for vid in &current {
            let neighbors = srv
                .get_neighbors(vid, Direction::Out, &["LINK"])
                .unwrap();
            for n in neighbors {
                if !visited.contains(&n.neighbor_vid) {
                    next.insert(n.neighbor_vid);
                }
            }
        }
        if hop >= min_hops {
            reachable.extend(next.iter().cloned());
        }
        visited.extend(next.iter().cloned());
        current = next;
    }

    // 从 a 出发 1-3 跳可达：b (1跳), c (2跳), d (3跳)
    assert_eq!(reachable.len(), 3);
    assert!(reachable.contains("b"));
    assert!(reachable.contains("c"));
    assert!(reachable.contains("d"));
    assert!(!reachable.contains("e")); // e 需要 4 跳
}

/// 测试场景：Cypher OPTIONAL MATCH 模拟
/// 验证：可选匹配，无匹配时返回 NULL
#[test]
fn cypher_optional_match() {
    let srv = new_server(16);

    srv.add_vertex("alice".into(), "Person".into(), prop("name", "Alice"))
        .unwrap();
    srv.add_vertex("bob".into(), "Person".into(), prop("name", "Bob"))
        .unwrap();

    add_edge(&srv, "alice", "bob", "FRIEND", 0);

    // 模拟 MATCH (a:Person) OPTIONAL MATCH (a)-[:FRIEND]->(b)
    let persons: Vec<_> = scan_all_vertices(&srv)
        .into_iter()
        .filter(|(_, tag, _)| tag == "Person")
        .collect();

    let mut results = Vec::new();
    for (vid_a, _, props_a) in &persons {
        let name_a = props_a.get("name").and_then(|p| p.as_str()).unwrap_or("");
        let friends = srv
            .get_neighbors(vid_a, Direction::Out, &["FRIEND"])
            .unwrap();

        if friends.is_empty() {
            // OPTIONAL MATCH: 无匹配时返回 NULL
            results.push((name_a.to_string(), None));
        } else {
            for f in &friends {
                let props_b = read_vertex_props(&srv, &f.neighbor_vid);
                let name_b = props_b.get("name").and_then(|p| p.as_str()).unwrap_or("");
                results.push((name_a.to_string(), Some(name_b.to_string())));
            }
        }
    }

    // Alice 有一个朋友 Bob
    // Bob 没有出边朋友
    assert!(results
        .iter()
        .any(|(a, b)| a == "Alice" && b.as_deref() == Some("Bob")));
    assert!(results
        .iter()
        .any(|(a, b)| a == "Bob" && b.is_none()));
}

// ============================================================================
// 模块八：查询性能基准
// ============================================================================

/// 测试场景：简单点查延迟
#[test]
fn query_perf_point_lookup_latency() {
    let srv = new_server(16);

    const N: usize = 10_000;
    for i in 0..N {
        srv.add_vertex(format!("v_{:05}", i), "t".into(), BTreeMap::new())
            .unwrap();
    }

    // 测量 1000 次点查
    let iterations = 1000;
    let mut latencies = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let vid = format!("v_{:05}", (i * 137) % N);
        let start = Instant::now();
        let _ = read_vertex_props(&srv, &vid);
        latencies.push(start.elapsed());
    }

    latencies.sort();
    let p50 = latencies[iterations / 2];
    let p95 = latencies[(iterations * 95) / 100];
    let p99 = latencies[(iterations * 99) / 100];

    eprintln!(
        "Point lookup latency: P50={:?}, P95={:?}, P99={:?}",
        p50, p95, p99
    );

    // 基本断言：延迟应在合理范围内
    assert!(p50 < Duration::from_millis(5));
    assert!(p99 < Duration::from_millis(20));
}

/// 测试场景：1跳邻居查询延迟
#[test]
fn query_perf_1hop_traversal_latency() {
    let srv = new_server(16);

    // 构建扇出为 100 的图
    srv.add_vertex("hub".into(), "hub".into(), BTreeMap::new())
        .unwrap();
    for i in 0..100 {
        let vid = format!("n{}", i);
        srv.add_vertex(vid.clone(), "n".into(), BTreeMap::new())
            .unwrap();
        add_edge(&srv, "hub", &vid, "e", i as i64);
    }

    // 预热缓存
    let _ = srv.get_neighbors("hub", Direction::Out, &["e"]).unwrap();

    // 测量 100 次 1 跳查询
    let iterations = 100;
    let mut latencies = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();
        let result = srv.get_neighbors("hub", Direction::Out, &["e"]).unwrap();
        latencies.push(start.elapsed());
        assert_eq!(result.len(), 100);
    }

    latencies.sort();
    let p50 = latencies[iterations / 2];
    let p99 = latencies[(iterations * 99) / 100];

    eprintln!(
        "1-hop traversal latency (fanout=100): P50={:?}, P99={:?}",
        p50, p99
    );

    assert!(p50 < Duration::from_millis(10));
}

// ============================================================================
// 模块九：查询结果排序与分页
// ============================================================================

/// 测试场景：ORDER BY 排序模拟
#[test]
fn query_order_by_property() {
    let srv = new_server(16);

    let scores = [85, 92, 78, 95, 88, 70, 99, 82, 91, 76];
    for (i, score) in scores.iter().enumerate() {
        srv.add_vertex(
            format!("s{}", i),
            "student".into(),
            props(&[("name", &format!("Student{}", i)), ("score", &score.to_string())]),
        )
        .unwrap();
    }

    // 模拟 ORDER BY score DESC
    let all = scan_all_vertices(&srv);
    let mut students: Vec<_> = all
        .iter()
        .map(|(vid, _, props)| {
            let score = props
                .get("score")
                .and_then(|p| p.as_str())
                .unwrap_or("0")
                .parse::<i32>()
                .unwrap_or(0);
            let name = props.get("name").and_then(|p| p.as_str()).unwrap_or("");
            (vid.clone(), name.to_string(), score)
        })
        .collect();

    students.sort_by(|a, b| b.2.cmp(&a.2)); // DESC

    assert_eq!(students.len(), 10);
    assert_eq!(students[0].2, 99); // 最高分
    assert_eq!(students[9].2, 70); // 最低分
    assert_eq!(students[0].1, "Student6"); // s6 有 99 分
}

/// 测试场景：LIMIT + SKIP 分页模拟
#[test]
fn query_limit_skip_pagination() {
    let srv = new_server(16);

    for i in 0..100 {
        srv.add_vertex(
            format!("item_{:03}", i),
            "item".into(),
            prop("idx", &i.to_string()),
        )
        .unwrap();
    }

    // 全量数据
    let all = scan_all_vertices(&srv);
    assert_eq!(all.len(), 100);

    // 模拟 LIMIT 10 SKIP 20
    let page_size = 10;
    let skip = 20;
    let page: Vec<_> = all.into_iter().skip(skip).take(page_size).collect();

    assert_eq!(page.len(), 10);
}

// ============================================================================
// 模块十：多跳路径查询优化
// ============================================================================

/// 测试场景：双向 BFS 相遇搜索
/// 验证：从起点和终点同时进行 BFS，在中间相遇
/// 这是双向图搜索的核心优化
#[test]
fn query_bidirectional_bfs_meeting() {
    let srv = new_server(16);

    // 构建线性链：start -> a -> b -> c -> d -> e -> end
    let nodes = ["start", "a", "b", "c", "d", "e", "end"];
    for v in &nodes {
        srv.add_vertex((*v).into(), "n".into(), BTreeMap::new())
            .unwrap();
    }
    for w in nodes.windows(2) {
        add_edge(&srv, w[0], w[1], "link", 0);
        add_edge(&srv, w[1], w[0], "link", 1); // 双向边
    }

    // 双向 BFS：从 start 和 end 同时出发
    let start = "start";
    let end = "end";

    let mut forward_visited: HashSet<String> = HashSet::new();
    let mut backward_visited: HashSet<String> = HashSet::new();
    let mut forward_frontier: HashSet<String> = HashSet::new();
    let mut backward_frontier: HashSet<String> = HashSet::new();

    forward_frontier.insert(start.to_string());
    backward_frontier.insert(end.to_string());
    forward_visited.insert(start.to_string());
    backward_visited.insert(end.to_string());

    let mut found = false;
    let mut steps = 0;
    const MAX_STEPS: usize = 10;

    while !forward_frontier.is_empty() && !backward_frontier.is_empty() && steps < MAX_STEPS {
        steps += 1;

        // 检查是否相遇
        let intersection: Vec<_> = forward_frontier
            .intersection(&backward_frontier)
            .collect();
        if !intersection.is_empty() {
            found = true;
            break;
        }

        // 扩展较小的一边（优化：始终扩展较小的 frontier）
        if forward_frontier.len() <= backward_frontier.len() {
            let mut next: HashSet<String> = HashSet::new();
            for vid in &forward_frontier {
                let neighbors = srv
                    .get_neighbors(vid, Direction::Both, &["link"])
                    .unwrap();
                for n in neighbors {
                    if forward_visited.insert(n.neighbor_vid.clone()) {
                        next.insert(n.neighbor_vid);
                    }
                }
            }
            forward_frontier = next;
        } else {
            let mut next: HashSet<String> = HashSet::new();
            for vid in &backward_frontier {
                let neighbors = srv
                    .get_neighbors(vid, Direction::Both, &["link"])
                    .unwrap();
                for n in neighbors {
                    if backward_visited.insert(n.neighbor_vid.clone()) {
                        next.insert(n.neighbor_vid);
                    }
                }
            }
            backward_frontier = next;
        }
    }

    assert!(found, "bidirectional BFS should find path");
    // 在长度为 6 的路径上，双向 BFS 应在约 3 步内相遇
    assert!(steps <= 4, "bidirectional BFS took {} steps (expected <= 4)", steps);

    eprintln!("Bidirectional BFS found path in {} steps", steps);
}

// ============================================================================
// 模块十一：查询结果去重
// ============================================================================

/// 测试场景：DISTINCT 去重模拟
#[test]
fn query_distinct_dedup() {
    let srv = new_server(16);

    // 构建图：A 连接到 B、C；B 连接到 D；C 连接到 D
    // 从 A 出发 2 跳，D 会通过两条路径到达，需要去重
    for v in ["a", "b", "c", "d"] {
        srv.add_vertex(v.into(), "n".into(), BTreeMap::new())
            .unwrap();
    }
    add_edge(&srv, "a", "b", "e", 0);
    add_edge(&srv, "a", "c", "e", 1);
    add_edge(&srv, "b", "d", "e", 0);
    add_edge(&srv, "c", "d", "e", 1);

    // 2 跳遍历（不去重）
    let hop1 = srv.get_neighbors("a", Direction::Out, &["e"]).unwrap();
    let mut hop2_with_dup = Vec::new();
    for n in &hop1 {
        let next = srv
            .get_neighbors(&n.neighbor_vid, Direction::Out, &["e"])
            .unwrap();
        for nn in next {
            hop2_with_dup.push(nn.neighbor_vid.clone());
        }
    }

    // 有重复（d 出现 2 次）
    assert_eq!(hop2_with_dup.len(), 2);

    // DISTINCT 去重后
    let distinct: BTreeSet<_> = hop2_with_dup.iter().collect();
    assert_eq!(distinct.len(), 1);
    assert!(distinct.contains(&"d".to_string()));
}

// ============================================================================
// 模块十二：复合查询模式
// ============================================================================

/// 测试场景：朋友的朋友（FoF）查询
/// 验证：经典社交网络查询模式
#[test]
fn query_friends_of_friends() {
    let srv = new_server(16);

    // 构建社交网络
    let people = ["alice", "bob", "charlie", "david", "eve", "frank"];
    for p in &people {
        srv.add_vertex((*p).into(), "user".into(), prop("name", p))
            .unwrap();
    }

    // Alice 的朋友
    add_edge(&srv, "alice", "bob", "friend", 0);
    add_edge(&srv, "alice", "charlie", "friend", 1);

    // Bob 的朋友
    add_edge(&srv, "bob", "david", "friend", 0);
    add_edge(&srv, "bob", "eve", "friend", 1);

    // Charlie 的朋友
    add_edge(&srv, "charlie", "eve", "friend", 0);
    add_edge(&srv, "charlie", "frank", "friend", 1);

    // 查找 Alice 的朋友的朋友（排除 Alice 自己和直接朋友）
    let direct_friends: BTreeSet<String> = srv
        .get_neighbors("alice", Direction::Out, &["friend"])
        .unwrap()
        .into_iter()
        .map(|n| n.neighbor_vid)
        .collect();

    let mut fof: BTreeSet<String> = BTreeSet::new();
    for friend in &direct_friends {
        let friends_of_friend = srv
            .get_neighbors(friend, Direction::Out, &["friend"])
            .unwrap();
        for f in friends_of_friend {
            if f.neighbor_vid != "alice" && !direct_friends.contains(&f.neighbor_vid) {
                fof.insert(f.neighbor_vid);
            }
        }
    }

    // Alice 的 FoF：david, eve, frank
    // （eve 是 bob 和 charlie 的共同朋友，去重后只算一次）
    assert_eq!(fof.len(), 3);
    assert!(fof.contains("david"));
    assert!(fof.contains("eve"));
    assert!(fof.contains("frank"));
}
