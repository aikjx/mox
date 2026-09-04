// =============================================================================
// MOX 统一配置中心（mox-config-core）
// =============================================================================
//
// 企业级配置管理基础设施，提供：
//
// 1. **配置模型**（config）— 分层配置：默认值 → 环境变量 → 配置文件 → 远程配置
// 2. **配置源**（source）— 多源加载：环境变量 / JSON文件 / 远程HTTP / 内存
// 3. **热更新**（watcher）— 文件变更监听 + 远程轮询 + 回调通知
// 4. **配置验证**（validator）— Schema 验证 + 业务规则校验
// 5. **环境隔离**（environment）— dev/staging/prod 多环境配置
//
// 设计原则：
// - 单一数据源：所有配置通过 ConfigManager 统一访问
// - 类型安全：配置项有明确的类型定义
// - 可观测：配置变更有审计日志
// - 容错：配置加载失败时有合理的降级策略
// =============================================================================

pub mod config;
pub mod source;
pub mod watcher;
pub mod validator;
pub mod environment;

// ── 重导出 ────────────────────────────────────────────────────────────────

pub use config::{Config, ConfigValue, ConfigKey, ConfigManager, ConfigSnapshot};
pub use source::{ConfigSource, EnvironmentSource, FileSource, MemorySource, RemoteSource};
pub use watcher::{ConfigWatcher, WatchEvent, WatchCallback};
pub use validator::{ConfigValidator, ValidationResult, ValidationError};
pub use environment::{Environment, EnvironmentConfig};

// ── Crate 元数据 ──────────────────────────────────────────────────────────

pub const CRATE_ID: &str = "mox-config-core";
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

use serde::{Deserialize, Serialize};

/// 配置中心错误
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("配置键不存在: {0}")]
    KeyNotFound(String),
    #[error("配置类型不匹配: 期望 {expected}, 实际 {actual}")]
    TypeMismatch { expected: String, actual: String },
    #[error("配置验证失败: {0}")]
    ValidationFailed(String),
    #[error("配置源加载失败: {0}")]
    SourceLoadFailed(String),
    #[error("配置文件解析失败: {0}")]
    ParseError(String),
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON错误: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// 配置中心结果类型
pub type ConfigResult<T> = Result<T, ConfigError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_error_display() {
        let err = ConfigError::KeyNotFound("test.key".to_string());
        assert!(format!("{}", err).contains("test.key"));
    }
}
