// 集成测试：统一算法核心库跨域一致性验证

use mox_unified_algo_core::*;
use mox_unified_algo_core::similarity::*;
use mox_unified_algo_core::ranking::*;
use mox_unified_algo_core::graph::*;
use mox_unified_algo_core::utils::*;
use mox_unified_algo_core::traits::*;
use petgraph::Graph;
use std::collections::HashMap;

// ============================================================================
// 测试 1：相似度算法一致性
// ============================================================================

#[test]
fn test_cosine_similarity_consistency() {
    let sim = CosineSimilarity::default();

    // 场景：专家画像向量匹配（EA 域）
    let expert_profile = vec![0.8, 0.6, 0.9, 0.3];
    let task_requirement = vec![0.7, 0.5, 0.8, 0.4];
    let ea_score = sim.similarity(&expert_profile, &task_requirement);

    // 场景：文档向量检索（Cloud 域）
    let doc_vector = vec![0.8, 0.6, 0.9, 0.3];
    let query_vector = vec![0.7, 0.5, 0.8, 0.4];
    let cloud_score = sim.similarity(&doc_vector, &query_vector);

    // 场景：节点向量相似度（KG 域）
    let node_vec_a = vec![0.8, 0.6, 0.9, 0.3];
    let node_vec_b = vec![0.7, 0.5, 0.8, 0.4];
    let kg_score = sim.similarity(&node_vec_a, &node_vec_b);

    // 三域使用同一算法，结果必须完全一致
    assert!((ea_score - cloud_score).abs() < 1e-10);
    assert!((cloud_score - kg_score).abs() < 1e-10);
    assert!(ea_score > 0.95); // 高相似度预期
}

#[test]
fn test_jaccard_similarity_cross_domain() {
    let sim = JaccardSimilarity;

    // EA 域：专家技能标签匹配
    let expert_skills: Vec<String> = vec![
        "Rust".into(), "Python".into(), "图算法".into(), "分布式".into(),
    ];
    let task_skills: Vec<String> = vec![
        "Rust".into(), "图算法".into(), "机器学习".into(),
    ];
    let ea_sim = sim.similarity(&expert_skills, &task_skills);

    // Cloud 域：文档标签重叠
    let doc_tags: Vec<String> = vec![
        "Rust".into(), "Python".into(), "图算法".into(), "分布式".into(),
    ];
    let search_tags: Vec<String> = vec![
        "Rust".into(), "图算法".into(), "机器学习".into(),
    ];
    let cloud_sim = sim.similarity(&doc_tags, &search_tags);

    // KG 域：节点共同邻居
    let neighbors_a: Vec<usize> = vec![1, 2, 3, 5];
    let neighbors_b: Vec<usize> = vec![1, 3, 4];
    let kg_sim = sim.similarity(&neighbors_a, &neighbors_b);

    // 字符串集合和数字集合结果应一致（相同的交集/并集比例）
    assert!((ea_sim - cloud_sim).abs() < 1e-10);
    // 2/5 = 0.4
    assert!((kg_sim - 0.4).abs() < 1e-6);
    assert!((ea_sim - kg_sim).abs() < 1e-6);
}

// ============================================================================
// 测试 2：排名算法跨域一致性
// ============================================================================

#[test]
fn test_weighted_ranking_cross_domain() {
    let ranker = WeightedRanker::new(vec![
        ("relevance".to_string(), 0.5),
        ("quality".to_string(), 0.3),
        ("recency".to_string(), 0.2),
    ]);

    // EA 域：专家匹配排名
    let experts: Vec<(String, HashMap<String, f64>)> = vec![
        (
            "expert1".to_string(),
            HashMap::from([
                ("relevance".to_string(), 0.9),
                ("quality".to_string(), 0.8),
                ("recency".to_string(), 0.7),
            ]),
        ),
        (
            "expert2".to_string(),
            HashMap::from([
                ("relevance".to_string(), 0.7),
                ("quality".to_string(), 0.9),
                ("recency".to_string(), 0.8),
            ]),
        ),
    ];
    let ea_result = ranker.rank(&experts);

    // Cloud 域：文档搜索排名
    let docs: Vec<(String, HashMap<String, f64>)> = vec![
        (
            "doc1".to_string(),
            HashMap::from([
                ("relevance".to_string(), 0.9),
                ("quality".to_string(), 0.8),
                ("recency".to_string(), 0.7),
            ]),
        ),
        (
            "doc2".to_string(),
            HashMap::from([
                ("relevance".to_string(), 0.7),
                ("quality".to_string(), 0.9),
                ("recency".to_string(), 0.8),
            ]),
        ),
    ];
    let cloud_result = ranker.rank(&docs);

    // KG 域：节点推荐排名
    let nodes: Vec<(String, HashMap<String, f64>)> = vec![
        (
            "node1".to_string(),
            HashMap::from([
                ("relevance".to_string(), 0.9),
                ("quality".to_string(), 0.8),
                ("recency".to_string(), 0.7),
            ]),
        ),
        (
            "node2".to_string(),
            HashMap::from([
                ("relevance".to_string(), 0.7),
                ("quality".to_string(), 0.9),
                ("recency".to_string(), 0.8),
            ]),
        ),
    ];
    let kg_result = ranker.rank(&nodes);

    // 三域排名结果的得分模式应一致
    assert!((ea_result.items[0].score - cloud_result.items[0].score).abs() < 1e-10);
    assert!((cloud_result.items[0].score - kg_result.items[0].score).abs() < 1e-10);
    assert_eq!(ea_result.total, cloud_result.total);
    assert_eq!(cloud_result.total, kg_result.total);
}

