// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! KG 连接器抽象（trait 注入模式）
//!
//! 联盟引擎不直接依赖 kg-hub 或 kg-sdk，而是通过 [`KgConnector`] trait
//! 注入图谱能力。这样联盟引擎是纯领域逻辑，不耦合具体的图谱实现。
//!
//! # 设计原则
//! - 联盟引擎只依赖 trait，不依赖具体实现
//! - 上游服务（expert-svc / orchestrator-svc）负责注入具体连接器
//! - 图谱不可用时自动降级（degraded 模式），不阻断主流程
//!
//! # 主要能力
//! - `spread` — 激活扩散（用于意图分类的 HC-2 spread 方法）
//! - `search` — 图谱搜索（用于专家匹配增强）
//! - `name` — 连接器名称（用于日志/审计）

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 图谱搜索命中项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSearchHit {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub layer: String,
    pub path: String,
    pub summary: String,
    pub score: f64,
    pub keyword_score: f64,
    pub vector_score: f64,
    pub graph_score: f64,
    pub matched_by: Vec<String>,
}

/// 专家图谱增强结果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExpertGraphBoost {
    pub boosts: BTreeMap<String, f64>,
    pub graph_used: bool,
}

/// KG 连接器统一 trait
///
/// 联盟引擎通过此 trait 访问图谱能力，不依赖具体实现。
/// 上游服务负责注入 HTTP / SDK / Mock 等具体连接器。
#[async_trait]
pub trait KgConnector: Send + Sync + std::fmt::Debug {
    /// 连接器名称（用于日志/审计/可观测性）
    fn name(&self) -> &str;

    /// 激活扩散（HC-2 spread 方法）
    ///
    /// - `seeds`: 种子节点列表
    /// - `damping`: 阻尼因子（HC-2 固定 0.85）
    /// - `rounds`: 迭代轮数（HC-2 固定 30）
    ///
    /// 返回 `{intent_label: score}` 映射。
    /// 若图谱不可用或调用失败，返回 `Err`，调用方应自动降级。
    async fn spread(
        &self,
        seeds: &[String],
        damping: f64,
        rounds: u32,
    ) -> Result<BTreeMap<String, f64>, String>;

    /// 图谱搜索（用于专家匹配增强）
    ///
    /// - `query`: 查询文本
    /// - `limit`: 返回数量上限
    ///
    /// 返回命中项列表。
    fn search(&self, query: &str, limit: usize) -> Result<Vec<GraphSearchHit>, String>;

    /// 是否可用（健康检查）
    fn is_available(&self) -> bool {
        true
    }
}

/// spread_fn 适配器：将 `KgConnector` 适配为 `classify_intent` 需要的函数签名
///
/// 使用方式：
/// ```rust,ignore
/// let connector = HttpKgHubConnector::new("http://kg-hub:8080");
/// let intent = classify_intent(query, Some(spread_fn(&connector)));
/// ```
pub fn spread_fn<'a, C: KgConnector + ?Sized>(
    connector: &'a C,
) -> impl FnOnce(&[String], f64, u32) -> Result<BTreeMap<String, f64>, String> + 'a {
    move |seeds: &[String], damping: f64, rounds: u32| {
        // 同步适配：使用 tokio::runtime::Handle 阻塞当前线程
        // 注：实际使用时应优先使用异步版本，此适配器为兼容同步调用而存在
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async move { connector.spread(seeds, damping, rounds).await })
    }
}

/// 专家匹配增强：基于图谱搜索结果为专家打分加权
pub fn enhance_expert_matching<C: KgConnector + ?Sized>(
    connector: &C,
    query: &str,
    expert_ids: &[String],
    _dimensions: &BTreeMap<String, String>,
) -> ExpertGraphBoost {
    let mut result = ExpertGraphBoost::default();

    match connector.search(query, 20) {
        Ok(hits) if !hits.is_empty() => {
            result.graph_used = true;
            for hit in &hits {
                // 简单启发式：命中项的 path 或 kind 与专家 id 匹配则加分
                for eid in expert_ids {
                    let hit_text = format!("{} {} {} {}", hit.name, hit.kind, hit.path, hit.summary);
                    if hit_text.to_lowercase().contains(&eid.to_lowercase())
                        || eid.to_lowercase().contains(&hit.kind.to_lowercase())
                    {
                        let boost = result.boosts.entry(eid.clone()).or_insert(0.0);
                        *boost = (*boost + hit.score * 0.1).min(1.0);
                    }
                }
            }
        }
        _ => {
            // 图谱不可用或无结果，不加分
            result.graph_used = false;
        }
    }

    result
}

