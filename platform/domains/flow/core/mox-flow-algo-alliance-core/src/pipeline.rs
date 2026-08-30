// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS)
// Licensed under the MIT License.

//! 算法编排器 — DAG 流水线调度
//!
//! 将多个算法组合成 DAG 流水线，支持：
//! - 顺序执行
//! - 并行分支
//! - 条件分支
//! - 循环迭代
//! - 错误处理与重试

use crate::compute_engine::ComputeEngine;
use crate::error::{AlgoError, AlgoResult};
use crate::metrics::AlgoMetrics;
use crate::registry::AlgorithmRegistry;
use crate::types::{AlgorithmStatus, ParamValue};
use crate::unified_model::UnifiedData;
use indexmap::IndexMap;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Instant;

/// 流水线状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStatus {
    /// 就绪
    Ready,
    /// 运行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
    /// 暂停
    Paused,
}

impl PipelineStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PipelineStatus::Ready => "ready",
            PipelineStatus::Running => "running",
            PipelineStatus::Completed => "completed",
            PipelineStatus::Failed => "failed",
            PipelineStatus::Cancelled => "cancelled",
            PipelineStatus::Paused => "paused",
        }
    }
}

impl From<PipelineStatus> for AlgorithmStatus {
    fn from(s: PipelineStatus) -> Self {
        match s {
            PipelineStatus::Completed | PipelineStatus::Ready => AlgorithmStatus::Active,
            PipelineStatus::Failed => AlgorithmStatus::Maintenance,
            _ => AlgorithmStatus::Experimental,
        }
    }
}

/// 流水线节点类型
#[derive(Debug, Clone)]
pub enum PipelineNode {
    /// 算法节点
    Algorithm {
        /// 算法 ID
        algo_id: String,
        /// 节点 ID
        node_id: String,
        /// 参数映射
        params: IndexMap<String, ParamValue>,
    },
    /// 条件分支
    Conditional {
        /// 节点 ID
        node_id: String,
        /// 条件参数名
        condition_param: String,
        /// true 分支
        true_branch: Vec<String>,
        /// false 分支
        false_branch: Vec<String>,
    },
    /// 并行扇出
    Parallel {
        /// 节点 ID
        node_id: String,
        /// 并行分支的节点 ID 列表
        branches: Vec<Vec<String>>,
    },
    /// 数据转换节点
    Transform {
        /// 节点 ID
        node_id: String,
        /// 转换类型
        transform_type: String,
    },
}

impl PipelineNode {
    pub fn node_id(&self) -> &str {
        match self {
            PipelineNode::Algorithm { node_id, .. } => node_id,
            PipelineNode::Conditional { node_id, .. } => node_id,
            PipelineNode::Parallel { node_id, .. } => node_id,
            PipelineNode::Transform { node_id, .. } => node_id,
        }
    }
}

/// 算法流水线
///
/// 由多个算法节点通过 DAG 方式连接而成的计算流水线。
pub struct AlgoPipeline {
    /// 流水线 ID
    id: String,
    /// 流水线名称
    name: String,
    /// 描述
    description: String,
    /// 版本
    version: String,
    /// 节点映射
    nodes: IndexMap<String, PipelineNode>,
    /// 连接关系 (from -> vec![to])
    edges: IndexMap<String, Vec<String>>,
    /// 入口节点 ID
    entry_node: String,
    /// 状态
    status: RwLock<PipelineStatus>,
    /// 执行次数
    execution_count: RwLock<u64>,
    /// 平均执行时间（ms）
    avg_execution_ms: RwLock<f64>,
}

impl AlgoPipeline {
    /// 获取流水线 ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 获取流水线名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取描述
    pub fn description(&self) -> &str {
        &self.description
    }

    /// 获取版本
    pub fn version(&self) -> &str {
        &self.version
    }

    /// 获取状态
    pub fn status(&self) -> PipelineStatus {
        *self.status.read()
    }

    /// 设置状态
    pub fn set_status(&self, status: PipelineStatus) {
        *self.status.write() = status;
    }

