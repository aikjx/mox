// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MetaStorageProvider trait（后端统一 API）+ MetaBackend::name()。
//!
//! pjd-fstest 常量、后端列表常量也在此处。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::FilerResult;

// ========================= 批量操作结果类型 =========================

/// 批量创建结果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BatchCreateResult {
    /// 成功创建的条目：(名称, inode)
    pub created: Vec<(String, u64)>,
    /// 失败的条目：(名称, 错误信息)
    pub failed: Vec<(String, String)>,
}

/// 批量读取属性结果
#[derive(Debug, Clone, Default)]
pub struct BatchReadAttrResult {
    /// 成功找到的属性
    pub found: Vec<Attr>,
    /// 未找到的 inode
    pub not_found: Vec<u64>,
}

/// 批量删除结果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BatchDeleteResult {
    /// 成功删除的 inode
    pub deleted: Vec<u64>,
    /// 失败的条目：(inode, 错误信息)
    pub failed: Vec<(u64, String)>,
}

/// 目录列表分页结果（用于异步迭代/大数据量遍历）
#[derive(Debug, Clone)]
pub struct DirListPage {
    /// 当前页的目录项
    pub entries: Vec<DirEntry>,
    /// 下一页的标记（None 表示没有更多）
    pub next_marker: Option<String>,
    /// 是否还有更多数据
    pub has_more: bool,
}

/// 事务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxStatus {
    /// 活动中
    Active,
    /// 已准备（两阶段提交的第一阶段）
    Prepared,
    /// 已提交
    Committed,
    /// 已回滚
    RolledBack,
    /// 未知
    Unknown,
}

impl TxStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TxStatus::Active => "active",
            TxStatus::Prepared => "prepared",
            TxStatus::Committed => "committed",
            TxStatus::RolledBack => "rolled_back",
            TxStatus::Unknown => "unknown",
        }
    }
}

/// pjd-fstest 模拟用：总 cases（我们用 10 个高层操作，每个代表一组 case）。
pub const PJD_CASES_TOTAL: usize = 10;
/// 通过阈值 95%（9.5/10 → 所以 10/10 才真正 ≥ 阈值）。
pub const PJD_PASS_THRESHOLD: f64 = 0.95;
/// 三后端枚举。
pub const META_BACKENDS: &[&str] = &["sqlite", "pg_citus", "redis"];

/// POSIX 文件属性（inode 层：对应 stat 结构）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attr {
    pub ino: u64,
    pub parent: u64,
    pub name: String,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub nlink: u32,
    /// 0 for dir; data only meaningful for regular files (simulated inline).
    pub data: Vec<u8>,
    /// symlink target.
    pub symlink: Option<String>,
}

/// 目录项（readdir 用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub ino: u64,
    pub typ: u8, // 1=dir, 2=file, 3=symlink
}

