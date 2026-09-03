// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 真实 Redis 元数据后端（SET / GET / EXPIRE / HSET / HGET / HGETALL / INCR / DEL）。
//!
//! 键空间约定：
//! - `filer:next_ino` → INCR 计数器（root=1，起始 2）
//! - `filer:inode:{id}` → JSON(Attr)（EXPIRE 设置 TTL，0=永不过期）
//! - `filer:dir:{parent}` → HASH field=name, value=ino（目录项索引）
//!
//! 连接配置从环境变量 `REDIS_URL` 读取（默认 `redis://127.0.0.1:6379/`）。
//! 连接失败时返回真实错误，不静默降级到内存。
//! 仅在显式调用 `RedisMeta::new_in_memory()` 时使用内存实现（测试用）。

use async_trait::async_trait;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::{FilerError, FilerResult};
use crate::meta_trait::{
    now_secs, Attr, AttrPatch, DirEntry, InMemInodeStore, MetaBackend, MetaStorageProvider,
    S_IFDIR, S_IFLNK, S_IFREG,
};

// ============================================================================
// 真实 Redis 存储
// ============================================================================

/// 真实 Redis 客户端封装。
///
/// 持有 `MultiplexedConnection`（Clone + Send + Sync），可在 async trait 方法中
/// 按调用克隆，无需外部锁。
#[derive(Clone)]
pub struct RealRedisStore {
    con: redis::aio::MultiplexedConnection,
}

impl std::fmt::Debug for RealRedisStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealRedisStore").finish()
    }
}

impl RealRedisStore {
    /// 依据 `REDIS_URL` 环境变量连接 Redis；连接失败返回真实错误。
    pub async fn connect_from_env() -> FilerResult<Self> {
        let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".into());
        Self::connect(&url).await
    }

