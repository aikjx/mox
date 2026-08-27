//! 企业级内容多平台发布服务
//!
//! 基于Connector Framework，支持一键发布内容到多个第三方系统。
//!
//! ## 功能
//! - 多平台内容发布（CMS/电商/社交/自定义）
//! - 并发发布，减少总耗时
//! - 发布结果汇总与统计
//! - 全链路追踪
//! - 发布历史记录
//!
//! ## 快速开始
//!
//! ```rust
//! use mox_content_publisher::prelude::*;
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // 1. 获取Connector Registry（从IntegrationRuntime）
//! // let connector_registry = runtime.connector_registry();
//!
//! // 2. 创建发布器
//! // let publisher = ContentPublisher::new(connector_registry);
//!
//! // 3. 构建内容
//! let content = Content::new("标题", "正文", "category")
//!     .with_author("author")
//!     .with_tags(vec!["tag1".into(), "tag2".into()]);
//!
//! // 4. 发布到所有平台
//! // let results = publisher.publish_to_all(&content).await;
//!
//! // 5. 发布到指定平台
//! // let results = publisher.publish_to_platforms(&content, &["cms-grpc"]).await;
//! # Ok(())
//! # }
//! ```

pub mod api;
pub mod model;
pub mod publisher;

/// 预导入模块
pub mod prelude {
    pub use crate::api::{PublishContentRequest, PublishContentResponse, PublishResultDto, PublishSummaryDto};
    pub use crate::model::{Content, PublishPlatform, PublishResult, PublishStatus, PublishSummary, PlatformType};
    pub use crate::publisher::ContentPublisher;
}
