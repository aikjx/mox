// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS)
// Licensed under the MIT License.

//! 算法适配器 — 接入现有算法到算法联盟
//!
//! 提供适配器模式，将 KG 图算法、Cloud 纠删码等现有算法
//! 封装为统一的 `Algorithm` trait，注册到算法联盟中。

use crate::error::{AlgoError, AlgoResult};
use crate::types::{
    Algorithm, AlgorithmCategory, ComputeModel, DataShape, ParamSpec, ParamValue,
};
use crate::unified_model::UnifiedData;
use async_trait::async_trait;
use indexmap::IndexMap;
use std::sync::Arc;

use crate::compute_engine::ComputeEngine;

/// 算法适配器
///
/// 将任意算法函数包装为符合 `Algorithm` trait 的对象。
/// 支持函数式和闭包式两种适配器。
pub struct AlgorithmAdapter<F>
where
    F: Fn(UnifiedData, IndexMap<String, ParamValue>) -> AlgoResult<UnifiedData> + Send + Sync,
{
    id: String,
    name: String,
    category: AlgorithmCategory,
    version: String,
    description: String,
    input_spec: Vec<DataShape>,
    output_spec: Vec<DataShape>,
    param_specs: Vec<ParamSpec>,
    func: F,
}

impl<F> AlgorithmAdapter<F>
where
    F: Fn(UnifiedData, IndexMap<String, ParamValue>) -> AlgoResult<UnifiedData> + Send + Sync,
{
    /// 创建新的适配器
    pub fn new(
        id: &str,
        name: &str,
        category: AlgorithmCategory,
        func: F,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            category,
            version: "1.0.0".to_string(),
            description: String::new(),
            input_spec: vec![],
            output_spec: vec![],
            param_specs: vec![],
            func,
        }
    }

    /// 设置版本
    pub fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    /// 设置描述
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// 设置输入规格
    pub fn with_input_spec(mut self, spec: Vec<DataShape>) -> Self {
        self.input_spec = spec;
        self
    }

    /// 设置输出规格
    pub fn with_output_spec(mut self, spec: Vec<DataShape>) -> Self {
        self.output_spec = spec;
        self
    }

    /// 设置参数规格
    pub fn with_param_specs(mut self, specs: Vec<ParamSpec>) -> Self {
        self.param_specs = specs;
        self
    }
}

#[async_trait]
impl<F> Algorithm for AlgorithmAdapter<F>
where
    F: Fn(UnifiedData, IndexMap<String, ParamValue>) -> AlgoResult<UnifiedData> + Send + Sync,
{
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn category(&self) -> AlgorithmCategory {
        self.category
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn input_spec(&self) -> Vec<DataShape> {
        self.input_spec.clone()
    }

    fn output_spec(&self) -> Vec<DataShape> {
        self.output_spec.clone()
    }

    fn param_specs(&self) -> Vec<ParamSpec> {
        self.param_specs.clone()
    }

    fn supported_compute_models(&self) -> Vec<ComputeModel> {
        vec![ComputeModel::SingleThread]
    }

    async fn execute(
        &self,
        input: UnifiedData,
        params: IndexMap<String, ParamValue>,
        _compute_engine: Arc<ComputeEngine>,
    ) -> AlgoResult<UnifiedData> {
        (self.func)(input, params)
    }
}

/// KG 图算法适配器工厂
pub struct GraphAlgoAdapter;

impl GraphAlgoAdapter {
    /// 创建 PageRank 算法适配器
    pub fn pagerank() -> impl Algorithm {
        AlgorithmAdapter::new(
            "graph.pagerank",
            "PageRank",
            AlgorithmCategory::Graph,
            |input, params| {
                let damping = params
                    .get("damping")
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.85);

                let nodes = input.graph_node_count().unwrap_or(0);
                if nodes == 0 {
                    return Err(AlgoError::InputTypeMismatch {
                        expected: "graph".to_string(),
                        got: input.value_type().as_str().to_string(),
                    });
                }

                // 简化版：返回每个节点的初始 PageRank 值
                // 实际实现需要迭代计算
                let initial_pr = 1.0 / nodes as f64;
                let mut result = std::collections::HashMap::new();

                if let UnifiedData::Graph { nodes, .. } = &input {
                    for node in nodes {
                        result.insert(node.clone(), UnifiedData::Float(initial_pr));
                    }
                }

                // 把阻尼因子也作为元数据返回
                result.insert("_damping".to_string(), UnifiedData::Float(damping));

                Ok(UnifiedData::Object(result))
            },
        )
        .with_description("PageRank centrality algorithm for graphs")
        .with_input_spec(vec![DataShape::graph()])
        .with_output_spec(vec![DataShape::scalar("object")])
        .with_param_specs(vec![ParamSpec::new(
            "damping",
            "float",
            false,
            "PageRank damping factor",
        )
        .with_default(0.85)
        .with_range(0.0, 1.0)])
    }

    /// 创建 Louvain 社区发现算法适配器
    pub fn louvain() -> impl Algorithm {
        AlgorithmAdapter::new(
            "graph.louvain",
            "Louvain Community Detection",
            AlgorithmCategory::Graph,
            |input, params| {
                let resolution = params
                    .get("resolution")
                    .and_then(|v| v.as_float())
                    .unwrap_or(1.0);

                let nodes = input.graph_node_count().unwrap_or(0);
                if nodes == 0 {
                    return Err(AlgoError::InputTypeMismatch {
                        expected: "graph".to_string(),
                        got: input.value_type().as_str().to_string(),
                    });
                }

                // 简化版：每个节点一个社区
                // 实际实现需要模块化优化
                let mut communities = std::collections::HashMap::new();

                if let UnifiedData::Graph { nodes, .. } = &input {
                    for (i, node) in nodes.iter().enumerate() {
                        communities.insert(
                            node.clone(),
                            UnifiedData::Int((i as f64 * resolution) as i64),
                        );
                    }
                }

                Ok(UnifiedData::Object(communities))
            },
        )
        .with_description("Louvain community detection algorithm")
        .with_input_spec(vec![DataShape::graph()])
        .with_output_spec(vec![DataShape::scalar("object")])
        .with_param_specs(vec![ParamSpec::new(
            "resolution",
            "float",
            false,
            "Community resolution parameter",
        )
        .with_default(1.0)
        .with_range(0.1, 10.0)])
    }

    /// 创建 BFS 最短路径算法适配器
    pub fn bfs_shortest_path() -> impl Algorithm {
        AlgorithmAdapter::new(
            "graph.bfs_shortest_path",
            "BFS Shortest Path",
            AlgorithmCategory::Graph,
            |input, params| {
                let _source = params
                    .get("source")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .ok_or_else(|| AlgoError::MissingParameter("source".to_string()))?;

                if input.graph_node_count().unwrap_or(0) == 0 {
                    return Err(AlgoError::InputTypeMismatch {
                        expected: "graph".to_string(),
                        got: input.value_type().as_str().to_string(),
                    });
                }

                // 简化版：返回空结果
                // 实际实现需要 BFS 遍历
                let mut distances = std::collections::HashMap::new();

                if let UnifiedData::Graph { nodes, .. } = &input {
                    for node in nodes {
                        distances.insert(node.clone(), UnifiedData::Int(-1));
                    }
                }

                Ok(UnifiedData::Object(distances))
            },
        )
        .with_description("BFS-based shortest path algorithm")
        .with_input_spec(vec![DataShape::graph()])
        .with_output_spec(vec![DataShape::scalar("object")])
        .with_param_specs(vec![ParamSpec::new(
            "source",
            "string",
            true,
            "Source node ID",
        )])
    }
}

