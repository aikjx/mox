// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! 工作台数据聚合器
//!
//! 负责从 KG / Cloud / Expert 三个域拉取数据并聚合成统一视图。
//! 实际生产中通过 gRPC 调用各域服务，此处为架构验证实现。

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::WorkspaceResult;
use crate::types::*;

/// 域数据源接口（生产环境由 gRPC client 实现）
#[async_trait::async_trait]
pub trait DomainDataSource: Send + Sync {
    /// 域名称
    fn domain_name(&self) -> &str;

    /// 健康检查
    async fn health_check(&self) -> bool;
}

/// KG 域数据源
#[async_trait::async_trait]
pub trait KgDataSource: DomainDataSource {
    /// 搜索图谱节点
    async fn search_nodes(&self, query: &str, limit: usize) -> WorkspaceResult<Vec<GraphNodeInfo>>;

    /// 获取节点详情
    async fn get_node(&self, id: &str) -> WorkspaceResult<Option<GraphNodeInfo>>;

    /// 获取相关节点
    async fn get_related_nodes(&self, id: &str, depth: u32) -> WorkspaceResult<Vec<GraphNodeInfo>>;
}

/// Cloud 域数据源
#[async_trait::async_trait]
pub trait CloudDataSource: DomainDataSource {
    /// 搜索文档
    async fn search_documents(&self, query: &str, limit: usize) -> WorkspaceResult<Vec<DocumentInfo>>;

    /// 获取文档详情
    async fn get_document(&self, id: &str) -> WorkspaceResult<Option<DocumentInfo>>;

    /// 相关文档推荐
    async fn get_related_docs(&self, doc_id: &str, limit: usize) -> WorkspaceResult<Vec<DocumentInfo>>;
}

/// Expert 域数据源
#[async_trait::async_trait]
pub trait ExpertDataSource: DomainDataSource {
    /// 搜索专家
    async fn search_experts(&self, query: &str, limit: usize) -> WorkspaceResult<Vec<ExpertProfile>>;

    /// 获取专家详情
    async fn get_expert(&self, id: &str) -> WorkspaceResult<Option<ExpertProfile>>;

    /// 匹配专家
    async fn match_experts(&self, skills: &[String], limit: usize) -> WorkspaceResult<Vec<ExpertProfile>>;
}

/// 工作台聚合器
pub struct WorkspaceAggregator {
    kg_source: Option<Arc<dyn KgDataSource>>,
    cloud_source: Option<Arc<dyn CloudDataSource>>,
    expert_source: Option<Arc<dyn ExpertDataSource>>,
    /// 本地缓存（减轻跨域调用压力）
    resource_cache: RwLock<HashMap<String, WorkspaceResource>>,
}

impl WorkspaceAggregator {
    /// 创建聚合器
    pub fn new() -> Self {
        Self {
            kg_source: None,
            cloud_source: None,
            expert_source: None,
            resource_cache: RwLock::new(HashMap::new()),
        }
    }

    /// 注册 KG 数据源
    pub fn with_kg_source(mut self, source: Arc<dyn KgDataSource>) -> Self {
        self.kg_source = Some(source);
        self
    }

    /// 注册 Cloud 数据源
    pub fn with_cloud_source(mut self, source: Arc<dyn CloudDataSource>) -> Self {
        self.cloud_source = Some(source);
        self
    }

    /// 注册 Expert 数据源
    pub fn with_expert_source(mut self, source: Arc<dyn ExpertDataSource>) -> Self {
        self.expert_source = Some(source);
        self
    }

    /// 获取工作台概览
    pub async fn get_overview(&self) -> WorkspaceResult<WorkspaceOverview> {
        // 并行获取各域统计数据
        let (experts, docs, nodes) = tokio::join!(
            self.get_expert_count(),
            self.get_document_count(),
            self.get_graph_node_count()
        );

        let recent_activities = self.get_recent_activities().await?;
        let recommended_experts = self.get_recommended_experts(5).await?;
        let trending_topics = self.get_trending_topics(10).await?;

        Ok(WorkspaceOverview {
            total_experts: experts,
            total_documents: docs,
            total_graph_nodes: nodes,
            active_tasks: 0, // 待接入任务域
            recent_activities,
            recommended_experts,
            trending_topics,
        })
    }