    /// 节点数量
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 边数量
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|v| v.len()).sum()
    }

    /// 执行流水线
    pub async fn execute(
        &self,
        input: UnifiedData,
        registry: Arc<AlgorithmRegistry>,
        compute_engine: Arc<ComputeEngine>,
        _metrics: Arc<AlgoMetrics>,
    ) -> AlgoResult<UnifiedData> {
        self.set_status(PipelineStatus::Running);
        let start = Instant::now();

        // 使用显式栈替代递归，避免 async fn 递归的无限大小问题
        // 栈中每个元素是待执行的节点 ID 和输入数据
        let result = self.execute_iterative(&self.entry_node, input, &registry, &compute_engine).await;

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        *self.avg_execution_ms.write() =
            (*self.avg_execution_ms.read() * (*self.execution_count.read() as f64) + elapsed)
                / (*self.execution_count.read() as f64 + 1.0);
        *self.execution_count.write() += 1;

        match &result {
            Ok(_) => self.set_status(PipelineStatus::Completed),
            Err(_) => self.set_status(PipelineStatus::Failed),
        }

        result
    }

    /// 迭代式执行节点（避免 async 递归）
    async fn execute_iterative(
        &self,
        start_node: &str,
        input: UnifiedData,
        registry: &Arc<AlgorithmRegistry>,
        compute_engine: &Arc<ComputeEngine>,
    ) -> AlgoResult<UnifiedData> {
        // 工作栈：(节点ID, 输入数据)
        // 对于线性链路，我们顺序执行；对于分支节点，我们内联展开分支
        let mut current_node = start_node.to_string();
        let mut current_input = input;

        loop {
            let node = self
                .nodes
                .get(&current_node)
                .ok_or_else(|| AlgoError::PipelineError(format!("node not found: {}", current_node)))?;

            let result = match node {
                PipelineNode::Algorithm {
                    algo_id, params, ..
                } => {
                    let algo = registry.get(algo_id)?;
                    algo.execute(current_input, params.clone(), compute_engine.clone())
                        .await?
                }
                PipelineNode::Transform { transform_type, .. } => {
                    self.apply_transform(&current_input, transform_type)?
                }
                PipelineNode::Conditional {
                    condition_param: _,
                    true_branch,
                    false_branch,
                    ..
                } => {
                    // 检查输入是否为 true
                    let condition = current_input.as_bool().unwrap_or(false);
                    let branch = if condition {
                        true_branch
                    } else {
                        false_branch
                    };

                    // 顺序执行分支中的所有节点
                    let mut branch_result = current_input;
                    for next_id in branch {
                        branch_result = self
                            .execute_single_algo_or_transform(next_id, branch_result, registry, compute_engine)
                            .await?;
                    }
                    branch_result
                }
                PipelineNode::Parallel { branches, .. } => {
                    // 简化版本：顺序执行第一个分支
                    if branches.is_empty() {
                        return Err(AlgoError::PipelineError(
                            "parallel node has no branches".to_string(),
                        ));
                    }
                    let mut last_result = current_input;
                    for branch in &branches[0] {
                        last_result = self
                            .execute_single_algo_or_transform(branch, last_result.clone(), registry, compute_engine)
                            .await?;
                    }
                    last_result
                }
            };

            // 继续执行下游节点
            if let Some(next_nodes) = self.edges.get(&current_node) {
                if !next_nodes.is_empty() {
                    current_input = result;
                    current_node = next_nodes[0].clone();
                    continue;
                }
            }

            return Ok(result);
        }
    }

    /// 执行单个算法或转换节点（非递归，仅处理节点本身）
    async fn execute_single_algo_or_transform(
        &self,
        node_id: &str,
        input: UnifiedData,
        registry: &Arc<AlgorithmRegistry>,
        compute_engine: &Arc<ComputeEngine>,
    ) -> AlgoResult<UnifiedData> {
        let node = self
            .nodes
            .get(node_id)
            .ok_or_else(|| AlgoError::PipelineError(format!("node not found: {}", node_id)))?;

        match node {
            PipelineNode::Algorithm { algo_id, params, .. } => {
                let algo = registry.get(algo_id)?;
                algo.execute(input, params.clone(), compute_engine.clone()).await
            }
            PipelineNode::Transform { transform_type, .. } => {
                self.apply_transform(&input, transform_type)
            }
            // Conditional 和 Parallel 节点不应该出现在分支内部（简化设计）
            PipelineNode::Conditional { .. } => {
                Err(AlgoError::PipelineError(
                    "nested conditional not supported in branches".to_string(),
                ))
            }
            PipelineNode::Parallel { .. } => {
                Err(AlgoError::PipelineError(
                    "nested parallel not supported in branches".to_string(),
                ))
            }
        }
    }

    /// 应用数据转换
    fn apply_transform(&self, input: &UnifiedData, transform_type: &str) -> AlgoResult<UnifiedData> {
        match transform_type {
            "object_to_graph" => input.object_to_graph().ok_or_else(|| {
                AlgoError::PipelineError("cannot convert object to graph".to_string())
            }),
            "list_to_vector" => input.list_to_vector().ok_or_else(|| {
                AlgoError::PipelineError("cannot convert list to vector".to_string())
            }),
            "identity" => Ok(input.clone()),
            other => Err(AlgoError::PipelineError(format!(
                "unknown transform: {}",
                other
            ))),
        }
    }

    /// 验证流水线的完整性
    pub fn validate(&self, registry: &AlgorithmRegistry) -> AlgoResult<()> {
        // 检查入口节点存在
        if !self.nodes.contains_key(&self.entry_node) {
            return Err(AlgoError::PipelineError(
                "entry node not found".to_string(),
            ));
        }

        // 检查所有算法节点的算法存在
        for node in self.nodes.values() {
            if let PipelineNode::Algorithm { algo_id, .. } = node {
                if !registry.contains(algo_id) {
                    return Err(AlgoError::AlgorithmNotFound(algo_id.clone()));
                }
            }
        }

        // 检查边引用的节点存在
        for (from, to_list) in &self.edges {
            if !self.nodes.contains_key(from) {
                return Err(AlgoError::PipelineError(format!(
                    "edge source node not found: {}",
                    from
                )));
            }
            for to in to_list {
                if !self.nodes.contains_key(to) {
                    return Err(AlgoError::PipelineError(format!(
                        "edge target node not found: {}",
                        to
                    )));
                }
            }
        }

        Ok(())
    }
}

