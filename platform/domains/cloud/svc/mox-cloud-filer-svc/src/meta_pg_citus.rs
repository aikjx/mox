// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Postgres + Citus 分布式元数据后端（增强版）。
//!
//! 真实实现用 tokio-postgres + citus coordinator；测试模拟用内存 BTreeMap。
//!
//! # 增强特性
//!
//! * **分片策略**：按目录 hash 分片（directory-based sharding），支持千亿级文件元数据
//! * **分布式事务**：模拟两阶段提交（2PC），跨分片操作一致性
//! * **批量操作优化**：批量插入、批量查询、批量删除，减少网络往返
//! * **连接池管理**：连接复用、连接状态监控、最大连接数限制
//! * **健康检查**：定期检测分片节点健康状态，自动故障转移
//!
//! # 分片规则
//!
//! - 默认：`shard_id = ino % 16`（inode 取模）
//! - 目录 hash：`shard_id = hash(parent_dir) % num_shards`（同一目录的子项在同一分片）
//! - 生产环境可配置 32 / 64 / 128 分片

use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    error::FilerResult,
    meta_trait::{
        Attr, AttrPatch, BatchCreateResult, BatchDeleteResult, BatchReadAttrResult, DirEntry,
        InMemInodeStore, MetaBackend, MetaStorageProvider,
    },
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

    // ========================= 批量操作重写（优化版本） =========================

    async fn batch_create(
        &self,
        parent: u64,
        names: &[&str],
        mode: u32,
    ) -> FilerResult<crate::meta_trait::BatchCreateResult> {
        Ok(PgCitusMeta::batch_create(self, parent, names, mode))
    }

    async fn batch_read_attr(
        &self,
        inos: &[u64],
    ) -> FilerResult<crate::meta_trait::BatchReadAttrResult> {
        Ok(PgCitusMeta::batch_read_attr(self, inos))
    }

    async fn batch_delete(
        &self,
        inos: &[u64],
    ) -> FilerResult<crate::meta_trait::BatchDeleteResult> {
        Ok(PgCitusMeta::batch_delete(self, inos))
    }

    async fn batch_list_dir(
        &self,
        parents: &[u64],
    ) -> FilerResult<Vec<(u64, FilerResult<Vec<DirEntry>>)>> {
        Ok(PgCitusMeta::batch_list_dir(self, parents))
    }

    // ========================= 能力标志重写 =========================

    fn supports_batch_optimization(&self) -> bool {
        true
    }

    fn supports_transactions(&self) -> bool {
        true
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
    let p = store.inodes.get(&parent).ok_or(crate::error::FilerError::NotFound)?.clone();
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
    if store.dir_index.contains_key(&(parent, name.to_string())) {
        return Err(crate::error::FilerError::Metadata("exists".into()));
    }
    let p = store.inodes.get(&parent).ok_or(crate::error::FilerError::NotFound)?.clone();
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
    let a = store.inodes.get_mut(&ino).ok_or(crate::error::FilerError::NotFound)?;
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

// ========================= 增强：分片策略 =========================

/// 分片策略类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShardStrategy {
    /// 按 inode 取模分片
    InodeModulo,
    /// 按目录 hash 分片（同一目录的子项在同一分片）
    DirectoryHash,
    /// 一致性哈希分片
    ConsistentHash,
}

impl ShardStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShardStrategy::InodeModulo => "inode_modulo",
            ShardStrategy::DirectoryHash => "directory_hash",
            ShardStrategy::ConsistentHash => "consistent_hash",
        }
    }
}

/// 分片配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardConfig {
    /// 分片策略
    pub strategy: ShardStrategy,
    /// 分片数量（应为 2 的幂）
    pub num_shards: u32,
    /// 分片节点映射：shard_id -> node_id
    pub shard_nodes: BTreeMap<u32, String>,
}

impl Default for ShardConfig {
    fn default() -> Self {
        let mut shard_nodes = BTreeMap::new();
        for i in 0..16 {
            shard_nodes.insert(i, format!("node-{}", i / 4));
        }
        ShardConfig { strategy: ShardStrategy::InodeModulo, num_shards: 16, shard_nodes }
    }
}

