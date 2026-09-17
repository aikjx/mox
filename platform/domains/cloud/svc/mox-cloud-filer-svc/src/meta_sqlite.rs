// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! SQLite 持久化元数据后端。
//!
//! 打开持久文件（默认 `data/cloud-filer.db`，env `CLOUD_FILER_DB_PATH` 可覆盖），
//! 开启 WAL（`journal_mode=WAL; synchronous=NORMAL`），幂等建 `inodes` 表。
//! 启动时从 DB 加载全部 inode 到内存工作集（`InMemInodeStore`），
//! 每次变更写穿回 SQLite，保证重启后目录索引不丢。
//!
//! 若 feature `rusqlite_backend` 关闭，则回退到纯内存 `InMemInodeStore`。

use async_trait::async_trait;
use parking_lot::Mutex;
use std::path::{Path, PathBuf};

use crate::{
    error::{FilerError, FilerResult},
    meta_pg_citus::{
        meta_create, meta_delete, meta_link, meta_list_dir, meta_mkdir, meta_rename, meta_symlink,
        meta_unlink, meta_write_attr,
    },
    meta_trait::{Attr, AttrPatch, DirEntry, InMemInodeStore, MetaBackend, MetaStorageProvider},
};

#[cfg(feature = "rusqlite_backend")]
use rusqlite::Connection;

/// 默认持久 DB 相对路径（相对进程 CWD）。
const DEFAULT_DB_PATH: &str = "data/cloud-filer.db";
/// env 变量名，覆盖默认 DB 路径。
const ENV_DB_PATH: &str = "CLOUD_FILER_DB_PATH";

fn resolve_db_path() -> PathBuf {
    std::env::var(ENV_DB_PATH)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_DB_PATH))
}

// ========================= SqliteMeta =========================

pub struct SqliteMeta {
    inner: Mutex<Inner>,
}

impl std::fmt::Debug for SqliteMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteMeta").finish()
    }
}

struct Inner {
    store: InMemInodeStore,
    #[cfg(feature = "rusqlite_backend")]
    conn: Connection,
}

impl SqliteMeta {
    /// 打开默认持久 DB（env `CLOUD_FILER_DB_PATH` 或 `data/cloud-filer.db`）。
    pub fn new() -> Self {
        let path = resolve_db_path();
        Self::open_at(&path).expect("failed to open cloud-filer metadata db")
    }

    /// 用指定路径打开持久 DB（测试用，隔离每个 case 的 DB 文件）。
    pub fn with_path<P: AsRef<Path>>(path: P) -> FilerResult<Self> {
        Self::open_at(path.as_ref())
    }

    fn open_at(path: &Path) -> FilerResult<Self> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    FilerError::Metadata(format!("create db parent dir {parent:?}: {e}"))
                })?;
            }
        }

        #[cfg(feature = "rusqlite_backend")]
        {
            let conn = Connection::open(path)
                .map_err(|e| FilerError::Metadata(format!("open sqlite {path:?}: {e}")))?;
            // WAL + NORMAL synchronous
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
                .map_err(|e| FilerError::Metadata(format!("pragma wal: {e}")))?;
            // 幂等建表
            conn.execute(
                "CREATE TABLE IF NOT EXISTS inodes (
                    id INTEGER PRIMARY KEY,
                    parent INTEGER NOT NULL,
                    name TEXT NOT NULL,
                    mode INTEGER NOT NULL,
                    uid INTEGER NOT NULL,
                    gid INTEGER NOT NULL,
                    size INTEGER NOT NULL,
                    atime INTEGER NOT NULL,
                    mtime INTEGER NOT NULL,
                    ctime INTEGER NOT NULL,
                    nlink INTEGER NOT NULL,
                    data BLOB NOT NULL,
                    symlink TEXT
                )",
                [],
            )
            .map_err(|e| FilerError::Metadata(format!("create table: {e}")))?;

            // 从 DB 加载到内存工作集
            let store = load_inodes(&conn)?;

            // 若 DB 为空（首次启动），把 root ino=1 落盘
            let row: i64 = conn
                .query_row("SELECT COUNT(*) FROM inodes", [], |r| r.get(0))
                .map_err(|e| FilerError::Metadata(format!("count inodes: {e}")))?;
            if row == 0 {
                let root = store.inodes.get(&1).cloned().ok_or(FilerError::Metadata(
                    "root inode missing after load".into(),
                ))?;
                upsert_row(&conn, &root)?;
            }

            Ok(Self { inner: Mutex::new(Inner { store, conn }) })
        }

        #[cfg(not(feature = "rusqlite_backend"))]
        {
            let _ = path; // path ignored when feature off
            Ok(Self { inner: Mutex::new(Inner { store: InMemInodeStore::new() }) })
        }
    }

    /// 读路径快捷：列目录时从内存工作集取（read-only，无需写穿）。
    fn with_store<R>(&self, f: impl FnOnce(&InMemInodeStore) -> R) -> R {
        let lock = self.inner.lock();
        f(&lock.store)
    }

    /// 写路径：先改内存工作集，再写穿 SQLite。
    fn with_store_mut<R>(&self, f: impl FnOnce(&mut Inner) -> FilerResult<R>) -> FilerResult<R> {
        let mut lock = self.inner.lock();
        f(&mut lock)
    }
}