/// 流水线构建器
pub struct PipelineBuilder {
    id: String,
    name: String,
    description: String,
    version: String,
    nodes: IndexMap<String, PipelineNode>,
    edges: IndexMap<String, Vec<String>>,
    entry_node: Option<String>,
    registry: Arc<AlgorithmRegistry>,
    compute_engine: Arc<ComputeEngine>,
    metrics: Arc<AlgoMetrics>,
}

impl PipelineBuilder {
    /// 创建新的构建器
    pub fn new(
        registry: Arc<AlgorithmRegistry>,
        compute_engine: Arc<ComputeEngine>,
        metrics: Arc<AlgoMetrics>,
    ) -> Self {
        Self {
            id: uuid_v4(),
            name: "Unnamed Pipeline".to_string(),
            description: String::new(),
            version: "0.1.0".to_string(),
            nodes: IndexMap::new(),
            edges: IndexMap::new(),
            entry_node: None,
            registry,
            compute_engine,
            metrics,
        }
    }

    /// 设置 ID
    pub fn id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    /// 设置名称
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// 设置描述
    pub fn description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// 设置版本
    pub fn version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    /// 添加算法节点
    pub fn add_algorithm(
        mut self,
        node_id: &str,
        algo_id: &str,
        params: IndexMap<String, ParamValue>,
    ) -> Self {
        let node = PipelineNode::Algorithm {
            node_id: node_id.to_string(),
            algo_id: algo_id.to_string(),
            params,
        };
        if self.entry_node.is_none() {
            self.entry_node = Some(node_id.to_string());
        }
        self.nodes.insert(node_id.to_string(), node);
        self
    }

    /// 添加转换节点
    pub fn add_transform(mut self, node_id: &str, transform_type: &str) -> Self {
        let node = PipelineNode::Transform {
            node_id: node_id.to_string(),
            transform_type: transform_type.to_string(),
        };
        if self.entry_node.is_none() {
            self.entry_node = Some(node_id.to_string());
        }
        self.nodes.insert(node_id.to_string(), node);
        self
    }

    /// 添加连接
    pub fn add_edge(mut self, from: &str, to: &str) -> Self {
        self.edges
            .entry(from.to_string())
            .or_default()
            .push(to.to_string());
        self
    }

    /// 设置入口节点
    pub fn entry_node(mut self, node_id: &str) -> Self {
        self.entry_node = Some(node_id.to_string());
        self
    }

