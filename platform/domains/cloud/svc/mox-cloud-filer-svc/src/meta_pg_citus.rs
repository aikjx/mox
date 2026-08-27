// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Postgres + Citus Mock 后端。
//!
//! 真实实现用 tokio-postgres + citus coordinator；测试模拟用内存 BTreeMap。
//! Citus 分片规则：`shard_id = ino % 16`。

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::BTreeMap;

use crate::error::FilerResult;
use crate::meta_trait::{
    Attr, AttrPatch, DirEntry, InMemInodeStore, MetaBackend, MetaStorageProvider,
};

#[derive(Debug, Default)]
pub struct PgCitusMeta {
    inner: Mutex<PgStore>,
}

#[derive(Debug)]
struct PgStore {
    #[allow(dead_code)]
    shards: BTreeMap<u64, ()>,
    ino_shard: BTreeMap<u64, u64>,
    store: InMemInodeStore,
}

impl Default for PgStore {
    fn default() -> Self {
        Self {
            shards: (0..16).map(|i| (i, ())).collect(),
            ino_shard: BTreeMap::new(),
            store: InMemInodeStore::new(),
        }
    }
}

impl PgCitusMeta {
    pub fn new() -> Self {
        Self::default()
    }
    /// 查询 ino 对应 citus shard_id = id % 16；若已登记则返回登记值。
    pub fn shard_id_of(&self, ino: u64) -> u64 {
        let s = self.inner.lock();
        s.ino_shard.get(&ino).copied().unwrap_or(ino % 16)
    }

    fn with_store_mut<R>(&self, f: impl FnOnce(&mut PgStore) -> R) -> R {
        let mut lock = self.inner.lock();
        f(&mut lock)
    }

    fn track_shard(pg: &mut PgStore, ino: u64) {
        pg.ino_shard.entry(ino).or_insert(ino % 16);
    }
}

impl MetaBackend for PgCitusMeta {
    fn name() -> &'static str {
        "pg_citus"
    }
}