/// 元数据后端统一接口。每个方法对应 POSIX 文件系统操作。
#[async_trait]
pub trait MetaStorageProvider: Send + Sync {
    /// 创建目录。
    async fn inode_mkdir(&self, parent: u64, name: &str, mode: u32) -> FilerResult<u64>;
    /// 创建普通文件。
    async fn inode_create(&self, parent: u64, name: &str, mode: u32) -> FilerResult<u64>;
    /// parent + name → ino。
    async fn inode_lookup(&self, parent: u64, name: &str) -> FilerResult<u64>;
    /// 写入属性（例如 size / data / mtime）。
    async fn inode_write_attr(&self, ino: u64, patch: AttrPatch<'_>) -> FilerResult<()>;
    /// 删除 inode（按 ino，caller 需保证无引用）。
    async fn inode_delete(&self, ino: u64) -> FilerResult<()>;
    /// 读取属性。
    async fn inode_read_attr(&self, ino: u64) -> FilerResult<Attr>;
    /// 列目录。
    async fn inode_list_dir(&self, parent: u64) -> FilerResult<Vec<DirEntry>>;
    /// 硬链接：新 (parent, name) → 旧 ino，nlink +1。
    async fn inode_link(&self, ino: u64, new_parent: u64, new_name: &str) -> FilerResult<()>;
    /// unlink: (parent, name) → nlink -1，若 0 则 delete。
    async fn inode_unlink(&self, parent: u64, name: &str) -> FilerResult<()>;
    /// symlink 创建。
    async fn inode_symlink(&self, parent: u64, name: &str, target: &str) -> FilerResult<u64>;
    /// rename。
    async fn inode_rename(
        &self,
        old_parent: u64,
        old_name: &str,
        new_parent: u64,
        new_name: &str,
    ) -> FilerResult<()>;

    // ========================= 批量操作（增强） =========================

    /// 批量创建文件。
    ///
    /// 默认实现：逐个调用 `inode_create`，不保证原子性。
    /// 高性能后端（如 Citus）应重写此方法以获得更好的性能。
    async fn batch_create(
        &self,
        parent: u64,
        names: &[&str],
        mode: u32,
    ) -> FilerResult<BatchCreateResult> {
        let mut result = BatchCreateResult::default();
        for name in names {
            match self.inode_create(parent, name, mode).await {
                Ok(ino) => result.created.push(((*name).to_string(), ino)),
                Err(e) => result.failed.push(((*name).to_string(), e.to_string())),
            }
        }
        Ok(result)
    }

    /// 批量读取属性。
    ///
    /// 默认实现：逐个调用 `inode_read_attr`。
    async fn batch_read_attr(&self, inos: &[u64]) -> FilerResult<BatchReadAttrResult> {
        let mut result = BatchReadAttrResult::default();
        for ino in inos {
            match self.inode_read_attr(*ino).await {
                Ok(attr) => result.found.push(attr),
                Err(_) => result.not_found.push(*ino),
            }
        }
        Ok(result)
    }

    /// 批量删除 inode。
    ///
    /// 默认实现：逐个调用 `inode_delete`，不保证原子性。
    async fn batch_delete(&self, inos: &[u64]) -> FilerResult<BatchDeleteResult> {
        let mut result = BatchDeleteResult::default();
        for ino in inos {
            match self.inode_delete(*ino).await {
                Ok(()) => result.deleted.push(*ino),
                Err(e) => result.failed.push((*ino, e.to_string())),
            }
        }
        Ok(result)
    }

    /// 批量列目录。
    ///
    /// 默认实现：逐个调用 `inode_list_dir`。
    async fn batch_list_dir(&self, parents: &[u64]) -> FilerResult<Vec<(u64, FilerResult<Vec<DirEntry>>)>> {
        let mut results = Vec::with_capacity(parents.len());
        for parent in parents {
            let r = self.inode_list_dir(*parent).await;
            results.push((*parent, r));
        }
        Ok(results)
    }

    // ========================= 事务支持（增强） =========================

    /// 开始一个新事务，返回事务 ID。
    ///
    /// 默认实现：返回 Unsupported 错误。
    /// 支持事务的后端（如 Citus/Postgres）应重写此方法。
    async fn begin_tx(&self) -> FilerResult<u64> {
        Err(crate::error::FilerError::Unsupported(
            "transactions not supported by this backend",
        ))
    }

    /// 提交事务。
    async fn commit_tx(&self, _tx_id: u64) -> FilerResult<()> {
        Err(crate::error::FilerError::Unsupported(
            "transactions not supported by this backend",
        ))
    }

    /// 回滚事务。
    async fn rollback_tx(&self, _tx_id: u64) -> FilerResult<()> {
        Err(crate::error::FilerError::Unsupported(
            "transactions not supported by this backend",
        ))
    }

    /// 获取事务状态。
    async fn tx_status(&self, _tx_id: u64) -> FilerResult<TxStatus> {
        Err(crate::error::FilerError::Unsupported(
            "transactions not supported by this backend",
        ))
    }

    // ========================= 异步迭代/分页（增强） =========================

    /// 分页列目录（适用于超大目录，避免一次性加载所有条目）。
    ///
    /// - `parent`: 父目录 inode
    /// - `marker`: 分页标记（从上次结果的 next_marker 获取，None 表示从头开始）
    /// - `page_size`: 每页最大条目数
    ///
    /// 默认实现：调用 `inode_list_dir` 后在内存中分页。
    /// 高性能后端应重写此方法以支持数据库端分页。
    async fn list_dir_paged(
        &self,
        parent: u64,
        marker: Option<&str>,
        page_size: u32,
    ) -> FilerResult<DirListPage> {
        let all = self.inode_list_dir(parent).await?;
        let page_size = page_size.max(1) as usize;

        // 找到起始位置
        let start_idx = match marker {
            Some(m) => {
                match all.binary_search_by(|e| e.name.as_str().cmp(m)) {
                    Ok(idx) => idx + 1, // 从标记的下一个开始
                    Err(idx) => idx,    // 插入位置
                }
            }
            None => 0,
        };

        let end_idx = (start_idx + page_size).min(all.len());
        let entries: Vec<DirEntry> = all[start_idx..end_idx].to_vec();
        let has_more = end_idx < all.len();
        let next_marker = if has_more {
            entries.last().map(|e| e.name.clone())
        } else {
            None
        };

        Ok(DirListPage {
            entries,
            next_marker,
            has_more,
        })
    }

    /// 检查后端是否支持原生分页目录列表。
    fn supports_native_pagination(&self) -> bool {
        false
    }

    /// 检查后端是否支持事务。
    fn supports_transactions(&self) -> bool {
        false
    }

    /// 检查后端是否支持批量操作优化。
    fn supports_batch_optimization(&self) -> bool {
        false
    }
}

/// 属性 patch（供 inode_write_attr 使用；部分字段可选）。
#[derive(Debug, Default, Clone)]
pub struct AttrPatch<'a> {
    pub size: Option<u64>,
    pub data: Option<&'a [u8]>,
    pub mode: Option<u32>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub mtime: Option<u64>,
    pub atime: Option<u64>,
    pub nlink: Option<u32>,
}