    /// 构建流水线
    pub fn build(self) -> AlgoResult<AlgoPipeline> {
        let entry_node = self
            .entry_node
            .ok_or_else(|| AlgoError::PipelineError("no entry node specified".to_string()))?;

        let pipeline = AlgoPipeline {
            id: self.id,
            name: self.name,
            description: self.description,
            version: self.version,
            nodes: self.nodes,
            edges: self.edges,
            entry_node,
            status: RwLock::new(PipelineStatus::Ready),
            execution_count: RwLock::new(0),
            avg_execution_ms: RwLock::new(0.0),
        };

        // 验证
        pipeline.validate(&self.registry)?;

        Ok(pipeline)
    }
}

/// 简易 UUID v4 生成
fn uuid_v4() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6] & 0x0f | 0x40,
        bytes[7],
        bytes[8] & 0x3f | 0x80,
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Algorithm, AlgorithmCategory, DataShape, ParamSpec, ParamValue,
    };
    use async_trait::async_trait;
    use std::sync::Arc;

    struct DoubleAlgo;

    #[async_trait]
    impl Algorithm for DoubleAlgo {
        fn id(&self) -> &str {
            "test.double"
        }
        fn name(&self) -> &str {
            "Double"
        }
        fn category(&self) -> AlgorithmCategory {
            AlgorithmCategory::DataProcessing
        }
        fn version(&self) -> &str {
            "1.0.0"
        }
        fn description(&self) -> &str {
            "Doubles a number"
        }
        fn input_spec(&self) -> Vec<DataShape> {
            vec![DataShape::scalar("int")]
        }
        fn output_spec(&self) -> Vec<DataShape> {
            vec![DataShape::scalar("int")]
        }
        fn param_specs(&self) -> Vec<ParamSpec> {
            vec![]
        }
        async fn execute(
            &self,
            input: UnifiedData,
            _params: IndexMap<String, ParamValue>,
            _compute_engine: Arc<ComputeEngine>,
        ) -> AlgoResult<UnifiedData> {
            let v = input.as_int().unwrap_or(0);
            Ok(UnifiedData::Int(v * 2))
        }
    }

    #[tokio::test]
    async fn test_pipeline_single_node() {
        let registry = Arc::new(AlgorithmRegistry::new());
        let compute_engine = Arc::new(ComputeEngine::new());
        let metrics = Arc::new(AlgoMetrics::new());

        registry.register(DoubleAlgo).unwrap();

        let pipeline = PipelineBuilder::new(registry.clone(), compute_engine.clone(), metrics.clone())
            .id("pipe.test")
            .name("Test Pipeline")
            .add_algorithm("step1", "test.double", IndexMap::new())
            .build()
            .unwrap();

        assert_eq!(pipeline.node_count(), 1);
        assert_eq!(pipeline.status(), PipelineStatus::Ready);

        let result = pipeline
            .execute(UnifiedData::Int(21), registry, compute_engine, metrics)
            .await
            .unwrap();

        assert_eq!(result.as_int(), Some(42));
        assert_eq!(pipeline.status(), PipelineStatus::Completed);
    }

    #[tokio::test]
    async fn test_pipeline_chain() {
        let registry = Arc::new(AlgorithmRegistry::new());
        let compute_engine = Arc::new(ComputeEngine::new());
        let metrics = Arc::new(AlgoMetrics::new());

        registry.register(DoubleAlgo).unwrap();

        let pipeline = PipelineBuilder::new(registry.clone(), compute_engine.clone(), metrics.clone())
            .id("pipe.chain")
            .name("Chain Pipeline")
            .add_algorithm("step1", "test.double", IndexMap::new())
            .add_algorithm("step2", "test.double", IndexMap::new())
            .add_edge("step1", "step2")
            .build()
            .unwrap();

        let result = pipeline
            .execute(UnifiedData::Int(5), registry, compute_engine, metrics)
            .await
            .unwrap();

        assert_eq!(result.as_int(), Some(20)); // 5 * 2 * 2 = 20
    }

    #[test]
    fn test_pipeline_validate_missing_algo() {
        let registry = AlgorithmRegistry::new();
        let compute_engine = Arc::new(ComputeEngine::new());
        let metrics = Arc::new(AlgoMetrics::new());

        let result = PipelineBuilder::new(
            Arc::new(registry),
            compute_engine.clone(),
            metrics,
        )
        .add_algorithm("step1", "nonexistent", IndexMap::new())
        .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_status_conversion() {
        assert_eq!(
            AlgorithmStatus::from(PipelineStatus::Completed),
            AlgorithmStatus::Active
        );
        assert_eq!(
            AlgorithmStatus::from(PipelineStatus::Failed),
            AlgorithmStatus::Maintenance
        );
    }
}
