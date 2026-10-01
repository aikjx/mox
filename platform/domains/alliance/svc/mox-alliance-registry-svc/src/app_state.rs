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
    /// 是否启用后台主动健康探测（默认关闭，仅被动心跳租约）
    pub health_probe_enabled: bool,
    /// 主动探测周期（毫秒）
    pub health_probe_interval_ms: u64,
    /// 单次 HTTP 探测超时（毫秒）
    pub health_probe_timeout_ms: u64,
    /// 专家成功率健康判定阈值：成功率低于此值即 Degraded（默认 0.80）。
    /// 启动加载自 env；热更新接缝见 `Config::health_thresholds` 文档。
    pub health_healthy_min: f64,
    /// 专家健康判定阈值：失败占比高于此值即 Degraded（默认 0.20）。
    pub health_degraded_max_ratio: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:3400".to_string(),
            database_path: "./data/registry.db".to_string(), // allow: dev-default-prod-overridden
            snapshot_path: Some("./data/registry_instances.json".to_string()),
            reap_interval_ms: 5_000,
            // 主动探测默认关闭：保持「仅被动心跳租约」的历史行为，
            // 开启后才会启动后台探测任务（见 server.rs）。
            health_probe_enabled: false,
            health_probe_interval_ms: 30_000,
            health_probe_timeout_ms: 5_000,
            // 健康阈值单一来源的 svc 侧默认值（与 registry-core `HealthThresholds::DEFAULT` 对齐）。
            health_healthy_min: 0.80,
            health_degraded_max_ratio: 0.20,
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
    /// - `MOX_ALLIANCE_REGISTRY_PROBE_ENABLED`：主动健康探测开关（`1`/`true`/`yes` 开启）
    /// - `MOX_ALLIANCE_REGISTRY_PROBE_INTERVAL_MS`：探测周期（毫秒，须为正）
    /// - `MOX_ALLIANCE_REGISTRY_PROBE_TIMEOUT_MS`：单次探测超时（毫秒，须为正）
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
        if let Ok(v) = std::env::var("MOX_ALLIANCE_REGISTRY_PROBE_ENABLED") {
            let low = v.to_lowercase();
            cfg.health_probe_enabled = matches!(low.as_str(), "1" | "true" | "yes" | "on");
        }
        if let Ok(v) = std::env::var("MOX_ALLIANCE_REGISTRY_PROBE_INTERVAL_MS") {
            if let Ok(ms) = v.parse::<u64>() {
                if ms > 0 {
                    cfg.health_probe_interval_ms = ms;
                }
            }
        }
        if let Ok(v) = std::env::var("MOX_ALLIANCE_REGISTRY_PROBE_TIMEOUT_MS") {
            if let Ok(ms) = v.parse::<u64>() {
                if ms > 0 {
                    cfg.health_probe_timeout_ms = ms;
                }
            }
        }
        if let Ok(v) = std::env::var("MOX_ALLIANCE_REGISTRY_HEALTHY_MIN") {
            if let Ok(rate) = v.parse::<f64>() {
                if (0.0..=1.0).contains(&rate) {
                    cfg.health_healthy_min = rate;
                }
            }
        }
        if let Ok(v) = std::env::var("MOX_ALLIANCE_REGISTRY_DEGRADED_MAX_RATIO") {
            if let Ok(ratio) = v.parse::<f64>() {
                if (0.0..=1.0).contains(&ratio) {
                    cfg.health_degraded_max_ratio = ratio;
                }
            }
        }
        cfg
    }

    /// 把 svc 启动配置投影为 registry-core 的健康阈值单一来源。
    ///
    /// **热更新接缝（未做，需评审）**：本 svc 当前不依赖 `mox-alliance-config-core`
    /// （其带 tokio/store/broadcast，registry-svc 加这条依赖边需走 Cargo.toml 变更评审，
    /// 本轮铁律禁止）。故本阈值为**启动加载档**：进程启动时从 env 读入，运行期不可变。
    /// 后续若接入 config-engine 的 broadcast 变更事件，只需在收到 `health.*` 变更时
    /// 用本方法重建 `HealthThresholds` 并热替换 `AppState` 内的句柄，handler 签名不变。
    pub fn health_thresholds(&self) -> mox_alliance_registry_core::HealthThresholds {
        mox_alliance_registry_core::HealthThresholds {
            degraded_success_rate: self.health_healthy_min,
            degraded_failure_ratio: self.health_degraded_max_ratio,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use mox_alliance_registry_core::{
        ExpertHealth, ExpertMetrics, HealthThresholds, classify_with, record_result,
    };

    #[test]
    fn default_thresholds_are_80_20_and_aligned_with_core_default() {
        let cfg = Config::default();
        assert_eq!(cfg.health_healthy_min, 0.80);
        assert_eq!(cfg.health_degraded_max_ratio, 0.20);
        assert_eq!(cfg.health_thresholds(), HealthThresholds::DEFAULT);
    }

    #[test]
    fn custom_thresholds_change_classification_outcome() {
        // 一个 7/10 成功率（0.70）的专家：默认 0.80 阈值下 Degraded
        let mut m = ExpertMetrics::new("x");
        for _ in 0..7 {
            record_result(&mut m, true, 10);
        }
        for _ in 0..3 {
            record_result(&mut m, false, 10);
        }
        assert_eq!(
            classify_with(&m, &Config::default().health_thresholds()),
            ExpertHealth::Degraded
        );

        // svc 侧把阈值放宽到 0.60 → 同一专家转 Healthy
        let mut relaxed = Config::default();
        relaxed.health_healthy_min = 0.60;
        relaxed.health_degraded_max_ratio = 0.50;
        assert_eq!(
            classify_with(&m, &relaxed.health_thresholds()),
            ExpertHealth::Healthy
        );
    }

    #[test]
    fn custom_fields_project_into_thresholds() {
        let mut cfg = Config::default();
        cfg.health_healthy_min = 0.95;
        cfg.health_degraded_max_ratio = 0.10;
        let th = cfg.health_thresholds();
        assert_eq!(th.degraded_success_rate, 0.95);
        assert_eq!(th.degraded_failure_ratio, 0.10);
    }
}
