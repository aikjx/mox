// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! MOX Platform Paths — 统一路径管理，确保架构与数据严格分离。
//!
//! # 核心原则
//! - `platform/` 目录是纯架构代码的只读边界，禁止存放运行时数据
//! - 所有数据/插件/第三方/运行时状态必须放在 `platform/` 之外
//! - 代码中禁止硬编码 `./data/`、`./config/`、`./plugins/` 等相对路径
//! - 所有路径通过环境变量覆盖，默认值遵循项目根目录布局
//!
//! # 目录布局
//! ```text
//! infotopograph/
//! ├── platform/          # 🔒 纯架构代码（Git 追踪）
//! ├── config/            # 📁 配置文件（Git 追踪）
//! ├── data/              # 💾 运行时数据（.gitignore）
//! ├── plugins/           # 🔌 第三方插件（.gitignore）
//! ├── third_party/       # 📦 第三方源码/模型（.gitignore 或 submodule）
//! ├── shared/            # 🔗 跨语言共享（Git 追踪）
//! └── .runtime/          # ⚡ 运行时状态（.gitignore）
//! ```

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathError {
    #[error("path not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("invalid path: {0}")]
    InvalidPath(String),
}

pub type PathResult<T> = Result<T, PathError>;

/// 项目根目录（通过 `MOX_ROOT` 环境变量或自动检测）
#[derive(Debug, Clone)]
pub struct ProjectRoot {
    root: PathBuf,
}

impl ProjectRoot {
    /// 自动检测项目根目录（向上查找包含 `platform/` 和 `Cargo.toml` 的目录）
    pub fn detect() -> Self {
        if let Ok(root) = std::env::var("MOX_ROOT") {
            return Self { root: PathBuf::from(root) };
        }
        let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        for _ in 0..15 {
            if dir.join("platform").is_dir() && dir.join("Cargo.toml").is_file() {
                return Self { root: dir };
            }
            if !dir.pop() {
                break;
            }
        }
        Self { root: PathBuf::from(".") }
    }

