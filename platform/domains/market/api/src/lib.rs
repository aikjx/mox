// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! MOX Market Domain API — trait contracts for marketplace, plugins, extensions.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MarketApiError {
    #[error("plugin not found: {0}")]
    NotFound(String),
    #[error("plugin conflict: {0}")]
    Conflict(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("installation failed: {0}")]
    Installation(String),
    #[error("internal: {0}")]
    Internal(String),
}

pub type MarketApiResult<T> = Result<T, MarketApiError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginStatus { Available, Installed, Enabled, Disabled, Updating, Error }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginType { Source, Transform, Sink, Filter, Enrich, Auth, Storage, Analytics, Ui, Other }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub plugin_type: PluginType,
    pub status: PluginStatus,
    pub tags: Vec<String>,
    pub config_schema: serde_json::Value,
    pub installed_at: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstallation {
    pub plugin_id: String,
    pub tenant_id: String,
    pub config: serde_json::Value,
    pub installed_at: String,
    pub installed_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionPoint {
    pub id: String,
    pub name: String,
    pub description: String,
    pub domain: String,
    pub required_interface: String,
    pub registered_plugins: Vec<String>,
}

#[async_trait]
pub trait PluginRegistry: Send + Sync {
    async fn register(&self, plugin: PluginInfo) -> MarketApiResult<()>;
    async fn unregister(&self, plugin_id: &str) -> MarketApiResult<bool>;
    async fn get(&self, plugin_id: &str) -> MarketApiResult<Option<PluginInfo>>;
    async fn search(&self, query: &str, plugin_type: Option<PluginType>) -> MarketApiResult<Vec<PluginInfo>>;
    async fn list(&self, status: Option<PluginStatus>) -> MarketApiResult<Vec<PluginInfo>>;
    async fn update(&self, plugin_id: &str, new_version: &str) -> MarketApiResult<PluginInfo>;
}

#[async_trait]
pub trait PluginManager: Send + Sync {
    async fn install(&self, installation: PluginInstallation) -> MarketApiResult<PluginInfo>;
    async fn uninstall(&self, plugin_id: &str, tenant_id: &str) -> MarketApiResult<bool>;
    async fn enable(&self, plugin_id: &str, tenant_id: &str) -> MarketApiResult<()>;
    async fn disable(&self, plugin_id: &str, tenant_id: &str) -> MarketApiResult<()>;
    async fn configure(&self, plugin_id: &str, tenant_id: &str, config: serde_json::Value) -> MarketApiResult<()>;
    async fn list_installed(&self, tenant_id: &str) -> MarketApiResult<Vec<PluginInfo>>;
}

#[async_trait]
pub trait ExtensionPointRegistry: Send + Sync {
    async fn register_extension_point(&self, point: ExtensionPoint) -> MarketApiResult<()>;
    async fn get_extension_point(&self, id: &str) -> MarketApiResult<Option<ExtensionPoint>>;
    async fn list_extension_points(&self, domain: Option<&str>) -> MarketApiResult<Vec<ExtensionPoint>>;
    async fn attach_plugin(&self, extension_point_id: &str, plugin_id: &str) -> MarketApiResult<()>;
    async fn detach_plugin(&self, extension_point_id: &str, plugin_id: &str) -> MarketApiResult<bool>;
}

pub trait PluginExecutor: Send + Sync {
    fn execute(&self, plugin_id: &str, input: &serde_json::Value, context: &serde_json::Value) -> MarketApiResult<serde_json::Value>;
    fn validate(&self, plugin_id: &str, config: &serde_json::Value) -> MarketApiResult<()>;
}
