// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 配置引擎错误类型

use mox_alliance_common_proto::ConfigType;
use thiserror::Error;

/// 配置引擎错误
#[derive(Debug, Error)]
pub enum ConfigError {
    /// 模块不存在
    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    /// 全局配置不存在
    #[error("Global {0} config not found")]
    GlobalConfigNotFound(String),

    /// 配置类型不匹配
    #[error("Config type mismatch: expected {expected:?}, got {got:?}")]
    ConfigTypeMismatch {
        expected: ConfigType,
        got: ConfigType,
    },

    /// 版本不存在
    #[error("Version {version} not found for module {module_id}")]
    VersionNotFound { module_id: String, version: u32 },

    /// 配置已存在
    #[error("Module config already exists: {0}")]
    ModuleAlreadyExists(String),

    /// 配置验证失败
    #[error("Config validation failed: {0}")]
    ValidationFailed(String),

    /// Provider 不存在
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    /// 序列化/反序列化错误
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// 存储错误
    #[error("Storage error: {0}")]
    StorageError(String),

    /// 并发冲突
    #[error("Concurrent modification conflict for module {0}")]
    ConcurrentConflict(String),

    /// 内部错误
    #[error("Internal config error: {0}")]
    Internal(String),
}

/// 配置引擎结果类型
pub type ConfigResult<T> = Result<T, ConfigError>;
