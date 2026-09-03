// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Volume 服务全局配置模块
//!
//! 将原先散落在各模块中的硬编码参数统一提取到配置结构体：
//! - 纠删码默认配置（data/parity shards、min object size、SIMD）
//! - 背压配置（re-export 自 [`crate::backpressure`]）
//! - 写仲裁配置（提取自 `WriteProgressPolicy` 的 stall_timeout / absolute_cap / quorum）
//! - 读仲裁配置（提取自 `HedgedReader` 的 hedge_delay / read_timeout）
//! - 缓冲池配置（预留接入点，buffer_pool 模块由并行子任务实现）
//! - Feature flag 控制可选 RustFS 后端接入点（默认关闭）
//!
//! 环境变量命名约定：`MOX_VOLUME_<SECTION>_<FIELD>`
//!
//! 算法参考：RustFS ecstore / rio 配置集成（Apache 2.0），
//! `ais/RustFS/crates/`，本实现为自研重写。

use crate::backpressure::BackpressureConfig;
use std::time::Duration;

// ---------------------------------------------------------------------------
// 顶层配置
// ---------------------------------------------------------------------------

/// Volume 服务全局配置
#[derive(Debug, Clone)]
pub struct VolumeServiceConfig {
    /// 纠删码默认配置
    pub erasure_coding: ErasureCodingConfig,
    /// 背压配置
    pub backpressure: BackpressureConfig,
    /// 写仲裁配置
    pub write_arbitration: WriteArbitrationConfig,
    /// 读仲裁配置
    pub read_arbitration: ReadArbitrationConfig,
    /// 缓冲池配置
    pub buffer_pool: BufferPoolConfig,
    /// 功能开关
    pub features: VolumeFeatureFlags,
}

// ---------------------------------------------------------------------------
// 纠删码配置
// ---------------------------------------------------------------------------

/// 纠删码默认配置
///
/// 提取自 `profile.rs` 的 `EcProfile::default()`（4+2, min_obj_size=64KB）
/// 和 `gf256_simd` 的 SIMD 自动检测逻辑。
#[derive(Debug, Clone)]
pub struct ErasureCodingConfig {
    /// 默认 data shards（默认 4）
    pub default_data_shards: u16,
    /// 默认 parity shards（默认 2）
    pub default_parity_shards: u16,
    /// 最小对象大小（默认 65536 = 64KB，对齐 `DEFAULT_MIN_OBJ_SIZE`）
    pub min_object_size: usize,
    /// 强制 SIMD 路径（None = 自动检测 AVX2）
    pub force_simd: Option<bool>,
}

// ---------------------------------------------------------------------------
// 写仲裁配置
// ---------------------------------------------------------------------------

/// 写仲裁配置
///
/// 提取自 `multi_writer.rs` 的 `WriteProgressPolicy` 硬编码默认值：
/// - stall_timeout = 30s
/// - absolute_cap = None
/// - write_quorum = data_shards + 1（通过 quorum_ratio 计算）
#[derive(Debug, Clone)]
pub struct WriteArbitrationConfig {
    /// 写入 stall 超时秒数（默认 30s，按块 re-arm）
    pub stall_timeout_secs: u64,
    /// 绝对超时上限秒数（None = 关闭，防 slow-drip peer）
    pub absolute_cap_secs: Option<u64>,
    /// 写仲裁法定数比例（默认 0.5，即 data_shards * 0.5 + 1）
    ///
    /// 实际 quorum = max(1, (data_shards as f32 * quorum_ratio) as usize + 1)
    pub quorum_ratio: f32,
}

impl WriteArbitrationConfig {
    /// 根据 data_shards 计算实际写仲裁法定数
    pub fn quorum_for(&self, data_shards: usize) -> usize {
        ((data_shards as f32 * self.quorum_ratio) as usize).saturating_add(1).max(1)
    }