    /// 连接指定 URL 并初始化根 inode（幂等）。
    pub async fn connect(url: &str) -> FilerResult<Self> {
        let client = redis::Client::open(url)
            .map_err(|e| FilerError::Other(format!("Redis URL 无效 '{url}': {e}")))?;
        let con = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| FilerError::Other(format!("Redis 连接失败 '{url}': {e}")))?;
        let store = Self { con };
        store.ensure_root().await?;
        Ok(store)
    }

    // ---- 键名辅助 ----

    #[inline]
    fn inode_key(ino: u64) -> String {
        format!("filer:inode:{ino}")
    }

    #[inline]
    fn dir_key(parent: u64) -> String {
        format!("filer:dir:{parent}")
    }

    // ---- 基础操作 ----

    async fn get_inode(&self, ino: u64) -> FilerResult<Attr> {
        let mut con = self.con.clone();
        let raw: Option<String> = con.get(Self::inode_key(ino))
            .await
            .map_err(|e| FilerError::Other(format!("Redis GET inode {ino} 失败: {e}")))?;
        let raw = raw.ok_or(FilerError::NotFound)?;
        serde_json::from_str(&raw)
            .map_err(|e| FilerError::Other(format!("Redis inode {ino} JSON 解析失败: {e}")))
    }

    async fn put_inode(&self, attr: &Attr) -> FilerResult<()> {
        let mut con = self.con.clone();
        let raw = serde_json::to_string(attr)
            .map_err(|e| FilerError::Other(format!("inode JSON 序列化失败: {e}")))?;
        let _: () = con.set(Self::inode_key(attr.ino), raw)
            .await
            .map_err(|e| FilerError::Other(format!("Redis SET inode {} 失败: {e}", attr.ino)))?;
        Ok(())
    }

    async fn delete_inode(&self, ino: u64) -> FilerResult<()> {
        let mut con = self.con.clone();
        let _: () = con.del(Self::inode_key(ino))
            .await
            .map_err(|e| FilerError::Other(format!("Redis DEL inode {ino} 失败: {e}")))?;
        Ok(())
    }

    async fn next_ino(&self) -> FilerResult<u64> {
        let mut con = self.con.clone();
        let v: u64 = con.incr("filer:next_ino", 1)
            .await
            .map_err(|e| FilerError::Other(format!("Redis INCR next_ino 失败: {e}")))?;
        Ok(v)
    }

    async fn dir_get(&self, parent: u64, name: &str) -> FilerResult<Option<u64>> {
        let mut con = self.con.clone();
        let v: Option<String> = con.hget(Self::dir_key(parent), name)
            .await
            .map_err(|e| FilerError::Other(format!("Redis HGET dir {parent}/{name} 失败: {e}")))?;
        Ok(v.and_then(|s| s.parse::<u64>().ok()))
    }

    async fn dir_set(&self, parent: u64, name: &str, ino: u64) -> FilerResult<()> {
        let mut con = self.con.clone();
        let _: () = con.hset(Self::dir_key(parent), name, ino.to_string())
            .await
            .map_err(|e| FilerError::Other(format!("Redis HSET dir {parent}/{name} 失败: {e}")))?;
        Ok(())
    }

    async fn dir_remove(&self, parent: u64, name: &str) -> FilerResult<()> {
        let mut con = self.con.clone();
        let _: () = con.hdel(Self::dir_key(parent), name)
            .await
            .map_err(|e| FilerError::Other(format!("Redis HDEL dir {parent}/{name} 失败: {e}")))?;
        Ok(())
    }

    async fn dir_all(&self, parent: u64) -> FilerResult<BTreeMap<String, u64>> {
        let mut con = self.con.clone();
        let raw: BTreeMap<String, String> = con.hgetall(Self::dir_key(parent))
            .await
            .map_err(|e| FilerError::Other(format!("Redis HGETALL dir {parent} 失败: {e}")))?;
        let mut out = BTreeMap::new();
        for (k, v) in raw {
            if let Ok(ino) = v.parse::<u64>() {
                out.insert(k, ino);
            }
        }
        Ok(out)
    }

    /// 初始化根 inode（ino=1）和 next_ino 计数器（幂等 SET NX）。
    async fn ensure_root(&self) -> FilerResult<()> {
        let mut con = self.con.clone();
        let t = now_secs();
        let root = Attr {
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
        };
        let raw = serde_json::to_string(&root)
            .map_err(|e| FilerError::Other(format!("root inode JSON 序列化失败: {e}")))?;
        // SET NX：仅当不存在时设置（幂等）
        let _: () = con.set_nx(Self::inode_key(1), raw)
            .await
            .map_err(|e| FilerError::Other(format!("Redis SET root inode 失败: {e}")))?;
        // next_ino 初始化为 2（root=1）
        let _: () = con.set_nx("filer:next_ino", "2")
            .await
            .map_err(|e| FilerError::Other(format!("Redis SET next_ino 失败: {e}")))?;
        Ok(())
    }

    // ---- 高级：真实 EXPIRE ----

    /// 对 inode 键设置真实 Redis EXPIRE（秒）。
    pub async fn expire(&self, ino: u64, ttl_seconds: u64) -> FilerResult<()> {
        let mut con = self.con.clone();
        let _: () = con.expire(Self::inode_key(ino), ttl_seconds as i64)
            .await
            .map_err(|e| FilerError::Other(format!("Redis EXPIRE inode {ino} 失败: {e}")))?;
        Ok(())
    }

    /// 真实 GET（返回 inode JSON 字符串）。
    pub async fn get(&self, ino: u64) -> FilerResult<Option<String>> {
        let mut con = self.con.clone();
        let v: Option<String> = con.get(Self::inode_key(ino))
            .await
            .map_err(|e| FilerError::Other(format!("Redis GET inode {ino} 失败: {e}")))?;
        Ok(v)
    }

    /// 真实 SMEMBERS（目录子项名称列表；底层用 HGETALL 取 field）。
    pub async fn smembers(&self, parent: u64) -> FilerResult<Vec<String>> {
        let all = self.dir_all(parent).await?;
        Ok(all.into_keys().collect())
    }
}

// ============================================================================
// 内存实现（仅测试 / 显式回退用）
// ============================================================================

/// 内存 Redis 模拟（保留用于 `#[cfg(test)]` 和显式 `new_in_memory()`）。
#[derive(Debug, Default)]
pub struct InMemoryRedisStore {
    inner: parking_lot::Mutex<InMemRedisState>,
}