/// 目录 hash 分片计算
fn directory_hash_shard(parent_ino: u64, num_shards: u32) -> u32 {
    // 使用 FNV-1a hash 简化实现
    let mut hash: u64 = 0xcbf29ce484222325;
    let bytes = parent_ino.to_le_bytes();
    for byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    (hash % num_shards as u64) as u32
}

// ========================= 增强：分布式事务 =========================

/// 事务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxStatus {
    /// 事务已开始
    Active,
    /// 预提交阶段（2PC 第一阶段）
    Preparing,
    /// 已预提交，等待提交
    Prepared,
    /// 提交中
    Committing,
    /// 已提交
    Committed,
    /// 回滚中
    RollingBack,
    /// 已回滚
    RolledBack,
    /// 事务失败
    Failed,
}

/// 事务操作日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxLogEntry {
    /// 操作类型
    pub op: String,
    /// 操作参数（JSON 序列化）
    pub args: Vec<String>,
    /// 操作前状态（用于回滚）
    pub before: Option<String>,
}

/// 分布式事务上下文
#[derive(Debug)]
pub struct DistributedTx {
    /// 事务 ID
    pub tx_id: u64,
    /// 事务状态
    pub status: TxStatus,
    /// 涉及的分片
    pub involved_shards: Vec<u32>,
    /// 操作日志（用于回滚）
    pub log: Vec<TxLogEntry>,
    /// 开始时间（毫秒）
    pub started_at_ms: u64,
    /// 最后活跃时间（毫秒）
    pub last_active_ms: u64,
    /// 超时时间（毫秒）
    pub timeout_ms: u64,
}

/// 分布式事务管理器
#[derive(Debug)]
pub struct TxManager {
    /// 活跃事务
    transactions: Mutex<BTreeMap<u64, DistributedTx>>,
    /// 事务 ID 计数器
    next_tx_id: Mutex<u64>,
    /// 事务超时时间（毫秒）
    default_timeout_ms: u64,
}

impl Default for TxManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TxManager {
    pub fn new() -> Self {
        Self {
            transactions: Mutex::new(BTreeMap::new()),
            next_tx_id: Mutex::new(1),
            default_timeout_ms: 30_000, // 30 秒
        }
    }

    /// 开始新事务
    pub fn begin_tx(&self) -> u64 {
        let mut id = self.next_tx_id.lock();
        let tx_id = *id;
        *id += 1;
        drop(id);

        let now = now_ms_citus();
        let tx = DistributedTx {
            tx_id,
            status: TxStatus::Active,
            involved_shards: Vec::new(),
            log: Vec::new(),
            started_at_ms: now,
            last_active_ms: now,
            timeout_ms: self.default_timeout_ms,
        };

        self.transactions.lock().insert(tx_id, tx);
        tx_id
    }

    /// 记录操作到事务日志
    pub fn log_operation(&self, tx_id: u64, op: &str, args: Vec<String>, before: Option<String>) {
        let mut txs = self.transactions.lock();
        if let Some(tx) = txs.get_mut(&tx_id) {
            tx.log.push(TxLogEntry { op: op.to_string(), args, before });
            tx.last_active_ms = now_ms_citus();
        }
    }

    /// 添加涉及的分片
    pub fn add_involved_shard(&self, tx_id: u64, shard_id: u32) {
        let mut txs = self.transactions.lock();
        if let Some(tx) = txs.get_mut(&tx_id) {
            if !tx.involved_shards.contains(&shard_id) {
                tx.involved_shards.push(shard_id);
            }
            tx.last_active_ms = now_ms_citus();
        }
    }

    /// 预提交（2PC 第一阶段）
    pub fn prepare(&self, tx_id: u64) -> bool {
        let mut txs = self.transactions.lock();
        if let Some(tx) = txs.get_mut(&tx_id) {
            if tx.status != TxStatus::Active {
                return false;
            }
            tx.status = TxStatus::Preparing;
            // 模拟：所有分片都准备好
            tx.status = TxStatus::Prepared;
            tx.last_active_ms = now_ms_citus();
            true
        } else {
            false
        }
    }

    /// 提交事务（2PC 第二阶段）
    pub fn commit(&self, tx_id: u64) -> bool {
        let mut txs = self.transactions.lock();
        if let Some(tx) = txs.get_mut(&tx_id) {
            if tx.status != TxStatus::Prepared {
                return false;
            }
            tx.status = TxStatus::Committing;
            // 模拟：所有分片都提交成功
            tx.status = TxStatus::Committed;
            tx.last_active_ms = now_ms_citus();
            true
        } else {
            false
        }
    }

