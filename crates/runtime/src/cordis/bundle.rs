//! Bundle 插件包管理

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use operator_core::Operator;

/// Bundle管理器
pub struct BundleManager {
    /// 已加载Bundle
    loaded: HashMap<String, Bundle>,
}

impl BundleManager {
    pub fn new() -> Self {
        Self {
            loaded: HashMap::new(),
        }
    }

    /// 加载Bundle
    pub async fn load(&mut self, path: &str) -> Result<Bundle, BundleError> {
        // 读取manifest
        let manifest_path = format!("{}/manifest.yaml", path);
        let content = tokio::fs::read_to_string(&manifest_path).await
            .map_err(|e| BundleError::LoadError(format!("Failed to read manifest: {}", e)))?;

        let manifest: BundleManifest = serde_yaml::from_str(&content)
            .map_err(|e| BundleError::InvalidManifest(format!("YAML parse error: {}", e)))?;

        // TODO: 加载算子、Agent、事件处理器

        let bundle = Bundle {
            manifest,
            operators: Vec::new(),
            agents: Vec::new(),
            event_handlers: Vec::new(),
        };

        self.loaded.insert(path.to_string(), bundle.clone());

        Ok(bundle)
    }

    /// 卸载Bundle
    pub async fn unmount(&mut self, name: &str) -> Result<(), BundleError> {
        self.loaded.remove(name);
        Ok(())
    }
}

impl Default for BundleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Bundle插件包
#[derive(Clone)]
pub struct Bundle {
    pub manifest: BundleManifest,
    pub operators: Vec<Arc<dyn Operator>>,
    pub agents: Vec<AgentDefinition>,
    pub event_handlers: Vec<EventHandler>,
}

/// Bundle清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub operators: Vec<OperatorMeta>,
    pub agents: Vec<String>,
    pub dependencies: Option<Vec<Dependency>>,
}

/// 插件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}

/// 算子元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorMeta {
    pub name: String,
    pub type_name: String,
    pub input_type: String,
    pub output_type: String,
}

/// 依赖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
}

/// Agent定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub tools: Vec<String>,
}

/// 事件处理器
#[derive(Clone)]
pub struct EventHandler {
    pub domain: String,
    pub event_type: String,
    pub callback: EventHandlerFn,
}

/// 事件处理函数
pub type EventHandlerFn = Arc<dyn Fn(serde_json::Value) + Send + Sync>;

/// Bundle错误
#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("Bundle not found: {0}")]
    NotFound(String),

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("Load error: {0}")]
    LoadError(String),
}