#[derive(Debug, Default)]
struct InMemRedisState {
    inodes: BTreeMap<u64, RedisInode>,
    dirs: BTreeMap<u64, BTreeMap<String, u64>>,
    next_ino: u64,
    store: InMemInodeStore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RedisInode {
    attr: Attr,
    /// 假 TTL：绝对 unix 秒过期；0 = 永不过期。
    expire_at: u64,
}

impl InMemoryRedisStore {
    pub fn new() -> Self {
        let mut s = InMemRedisState::default();
        s.next_ino = 2;
        // 初始化 root
        let t = now_secs();
        s.inodes.insert(
            1,
            RedisInode {
                attr: Attr {
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
                expire_at: 0,
            },
        );
        Self {
            inner: parking_lot::Mutex::new(s),
        }
    }

    fn sync_store_to_redis(s: &mut InMemRedisState) {
        // 过期清理
        let now = now_secs();
        let expired: Vec<u64> = s
            .inodes
            .iter()
            .filter(|(_, r)| r.expire_at != 0 && r.expire_at < now)
            .map(|(i, _)| *i)
            .collect();
        for i in &expired {
            s.inodes.remove(i);
            if let Some(a) = s.store.inodes.remove(i) {
                s.store.dir_index.remove(&(a.parent, a.name));
            }
        }
        // 完全同步 inode
        let store_inos: std::collections::HashSet<u64> = s.store.inodes.keys().copied().collect();
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
        // dirs 完全重建
        s.dirs.clear();
        for ((pid, name), ino) in s.store.dir_index.iter() {
            s.dirs.entry(*pid).or_default().insert(name.clone(), *ino);
        }
    }
}

// ============================================================================
// RedisMeta：统一分发（真实 Redis / 内存）
// ============================================================================

enum RedisBackend {
    Real(RealRedisStore),
    InMemory(Arc<InMemoryRedisStore>),
}

/// Redis 元数据后端。默认连接真实 Redis；仅 `new_in_memory()` 使用内存。
pub struct RedisMeta {
    backend: RedisBackend,
}

impl RedisMeta {
    /// 连接真实 Redis（从 `REDIS_URL` 环境变量读取）。
    /// 连接失败返回真实错误，不静默降级。
    pub async fn new() -> FilerResult<Self> {
        let store = RealRedisStore::connect_from_env().await?;
        Ok(Self {
            backend: RedisBackend::Real(store),
        })
    }

    /// 显式使用内存实现（测试 / 无 Redis 环境用）。
    pub fn new_in_memory() -> Self {
        Self {
            backend: RedisBackend::InMemory(Arc::new(InMemoryRedisStore::new())),
        }
    }

    /// 是否为真实 Redis 后端。
    pub fn is_real(&self) -> bool {
        matches!(self.backend, RedisBackend::Real(_))
    }
}

impl MetaBackend for RedisMeta {
    fn name() -> &'static str {
        "redis"
    }
}

// ============================================================================
// MetaStorageProvider 实现
// ============================================================================

#[async_trait]
impl MetaStorageProvider for RedisMeta {
    async fn inode_mkdir(&self, parent: u64, name: &str, mode: u32) -> FilerResult<u64> {
        match &self.backend {
            RedisBackend::Real(r) => real_mkdir(r, parent, name, mode).await,
            RedisBackend::InMemory(m) => {
                let mut lock = m.inner.lock();
                let r = crate::meta_pg_citus::meta_mkdir(&mut lock.store, parent, name, mode);
                InMemoryRedisStore::sync_store_to_redis(&mut lock);
                r
            }
        }
    }

    async fn inode_create(&self, parent: u64, name: &str, mode: u32) -> FilerResult<u64> {
        match &self.backend {
            RedisBackend::Real(r) => real_create(r, parent, name, mode).await,
            RedisBackend::InMemory(m) => {
                let mut lock = m.inner.lock();
                let r = crate::meta_pg_citus::meta_create(&mut lock.store, parent, name, mode);
                InMemoryRedisStore::sync_store_to_redis(&mut lock);
                r
            }
        }
    }

