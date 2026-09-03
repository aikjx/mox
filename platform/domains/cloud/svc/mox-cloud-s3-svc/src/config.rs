// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! S3 服务全局配置模块
//!
//! 将原先散落在各模块中的硬编码参数统一提取到配置结构体，支持：
//! - 编译期默认值（`Default` trait）
//! - 环境变量运行时覆盖（`from_env()`）
//! - Feature flag 控制可选 RustFS 后端接入点（默认关闭，不引入实际依赖）
//!
//! 环境变量命名约定：
//! - Feature flags：`MOX_FEATURE_<NAME>`（true/false/1/0/yes/no/on/off）
//! - 生命周期：`MOX_S3_LIFECYCLE_<FIELD>`
//! - 复制：`MOX_S3_REPLICATION_<FIELD>`
//! - 清单：`MOX_S3_INVENTORY_<FIELD>`
//! - 扫描预算：`MOX_S3_SCAN_<DIMENSION>_<FIELD>`
//!
//! 算法参考：RustFS lifecycle evaluator 配置集成（Apache 2.0），
//! `ais/RustFS/crates/lifecycle/src/evaluator.rs`，本实现为自研重写。

use crate::scanner::ScanBudget;
use std::time::Duration;

// ---------------------------------------------------------------------------
// 顶层配置
// ---------------------------------------------------------------------------

/// S3 服务全局配置
#[derive(Debug, Clone)]
pub struct S3ServiceConfig {
    /// 生命周期配置
    pub lifecycle: LifecycleConfig,
    /// 扫描预算（三维：时间 / IO / 容量）
    pub scan_budget: ScanBudget,
    /// 复制配置
    pub replication: ReplicationConfig,
    /// 清单配置
    pub inventory: InventoryConfig,
    /// 功能开关
    pub features: FeatureFlags,
}

// ---------------------------------------------------------------------------
// 生命周期配置
// ---------------------------------------------------------------------------

/// 生命周期配置（提取自 lifecycle.rs 硬编码阈值）
#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    /// Hot → Warm 转换天数（默认 30）
    pub hot_to_warm_days: u32,
    /// Warm → Cold 转换天数（默认 90）
    pub warm_to_cold_days: u32,
    /// Cold → Glacier 转换天数（默认 180）
    ///
    /// 注意：lifecycle.rs 中 `LifecycleThresholds::cold_to_glacier_ms` 默认 365 天，
    /// 此处配置默认 180 天为更激进的归档策略，使用方需显式设置以覆盖。
    pub cold_to_glacier_days: u32,
    /// 扫描间隔秒数（默认 3600s = 1 小时）
    pub scan_interval_secs: u64,
    /// 是否启用生命周期（默认 true）
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// 复制配置
// ---------------------------------------------------------------------------

/// 跨区域 / 同区域复制配置
#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    /// 复制超时秒数（默认 300s = 5 分钟）
    pub timeout_secs: u64,
    /// 最大重试次数（默认 3）
    pub max_retries: u32,
    /// 复制并发数（默认 4）
    pub parallelism: usize,
}

// ---------------------------------------------------------------------------
// 清单配置
// ---------------------------------------------------------------------------

/// 存储桶清单（Inventory）配置
#[derive(Debug, Clone)]
pub struct InventoryConfig {
    /// 清单生成间隔秒数（默认 86400s = 1 天）
    pub interval_secs: u64,
    /// 最大并发清单任务数（默认 2）
    pub max_concurrent_jobs: usize,
}

// ---------------------------------------------------------------------------
// Feature Flags
// ---------------------------------------------------------------------------

/// 功能开关（feature flags）
///
/// 所有 RustFS 后端接入点默认 `false`，仅定义开关和接入点，
/// 不实际引入 RustFS 依赖或代码。启用后由上层服务在初始化时
/// 选择对应的后端实现（预留接入点）。
#[derive(Debug, Clone)]
pub struct FeatureFlags {
    /// 是否启用 RustFS ecstore 作为可选后端（默认 false）
    pub rustfs_ecstore_backend: bool,
    /// 是否启用 RustFS rio I/O 管线（默认 false）
    pub rustfs_rio_backend: bool,
    /// 是否启用 hedged read（对冲读，默认 true）
    pub hedged_read_enabled: bool,
    /// 是否启用背压（默认 true）
    pub backpressure_enabled: bool,
    /// 是否启用生命周期复制门控（默认 true）
    pub replication_gate_enabled: bool,
    /// 是否启用 DeleteAllVersions 短路路径（默认 true）
    pub delete_all_versions_short_circuit: bool,
}