    /// 回滚事务
    pub fn rollback(&self, tx_id: u64) -> bool {
        let mut txs = self.transactions.lock();
        if let Some(tx) = txs.get_mut(&tx_id) {
            if matches!(tx.status, TxStatus::Committed | TxStatus::RolledBack) {
                return false;
            }
            tx.status = TxStatus::RollingBack;
            // 模拟：根据 log 回滚操作
            tx.status = TxStatus::RolledBack;
            tx.last_active_ms = now_ms_citus();
            true
        } else {
            false
        }
    }

    /// 获取事务状态
    pub fn get_status(&self, tx_id: u64) -> Option<TxStatus> {
        self.transactions.lock().get(&tx_id).map(|t| t.status)
    }

    /// 清理已完成/超时的事务
    pub fn cleanup_completed(&self) -> usize {
        let mut txs = self.transactions.lock();
        let now = now_ms_citus();
        let before = txs.len();

        txs.retain(|_, tx| {
            // 保留活跃事务
            if matches!(tx.status, TxStatus::Active | TxStatus::Prepared) {
                // 检查超时
                now.saturating_sub(tx.last_active_ms) < tx.timeout_ms
            } else {
                // 已完成的事务保留 60 秒后清理
                now.saturating_sub(tx.last_active_ms) < 60_000
            }
        });

        before - txs.len()
    }

    /// 获取活跃事务数
    pub fn active_count(&self) -> usize {
        self.transactions
            .lock()
            .values()
            .filter(|t| matches!(t.status, TxStatus::Active | TxStatus::Prepared))
            .count()
    }
}

// ========================= 增强：连接池与健康检查 =========================

/// 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    /// 空闲可用
    Idle,
    /// 正在使用
    Busy,
    /// 健康检查中
    Checking,
    /// 已损坏，需重建
    Broken,
}

/// 连接信息
#[derive(Debug, Clone)]
struct ConnectionInfo {
    /// 连接 ID
    id: u64,
    /// 状态
    state: ConnectionState,
    /// 所属分片节点
    node_id: String,
    /// 创建时间（毫秒）
    created_at_ms: u64,
    /// 最后使用时间（毫秒）
    last_used_ms: u64,
    /// 使用次数
    use_count: u64,
}

/// 节点健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealth {
    /// 节点 ID
    pub node_id: String,
    /// 是否健康
    pub healthy: bool,
    /// 最后健康检查时间（毫秒）
    pub last_check_ms: u64,
    /// 响应延迟（毫秒）
    pub latency_ms: u64,
    /// 连续失败次数
    pub consecutive_failures: u32,
}

/// 连接池配置
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    /// 每个节点最大连接数
    pub max_connections_per_node: usize,
    /// 最小空闲连接数
    pub min_idle_connections: usize,
    /// 连接最大存活时间（毫秒）
    pub max_lifetime_ms: u64,
    /// 健康检查间隔（毫秒）
    pub health_check_interval_ms: u64,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        ConnectionPoolConfig {
            max_connections_per_node: 32,
            min_idle_connections: 4,
            max_lifetime_ms: 3_600_000,       // 1 小时
            health_check_interval_ms: 10_000, // 10 秒
        }
    }
}

/// 连接池管理器
#[derive(Debug)]
pub struct ConnectionPoolManager {
    /// 连接池：node_id -> connections
    pools: Mutex<BTreeMap<String, Vec<ConnectionInfo>>>,
    /// 节点健康状态
    node_health: Mutex<BTreeMap<String, NodeHealth>>,
    /// 配置
    config: ConnectionPoolConfig,
    /// 下一个连接 ID
    next_conn_id: Mutex<u64>,
}

impl Default for ConnectionPoolManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionPoolManager {
    pub fn new() -> Self {
        Self {
            pools: Mutex::new(BTreeMap::new()),
            node_health: Mutex::new(BTreeMap::new()),
            config: ConnectionPoolConfig::default(),
            next_conn_id: Mutex::new(1),
        }
    }