#[async_trait]
impl MetaStorageProvider for PgCitusMeta {
    async fn inode_mkdir(&self, parent: u64, name: &str, mode: u32) -> FilerResult<u64> {
        self.with_store_mut(|pg| {
            let ino = meta_mkdir(&mut pg.store, parent, name, mode)?;
            Self::track_shard(pg, ino);
            Ok(ino)
        })
    }
    async fn inode_create(&self, parent: u64, name: &str, mode: u32) -> FilerResult<u64> {
        self.with_store_mut(|pg| {
            let ino = meta_create(&mut pg.store, parent, name, mode)?;
            Self::track_shard(pg, ino);
            Ok(ino)
        })
    }
    async fn inode_lookup(&self, parent: u64, name: &str) -> FilerResult<u64> {
        self.inner.lock().store.lookup_name(parent, name)
    }
    async fn inode_write_attr(&self, ino: u64, patch: AttrPatch<'_>) -> FilerResult<()> {
        self.with_store_mut(|pg| meta_write_attr(&mut pg.store, ino, patch))
    }
    async fn inode_delete(&self, ino: u64) -> FilerResult<()> {
        self.with_store_mut(|pg| {
            let res = meta_delete(&mut pg.store, ino);
            if res.is_ok() {
                pg.ino_shard.remove(&ino);
            }
            res
        })
    }
    async fn inode_read_attr(&self, ino: u64) -> FilerResult<Attr> {
        self.inner
            .lock()
            .store
            .inodes
            .get(&ino)
            .cloned()
            .ok_or(crate::error::FilerError::NotFound)
    }
    async fn inode_list_dir(&self, parent: u64) -> FilerResult<Vec<DirEntry>> {
        self.with_store_mut(|pg| meta_list_dir(&mut pg.store, parent))
    }
    async fn inode_link(&self, ino: u64, new_parent: u64, new_name: &str) -> FilerResult<()> {
        self.with_store_mut(|pg| meta_link(&mut pg.store, ino, new_parent, new_name))
    }
    async fn inode_unlink(&self, parent: u64, name: &str) -> FilerResult<()> {
        self.with_store_mut(|pg| meta_unlink(&mut pg.store, parent, name))
    }
    async fn inode_symlink(&self, parent: u64, name: &str, target: &str) -> FilerResult<u64> {
        self.with_store_mut(|pg| {
            let ino = meta_symlink(&mut pg.store, parent, name, target)?;
            Self::track_shard(pg, ino);
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
        self.with_store_mut(|pg| {
            meta_rename(&mut pg.store, old_parent, old_name, new_parent, new_name)
        })
    }
}

// ========================= Free-function ops (work on &mut store) =========================

use crate::meta_trait::{now_secs, S_IFDIR, S_IFLNK, S_IFREG};

pub(crate) fn meta_mkdir(
    store: &mut InMemInodeStore,
    parent: u64,
    name: &str,
    mode: u32,
) -> FilerResult<u64> {
    if store.dir_index.contains_key(&(parent, name.to_string())) {
        return Err(crate::error::FilerError::Metadata("exists".into()));
    }
    let p = store
        .inodes
        .get(&parent)
        .ok_or(crate::error::FilerError::NotFound)?
        .clone();
    if (p.mode & 0o170000) != S_IFDIR {
        return Err(crate::error::FilerError::AttrInvalid);
    }
    let ino = store.next_ino();
    let t = now_secs();
    let effective_mode = (mode & 0o7777) | S_IFDIR;
    let attr = Attr {
        ino,
        parent,
        name: name.to_string(),
        mode: effective_mode,
        uid: 0,
        gid: 0,
        size: 0,
        atime: t,
        mtime: t,
        ctime: t,
        nlink: 2,
        data: Vec::new(),
        symlink: None,
    };
    store.inodes.insert(ino, attr);
    store.dir_index.insert((parent, name.to_string()), ino);
    if let Some(pp) = store.inodes.get_mut(&parent) {
        pp.nlink += 1;
        pp.mtime = t;
    }
    Ok(ino)
}

pub(crate) fn meta_create(
    store: &mut InMemInodeStore,
    parent: u64,
    name: &str,
    mode: u32,
) -> FilerResult<u64> {
    if let Some(&ino) = store.dir_index.get(&(parent, name.to_string())) {
        return Ok(ino);
    }
    let p = store
        .inodes
        .get(&parent)
        .ok_or(crate::error::FilerError::NotFound)?
        .clone();
    if (p.mode & 0o170000) != S_IFDIR {
        return Err(crate::error::FilerError::AttrInvalid);
    }
    let ino = store.next_ino();
    let t = now_secs();
    let effective_mode = (mode & 0o7777) | S_IFREG;
    let attr = Attr {
        ino,
        parent,
        name: name.to_string(),
        mode: effective_mode,
        uid: 0,
        gid: 0,
        size: 0,
        atime: t,
        mtime: t,
        ctime: t,
        nlink: 1,
        data: Vec::new(),
        symlink: None,
    };
    store.inodes.insert(ino, attr);
    store.dir_index.insert((parent, name.to_string()), ino);
    if let Some(pp) = store.inodes.get_mut(&parent) {
        pp.mtime = t;
    }
    Ok(ino)
}

pub(crate) fn meta_write_attr(
    store: &mut InMemInodeStore,
    ino: u64,
    patch: AttrPatch<'_>,
) -> FilerResult<()> {
    let a = store
        .inodes
        .get_mut(&ino)
        .ok_or(crate::error::FilerError::NotFound)?;
    if let Some(v) = patch.size {
        a.size = v;
    }
    if let Some(d) = patch.data {
        a.data = d.to_vec();
        a.size = d.len() as u64;
    }
    if let Some(m) = patch.mode {
        a.mode = (a.mode & 0o170000) | (m & 0o7777);
    }
    if let Some(u) = patch.uid {
        a.uid = u;
    }
    if let Some(g) = patch.gid {
        a.gid = g;
    }
    if let Some(m) = patch.mtime {
        a.mtime = m;
    }
    if let Some(m) = patch.atime {
        a.atime = m;
    }
    if let Some(n) = patch.nlink {
        a.nlink = n;
    }
    a.ctime = now_secs();
    Ok(())
}

pub(crate) fn meta_delete(store: &mut InMemInodeStore, ino: u64) -> FilerResult<()> {
    if ino == 1 {
        return Err(crate::error::FilerError::AttrInvalid);
    }
    let a = store
        .inodes
        .remove(&ino)
        .ok_or(crate::error::FilerError::NotFound)?;
    store.dir_index.remove(&(a.parent, a.name));
    Ok(())
}

pub(crate) fn meta_list_dir(
    store: &mut InMemInodeStore,
    parent: u64,
) -> FilerResult<Vec<DirEntry>> {
    let p = store
        .inodes
        .get(&parent)
        .ok_or(crate::error::FilerError::NotFound)?
        .clone();
    if (p.mode & 0o170000) != S_IFDIR {
        return Err(crate::error::FilerError::AttrInvalid);
    }
    let mut out = Vec::new();
    for ((pid, n), ino) in store.dir_index.iter() {
        if *pid == parent {
            let t = if let Some(a) = store.inodes.get(ino) {
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

pub(crate) fn meta_link(
    store: &mut InMemInodeStore,
    ino: u64,
    new_parent: u64,
    new_name: &str,
) -> FilerResult<()> {
    if store
        .dir_index
        .contains_key(&(new_parent, new_name.to_string()))
    {
        return Err(crate::error::FilerError::Metadata("exists".into()));
    }
    {
        let a = store
            .inodes
            .get_mut(&ino)
            .ok_or(crate::error::FilerError::NotFound)?;
        if (a.mode & 0o170000) == S_IFDIR {
            return Err(crate::error::FilerError::AttrInvalid);
        }
        a.nlink += 1;
    }
    store
        .dir_index
        .insert((new_parent, new_name.to_string()), ino);
    Ok(())
}

pub(crate) fn meta_unlink(store: &mut InMemInodeStore, parent: u64, name: &str) -> FilerResult<()> {
    let ino = store
        .dir_index
        .remove(&(parent, name.to_string()))
        .ok_or(crate::error::FilerError::NotFound)?;
    let remove;
    {
        let a = store
            .inodes
            .get_mut(&ino)
            .ok_or(crate::error::FilerError::NotFound)?;
        if (a.mode & 0o170000) == S_IFDIR {
            let empty = !store.dir_index.keys().any(|(p, _)| *p == ino);
            if !empty {
                store.dir_index.insert((parent, name.to_string()), ino);
                return Err(crate::error::FilerError::NotEmpty);
            }
            a.nlink = a.nlink.saturating_sub(1);
            remove = a.nlink <= 1;
        } else {
            a.nlink -= 1;
            remove = a.nlink == 0;
        }
    }
    if remove {
        store.inodes.remove(&ino);
    }
    if let Some(pp) = store.inodes.get_mut(&parent) {
        pp.mtime = now_secs();
    }
    Ok(())
}

pub(crate) fn meta_symlink(
    store: &mut InMemInodeStore,
    parent: u64,
    name: &str,
    target: &str,
) -> FilerResult<u64> {
    if store.dir_index.contains_key(&(parent, name.to_string())) {
        return Err(crate::error::FilerError::Metadata("exists".into()));
    }
    let ino = store.next_ino();
    let t = now_secs();
    let attr = Attr {
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
    };
    store.inodes.insert(ino, attr);
    store.dir_index.insert((parent, name.to_string()), ino);
    Ok(ino)
}

pub(crate) fn meta_rename(
    store: &mut InMemInodeStore,
    old_parent: u64,
    old_name: &str,
    new_parent: u64,
    new_name: &str,
) -> FilerResult<()> {
    let ino = store
        .dir_index
        .remove(&(old_parent, old_name.to_string()))
        .ok_or(crate::error::FilerError::NotFound)?;
    if let Some(target_ino) = store.dir_index.remove(&(new_parent, new_name.to_string())) {
        if let Some(tattr) = store.inodes.get_mut(&target_ino) {
            if (tattr.mode & 0o170000) == S_IFDIR {
                let empty = !store.dir_index.keys().any(|(p, _)| *p == target_ino);
                if !empty {
                    store
                        .dir_index
                        .insert((new_parent, new_name.to_string()), target_ino);
                    store
                        .dir_index
                        .insert((old_parent, old_name.to_string()), ino);
                    return Err(crate::error::FilerError::NotEmpty);
                }
            }
            tattr.nlink = tattr.nlink.saturating_sub(1);
            if tattr.nlink == 0 {
                store.inodes.remove(&target_ino);
            }
        }
    }
    store
        .dir_index
        .insert((new_parent, new_name.to_string()), ino);
    if let Some(a) = store.inodes.get_mut(&ino) {
        a.parent = new_parent;
        a.name = new_name.to_string();
        a.ctime = now_secs();
    }
    Ok(())
}