/// 编码算法适配器工厂
pub struct EncodingAlgoAdapter;

impl EncodingAlgoAdapter {
    /// 创建 Reed-Solomon 编码适配器
    pub fn reed_solomon_encode() -> impl Algorithm {
        AlgorithmAdapter::new(
            "encoding.reed_solomon_encode",
            "Reed-Solomon Encode",
            AlgorithmCategory::Encoding,
            |input, params| {
                let data_shards = params
                    .get("data_shards")
                    .and_then(|v| v.as_int())
                    .unwrap_or(4) as usize;
                let parity_shards = params
                    .get("parity_shards")
                    .and_then(|v| v.as_int())
                    .unwrap_or(2) as usize;

                let data = input.as_bytes().ok_or_else(|| AlgoError::InputTypeMismatch {
                    expected: "bytes".to_string(),
                    got: input.value_type().as_str().to_string(),
                })?;

                // 简化版：返回原始数据 + 模拟的校验分片
                // 实际实现需要真正的 RS 编码
                let total_shards = data_shards + parity_shards;
                let shard_size = (data.len() + data_shards - 1) / data_shards;

                let mut shards = std::collections::HashMap::new();
                for i in 0..data_shards {
                    let start = i * shard_size;
                    let end = ((i + 1) * shard_size).min(data.len());
                    shards.insert(
                        format!("data_{}", i),
                        UnifiedData::Bytes(data[start..end].to_vec()),
                    );
                }
                for i in 0..parity_shards {
                    shards.insert(
                        format!("parity_{}", i),
                        UnifiedData::Bytes(vec![0u8; shard_size]),
                    );
                }

                shards.insert(
                    "total_shards".to_string(),
                    UnifiedData::Int(total_shards as i64),
                );
                shards.insert("shard_size".to_string(), UnifiedData::Int(shard_size as i64));

                Ok(UnifiedData::Object(shards))
            },
        )
        .with_description("Reed-Solomon erasure coding encoder")
        .with_input_spec(vec![DataShape::scalar("bytes")])
        .with_output_spec(vec![DataShape::scalar("object")])
        .with_param_specs(vec![
            ParamSpec::new("data_shards", "int", false, "Number of data shards")
                .with_default(4)
                .with_range(1.0, 255.0),
            ParamSpec::new("parity_shards", "int", false, "Number of parity shards")
                .with_default(2)
                .with_range(1.0, 255.0),
        ])
    }
}

