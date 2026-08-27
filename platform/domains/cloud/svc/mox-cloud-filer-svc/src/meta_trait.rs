// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! MetaStorageProvider trait（后端统一 API）+ MetaBackend::name()。
//!
//! pjd-fstest 常量、后端列表常量也在此处。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::FilerResult;

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
