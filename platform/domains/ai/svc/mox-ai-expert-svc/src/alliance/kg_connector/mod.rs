// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! KG 连接器（归一化重构 · P1 代码去重）
//!
//! # 重构说明
//!
//! 原 `kg_connector.rs` 单文件重构为模块化结构：
//!
//! ```text
//! kg_connector/
//! ├── mod.rs          ← 本文件，对外统一导出
//! ├── types.rs        ← 共享类型（GraphSearchHit, ExpertGraphBoost, ApiResp）
//! ├── traits.rs       ← 统一 KgConnector trait（KgHubConnector 为别名）
//! ├── http.rs         ← HttpKgHubConnector（HTTP 方式，生产默认）
//! ├── sdk.rs          ← SdkKgConnector（基于 mox-kg-sdk，进程内）
//! ├── mock.rs         ← MockKgHubConnector（测试用）
//! └── adapter.rs      ← spread_fn / enhance_expert_matching 适配器
//! ```
//!
//! # 去重点
//!
//! 1. **类型去重**：`GraphSearchHit` 等类型集中定义在 `types.rs`，各实现复用
//! 2. **工具函数去重**：消除手写 `urlencode`，改用 `reqwest::RequestBuilder::query()` 自动编码
//! 3. **trait 统一**：`KgConnector` 为统一 trait，`KgHubConnector` 保留为 type alias 兼容旧代码
//! 4. **多实现**：HTTP / SDK / Mock 三种实现共存，可按需切换
//!
//! # 向后兼容
//!
//! - 所有原有类型和函数均通过本模块重新导出
//! - `KgHubConnector` trait 名保留（`pub use KgConnector as KgHubConnector`）
//! - `MockKgHubConnector`、`HttpKgHubConnector` 构造 API 不变
//! - `spread_fn`、`enhance_expert_matching` 签名不变

pub mod adapter;
pub mod http;
pub mod mock;
pub mod sdk;
pub mod traits;
pub mod types;

// ================== 统一对外导出 ==================

// 类型
pub use types::{ExpertGraphBoost, GraphSearchHit};

// Trait（含旧名兼容）
pub use traits::{KgConnector, KgHubConnector};

// 连接器实现
pub use http::HttpKgHubConnector;
pub use mock::MockKgHubConnector;
pub use sdk::SdkKgConnector;

// 适配器与增强函数
pub use adapter::{enhance_expert_matching, spread_fn};

// ================== 集成测试 ==================

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::alliance::intent::classify_intent;
    use std::collections::BTreeMap;

    /// 集成测试：所有连接器实现都能通过 spread_fn 接入 classify_intent
    #[test]
    fn all_connectors_work_with_spread_fn() {
        let mut spread_result = BTreeMap::new();
        spread_result.insert("code".to_string(), 0.95);

        // Mock 连接器
        let mock = MockKgHubConnector::new().with_spread(spread_result.clone());
        let r1 = classify_intent("test", Some(spread_fn(&mock)));
        assert!(!r1.degraded, "mock connector should work");

        // HTTP 连接器（不可用时降级）
        let http = HttpKgHubConnector::with_timeout("http://127.0.0.1:19999", 100);
        let r2 = classify_intent("test", Some(spread_fn(&http)));
        assert!(r2.degraded, "http connector to dead port should degrade");
    }

    /// 集成测试：所有连接器都可用于 enhance_expert_matching
    #[test]
    fn all_connectors_work_with_enhance_expert_matching() {
        let expert_ids = vec!["code".to_string()];
        let dimensions = BTreeMap::new();

        // Mock 连接器 - 空结果
        let mock = MockKgHubConnector::new();
        let b1 = enhance_expert_matching(&mock, "test", &expert_ids, &dimensions);
        assert!(!b1.graph_used);

        // Mock 连接器 - 有结果
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
        let mock2 = MockKgHubConnector::new().with_search(hits);
        let b2 = enhance_expert_matching(&mock2, "code", &expert_ids, &dimensions);
        assert!(b2.graph_used);
        assert!(b2.boosts.get("code").copied().unwrap_or(0.0) > 0.0);
    }

    /// 集成测试：SDK 连接器与 Mock 连接器行为一致（搜索方向）
    #[test]
    fn sdk_connector_search_semantics() {
        let client = mox_kg_sdk::GraphClient::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            client.spark_seed_nodes(5).await.unwrap();
        });

        let sdk = SdkKgConnector::new(client);
        let hits = sdk.search("User", 10).unwrap();

        // SDK 连接器搜索应返回结果（5 个节点中应该有 User 类型）
        assert!(!hits.is_empty());
        assert!(hits.iter().all(|h| h.score > 0.0));
    }

    /// 验证向后兼容：KgHubConnector trait 名仍然可用
    #[test]
    fn backward_compat_kg_hub_connector_trait() {
        fn old_style_fn<C: KgHubConnector>(c: &C) -> &str {
            c.name()
        }

        let mock = MockKgHubConnector::new();
        assert_eq!(old_style_fn(&mock), "mock-kg-hub");

        let http = HttpKgHubConnector::new("http://localhost:8080");
        assert_eq!(old_style_fn(&http), "http-kg-hub");
    }

    /// 验证向后兼容：所有导出项与原单文件一致
    #[test]
    fn backward_compat_all_exports_exist() {
        // 类型
        let _hit = GraphSearchHit {
            id: "".into(), name: "".into(), kind: "".into(),
            layer: "".into(), path: "".into(), summary: "".into(),
            score: 0.0, keyword_score: 0.0, vector_score: 0.0,
            graph_score: 0.0, matched_by: vec![],
        };
        let _boost = ExpertGraphBoost {
            boosts: BTreeMap::new(),
            graph_used: false,
        };

        // 连接器
        let _mock = MockKgHubConnector::new();
        let _http = HttpKgHubConnector::new("http://localhost:8080");

        // 函数
        let mock = MockKgHubConnector::new();
        let _f = spread_fn(&mock);
        let _b = enhance_expert_matching(&mock, "q", &[], &BTreeMap::new());
    }
}