    /// 转换为 `WriteProgressPolicy` 所需的 Duration
    pub fn stall_timeout(&self) -> Duration {
        Duration::from_secs(self.stall_timeout_secs)
    }

    /// 转换为 `WriteProgressPolicy` 所需的 Option<Duration>
    pub fn absolute_cap(&self) -> Option<Duration> {
        self.absolute_cap_secs.map(Duration::from_secs)
    }
}

// ---------------------------------------------------------------------------
// 读仲裁配置
// ---------------------------------------------------------------------------

/// 读仲裁配置
///
/// 提取自 `hedged_reader.rs` 的 `HedgedReader` 硬编码参数：
/// - hedge_delay 建议取 min(read_timeout, 100ms)
/// - read_timeout 默认 30s
#[derive(Debug, Clone)]
pub struct ReadArbitrationConfig {
    /// hedge 延迟毫秒数（默认 100ms）
    pub hedge_delay_ms: u64,
    /// 读取超时秒数（默认 30s）
    pub read_timeout_secs: u64,
}

impl ReadArbitrationConfig {
    /// hedge 延迟作为 Duration
    pub fn hedge_delay(&self) -> Duration {
        Duration::from_millis(self.hedge_delay_ms)
    }

    /// 读取超时作为 Duration
    pub fn read_timeout(&self) -> Duration {
        Duration::from_secs(self.read_timeout_secs)
    }
}

// ---------------------------------------------------------------------------
// 缓冲池配置
// ---------------------------------------------------------------------------

/// 缓冲池配置（re-export 自 [`crate::buffer_pool::BufferPoolConfig`]）
///
/// 由并行子任务在 `buffer_pool.rs` 中实现，包含 tier 配置和全局字节上限。
/// 此处通过 `pub use` 重新导出，保持 `config` 模块的统一访问入口。
pub use crate::buffer_pool::BufferPoolConfig;

// ---------------------------------------------------------------------------
// Feature Flags
// ---------------------------------------------------------------------------

/// Volume 服务功能开关
///
/// 所有 RustFS 后端接入点默认 `false`，仅定义开关和接入点，
/// 不实际引入 RustFS 依赖或代码。
#[derive(Debug, Clone)]
pub struct VolumeFeatureFlags {
    /// 是否启用 RustFS ecstore 作为可选后端（默认 false）
    pub rustfs_ecstore_backend: bool,
    /// 是否启用 RustFS rio I/O 管线（默认 false）
    pub rustfs_rio_backend: bool,
    /// 是否启用 hedged read（对冲读，默认 true）
    pub hedged_read_enabled: bool,
    /// 是否启用背压（默认 true）
    pub backpressure_enabled: bool,
    /// 是否启用 SIMD 加速（默认 true，运行时自动检测 AVX2）
    pub simd_enabled: bool,
}

// ---------------------------------------------------------------------------
// Default 实现
// ---------------------------------------------------------------------------

impl Default for VolumeServiceConfig {
    fn default() -> Self {
        Self {
            erasure_coding: ErasureCodingConfig {
                default_data_shards: 4,
                default_parity_shards: 2,
                min_object_size: 65536, // DEFAULT_MIN_OBJ_SIZE
                force_simd: None,
            },
            backpressure: BackpressureConfig::default(),
            write_arbitration: WriteArbitrationConfig {
                stall_timeout_secs: 30,
                absolute_cap_secs: None,
                quorum_ratio: 0.5,
            },
            read_arbitration: ReadArbitrationConfig {
                hedge_delay_ms: 100,
                read_timeout_secs: 30,
            },
            buffer_pool: BufferPoolConfig::default(),
            features: VolumeFeatureFlags {
                rustfs_ecstore_backend: false,
                rustfs_rio_backend: false,
                hedged_read_enabled: true,
                backpressure_enabled: true,
                simd_enabled: true,
            },
        }
    }
}

impl Default for ErasureCodingConfig {
    fn default() -> Self {
        VolumeServiceConfig::default().erasure_coding
    }
}