/// 统计分析适配器工厂
pub struct StatsAlgoAdapter;

impl StatsAlgoAdapter {
    /// 计算向量统计指标
    pub fn vector_stats() -> impl Algorithm {
        AlgorithmAdapter::new(
            "stats.vector_stats",
            "Vector Statistics",
            AlgorithmCategory::Statistics,
            |input, _params| {
                let vec_data = input.as_vector().ok_or_else(|| AlgoError::InputTypeMismatch {
                    expected: "vector".to_string(),
                    got: input.value_type().as_str().to_string(),
                })?;

                if vec_data.is_empty() {
                    return Err(AlgoError::InvalidParameter {
                        param: "input".to_string(),
                        reason: "vector is empty".to_string(),
                    });
                }

                let n = vec_data.len() as f64;
                let sum: f64 = vec_data.iter().sum();
                let mean = sum / n;

                let mut sorted = vec_data.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                let median = if vec_data.len() % 2 == 0 {
                    let mid = vec_data.len() / 2;
                    (sorted[mid - 1] + sorted[mid]) / 2.0
                } else {
                    sorted[vec_data.len() / 2]
                };

                let min = sorted[0];
                let max = sorted[vec_data.len() - 1];

                let variance: f64 = vec_data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
                let std_dev = variance.sqrt();

                let mut result = std::collections::HashMap::new();
                result.insert("count".to_string(), UnifiedData::Int(vec_data.len() as i64));
                result.insert("sum".to_string(), UnifiedData::Float(sum));
                result.insert("mean".to_string(), UnifiedData::Float(mean));
                result.insert("median".to_string(), UnifiedData::Float(median));
                result.insert("min".to_string(), UnifiedData::Float(min));
                result.insert("max".to_string(), UnifiedData::Float(max));
                result.insert("variance".to_string(), UnifiedData::Float(variance));
                result.insert("std_dev".to_string(), UnifiedData::Float(std_dev));
                result.insert("range".to_string(), UnifiedData::Float(max - min));

                Ok(UnifiedData::Object(result))
            },
        )
        .with_description("Compute basic statistics for a numeric vector")
        .with_input_spec(vec![DataShape::vector("f64", 0)])
        .with_output_spec(vec![DataShape::scalar("object")])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_new() {
        let adapter = AlgorithmAdapter::new(
            "test.adapter",
            "Test Adapter",
            AlgorithmCategory::Other,
            |input, _params| Ok(input),
        );

        assert_eq!(adapter.id(), "test.adapter");
        assert_eq!(adapter.name(), "Test Adapter");
        assert_eq!(adapter.category(), AlgorithmCategory::Other);
        assert_eq!(adapter.version(), "1.0.0");
    }

    #[tokio::test]
    async fn test_adapter_execute() {
        let adapter = AlgorithmAdapter::new(
            "test.double",
            "Double",
            AlgorithmCategory::DataProcessing,
            |input, _params| {
                let v = input.as_int().unwrap_or(0);
                Ok(UnifiedData::Int(v * 2))
            },
        );

        let engine = Arc::new(ComputeEngine::new());
        let result = adapter
            .execute(UnifiedData::Int(21), IndexMap::new(), engine)
            .await
            .unwrap();

        assert_eq!(result.as_int(), Some(42));
    }

    #[test]
    fn test_graph_pagerank_adapter() {
        let algo = GraphAlgoAdapter::pagerank();
        assert_eq!(algo.id(), "graph.pagerank");
        assert_eq!(algo.category(), AlgorithmCategory::Graph);
        assert!(!algo.param_specs().is_empty());
    }

    #[test]
    fn test_stats_vector_stats() {
        let algo = StatsAlgoAdapter::vector_stats();
        assert_eq!(algo.id(), "stats.vector_stats");
        assert_eq!(algo.category(), AlgorithmCategory::Statistics);
    }

    #[test]
    fn test_reed_solomon_adapter() {
        let algo = EncodingAlgoAdapter::reed_solomon_encode();
        assert_eq!(algo.id(), "encoding.reed_solomon_encode");
        assert_eq!(algo.category(), AlgorithmCategory::Encoding);
    }

    #[tokio::test]
    async fn test_vector_stats_execute() {
        let algo = StatsAlgoAdapter::vector_stats();
        let engine = Arc::new(ComputeEngine::new());
        let data = UnifiedData::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);

        let result = algo.execute(data, IndexMap::new(), engine).await.unwrap();

        if let UnifiedData::Object(map) = result {
            assert_eq!(map.get("count").unwrap().as_int(), Some(5));
            assert_eq!(map.get("mean").unwrap().as_float(), Some(3.0));
            assert_eq!(map.get("min").unwrap().as_float(), Some(1.0));
            assert_eq!(map.get("max").unwrap().as_float(), Some(5.0));
            assert_eq!(map.get("sum").unwrap().as_float(), Some(15.0));
        } else {
            panic!("expected object result");
        }
    }
}