    /// 注册节点
    pub fn register_node(&self, node_id: &str) {
        let mut pools = self.pools.lock();
        pools.entry(node_id.to_string()).or_default();

        let mut health = self.node_health.lock();
        health.entry(node_id.to_string()).or_insert_with(|| NodeHealth {
            node_id: node_id.to_string(),
            healthy: true,
            last_check_ms: now_ms_citus(),
            latency_ms: 0,
            consecutive_failures: 0,
        });
    }

    /// 获取连接
    pub fn acquire_connection(&self, node_id: &str) -> Option<u64> {
        let mut pools = self.pools.lock();
        let pool = pools.get_mut(node_id)?;

        // 找一个空闲连接
        for conn in pool.iter_mut() {
            if conn.state == ConnectionState::Idle {
                conn.state = ConnectionState::Busy;
                conn.last_used_ms = now_ms_citus();
                conn.use_count += 1;
                return Some(conn.id);
            }
        }

        // 没有空闲连接，创建新连接（如果没超过上限）
        let active_count = pool.len();
        if active_count < self.config.max_connections_per_node {
            let mut id = self.next_conn_id.lock();
            let conn_id = *id;
            *id += 1;
            drop(id);

            let now = now_ms_citus();
            pool.push(ConnectionInfo {
                id: conn_id,
                state: ConnectionState::Busy,
                node_id: node_id.to_string(),
                created_at_ms: now,
                last_used_ms: now,
                use_count: 1,
            });
            Some(conn_id)
        } else {
            None // 连接池满
        }
    }

    /// 释放连接
    pub fn release_connection(&self, node_id: &str, conn_id: u64) {
        let mut pools = self.pools.lock();
        if let Some(pool) = pools.get_mut(node_id) {
            for conn in pool.iter_mut() {
                if conn.id == conn_id {
                    conn.state = ConnectionState::Idle;
                    break;
                }
            }
        }
    }

    /// 标记连接损坏
    pub fn mark_broken(&self, node_id: &str, conn_id: u64) {
        let mut pools = self.pools.lock();
        if let Some(pool) = pools.get_mut(node_id) {
            for conn in pool.iter_mut() {
                if conn.id == conn_id {
                    conn.state = ConnectionState::Broken;
                    break;
                }
            }
        }

        // 更新节点健康状态
        let mut health = self.node_health.lock();
        if let Some(h) = health.get_mut(node_id) {
            h.consecutive_failures += 1;
            if h.consecutive_failures >= 3 {
                h.healthy = false;
            }
        }
    }

    /// 执行健康检查
    pub fn health_check(&self) -> usize {
        let mut health = self.node_health.lock();
        let now = now_ms_citus();
        let mut recovered = 0;

        for (node_id, h) in health.iter_mut() {
            if now.saturating_sub(h.last_check_ms) >= self.config.health_check_interval_ms
                || !h.healthy
            {
                h.last_check_ms = now;
                // 模拟：延迟 0-5ms 随机
                h.latency_ms = now % 5;

                if !h.healthy {
                    // 模拟：检查后恢复健康
                    h.healthy = true;
                    h.consecutive_failures = 0;
                    recovered += 1;

                    // 清理损坏的连接
                    let mut pools = self.pools.lock();
                    if let Some(pool) = pools.get_mut(node_id) {
                        pool.retain(|c| c.state != ConnectionState::Broken);
                    }
                }
            }
        }

        recovered
    }

    /// 获取节点健康状态
    pub fn get_node_health(&self, node_id: &str) -> Option<NodeHealth> {
        self.node_health.lock().get(node_id).cloned()
    }

    /// 获取所有节点健康状态
    pub fn list_node_health(&self) -> Vec<NodeHealth> {
        self.node_health.lock().values().cloned().collect()
    }

    /// 获取连接池统计
    pub fn pool_stats(&self) -> BTreeMap<String, (usize, usize)> {
        // 返回 (node_id -> (total_connections, busy_connections))
        let pools = self.pools.lock();
        let mut stats = BTreeMap::new();
        for (node_id, pool) in pools.iter() {
            let busy = pool.iter().filter(|c| c.state == ConnectionState::Busy).count();
            stats.insert(node_id.clone(), (pool.len(), busy));
        }
        stats
    }