/// MetaBackend：最小 trait，返回 name()。三后端需实现 MetaStorageProvider + MetaBackend。
pub trait MetaBackend {
    fn name() -> &'static str
    where
        Self: Sized;
}

/// 共享辅助：当前 unix 秒数。
pub(crate) fn now_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 内存中通用 Inode store。所有后端都可用此结构实现（可避免外部依赖）。
#[derive(Debug, Clone)]
pub struct InMemInodeStore {
    pub next_ino: u64,
    /// ino -> attr
    pub inodes: BTreeMap<u64, Attr>,
    /// (parent, name) -> ino
    pub dir_index: BTreeMap<(u64, String), u64>,
}

impl Default for InMemInodeStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemInodeStore {
    pub fn new() -> Self {
        let mut s = Self {
            next_ino: 2, // root=1
            inodes: BTreeMap::new(),
            dir_index: BTreeMap::new(),
        };
        let t = now_secs();
        s.inodes.insert(
            1,
            Attr {
                ino: 1,
                parent: 1,
                name: "/".into(),
                mode: 0o40755,
                uid: 0,
                gid: 0,
                size: 0,
                atime: t,
                mtime: t,
                ctime: t,
                nlink: 2,
                data: Vec::new(),
                symlink: None,
            },
        );
        s
    }
    pub fn next_ino(&mut self) -> u64 {
        let i = self.next_ino;
        self.next_ino += 1;
        i
    }
    pub fn lookup_name(&self, parent: u64, name: &str) -> FilerResult<u64> {
        self.dir_index
            .get(&(parent, name.to_string()))
            .copied()
            .ok_or(crate::error::FilerError::NotFound)
    }
    pub fn is_dir(&self, ino: u64) -> FilerResult<bool> {
        let a = self
            .inodes
            .get(&ino)
            .ok_or(crate::error::FilerError::NotFound)?;
        Ok((a.mode & libc_sifmt()) == S_IFDIR)
    }
}

/// 简化的 mode 常量（跨平台安全）。
pub const S_IFDIR: u32 = 0o040000;
pub const S_IFREG: u32 = 0o100000;
pub const S_IFLNK: u32 = 0o120000;
fn libc_sifmt() -> u32 {
    0o170000
}

// ========================= 增强功能单元测试 =========================

