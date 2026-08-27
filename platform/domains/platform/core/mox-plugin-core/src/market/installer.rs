// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 插件安装器 — 下载/验证/安装/卸载/升级

use super::client::{MarketClient, MarketClientError};
use super::remote_registry::RemotePluginVersion;
use super::version::VersionManager;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;

/// 安装结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub plugin_id: String,
    pub version: String,
    pub success: bool,
    pub installed_path: Option<PathBuf>,
    pub message: String,
    pub duration_ms: u64,
}

/// 卸载结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallResult {
    pub plugin_id: String,
    pub success: bool,
    pub message: String,
}

/// 安装状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallStatus {
    /// 未安装
    NotInstalled,
    /// 下载中
    Downloading,
    /// 验证中
    Verifying,
    /// 安装中
    Installing,
    /// 已安装
    Installed,
    /// 安装失败
    Failed,
}

/// 插件安装器
pub struct PluginInstaller {
    client: MarketClient,
    plugins_dir: PathBuf,
    /// 临时下载目录
    temp_dir: PathBuf,
    /// 安装状态记录
    install_status: Arc<RwLock<std::collections::HashMap<String, InstallStatus>>>,
    /// 版本管理器
    version_manager: VersionManager,
}

impl PluginInstaller {
    pub fn new(client: MarketClient, plugins_dir: impl Into<PathBuf>) -> Self {
        let plugins_dir = plugins_dir.into();
        let temp_dir = plugins_dir.join(".temp");
        let version_manager = VersionManager::new(plugins_dir.clone());
        Self {
            client,
            plugins_dir,
            temp_dir,
            install_status: Arc::new(RwLock::new(std::collections::HashMap::new())),
            version_manager,
        }
    }

    /// 安装插件（指定版本）
    pub async fn install(&self, plugin_id: &str, version: &str) -> Result<InstallResult, InstallerError> {
        let start = std::time::Instant::now();
        self.set_status(plugin_id, InstallStatus::Downloading);

        // 1. 获取版本信息（含下载URL和SHA-256）
        let version_info = self.client.get(&format!("/plugins/{}/versions/{}", plugin_id, version), None)
            .await
            .map_err(|e| InstallerError::MarketError(e))?;
        let version_info: RemotePluginVersion = serde_json::from_value(version_info)
            .map_err(|e| InstallerError::ParseError(e.to_string()))?;

        // 2. 下载WASM文件
        self.set_status(plugin_id, InstallStatus::Downloading);
        let wasm_bytes = self.client.download(&version_info.download_url).await
            .map_err(|e| InstallerError::DownloadError(e.to_string()))?;

        // 3. 验证SHA-256
        self.set_status(plugin_id, InstallStatus::Verifying);
        let actual_hash = sha256_hex(&wasm_bytes);
        if actual_hash != version_info.sha256 {
            self.set_status(plugin_id, InstallStatus::Failed);
            return Err(InstallerError::VerificationError {
                expected: version_info.sha256,
                actual: actual_hash,
            });
        }

        // 4. 安装到本地目录
        self.set_status(plugin_id, InstallStatus::Installing);
        let plugin_dir = self.plugins_dir.join(plugin_id);
        tokio::fs::create_dir_all(&plugin_dir).await
            .map_err(|e| InstallerError::IoError(e.to_string()))?;

        // 写入WASM文件
        let wasm_path = plugin_dir.join("plugin.wasm");
        tokio::fs::write(&wasm_path, &wasm_bytes).await
            .map_err(|e| InstallerError::IoError(e.to_string()))?;

        // 生成manifest.json（从版本信息构建）
        let manifest = build_manifest(plugin_id, &version_info);
        let manifest_path = plugin_dir.join("manifest.json");
        tokio::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap())
            .await
            .map_err(|e| InstallerError::IoError(e.to_string()))?;

        // 5. 记录版本信息
        self.version_manager.record_installed(plugin_id, version).await;