#[test]
fn test_borda_fusion_expert_consensus() {
    let fusion = BordaFusion;

    // 模拟多专家推荐结果融合
    let expert1_rank = vec!["A".to_string(), "B".to_string(), "C".to_string()];
    let expert2_rank = vec!["B".to_string(), "A".to_string(), "C".to_string()];
    let expert3_rank = vec!["A".to_string(), "C".to_string(), "B".to_string()];

    let result = fusion.fuse(&[expert1_rank, expert2_rank, expert3_rank]);

    // A 应该是第一名（两个第一，一个第二）
    assert_eq!(result.items[0].key, "A");
    assert_eq!(result.items[0].rank, 1);
    assert_eq!(result.total, 3);
}

// ============================================================================
// 测试 3：图算法跨域一致性
// ============================================================================

#[test]
fn test_graph_algorithms_cross_domain() {
    let engine = UnifiedGraphEngine;

    // EA 域：专家协作关系图
    let mut expert_graph: Graph<&str, f64> = Graph::new();
    let e1 = expert_graph.add_node("算法专家");
    let e2 = expert_graph.add_node("架构专家");
    let e3 = expert_graph.add_node("数据专家");
    let e4 = expert_graph.add_node("产品专家");
    expert_graph.add_edge(e1, e2, 1.0);
    expert_graph.add_edge(e2, e3, 1.0);
    expert_graph.add_edge(e1, e3, 0.5);
    expert_graph.add_edge(e3, e4, 1.0);

    // KG 域：概念关系图（同构的图结构）
    let mut kg_graph: Graph<i32, f64> = Graph::new();
    let n1 = kg_graph.add_node(100);
    let n2 = kg_graph.add_node(200);
    let n3 = kg_graph.add_node(300);
    let n4 = kg_graph.add_node(400);
    kg_graph.add_edge(n1, n2, 1.0);
    kg_graph.add_edge(n2, n3, 1.0);
    kg_graph.add_edge(n1, n3, 0.5);
    kg_graph.add_edge(n3, n4, 1.0);

    // PageRank 结果应一致（同构图）
    let ea_pr = engine.pagerank(&expert_graph, 0.85, 50, 1e-6);
    let kg_pr = engine.pagerank(&kg_graph, 0.85, 50, 1e-6);

    for i in 0..ea_pr.len() {
        assert!((ea_pr[i] - kg_pr[i]).abs() < 1e-6, "节点 {} PageRank 不一致", i);
    }

    // 度中心性应一致
    let ea_dc = engine.degree_centrality(&expert_graph);
    let kg_dc = engine.degree_centrality(&kg_graph);
    for i in 0..ea_dc.len() {
        assert!((ea_dc[i] - kg_dc[i]).abs() < 1e-6, "节点 {} 度中心性不一致", i);
    }
}

#[test]
fn test_personalized_pagerank_activation() {
    let engine = UnifiedGraphEngine;
    let mut g: Graph<&str, f64> = Graph::new();
    let a = g.add_node("A");
    let b = g.add_node("B");
    let c = g.add_node("C");
    let d = g.add_node("D");
    g.add_edge(a, b, 1.0);
    g.add_edge(b, c, 1.0);
    g.add_edge(c, d, 1.0);
    g.add_edge(a, c, 1.0);

    // 从 A 激活扩散
    let ranks_from_a = engine.personalized_pagerank(&g, &[a.index()], 0.85, 50, 1e-6);
    // A 应该得分最高
    assert!(ranks_from_a[a.index()] > ranks_from_a[b.index()]);
    assert!(ranks_from_a[b.index()] > ranks_from_a[d.index()]);

    // 从 D 激活扩散
    let ranks_from_d = engine.personalized_pagerank(&g, &[d.index()], 0.85, 50, 1e-6);
    // D 应该得分最高
    assert!(ranks_from_d[d.index()] > ranks_from_d[c.index()]);
}