    /// 获取专家数量
    async fn get_expert_count(&self) -> usize {
        // 实际环境调用 expert_source，此处返回模拟值
        128
    }

    /// 获取文档数量
    async fn get_document_count(&self) -> usize {
        2048
    }

    /// 获取图谱节点数量
    async fn get_graph_node_count(&self) -> usize {
        15632
    }

    /// 获取最近活动
    async fn get_recent_activities(&self) -> WorkspaceResult<Vec<ActivityItem>> {
        use chrono::Duration;

        let now = chrono::Utc::now();
        Ok(vec![
            ActivityItem {
                id: "act-001".to_string(),
                activity_type: "expert_join".to_string(),
                title: "新专家加入".to_string(),
                description: "张博士加入知识图谱算法专家组".to_string(),
                actor: "系统".to_string(),
                timestamp: now - Duration::minutes(5),
            },
            ActivityItem {
                id: "act-002".to_string(),
                activity_type: "document_upload".to_string(),
                title: "文档上传".to_string(),
                description: "上传《架构设计规范 v3.0》".to_string(),
                actor: "李工".to_string(),
                timestamp: now - Duration::minutes(15),
            },
            ActivityItem {
                id: "act-003".to_string(),
                activity_type: "graph_update".to_string(),
                title: "图谱更新".to_string(),
                description: "新增 128 个实体节点和 256 条关系".to_string(),
                actor: "KG 服务".to_string(),
                timestamp: now - Duration::hours(1),
            },
        ])
    }

    /// 获取推荐专家
    async fn get_recommended_experts(&self, limit: usize) -> WorkspaceResult<Vec<ExpertMatchResult>> {
        let _ = limit;
        Ok(vec![])
    }

    /// 获取热门话题
    async fn get_trending_topics(&self, limit: usize) -> WorkspaceResult<Vec<TopicItem>> {
        let _ = limit;
        Ok(vec![
            TopicItem { id: "t1".into(), name: "知识图谱".into(), weight: 0.95, related_count: 256 },
            TopicItem { id: "t2".into(), name: "RAG 检索增强".into(), weight: 0.88, related_count: 189 },
            TopicItem { id: "t3".into(), name: "向量数据库".into(), weight: 0.82, related_count: 145 },
            TopicItem { id: "t4".into(), name: "专家匹配".into(), weight: 0.76, related_count: 98 },
            TopicItem { id: "t5".into(), name: "图神经网络".into(), weight: 0.71, related_count: 87 },
        ])
    }

    /// 缓存资源
    pub fn cache_resource(&self, resource: WorkspaceResource) {
        self.resource_cache
            .write()
            .insert(resource.id.clone(), resource);
    }

    /// 从缓存获取资源
    pub fn get_cached_resource(&self, id: &str) -> Option<WorkspaceResource> {
        self.resource_cache.read().get(id).cloned()
    }

    /// 缓存大小
    pub fn cache_size(&self) -> usize {
        self.resource_cache.read().len()
    }
}

impl Default for WorkspaceAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_aggregator_creation() {
        let agg = WorkspaceAggregator::new();
        assert_eq!(agg.cache_size(), 0);
    }

    #[tokio::test]
    async fn test_overview_structure() {
        let agg = WorkspaceAggregator::new();
        let overview = agg.get_overview().await.unwrap();
        assert!(overview.total_experts > 0);
        assert!(overview.total_documents > 0);
        assert!(overview.total_graph_nodes > 0);
        assert!(!overview.recent_activities.is_empty());
        assert!(!overview.trending_topics.is_empty());
    }

    #[tokio::test]
    async fn test_resource_cache() {
        let agg = WorkspaceAggregator::new();
        let resource = WorkspaceResource {
            id: "test-001".to_string(),
            title: "测试资源".to_string(),
            ..Default::default()
        };
        agg.cache_resource(resource);
        assert_eq!(agg.cache_size(), 1);

        let cached = agg.get_cached_resource("test-001");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().title, "测试资源");
    }
}