impl Default for SqliteMeta {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaBackend for SqliteMeta {
    fn name() -> &'static str {
        "sqlite"
    }
}

// ========================= 写穿辅助（feature-gated） =========================

#[cfg(feature = "rusqlite_backend")]
impl Inner {
    fn persist_inode(&mut self, attr: &Attr) -> FilerResult<()> {
        upsert_row(&self.conn, attr)
    }

    fn delete_inode_row(&mut self, ino: u64) -> FilerResult<()> {
        self.conn
            .execute(
                "DELETE FROM inodes WHERE id = ?1",
                rusqlite::params![ino as i64],
            )
            .map_err(|e| FilerError::Metadata(format!("sqlite delete ino {ino}: {e}")))?;
        Ok(())
    }
}

/// INSERT OR REPLACE 单行 inode。
#[cfg(feature = "rusqlite_backend")]
fn upsert_row(conn: &Connection, attr: &Attr) -> FilerResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO inodes
            (id, parent, name, mode, uid, gid, size, atime, mtime, ctime, nlink, data, symlink)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            attr.ino as i64,
            attr.parent as i64,
            attr.name,
            attr.mode as i64,
            attr.uid as i64,
            attr.gid as i64,
            attr.size as i64,
            attr.atime as i64,
            attr.mtime as i64,
            attr.ctime as i64,
            attr.nlink as i64,
            attr.data,
            attr.symlink,
        ],
    )
    .map_err(|e| FilerError::Metadata(format!("sqlite upsert ino {}: {e}", attr.ino)))?;
    Ok(())
}

/// 从 SQLite 加载全部 inode 到内存工作集。
#[cfg(feature = "rusqlite_backend")]
fn load_inodes(conn: &Connection) -> FilerResult<InMemInodeStore> {
    let mut store = InMemInodeStore::new(); // root=1, next_ino=2
    let mut max_ino: u64 = 1;

    let mut stmt = conn
        .prepare(
            "SELECT id, parent, name, mode, uid, gid, size, atime, mtime, ctime, nlink, data, symlink
             FROM inodes",
        )
        .map_err(|e| FilerError::Metadata(format!("prepare load: {e}")))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Attr {
                ino: row.get::<_, i64>(0)? as u64,
                parent: row.get::<_, i64>(1)? as u64,
                name: row.get::<_, String>(2)?,
                mode: row.get::<_, i64>(3)? as u32,
                uid: row.get::<_, i64>(4)? as u32,
                gid: row.get::<_, i64>(5)? as u32,
                size: row.get::<_, i64>(6)? as u64,
                atime: row.get::<_, i64>(7)? as u64,
                mtime: row.get::<_, i64>(8)? as u64,
                ctime: row.get::<_, i64>(9)? as u64,
                nlink: row.get::<_, i64>(10)? as u32,
                data: row.get::<_, Vec<u8>>(11)?,
                symlink: row.get::<_, Option<String>>(12)?,
            })
        })
        .map_err(|e| FilerError::Metadata(format!("query_map load: {e}")))?;

    for row in rows {
        let attr = row.map_err(|e| FilerError::Metadata(format!("row decode: {e}")))?;
        let ino = attr.ino;
        if ino > max_ino {
            max_ino = ino;
        }
        if ino == 1 {
            // root：用 DB 版本覆盖默认
            store.inodes.insert(1, attr);
        } else {
            store.dir_index.insert((attr.parent, attr.name.clone()), ino);
            store.inodes.insert(ino, attr);
        }
    }

    store.next_ino = max_ino + 1;
    Ok(store)
}

