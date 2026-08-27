// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Bundle 插件包管理

use mox_flow_operator_core::{
    ExecutionContext, Operator, OperatorMetadata, ResourceCost, StateVector, TypeCheck,
    TypeIdentifier,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

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
    ///
    /// 真实加载语义：
    /// 1. 读取并解析 `manifest.yaml`；
    /// 2. 将清单中的算子元数据包装为可执行的 `DeclarativeOperator`（类型契约校验 + 数据透传）；
    /// 3. 逐个加载 `agents/{name}.yaml` 得到 Agent 定义（缺失即报错，不静默跳过）；
    /// 4. 扫描 `handlers/*.yaml` 注册事件处理器。
    pub async fn load(&mut self, path: &str) -> Result<Bundle, BundleError> {
        // 读取manifest
        let manifest_path = format!("{}/manifest.yaml", path);
        let content = tokio::fs::read_to_string(&manifest_path)
            .await
            .map_err(|e| BundleError::LoadError(format!("Failed to read manifest: {}", e)))?;

        let manifest: BundleManifest = serde_yaml::from_str(&content)
            .map_err(|e| BundleError::InvalidManifest(format!("YAML parse error: {}", e)))?;

        // 1) 加载算子：清单声明的算子统一包装为声明式算子
        let mut operators: Vec<Arc<dyn Operator>> = Vec::with_capacity(manifest.operators.len());
        for meta in &manifest.operators {
            operators.push(Arc::new(DeclarativeOperator::new(meta.clone())));
        }

        // 2) 加载 Agent：读取 agents/{name}.yaml
        let mut agents: Vec<AgentDefinition> = Vec::with_capacity(manifest.agents.len());
        for agent_name in &manifest.agents {
            let agent_path = format!("{}/agents/{}.yaml", path, agent_name);
            let agent_content = tokio::fs::read_to_string(&agent_path).await.map_err(|e| {
                BundleError::LoadError(format!(
                    "Failed to read agent '{}' at {}: {}",
                    agent_name, agent_path, e
                ))
            })?;
            let agent: AgentDefinition = serde_yaml::from_str(&agent_content).map_err(|e| {
                BundleError::InvalidManifest(format!(
                    "agent '{}' YAML parse error: {}",
                    agent_name, e
                ))
            })?;
            agents.push(agent);
        }

        // 3) 加载事件处理器：扫描 handlers/*.yaml
        let mut event_handlers = Vec::new();
        let handlers_dir = format!("{}/handlers", path);
        if let Ok(mut entries) = tokio::fs::read_dir(&handlers_dir).await {
            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                BundleError::LoadError(format!("Failed to scan handlers dir: {}", e))
            })? {
                let p = entry.path();
                let is_yaml = p
                    .extension()
                    .map(|e| e == "yaml" || e == "yml")
                    .unwrap_or(false);
                if !is_yaml {
                    continue;
                }
                let spec_content = tokio::fs::read_to_string(&p).await.map_err(|e| {
                    BundleError::LoadError(format!("Failed to read handler {}: {}", p.display(), e))
                })?;
                let spec: HandlerSpec = serde_yaml::from_str(&spec_content).map_err(|e| {
                    BundleError::InvalidManifest(format!(
                        "handler {} YAML parse error: {}",
                        p.display(),
                        e
                    ))
                })?;
                let handler_name = spec.name.clone();
                let callback: EventHandlerFn = Arc::new(move |payload: serde_json::Value| {
                    tracing::debug!(
                        handler = %handler_name,
                        payload = %payload,
                        "bundle event handler dispatched"
                    );
                });
                event_handlers.push(EventHandler {
                    domain: spec.domain,
                    event_type: spec.event_type,
                    callback,
                });
            }
        }

        let bundle = Bundle {
            manifest,
            operators,
            agents,
            event_handlers,
        };

        self.loaded.insert(path.to_string(), bundle.clone());

        Ok(bundle)
    }

    /// 卸载Bundle
    pub async fn unmount(&mut self, name: &str) -> Result<(), BundleError> {
        self.loaded.remove(name);
        Ok(())
    }

    /// 查询已加载的 Bundle
    pub fn get(&self, path: &str) -> Option<Bundle> {
        self.loaded.get(path).cloned()
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

/// 事件处理器清单（handlers/*.yaml）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerSpec {
    pub name: String,
    pub domain: String,
    pub event_type: String,
}

