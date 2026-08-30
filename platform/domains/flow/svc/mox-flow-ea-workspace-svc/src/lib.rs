// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! # 专家联盟统一工作台聚合服务
//!
//! 跨域集成层 L3：知识图谱 + 知识库云盘 + 专家联盟 三域聚合
//!
//! ## 职责
//! - 统一数据视图：将 KG 节点、云盘文档、专家档案融合为统一资源模型
//! - 跨域编排：协调多服务完成复杂业务流程（如专家匹配 + 相关文档 + 关联节点）
//! - 算法调度：调用 mox-unified-algo-core 完成相似度、排序、图算法计算
//! - 聚合 API：为前端统一工作台提供一站式接口

pub mod error;
pub mod types;
pub mod aggregator;
pub mod matching;
pub mod search;

pub use error::{WorkspaceError, WorkspaceResult};
pub use types::*;
pub use aggregator::WorkspaceAggregator;
pub use matching::ExpertMatchingEngine;
pub use search::UnifiedSearchEngine;

/// 服务元信息
pub const CRATE_META: mox_platform_foundation::CrateMeta =
    mox_platform_foundation::CrateMeta {
        id: "mox-flow-ea-workspace-svc",
        name: "专家联盟工作台聚合服务",
        version: env!("CARGO_PKG_VERSION"),
        layer: mox_platform_foundation::AisLayer::L3Orchestration,
        owner: "expert-alliance",
    };
