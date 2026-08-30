// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! 统一数据类型 · 跨域资源模型
//!
//! 将 KG 节点、Cloud 文档、Expert 专家统一抽象为 WorkspaceResource。

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 资源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    /// 知识图谱节点
    GraphNode,
    /// 知识库文档
    Document,
    /// 专家
    Expert,
    /// 项目
    Project,
    /// 任务
    Task,
}

impl ResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceType::GraphNode => "graph_node",
            ResourceType::Document => "document",
            ResourceType::Expert => "expert",
            ResourceType::Project => "project",
            ResourceType::Task => "task",
        }
    }
}

/// 统一资源对象（跨域聚合后的结果）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceResource {
    /// 资源唯一 ID
    pub id: String,
    /// 资源类型
    pub resource_type: ResourceType,
    /// 标题/名称
    pub title: String,
    /// 描述/摘要
    pub description: String,
    /// 标签
    pub tags: Vec<String>,
    /// 关联的领域原始 ID
    pub domain_id: String,
    /// 来源域
    pub source_domain: String,
    /// 匹配度/相关度评分（0-1）
    pub relevance_score: f64,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 额外属性（各域自定义）
    pub metadata: serde_json::Value,
}

/// 专家信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertProfile {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub title: String,
    pub organization: String,
    pub skills: Vec<String>,
    pub domains: Vec<String>,
    pub rating: f64,
    pub completed_tasks: u32,
    pub availability: ExpertAvailability,
    pub hourly_rate: Option<f64>,
    pub description: String,
    pub skill_vector: Vec<f64>,
}

/// 专家可用性
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpertAvailability {
    Available,
    Busy,
    Offline,
    OnLeave,
}

/// 专家匹配结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertMatchResult {
    pub expert: ExpertProfile,
    pub match_score: f64,
    pub skill_overlap: Vec<String>,
    pub match_reasons: Vec<String>,
}

/// 文档信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub mime_type: String,
    pub version: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author: String,
    pub content_vector: Option<Vec<f64>>,
}

/// 图谱节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNodeInfo {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub properties: serde_json::Value,
    pub degree: u32,
    pub embedding: Option<Vec<f64>>,
}

/// 统一搜索请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSearchRequest {
    pub query: String,
    pub resource_types: Vec<ResourceType>,
    pub filters: serde_json::Value,
    pub limit: usize,
    pub offset: usize,
    pub query_vector: Option<Vec<f64>>,
}

/// 统一搜索响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSearchResponse {
    pub results: Vec<WorkspaceResource>,
    pub total: usize,
    pub took_ms: u64,
    pub aggregations: serde_json::Value,
}

/// 工作台概览数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceOverview {
    pub total_experts: usize,
    pub total_documents: usize,
    pub total_graph_nodes: usize,
    pub active_tasks: usize,
    pub recent_activities: Vec<ActivityItem>,
    pub recommended_experts: Vec<ExpertMatchResult>,
    pub trending_topics: Vec<TopicItem>,
}

/// 活动项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityItem {
    pub id: String,
    pub activity_type: String,
    pub title: String,
    pub description: String,
    pub actor: String,
    pub timestamp: DateTime<Utc>,
}

/// 话题项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicItem {
    pub id: String,
    pub name: String,
    pub weight: f64,
    pub related_count: usize,
}

impl Default for WorkspaceResource {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            resource_type: ResourceType::Document,
            title: String::new(),
            description: String::new(),
            tags: vec![],
            domain_id: String::new(),
            source_domain: String::new(),
            relevance_score: 0.0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: serde_json::Value::Null,
        }
    }
}
