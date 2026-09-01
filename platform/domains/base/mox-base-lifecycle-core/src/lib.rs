//! MOX 统一基座 · 生命周期层
//!
//! 定义数据生命周期的统一契约：
//! - **版本管理**（`VersionManager`）：节点 / Blob 版本控制与回滚
//! - **去重**（`Deduplicator`）：内容寻址（SHA-256）去重
//! - **垃圾回收**（`GarbageCollector`）：孤儿对象回收
//! - **回收站**（`RecycleBin`）：软删除与恢复
//!
//! ## 设计原则
//! - 只定义 trait 契约，不内置后端。
//! - cloud 域 mox-cloud-rebalance-svc 挂接本层（去重/GC/再平衡）。

use async_trait::async_trait;
use mox_base_store_core::StoreError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 生命周期错误
#[derive(Debug, Error)]
pub enum LifecycleError {
    #[error("版本不存在: {id} @ v{version}")]
    VersionNotFound { id: String, version: u64 },
    #[error("对象不存在: {0}")]
    NotFound(String),
    #[error("去重冲突: {0}")]
    Dedup(String),
    #[error("其他错误: {0}")]
    Other(String),
}

/// 生命周期结果
pub type LifecycleResult<T> = Result<T, LifecycleError>;

/// 版本快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionSnapshot {
    /// 实体 ID
    pub id: String,
    /// 版本号
    pub version: u64,
    /// 创建时间（epoch ms）
    pub created_at_ms: u64,
    /// 版本备注
    pub note: String,
}

/// 版本管理器 trait
#[async_trait]
pub trait VersionManager: Send + Sync {
    /// 保存新版本
    async fn save_version(&self, id: &str, note: &str) -> LifecycleResult<VersionSnapshot>;

    /// 列出版本历史（降序）
    async fn list_versions(&self, id: &str) -> LifecycleResult<Vec<VersionSnapshot>>;

    /// 回滚到指定版本
    async fn rollback(&self, id: &str, version: u64) -> LifecycleResult<()>;

    /// 当前版本号
    async fn current_version(&self, id: &str) -> LifecycleResult<u64>;
}

/// 去重器 trait
#[async_trait]
pub trait Deduplicator: Send + Sync {
    /// 内容寻址去重：若已存在相同哈希，返回已存在对象的路径；否则 None
    async fn dedup(&self, sha256: &str) -> LifecycleResult<Option<String>>;

    /// 注册一个对象的哈希（写入去重索引）
    async fn register(&self, sha256: &str, path: &str) -> LifecycleResult<()>;
}

/// 垃圾回收器 trait
#[async_trait]
pub trait GarbageCollector: Send + Sync {
    /// 扫描并回收未被引用的孤儿对象，返回回收数量
    async fn collect_garbage(&self) -> LifecycleResult<u64>;

    /// 统计当前孤儿对象数量
    async fn orphan_count(&self) -> LifecycleResult<u64>;
}

/// 回收站 trait（软删除 / 恢复）
#[async_trait]
pub trait RecycleBin: Send + Sync {
    /// 软删除（进入回收站）
    async fn soft_delete(&self, path: &str) -> LifecycleResult<()>;

    /// 从回收站恢复
    async fn restore(&self, path: &str) -> LifecycleResult<()>;

    /// 永久删除（清出回收站）
    async fn purge(&self, path: &str) -> LifecycleResult<()>;

    /// 列出回收站内容
    async fn list_trash(&self) -> LifecycleResult<Vec<String>>;
}

/// 内存版版本管理器（参考实现 / 测试用）
pub struct InMemoryVersionManager {
    versions: std::sync::Mutex<std::collections::HashMap<String, Vec<VersionSnapshot>>>,
    current: std::sync::Mutex<std::collections::HashMap<String, u64>>,
}