#[cfg(test)]
mod trait_enhanced_tests {
    use super::*;
    use async_trait::async_trait;

    /// 测试用 mock 后端，基于 InMemInodeStore
    struct MockMeta {
        store: parking_lot::Mutex<InMemInodeStore>,
    }

    impl MockMeta {
        fn new() -> Self {
            MockMeta {
                store: parking_lot::Mutex::new(InMemInodeStore::new()),
            }
        }
    }

    #[async_trait]
    impl MetaStorageProvider for MockMeta {
        async fn inode_mkdir(&self, parent: u64, name: &str, mode: u32) -> FilerResult<u64> {
            let mut s = self.store.lock();
            if s.dir_index.contains_key(&(parent, name.to_string())) {
                return Err(FilerError::Metadata("exists".into()));
            }
            let ino = s.next_ino();
            let t = now_secs();
            s.inodes.insert(
                ino,
                Attr {
                    ino,
                    parent,
                    name: name.to_string(),
                    mode: S_IFDIR | mode,
                    uid: 0,
                    gid: 0,
                    size: 0,
                    atime: t,
                    mtime: t,
                    ctime: t,
                    nlink: 2,
                    data: Vec::new(),
                    symlink: None,
                },
            );
            s.dir_index.insert((parent, name.to_string()), ino);
            Ok(ino)
        }

        async fn inode_create(&self, parent: u64, name: &str, mode: u32) -> FilerResult<u64> {
            let mut s = self.store.lock();
            if s.dir_index.contains_key(&(parent, name.to_string())) {
                return Err(FilerError::Metadata("exists".into()));
            }
            let ino = s.next_ino();
            let t = now_secs();
            s.inodes.insert(
                ino,
                Attr {
                    ino,
                    parent,
                    name: name.to_string(),
                    mode: S_IFREG | mode,
                    uid: 0,
                    gid: 0,
                    size: 0,
                    atime: t,
                    mtime: t,
                    ctime: t,
                    nlink: 1,
                    data: Vec::new(),
                    symlink: None,
                },
            );
            s.dir_index.insert((parent, name.to_string()), ino);
            Ok(ino)
        }

        async fn inode_lookup(&self, parent: u64, name: &str) -> FilerResult<u64> {
            let s = self.store.lock();
            s.lookup_name(parent, name)
        }

        async fn inode_write_attr(&self, ino: u64, patch: AttrPatch<'_>) -> FilerResult<()> {
            let mut s = self.store.lock();
            let a = s.inodes.get_mut(&ino).ok_or(FilerError::NotFound)?;
            if let Some(sz) = patch.size {
                a.size = sz;
            }
            if let Some(d) = patch.data {
                a.data = d.to_vec();
                a.size = d.len() as u64;
            }
            if let Some(m) = patch.mode {
                a.mode = (a.mode & 0o170000) | m;
            }
            if let Some(u) = patch.uid {
                a.uid = u;
            }
            if let Some(g) = patch.gid {
                a.gid = g;
            }
            if let Some(mt) = patch.mtime {
                a.mtime = mt;
            }
            if let Some(at) = patch.atime {
                a.atime = at;
            }
            if let Some(n) = patch.nlink {
                a.nlink = n;
            }
            a.ctime = now_secs();
            Ok(())
        }

        async fn inode_delete(&self, ino: u64) -> FilerResult<()> {
            let mut s = self.store.lock();
            if ino == 1 {
                return Err(FilerError::AttrInvalid);
            }
            let a = s.inodes.remove(&ino).ok_or(FilerError::NotFound)?;
            s.dir_index.remove(&(a.parent, a.name));
            Ok(())
        }

        async fn inode_read_attr(&self, ino: u64) -> FilerResult<Attr> {
            let s = self.store.lock();
            s.inodes.get(&ino).cloned().ok_or(FilerError::NotFound)
        }

