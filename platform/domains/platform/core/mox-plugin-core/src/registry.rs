// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 插件注册表 — 管理所有已加载插件的状态和实例

use crate::lifecycle::{LifecycleError, LifecycleEvent, PluginState};
use crate::manifest::{PluginConfig, PluginManifest};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// 插件实例（运行时状态）
pub struct PluginInstance {
    /// 插件描述符
    pub manifest: PluginManifest,
    /// 当前状态
    pub state: RwLock<PluginState>,
    /// 运行时配置
    pub config: RwLock<PluginConfig>,
    /// WASM模块实例（运行时设置）
    pub wasm_instance: RwLock<Option<wasmer::Instance>>,
    /// 注册的能力ID列表
    pub capabilities: RwLock<Vec<String>>,
    /// 加载时间戳
    pub loaded_at: i64,
    /// 最后错误信息
    pub last_error: RwLock<Option<String>>,
}

impl PluginInstance {
    pub fn new(manifest: PluginManifest) -> Self {
        Self {
            manifest,
            state: RwLock::new(PluginState::Loaded),
            config: RwLock::new(PluginConfig::default()),
            wasm_instance: RwLock::new(None),
            capabilities: RwLock::new(vec![]),
            loaded_at: chrono::Utc::now().timestamp(),
            last_error: RwLock::new(None),
        }
    }

    pub fn id(&self) -> &str { &self.manifest.id }
    pub fn name(&self) -> &str { &self.manifest.name }
    pub fn version(&self) -> &str { &self.manifest.version }

    pub fn current_state(&self) -> PluginState {
        *self.state.read()
    }

    /// 安全状态转换
    pub fn transition_to(&self, target: PluginState) -> Result<(), LifecycleError> {
        let mut state = self.state.write();
        if !state.can_transition_to(target) {
            return Err(LifecycleError::InvalidTransition {
                from: *state,
                to: target,
            });
        }
        tracing::info!("plugin {} state: {} -> {}", self.id(), state, target);
        *state = target;
        Ok(())
    }

    pub fn set_error(&self, error: impl Into<String>) {
        *self.last_error.write() = Some(error.into());
        let _ = self.transition_to(PluginState::Error);
    }

    pub fn is_running(&self) -> bool {
        self.current_state() == PluginState::Running
    }
}

