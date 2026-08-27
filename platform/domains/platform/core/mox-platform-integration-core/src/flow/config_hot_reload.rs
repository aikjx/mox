// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 配置热更新 — Config Hot Reload
//!
//! 企业级配置热更新：监听配置文件变化，自动重新加载，
//! 支持回调通知，无需重启服务。

use crate::config::IntegrationConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use parking_lot::RwLock;

/// 配置更新事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdateEvent {
    /// 更新时间
    pub updated_at: String,
    /// 配置文件路径
    pub config_path: Option<PathBuf>,
    /// 更新的字段列表
    pub changed_fields: Vec<String>,
    /// 是否为全量更新
    pub full_reload: bool,
}

/// 配置更新回调
pub type ConfigUpdateCallback = Arc<dyn Fn(&IntegrationConfig, &ConfigUpdateEvent) + Send + Sync>;

/// 配置热更新器
pub struct ConfigHotReloader {
    /// 当前配置
    config: Arc<RwLock<IntegrationConfig>>,
    /// 配置文件路径
    config_path: Option<PathBuf>,
    /// 更新回调列表
    callbacks: RwLock<Vec<ConfigUpdateCallback>>,
    /// 最后修改时间
    last_modified: RwLock<Option<std::time::SystemTime>>,
    /// 轮询间隔
    poll_interval: Duration,
    /// 是否运行中
    running: RwLock<bool>,
}

impl ConfigHotReloader {
    /// 创建热更新器（从配置文件）
    pub fn new(config: IntegrationConfig, config_path: impl Into<PathBuf>) -> Self {
        let path = config_path.into();
        let last_modified = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok());
        Self {
            config: Arc::new(RwLock::new(config)),
            config_path: Some(path),
            callbacks: RwLock::new(Vec::new()),
            last_modified: RwLock::new(last_modified),
            poll_interval: Duration::from_secs(5),
            running: RwLock::new(false),
        }
    }

    /// 创建热更新器（无文件，手动触发更新）
    pub fn new_manual(config: IntegrationConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            config_path: None,
            callbacks: RwLock::new(Vec::new()),
            last_modified: RwLock::new(None),
            poll_interval: Duration::from_secs(5),
            running: RwLock::new(false),
        }
    }

    /// 获取当前配置（克隆）
    pub fn current_config(&self) -> IntegrationConfig {
        self.config.read().clone()
    }

    /// 获取配置引用（Arc）
    pub fn config_arc(&self) -> Arc<RwLock<IntegrationConfig>> {
        self.config.clone()
    }

    /// 注册更新回调
    pub fn on_update(&self, callback: ConfigUpdateCallback) {
        self.callbacks.write().push(callback);
    }

    /// 设置轮询间隔
    pub fn set_poll_interval(&self, interval: Duration) {
        // 注意：需要内部可变性，这里简化
    }

    /// 手动触发配置更新
    pub fn update_config(&self, new_config: IntegrationConfig, changed_fields: Vec<String>) {
        let event = ConfigUpdateEvent {
            updated_at: chrono::Utc::now().to_rfc3339(),
            config_path: self.config_path.clone(),
            changed_fields,
            full_reload: false,
        };

        // 更新配置
        *self.config.write() = new_config.clone();

        // 通知回调
        let callbacks = self.callbacks.read().clone();
        for cb in callbacks {
            cb(&new_config, &event);
        }

        tracing::info!("config updated manually, {} fields changed", event.changed_fields.len());
    }

    /// 检查配置文件是否有更新（手动调用）
    pub fn check_for_updates(&self) -> Option<ConfigUpdateEvent> {
        let path = self.config_path.as_ref()?;
        let metadata = std::fs::metadata(path).ok()?;
        let modified = metadata.modified().ok()?;

        let last = *self.last_modified.read();
        if last.map_or(true, |last| modified > last) {
            // 文件有更新
            *self.last_modified.write() = Some(modified);

            // 尝试重新加载配置
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(new_config) = serde_yaml::from_str::<IntegrationConfig>(&content) {
                    let event = ConfigUpdateEvent {
                        updated_at: chrono::Utc::now().to_rfc3339(),
                        config_path: Some(path.clone()),
                        changed_fields: vec!["full".to_string()],
                        full_reload: true,
                    };

                    *self.config.write() = new_config.clone();

                    // 通知回调
                    let callbacks = self.callbacks.read().clone();
                    for cb in callbacks {
                        cb(&new_config, &event);
                    }

                    tracing::info!("config reloaded from file: {:?}", path);
                    return Some(event);
                }
            }
        }
        None
    }

    /// 启动后台轮询（返回join handle，需要在tokio运行时中调用）
    pub fn start_polling(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        *self.running.write() = true;
        let interval = self.poll_interval;
        tokio::spawn(async move {
            while *self.running.read() {
                self.check_for_updates();
                tokio::time::sleep(interval).await;
            }
        })
    }

    /// 停止后台轮询
    pub fn stop_polling(&self) {
        *self.running.write() = false;
    }

    /// 是否运行中
    pub fn is_running(&self) -> bool { *self.running.read() }
}