// ---------------------------------------------------------------------------
// Default 实现
// ---------------------------------------------------------------------------

impl Default for S3ServiceConfig {
    fn default() -> Self {
        Self {
            lifecycle: LifecycleConfig {
                hot_to_warm_days: 30,
                warm_to_cold_days: 90,
                cold_to_glacier_days: 180,
                scan_interval_secs: 3600,
                enabled: true,
            },
            scan_budget: ScanBudget::default(),
            replication: ReplicationConfig { timeout_secs: 300, max_retries: 3, parallelism: 4 },
            inventory: InventoryConfig { interval_secs: 86400, max_concurrent_jobs: 2 },
            features: FeatureFlags {
                rustfs_ecstore_backend: false,
                rustfs_rio_backend: false,
                hedged_read_enabled: true,
                backpressure_enabled: true,
                replication_gate_enabled: true,
                delete_all_versions_short_circuit: true,
            },
        }
    }
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        S3ServiceConfig::default().lifecycle
    }
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        S3ServiceConfig::default().replication
    }
}

impl Default for InventoryConfig {
    fn default() -> Self {
        S3ServiceConfig::default().inventory
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        S3ServiceConfig::default().features
    }
}

// ---------------------------------------------------------------------------
// 环境变量加载
// ---------------------------------------------------------------------------

impl S3ServiceConfig {
    /// 从环境变量加载配置，覆盖默认值
    ///
    /// 解析失败时静默使用默认值（不 panic）。所有环境变量均为可选。
    ///
    /// # 支持的环境变量
    ///
    /// | 环境变量 | 类型 | 对应字段 |
    /// |---|---|---|
    /// | `MOX_S3_LIFECYCLE_HOT_TO_WARM_DAYS` | u32 | lifecycle.hot_to_warm_days |
    /// | `MOX_S3_LIFECYCLE_WARM_TO_COLD_DAYS` | u32 | lifecycle.warm_to_cold_days |
    /// | `MOX_S3_LIFECYCLE_COLD_TO_GLACIER_DAYS` | u32 | lifecycle.cold_to_glacier_days |
    /// | `MOX_S3_LIFECYCLE_SCAN_INTERVAL_SECS` | u64 | lifecycle.scan_interval_secs |
    /// | `MOX_S3_LIFECYCLE_ENABLED` | bool | lifecycle.enabled |
    /// | `MOX_S3_REPLICATION_TIMEOUT_SECS` | u64 | replication.timeout_secs |
    /// | `MOX_S3_REPLICATION_MAX_RETRIES` | u32 | replication.max_retries |
    /// | `MOX_S3_REPLICATION_PARALLELISM` | usize | replication.parallelism |
    /// | `MOX_S3_INVENTORY_INTERVAL_SECS` | u64 | inventory.interval_secs |
    /// | `MOX_S3_INVENTORY_MAX_CONCURRENT_JOBS` | usize | inventory.max_concurrent_jobs |
    /// | `MOX_S3_SCAN_MAX_OBJECTS_PER_SEC` | u32 | scan_budget.io.max_objects_per_sec |
    /// | `MOX_S3_SCAN_MAX_IO_PER_SEC` | u32 | scan_budget.io.max_io_per_sec |
    /// | `MOX_S3_SCAN_MAX_PARALLELISM` | usize | scan_budget.io.max_parallelism |
    /// | `MOX_S3_SCAN_MAX_BYTES_PER_SCAN` | u64 | scan_budget.capacity.max_bytes_per_scan |
    /// | `MOX_S3_SCAN_MAX_MIGRATION_BYTES` | u64 | scan_budget.capacity.max_migration_bytes |
    /// | `MOX_S3_SCAN_MAX_OBJECTS_PER_SCAN` | u64 | scan_budget.capacity.max_objects_per_scan |
    /// | `MOX_S3_SCAN_MAX_DURATION_SECS` | u64 | scan_budget.time.max_duration (秒) |
    /// | `MOX_FEATURE_RUSTFS_ECSTORE` | bool | features.rustfs_ecstore_backend |
    /// | `MOX_FEATURE_RUSTFS_RIO` | bool | features.rustfs_rio_backend |
    /// | `MOX_FEATURE_HEDGED_READ` | bool | features.hedged_read_enabled |
    /// | `MOX_FEATURE_BACKPRESSURE` | bool | features.backpressure_enabled |
    /// | `MOX_FEATURE_REPLICATION_GATE` | bool | features.replication_gate_enabled |
    /// | `MOX_FEATURE_DELETE_ALL_VERSIONS_SHORT_CIRCUIT` | bool | features.delete_all_versions_short_circuit |
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // ---- 生命周期 ----
        if let Some(v) = env_u32("MOX_S3_LIFECYCLE_HOT_TO_WARM_DAYS") {
            config.lifecycle.hot_to_warm_days = v;
        }
        if let Some(v) = env_u32("MOX_S3_LIFECYCLE_WARM_TO_COLD_DAYS") {
            config.lifecycle.warm_to_cold_days = v;
        }
        if let Some(v) = env_u32("MOX_S3_LIFECYCLE_COLD_TO_GLACIER_DAYS") {
            config.lifecycle.cold_to_glacier_days = v;
        }
        if let Some(v) = env_u64("MOX_S3_LIFECYCLE_SCAN_INTERVAL_SECS") {
            config.lifecycle.scan_interval_secs = v;
        }
        if let Some(v) = env_bool("MOX_S3_LIFECYCLE_ENABLED") {
            config.lifecycle.enabled = v;
        }

