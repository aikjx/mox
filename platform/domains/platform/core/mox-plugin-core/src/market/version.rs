// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 版本管理 — 语义化版本/依赖解析/升级检查/回滚/版本锁定

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 版本更新信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionUpdateInfo {
    pub plugin_id: String,
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub is_major: bool,
    pub is_minor: bool,
    pub is_patch: bool,
    pub release_notes: String,
    pub breaking_changes: Vec<String>,
}

/// 版本锁定文件（.plugin-versions.json）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionLockFile {
    /// 插件ID -> 锁定版本
    #[serde(default)]
    pub locked: HashMap<String, String>,
    /// 插件ID -> 已安装版本
    #[serde(default)]
    pub installed: HashMap<String, String>,
    /// 插件ID -> 备份版本列表（用于回滚）
    #[serde(default)]
    pub backups: HashMap<String, Vec<String>>,
}

/// 版本管理器
pub struct VersionManager {
    plugins_dir: PathBuf,
    lock_file_path: PathBuf,
}

impl VersionManager {
    pub fn new(plugins_dir: impl Into<PathBuf>) -> Self {
        let plugins_dir = plugins_dir.into();
        let lock_file_path = plugins_dir.join(".plugin-versions.json");
        Self { plugins_dir, lock_file_path }
    }

