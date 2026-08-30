// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! 统一搜索引擎
//!
//! 跨域搜索：同时搜索 KG 节点、Cloud 文档、Expert 专家，
//! 使用 mox-unified-algo-core 进行结果融合与重排序。

use std::time::Instant;

use crate::error::WorkspaceResult;
use crate::types::*;
use mox_unified_algo_core::algorithms::ranking::BordaFusion;

/// 统一搜索引擎
pub struct UnifiedSearchEngine {
    borda: BordaFusion,
}

impl UnifiedSearchEngine {
    /// 创建搜索引擎
    pub fn new() -> Self {
        Self {
            borda: BordaFusion,
        }
    }

    /// 执行统一搜索
    ///
    /// 1. 并行搜索各域
    /// 2. 结果归一化到统一资源模型
    /// 3. Borda 融合排序
    /// 4. 返回分页结果
    pub async fn search(
        &self,
        request: &UnifiedSearchRequest,
    ) -> WorkspaceResult<UnifiedSearchResponse> {
        let start = Instant::now();

        // 并行搜索各域
        let (kg_results, doc_results, expert_results) = tokio::join!(
            self.search_kg(request),
            self.search_cloud(request),
            self.search_experts(request)
        );

        let kg_results = kg_results?;
        let doc_results = doc_results?;
        let expert_results = expert_results?;

        // 收集所有结果
        let mut all_results: Vec<WorkspaceResource> = Vec::new();

        // 过滤资源类型
        let want_kg = request.resource_types.contains(&ResourceType::GraphNode)
            || request.resource_types.is_empty();
        let want_doc = request.resource_types.contains(&ResourceType::Document)
            || request.resource_types.is_empty();
        let want_expert = request.resource_types.contains(&ResourceType::Expert)
            || request.resource_types.is_empty();

        if want_kg {
            all_results.extend(kg_results);
        }
        if want_doc {
            all_results.extend(doc_results);
        }
        if want_expert {
            all_results.extend(expert_results);
        }

        let total = all_results.len();

        // Borda 融合排序（如果有多种资源类型）
        if all_results.len() > 1 {
            all_results = self.fuse_rank(all_results);
        }

        // 分页
        let results: Vec<WorkspaceResource> = all_results
            .into_iter()
            .skip(request.offset)
            .take(request.limit)
            .collect();

        let took_ms = start.elapsed().as_millis() as u64;

        // 分面聚合
        let aggregations = self.build_aggregations(&results);

        Ok(UnifiedSearchResponse {
            results,
            total,
            took_ms,
            aggregations,
        })
    }

    /// 搜索 KG 域（模拟实现）
    async fn search_kg(
        &self,
        request: &UnifiedSearchRequest,
    ) -> WorkspaceResult<Vec<WorkspaceResource>> {
        let _ = request;
        // 模拟 KG 搜索结果
        let nodes = vec![
            ("kg-001", "知识图谱", "核心概念节点", vec!["KG", "核心"]),
            ("kg-002", "实体对齐", "算法节点", vec!["算法", "NLP"]),
            ("kg-003", "图嵌入", "技术节点", vec!["GNN", "嵌入"]),
        ];

        Ok(nodes
            .into_iter()
            .map(|(id, title, desc, tags)| WorkspaceResource {
                id: id.to_string(),
                resource_type: ResourceType::GraphNode,
                title: title.to_string(),
                description: desc.to_string(),
                tags: tags.into_iter().map(String::from).collect(),
                domain_id: id.to_string(),
                source_domain: "kg".to_string(),
                relevance_score: 0.85,
                ..Default::default()
            })
            .collect())
    }

    /// 搜索 Cloud 域（模拟实现）
    async fn search_cloud(
        &self,
        request: &UnifiedSearchRequest,
    ) -> WorkspaceResult<Vec<WorkspaceResource>> {
        let _ = request;
        let docs = vec![
            ("doc-001", "架构设计规范.pdf", "详细的架构设计文档", vec!["架构", "规范"]),
            ("doc-002", "知识图谱入门.md", "KG 基础知识", vec!["KG", "教程"]),
            ("doc-003", "API 接口文档.docx", "REST API 详细说明", vec!["API", "文档"]),
            ("doc-004", "部署运维手册.pdf", "生产环境部署指南", vec!["运维", "部署"]),
        ];

        Ok(docs
            .into_iter()
            .map(|(id, title, desc, tags)| WorkspaceResource {
                id: id.to_string(),
                resource_type: ResourceType::Document,
                title: title.to_string(),
                description: desc.to_string(),
                tags: tags.into_iter().map(String::from).collect(),
                domain_id: id.to_string(),
                source_domain: "cloud".to_string(),
                relevance_score: 0.72,
                ..Default::default()
            })
            .collect())
    }