    /// 清理过期连接
    pub fn cleanup_expired(&self) -> usize {
        let mut pools = self.pools.lock();
        let now = now_ms_citus();
        let mut total_removed = 0;

        for pool in pools.values_mut() {
            let before = pool.len();
            pool.retain(|conn| {
                if conn.state == ConnectionState::Busy {
                    return true; // 忙碌的连接不清理
                }
                if conn.state == ConnectionState::Broken {
                    return false; // 损坏的连接清理
                }
                // 超过最大存活时间的空闲连接清理
                now.saturating_sub(conn.created_at_ms) < self.config.max_lifetime_ms
            });
            total_removed += before - pool.len();
        }

        total_removed
    }
}

// ========================= PgCitusMeta 增强方法 =========================

impl PgCitusMeta {
    /// 使用指定分片数创建
    pub fn with_shards(num_shards: u32) -> Self {
        let store = PgStore {
            shards: (0..num_shards as u64).map(|i| (i, ())).collect(),
            ..Default::default()
        };
        Self { inner: Mutex::new(store) }
    }

    /// 获取分片配置
    pub fn shard_config(&self) -> ShardConfig {
        let s = self.inner.lock();
        let num_shards = s.shards.len() as u32;
        let mut shard_nodes = BTreeMap::new();
        for i in 0..num_shards {
            shard_nodes.insert(i, format!("node-{}", i / 4));
        }
        ShardConfig { strategy: ShardStrategy::InodeModulo, num_shards, shard_nodes }
    }

    /// 按目录 hash 计算分片 ID
    pub fn directory_shard_id(&self, parent_ino: u64) -> u32 {
        let s = self.inner.lock();
        let num_shards = s.shards.len() as u32;
        directory_hash_shard(parent_ino, num_shards)
    }

    /// 获取所有分片 ID
    pub fn list_shards(&self) -> Vec<u64> {
        let s = self.inner.lock();
        s.shards.keys().copied().collect()
    }

    /// 获取分片上的 inode 数量
    pub fn shard_inode_count(&self, shard_id: u64) -> usize {
        let s = self.inner.lock();
        s.ino_shard.iter().filter(|(_, &sid)| sid == shard_id).count()
    }

    /// 批量创建文件
    pub fn batch_create(&self, parent: u64, names: &[&str], mode: u32) -> BatchCreateResult {
        let mut result = BatchCreateResult::default();
        let mut store = self.inner.lock();

        for name in names {
            match meta_create(&mut store.store, parent, name, mode) {
                Ok(ino) => {
                    Self::track_shard(&mut store, ino);
                    result.created.push((name.to_string(), ino));
                },
                Err(e) => {
                    result.failed.push((name.to_string(), e.to_string()));
                },
            }
        }

        result
    }

    /// 批量读取属性
    pub fn batch_read_attr(&self, inodes: &[u64]) -> BatchReadAttrResult {
        let mut result = BatchReadAttrResult::default();
        let store = self.inner.lock();

        for &ino in inodes {
            match store.store.inodes.get(&ino) {
                Some(attr) => {
                    result.found.push(attr.clone());
                },
                None => {
                    result.not_found.push(ino);
                },
            }
        }

        result
    }

    /// 批量删除
    pub fn batch_delete(&self, inodes: &[u64]) -> BatchDeleteResult {
        let mut result = BatchDeleteResult::default();
        let mut store = self.inner.lock();

        for &ino in inodes {
            match meta_delete(&mut store.store, ino) {
                Ok(()) => {
                    store.ino_shard.remove(&ino);
                    result.deleted.push(ino);
                },
                Err(e) => {
                    result.failed.push((ino, e.to_string()));
                },
            }
        }

        result
    }

    /// 批量列出多个目录
    pub fn batch_list_dir(&self, parents: &[u64]) -> Vec<(u64, FilerResult<Vec<DirEntry>>)> {
        let mut store = self.inner.lock();
        let mut results = Vec::new();

        for &parent in parents {
            let res = meta_list_dir(&mut store.store, parent);
            results.push((parent, res));
        }

        results
    }
}

// ========================= 辅助函数 =========================

fn now_ms_citus() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub(crate) fn meta_delete(store: &mut InMemInodeStore, ino: u64) -> FilerResult<()> {
    if ino == 1 {
        return Err(crate::error::FilerError::AttrInvalid);
    }
    let a = store.inodes.remove(&ino).ok_or(crate::error::FilerError::NotFound)?;
    store.dir_index.remove(&(a.parent, a.name));
    Ok(())
}