    /// 加载锁定文件
    async fn load_lock(&self) -> VersionLockFile {
        if self.lock_file_path.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&self.lock_file_path).await {
                if let Ok(lock) = serde_json::from_str(&content) {
                    return lock;
                }
            }
        }
        VersionLockFile::default()
    }

    /// 保存锁定文件
    async fn save_lock(&self, lock: &VersionLockFile) -> Result<(), String> {
        let content = serde_json::to_string_pretty(lock).map_err(|e| e.to_string())?;
        tokio::fs::write(&self.lock_file_path, content).await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 记录已安装版本
    pub async fn record_installed(&self, plugin_id: &str, version: &str) {
        let mut lock = self.load_lock().await;
        lock.installed.insert(plugin_id.into(), version.into());
        let _ = self.save_lock(&lock).await;
    }

    /// 记录已卸载
    pub async fn record_uninstalled(&self, plugin_id: &str) {
        let mut lock = self.load_lock().await;
        lock.installed.remove(plugin_id);
        lock.locked.remove(plugin_id);
        let _ = self.save_lock(&lock).await;
    }

    /// 获取已安装版本
    pub async fn get_installed_version(&self, plugin_id: &str) -> Option<String> {
        let lock = self.load_lock().await;
        lock.installed.get(plugin_id).cloned()
    }

    /// 锁定版本（禁止自动升级）
    pub async fn lock_version(&self, plugin_id: &str, version: &str) -> Result<(), String> {
        let mut lock = self.load_lock().await;
        lock.locked.insert(plugin_id.into(), version.into());
        self.save_lock(&lock).await
    }

    /// 解锁版本
    pub async fn unlock_version(&self, plugin_id: &str) -> Result<(), String> {
        let mut lock = self.load_lock().await;
        lock.locked.remove(plugin_id);
        self.save_lock(&lock).await
    }

    /// 检查是否被锁定
    pub async fn is_locked(&self, plugin_id: &str) -> Option<String> {
        let lock = self.load_lock().await;
        lock.locked.get(plugin_id).cloned()
    }

    /// 备份当前版本（升级前调用）
    pub async fn backup_version(&self, plugin_id: &str, version: &str) -> Result<(), String> {
        let plugin_dir = self.plugins_dir.join(plugin_id);
        if !plugin_dir.exists() {
            return Err(format!("plugin {} not found", plugin_id));
        }
        let backup_dir = self.plugins_dir.join(".backups").join(format!("{}_{}", plugin_id, version));
        if backup_dir.exists() {
            // 已存在备份，跳过
            return Ok(());
        }
        tokio::fs::create_dir_all(&backup_dir).await.map_err(|e| e.to_string())?;
        // 复制文件
        let mut entries = tokio::fs::read_dir(&plugin_dir).await.map_err(|e| e.to_string())?;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let src = entry.path();
            let dst = backup_dir.join(src.file_name().unwrap());
            if src.is_file() {
                tokio::fs::copy(&src, &dst).await.map_err(|e| e.to_string())?;
            }
        }
        // 记录备份
        let mut lock = self.load_lock().await;
        lock.backups.entry(plugin_id.into()).or_default().push(version.into());
        let _ = self.save_lock(&lock).await;
        Ok(())
    }

    /// 回滚到指定版本
    pub async fn rollback(&self, plugin_id: &str, target_version: Option<&str>) -> Result<String, String> {
        let lock = self.load_lock().await;
        let backups = lock.backups.get(plugin_id).cloned().unwrap_or_default();

        if backups.is_empty() {
            return Err(format!("no backups found for plugin {}", plugin_id));
        }

        // 确定目标版本
        let version = match target_version {
            Some(v) => {
                if !backups.contains(&v.to_string()) {
                    return Err(format!("backup version {} not found for plugin {}", v, plugin_id));
                }
                v.to_string()
            }
            None => {
                // 回滚到上一个备份版本
                backups.last().cloned().unwrap()
            }
        };

        // 恢复备份
        let backup_dir = self.plugins_dir.join(".backups").join(format!("{}_{}", plugin_id, version));
        let plugin_dir = self.plugins_dir.join(plugin_id);

        if !backup_dir.exists() {
            return Err(format!("backup directory not found: {:?}", backup_dir));
        }

        // 删除当前版本
        if plugin_dir.exists() {
            tokio::fs::remove_dir_all(&plugin_dir).await.map_err(|e| e.to_string())?;
        }
        tokio::fs::create_dir_all(&plugin_dir).await.map_err(|e| e.to_string())?;

        // 复制备份文件
        let mut entries = tokio::fs::read_dir(&backup_dir).await.map_err(|e| e.to_string())?;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let src = entry.path();
            let dst = plugin_dir.join(src.file_name().unwrap());
            if src.is_file() {
                tokio::fs::copy(&src, &dst).await.map_err(|e| e.to_string())?;
            }
        }

        // 更新记录
        let mut lock = self.load_lock().await;
        lock.installed.insert(plugin_id.into(), version.clone());
        // 移除已使用的备份
        if let Some(list) = lock.backups.get_mut(plugin_id) {
            list.retain(|v| v != &version);
        }
        let _ = self.save_lock(&lock).await;

        Ok(version)
    }

    /// 列出所有备份版本
    pub async fn list_backups(&self, plugin_id: &str) -> Vec<String> {
        let lock = self.load_lock().await;
        lock.backups.get(plugin_id).cloned().unwrap_or_default()
    }

    /// 检查所有已安装插件的更新
    pub async fn check_all_updates(&self, market_client: &crate::market::client::MarketClient) -> Vec<VersionUpdateInfo> {
        let lock = self.load_lock().await;
        let mut updates = Vec::new();

        for (plugin_id, current_version) in &lock.installed {
            // 跳过被锁定的
            if lock.locked.contains_key(plugin_id) {
                continue;
            }
            // 从市场获取最新版本
            if let Ok(body) = market_client.get(&format!("/plugins/{}/latest", plugin_id), None).await {
                if let Ok(latest) = serde_json::from_value::<crate::market::remote_registry::RemotePluginVersion>(body) {
                    let has_update = is_version_greater(&latest.version, current_version);
                    if has_update {
                        let (is_major, is_minor, is_patch) = classify_update(current_version, &latest.version);
                        updates.push(VersionUpdateInfo {
                            plugin_id: plugin_id.clone(),
                            current_version: current_version.clone(),
                            latest_version: latest.version,
                            has_update: true,
                            is_major,
                            is_minor,
                            is_patch,
                            release_notes: latest.release_notes,
                            breaking_changes: Vec::new(),
                        });
                    }
                }
            }
        }
        updates
    }

    /// 解析依赖（检查依赖是否满足）
    pub async fn resolve_dependencies(&self, _plugin_id: &str, dependencies: &[crate::market::remote_registry::RemoteDependency]) -> Result<Vec<DependencyStatus>, String> {
        let lock = self.load_lock().await;
        let mut statuses = Vec::new();

        for dep in dependencies {
            let installed = lock.installed.get(&dep.plugin_id).cloned();
            let satisfied = installed.as_ref()
                .map(|v| version_matches_constraint(v, &dep.version_constraint))
                .unwrap_or(false);

            statuses.push(DependencyStatus {
                plugin_id: dep.plugin_id.clone(),
                version_constraint: dep.version_constraint.clone(),
                installed_version: installed,
                satisfied,
                optional: dep.optional,
            });
        }
        Ok(statuses)
    }
}

/// 依赖状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyStatus {
    pub plugin_id: String,
    pub version_constraint: String,
    pub installed_version: Option<String>,
    pub satisfied: bool,
    pub optional: bool,
}