        // ---- 复制 ----
        if let Some(v) = env_u64("MOX_S3_REPLICATION_TIMEOUT_SECS") {
            config.replication.timeout_secs = v;
        }
        if let Some(v) = env_u32("MOX_S3_REPLICATION_MAX_RETRIES") {
            config.replication.max_retries = v;
        }
        if let Some(v) = env_usize("MOX_S3_REPLICATION_PARALLELISM") {
            config.replication.parallelism = v;
        }

        // ---- 清单 ----
        if let Some(v) = env_u64("MOX_S3_INVENTORY_INTERVAL_SECS") {
            config.inventory.interval_secs = v;
        }
        if let Some(v) = env_usize("MOX_S3_INVENTORY_MAX_CONCURRENT_JOBS") {
            config.inventory.max_concurrent_jobs = v;
        }

        // ---- 扫描预算：IO ----
        if let Some(v) = env_u32("MOX_S3_SCAN_MAX_OBJECTS_PER_SEC") {
            config.scan_budget.io.max_objects_per_sec = v;
        }
        if let Some(v) = env_u32("MOX_S3_SCAN_MAX_IO_PER_SEC") {
            config.scan_budget.io.max_io_per_sec = v;
        }
        if let Some(v) = env_usize("MOX_S3_SCAN_MAX_PARALLELISM") {
            config.scan_budget.io.max_parallelism = v;
        }

        // ---- 扫描预算：容量 ----
        if let Some(v) = env_u64("MOX_S3_SCAN_MAX_BYTES_PER_SCAN") {
            config.scan_budget.capacity.max_bytes_per_scan = v;
        }
        if let Some(v) = env_u64("MOX_S3_SCAN_MAX_MIGRATION_BYTES") {
            config.scan_budget.capacity.max_migration_bytes = v;
        }
        if let Some(v) = env_u64("MOX_S3_SCAN_MAX_OBJECTS_PER_SCAN") {
            config.scan_budget.capacity.max_objects_per_scan = v;
        }

        // ---- 扫描预算：时间 ----
        if let Some(v) = env_u64("MOX_S3_SCAN_MAX_DURATION_SECS") {
            config.scan_budget.time.max_duration = Some(Duration::from_secs(v));
        }

        // ---- Feature Flags ----
        if let Some(v) = env_bool("MOX_FEATURE_RUSTFS_ECSTORE") {
            config.features.rustfs_ecstore_backend = v;
        }
        if let Some(v) = env_bool("MOX_FEATURE_RUSTFS_RIO") {
            config.features.rustfs_rio_backend = v;
        }
        if let Some(v) = env_bool("MOX_FEATURE_HEDGED_READ") {
            config.features.hedged_read_enabled = v;
        }
        if let Some(v) = env_bool("MOX_FEATURE_BACKPRESSURE") {
            config.features.backpressure_enabled = v;
        }
        if let Some(v) = env_bool("MOX_FEATURE_REPLICATION_GATE") {
            config.features.replication_gate_enabled = v;
        }
        if let Some(v) = env_bool("MOX_FEATURE_DELETE_ALL_VERSIONS_SHORT_CIRCUIT") {
            config.features.delete_all_versions_short_circuit = v;
        }