    async fn inode_lookup(&self, parent: u64, name: &str) -> FilerResult<u64> {
        match &self.backend {
            RedisBackend::Real(r) => {
                let v = r.dir_get(parent, name).await?;
                v.ok_or(FilerError::NotFound)
            }
            RedisBackend::InMemory(m) => {
                let mut lock = m.inner.lock();
                InMemoryRedisStore::sync_store_to_redis(&mut lock);
                lock.store.lookup_name(parent, name)
            }
        }
    }

    async fn inode_write_attr(&self, ino: u64, patch: AttrPatch<'_>) -> FilerResult<()> {
        match &self.backend {
            RedisBackend::Real(r) => real_write_attr(r, ino, patch).await,
            RedisBackend::InMemory(m) => {
                let mut lock = m.inner.lock();
                let r = crate::meta_pg_citus::meta_write_attr(&mut lock.store, ino, patch);
                InMemoryRedisStore::sync_store_to_redis(&mut lock);
                r
            }
        }
    }

    async fn inode_delete(&self, ino: u64) -> FilerResult<()> {
        match &self.backend {
            RedisBackend::Real(r) => real_delete(r, ino).await,
            RedisBackend::InMemory(m) => {
                let mut lock = m.inner.lock();
                let r = crate::meta_pg_citus::meta_delete(&mut lock.store, ino);
                InMemoryRedisStore::sync_store_to_redis(&mut lock);
                r
            }
        }
    }

    async fn inode_read_attr(&self, ino: u64) -> FilerResult<Attr> {
        match &self.backend {
            RedisBackend::Real(r) => r.get_inode(ino).await,
            RedisBackend::InMemory(m) => {
                let mut lock = m.inner.lock();
                InMemoryRedisStore::sync_store_to_redis(&mut lock);
                lock.store
                    .inodes
                    .get(&ino)
                    .cloned()
                    .ok_or(FilerError::NotFound)
            }
        }
    }

    async fn inode_list_dir(&self, parent: u64) -> FilerResult<Vec<DirEntry>> {
        match &self.backend {
            RedisBackend::Real(r) => real_list_dir(r, parent).await,
            RedisBackend::InMemory(m) => {
                let mut lock = m.inner.lock();
                let r = crate::meta_pg_citus::meta_list_dir(&mut lock.store, parent);
                InMemoryRedisStore::sync_store_to_redis(&mut lock);
                r
            }
        }
    }

    async fn inode_link(&self, ino: u64, new_parent: u64, new_name: &str) -> FilerResult<()> {
        match &self.backend {
            RedisBackend::Real(r) => real_link(r, ino, new_parent, new_name).await,
            RedisBackend::InMemory(m) => {
                let mut lock = m.inner.lock();
                let r = crate::meta_pg_citus::meta_link(&mut lock.store, ino, new_parent, new_name);
                InMemoryRedisStore::sync_store_to_redis(&mut lock);
                r
            }
        }
    }

    async fn inode_unlink(&self, parent: u64, name: &str) -> FilerResult<()> {
        match &self.backend {
            RedisBackend::Real(r) => real_unlink(r, parent, name).await,
            RedisBackend::InMemory(m) => {
                let mut lock = m.inner.lock();
                let r = crate::meta_pg_citus::meta_unlink(&mut lock.store, parent, name);
                InMemoryRedisStore::sync_store_to_redis(&mut lock);
                r
            }
        }
    }

    async fn inode_symlink(&self, parent: u64, name: &str, target: &str) -> FilerResult<u64> {
        match &self.backend {
            RedisBackend::Real(r) => real_symlink(r, parent, name, target).await,
            RedisBackend::InMemory(m) => {
                let mut lock = m.inner.lock();
                let r = crate::meta_pg_citus::meta_symlink(&mut lock.store, parent, name, target);
                InMemoryRedisStore::sync_store_to_redis(&mut lock);
                r
            }
        }
    }