#[test]
fn test_shortest_path_cross_domain() {
    let engine = UnifiedGraphEngine;

    // EA 域：专家之间最短协作路径
    let mut g: Graph<&str, f64> = Graph::new();
    let a = g.add_node("A");
    let b = g.add_node("B");
    let c = g.add_node("C");
    let d = g.add_node("D");
    g.add_edge(a, b, 2.0);
    g.add_edge(b, c, 1.0);
    g.add_edge(a, c, 5.0);
    g.add_edge(c, d, 1.0);

    let result = engine.dijkstra(&g, a.index(), d.index());
    assert!(result.is_some());
    let (path, dist) = result.unwrap();

    // 最短路径: A -> B -> C -> D，总距离 2+1+1 = 4
    assert_eq!(path.len(), 4);
    assert_eq!(path[0], 0); // A
    assert_eq!(path[1], 1); // B
    assert_eq!(path[2], 2); // C
    assert_eq!(path[3], 3); // D
    assert!((dist - 4.0).abs() < 1e-6);
}

// ============================================================================
// 测试 4：算法归一化验证
// ============================================================================

#[test]
fn test_algorithm_registry() {
    use mox_unified_algo_core::registry::*;

    // 注册内置算法
    register_builtin_algorithms();

    // 验证各分类算法数量
    let graph_algos = global_algo_registry().list_by_category(&AlgoCategory::Graph);
    assert!(graph_algos.len() >= 4, "图算法至少应有 4 种");

    let sim_algos = global_algo_registry().list_by_category(&AlgoCategory::Similarity);
    assert!(sim_algos.len() >= 2, "相似度算法至少应有 2 种");

    let rank_algos = global_algo_registry().list_by_category(&AlgoCategory::Ranking);
    assert!(rank_algos.len() >= 2, "排序算法至少应有 2 种");

    let fusion_algos = global_algo_registry().list_by_category(&AlgoCategory::Fusion);
    assert!(fusion_algos.len() >= 2, "融合算法至少应有 2 种");

    // 验证算法查询
    let pagerank = global_algo_registry().get("graph.pagerank");
    assert!(pagerank.is_some());
    assert_eq!(pagerank.unwrap().name, "PageRank");
}

#[test]
fn test_global_constants_consistency() {
    // 验证全局参数与 KG 域原有参数一致
    use mox_unified_algo_core::{PPR_DAMPING, PPR_MAX_ITER};

    // 阻尼因子统一为 0.85
    assert!((PPR_DAMPING - 0.85).abs() < 1e-10);
    // 最大迭代统一为 30
    assert_eq!(PPR_MAX_ITER, 30);

    // 验证算法引擎使用了统一参数
    let engine = UnifiedGraphEngine;
    let mut g: Graph<&str, f64> = Graph::new();
    g.add_node("A");
    g.add_node("B");
    g.add_edge(g.node_indices().next().unwrap(), g.node_indices().nth(1).unwrap(), 1.0);

    let ranks = engine.pagerank(&g, PPR_DAMPING, PPR_MAX_ITER, 1e-6);
    assert_eq!(ranks.len(), 2);
}

// ============================================================================
// 测试 5：工具函数一致性
// ============================================================================

#[test]
fn test_utils_cross_domain() {
    // Min-Max 归一化
    let mut ea_scores = vec![10.0, 20.0, 30.0];
    let mut kg_scores = vec![10.0, 20.0, 30.0];
    let mut cloud_scores = vec![10.0, 20.0, 30.0];

    min_max_normalize(&mut ea_scores);
    min_max_normalize(&mut kg_scores);
    min_max_normalize(&mut cloud_scores);

    for i in 0..3 {
        assert!((ea_scores[i] - kg_scores[i]).abs() < 1e-10);
        assert!((kg_scores[i] - cloud_scores[i]).abs() < 1e-10);
    }

    // Top-K
    let scores = vec![0.3, 0.9, 0.5, 0.7, 0.1];
    let top3 = top_k_indices(&scores, 3);
    assert_eq!(top3.len(), 3);
    assert_eq!(top3[0], 1); // 0.9 最高
}

