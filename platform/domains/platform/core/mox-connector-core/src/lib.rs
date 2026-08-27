// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! # Mox Connector Core — 连接器框架
//!
//! 第三方系统即插即用：实现Connector trait + 注册到ConnectorRegistry。
//! 支持协议适配：REST / gRPC / WebSocket / SOAP / 文件。
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use mox_connector_core::prelude::*;
//! use std::sync::Arc;
//!
//! let registry = ConnectorRegistry::new();
//! registry.register(Arc::new(WebhookConnector::new(config)));
//!
//! // 调用连接器
//! let connector = registry.get("webhook").unwrap();
//! let result = connector.execute(&request).await?;
//! ```

pub mod connectors;
pub mod protocol;
pub mod registry;
pub mod traits;

// 重导出
pub use connectors::webhook::WebhookConnector;
pub use registry::ConnectorRegistry;
pub use traits::{Connector, ConnectorConfig, ConnectorRequest, ConnectorResponse, ConnectorType, ConnectorError, ConnectorResult};

/// 便捷预导入
pub mod prelude {
    pub use super::*;
}
