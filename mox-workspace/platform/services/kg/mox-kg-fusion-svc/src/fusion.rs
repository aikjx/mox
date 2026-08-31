//! 知识融合
//!
//! 将多个来源的知识合并为统一的知识图谱

use serde::{Deserialize, Serialize};
use crate::alignment::AlignmentResult;
use crate::error::FusionResult;

/// 冲突解决策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionStrategy {
    /// 信任最新数据
    LatestWins,
    /// 信任权威数据源
    TrustedSource,
    /// 投票机制
    Voting,
    /// 保守融合（取交集）
    Conservative,
    /// 宽松融合（取并集）
    Liberal,
    /// 人工审核
    ManualReview,
}

/// 融合配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionConfig {
    /// 冲突解决策略
    pub conflict_strategy: ConflictResolutionStrategy,
    /// 可信数据源优先级（ID 列表，按优先级排序）
    pub trusted_sources: Vec<String>,
    /// 是否保留原始来源信息
    pub keep_provenance: bool,
    /// 属性级融合规则
    pub attribute_rules: std::collections::HashMap<String, AttributeFusionRule>,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            conflict_strategy: ConflictResolutionStrategy::LatestWins,
            trusted_sources: vec![],
            keep_provenance: true,
            attribute_rules: std::collections::HashMap::new(),
        }
    }
}

/// 属性级融合规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeFusionRule {
    /// 属性名
    pub attribute_name: String,
    /// 冲突策略
    pub strategy: ConflictResolutionStrategy,
    /// 默认值
    pub default_value: Option<serde_json::Value>,
}

/// 融合结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionResultData {
    /// 融合后的图谱 ID
    pub fused_graph_id: String,
    /// 融合的实体数
    pub fused_entities: usize,
    /// 融合的关系数
    pub fused_relations: usize,
    /// 检测到的冲突数
    pub conflicts_detected: usize,
    /// 已解决的冲突数
    pub conflicts_resolved: usize,
    /// 待人工审核数
    pub pending_review: usize,
}

/// 知识融合服务接口
#[async_trait::async_trait]
pub trait KnowledgeFusion: Send + Sync {
    /// 融合服务名称
    fn name(&self) -> &str;

    /// 执行图谱融合
    async fn fuse_graphs(
        &self,
        source_graph_ids: &[String],
        target_graph_id: &str,
        alignments: &[AlignmentResult],
        config: &FusionConfig,
    ) -> FusionResult<FusionResultData>;

    /// 增量融合
    async fn fuse_incremental(
        &self,
        source_graph_id: &str,
        target_graph_id: &str,
        alignments: &[AlignmentResult],
        config: &FusionConfig,
    ) -> FusionResult<FusionResultData>;
}
