// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! # 算子注册表
//!
//! 实现 OP-NORM-02：算子元数据归一化注册表。
//! 所有算子统一注册、版本管理、血缘追踪、能力查询。

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::operator::Operator;
use crate::resource::ResourceCost;
use crate::types::TypeIdentifier;
use crate::{OperatorError, Result};

/// 算子能力描述（用于能力匹配与调度）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorCapability {
    /// 输入类型集合
    pub input_types: Vec<TypeIdentifier>,
    /// 输出类型集合
    pub output_types: Vec<TypeIdentifier>,
    /// 资源消耗画像
    pub resource_profile: ResourceCost,
    /// 守恒约束集合
    pub conservation_constraints: Vec<String>,
    /// 是否线程安全可并行
    pub parallel_safe: bool,
}

impl OperatorCapability {
    pub fn from_operator(op: &dyn Operator) -> Self {
        let pair = op.type_pair();
        Self {
            input_types: vec![pair.input.clone()],
            output_types: vec![pair.output.clone()],
            resource_profile: op.resource_cost(),
            conservation_constraints: Vec::new(),
            parallel_safe: true,
        }
    }
}

/// 算子版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorVersion {
    pub version: String,
    pub registered_at: u64,
    pub author: String,
    pub changelog: String,
}

fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// 已注册算子条目
pub struct RegisteredOperator {
    pub id: String,
    pub metadata: crate::OperatorMetadata,
    pub capability: OperatorCapability,
    pub versions: Vec<OperatorVersion>,
    pub dependencies: Vec<String>,
    pub deprecated: bool,
    pub instance: Arc<dyn Operator>,
}

impl std::fmt::Debug for RegisteredOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegisteredOperator")
            .field("id", &self.id)
            .field("name", &self.metadata.name)
            .field("version", &self.metadata.version)
            .field("deprecated", &self.deprecated)
            .finish()
    }
}

/// 算子注册表
///
/// 统一管理所有算子的注册、查询、版本控制与血缘追踪。
///
/// # 示例
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use mox_flow_operator_core::registry::OperatorRegistry;
/// use mox_flow_operator_core::operator::IdentityOperator;
///
/// let mut registry = OperatorRegistry::new();
/// registry.register(Arc::new(IdentityOperator::new(3))).unwrap();
/// let op = registry.resolve("Identity", None).unwrap();
/// ```
pub struct OperatorRegistry {
    operators: HashMap<String, RegisteredOperator>,
    name_index: HashMap<String, String>,
}

impl Default for OperatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl OperatorRegistry {
    pub fn new() -> Self {
        Self {
            operators: HashMap::new(),
            name_index: HashMap::new(),
        }
    }

    /// 注册算子
    pub fn register(&mut self, op: Arc<dyn Operator>) -> Result<()> {
        let meta = op.metadata();
        let id = meta.id.clone();
        let name = meta.name.clone();
        let capability = OperatorCapability::from_operator(op.as_ref());

        let entry = RegisteredOperator {
            id: id.clone(),
            metadata: meta,
            capability,
            versions: vec![OperatorVersion {
                version: "1.0.0".to_string(),
                registered_at: current_timestamp_ms(),
                author: "System".to_string(),
                changelog: "初始注册".to_string(),
            }],
            dependencies: Vec::new(),
            deprecated: false,
            instance: op,
        };

        self.name_index.insert(name, id.clone());
        self.operators.insert(id, entry);
        Ok(())
    }

    /// 按 ID 或名称解析算子
    pub fn resolve(&self, id_or_name: &str, _version: Option<&str>) -> Result<Arc<dyn Operator>> {
        let id = self
            .name_index
            .get(id_or_name)
            .cloned()
            .unwrap_or_else(|| id_or_name.to_string());

        let entry = self
            .operators
            .get(&id)
            .ok_or_else(|| OperatorError::ExecutionError(format!("算子未注册: {}", id_or_name)))?;

        if entry.deprecated {
            tracing::warn!("算子 {} 已被标记为 deprecated", id_or_name);
        }

        Ok(entry.instance.clone())
    }

    /// 获取算子元数据
    pub fn get_metadata(&self, id_or_name: &str) -> Option<&crate::OperatorMetadata> {
        let id = self
            .name_index
            .get(id_or_name)
            .cloned()
            .unwrap_or_else(|| id_or_name.to_string());

        self.operators.get(&id).map(|e| &e.metadata)
    }

    /// 列出所有已注册算子
    pub fn list(&self) -> Vec<&RegisteredOperator> {
        self.operators.values().collect()
    }

    /// 按能力查找兼容算子
    pub fn find_compatible(
        &self,
        input_type: &TypeIdentifier,
        output_type: &TypeIdentifier,
    ) -> Vec<&RegisteredOperator> {
        self.operators
            .values()
            .filter(|entry| {
                !entry.deprecated
                    && entry.capability.input_types.iter().any(|t| {
                        t.matches(input_type) || t.matches(&crate::types::builtin::any_type())
                    })
                    && entry.capability.output_types.iter().any(|t| {
                        t.matches(output_type) || t.matches(&crate::types::builtin::any_type())
                    })
            })
            .collect()
    }

    /// 按输入类型查找算子
    pub fn find_by_input(&self, input_type: &TypeIdentifier) -> Vec<&RegisteredOperator> {
        self.operators
            .values()
            .filter(|entry| {
                !entry.deprecated
                    && entry.capability.input_types.iter().any(|t| {
                        t.matches(input_type) || t.matches(&crate::types::builtin::any_type())
                    })
            })
            .collect()
    }