        config
    }

    /// 将生命周期天数转换为 `LifecycleThresholds`（毫秒）
    ///
    /// 方便直接传入 `HotWarmColdLifecycle::new()`。
    pub fn lifecycle_thresholds_ms(&self) -> (u64, u64, u64) {
        const DAY_MS: u64 = 24 * 60 * 60 * 1000;
        (
            self.lifecycle.hot_to_warm_days as u64 * DAY_MS,
            self.lifecycle.warm_to_cold_days as u64 * DAY_MS,
            self.lifecycle.cold_to_glacier_days as u64 * DAY_MS,
        )
    }
}

// ---------------------------------------------------------------------------
// 环境变量解析辅助函数
// ---------------------------------------------------------------------------

/// 读取并解析 u32 环境变量，失败返回 None
fn env_u32(key: &str) -> Option<u32> {
    std::env::var(key).ok().and_then(|v| v.trim().parse().ok())
}

/// 读取并解析 u64 环境变量，失败返回 None
fn env_u64(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|v| v.trim().parse().ok())
}

/// 读取并解析 usize 环境变量，失败返回 None
fn env_usize(key: &str) -> Option<usize> {
    std::env::var(key).ok().and_then(|v| v.trim().parse().ok())
}

/// 读取并解析 bool 环境变量，失败返回 None
///
/// 支持：true/false、1/0、yes/no、on/off（大小写不敏感）
fn env_bool(key: &str) -> Option<bool> {
    let val = std::env::var(key).ok()?;
    match val.trim().to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 1：S3ServiceConfig 默认值正确
    #[test]
    fn test_s3_config_default() {
        let c = S3ServiceConfig::default();
        assert_eq!(c.lifecycle.hot_to_warm_days, 30);
        assert_eq!(c.lifecycle.warm_to_cold_days, 90);
        assert_eq!(c.lifecycle.cold_to_glacier_days, 180);
        assert_eq!(c.lifecycle.scan_interval_secs, 3600);
        assert!(c.lifecycle.enabled);
        assert_eq!(c.replication.timeout_secs, 300);
        assert_eq!(c.replication.max_retries, 3);
        assert_eq!(c.replication.parallelism, 4);
        assert_eq!(c.inventory.interval_secs, 86400);
        assert_eq!(c.inventory.max_concurrent_jobs, 2);
        // scan_budget 默认无限制
        assert_eq!(c.scan_budget.io.max_objects_per_sec, 0);
        assert_eq!(c.scan_budget.io.max_parallelism, 4);
        assert_eq!(c.scan_budget.capacity.max_bytes_per_scan, 0);
    }

    /// 测试 2：FeatureFlags 默认值正确（RustFS 后端默认 false）
    #[test]
    fn test_feature_flags_default() {
        let f = FeatureFlags::default();
        assert!(!f.rustfs_ecstore_backend, "RustFS ecstore should default to false");
        assert!(!f.rustfs_rio_backend, "RustFS rio should default to false");
        assert!(f.hedged_read_enabled);
        assert!(f.backpressure_enabled);
        assert!(f.replication_gate_enabled);
        assert!(f.delete_all_versions_short_circuit);
    }

    /// 测试 3：环境变量覆盖默认值 + 无效值回退（合并为单测避免并行竞态）
    #[test]
    fn test_config_from_env() {
        // ---- 第一部分：有效值覆盖 ----
        unsafe {
            std::env::set_var("MOX_S3_LIFECYCLE_HOT_TO_WARM_DAYS", "60");
            std::env::set_var("MOX_S3_LIFECYCLE_WARM_TO_COLD_DAYS", "120");
            std::env::set_var("MOX_FEATURE_RUSTFS_ECSTORE", "true");
            std::env::set_var("MOX_FEATURE_HEDGED_READ", "false");
            std::env::set_var("MOX_S3_REPLICATION_PARALLELISM", "8");
            std::env::set_var("MOX_S3_SCAN_MAX_OBJECTS_PER_SCAN", "1000");
        }

        let c = S3ServiceConfig::from_env();
        assert_eq!(c.lifecycle.hot_to_warm_days, 60);
        assert_eq!(c.lifecycle.warm_to_cold_days, 120);
        assert_eq!(c.lifecycle.cold_to_glacier_days, 180);
        assert!(c.features.rustfs_ecstore_backend);
        assert!(!c.features.hedged_read_enabled);
        assert_eq!(c.replication.parallelism, 8);
        assert_eq!(c.scan_budget.capacity.max_objects_per_scan, 1000);

        // 清理
        unsafe {
            std::env::remove_var("MOX_S3_LIFECYCLE_HOT_TO_WARM_DAYS");
            std::env::remove_var("MOX_S3_LIFECYCLE_WARM_TO_COLD_DAYS");
            std::env::remove_var("MOX_FEATURE_RUSTFS_ECSTORE");
            std::env::remove_var("MOX_FEATURE_HEDGED_READ");
            std::env::remove_var("MOX_S3_REPLICATION_PARALLELISM");
            std::env::remove_var("MOX_S3_SCAN_MAX_OBJECTS_PER_SCAN");
        }

        // ---- 第二部分：无效值回退到默认 ----
        unsafe {
            std::env::set_var("MOX_S3_LIFECYCLE_HOT_TO_WARM_DAYS", "not_a_number");
            std::env::set_var("MOX_FEATURE_RUSTFS_ECSTORE", "maybe");
        }

        let c2 = S3ServiceConfig::from_env();
        assert_eq!(c2.lifecycle.hot_to_warm_days, 30);
        assert!(!c2.features.rustfs_ecstore_backend);

        unsafe {
            std::env::remove_var("MOX_S3_LIFECYCLE_HOT_TO_WARM_DAYS");
            std::env::remove_var("MOX_FEATURE_RUSTFS_ECSTORE");
        }
    }

    /// 测试 5：lifecycle_thresholds_ms 转换正确
    #[test]
    fn test_lifecycle_thresholds_ms() {
        let c = S3ServiceConfig::default();
        let (hot, warm, cold) = c.lifecycle_thresholds_ms();
        const DAY_MS: u64 = 24 * 60 * 60 * 1000;
        assert_eq!(hot, 30 * DAY_MS);
        assert_eq!(warm, 90 * DAY_MS);
        assert_eq!(cold, 180 * DAY_MS);
    }

    /// 测试 6：env_bool 解析各种格式
    #[test]
    fn test_env_bool_parsing() {
        unsafe {
            std::env::set_var("TEST_BOOL_TRUE", "true");
            std::env::set_var("TEST_BOOL_1", "1");
            std::env::set_var("TEST_BOOL_YES", "YES");
            std::env::set_var("TEST_BOOL_ON", "On");
            std::env::set_var("TEST_BOOL_FALSE", "false");
            std::env::set_var("TEST_BOOL_0", "0");
            std::env::set_var("TEST_BOOL_NO", "no");
            std::env::set_var("TEST_BOOL_OFF", "off");
            std::env::set_var("TEST_BOOL_INVALID", "maybe");
        }

        assert_eq!(env_bool("TEST_BOOL_TRUE"), Some(true));
        assert_eq!(env_bool("TEST_BOOL_1"), Some(true));
        assert_eq!(env_bool("TEST_BOOL_YES"), Some(true));
        assert_eq!(env_bool("TEST_BOOL_ON"), Some(true));
        assert_eq!(env_bool("TEST_BOOL_FALSE"), Some(false));
        assert_eq!(env_bool("TEST_BOOL_0"), Some(false));
        assert_eq!(env_bool("TEST_BOOL_NO"), Some(false));
        assert_eq!(env_bool("TEST_BOOL_OFF"), Some(false));
        assert_eq!(env_bool("TEST_BOOL_INVALID"), None);
        assert_eq!(env_bool("TEST_BOOL_NOT_EXIST"), None);

        unsafe {
            for k in &[
                "TEST_BOOL_TRUE",
                "TEST_BOOL_1",
                "TEST_BOOL_YES",
                "TEST_BOOL_ON",
                "TEST_BOOL_FALSE",
                "TEST_BOOL_0",
                "TEST_BOOL_NO",
                "TEST_BOOL_OFF",
                "TEST_BOOL_INVALID",
            ] {
                std::env::remove_var(k);
            }
        }
    }
}
