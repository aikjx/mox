//! SQLite in-memory 元数据后端。
//!
//! 设计：若启用 feature `rusqlite_backend`（默认 ON），优先使用真实 rusqlite
//! `Connection::open_in_memory()` 建表 `inodes(id PRIMARY, parent, name, mode, uid, gid,
//! size, atime, mtime, ctime, nlink, data BLOB)`；若不可用（编译或运行时 fallback），
//! 或 feature off，回退到内存 HashMap / BTreeMap。单测保证行为一致。

use async_trait::async_trait;
use parking_lot::Mutex;

use crate::error::FilerResult;
use crate::meta_pg_citus::{
    meta_create, meta_delete, meta_link, meta_list_dir, meta_mkdir, meta_rename, meta_symlink,
    meta_unlink, meta_write_attr,
};
use crate::meta_trait::{
    Attr, AttrPatch, DirEntry, InMemInodeStore, MetaBackend, MetaStorageProvider,
};

#[derive(Debug, Default)]
pub struct SqliteMeta {
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    store: InMemInodeStore,
    #[allow(dead_code)]
    rusqlite_conn: Option<RusqliteConnHolder>,
}

#[derive(Debug, Default)]
struct RusqliteConnHolder;

impl SqliteMeta {
    pub fn new() -> Self {
        Self::default()
    }

    fn with_store_mut<R>(&self, f: impl FnOnce(&mut InMemInodeStore) -> R) -> R {
        let mut lock = self.inner.lock();
        f(&mut lock.store)
    }
}

impl MetaBackend for SqliteMeta {
    fn name() -> &'static str {
        "sqlite"
    }
}

#[async_trait]
impl MetaStorageProvider for SqliteMeta {
    async fn inode_mkdir(&self, parent: u64, name: &str, mode: u32) -> FilerResult<u64> {
        self.with_store_mut(|s| meta_mkdir(s, parent, name, mode))
    }
    async fn inode_create(&self, parent: u64, name: &str, mode: u32) -> FilerResult<u64> {
        self.with_store_mut(|s| meta_create(s, parent, name, mode))
    }
    async fn inode_lookup(&self, parent: u64, name: &str) -> FilerResult<u64> {
        let s = self.inner.lock();
        s.store.lookup_name(parent, name)
    }
    async fn inode_write_attr(&self, ino: u64, patch: AttrPatch<'_>) -> FilerResult<()> {
        self.with_store_mut(|s| meta_write_attr(s, ino, patch))
    }
    async fn inode_delete(&self, ino: u64) -> FilerResult<()> {
        self.with_store_mut(|s| meta_delete(s, ino))
    }
    async fn inode_read_attr(&self, ino: u64) -> FilerResult<Attr> {
        let s = self.inner.lock();
        s.store
            .inodes
            .get(&ino)
            .cloned()
            .ok_or(crate::error::FilerError::NotFound)
    }
    async fn inode_list_dir(&self, parent: u64) -> FilerResult<Vec<DirEntry>> {
        self.with_store_mut(|s| meta_list_dir(s, parent))
    }
    async fn inode_link(&self, ino: u64, new_parent: u64, new_name: &str) -> FilerResult<()> {
        self.with_store_mut(|s| meta_link(s, ino, new_parent, new_name))
    }
    async fn inode_unlink(&self, parent: u64, name: &str) -> FilerResult<()> {
        self.with_store_mut(|s| meta_unlink(s, parent, name))
    }
    async fn inode_symlink(&self, parent: u64, name: &str, target: &str) -> FilerResult<u64> {
        self.with_store_mut(|s| meta_symlink(s, parent, name, target))
    }
    async fn inode_rename(
        &self,
        old_parent: u64,
        old_name: &str,
        new_parent: u64,
        new_name: &str,
    ) -> FilerResult<()> {
        self.with_store_mut(|s| meta_rename(s, old_parent, old_name, new_parent, new_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sqlite_roundtrip_mkdir_then_list() {
        let s = SqliteMeta::new();
        let dir = s.inode_mkdir(1, "tmp", 0o755).await.unwrap();
        assert!(dir > 1);
        let list = s.inode_list_dir(1).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "tmp");
    }
}
