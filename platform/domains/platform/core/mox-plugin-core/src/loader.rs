// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 插件加载器 — 从目录扫描、解析manifest、加载WASM模块、热重载

use crate::lifecycle::{LifecycleError, LifecycleEvent, PluginState};
use crate::manifest::PluginManifest;
use crate::registry::PluginRegistry;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// 插件加载器
pub struct PluginLoader {
    registry: Arc<PluginRegistry>,
    /// 插件根目录
    plugin_dir: RwLock<PathBuf>,
    /// 是否启用热重载
    hot_reload: RwLock<bool>,
    /// 热重载检查间隔
    hot_reload_interval: RwLock<Duration>,
}

impl PluginLoader {
    pub fn new(registry: Arc<PluginRegistry>, plugin_dir: impl Into<PathBuf>) -> Self {
        Self {
            registry,
            plugin_dir: RwLock::new(plugin_dir.into()),
            hot_reload: RwLock::new(false),
            hot_reload_interval: RwLock::new(Duration::from_secs(10)),
        }
    }

    pub fn plugin_dir(&self) -> PathBuf {
        self.plugin_dir.read().clone()
    }

    pub fn set_plugin_dir(&self, dir: impl Into<PathBuf>) {
        *self.plugin_dir.write() = dir.into();
    }

    pub fn enable_hot_reload(&self, interval: Duration) {
        *self.hot_reload.write() = true;
        *self.hot_reload_interval.write() = interval;
    }

    pub fn disable_hot_reload(&self) {
        *self.hot_reload.write() = false;
    }

    /// 扫描插件目录，加载所有插件
    pub async fn load_all(&self) -> Result<usize, LifecycleError> {
        let dir = self.plugin_dir.read().clone();
        if !dir.exists() {
            tracing::warn!("plugin directory not found: {:?}", dir);
            return Ok(0);
        }

        let mut loaded = 0;
        let mut entries = tokio::fs::read_dir(&dir).await
            .map_err(|e| LifecycleError::LoadFailed(format!("read dir failed: {}", e)))?;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() {
                match self.load_plugin_from_dir(&path).await {
                    Ok(_) => loaded += 1,
                    Err(e) => tracing::error!("failed to load plugin from {:?}: {}", path, e),
                }
            }
        }