/// Mock KG 连接器（测试用）
///
/// 可预设 spread 结果和 search 结果，用于单元测试和集成测试。
#[derive(Debug)]
pub struct MockKgConnector {
    spread_result: BTreeMap<String, f64>,
    search_result: Vec<GraphSearchHit>,
    available: bool,
}

impl MockKgConnector {
    pub fn new() -> Self {
        Self {
            spread_result: BTreeMap::new(),
            search_result: Vec::new(),
            available: true,
        }
    }

    pub fn with_spread(mut self, result: BTreeMap<String, f64>) -> Self {
        self.spread_result = result;
        self
    }

    pub fn with_search(mut self, hits: Vec<GraphSearchHit>) -> Self {
        self.search_result = hits;
        self
    }

    pub fn with_available(mut self, available: bool) -> Self {
        self.available = available;
        self
    }
}

impl Default for MockKgConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl KgConnector for MockKgConnector {
    fn name(&self) -> &str {
        "mock-kg-connector"
    }

    async fn spread(
        &self,
        _seeds: &[String],
        _damping: f64,
        _rounds: u32,
    ) -> Result<BTreeMap<String, f64>, String> {
        if !self.available {
            return Err("mock connector unavailable".into());
        }
        Ok(self.spread_result.clone())
    }

    fn search(&self, _query: &str, limit: usize) -> Result<Vec<GraphSearchHit>, String> {
        if !self.available {
            return Err("mock connector unavailable".into());
        }
        Ok(self.search_result.iter().take(limit).cloned().collect())
    }

    fn is_available(&self) -> bool {
        self.available
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_connector_spread_works() {
        let mut expected = BTreeMap::new();
        expected.insert("code".to_string(), 0.95);
        expected.insert("logic".to_string(), 0.70);

        let mock = MockKgConnector::new().with_spread(expected.clone());
        assert_eq!(mock.name(), "mock-kg-connector");
        assert!(mock.is_available());

        let result = mock.spread(&["test".into()], 0.85, 30).await.unwrap();
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn mock_connector_unavailable_returns_err() {
        let mock = MockKgConnector::new().with_available(false);
        assert!(!mock.is_available());
        let result = mock.spread(&["test".into()], 0.85, 30).await;
        assert!(result.is_err());
    }

    #[test]
    fn enhance_expert_matching_with_mock() {
        let expert_ids = vec!["code".to_string(), "security".to_string()];
        let dims = BTreeMap::new();

        // 无结果
        let mock = MockKgConnector::new();
        let r1 = enhance_expert_matching(&mock, "test", &expert_ids, &dims);
        assert!(!r1.graph_used);
        assert!(r1.boosts.is_empty());

        // 有结果
        let hits = vec![GraphSearchHit {
            id: "1".into(),
            name: "code review".into(),
            kind: "Function".into(),
            layer: "L3".into(),
            path: "code/review".into(),
            summary: "代码审查功能".into(),
            score: 0.9,
            keyword_score: 0.8,
            vector_score: 0.0,
            graph_score: 0.0,
            matched_by: vec!["keyword".into()],
        }];
        let mock2 = MockKgConnector::new().with_search(hits);
        let r2 = enhance_expert_matching(&mock2, "code", &expert_ids, &dims);
        assert!(r2.graph_used);
    }

    #[test]
    fn graph_search_hit_serialization() {
        let hit = GraphSearchHit {
            id: "1".into(),
            name: "test".into(),
            kind: "Function".into(),
            layer: "L1".into(),
            path: "a/b".into(),
            summary: "test summary".into(),
            score: 0.9,
            keyword_score: 0.8,
            vector_score: 0.1,
            graph_score: 0.0,
            matched_by: vec!["keyword".into()],
        };
        let json = serde_json::to_string(&hit).unwrap();
        let back: GraphSearchHit = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "1");
        assert_eq!(back.score, 0.9);
    }
}
