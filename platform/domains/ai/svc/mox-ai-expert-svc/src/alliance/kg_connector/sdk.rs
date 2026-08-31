// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! SDK 连接器实现
//!
//! 基于 `mox-kg-sdk` 的 `GraphClient` 实现 KgConnector trait。
//! 适用于：
//!   - 进程内集成场景（不需要独立部署 kg-hub 服务）
//!   - 单元测试（比 Mock 更接近真实图语义）
//!   - 开发环境快速启动
//!
//! 设计要点：
//!   - kg-sdk 的 GraphClient 方法为 async，此处用内嵌 tokio runtime 同步化
//!   - 节点 ID 类型转换：kg-sdk 用 i64，GraphSearchHit 用 String
//!   - 搜索算法：基于节点 label/typ/attrs 的简单文本匹配 + 图扩散

use std::collections::{BTreeMap, HashSet};

use mox_kg_sdk::GraphClient;

use super::traits::KgConnector;
use super::types::GraphSearchHit;

/// 基于 mox-kg-sdk 的图谱连接器
///
/// 在进程内直接使用 kg-sdk 的 GraphClient，无需独立 kg-hub 服务。
/// 适合测试、开发环境和轻量部署场景。
pub struct SdkKgConnector {
    client: GraphClient,
    runtime: tokio::runtime::Runtime,
    name: String,
}

impl SdkKgConnector {
    /// 创建 SDK 连接器
    pub fn new(client: GraphClient) -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for SdkKgConnector");
        Self {
            client,
            runtime,
            name: "sdk-kg".to_string(),
        }
    }

    /// 创建带自定义名称的 SDK 连接器
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// 获取 GraphClient 引用（用于外部填充测试数据）
    pub fn client(&self) -> &GraphClient {
        &self.client
    }

    /// 内部辅助：block_on 异步调用
    fn block_on<F: std::future::Future>(&self, f: F) -> F::Output {
        self.runtime.block_on(f)
    }

    /// 基于文本的简单节点搜索
    ///
    /// 在节点的 label、typ、attrs 值中查找匹配 query 的项，
    /// 按匹配程度打分，返回 top_k 条结果。
    fn text_search_nodes(&self, query: &str, top_k: usize) -> Result<Vec<ScoredNode>, String> {
        let nodes = self.block_on(self.client.list_nodes()).map_err(|e| e.to_string())?;
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<ScoredNode> = nodes
            .iter()
            .filter_map(|node| {
                // 构建节点文本：label + typ + 所有 attr 值
                let mut node_text = format!("{} {} ", node.label.to_lowercase(), node.typ.to_lowercase());
                for v in node.attrs.values() {
                    node_text.push_str(&v.to_lowercase());
                    node_text.push(' ');
                }

                // 计算匹配分数
                let mut score = 0.0_f64;
                let mut matched_terms = Vec::new();
                for term in &query_terms {
                    if node_text.contains(term) {
                        score += 1.0;
                        matched_terms.push(term.to_string());
                    }
                }

                // label 精确匹配加分
                if node.label.to_lowercase().contains(&query_lower) {
                    score += 2.0;
                }

                if score > 0.0 {
                    Some(ScoredNode {
                        id: node.id,
                        label: node.label.clone(),
                        typ: node.typ.clone(),
                        community: node.community,
                        score,
                        matched_by: matched_terms,
                    })
                } else {
                    None
                }
            })
            .collect();

        // 按分数降序排列
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        Ok(scored)
    }

    /// 图扩散：从种子节点出发，沿边扩散指定跳数
    ///
    /// 实现简化版 PageRank 风格扩散：
    ///   - 每跳分数按 damping 衰减
    ///   - 节点分数累加
    ///   - 返回所有可达节点及其分数
    fn graph_spread(
        &self,
        seed_ids: &[i64],
        damping: f64,
        rounds: u32,
    ) -> Result<BTreeMap<i64, f64>, String> {
        let edges = self.block_on(self.client.list_edges()).map_err(|e| e.to_string())?;

        // 构建邻接表（双向）
        let mut adj: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
        for edge in &edges {
            adj.entry(edge.src).or_default().push(edge.dst);
            adj.entry(edge.dst).or_default().push(edge.src);
        }

        // 初始化分数：种子节点初始分为 1.0
        let mut scores: BTreeMap<i64, f64> = BTreeMap::new();
        let mut current_front: HashSet<i64> = HashSet::new();
        for &id in seed_ids {
            scores.insert(id, 1.0);
            current_front.insert(id);
        }

        // 逐轮扩散
        let mut damping_factor = damping;
        for _ in 0..rounds {
            let mut next_front: HashSet<i64> = HashSet::new();
            for &node_id in &current_front {
                if let Some(neighbors) = adj.get(&node_id) {
                    let node_score = *scores.get(&node_id).unwrap_or(&0.0);
                    let spread_score = node_score * damping_factor;
                    for &neighbor in neighbors {
                        let entry = scores.entry(neighbor).or_insert(0.0);
                        *entry += spread_score;
                        next_front.insert(neighbor);
                    }
                }
            }
            current_front = next_front;
            damping_factor *= damping; // 每跳进一步衰减
        }

        Ok(scores)
    }
}

// 内部辅助结构：带分数的节点
struct ScoredNode {
    id: i64,
    label: String,
    typ: String,
    community: i64,
    score: f64,
    matched_by: Vec<String>,
}