    /// 按标签查找算子
    pub fn find_by_tag(&self, tag: &str) -> Vec<&RegisteredOperator> {
        self.operators
            .values()
            .filter(|entry| !entry.deprecated && entry.metadata.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// 标记算子为 deprecated
    pub fn deprecate(&mut self, id_or_name: &str) -> Result<()> {
        let id = self
            .name_index
            .get(id_or_name)
            .cloned()
            .unwrap_or_else(|| id_or_name.to_string());

        if let Some(entry) = self.operators.get_mut(&id) {
            entry.deprecated = true;
            Ok(())
        } else {
            Err(OperatorError::ExecutionError(format!(
                "算子未注册: {}",
                id_or_name
            )))
        }
    }

    /// 新增算子版本
    pub fn add_version(
        &mut self,
        id_or_name: &str,
        version: &str,
        author: &str,
        changelog: &str,
    ) -> Result<()> {
        let id = self
            .name_index
            .get(id_or_name)
            .cloned()
            .unwrap_or_else(|| id_or_name.to_string());

        if let Some(entry) = self.operators.get_mut(&id) {
            entry.versions.push(OperatorVersion {
                version: version.to_string(),
                registered_at: current_timestamp_ms(),
                author: author.to_string(),
                changelog: changelog.to_string(),
            });
            entry.metadata.version = version.to_string();
            Ok(())
        } else {
            Err(OperatorError::ExecutionError(format!(
                "算子未注册: {}",
                id_or_name
            )))
        }
    }

    /// 设置算子依赖关系
    pub fn set_dependencies(&mut self, id_or_name: &str, deps: Vec<String>) -> Result<()> {
        let id = self
            .name_index
            .get(id_or_name)
            .cloned()
            .unwrap_or_else(|| id_or_name.to_string());

        if let Some(entry) = self.operators.get_mut(&id) {
            entry.dependencies = deps;
            Ok(())
        } else {
            Err(OperatorError::ExecutionError(format!(
                "算子未注册: {}",
                id_or_name
            )))
        }
    }

    /// 获取算子血缘（依赖链）
    pub fn lineage(&self, id_or_name: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut visited = Vec::new();
        self.collect_lineage(id_or_name, &mut result, &mut visited);
        result
    }

    fn collect_lineage(
        &self,
        id_or_name: &str,
        result: &mut Vec<String>,
        visited: &mut Vec<String>,
    ) {
        if visited.contains(&id_or_name.to_string()) {
            return;
        }
        visited.push(id_or_name.to_string());

        let id = self
            .name_index
            .get(id_or_name)
            .cloned()
            .unwrap_or_else(|| id_or_name.to_string());

        if let Some(entry) = self.operators.get(&id) {
            for dep in &entry.dependencies {
                result.push(dep.clone());
                self.collect_lineage(dep, result, visited);
            }
        }
    }

    /// 已注册算子数量
    pub fn count(&self) -> usize {
        self.operators.len()
    }

    /// 导出注册表为 JSON
    pub fn export_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.operators.len(),
            "operators": self.operators.values().map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "name": e.metadata.name,
                    "version": e.metadata.version,
                    "deprecated": e.deprecated,
                    "tags": e.metadata.tags,
                    "input_type": e.capability.input_types.iter().map(|t| t.name.clone()).collect::<Vec<_>>(),
                    "output_type": e.capability.output_types.iter().map(|t| t.name.clone()).collect::<Vec<_>>(),
                    "parallel_safe": e.capability.parallel_safe,
                })
            }).collect::<Vec<_>>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::{FunctionOperator, IdentityOperator, LinearOperator};

    fn make_identity(dim: usize) -> Arc<dyn Operator> {
        Arc::new(IdentityOperator::new(dim))
    }

    fn make_linear() -> Arc<dyn Operator> {
        Arc::new(LinearOperator::identity(3))
    }

    fn make_function(name: &str) -> Arc<dyn Operator> {
        Arc::new(FunctionOperator::new(name, |s, _ctx| Ok(s.clone())))
    }

    #[test]
    fn test_register_and_resolve() {
        let mut registry = OperatorRegistry::new();
        registry.register(make_identity(3)).unwrap();
        registry.register(make_linear()).unwrap();

        assert_eq!(registry.count(), 2);

        let op = registry.resolve("Identity", None).unwrap();
        assert_eq!(op.metadata().name, "Identity");

        let op2 = registry.resolve("LinearTransform", None).unwrap();
        assert_eq!(op2.metadata().name, "LinearTransform");
    }

    #[test]
    fn test_find_compatible() {
        let mut registry = OperatorRegistry::new();
        registry.register(make_identity(3)).unwrap();
        registry.register(make_function("proc")).unwrap();

        let state_type = crate::types::builtin::state_vector_type();
        let compatible = registry.find_compatible(&state_type, &state_type);
        assert_eq!(compatible.len(), 2);
    }

    #[test]
    fn test_deprecate() {
        let mut registry = OperatorRegistry::new();
        registry.register(make_identity(3)).unwrap();
        registry.deprecate("Identity").unwrap();

        let entry = registry.get_metadata("Identity").unwrap();
        assert_eq!(entry.name, "Identity");
    }

    #[test]
    fn test_lineage() {
        let mut registry = OperatorRegistry::new();
        registry.register(make_identity(3)).unwrap();
        registry.register(make_function("child")).unwrap();

        registry
            .set_dependencies("Identity", vec!["child".to_string()])
            .unwrap();

        let lineage = registry.lineage("Identity");
        assert!(!lineage.is_empty());
    }

    #[test]
    fn test_export_json() {
        let mut registry = OperatorRegistry::new();
        registry.register(make_identity(3)).unwrap();

        let json = registry.export_json();
        assert!(json["count"].as_u64().unwrap() >= 1);
    }
}