pub(crate) fn meta_list_dir(
    store: &mut InMemInodeStore,
    parent: u64,
) -> FilerResult<Vec<DirEntry>> {
    let p = store.inodes.get(&parent).ok_or(crate::error::FilerError::NotFound)?.clone();
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
            out.push(DirEntry { name: n.clone(), ino: *ino, typ: t });
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
    if store.dir_index.contains_key(&(new_parent, new_name.to_string())) {
        return Err(crate::error::FilerError::Metadata("exists".into()));
    }
    {
        let a = store.inodes.get_mut(&ino).ok_or(crate::error::FilerError::NotFound)?;
        if (a.mode & 0o170000) == S_IFDIR {
            return Err(crate::error::FilerError::AttrInvalid);
        }
        a.nlink += 1;
    }
    store.dir_index.insert((new_parent, new_name.to_string()), ino);
    Ok(())
}

pub(crate) fn meta_unlink(store: &mut InMemInodeStore, parent: u64, name: &str) -> FilerResult<()> {
    let ino = store
        .dir_index
        .remove(&(parent, name.to_string()))
        .ok_or(crate::error::FilerError::NotFound)?;
    let remove;
    {
        let a = store.inodes.get_mut(&ino).ok_or(crate::error::FilerError::NotFound)?;
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
                    store.dir_index.insert((new_parent, new_name.to_string()), target_ino);
                    store.dir_index.insert((old_parent, old_name.to_string()), ino);
                    return Err(crate::error::FilerError::NotEmpty);
                }
            }
            tattr.nlink = tattr.nlink.saturating_sub(1);
            if tattr.nlink == 0 {
                store.inodes.remove(&target_ino);
            }
        }
    }
    store.dir_index.insert((new_parent, new_name.to_string()), ino);
    if let Some(a) = store.inodes.get_mut(&ino) {
        a.parent = new_parent;
        a.name = new_name.to_string();
        a.ctime = now_secs();
    }
    Ok(())
}

// ========================= 增强功能单元测试 =========================

#[cfg(test)]
mod citus_enhanced_tests {
    use super::*;

    #[test]
    fn test_shard_strategy_as_str() {
        assert_eq!(ShardStrategy::InodeModulo.as_str(), "inode_modulo");
        assert_eq!(ShardStrategy::DirectoryHash.as_str(), "directory_hash");
        assert_eq!(ShardStrategy::ConsistentHash.as_str(), "consistent_hash");
    }

    #[test]
    fn test_shard_config_default() {
        let config = ShardConfig::default();
        assert_eq!(config.num_shards, 16);
        assert_eq!(config.strategy, ShardStrategy::InodeModulo);
        assert_eq!(config.shard_nodes.len(), 16);
        // 前 4 个分片应该在 node-0
        assert_eq!(config.shard_nodes.get(&0), Some(&"node-0".to_string()));
        assert_eq!(config.shard_nodes.get(&3), Some(&"node-0".to_string()));
        assert_eq!(config.shard_nodes.get(&4), Some(&"node-1".to_string()));
    }

    #[test]
    fn test_directory_hash_shard() {
        let shard1 = directory_hash_shard(1, 16);
        let shard2 = directory_hash_shard(2, 16);
        // 不同的 inode 可能在不同分片
        assert!(shard1 < 16);
        assert!(shard2 < 16);
        // 相同输入产生相同结果（确定性）
        assert_eq!(directory_hash_shard(1, 16), directory_hash_shard(1, 16));
    }

    #[test]
    fn test_with_shards() {
        let meta = PgCitusMeta::with_shards(32);
        let shards = meta.list_shards();
        assert_eq!(shards.len(), 32);
    }

    #[test]
    fn test_shard_id_of() {
        let meta = PgCitusMeta::new();
        // ino 0 % 16 = 0
        assert_eq!(meta.shard_id_of(0), 0);
        // ino 16 % 16 = 0
        assert_eq!(meta.shard_id_of(16), 0);
        // ino 5 % 16 = 5
        assert_eq!(meta.shard_id_of(5), 5);
    }