        self.set_status(plugin_id, InstallStatus::Installed);
        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(InstallResult {
            plugin_id: plugin_id.into(),
            version: version.into(),
            success: true,
            installed_path: Some(plugin_dir),
            message: format!("plugin {}@{} installed successfully", plugin_id, version),
            duration_ms,
        })
    }

    /// 安装最新版本
    pub async fn install_latest(&self, plugin_id: &str, include_pre_release: bool) -> Result<InstallResult, InstallerError> {
        let version_info = self.client.get(&format!("/plugins/{}/latest", plugin_id), None)
            .await
            .map_err(|e| InstallerError::MarketError(e))?;
        let version: RemotePluginVersion = serde_json::from_value(version_info)
            .map_err(|e| InstallerError::ParseError(e.to_string()))?;
        if !include_pre_release && version.pre_release {
            return Err(InstallerError::PreReleaseNotAllowed(version.version));
        }
        self.install(plugin_id, &version.version).await
    }

    /// 卸载插件
    pub async fn uninstall(&self, plugin_id: &str) -> Result<UninstallResult, InstallerError> {
        let plugin_dir = self.plugins_dir.join(plugin_id);
        if !plugin_dir.exists() {
            return Ok(UninstallResult {
                plugin_id: plugin_id.into(),
                success: false,
                message: format!("plugin {} not installed", plugin_id),
            });
        }

        // 先备份到.old目录（支持回滚）
        let backup_dir = self.plugins_dir.join(".backups").join(format!("{}_removed", plugin_id));
        tokio::fs::create_dir_all(&backup_dir.parent().unwrap()).await.ok();
        let _ = tokio::fs::rename(&plugin_dir, &backup_dir).await;

        // 记录版本
        self.version_manager.record_uninstalled(plugin_id).await;
        self.install_status.write().remove(plugin_id);

        Ok(UninstallResult {
            plugin_id: plugin_id.into(),
            success: true,
            message: format!("plugin {} uninstalled", plugin_id),
        })
    }

    /// 升级插件到最新版本
    pub async fn upgrade(&self, plugin_id: &str, include_pre_release: bool) -> Result<InstallResult, InstallerError> {
        // 检查当前版本
        let current_version = self.version_manager.get_installed_version(plugin_id).await;
        if current_version.is_none() {
            return Err(InstallerError::NotInstalled(plugin_id.into()));
        }

        // 获取最新版本
        let latest = self.client.get(&format!("/plugins/{}/latest", plugin_id), None)
            .await
            .map_err(|e| InstallerError::MarketError(e))?;
        let latest_version: RemotePluginVersion = serde_json::from_value(latest)
            .map_err(|e| InstallerError::ParseError(e.to_string()))?;

        if !include_pre_release && latest_version.pre_release {
            return Err(InstallerError::PreReleaseNotAllowed(latest_version.version));
        }

        // 版本比较
        let current = current_version.unwrap();
        if !is_version_greater(&latest_version.version, &current) {
            return Ok(InstallResult {
                plugin_id: plugin_id.into(),
                version: current.clone(),
                success: true,
                installed_path: Some(self.plugins_dir.join(plugin_id)),
                message: format!("plugin {} already at latest version {}", plugin_id, current),
                duration_ms: 0,
            });
        }

        // 备份当前版本
        self.version_manager.backup_version(plugin_id, &current).await
            .map_err(|e| InstallerError::Other(e))?;

        // 安装新版本
        self.install(plugin_id, &latest_version.version).await
    }

    /// 检查插件是否已安装
    pub async fn is_installed(&self, plugin_id: &str) -> bool {
        self.plugins_dir.join(plugin_id).join("manifest.json").exists()
    }

    /// 获取安装状态
    pub fn get_status(&self, plugin_id: &str) -> InstallStatus {
        self.install_status.read()
            .get(plugin_id)
            .copied()
            .unwrap_or(if self.plugins_dir.join(plugin_id).exists() { InstallStatus::Installed } else { InstallStatus::NotInstalled })
    }

    fn set_status(&self, plugin_id: &str, status: InstallStatus) {
        self.install_status.write().insert(plugin_id.into(), status);
    }

    /// 获取已安装插件列表
    pub async fn list_installed(&self) -> Result<Vec<InstalledPluginInfo>, InstallerError> {
        let mut result = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.plugins_dir).await
            .map_err(|e| InstallerError::IoError(e.to_string()))?;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() && !path.file_name().unwrap().to_string_lossy().starts_with('.') {
                let manifest_path = path.join("manifest.json");
                if manifest_path.exists() {
                    if let Ok(content) = tokio::fs::read_to_string(&manifest_path).await {
                        if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content) {
                            result.push(InstalledPluginInfo {
                                id: manifest.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                name: manifest.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                version: manifest.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                path,
                            });
                        }
                    }
                }
            }
        }
        Ok(result)
    }
}

/// 已安装插件信息
#[derive(Debug, Clone)]
pub struct InstalledPluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub path: PathBuf,
}

/// 安装器错误
#[derive(Debug, thiserror::Error)]
pub enum InstallerError {
    #[error("market API error: {0}")]
    MarketError(#[from] MarketClientError),
    #[error("download error: {0}")]
    DownloadError(String),
    #[error("verification error: expected {expected}, got {actual}")]
    VerificationError { expected: String, actual: String },
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("plugin not installed: {0}")]
    NotInstalled(String),
    #[error("pre-release version not allowed: {0}")]
    PreReleaseNotAllowed(String),
    #[error("dependency error: {0}")]
    DependencyError(String),
    #[error("other error: {0}")]
    Other(String),
}

/// 计算SHA-256哈希（十六进制）
fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// 从远程版本信息构建本地manifest
fn build_manifest(plugin_id: &str, version: &RemotePluginVersion) -> serde_json::Value {
    serde_json::json!({
        "id": plugin_id,
        "name": plugin_id,
        "version": version.version,
        "author": "market",
        "description": "",
        "entry": "plugin.wasm",
        "permissions": [],
        "dependencies": version.dependencies.iter().map(|d| serde_json::json!({
            "id": d.plugin_id,
            "version": d.version_constraint,
            "optional": d.optional,
        })).collect::<Vec<_>>(),
        "config_schema": [],
        "capabilities": [],
        "tags": [],
        "min_platform_version": version.min_platform_version,
    })
}

/// 比较版本号（简化语义化版本）
fn is_version_greater(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<u32> = v.split('.')
            .filter_map(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok())
            .collect();
        (parts.first().copied().unwrap_or(0), parts.get(1).copied().unwrap_or(0), parts.get(2).copied().unwrap_or(0))
    };
    parse(a) > parse(b)
}