        tracing::info!("loaded {} plugins from {:?}", loaded, dir);
        Ok(loaded)
    }

    /// 从目录加载单个插件
    pub async fn load_plugin_from_dir(&self, dir: &Path) -> Result<Arc<crate::registry::PluginInstance>, LifecycleError> {
        // 1. 读取manifest.json
        let manifest_path = dir.join("manifest.json");
        if !manifest_path.exists() {
            return Err(LifecycleError::LoadFailed(format!("manifest.json not found in {:?}", dir)));
        }

        let manifest_content = tokio::fs::read_to_string(&manifest_path).await
            .map_err(|e| LifecycleError::LoadFailed(format!("read manifest failed: {}", e)))?;

        let manifest = PluginManifest::from_json(&manifest_content)
            .map_err(|e| LifecycleError::LoadFailed(format!("parse manifest failed: {}", e)))?;

        // 2. 检查WASM入口文件
        let wasm_path = dir.join(&manifest.entry);
        if !wasm_path.exists() {
            return Err(LifecycleError::LoadFailed(format!("WASM entry not found: {:?}", wasm_path)));
        }

        // 3. 检查依赖
        self.registry.check_dependencies(&manifest)
            .map_err(|e| LifecycleError::LoadFailed(format!("dependency check failed: {}", e)))?;

        // 4. 注册插件
        let instance = self.registry.register(manifest.clone())?;

        // 5. 加载WASM模块（异步，不阻塞）
        let wasm_path_clone = wasm_path.clone();
        let instance_clone = instance.clone();
        tokio::spawn(async move {
            match Self::load_wasm_module(&wasm_path_clone).await {
                Ok(wasm_module) => {
                    *instance_clone.wasm_instance.write() = Some(wasm_module);
                    tracing::info!("WASM module loaded for plugin {}", instance_clone.id());
                }
                Err(e) => {
                    instance_clone.set_error(format!("WASM load failed: {}", e));
                }
            }
        });

        // 6. 发送事件
        self.registry.emit_event(LifecycleEvent {
            plugin_id: instance.id().to_string(),
            plugin_name: instance.name().to_string(),
            from: PluginState::Unloaded,
            to: PluginState::Loaded,
            timestamp: chrono::Utc::now().timestamp(),
            reason: None,
        });

        Ok(instance)
    }

    /// 加载WASM模块
    async fn load_wasm_module(path: &Path) -> Result<wasmer::Instance, String> {
        let wasm_bytes = tokio::fs::read(path).await
            .map_err(|e| format!("read WASM file failed: {}", e))?;

        // 编译+实例化（CPU密集，全部放spawn_blocking）
        tokio::task::spawn_blocking(move || -> Result<wasmer::Instance, String> {
            // wasmer 4.x: 用From trait从compiler创建Engine，Store接受所有权
            let compiler = wasmer_compiler_cranelift::Cranelift::default();
            let engine = wasmer::Engine::from(compiler);
            let mut store = wasmer::Store::new(engine);

            let module = wasmer::Module::new(&store, &wasm_bytes)
                .map_err(|e| format!("compile WASM failed: {}", e))?;

            let import_object = wasmer::imports! {
                "env" => {
                    "host_log" => wasmer::Function::new_typed(
                        &mut store,
                        |_msg: i32| { /* plugin log placeholder */ }
                    ),
                }
            };

            let instance = wasmer::Instance::new(&mut store, &module, &import_object)
                .map_err(|e| format!("instantiate WASM failed: {}", e))?;

            Ok(instance)
        })
        .await
        .map_err(|e| format!("spawn_blocking failed: {}", e))?
    }

    /// 卸载插件
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<(), LifecycleError> {
        let instance = self.registry.get(plugin_id)?;
        // 停止
        if instance.current_state().is_active() {
            instance.transition_to(PluginState::Stopped)?;
        }
        // 释放WASM实例
        *instance.wasm_instance.write() = None;
        // 注销
        self.registry.unregister(plugin_id)?;
        tracing::info!("plugin unloaded: {}", plugin_id);
        Ok(())
    }

    /// 重新加载插件（热重载）
    pub async fn reload_plugin(&self, plugin_id: &str) -> Result<(), LifecycleError> {
        let dir = self.plugin_dir.read().join(plugin_id);
        self.unload_plugin(plugin_id).await?;
        self.load_plugin_from_dir(&dir).await?;
        tracing::info!("plugin reloaded: {}", plugin_id);
        Ok(())
    }

    /// 启动热重载监控（后台任务）
    pub fn start_hot_reload_watcher(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let interval = *self.hot_reload_interval.read();
            let mut ticker = tokio::time::interval(interval);
            tracing::info!("plugin hot-reload watcher started (interval: {:?})", interval);

            loop {
                ticker.tick().await;
                if !*self.hot_reload.read() { continue; }

                // 检查manifest文件变化（简化：比较修改时间）
                // 实际实现应使用notify crate监听文件系统事件
                let dir = self.plugin_dir.read().clone();
                if let Ok(mut entries) = tokio::fs::read_dir(&dir).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let manifest_path = entry.path().join("manifest.json");
                        if manifest_path.exists() {
                            // 检查是否已加载且manifest有变化
                            // 简化：这里只打日志，实际实现需记录上次修改时间
                            tracing::trace!("hot-reload check: {:?}", manifest_path);
                        }
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_all_empty_dir() {
        let registry = Arc::new(PluginRegistry::new());
        let temp_dir = std::env::temp_dir().join("mox_plugin_test_empty");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let loader = PluginLoader::new(registry, temp_dir);
        let count = loader.load_all().await.unwrap();
        assert_eq!(count, 0);
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