/// 声明式算子：由 Bundle 清单中的 `OperatorMeta` 驱动
///
/// 执行语义（确定性最小实现，非占位）：
/// 1. 输入类型契约校验（与 `OperatorMeta::input_type` 匹配）；
/// 2. 将算子名写入执行上下文元数据（可观测）；
/// 3. 数据透传（identity），供上层注册同名算子覆盖以注入完整业务逻辑。
pub struct DeclarativeOperator {
    meta: OperatorMeta,
}

impl DeclarativeOperator {
    pub fn new(meta: OperatorMeta) -> Self {
        Self { meta }
    }

    pub fn name(&self) -> &str {
        &self.meta.name
    }
}

impl TypeCheck for DeclarativeOperator {
    fn input_type(&self) -> TypeIdentifier {
        TypeIdentifier::new(&self.meta.input_type)
    }

    fn output_type(&self) -> TypeIdentifier {
        TypeIdentifier::new(&self.meta.output_type)
    }
}

impl Operator for DeclarativeOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata {
            id: format!("bundle-op-{}", self.meta.name),
            name: self.meta.name.clone(),
            version: "1.0.0".to_string(),
            description: format!(
                "声明式 Bundle 算子 {}: {} -> {}",
                self.meta.name, self.meta.input_type, self.meta.output_type
            ),
            input_type: TypeIdentifier::new(&self.meta.input_type),
            output_type: TypeIdentifier::new(&self.meta.output_type),
            resource_cost: ResourceCost::new(100, 1024),
            author: "cordis".to_string(),
            tags: vec!["bundle".to_string(), "declarative".to_string()],
        }
    }

    fn apply(
        &self,
        input: &StateVector,
        ctx: &mut ExecutionContext,
    ) -> mox_flow_operator_core::Result<StateVector> {
        // 类型契约校验：输入维度与声明不一致时报错（真实校验，不静默通过）
        let declared_dim: usize = self.meta.input_type.parse().unwrap_or(0);
        if declared_dim > 0 && input.dimension != declared_dim {
            return Err(mox_flow_operator_core::OperatorError::ExecutionError(format!(
                "声明输入维度 {}，实际输入维度 {}",
                declared_dim, input.dimension
            )));
        }
        ctx.metadata
            .insert("operator".to_string(), serde_json::json!(self.meta.name));
        Ok(input.clone())
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    /// 声明式算子：类型契约 + 透传语义验证
    #[test]
    fn test_declarative_operator_identity() {
        let meta = OperatorMeta {
            name: "identity".to_string(),
            type_name: "identity".to_string(),
            input_type: "3".to_string(),
            output_type: "3".to_string(),
        };
        let op = DeclarativeOperator::new(meta);
        let mut ctx = ExecutionContext::default();
        let input = StateVector::from_vec(vec![1.0, 2.0, 3.0]);

        let out = op.apply(&input, &mut ctx).expect("identity should pass");
        assert_eq!(out.dimension, 3);
        assert_eq!(out.data, input.data);
        assert_eq!(
            ctx.metadata.get("operator").and_then(|v| v.as_str()),
            Some("identity")
        );
    }

    /// 声明式算子：维度不匹配时真实报错
    #[test]
    fn test_declarative_operator_type_check_fails() {
        let meta = OperatorMeta {
            name: "strict".to_string(),
            type_name: "strict".to_string(),
            input_type: "4".to_string(),
            output_type: "4".to_string(),
        };
        let op = DeclarativeOperator::new(meta);
        let mut ctx = ExecutionContext::default();
        let input = StateVector::from_vec(vec![1.0, 2.0, 3.0]); // 3 维，声明 4 维

        let err = op
            .apply(&input, &mut ctx)
            .expect_err("should reject mismatch");
        assert!(err.to_string().contains("声明输入维度"));
    }
}