// ========================= MetaStorageProvider impl =========================

#[async_trait]
impl MetaStorageProvider for SqliteMeta {
    async fn inode_mkdir(&self, parent: u64, name: &str, mode: u32) -> FilerResult<u64> {
        self.with_store_mut(|inner| {
            let ino = meta_mkdir(&mut inner.store, parent, name, mode)?;
            #[cfg(feature = "rusqlite_backend")]
            {
                let new_attr = inner.store.inodes.get(&ino).cloned();
                let parent_attr = inner.store.inodes.get(&parent).cloned();
                if let Some(a) = &new_attr {
                    inner.persist_inode(a)?;
                }
                if let Some(a) = &parent_attr {
                    inner.persist_inode(a)?;
                }
            }
            Ok(ino)
        })
    }

    async fn inode_create(&self, parent: u64, name: &str, mode: u32) -> FilerResult<u64> {
        self.with_store_mut(|inner| {
            let ino = meta_create(&mut inner.store, parent, name, mode)?;
            #[cfg(feature = "rusqlite_backend")]
            {
                let new_attr = inner.store.inodes.get(&ino).cloned();
                let parent_attr = inner.store.inodes.get(&parent).cloned();
                if let Some(a) = &new_attr {
                    inner.persist_inode(a)?;
                }
                if let Some(a) = &parent_attr {
                    inner.persist_inode(a)?;
                }
            }
            Ok(ino)
        })
    }

    async fn inode_lookup(&self, parent: u64, name: &str) -> FilerResult<u64> {
        self.with_store(|s| s.lookup_name(parent, name))
    }

    async fn inode_write_attr(&self, ino: u64, patch: AttrPatch<'_>) -> FilerResult<()> {
        self.with_store_mut(|inner| {
            meta_write_attr(&mut inner.store, ino, patch)?;
            #[cfg(feature = "rusqlite_backend")]
            {
                let attr = inner.store.inodes.get(&ino).cloned();
                if let Some(a) = &attr {
                    inner.persist_inode(a)?;
                }
            }
            Ok(())
        })
    }

    async fn inode_delete(&self, ino: u64) -> FilerResult<()> {
        self.with_store_mut(|inner| {
            meta_delete(&mut inner.store, ino)?;
            #[cfg(feature = "rusqlite_backend")]
            {
                inner.delete_inode_row(ino)?;
            }
            Ok(())
        })
    }

    async fn inode_read_attr(&self, ino: u64) -> FilerResult<Attr> {
        self.with_store(|s| s.inodes.get(&ino).cloned().ok_or(FilerError::NotFound))
    }

    async fn inode_list_dir(&self, parent: u64) -> FilerResult<Vec<DirEntry>> {
        self.with_store_mut(|inner| meta_list_dir(&mut inner.store, parent))
    }

    async fn inode_link(&self, ino: u64, new_parent: u64, new_name: &str) -> FilerResult<()> {
        self.with_store_mut(|inner| {
            meta_link(&mut inner.store, ino, new_parent, new_name)?;
            #[cfg(feature = "rusqlite_backend")]
            {
                let attr = inner.store.inodes.get(&ino).cloned();
                if let Some(a) = &attr {
                    inner.persist_inode(a)?;
                }
            }
            Ok(())
        })
    }