/// 插件注册表
pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, Arc<PluginInstance>>>,
    /// 生命周期事件发送器
    event_sender: flume::Sender<LifecycleEvent>,
    /// 生命周期事件接收器（供外部订阅）
    event_receiver: flume::Receiver<LifecycleEvent>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        let (tx, rx) = flume::unbounded();
        Self {
            plugins: RwLock::new(HashMap::new()),
            event_sender: tx,
            event_receiver: rx,
        }
    }

    /// 注册插件（状态=Loaded）
    pub fn register(&self, manifest: PluginManifest) -> Result<Arc<PluginInstance>, LifecycleError> {
        let id = manifest.id.clone();
        if self.plugins.read().contains_key(&id) {
            return Err(LifecycleError::LoadFailed(format!("plugin already registered: {}", id)));
        }
        let instance = Arc::new(PluginInstance::new(manifest));
        self.plugins.write().insert(id.clone(), instance.clone());
        tracing::info!("plugin registered: {} v{}", instance.name(), instance.version());
        Ok(instance)
    }

    /// 注销插件
    pub fn unregister(&self, plugin_id: &str) -> Result<(), LifecycleError> {
        let instance = self.get(plugin_id)?;
        // 必须先停止
        if instance.current_state().is_active() {
            instance.transition_to(PluginState::Stopped)?;
        }
        instance.transition_to(PluginState::Unloaded)?;
        self.plugins.write().remove(plugin_id);
        tracing::info!("plugin unregistered: {}", plugin_id);
        Ok(())
    }

    /// 获取插件实例
    pub fn get(&self, plugin_id: &str) -> Result<Arc<PluginInstance>, LifecycleError> {
        self.plugins.read()
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| LifecycleError::NotFound(plugin_id.into()))
    }

    /// 列出所有插件
    pub fn list(&self) -> Vec<Arc<PluginInstance>> {
        self.plugins.read().values().cloned().collect()
    }

    /// 按状态筛选
    pub fn list_by_state(&self, state: PluginState) -> Vec<Arc<PluginInstance>> {
        self.plugins.read()
            .values()
            .filter(|p| p.current_state() == state)
            .cloned()
            .collect()
    }

    /// 按能力查找插件
    pub fn find_by_capability(&self, capability_id: &str) -> Vec<Arc<PluginInstance>> {
        self.plugins.read()
            .values()
            .filter(|p| {
                p.is_running()
                    && p.manifest.capabilities.iter().any(|c| c.id == capability_id)
            })
            .cloned()
            .collect()
    }

    /// 按标签筛选
    pub fn find_by_tag(&self, tag: &str) -> Vec<Arc<PluginInstance>> {
        self.plugins.read()
            .values()
            .filter(|p| p.manifest.tags.iter().any(|t| t == tag))
            .cloned()
            .collect()
    }

    /// 插件数量
    pub fn len(&self) -> usize {
        self.plugins.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.read().is_empty()
    }

    /// 发送生命周期事件
    pub fn emit_event(&self, event: LifecycleEvent) {
        let _ = self.event_sender.send(event);
    }

    /// 获取事件接收器（用于监听生命周期变化）
    pub fn event_receiver(&self) -> &flume::Receiver<LifecycleEvent> {
        &self.event_receiver
    }

    /// 检查依赖是否满足
    pub fn check_dependencies(&self, manifest: &PluginManifest) -> Result<(), String> {
        for dep in &manifest.dependencies {
            if dep.optional { continue; }
            match self.plugins.read().get(&dep.id) {
                Some(instance) => {
                    if !instance.manifest.version_matches(&dep.version) {
                        return Err(format!(
                            "dependency {} version mismatch: required {}, got {}",
                            dep.id, dep.version, instance.version()
                        ));
                    }
                    if !instance.current_state().is_active() {
                        return Err(format!("dependency {} not active", dep.id));
                    }
                }
                None => return Err(format!("dependency not found: {}", dep.id)),
            }
        }
        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manifest(id: &str) -> PluginManifest {
        PluginManifest {
            id: id.into(), name: "Test".into(), version: "1.0.0".into(),
            author: "test".into(), description: "test".into(), entry: "test.wasm".into(),
            permissions: vec![], dependencies: vec![], config_schema: vec![],
            capabilities: vec![], tags: vec![], homepage: None, repository: None,
            license: None, min_platform_version: "3.0.0".into(),
        }
    }

    #[test]
    fn test_register_and_get() {
        let registry = PluginRegistry::new();
        let instance = registry.register(test_manifest("test.plugin")).unwrap();
        assert_eq!(registry.len(), 1);
        assert!(registry.get("test.plugin").is_ok());
        let got = registry.get("test.plugin").unwrap();
        assert_eq!(got.id(), "test.plugin");
    }

    #[test]
    fn test_state_transition() {
        let registry = PluginRegistry::new();
        let instance = registry.register(test_manifest("test.plugin")).unwrap();
        assert_eq!(instance.current_state(), PluginState::Loaded);
        instance.transition_to(PluginState::Initialized).unwrap();
        instance.transition_to(PluginState::Running).unwrap();
        assert!(instance.is_running());
    }

    #[test]
    fn test_invalid_transition() {
        let registry = PluginRegistry::new();
        let instance = registry.register(test_manifest("test.plugin")).unwrap();
        // Loaded -> Running 非法（必须先Initialized）
        let result = instance.transition_to(PluginState::Running);
        assert!(result.is_err());
    }

    #[test]
    fn test_unregister() {
        let registry = PluginRegistry::new();
        registry.register(test_manifest("test.plugin")).unwrap();
        registry.unregister("test.plugin").unwrap();
        assert!(registry.is_empty());
    }
}
