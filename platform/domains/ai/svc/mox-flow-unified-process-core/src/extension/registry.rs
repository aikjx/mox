// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 扩展注册表
//!
//! 各服务可以注册自定义扩展（如 LLM 客户端、数据库连接池等），
//! 节点处理器通过 `ExecutionContext.extensions` 获取这些扩展能力。
//!
//! 这是核心库与业务服务之间的另一个解耦点：
//! 核心库不依赖具体业务实现，只通过 trait 定义接口。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// 扩展注册表
///
/// 基于 TypeId 的类型安全扩展容器。
/// 各服务将自己的能力（如 LLM 客户端、算子调用器等）注册进来，
/// 节点处理器在执行时通过类型获取。
#[derive(Default)]
pub struct ExtensionRegistry {
    extensions: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    names: HashMap<TypeId, &'static str>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
            names: HashMap::new(),
        }
    }

    /// 注册一个扩展
    ///
    /// T 必须是 Send + Sync + 'static 的类型。
    pub fn register<T: Send + Sync + 'static>(&mut self, name: &'static str, value: T) {
        let type_id = TypeId::of::<T>();
        self.extensions.insert(type_id, Arc::new(value));
        self.names.insert(type_id, name);
    }

    /// 获取一个扩展
    ///
    /// 返回 Arc<T>，如果未注册返回 None。
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let type_id = TypeId::of::<T>();
        self.extensions
            .get(&type_id)
            .and_then(|arc| arc.clone().downcast::<T>().ok())
    }

    /// 检查某个类型的扩展是否已注册
    pub fn has<T: Send + Sync + 'static>(&self) -> bool {
        self.extensions.contains_key(&TypeId::of::<T>())
    }

    /// 获取扩展名称（用于日志）
    pub fn name_of<T: Send + Sync + 'static>(&self) -> Option<&'static str> {
        self.names.get(&TypeId::of::<T>()).copied()
    }

    /// 已注册的扩展数量
    pub fn len(&self) -> usize {
        self.extensions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestExtension {
        value: String,
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = ExtensionRegistry::new();
        registry.register(
            "test_ext",
            TestExtension {
                value: "hello".into(),
            },
        );

        assert!(registry.has::<TestExtension>());
        assert_eq!(registry.name_of::<TestExtension>(), Some("test_ext"));

        let ext = registry.get::<TestExtension>().unwrap();
        assert_eq!(ext.value, "hello");
    }

    #[test]
    fn test_get_missing_returns_none() {
        let registry = ExtensionRegistry::new();
        assert!(registry.get::<TestExtension>().is_none());
        assert!(!registry.has::<TestExtension>());
    }

    #[test]
    fn test_empty_registry() {
        let registry = ExtensionRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }
}