    async fn inode_unlink(&self, parent: u64, name: &str) -> FilerResult<()> {
        self.with_store_mut(|inner| {
            // 先查 ino（unlink 会从 dir_index 移除）
            let removed_ino = inner.store.dir_index.get(&(parent, name.to_string())).copied();
            meta_unlink(&mut inner.store, parent, name)?;
            #[cfg(feature = "rusqlite_backend")]
            {
                if let Some(ino) = removed_ino {
                    match inner.store.inodes.get(&ino).cloned() {
                        Some(a) => inner.persist_inode(&a)?,
                        None => inner.delete_inode_row(ino)?,
                    }
                }
                // parent 的 mtime 可能变了
                let pa = inner.store.inodes.get(&parent).cloned();
                if let Some(a) = &pa {
                    inner.persist_inode(a)?;
                }
            }
            Ok(())
        })
    }

    async fn inode_symlink(&self, parent: u64, name: &str, target: &str) -> FilerResult<u64> {
        self.with_store_mut(|inner| {
            let ino = meta_symlink(&mut inner.store, parent, name, target)?;
            #[cfg(feature = "rusqlite_backend")]
            {
                let attr = inner.store.inodes.get(&ino).cloned();
                if let Some(a) = &attr {
                    inner.persist_inode(a)?;
                }
            }
            Ok(ino)
        })
    }

    async fn inode_rename(
        &self,
        old_parent: u64,
        old_name: &str,
        new_parent: u64,
        new_name: &str,
    ) -> FilerResult<()> {
        self.with_store_mut(|inner| {
            // 记录被替换掉的目标 ino（rename 可能删除它）
            let target_ino_before =
                inner.store.dir_index.get(&(new_parent, new_name.to_string())).copied();
            meta_rename(&mut inner.store, old_parent, old_name, new_parent, new_name)?;
            #[cfg(feature = "rusqlite_backend")]
            {
                // 被改名的 ino（在 dir_index 里以 new_parent+new_name 存在）
                if let Some(&renamed_ino) =
                    inner.store.dir_index.get(&(new_parent, new_name.to_string()))
                {
                    let attr = inner.store.inodes.get(&renamed_ino).cloned();
                    if let Some(a) = &attr {
                        inner.persist_inode(a)?;
                    }
                }
                // 被替换的目标 ino 可能已从 inodes 中移除
                if let Some(t_ino) = target_ino_before {
                    if inner.store.inodes.get(&t_ino).is_none() {
                        inner.delete_inode_row(t_ino)?;
                    }
                }
            }
            Ok(())
        })
    }
}

// ========================= 单元测试 =========================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sqlite_roundtrip_mkdir_then_list() {
        // 用临时目录隔离，不污染默认 data/cloud-filer.db
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("test.db");
        let s = SqliteMeta::with_path(&db).unwrap();
        let dir = s.inode_mkdir(1, "tmp", 0o755).await.unwrap();
        assert!(dir > 1);
        let list = s.inode_list_dir(1).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "tmp");
    }

    /// 核心持久化回归：写目录项 → drop filer → 同路径重开 → 目录项仍可列出。
    #[tokio::test]
    async fn sqlite_persists_across_drop_and_reopen() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("persist.db");

        // 第一次：写一个目录项
        {
            let s = SqliteMeta::with_path(&db).unwrap();
            let dir = s.inode_mkdir(1, "durable", 0o755).await.unwrap();
            assert!(dir > 1);
            let list = s.inode_list_dir(1).await.unwrap();
            assert_eq!(list.len(), 1);
            assert_eq!(list[0].name, "durable");
            // drop s（离开作用域）
        }

        // 第二次：用同一路径重开
        {
            let s = SqliteMeta::with_path(&db).unwrap();
            let list = s.inode_list_dir(1).await.unwrap();
            assert_eq!(list.len(), 1, "dir entry lost after reopen: {list:?}");
            assert_eq!(list[0].name, "durable");
            // 也验证 lookup 仍然可用
            let ino = s.inode_lookup(1, "durable").await.unwrap();
            assert!(ino > 1);
        }
    }
}