    /// 搜索 Expert 域（模拟实现）
    async fn search_experts(
        &self,
        request: &UnifiedSearchRequest,
    ) -> WorkspaceResult<Vec<WorkspaceResource>> {
        let _ = request;
        let experts = vec![
            ("exp-001", "张博士", "知识图谱算法专家", vec!["KG", "算法", "RAG"]),
            ("exp-002", "王教授", "图神经网络研究者", vec!["GNN", "深度学习"]),
        ];

        Ok(experts
            .into_iter()
            .map(|(id, title, desc, tags)| WorkspaceResource {
                id: id.to_string(),
                resource_type: ResourceType::Expert,
                title: title.to_string(),
                description: desc.to_string(),
                tags: tags.into_iter().map(String::from).collect(),
                domain_id: id.to_string(),
                source_domain: "expert".to_string(),
                relevance_score: 0.78,
                ..Default::default()
            })
            .collect())
    }

    /// Borda 融合排序
    fn fuse_rank(&self, results: Vec<WorkspaceResource>) -> Vec<WorkspaceResource> {
        // 使用 BordaFusion 对多源结果进行融合排序
        let ids: Vec<String> = results.iter().map(|r| r.id.clone()).collect();
        let rankings = vec![ids];
        let fused = self.borda.fuse(&rankings);

        let mut map: std::collections::HashMap<String, WorkspaceResource> =
            results.into_iter().map(|r| (r.id.clone(), r)).collect();

        let mut fused_results: Vec<WorkspaceResource> = fused
            .items
            .into_iter()
            .filter_map(|item| {
                let mut r = map.remove(&item.key)?;
                r.relevance_score = item.score;
                Some(r)
            })
            .collect();

        // 剩余未排序的追加到后面
        fused_results.extend(map.into_values());

        fused_results
    }

    /// 构建分面聚合
    fn build_aggregations(&self, results: &[WorkspaceResource]) -> serde_json::Value {
        use std::collections::HashMap;

        let mut type_counts: HashMap<&str, usize> = HashMap::new();
        let mut tag_counts: HashMap<&str, usize> = HashMap::new();

        for r in results {
            *type_counts.entry(r.resource_type.as_str()).or_insert(0) += 1;
            for tag in &r.tags {
                *tag_counts.entry(tag.as_str()).or_insert(0) += 1;
            }
        }

        serde_json::json!({
            "by_type": type_counts,
            "by_tag": tag_counts,
        })
    }
}

impl Default for UnifiedSearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_unified_search_all_types() {
        let engine = UnifiedSearchEngine::new();
        let request = UnifiedSearchRequest {
            query: "知识图谱".to_string(),
            resource_types: vec![],
            filters: serde_json::Value::Null,
            limit: 20,
            offset: 0,
            query_vector: None,
        };

        let response = engine.search(&request).await.unwrap();
        assert!(response.total > 0);
        assert!(response.took_ms < 1000);
        assert!(!response.results.is_empty());
    }

    #[tokio::test]
    async fn test_unified_search_filter_type() {
        let engine = UnifiedSearchEngine::new();
        let request = UnifiedSearchRequest {
            query: "test".to_string(),
            resource_types: vec![ResourceType::Document],
            filters: serde_json::Value::Null,
            limit: 10,
            offset: 0,
            query_vector: None,
        };

        let response = engine.search(&request).await.unwrap();
        assert!(response.results.iter().all(|r| matches!(r.resource_type, ResourceType::Document)));
    }

    #[tokio::test]
    async fn test_search_pagination() {
        let engine = UnifiedSearchEngine::new();
        let request = UnifiedSearchRequest {
            query: "test".to_string(),
            resource_types: vec![],
            filters: serde_json::Value::Null,
            limit: 2,
            offset: 0,
            query_vector: None,
        };

        let response = engine.search(&request).await.unwrap();
        assert!(response.results.len() <= 2);
    }

    #[tokio::test]
    async fn test_search_aggregations() {
        let engine = UnifiedSearchEngine::new();
        let request = UnifiedSearchRequest {
            query: "test".to_string(),
            resource_types: vec![],
            filters: serde_json::Value::Null,
            limit: 20,
            offset: 0,
            query_vector: None,
        };

        let response = engine.search(&request).await.unwrap();
        let agg = response.aggregations;
        assert!(agg.get("by_type").is_some());
        assert!(agg.get("by_tag").is_some());
    }
}