        async fn inode_list_dir(&self, parent: u64) -> FilerResult<Vec<DirEntry>> {
            let s = self.store.lock();
            let p = s.inodes.get(&parent).ok_or(FilerError::NotFound)?.clone();
            if (p.mode & 0o170000) != S_IFDIR {
                return Err(FilerError::AttrInvalid);
            }
            let mut out = Vec::new();
            for ((pid, n), ino) in s.dir_index.iter() {
                if *pid == parent {
                    let t = if let Some(a) = s.inodes.get(ino) {
                        let fmt = a.mode & 0o170000;
                        if fmt == S_IFDIR {
                            1
                        } else if fmt == S_IFLNK {
                            3
                        } else {
                            2
                        }
                    } else {
                        2
                    };
                    out.push(DirEntry {
                        name: n.clone(),
                        ino: *ino,
                        typ: t,
                    });
                }
            }
            out.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(out)
        }

        async fn inode_link(
            &self,
            ino: u64,
            new_parent: u64,
            new_name: &str,
        ) -> FilerResult<()> {
            let mut s = self.store.lock();
            if s.dir_index.contains_key(&(new_parent, new_name.to_string())) {
                return Err(FilerError::Metadata("exists".into()));
            }
            {
                let a = s.inodes.get_mut(&ino).ok_or(FilerError::NotFound)?;
                if (a.mode & 0o170000) == S_IFDIR {
                    return Err(FilerError::AttrInvalid);
                }
                a.nlink += 1;
            }
            s.dir_index.insert((new_parent, new_name.to_string()), ino);
            Ok(())
        }

        async fn inode_unlink(&self, parent: u64, name: &str) -> FilerResult<()> {
            let mut s = self.store.lock();
            let ino = s
                .dir_index
                .remove(&(parent, name.to_string()))
                .ok_or(FilerError::NotFound)?;
            let remove;
            {
                let a = s.inodes.get_mut(&ino).ok_or(FilerError::NotFound)?;
                a.nlink -= 1;
                remove = a.nlink == 0;
            }
            if remove {
                s.inodes.remove(&ino);
            }
            Ok(())
        }

        async fn inode_symlink(
            &self,
            parent: u64,
            name: &str,
            target: &str,
        ) -> FilerResult<u64> {
            let mut s = self.store.lock();
            if s.dir_index.contains_key(&(parent, name.to_string())) {
                return Err(FilerError::Metadata("exists".into()));
            }
            let ino = s.next_ino();
            let t = now_secs();
            s.inodes.insert(
                ino,
                Attr {
                    ino,
                    parent,
                    name: name.to_string(),
                    mode: S_IFLNK | 0o777,
                    uid: 0,
                    gid: 0,
                    size: target.len() as u64,
                    atime: t,
                    mtime: t,
                    ctime: t,
                    nlink: 1,
                    data: Vec::new(),
                    symlink: Some(target.to_string()),
                },
            );
            s.dir_index.insert((parent, name.to_string()), ino);
            Ok(ino)
        }

        async fn inode_rename(
            &self,
            old_parent: u64,
            old_name: &str,
            new_parent: u64,
            new_name: &str,
        ) -> FilerResult<()> {
            let mut s = self.store.lock();
            let ino = s
                .dir_index
                .remove(&(old_parent, old_name.to_string()))
                .ok_or(FilerError::NotFound)?;
            // 简化：不处理目标已存在的情况
            s.dir_index.insert((new_parent, new_name.to_string()), ino);
            if let Some(a) = s.inodes.get_mut(&ino) {
                a.parent = new_parent;
                a.name = new_name.to_string();
                a.ctime = now_secs();
            }
            Ok(())
        }
    }