    async fn inode_rename(
        &self,
        old_parent: u64,
        old_name: &str,
        new_parent: u64,
        new_name: &str,
    ) -> FilerResult<()> {
        match &self.backend {
            RedisBackend::Real(r) => real_rename(r, old_parent, old_name, new_parent, new_name).await,
            RedisBackend::InMemory(m) => {
                let mut lock = m.inner.lock();
                let r = crate::meta_pg_citus::meta_rename(
                    &mut lock.store,
                    old_parent,
                    old_name,
                    new_parent,
                    new_name,
                );
                InMemoryRedisStore::sync_store_to_redis(&mut lock);
                r
            }
        }
    }
}

// ============================================================================
// 真实 Redis 操作实现
// ============================================================================

async fn real_mkdir(r: &RealRedisStore, parent: u64, name: &str, mode: u32) -> FilerResult<u64> {
    // 检查父目录存在且为目录
    let p = r.get_inode(parent).await?;
    if (p.mode & 0o170000) != S_IFDIR {
        return Err(FilerError::AttrInvalid);
    }
    // 检查名称不冲突
    if r.dir_get(parent, name).await?.is_some() {
        return Err(FilerError::Metadata("exists".into()));
    }
    let ino = r.next_ino().await?;
    let t = now_secs();
    let attr = Attr {
        ino,
        parent,
        name: name.to_string(),
        mode: (mode & 0o7777) | S_IFDIR,
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
    r.put_inode(&attr).await?;
    r.dir_set(parent, name, ino).await?;
    // 更新父目录 nlink + mtime
    let mut p = p;
    p.nlink += 1;
    p.mtime = t;
    r.put_inode(&p).await?;
    Ok(ino)
}

async fn real_create(r: &RealRedisStore, parent: u64, name: &str, mode: u32) -> FilerResult<u64> {
    let p = r.get_inode(parent).await?;
    if (p.mode & 0o170000) != S_IFDIR {
        return Err(FilerError::AttrInvalid);
    }
    if r.dir_get(parent, name).await?.is_some() {
        return Err(FilerError::Metadata("exists".into()));
    }
    let ino = r.next_ino().await?;
    let t = now_secs();
    let attr = Attr {
        ino,
        parent,
        name: name.to_string(),
        mode: (mode & 0o7777) | S_IFREG,
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
    r.put_inode(&attr).await?;
    r.dir_set(parent, name, ino).await?;
    // 更新父目录 mtime
    let mut p = p;
    p.mtime = t;
    r.put_inode(&p).await?;
    Ok(ino)
}

async fn real_write_attr(r: &RealRedisStore, ino: u64, patch: AttrPatch<'_>) -> FilerResult<()> {
    let mut a = r.get_inode(ino).await?;
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
    r.put_inode(&a).await
}

async fn real_delete(r: &RealRedisStore, ino: u64) -> FilerResult<()> {
    if ino == 1 {
        return Err(FilerError::AttrInvalid);
    }
    let a = r.get_inode(ino).await?;
    r.dir_remove(a.parent, &a.name).await?;
    r.delete_inode(ino).await
}

async fn real_list_dir(r: &RealRedisStore, parent: u64) -> FilerResult<Vec<DirEntry>> {
    let p = r.get_inode(parent).await?;
    if (p.mode & 0o170000) != S_IFDIR {
        return Err(FilerError::AttrInvalid);
    }
    let all = r.dir_all(parent).await?;
    let mut out = Vec::with_capacity(all.len());
    for (name, ino) in all {
        let typ = match r.get_inode(ino).await {
            Ok(a) => {
                let fmt = a.mode & 0o170000;
                if fmt == S_IFDIR {
                    1
                } else if fmt == S_IFLNK {
                    3
                } else {
                    2
                }
            }
            Err(_) => 2,
        };
        out.push(DirEntry { name, ino, typ });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

async fn real_link(r: &RealRedisStore, ino: u64, new_parent: u64, new_name: &str) -> FilerResult<()> {
    if r.dir_get(new_parent, new_name).await?.is_some() {
        return Err(FilerError::Metadata("exists".into()));
    }
    let mut a = r.get_inode(ino).await?;
    if (a.mode & 0o170000) == S_IFDIR {
        return Err(FilerError::AttrInvalid);
    }
    a.nlink += 1;
    r.put_inode(&a).await?;
    r.dir_set(new_parent, new_name, ino).await
}

async fn real_unlink(r: &RealRedisStore, parent: u64, name: &str) -> FilerResult<()> {
    let ino = r.dir_get(parent, name).await?.ok_or(FilerError::NotFound)?;
    let mut a = r.get_inode(ino).await?;
    let remove;
    if (a.mode & 0o170000) == S_IFDIR {
        // 检查目录是否为空
        let children = r.dir_all(ino).await?;
        if !children.is_empty() {
            return Err(FilerError::NotEmpty);
        }
        a.nlink = a.nlink.saturating_sub(1);
        remove = a.nlink <= 1;
    } else {
        a.nlink -= 1;
        remove = a.nlink == 0;
    }
    r.dir_remove(parent, name).await?;
    if remove {
        r.delete_inode(ino).await?;
    } else {
        r.put_inode(&a).await?;
    }
    // 更新父目录 mtime
    if let Ok(mut p) = r.get_inode(parent).await {
        p.mtime = now_secs();
        let _ = r.put_inode(&p).await;
    }
    Ok(())
}

async fn real_symlink(r: &RealRedisStore, parent: u64, name: &str, target: &str) -> FilerResult<u64> {
    if r.dir_get(parent, name).await?.is_some() {
        return Err(FilerError::Metadata("exists".into()));
    }
    let ino = r.next_ino().await?;
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
    r.put_inode(&attr).await?;
    r.dir_set(parent, name, ino).await?;
    Ok(ino)
}

async fn real_rename(
    r: &RealRedisStore,
    old_parent: u64,
    old_name: &str,
    new_parent: u64,
    new_name: &str,
) -> FilerResult<()> {
    let ino = r
        .dir_get(old_parent, old_name)
        .await?
        .ok_or(FilerError::NotFound)?;
    // 处理目标已存在
    if let Some(target_ino) = r.dir_get(new_parent, new_name).await? {
        if let Ok(tattr) = r.get_inode(target_ino).await {
            if (tattr.mode & 0o170000) == S_IFDIR {
                let children = r.dir_all(target_ino).await?;
                if !children.is_empty() {
                    return Err(FilerError::NotEmpty);
                }
            }
            let mut tattr = tattr;
            tattr.nlink = tattr.nlink.saturating_sub(1);
            if tattr.nlink == 0 {
                r.delete_inode(target_ino).await?;
            } else {
                r.put_inode(&tattr).await?;
            }
        }
    }
    r.dir_remove(old_parent, old_name).await?;
    r.dir_set(new_parent, new_name, ino).await?;
    // 更新 inode 的 parent/name/ctime
    let mut a = r.get_inode(ino).await?;
    a.parent = new_parent;
    a.name = new_name.to_string();
    a.ctime = now_secs();
    r.put_inode(&a).await
}

// ============================================================================
// 单元测试（内存模式，不发真实网络请求）
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_roundtrip() {
        let meta = RedisMeta::new_in_memory();
        let ino = meta.inode_create(1, "test.txt", 0o644).await.unwrap();
        assert!(ino > 1);
        let attr = meta.inode_read_attr(ino).await.unwrap();
        assert_eq!(attr.name, "test.txt");
        let found = meta.inode_lookup(1, "test.txt").await.unwrap();
        assert_eq!(found, ino);
        let listing = meta.inode_list_dir(1).await.unwrap();
        assert_eq!(listing.len(), 1);
        assert_eq!(listing[0].name, "test.txt");
    }

    #[tokio::test]
    async fn real_connection_fails_without_redis() {
        // 指向一个几乎肯定不可达的地址，验证返回真实错误而非静默降级
        let result = RealRedisStore::connect("redis://127.0.0.1:1/").await;
        assert!(result.is_err(), "应返回连接错误");
        let err = result.unwrap_err();
        assert!(
            matches!(err, FilerError::Other(_)),
            "应为 Other 类型错误: {err:?}"
        );
    }

    #[test]
    fn in_memory_is_not_real() {
        let meta = RedisMeta::new_in_memory();
        assert!(!meta.is_real());
    }
}