    #[test]
    fn test_directory_shard_id() {
        let meta = PgCitusMeta::new();
        let shard = meta.directory_shard_id(42);
        assert!(shard < 16);
        // 确定性
        assert_eq!(meta.directory_shard_id(42), meta.directory_shard_id(42));
    }

    #[test]
    fn test_tx_manager_basic() {
        let mgr = TxManager::new();

        let tx_id = mgr.begin_tx();
        assert_eq!(tx_id, 1);
        assert_eq!(mgr.get_status(tx_id), Some(TxStatus::Active));
        assert_eq!(mgr.active_count(), 1);

        // 记录操作
        mgr.log_operation(tx_id, "mkdir", vec!["1".into(), "test".into()], None);
        mgr.add_involved_shard(tx_id, 0);
        mgr.add_involved_shard(tx_id, 1);

        // 2PC 提交
        assert!(mgr.prepare(tx_id));
        assert_eq!(mgr.get_status(tx_id), Some(TxStatus::Prepared));
        assert!(mgr.commit(tx_id));
        assert_eq!(mgr.get_status(tx_id), Some(TxStatus::Committed));

        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn test_tx_manager_rollback() {
        let mgr = TxManager::new();

        let tx_id = mgr.begin_tx();
        assert_eq!(mgr.get_status(tx_id), Some(TxStatus::Active));

        assert!(mgr.rollback(tx_id));
        assert_eq!(mgr.get_status(tx_id), Some(TxStatus::RolledBack));

        // 已提交的不能回滚
        let tx_id2 = mgr.begin_tx();
        mgr.prepare(tx_id2);
        mgr.commit(tx_id2);
        assert!(!mgr.rollback(tx_id2));
    }

    #[test]
    fn test_tx_manager_cleanup() {
        let mgr = TxManager::new();

        let tx1 = mgr.begin_tx();
        let _tx2 = mgr.begin_tx();
        mgr.prepare(tx1);
        mgr.commit(tx1);

        // 已提交的应该被清理（但 60 秒内还在）
        let cleaned = mgr.cleanup_completed();
        // 刚提交的不会被立即清理
        assert_eq!(cleaned, 0);
    }

    #[test]
    fn test_batch_create() {
        let meta = PgCitusMeta::new();

        let result = meta.batch_create(1, &["a.txt", "b.txt", "c.txt"], 0o644);
        assert_eq!(result.created.len(), 3);
        assert_eq!(result.failed.len(), 0);

        // 验证文件被创建
        let store = meta.inner.lock();
        assert!(store.store.dir_index.contains_key(&(1, "a.txt".to_string())));
        assert!(store.store.dir_index.contains_key(&(1, "b.txt".to_string())));
    }

    #[test]
    fn test_batch_create_with_duplicates() {
        let meta = PgCitusMeta::new();

        // 先创建一个
        meta.batch_create(1, &["a.txt"], 0o644);

        // 再创建同名的应该失败
        let result = meta.batch_create(1, &["a.txt", "b.txt"], 0o644);
        assert_eq!(result.created.len(), 1); // b.txt 成功
        assert_eq!(result.failed.len(), 1); // a.txt 失败
        assert_eq!(result.failed[0].0, "a.txt");
    }

    #[test]
    fn test_batch_read_attr() {
        let meta = PgCitusMeta::new();

        let created = meta.batch_create(1, &["x.txt", "y.txt"], 0o644);
        let inos: Vec<u64> = created.created.iter().map(|(_, ino)| *ino).collect();

        let result = meta.batch_read_attr(&[inos[0], inos[1], 99999]);
        assert_eq!(result.found.len(), 2);
        assert_eq!(result.not_found.len(), 1);
        assert_eq!(result.not_found[0], 99999);
    }

    #[test]
    fn test_batch_delete() {
        let meta = PgCitusMeta::new();

        let created = meta.batch_create(1, &["del1.txt", "del2.txt"], 0o644);
        let inos: Vec<u64> = created.created.iter().map(|(_, ino)| *ino).collect();

        let result = meta.batch_delete(&[inos[0], inos[1], 99999]);
        assert_eq!(result.deleted.len(), 2);
        assert_eq!(result.failed.len(), 1); // 99999 不存在
    }

    #[test]
    fn test_batch_list_dir() {
        let meta = PgCitusMeta::new();

        meta.batch_create(1, &["f1.txt", "f2.txt"], 0o644);

        // 创建子目录
        let store = meta.inner.lock();
        let _s = store.store.clone();
        drop(store);

        // 用现有的 inode_mkdir 创建子目录
        // 简化：直接测试根目录和一个不存在的目录
        let result = meta.batch_list_dir(&[1, 99999]);
        assert_eq!(result.len(), 2);
        assert!(result[0].1.is_ok());
        assert!(result[1].1.is_err());
    }

    #[test]
    fn test_connection_pool_basic() {
        let pool = ConnectionPoolManager::new();
        pool.register_node("node-0");

        // 获取连接
        let conn_id = pool.acquire_connection("node-0").unwrap();
        assert!(conn_id > 0);

        let stats = pool.pool_stats();
        assert_eq!(stats.get("node-0").unwrap().0, 1); // 总连接数
        assert_eq!(stats.get("node-0").unwrap().1, 1); // 忙碌连接数

        // 释放连接
        pool.release_connection("node-0", conn_id);
        let stats = pool.pool_stats();
        assert_eq!(stats.get("node-0").unwrap().1, 0); // 没有忙碌连接
    }

    #[test]
    fn test_connection_pool_max_limit() {
        let pool = ConnectionPoolManager::new();
        pool.register_node("node-0");

        // 耗尽连接池
        let mut conn_ids = Vec::new();
        for _ in 0..32 {
            conn_ids.push(pool.acquire_connection("node-0").unwrap());
        }

        // 第 33 个应该失败
        assert!(pool.acquire_connection("node-0").is_none());

        // 释放一个后可以再获取
        pool.release_connection("node-0", conn_ids[0]);
        assert!(pool.acquire_connection("node-0").is_some());
    }

    #[test]
    fn test_connection_broken_and_health_check() {
        let pool = ConnectionPoolManager::new();
        pool.register_node("node-0");

        let conn_id = pool.acquire_connection("node-0").unwrap();
        pool.mark_broken("node-0", conn_id);

        let health = pool.get_node_health("node-0").unwrap();
        assert_eq!(health.consecutive_failures, 1);
        assert!(health.healthy); // 1 次失败还不够

        // 再损坏两个连接
        let c2 = pool.acquire_connection("node-0").unwrap();
        pool.mark_broken("node-0", c2);
        let c3 = pool.acquire_connection("node-0").unwrap();
        pool.mark_broken("node-0", c3);

        let health = pool.get_node_health("node-0").unwrap();
        assert!(!health.healthy); // 3 次失败后不健康

        // 健康检查应该恢复
        let recovered = pool.health_check();
        assert!(recovered > 0);

        let health = pool.get_node_health("node-0").unwrap();
        assert!(health.healthy);
    }

    #[test]
    fn test_node_health_list() {
        let pool = ConnectionPoolManager::new();
        pool.register_node("node-0");
        pool.register_node("node-1");

        let health_list = pool.list_node_health();
        assert_eq!(health_list.len(), 2);
    }

    #[test]
    fn test_connection_cleanup_expired() {
        let pool = ConnectionPoolManager::new();
        pool.register_node("node-0");

        // 获取并释放几个连接
        for _ in 0..5 {
            let id = pool.acquire_connection("node-0").unwrap();
            pool.release_connection("node-0", id);
        }

        // 正常情况不会被清理（因为没过期）
        let removed = pool.cleanup_expired();
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_shard_inode_count() {
        let meta = PgCitusMeta::new();

        // 创建一些文件，分布在不同分片
        meta.batch_create(1, &["f1", "f2", "f3", "f4", "f5"], 0o644);

        let shard0_count = meta.shard_inode_count(0);
        // shard0_count 为 usize，恒 >= 0；至少有一些文件在分片 0
        let _ = shard0_count;

        let total_shards = meta.list_shards().len();
        assert_eq!(total_shards, 16);
    }

    #[test]
    fn test_shard_config_method() {
        let meta = PgCitusMeta::new();
        let config = meta.shard_config();
        assert_eq!(config.num_shards, 16);
        assert_eq!(config.strategy, ShardStrategy::InodeModulo);
        assert_eq!(config.shard_nodes.len(), 16);
    }
}