/// 语义化版本解析
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SemVer {
    pub fn parse(version: &str) -> Option<Self> {
        let parts: Vec<u32> = version.split('.')
            .filter_map(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok())
            .collect();
        if parts.len() < 2 {
            return None;
        }
        Some(Self {
            major: parts.first().copied().unwrap_or(0),
            minor: parts.get(1).copied().unwrap_or(0),
            patch: parts.get(2).copied().unwrap_or(0),
        })
    }

    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// 比较版本号
pub fn is_version_greater(a: &str, b: &str) -> bool {
    match (SemVer::parse(a), SemVer::parse(b)) {
        (Some(va), Some(vb)) => va > vb,
        _ => a > b,
    }
}

/// 分类更新类型
pub fn classify_update(current: &str, latest: &str) -> (bool, bool, bool) {
    match (SemVer::parse(current), SemVer::parse(latest)) {
        (Some(c), Some(l)) => {
            if l.major > c.major { (true, false, false) }
            else if l.minor > c.minor { (false, true, false) }
            else if l.patch > c.patch { (false, false, true) }
            else { (false, false, false) }
        }
        _ => (false, false, false),
    }
}

/// 检查版本是否满足约束（简化版，支持 ^, ~, >=, <=, =, *）
pub fn version_matches_constraint(version: &str, constraint: &str) -> bool {
    let constraint = constraint.trim();
    if constraint == "*" || constraint.is_empty() {
        return true;
    }

    let ver = match SemVer::parse(version) {
        Some(v) => v,
        None => return false,
    };

    if let Some(rest) = constraint.strip_prefix('^') {
        // ^1.2.3: >=1.2.3, <2.0.0
        if let Some(target) = SemVer::parse(rest) {
            return ver >= target && ver.major == target.major;
        }
    }

    if let Some(rest) = constraint.strip_prefix('~') {
        // ~1.2.3: >=1.2.3, <1.3.0
        if let Some(target) = SemVer::parse(rest) {
            return ver >= target && ver.major == target.major && ver.minor == target.minor;
        }
    }

    if let Some(rest) = constraint.strip_prefix(">=") {
        if let Some(target) = SemVer::parse(rest) {
            return ver >= target;
        }
    }

    if let Some(rest) = constraint.strip_prefix("<=") {
        if let Some(target) = SemVer::parse(rest) {
            return ver <= target;
        }
    }

    if let Some(rest) = constraint.strip_prefix('>') {
        if let Some(target) = SemVer::parse(rest) {
            return ver > target;
        }
    }

    if let Some(rest) = constraint.strip_prefix('<') {
        if let Some(target) = SemVer::parse(rest) {
            return ver < target;
        }
    }

    if let Some(rest) = constraint.strip_prefix('=') {
        if let Some(target) = SemVer::parse(rest) {
            return ver == target;
        }
    }

    // 精确匹配
    if let Some(target) = SemVer::parse(constraint) {
        return ver == target;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_parse() {
        assert_eq!(SemVer::parse("1.2.3"), Some(SemVer { major: 1, minor: 2, patch: 3 }));
        assert_eq!(SemVer::parse("2.0"), Some(SemVer { major: 2, minor: 0, patch: 0 }));
        assert_eq!(SemVer::parse("invalid"), None);
    }

    #[test]
    fn test_is_version_greater() {
        assert!(is_version_greater("2.0.0", "1.9.9"));
        assert!(is_version_greater("1.3.0", "1.2.9"));
        assert!(is_version_greater("1.2.4", "1.2.3"));
        assert!(!is_version_greater("1.2.3", "1.2.3"));
        assert!(!is_version_greater("1.2.3", "2.0.0"));
    }

    #[test]
    fn test_classify_update() {
        assert_eq!(classify_update("1.2.3", "2.0.0"), (true, false, false));
        assert_eq!(classify_update("1.2.3", "1.3.0"), (false, true, false));
        assert_eq!(classify_update("1.2.3", "1.2.4"), (false, false, true));
    }

    #[test]
    fn test_version_matches_constraint() {
        assert!(version_matches_constraint("1.2.3", "*"));
        assert!(version_matches_constraint("1.2.3", "^1.0.0"));
        assert!(!version_matches_constraint("2.0.0", "^1.0.0"));
        assert!(version_matches_constraint("1.2.5", "~1.2.0"));
        assert!(!version_matches_constraint("1.3.0", "~1.2.0"));
        assert!(version_matches_constraint("1.5.0", ">=1.0.0"));
        assert!(!version_matches_constraint("0.9.0", ">=1.0.0"));
        assert!(version_matches_constraint("1.2.3", "1.2.3"));
    }
}