    #[test]
    fn test_tx_status_as_str() {
        assert_eq!(TxStatus::Active.as_str(), "active");
        assert_eq!(TxStatus::Prepared.as_str(), "prepared");
        assert_eq!(TxStatus::Committed.as_str(), "committed");
        assert_eq!(TxStatus::RolledBack.as_str(), "rolled_back");
        assert_eq!(TxStatus::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_batch_create_result_default() {
        let r = BatchCreateResult::default();
        assert!(r.created.is_empty());
        assert!(r.failed.is_empty());
    }

    #[test]
    fn test_batch_delete_result_default() {
        let r = BatchDeleteResult::default();
        assert!(r.deleted.is_empty());
        assert!(r.failed.is_empty());
    }

    #[test]
    fn test_batch_read_attr_result_default() {
        let r = BatchReadAttrResult::default();
        assert!(r.found.is_empty());
        assert!(r.not_found.is_empty());
    }

    #[tokio::test]
    async fn test_default_batch_create() {
        let meta = MockMeta::new();
        let result = meta.batch_create(1, &["a.txt", "b.txt", "c.txt"], 0o644).await.unwrap();
        assert_eq!(result.created.len(), 3);
        assert_eq!(result.failed.len(), 0);

        // 验证文件确实被创建
        let s = meta.store.lock();
        assert!(s.dir_index.contains_key(&(1, "a.txt".to_string())));
        assert!(s.dir_index.contains_key(&(1, "b.txt".to_string())));
        assert!(s.dir_index.contains_key(&(1, "c.txt".to_string())));
    }

    #[tokio::test]
    async fn test_default_batch_create_with_failures() {
        let meta = MockMeta::new();
        // 先创建一个文件
        meta.inode_create(1, "existing.txt", 0o644).await.unwrap();

        // 批量创建，包含已存在的
        let result = meta
            .batch_create(1, &["existing.txt", "new.txt"], 0o644)
            .await
            .unwrap();
        assert_eq!(result.created.len(), 1);
        assert_eq!(result.created[0].0, "new.txt");
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].0, "existing.txt");
    }

    #[tokio::test]
    async fn test_default_batch_read_attr() {
        let meta = MockMeta::new();
        let ino1 = meta.inode_create(1, "x.txt", 0o644).await.unwrap();
        let ino2 = meta.inode_create(1, "y.txt", 0o644).await.unwrap();

        let result = meta.batch_read_attr(&[ino1, ino2, 99999]).await.unwrap();
        assert_eq!(result.found.len(), 2);
        assert_eq!(result.not_found.len(), 1);
        assert_eq!(result.not_found[0], 99999);
    }

    #[tokio::test]
    async fn test_default_batch_delete() {
        let meta = MockMeta::new();
        let ino1 = meta.inode_create(1, "d1.txt", 0o644).await.unwrap();
        let ino2 = meta.inode_create(1, "d2.txt", 0o644).await.unwrap();

        let result = meta.batch_delete(&[ino1, ino2, 99999]).await.unwrap();
        assert_eq!(result.deleted.len(), 2);
        assert_eq!(result.failed.len(), 1);
    }