// ============================================================================
// 测试 6：加权投票融合（专家联盟核心能力）
// ============================================================================

#[test]
fn test_weighted_voting_expert_alliance() {
    let fusion = WeightedVotingFusion;

    // 模拟 5 位专家对架构方案的投票
    let votes = vec![
        ("架构专家".to_string(), "方案A".to_string(), 0.9, 0.95),
        ("算法专家".to_string(), "方案A".to_string(), 0.8, 0.90),
        ("数据专家".to_string(), "方案B".to_string(), 0.7, 0.85),
        ("产品专家".to_string(), "方案B".to_string(), 0.6, 0.80),
        ("安全专家".to_string(), "方案A".to_string(), 0.75, 0.88),
    ];

    let result = fusion.vote_options(&votes);

    // 方案A 应该胜出
    assert_eq!(result[0].0, "方案A");

    // 方案A 得分：0.9*0.95 + 0.8*0.90 + 0.75*0.88 = 0.855 + 0.72 + 0.66 = 2.235
    // 方案B 得分：0.7*0.85 + 0.6*0.80 = 0.595 + 0.48 = 1.075
    assert!((result[0].1 - 2.235).abs() < 1e-6);
    assert!((result[1].1 - 1.075).abs() < 1e-6);
}

#[test]
fn test_continuous_value_fusion() {
    let fusion = WeightedVotingFusion;

    // 多专家对性能指标的评估融合
    let values = vec![
        ("专家A".to_string(), 85.0, 0.9, 0.95),  // 高性能专家
        ("专家B".to_string(), 78.0, 0.7, 0.90),  // 中等权重
        ("专家C".to_string(), 92.0, 0.5, 0.85),  // 低权重
    ];

    let result = fusion.fuse_continuous(&values);

    // 加权平均：(85*0.855 + 78*0.63 + 92*0.425) / (0.855 + 0.63 + 0.425)
    // = (72.675 + 49.14 + 39.1) / 1.91
    // = 160.915 / 1.91 ≈ 84.25
    assert!(result > 80.0);
    assert!(result < 90.0);
}

// ============================================================================
// 测试 7：完整端到端场景：专家匹配工作流
// ============================================================================

#[test]
fn test_expert_matching_workflow() {
    // 场景：为一个新任务匹配最合适的专家
    // 步骤：1. 计算技能相似度 2. 计算经验评分 3. 加权排名 4. Top-K 推荐

    // 1. 技能向量相似度（余弦）
    let cosine = CosineSimilarity::default();
    let task_skills = vec![0.9, 0.7, 0.5, 0.3]; // [Rust, 图算法, 云原生, 前端]

    let experts = vec![
        ("专家A".to_string(), vec![0.95, 0.8, 0.3, 0.1], 8, 0.9), // 技能, 经验年数, 可用度
        ("专家B".to_string(), vec![0.7, 0.9, 0.6, 0.2], 5, 0.8),
        ("专家C".to_string(), vec![0.5, 0.6, 0.9, 0.4], 10, 0.7),
    ];

    // 2. 构建多因子评分
    let ranker = WeightedRanker::new(vec![
        ("skill_sim".to_string(), 0.5),
        ("experience".to_string(), 0.3),
        ("availability".to_string(), 0.2),
    ]);

    let items: Vec<(String, HashMap<String, f64>)> = experts
        .iter()
        .map(|(name, skills, exp, avail)| {
            let sim = cosine.similarity(&task_skills, skills);
            let exp_norm = (*exp as f64) / 10.0; // 归一化到 0-1
            (
                name.clone(),
                HashMap::from([
                    ("skill_sim".to_string(), sim),
                    ("experience".to_string(), exp_norm),
                    ("availability".to_string(), *avail),
                ]),
            )
        })
        .collect();

    // 3. 排名
    let result = ranker.rank(&items);

    // 4. 验证：专家 A 的技能最匹配，应该排名靠前
    assert_eq!(result.total, 3);
    assert!(result.items[0].score > result.items[1].score);
    assert!(result.items[1].score > result.items[2].score);

    // 专家 A 的技能相似度最高
    let expert_a_score = result
        .items
        .iter()
        .find(|i| i.key == "专家A")
        .unwrap()
        .score;
    let expert_c_score = result
        .items
        .iter()
        .find(|i| i.key == "专家C")
        .unwrap()
        .score;

    // 专家 A 应该排在专家 C 前面（技能权重最高）
    assert!(expert_a_score > expert_c_score);
}