impl Default for InMemoryVersionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryVersionManager {
    /// 新建内存版本管理器
    pub fn new() -> Self {
        Self {
            versions: std::sync::Mutex::new(std::collections::HashMap::new()),
            current: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl VersionManager for InMemoryVersionManager {
    async fn save_version(&self, id: &str, note: &str) -> LifecycleResult<VersionSnapshot> {
        let mut cur = self
            .current
            .lock()
            .map_err(|e| LifecycleError::Other(e.to_string()))?;
        let v = cur.entry(id.to_string()).or_insert(0);
        *v += 1;
        let snap = VersionSnapshot {
            id: id.to_string(),
            version: *v,
            created_at_ms: now_ms(),
            note: note.to_string(),
        };
        self.versions
            .lock()
            .map_err(|e| LifecycleError::Other(e.to_string()))?
            .entry(id.to_string())
            .or_default()
            .push(snap.clone());
        Ok(snap)
    }

    async fn list_versions(&self, id: &str) -> LifecycleResult<Vec<VersionSnapshot>> {
        let versions = self
            .versions
            .lock()
            .map_err(|e| LifecycleError::Other(e.to_string()))?;
        let mut list = versions.get(id).cloned().unwrap_or_default();
        list.sort_by_key(|s| std::cmp::Reverse(s.version));
        Ok(list)
    }

    async fn rollback(&self, id: &str, version: u64) -> LifecycleResult<()> {
        let versions = self
            .versions
            .lock()
            .map_err(|e| LifecycleError::Other(e.to_string()))?;
        let exists = versions
            .get(id)
            .map(|v| v.iter().any(|s| s.version == version))
            .unwrap_or(false);
        if !exists {
            return Err(LifecycleError::VersionNotFound {
                id: id.to_string(),
                version,
            });
        }
        let mut cur = self
            .current
            .lock()
            .map_err(|e| LifecycleError::Other(e.to_string()))?;
        cur.insert(id.to_string(), version);
        Ok(())
    }

    async fn current_version(&self, id: &str) -> LifecycleResult<u64> {
        Ok(self
            .current
            .lock()
            .map_err(|e| LifecycleError::Other(e.to_string()))?
            .get(id)
            .copied()
            .unwrap_or(0))
    }
}

/// 内存版去重器（参考实现 / 测试用）
pub struct InMemoryDeduplicator {
    index: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl Default for InMemoryDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryDeduplicator {
    /// 新建内存去重器
    pub fn new() -> Self {
        Self {
            index: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl Deduplicator for InMemoryDeduplicator {
    async fn dedup(&self, sha256: &str) -> LifecycleResult<Option<String>> {
        Ok(self
            .index
            .lock()
            .map_err(|e| LifecycleError::Other(e.to_string()))?
            .get(sha256)
            .cloned())
    }

    async fn register(&self, sha256: &str, path: &str) -> LifecycleResult<()> {
        self.index
            .lock()
            .map_err(|e| LifecycleError::Other(e.to_string()))?
            .insert(sha256.to_string(), path.to_string());
        Ok(())
    }
}

/// 当前时间（epoch ms）
fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 适配：将生命周期错误转为存储错误（供 store 层桥接）
impl From<LifecycleError> for StoreError {
    fn from(e: LifecycleError) -> Self {
        match e {
            LifecycleError::NotFound(p) => StoreError::NotFound { path: p },
            LifecycleError::Other(msg) => StoreError::Other(msg),
            other => StoreError::Other(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn version_save_list_rollback() {
        let vm = InMemoryVersionManager::new();
        vm.save_version("n1", "v1").await.unwrap();
        vm.save_version("n1", "v2").await.unwrap();
        assert_eq!(vm.current_version("n1").await.unwrap(), 2);
        let list = vm.list_versions("n1").await.unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].version, 2); // 降序
        vm.rollback("n1", 1).await.unwrap();
        assert_eq!(vm.current_version("n1").await.unwrap(), 1);
    }

    #[tokio::test]
    async fn version_rollback_missing_errors() {
        let vm = InMemoryVersionManager::new();
        vm.save_version("n1", "v1").await.unwrap();
        let r = vm.rollback("n1", 99).await;
        assert!(matches!(r, Err(LifecycleError::VersionNotFound { .. })));
    }

    #[tokio::test]
    async fn dedup_registers_and_hits() {
        let d = InMemoryDeduplicator::new();
        assert!(d.dedup("abc").await.unwrap().is_none());
        d.register("abc", "kg/a.png").await.unwrap();
        assert_eq!(d.dedup("abc").await.unwrap(), Some("kg/a.png".to_string()));
    }

    #[test]
    fn lifecycle_error_to_store_error() {
        let e = LifecycleError::NotFound("x".into());
        let se: StoreError = e.into();
        assert!(matches!(se, StoreError::NotFound { .. }));
    }
}