impl Default for WriteArbitrationConfig {
    fn default() -> Self {
        VolumeServiceConfig::default().write_arbitration
    }
}

impl Default for ReadArbitrationConfig {
    fn default() -> Self {
        VolumeServiceConfig::default().read_arbitration
    }
}

impl Default for VolumeFeatureFlags {
    fn default() -> Self {
        VolumeServiceConfig::default().features
    }
}

// ---------------------------------------------------------------------------
// 环境变量加载
// ---------------------------------------------------------------------------

impl VolumeServiceConfig {
    /// 从环境变量加载配置，覆盖默认值
    ///
    /// 解析失败时静默使用默认值（不 panic）。所有环境变量均为可选。
    ///
    /// # 支持的环境变量
    ///
    /// | 环境变量 | 类型 | 对应字段 |
    /// |---|---|---|
    /// | `MOX_VOLUME_EC_DEFAULT_DATA_SHARDS` | u16 | erasure_coding.default_data_shards |
    /// | `MOX_VOLUME_EC_DEFAULT_PARITY_SHARDS` | u16 | erasure_coding.default_parity_shards |
    /// | `MOX_VOLUME_EC_MIN_OBJECT_SIZE` | usize | erasure_coding.min_object_size |
    /// | `MOX_VOLUME_EC_FORCE_SIMD` | bool | erasure_coding.force_simd |
    /// | `MOX_VOLUME_WRITE_STALL_TIMEOUT_SECS` | u64 | write_arbitration.stall_timeout_secs |
    /// | `MOX_VOLUME_WRITE_ABSOLUTE_CAP_SECS` | u64 | write_arbitration.absolute_cap_secs |
    /// | `MOX_VOLUME_WRITE_QUORUM_RATIO` | f32 | write_arbitration.quorum_ratio |
    /// | `MOX_VOLUME_READ_HEDGE_DELAY_MS` | u64 | read_arbitration.hedge_delay_ms |
    /// | `MOX_VOLUME_READ_TIMEOUT_SECS` | u64 | read_arbitration.read_timeout_secs |
    /// | `MOX_FEATURE_RUSTFS_ECSTORE` | bool | features.rustfs_ecstore_backend |
    /// | `MOX_FEATURE_RUSTFS_RIO` | bool | features.rustfs_rio_backend |
    /// | `MOX_FEATURE_HEDGED_READ` | bool | features.hedged_read_enabled |
    /// | `MOX_FEATURE_BACKPRESSURE` | bool | features.backpressure_enabled |
    /// | `MOX_FEATURE_SIMD` | bool | features.simd_enabled |
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // ---- 纠删码 ----
        if let Some(v) = env_u16("MOX_VOLUME_EC_DEFAULT_DATA_SHARDS") {
            config.erasure_coding.default_data_shards = v;
        }
        if let Some(v) = env_u16("MOX_VOLUME_EC_DEFAULT_PARITY_SHARDS") {
            config.erasure_coding.default_parity_shards = v;
        }
        if let Some(v) = env_usize("MOX_VOLUME_EC_MIN_OBJECT_SIZE") {
            config.erasure_coding.min_object_size = v;
        }
        if let Some(v) = env_bool("MOX_VOLUME_EC_FORCE_SIMD") {
            config.erasure_coding.force_simd = Some(v);
        }

        // ---- 写仲裁 ----
        if let Some(v) = env_u64("MOX_VOLUME_WRITE_STALL_TIMEOUT_SECS") {
            config.write_arbitration.stall_timeout_secs = v;
        }
        if let Some(v) = env_u64("MOX_VOLUME_WRITE_ABSOLUTE_CAP_SECS") {
            config.write_arbitration.absolute_cap_secs = Some(v);
        }
        if let Some(v) = env_f32("MOX_VOLUME_WRITE_QUORUM_RATIO") {
            config.write_arbitration.quorum_ratio = v;
        }