impl KgConnector for SdkKgConnector {
    fn spread(
        &self,
        seeds: &[String],
        damping: f64,
        rounds: u32,
    ) -> Result<BTreeMap<String, f64>, String> {
        // Step 1: 用 seeds 作为查询词搜索种子节点
        let query = seeds.join(" ");
        let seed_nodes = self.text_search_nodes(&query, 20)?;

        if seed_nodes.is_empty() {
            return Ok(BTreeMap::new());
        }

        let seed_ids: Vec<i64> = seed_nodes.iter().map(|n| n.id).collect();

        // Step 2: 从种子节点出发进行图扩散
        let spread_scores = self.graph_spread(&seed_ids, damping, rounds.min(5))?;

        // Step 3: 转换为 {label: score} 映射
        // 同时用 id 和 label 作为 key，提高 intent 归一命中率
        let nodes = self.block_on(self.client.list_nodes()).map_err(|e| e.to_string())?;
        let node_map: BTreeMap<i64, &mox_kg_sdk::Node> =
            nodes.iter().map(|n| (n.id, n)).collect();

        let mut result: BTreeMap<String, f64> = BTreeMap::new();
        for (id, score) in &spread_scores {
            if *score > 0.0 {
                // 用 id（字符串形式）作为 key
                result.insert(id.to_string(), *score);
                // 用 label 作为 key
                if let Some(node) = node_map.get(id) {
                    result.insert(node.label.clone(), *score);
                }
            }
        }

        Ok(result)
    }

    fn search(&self, query: &str, top_k: usize) -> Result<Vec<GraphSearchHit>, String> {
        let scored = self.text_search_nodes(query, top_k)?;

        let hits: Vec<GraphSearchHit> = scored
            .into_iter()
            .map(|sn| {
                // 归一化分数（简单处理：最大值为 1.0 的比例，最多 1.0）
                let normalized_score = (sn.score / 5.0).min(1.0);
                GraphSearchHit {
                    id: sn.id.to_string(),
                    name: sn.label.clone(),
                    kind: sn.typ.clone(),
                    layer: format!("L{}", sn.community % 7 + 1), // 模拟 layer
                    path: format!("{}/{}", sn.typ, sn.label),
                    summary: format!("SDK 节点: type={}, community={}", sn.typ, sn.community),
                    score: normalized_score,
                    keyword_score: normalized_score,
                    vector_score: 0.0,
                    graph_score: 0.0,
                    matched_by: sn.matched_by,
                }
            })
            .collect();

        Ok(hits)
    }

    fn available(&self) -> bool {
        // 内存 SDK 连接器始终可用
        true
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_client() -> GraphClient {
        let client = GraphClient::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // 种子一些节点
            client.spark_seed_nodes(10).await.unwrap();
            client.spark_seed_edges(15).await.unwrap();
        });
        client
    }

    /// SdkKgConnector 构造成功
    #[test]
    fn sdk_connector_constructs() {
        let client = setup_client();
        let conn = SdkKgConnector::new(client);
        assert!(conn.available());
        assert_eq!(conn.name(), "sdk-kg");
    }

    /// SdkKgConnector 自定义名称
    #[test]
    fn sdk_connector_custom_name() {
        let client = setup_client();
        let conn = SdkKgConnector::new(client).with_name("my-kg");
        assert_eq!(conn.name(), "my-kg");
    }

    /// SdkKgConnector search 返回结果
    #[test]
    fn sdk_connector_search_returns_hits() {
        let client = setup_client();
        let conn = SdkKgConnector::new(client);

        // 搜索 User（seed_nodes 中会有 User 类型的节点）
        let hits = conn.search("User", 5).unwrap();
        // 应该能找到一些匹配（10 个节点中有 User 类型）
        assert!(!hits.is_empty(), "should find at least one User node");
        assert!(hits.len() <= 5, "should respect top_k");

        // 验证 GraphSearchHit 字段填充
        let hit = &hits[0];
        assert!(!hit.id.is_empty());
        assert!(!hit.name.is_empty());
        assert!(!hit.kind.is_empty());
        assert!(hit.score > 0.0 && hit.score <= 1.0);
    }

    /// SdkKgConnector search 无匹配时返回空
    #[test]
    fn sdk_connector_search_no_match() {
        let client = setup_client();
        let conn = SdkKgConnector::new(client);
        let hits = conn.search("nonexistent_xyz_123", 10).unwrap();
        assert!(hits.is_empty());
    }

    /// SdkKgConnector spread 工作
    #[test]
    fn sdk_connector_spread_works() {
        let client = setup_client();
        let conn = SdkKgConnector::new(client);

        // 搜索种子并扩散
        let result = conn
            .spread(&["User".to_string()], 0.8, 2)
            .unwrap();

        // 应该有一些扩散结果
        // （具体数量取决于图结构，但至少应该有种子节点本身）
        assert!(!result.is_empty(), "spread should return some results");
    }

    /// SdkKgConnector spread 空种子返回空
    #[test]
    fn sdk_connector_spread_empty_seeds() {
        let client = setup_client();
        let conn = SdkKgConnector::new(client);
        let result = conn
            .spread(&["nonexistent_term_xyz".to_string()], 0.8, 2)
            .unwrap();
        // 找不到种子节点时返回空
        assert!(result.is_empty());
    }

    /// SdkKgConnector 实现 KgConnector trait
    #[test]
    fn sdk_connector_implements_kg_connector() {
        fn check<C: KgConnector>(_: &C) {}
        let client = setup_client();
        let conn = SdkKgConnector::new(client);
        check(&conn);
    }
}
