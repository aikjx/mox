// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Redis Mock 后端（假 SET / GET / EXPIRE / SMEMBERS）。
//!
//! 键空间约定：
//! - `filer:inode:{id}` → JSON(Attr)（TTL 字段保存在 inodes.expire_at）
//! - `filer:dir:{parent}` → BTreeSet<(name, ino)> (SMEMBERS 模拟)

use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::error::FilerResult;
use crate::meta_pg_citus::{
    meta_create, meta_delete, meta_link, meta_list_dir, meta_mkdir, meta_rename, meta_symlink,
    meta_unlink, meta_write_attr,
};
use crate::meta_trait::{
    now_secs, Attr, AttrPatch, DirEntry, InMemInodeStore, MetaBackend, MetaStorageProvider,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RedisInode {
    attr: Attr,
    /// 假 TTL：绝对 unix 秒过期；0 = 永不过期。
    expire_at: u64,
}

#[derive(Debug, Default)]
pub struct RedisMeta {
    inner: Mutex<RedisStore>,
}

#[derive(Debug, Default)]
struct RedisStore {
    inodes: BTreeMap<u64, RedisInode>,
    dirs: BTreeMap<u64, BTreeSet<(String, u64)>>,
    store: InMemInodeStore,
}

impl RedisMeta {
    pub fn new() -> Self {
        Self::default()
    }

    /// 模拟 Redis EXPIRE ino ttl_seconds。
    pub fn fake_expire(&self, ino: u64, ttl_seconds: u64) {
        let mut s = self.inner.lock();
        if let Some(r) = s.inodes.get_mut(&ino) {
            r.expire_at = now_secs() + ttl_seconds;
        }
    }
    /// 假 GET（返回 JSON 字符串）。
    pub fn fake_get(&self, ino: u64) -> Option<String> {
        let mut s = self.inner.lock();
        sync_store_to_redis(&mut s);
        s.inodes
            .get(&ino)
            .and_then(|r| serde_json::to_string(r).ok())
    }
    /// 假 SMEMBERS（dir 子项）。
    pub fn fake_smembers(&self, parent: u64) -> Vec<String> {
        let mut s = self.inner.lock();
        sync_store_to_redis(&mut s);
        s.dirs
            .get(&parent)
            .map(|set| set.iter().map(|(n, _)| n.clone()).collect())
            .unwrap_or_default()
    }

    fn with_store_mut<R>(&self, f: impl FnOnce(&mut InMemInodeStore) -> R) -> R {
        let mut lock = self.inner.lock();
        let r = f(&mut lock.store);
        sync_store_to_redis(&mut lock);
        r
    }
}

fn sync_store_to_redis(s: &mut RedisStore) {
    use std::collections::HashSet;
    // 1) 过期清理
    let now = now_secs();
    let mut expired: Vec<u64> = Vec::new();
    for (i, r) in s.inodes.iter() {
        if r.expire_at != 0 && r.expire_at < now {
            expired.push(*i);
        }
    }
    for i in &expired {
        s.inodes.remove(i);
        if let Some(a) = s.store.inodes.remove(i) {
            s.store.dir_index.remove(&(a.parent, a.name));
        }
    }

    // 2) 完全同步: 先收集 diff
    let store_inos: HashSet<u64> = s.store.inodes.keys().copied().collect();
    let redis_inos: Vec<u64> = s.inodes.keys().copied().collect();
    for i in redis_inos {
        if !store_inos.contains(&i) {
            s.inodes.remove(&i);
        }
    }
    for (ino, a) in s.store.inodes.iter() {
        s.inodes
            .entry(*ino)
            .or_insert(RedisInode {
                attr: a.clone(),
                expire_at: 0,
            })
            .attr = a.clone();
    }

    // 3) dirs 同步 (完全重建，保证一致)
    s.dirs.clear();
    for ((pid, name), ino) in s.store.dir_index.iter() {
        s.dirs.entry(*pid).or_default().insert((name.clone(), *ino));
    }
}

impl MetaBackend for RedisMeta {
    fn name() -> &'static str {
        "redis"
    }
}

#[async_trait]
impl MetaStorageProvider for RedisMeta {
    async fn inode_mkdir(&self, parent: u64, name: &str, mode: u32) -> FilerResult<u64> {
        self.with_store_mut(|s| meta_mkdir(s, parent, name, mode))
    }
    async fn inode_create(&self, parent: u64, name: &str, mode: u32) -> FilerResult<u64> {
        self.with_store_mut(|s| meta_create(s, parent, name, mode))
    }
    async fn inode_lookup(&self, parent: u64, name: &str) -> FilerResult<u64> {
        let mut s = self.inner.lock();
        sync_store_to_redis(&mut s);
        s.store.lookup_name(parent, name)
    }
    async fn inode_write_attr(&self, ino: u64, patch: AttrPatch<'_>) -> FilerResult<()> {
        self.with_store_mut(|s| meta_write_attr(s, ino, patch))
    }
    async fn inode_delete(&self, ino: u64) -> FilerResult<()> {
        self.with_store_mut(|s| meta_delete(s, ino))
    }
    async fn inode_read_attr(&self, ino: u64) -> FilerResult<Attr> {
        let mut s = self.inner.lock();
        sync_store_to_redis(&mut s);
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