        // ---- 读仲裁 ----
        if let Some(v) = env_u64("MOX_VOLUME_READ_HEDGE_DELAY_MS") {
            config.read_arbitration.hedge_delay_ms = v;
        }
        if let Some(v) = env_u64("MOX_VOLUME_READ_TIMEOUT_SECS") {
            config.read_arbitration.read_timeout_secs = v;
        }

        // ---- 缓冲池 ----
        // 注：BufferPoolConfig 包含 tier Vec 结构，环境变量覆盖较复杂，
        // 当前使用默认值；如需自定义请通过代码构造 VolumeServiceConfig。

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
        if let Some(v) = env_bool("MOX_FEATURE_SIMD") {
            config.features.simd_enabled = v;
        }

        config
    }
}

// ---------------------------------------------------------------------------
// 环境变量解析辅助函数
// ---------------------------------------------------------------------------

fn env_u16(key: &str) -> Option<u16> {
    std::env::var(key).ok().and_then(|v| v.trim().parse().ok())
}

fn env_u64(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|v| v.trim().parse().ok())
}

fn env_usize(key: &str) -> Option<usize> {
    std::env::var(key).ok().and_then(|v| v.trim().parse().ok())
}

fn env_f32(key: &str) -> Option<f32> {
    std::env::var(key).ok().and_then(|v| v.trim().parse().ok())
}

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

    /// 测试 1：VolumeServiceConfig 默认值正确
    #[test]
    fn test_volume_config_default() {
        let c = VolumeServiceConfig::default();
        // EC
        assert_eq!(c.erasure_coding.default_data_shards, 4);
        assert_eq!(c.erasure_coding.default_parity_shards, 2);
        assert_eq!(c.erasure_coding.min_object_size, 65536);
        assert!(c.erasure_coding.force_simd.is_none());
        // 背压（re-export 默认值）
        assert_eq!(c.backpressure.max_concurrent, 32);
        assert_eq!(c.backpressure.high_water, 0.8);
        // 写仲裁
        assert_eq!(c.write_arbitration.stall_timeout_secs, 30);
        assert!(c.write_arbitration.absolute_cap_secs.is_none());
        assert_eq!(c.write_arbitration.quorum_ratio, 0.5);
        // 读仲裁
        assert_eq!(c.read_arbitration.hedge_delay_ms, 100);
        assert_eq!(c.read_arbitration.read_timeout_secs, 30);
        // 缓冲池（使用 buffer_pool 模块的真实结构）
        assert!(!c.buffer_pool.tiers.is_empty());
        assert!(c.buffer_pool.global_max_bytes >= 0);
    }

    /// 测试 2：VolumeFeatureFlags 默认值正确（RustFS 后端默认 false）
    #[test]
    fn test_volume_feature_flags_default() {
        let f = VolumeFeatureFlags::default();
        assert!(!f.rustfs_ecstore_backend, "RustFS ecstore should default to false");
        assert!(!f.rustfs_rio_backend, "RustFS rio should default to false");
        assert!(f.hedged_read_enabled);
        assert!(f.backpressure_enabled);
        assert!(f.simd_enabled);
    }

    /// 测试 3：环境变量覆盖默认值 + 无效值回退（合并为单测避免并行竞态）
    #[test]
    fn test_volume_config_from_env() {
        // ---- 第一部分：有效值覆盖 ----
        unsafe {
            std::env::set_var("MOX_VOLUME_EC_DEFAULT_DATA_SHARDS", "8");
            std::env::set_var("MOX_VOLUME_EC_DEFAULT_PARITY_SHARDS", "4");
            std::env::set_var("MOX_VOLUME_READ_HEDGE_DELAY_MS", "200");
            std::env::set_var("MOX_VOLUME_WRITE_STALL_TIMEOUT_SECS", "60");
            std::env::set_var("MOX_FEATURE_RUSTFS_ECSTORE", "true");
            std::env::set_var("MOX_FEATURE_SIMD", "false");
        }

        let c = VolumeServiceConfig::from_env();
        assert_eq!(c.erasure_coding.default_data_shards, 8);
        assert_eq!(c.erasure_coding.default_parity_shards, 4);
        assert_eq!(c.read_arbitration.hedge_delay_ms, 200);
        assert_eq!(c.write_arbitration.stall_timeout_secs, 60);
        assert!(c.features.rustfs_ecstore_backend);
        assert!(!c.features.simd_enabled);
        assert_eq!(c.erasure_coding.min_object_size, 65536);
        assert_eq!(c.read_arbitration.read_timeout_secs, 30);

        unsafe {
            std::env::remove_var("MOX_VOLUME_EC_DEFAULT_DATA_SHARDS");
            std::env::remove_var("MOX_VOLUME_EC_DEFAULT_PARITY_SHARDS");
            std::env::remove_var("MOX_VOLUME_READ_HEDGE_DELAY_MS");
            std::env::remove_var("MOX_VOLUME_WRITE_STALL_TIMEOUT_SECS");
            std::env::remove_var("MOX_FEATURE_RUSTFS_ECSTORE");
            std::env::remove_var("MOX_FEATURE_SIMD");
        }

        // ---- 第二部分：无效值回退到默认 ----
        unsafe {
            std::env::set_var("MOX_VOLUME_EC_DEFAULT_DATA_SHARDS", "abc");
            std::env::set_var("MOX_VOLUME_WRITE_QUORUM_RATIO", "not_a_float");
        }
        let c2 = VolumeServiceConfig::from_env();
        assert_eq!(c2.erasure_coding.default_data_shards, 4);
        assert_eq!(c2.write_arbitration.quorum_ratio, 0.5);

        unsafe {
            std::env::remove_var("MOX_VOLUME_EC_DEFAULT_DATA_SHARDS");
            std::env::remove_var("MOX_VOLUME_WRITE_QUORUM_RATIO");
        }
    }

    /// 测试 4：WriteArbitrationConfig::quorum_for 计算正确
    #[test]
    fn test_write_quorum_for() {
        let cfg = WriteArbitrationConfig::default(); // quorum_ratio = 0.5
        // data_shards=4: 4*0.5=2 → +1 = 3
        assert_eq!(cfg.quorum_for(4), 3);
        // data_shards=2: 2*0.5=1 → +1 = 2
        assert_eq!(cfg.quorum_for(2), 2);
        // data_shards=1: 1*0.5=0 → +1 = 1
        assert_eq!(cfg.quorum_for(1), 1);
        // data_shards=0: 0 → +1 = 1 (max(1))
        assert_eq!(cfg.quorum_for(0), 1);

        // 自定义 ratio = 1.0（全写）
        let cfg_full = WriteArbitrationConfig {
            quorum_ratio: 1.0,
            ..Default::default()
        };
        assert_eq!(cfg_full.quorum_for(4), 5);
    }

    /// 测试 5：ReadArbitrationConfig Duration 转换正确
    #[test]
    fn test_read_arbitration_duration_conversion() {
        let cfg = ReadArbitrationConfig::default();
        assert_eq!(cfg.hedge_delay(), Duration::from_millis(100));
        assert_eq!(cfg.read_timeout(), Duration::from_secs(30));
    }

    /// 测试 7：WriteArbitrationConfig Duration 转换正确
    #[test]
    fn test_write_arbitration_duration_conversion() {
        let cfg = WriteArbitrationConfig::default();
        assert_eq!(cfg.stall_timeout(), Duration::from_secs(30));
        assert_eq!(cfg.absolute_cap(), None);

        let cfg_with_cap = WriteArbitrationConfig {
            absolute_cap_secs: Some(120),
            ..Default::default()
        };
        assert_eq!(cfg_with_cap.absolute_cap(), Some(Duration::from_secs(120)));
    }
}
