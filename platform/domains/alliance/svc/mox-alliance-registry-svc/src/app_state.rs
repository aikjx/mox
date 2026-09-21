// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 应用状态
//!
//! 同时持有：
//! - [`ExpertStore`]：静态专家目录（SQLite，向后兼容旧 `/api/v1/experts`）
//! - [`RegistryStore`]：应用级专家实例注册中心（内存 + JSON 快照，核心能力）
//!
//! 配置加载优先级：内置默认 < 环境变量 `MOX_ALLIANCE_REGISTRY_*`。
//! （与联盟域 `MOX_ALLIANCE_*` 命名约定一致。）

use std::sync::Arc;

use crate::storage::{ExpertStore, RegistryStore};

/// 应用共享状态（Clone 后分发到各 axum handler）
#[derive(Clone)]
pub struct AppState {
    /// 服务配置
    pub config: Arc<Config>,
    /// 静态专家目录存储（SQLite）
    pub dir_store: Arc<ExpertStore>,
    /// 应用级专家实例注册中心
    pub registry: Arc<RegistryStore>,
}

/// 服务配置
#[derive(Debug, Clone)]
pub struct Config {
    /// 监听地址（端口 3400，见 docs/api/PORT-REGISTRY.md）
    pub bind_addr: String,
    /// 静态目录 SQLite 路径
    pub database_path: String,
    /// 实例注册中心 JSON 快照路径；为空则纯内存
    pub snapshot_path: Option<String>,
    /// 后台过期回收任务间隔（毫秒）
    pub reap_interval_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:3400".to_string(),
            database_path: "./data/registry.db".to_string(), // allow: dev-default-prod-overridden
            snapshot_path: Some("./data/registry_instances.json".to_string()),
            reap_interval_ms: 5_000,
        }
    }
}

impl Config {
    /// 从环境变量覆盖默认配置
    ///
    /// - `MOX_ALLIANCE_REGISTRY_ADDR`：监听地址（如 `0.0.0.0:3400`）
    /// - `MOX_ALLIANCE_REGISTRY_DB`：SQLite 路径
    /// - `MOX_ALLIANCE_REGISTRY_SNAPSHOT`：实例快照路径；设为空串则纯内存
    /// - `MOX_ALLIANCE_REGISTRY_REAP_MS`：回收任务间隔（毫秒）
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(v) = std::env::var("MOX_ALLIANCE_REGISTRY_ADDR") {
            if !v.is_empty() {
                cfg.bind_addr = v;
            }
        }
        if let Ok(v) = std::env::var("MOX_ALLIANCE_REGISTRY_DB") {
            if !v.is_empty() {
                cfg.database_path = v;
            }
        }
        if let Ok(v) = std::env::var("MOX_ALLIANCE_REGISTRY_SNAPSHOT") {
            cfg.snapshot_path = if v.is_empty() { None } else { Some(v) };
        }
        if let Ok(v) = std::env::var("MOX_ALLIANCE_REGISTRY_REAP_MS") {
            if let Ok(ms) = v.parse::<u64>() {
                if ms > 0 {
                    cfg.reap_interval_ms = ms;
                }
            }
        }
        cfg
    }
}

impl AppState {
    /// 用配置构造应用状态：打开目录库 + 装载注册中心快照
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let dir_store = Arc::new(ExpertStore::open(&config.database_path)?);
        let registry = match &config.snapshot_path {
            Some(path) => Arc::new(RegistryStore::with_snapshot(path)),
            None => Arc::new(RegistryStore::new_memory()),
        };
        Ok(Self {
            config: Arc::new(config),
            dir_store,
            registry,
        })
    }

    /// 测试用：纯内存状态（不落盘、不连真实 DB）
    pub fn for_test() -> Self {
        let dir_store = Arc::new(ExpertStore::memory().expect("in-memory db"));
        let registry = Arc::new(RegistryStore::new_memory());
        Self {
            config: Arc::new(Config::default()),
            dir_store,
            registry,
        }
    }
}
