// Copyright (c) 2026 璇玑 RelGraph · 统一架构核心 (Unified Architecture Core)
// Licensed under the MIT License.

//! 统一架构核心
//!
//! 六大归一化体系之「架构归一化」：
//! - 统一接入协议（REST/GraphQL/gRPC/WebSocket 统一入口）
//! - 统一数据模型（KG/Cloud/Algorithm/Entity 统一抽象）
//! - 第三方对接标准（连接器框架 + 适配器模式）
//! - 统一编排调度

pub mod error;
pub mod types;
pub mod protocol;
pub mod connector;
pub mod adapter;
pub mod unified_api;
pub mod integration;

pub use error::{ArchError, ArchResult};
pub use types::{
    ApiRequest, ApiResponse, ApiStatus, ConnectorCategory, ConnectorInfo, ProtocolType,
    UnifiedResource,
};
pub use connector::{Connector, ConnectorRegistry};
pub use unified_api::UnifiedApiGateway;
pub use integration::IntegrationManager;
