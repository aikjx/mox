// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Mock 连接器实现
//!
//! 内存 Mock 连接器，用于单元测试（不依赖外部 kg-hub 服务）。
//! 完全保留原有 MockKgHubConnector 的 API，确保测试代码零修改迁移。

use std::collections::BTreeMap;

use super::traits::KgConnector;
use super::types::GraphSearchHit;

/// 内存 Mock 连接器，用于单元测试（不依赖外部 kg-hub 服务）
pub struct MockKgHubConnector {
    spread_result: BTreeMap<String, f64>,
    search_result: Vec<GraphSearchHit>,
    available: bool,
}

impl MockKgHubConnector {
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

    pub fn unavailable(mut self) -> Self {
        self.available = false;
        self
    }
}

impl Default for MockKgHubConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl KgConnector for MockKgHubConnector {
    fn spread(
        &self,
        _seeds: &[String],
        _damping: f64,
        _rounds: u32,
    ) -> Result<BTreeMap<String, f64>, String> {
        if !self.available {
            return Err("mock kg-hub unavailable".to_string());
        }
        Ok(self.spread_result.clone())
    }

    fn search(&self, _query: &str, top_k: usize) -> Result<Vec<GraphSearchHit>, String> {
        if !self.available {
            return Err("mock kg-hub unavailable".to_string());
        }
        Ok(self.search_result.iter().take(top_k).cloned().collect())
    }

    fn available(&self) -> bool {
        self.available
    }

    fn name(&self) -> &str {
        "mock-kg-hub"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock 连接器默认可用
    #[test]
    fn mock_default_available() {
        let m = MockKgHubConnector::new();
        assert!(m.available());
        assert_eq!(m.name(), "mock-kg-hub");
    }

    /// Mock 连接器 unavailable 后 spread 和 search 返回 Err
    #[test]
    fn mock_unavailable_returns_err() {
        let m = MockKgHubConnector::new().unavailable();
        assert!(!m.available());
        assert!(m.spread(&["test".to_string()], 0.8, 2).is_err());
        assert!(m.search("test", 10).is_err());
    }

    /// Mock 连接器 with_spread 设置扩散结果
    #[test]
    fn mock_with_spread_works() {
        let mut result = BTreeMap::new();
        result.insert("code".to_string(), 0.9);
        result.insert("rust".to_string(), 0.8);

        let m = MockKgHubConnector::new().with_spread(result.clone());
        let spread = m.spread(&["test".to_string()], 0.8, 2).unwrap();
        assert_eq!(spread, result);
    }

    /// Mock 连接器 with_search 设置搜索结果并按 top_k 截断
    #[test]
    fn mock_with_search_respects_top_k() {
        let hits = vec![
            GraphSearchHit {
                id: "1".into(), name: "a".into(), kind: "".into(),
                layer: "".into(), path: "".into(), summary: "".into(),
                score: 0.9, keyword_score: 0.0, vector_score: 0.0,
                graph_score: 0.0, matched_by: vec![],
            },
            GraphSearchHit {
                id: "2".into(), name: "b".into(), kind: "".into(),
                layer: "".into(), path: "".into(), summary: "".into(),
                score: 0.8, keyword_score: 0.0, vector_score: 0.0,
                graph_score: 0.0, matched_by: vec![],
            },
        ];

        let m = MockKgHubConnector::new().with_search(hits);
        let result = m.search("test", 1).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "1");
    }

    /// Mock 连接器实现 Default
    #[test]
    fn mock_default_trait() {
        let m: MockKgHubConnector = Default::default();
        assert!(m.available());
    }
}