    /// 从指定路径创建
    pub fn from_path(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    // ═══════════════════════════════════════════════════════════════
    // 架构代码路径（只读，Git 追踪）
    // ═══════════════════════════════════════════════════════════════

    /// 架构代码根目录 `platform/`
    pub fn platform_dir(&self) -> PathBuf {
        self.root.join("platform")
    }

    /// 配置文件目录 `config/`
    pub fn config_dir(&self) -> PathBuf {
        std::env::var("MOX_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.root.join("config"))
    }

    /// 跨语言共享目录 `shared/`
    pub fn shared_dir(&self) -> PathBuf {
        self.root.join("shared")
    }

    /// 文档目录 `docs/`
    pub fn docs_dir(&self) -> PathBuf {
        self.root.join("docs")
    }

    /// 前端目录 `frontend-ui/`
    pub fn frontend_dir(&self) -> PathBuf {
        self.root.join("frontend-ui")
    }

    // ═══════════════════════════════════════════════════════════════
    // 运行时数据路径（运行时生成，.gitignore）
    // ═══════════════════════════════════════════════════════════════

    /// 数据根目录 `data/`（可通过 `MOX_DATA_DIR` 覆盖）
    pub fn data_dir(&self) -> PathBuf {
        std::env::var("MOX_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.root.join("data"))
    }

    /// 持久化存储目录 `data/storage/`（SQLite/LevelDB 等）
    pub fn storage_dir(&self) -> PathBuf {
        self.data_dir().join("storage")
    }

    /// 缓存目录 `data/cache/`
    pub fn cache_dir(&self) -> PathBuf {
        self.data_dir().join("cache")
    }

    /// 日志目录 `data/logs/`
    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir().join("logs")
    }

    /// 用户上传目录 `data/uploads/`
    pub fn uploads_dir(&self) -> PathBuf {
        self.data_dir().join("uploads")
    }

    /// 导出文件目录 `data/exports/`
    pub fn exports_dir(&self) -> PathBuf {
        self.data_dir().join("exports")
    }

    /// 获取指定域的存储子目录 `data/storage/{domain}/`
    pub fn domain_storage_dir(&self, domain: &str) -> PathBuf {
        self.storage_dir().join(domain)
    }

    /// 获取指定域的 SQLite 数据库文件路径
    pub fn domain_db_path(&self, domain: &str) -> PathBuf {
        self.domain_storage_dir(domain).join(format!("{domain}.db"))
    }

    // ═══════════════════════════════════════════════════════════════
    // 插件路径（按需加载，.gitignore）
    // ═══════════════════════════════════════════════════════════════

    /// 插件根目录 `plugins/`（可通过 `MOX_PLUGINS_DIR` 覆盖）
    pub fn plugins_dir(&self) -> PathBuf {
        std::env::var("MOX_PLUGINS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.root.join("plugins"))
    }

    /// WASM 插件目录 `plugins/wasm/`
    pub fn wasm_plugins_dir(&self) -> PathBuf {
        self.plugins_dir().join("wasm")
    }

    /// 脚本插件目录 `plugins/scripts/`
    pub fn script_plugins_dir(&self) -> PathBuf {
        self.plugins_dir().join("scripts")
    }

    /// 扩展包目录 `plugins/extensions/`
    pub fn extensions_dir(&self) -> PathBuf {
        self.plugins_dir().join("extensions")
    }

    // ═══════════════════════════════════════════════════════════════
    // 第三方路径（.gitignore 或 git submodule）
    // ═══════════════════════════════════════════════════════════════

    /// 第三方根目录 `third_party/`（可通过 `MOX_THIRD_PARTY_DIR` 覆盖）
    pub fn third_party_dir(&self) -> PathBuf {
        std::env::var("MOX_THIRD_PARTY_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.root.join("third_party"))
    }

    /// AI 模型权重目录 `third_party/models/`
    pub fn models_dir(&self) -> PathBuf {
        self.third_party_dir().join("models")
    }

    // ═══════════════════════════════════════════════════════════════
    // 运行时状态路径（.gitignore）
    // ═══════════════════════════════════════════════════════════════

    /// 运行时状态目录 `.runtime/`（可通过 `MOX_RUNTIME_DIR` 覆盖）
    pub fn runtime_dir(&self) -> PathBuf {
        std::env::var("MOX_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.root.join(".runtime"))
    }

    /// PID 文件路径
    pub fn pid_file(&self, service: &str) -> PathBuf {
        self.runtime_dir().join(format!("{service}.pid"))
    }

    /// Socket 文件路径
    pub fn socket_file(&self, service: &str) -> PathBuf {
        self.runtime_dir().join(format!("{service}.sock"))
    }

    /// Lock 文件路径
    pub fn lock_file(&self, resource: &str) -> PathBuf {
        self.runtime_dir().join(format!("{resource}.lock"))
    }

    // ═══════════════════════════════════════════════════════════════
    // 目录确保方法（启动时调用）
    // ═══════════════════════════════════════════════════════════════

    /// 确保所有数据目录存在
    pub fn ensure_data_dirs(&self) -> PathResult<()> {
        for dir in &[
            self.storage_dir(),
            self.cache_dir(),
            self.logs_dir(),
            self.uploads_dir(),
            self.exports_dir(),
        ] {
            std::fs::create_dir_all(dir).map_err(|e| PathError::Io(e.to_string()))?;
        }
        Ok(())
    }

    /// 确保所有插件目录存在
    pub fn ensure_plugin_dirs(&self) -> PathResult<()> {
        for dir in &[
            self.plugins_dir(),
            self.wasm_plugins_dir(),
            self.script_plugins_dir(),
            self.extensions_dir(),
        ] {
            std::fs::create_dir_all(dir).map_err(|e| PathError::Io(e.to_string()))?;
        }
        Ok(())
    }

    /// 确保运行时目录存在
    pub fn ensure_runtime_dirs(&self) -> PathResult<()> {
        std::fs::create_dir_all(self.runtime_dir())
            .map_err(|e| PathError::Io(e.to_string()))?;
        Ok(())
    }

    /// 确保所有目录存在（启动时一次性调用）
    pub fn ensure_all_dirs(&self) -> PathResult<()> {
        self.ensure_data_dirs()?;
        self.ensure_plugin_dirs()?;
        self.ensure_runtime_dirs()?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════
    // 分离不变量验证
    // ═══════════════════════════════════════════════════════════════

    /// 验证架构与数据分离不变量：platform/ 与 data/ 不重叠
    pub fn verify_separation(&self) -> PathResult<()> {
        let platform = self.platform_dir();
        let data = self.data_dir();
        let plugins = self.plugins_dir();
        let third_party = self.third_party_dir();

        if platform.starts_with(&data) || data.starts_with(&platform) {
            return Err(PathError::InvalidPath(
                "platform/ and data/ must not overlap".into(),
            ));
        }
        if platform.starts_with(&plugins) || plugins.starts_with(&platform) {
            return Err(PathError::InvalidPath(
                "platform/ and plugins/ must not overlap".into(),
            ));
        }
        if platform.starts_with(&third_party) || third_party.starts_with(&platform) {
            return Err(PathError::InvalidPath(
                "platform/ and third_party/ must not overlap".into(),
            ));
        }
        Ok(())
    }
}

/// 路径配置（可从 `config/paths.yaml` 或环境变量加载）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    pub data_dir: Option<String>,
    pub plugins_dir: Option<String>,
    pub third_party_dir: Option<String>,
    pub runtime_dir: Option<String>,
    pub config_dir: Option<String>,
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            data_dir: Some("./data".into()),
            plugins_dir: Some("./plugins".into()),
            third_party_dir: Some("./third_party".into()),
            runtime_dir: Some("./.runtime".into()),
            config_dir: Some("./config".into()),
        }
    }
}

impl PathConfig {
    /// 从环境变量加载（如果设置了对应变量）
    pub fn from_env() -> Self {
        Self {
            data_dir: std::env::var("MOX_DATA_DIR").ok(),
            plugins_dir: std::env::var("MOX_PLUGINS_DIR").ok(),
            third_party_dir: std::env::var("MOX_THIRD_PARTY_DIR").ok(),
            runtime_dir: std::env::var("MOX_RUNTIME_DIR").ok(),
            config_dir: std::env::var("MOX_CONFIG_DIR").ok(),
        }
    }

    /// 应用配置到环境变量（供后续 ProjectRoot::detect() 使用）
    pub fn apply_to_env(&self) {
        if let Some(dir) = &self.data_dir {
            std::env::set_var("MOX_DATA_DIR", dir);
        }
        if let Some(dir) = &self.plugins_dir {
            std::env::set_var("MOX_PLUGINS_DIR", dir);
        }
        if let Some(dir) = &self.third_party_dir {
            std::env::set_var("MOX_THIRD_PARTY_DIR", dir);
        }
        if let Some(dir) = &self.runtime_dir {
            std::env::set_var("MOX_RUNTIME_DIR", dir);
        }
        if let Some(dir) = &self.config_dir {
            std::env::set_var("MOX_CONFIG_DIR", dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_root() {
        let root = ProjectRoot::detect();
        assert!(root.platform_dir().exists(), "platform/ dir should exist");
    }

    #[test]
    fn test_data_paths() {
        let root = ProjectRoot::detect();
        assert!(root.data_dir().ends_with("data"));
        assert!(root.storage_dir().ends_with("data/storage"));
        assert!(root.cache_dir().ends_with("data/cache"));
        assert!(root.logs_dir().ends_with("data/logs"));
    }

    #[test]
    fn test_plugin_paths() {
        let root = ProjectRoot::detect();
        assert!(root.plugins_dir().ends_with("plugins"));
        assert!(root.wasm_plugins_dir().ends_with("plugins/wasm"));
        assert!(root.script_plugins_dir().ends_with("plugins/scripts"));
    }

    #[test]
    fn test_runtime_paths() {
        let root = ProjectRoot::detect();
        assert!(root.runtime_dir().ends_with(".runtime"));
        assert!(root.pid_file("gateway").ends_with(".runtime/gateway.pid"));
        assert!(root.socket_file("api").ends_with(".runtime/api.sock"));
    }

    #[test]
    fn test_domain_storage() {
        let root = ProjectRoot::detect();
        // Use file_name() and components() for cross-platform path comparison
        let storage = root.domain_storage_dir("kg");
        assert_eq!(storage.file_name().unwrap(), "kg");
        assert_eq!(storage.parent().unwrap().file_name().unwrap(), "storage");

        let db = root.domain_db_path("data");
        assert_eq!(db.file_name().unwrap(), "data.db");
        assert_eq!(db.parent().unwrap().file_name().unwrap(), "data");
        assert_eq!(db.parent().unwrap().parent().unwrap().file_name().unwrap(), "storage");
    }

    #[test]
    fn test_separation_invariant() {
        let root = ProjectRoot::detect();
        // 架构路径与数据路径必须不重叠
        let platform = root.platform_dir();
        let data = root.data_dir();
        assert!(!platform.starts_with(&data));
        assert!(!data.starts_with(&platform));
        root.verify_separation().expect("separation should hold");
    }

    #[test]
    fn test_path_config_default() {
        let config = PathConfig::default();
        assert_eq!(config.data_dir.as_deref(), Some("./data"));
        assert_eq!(config.plugins_dir.as_deref(), Some("./plugins"));
        assert_eq!(config.third_party_dir.as_deref(), Some("./third_party"));
        assert_eq!(config.runtime_dir.as_deref(), Some("./.runtime"));
    }

    #[test]
    fn test_config_dir() {
        let root = ProjectRoot::detect();
        assert!(root.config_dir().ends_with("config"));
    }
}
