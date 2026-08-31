//! 实体对齐
//!
//! 发现不同图谱中指向同一现实对象的实体

use serde::{Deserialize, Serialize};
use mox_kg_meta_core::NodeId;
use crate::error::FusionResult;

/// 对齐策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentStrategy {
    /// 基于属性相似度
    AttributeSimilarity,
    /// 基于结构相似度
    StructuralSimilarity,
    /// 基于嵌入相似度
    EmbeddingSimilarity,
    /// 混合策略
    Hybrid,
}

/// 对齐配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentConfig {
    /// 对齐策略
    pub strategy: AlignmentStrategy,
    /// 相似度阈值（0-1）
    pub similarity_threshold: f64,
    /// 参与对齐的属性列表
    pub attributes: Vec<String>,
    /// 最大候选数
    pub max_candidates: usize,
}

impl Default for AlignmentConfig {
    fn default() -> Self {
        Self {
            strategy: AlignmentStrategy::Hybrid,
            similarity_threshold: 0.8,
            attributes: vec![],
            max_candidates: 10,
        }
    }
}

/// 实体对齐结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentResult {
    /// 源实体 ID
    pub source_id: NodeId,
    /// 目标实体 ID
    pub target_id: NodeId,
    /// 相似度得分（0-1）
    pub similarity: f64,
    /// 匹配的属性详情
    pub attribute_scores: std::collections::HashMap<String, f64>,
}

/// 实体对齐服务接口
#[async_trait::async_trait]
pub trait EntityAligner: Send + Sync {
    /// 对齐名称
    fn name(&self) -> &str;

    /// 执行实体对齐
    async fn align(
        &self,
        source_graph_id: &str,
        target_graph_id: &str,
        config: &AlignmentConfig,
    ) -> FusionResult<Vec<AlignmentResult>>;

    /// 查找单个实体的匹配候选
    async fn find_matches(
        &self,
        entity_id: &NodeId,
        source_graph_id: &str,
        target_graph_id: &str,
        config: &AlignmentConfig,
    ) -> FusionResult<Vec<AlignmentResult>>;
}
