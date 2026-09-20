// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 应用状态

use std::sync::Arc;

use crate::storage::ExpertStore;

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    /// 服务配置
    pub config: Arc<Config>,
    /// 专家存储（SQLite）
    pub store: Arc<ExpertStore>,
}

/// 服务配置
#[derive(Debug, Clone)]
pub struct Config {
    /// 监听地址
    pub bind_addr: String,
    /// 数据库路径
    pub database_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:3400".to_string(),
            database_path: "./data/registry.db".to_string(),
        }
    }
}

impl AppState {
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let store = ExpertStore::open(&config.database_path)?;
        Ok(Self {
            config: Arc::new(config),
            store: Arc::new(store),
        })
    }
}