    #[tokio::test]
    async fn test_default_batch_list_dir() {
        let meta = MockMeta::new();
        meta.inode_create(1, "f1.txt", 0o644).await.unwrap();
        meta.inode_create(1, "f2.txt", 0o644).await.unwrap();

        let result = meta.batch_list_dir(&[1, 99999]).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].1.is_ok());
        assert_eq!(result[0].1.as_ref().unwrap().len(), 2);
        assert!(result[1].1.is_err());
    }

    #[tokio::test]
    async fn test_default_transactions_unsupported() {
        let meta = MockMeta::new();
        assert!(!meta.supports_transactions());

        let result = meta.begin_tx().await;
        assert!(result.is_err());
        match result.err().unwrap() {
            FilerError::Unsupported(_) => {}
            _ => panic!("expected Unsupported error"),
        }
    }

    #[tokio::test]
    async fn test_default_pagination() {
        let meta = MockMeta::new();
        // 创建 10 个文件（按字母顺序命名）
        for i in 0..10 {
            let name = format!("file_{:02}.txt", i);
            meta.inode_create(1, &name, 0o644).await.unwrap();
        }

        assert!(!meta.supports_native_pagination());

        // 第一页，每页 3 个
        let page1 = meta.list_dir_paged(1, None, 3).await.unwrap();
        assert_eq!(page1.entries.len(), 3);
        assert!(page1.has_more);
        assert!(page1.next_marker.is_some());
        assert_eq!(page1.entries[0].name, "file_00.txt");
        assert_eq!(page1.entries[2].name, "file_02.txt");

        // 第二页
        let page2 = meta
            .list_dir_paged(1, page1.next_marker.as_deref(), 3)
            .await
            .unwrap();
        assert_eq!(page2.entries.len(), 3);
        assert!(page2.has_more);
        assert_eq!(page2.entries[0].name, "file_03.txt");

        // 第三页
        let page3 = meta
            .list_dir_paged(1, page2.next_marker.as_deref(), 3)
            .await
            .unwrap();
        assert_eq!(page3.entries.len(), 3);
        assert!(page3.has_more);

        // 第四页（最后一页，只有 1 个）
        let page4 = meta
            .list_dir_paged(1, page3.next_marker.as_deref(), 3)
            .await
            .unwrap();
        assert_eq!(page4.entries.len(), 1);
        assert!(!page4.has_more);
        assert!(page4.next_marker.is_none());
    }

    #[tokio::test]
    async fn test_pagination_empty_dir() {
        let meta = MockMeta::new();
        let page = meta.list_dir_paged(1, None, 10).await.unwrap();
        assert!(page.entries.is_empty());
        assert!(!page.has_more);
        assert!(page.next_marker.is_none());
    }

    #[tokio::test]
    async fn test_pagination_page_size_zero() {
        let meta = MockMeta::new();
        meta.inode_create(1, "test.txt", 0o644).await.unwrap();

        // page_size 为 0 时应该至少返回 1 个
        let page = meta.list_dir_paged(1, None, 0).await.unwrap();
        assert_eq!(page.entries.len(), 1);
    }

    #[test]
    fn test_default_capability_flags() {
        // 测试默认值（需要一个实现了 trait 的类型，用 function pointer 技巧）
        struct Dummy;
        #[async_trait]
        impl MetaStorageProvider for Dummy {
            async fn inode_mkdir(&self, _: u64, _: &str, _: u32) -> FilerResult<u64> {
                Err(FilerError::Unsupported("".into()))
            }
            async fn inode_create(&self, _: u64, _: &str, _: u32) -> FilerResult<u64> {
                Err(FilerError::Unsupported("".into()))
            }
            async fn inode_lookup(&self, _: u64, _: &str) -> FilerResult<u64> {
                Err(FilerError::Unsupported("".into()))
            }
            async fn inode_write_attr(&self, _: u64, _: AttrPatch<'_>) -> FilerResult<()> {
                Err(FilerError::Unsupported("".into()))
            }
            async fn inode_delete(&self, _: u64) -> FilerResult<()> {
                Err(FilerError::Unsupported("".into()))
            }
            async fn inode_read_attr(&self, _: u64) -> FilerResult<Attr> {
                Err(FilerError::Unsupported("".into()))
            }
            async fn inode_list_dir(&self, _: u64) -> FilerResult<Vec<DirEntry>> {
                Err(FilerError::Unsupported("".into()))
            }
            async fn inode_link(&self, _: u64, _: u64, _: &str) -> FilerResult<()> {
                Err(FilerError::Unsupported("".into()))
            }
            async fn inode_unlink(&self, _: u64, _: &str) -> FilerResult<()> {
                Err(FilerError::Unsupported("".into()))
            }
            async fn inode_symlink(&self, _: u64, _: &str, _: &str) -> FilerResult<u64> {
                Err(FilerError::Unsupported("".into()))
            }
            async fn inode_rename(
                &self,
                _: u64,
                _: &str,
                _: u64,
                _: &str,
            ) -> FilerResult<()> {
                Err(FilerError::Unsupported("".into()))
            }
        }

        let d = Dummy;
        assert!(!d.supports_transactions());
        assert!(!d.supports_native_pagination());
        assert!(!d.supports_batch_optimization());
    }

    #[test]
    fn test_mode_constants() {
        // 验证 mode 常量的正确性
        assert_eq!(S_IFDIR & 0o170000, S_IFDIR);
        assert_eq!(S_IFREG & 0o170000, S_IFREG);
        assert_eq!(S_IFLNK & 0o170000, S_IFLNK);
        assert_ne!(S_IFDIR, S_IFREG);
        assert_ne!(S_IFDIR, S_IFLNK);
        assert_ne!(S_IFREG, S_IFLNK);
    }
}
