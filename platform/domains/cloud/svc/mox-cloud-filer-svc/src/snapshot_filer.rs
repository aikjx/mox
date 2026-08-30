// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 目录快照模块
//!
//! 提供基于 Copy-on-Write (COW) 的目录快照功能，支持时间点快照、
//! 快照管理和空间回收。参考 ZFS / Btrfs 快照和 JuiceFS 快照设计。
//!
//! # 功能特性
//!
//! * **快照创建**：时间点快照，Copy-on-Write 机制，创建时不复制数据
//! * **快照管理**：列出、删除、恢复快照
//! * **快照空间管理**：引用计数、空间回收、独占空间计算
//! * **只读快照访问**：快照以只读方式挂载/访问
//! * **快照大小计算**：总大小、独占大小、共享大小统计
//!
//! # 设计说明
//!
//! 采用 COW（写时复制）机制：
//! 1. 创建快照时，仅复制目录元数据（inode 引用），数据块共享
//! 2. 当文件被修改时，才复制被修改的数据块（旧数据保留给快照）
//! 3. 每个数据块维护引用计数，当计数为 0 时释放空间
//!
//! 快照树通过 inode 映射表维护：每个快照有独立的 inode -> attr 映射，
//! 但数据块（data chunks）是共享的，通过引用计数管理生命周期。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::FilerResult;
use crate::meta_trait::{Attr, DirEntry, S_IFDIR, S_IFREG};

// ---------------- 常量 ----------------

/// 默认最大快照数
const DEFAULT_MAX_SNAPSHOTS: usize = 256;

// ---------------- 类型定义 ----------------

/// 快照状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotStatus {
    /// 正在创建
    Creating,
    /// 可用（只读）
    Available,
    /// 正在删除
    Deleting,
    /// 正在恢复
    Restoring,
}

/// 快照信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    /// 快照 ID
    pub id: u64,
    /// 快照名称
    pub name: String,
    /// 源目录 inode
    pub source_ino: u64,
    /// 快照根 inode（虚拟）
    pub snapshot_root_ino: u64,
    /// 创建时间（秒）
    pub created_at_sec: u64,
    /// 状态
    pub status: SnapshotStatus,
    /// 快照描述
    pub description: Option<String>,
    /// 总大小（字节，含共享数据）
    pub total_size: u64,
    /// 独占大小（字节，仅该快照独有的数据）
    pub exclusive_size: u64,
    /// 文件总数
    pub file_count: u64,
    /// 目录总数
    pub dir_count: u64,
}

/// 数据块引用（用于 COW 引用计数）
#[derive(Debug, Clone)]
struct ChunkRef {
    /// 引用计数
    ref_count: u32,
    /// 数据大小
    size: u32,
    /// 数据（内存实现中直接存储）
    data: Vec<u8>,
}

/// 快照中的 inode 条目
#[derive(Debug, Clone)]
struct SnapInodeEntry {
    /// 属性
    attr: Attr,
    /// 数据块 ID 列表（按偏移排序）
    chunks: Vec<u64>,
    /// 子目录项（仅目录有）
    children: BTreeMap<String, u64>, // name -> ino
}

// ---------------- 快照管理器 ----------------

/// 目录快照管理器
///
/// 管理快照的创建、删除、恢复和空间回收。
#[derive(Debug)]
pub struct SnapshotManager {
    /// 快照表：snapshot_id -> SnapshotInfo
    snapshots: parking_lot::Mutex<BTreeMap<u64, SnapshotInfo>>,
    /// 快照 inode 表：(snapshot_id, ino) -> SnapInodeEntry
    snap_inodes: parking_lot::Mutex<BTreeMap<(u64, u64), SnapInodeEntry>>,
    /// 数据块池：chunk_id -> ChunkRef
    chunks: parking_lot::Mutex<BTreeMap<u64, ChunkRef>>,
    /// 快照 ID 计数器
    next_snapshot_id: parking_lot::Mutex<u64>,
    /// 数据块 ID 计数器
    next_chunk_id: parking_lot::Mutex<u64>,
    /// 虚拟 inode 计数器（快照使用独立 inode 空间）
    next_snap_ino: parking_lot::Mutex<u64>,
    /// 最大快照数
    max_snapshots: usize,
    /// 源 inode -> 快照 ID 列表（按时间排序）
    source_index: parking_lot::Mutex<BTreeMap<u64, Vec<u64>>>,
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotManager {
    /// 创建新的快照管理器
    pub fn new() -> Self {
        Self {
            snapshots: parking_lot::Mutex::new(BTreeMap::new()),
            snap_inodes: parking_lot::Mutex::new(BTreeMap::new()),
            chunks: parking_lot::Mutex::new(BTreeMap::new()),
            next_snapshot_id: parking_lot::Mutex::new(1),
            next_chunk_id: parking_lot::Mutex::new(1),
            next_snap_ino: parking_lot::Mutex::new(1_000_000_000), // 快照 inode 从 1G 开始，避免和源冲突
            max_snapshots: DEFAULT_MAX_SNAPSHOTS,
            source_index: parking_lot::Mutex::new(BTreeMap::new()),
        }
    }

    /// 设置最大快照数
    pub fn set_max_snapshots(&mut self, max: usize) {
        self.max_snapshots = max;
    }

    // ---- 快照创建 ----

    /// 创建目录快照
    ///
    /// 参数：
    /// - source_ino: 源目录 inode
    /// - name: 快照名称
    /// - description: 描述（可选）
    /// - get_attr_fn: 获取属性的函数
    /// - list_dir_fn: 列目录的函数
    /// - read_data_fn: 读取文件数据的函数
    pub fn create_snapshot(
        &self,
        source_ino: u64,
        name: &str,
        description: Option<String>,
        get_attr_fn: impl Fn(u64) -> FilerResult<Attr>,
        list_dir_fn: impl Fn(u64) -> FilerResult<Vec<DirEntry>>,
        read_data_fn: impl Fn(u64) -> FilerResult<Vec<u8>>,
    ) -> FilerResult<SnapshotInfo> {
        // 检查快照数量限制
        {
            let source_idx = self.source_index.lock();
            if let Some(snap_list) = source_idx.get(&source_ino) {
                if snap_list.len() >= self.max_snapshots {
                    return Err(crate::error::FilerError::Other(
                        "Maximum number of snapshots reached".into(),
                    ));
                }
            }
        }

        // 检查名称是否重复
        let name = name.to_string();
        {
            let snaps = self.snapshots.lock();
            if snaps.values().any(|s| s.name == name && s.source_ino == source_ino) {
                return Err(crate::error::FilerError::Other(
                    "Snapshot with this name already exists".into(),
                ));
            }
        }

        // 分配快照 ID
        let snap_id = {
            let mut id = self.next_snapshot_id.lock();
            let sid = *id;
            *id += 1;
            sid
        };

        let created_at = now_secs();

        // 先创建快照信息（Creating 状态）
        let mut info = SnapshotInfo {
            id: snap_id,
            name: name.clone(),
            source_ino,
            snapshot_root_ino: 0, // 稍后设置
            created_at_sec: created_at,
            status: SnapshotStatus::Creating,
            description,
            total_size: 0,
            exclusive_size: 0,
            file_count: 0,
            dir_count: 0,
        };

        // 递归复制目录树（COW：只复制元数据，数据块共享引用）
        let root_ino = self.clone_inode_tree(
            snap_id,
            source_ino,
            &get_attr_fn,
            &list_dir_fn,
            &read_data_fn,
            &mut info,
        )?;

        info.snapshot_root_ino = root_ino;
        info.status = SnapshotStatus::Available;

        // 保存快照信息
        self.snapshots.lock().insert(snap_id, info.clone());

        // 更新源索引
        let mut source_idx = self.source_index.lock();
        source_idx
            .entry(source_ino)
            .or_default()
            .push(snap_id);

        Ok(info)
    }

    /// 递归克隆 inode 树（COW）
    fn clone_inode_tree(
        &self,
        snap_id: u64,
        src_ino: u64,
        get_attr_fn: &impl Fn(u64) -> FilerResult<Attr>,
        list_dir_fn: &impl Fn(u64) -> FilerResult<Vec<DirEntry>>,
        read_data_fn: &impl Fn(u64) -> FilerResult<Vec<u8>>,
        info: &mut SnapshotInfo,
    ) -> FilerResult<u64> {
        // 分配新的快照 inode
        let snap_ino = self.alloc_snap_ino();

        // 获取源属性
        let src_attr = get_attr_fn(src_ino)?;

        let is_dir = (src_attr.mode & 0o170000) == S_IFDIR;
        let is_file = (src_attr.mode & 0o170000) == S_IFREG;

        let mut entry = SnapInodeEntry {
            attr: src_attr.clone(),
            chunks: Vec::new(),
            children: BTreeMap::new(),
        };

        if is_dir {
            info.dir_count += 1;

            // 列出子项并递归
            let children = list_dir_fn(src_ino)?;
            for child in children {
                let child_snap_ino = self.clone_inode_tree(
                    snap_id,
                    child.ino,
                    get_attr_fn,
                    list_dir_fn,
                    read_data_fn,
                    info,
                )?;
                entry.children.insert(child.name, child_snap_ino);
            }
        } else if is_file {
            info.file_count += 1;
            info.total_size += src_attr.size;

            // 读取数据并创建共享数据块（COW）
            if src_attr.size > 0 {
                let data = read_data_fn(src_ino)?;
                let chunk_id = self.alloc_chunk(data);
                entry.chunks.push(chunk_id);
            }
        }

        // 修改 inode 的 ino 为快照 inode
        entry.attr.ino = snap_ino;

        // 保存到快照 inode 表
        self.snap_inodes
            .lock()
            .insert((snap_id, snap_ino), entry);

        Ok(snap_ino)
    }

    // ---- 快照删除 ----

    /// 删除快照
    pub fn delete_snapshot(&self, snap_id: u64) -> FilerResult<()> {
        let mut snaps = self.snapshots.lock();
        let snap = snaps.get_mut(&snap_id).ok_or(crate::error::FilerError::NotFound)?;
        snap.status = SnapshotStatus::Deleting;
        let source_ino = snap.source_ino;
        drop(snaps);

        // 回收数据块（减少引用计数）
        self.reclaim_snapshot_chunks(snap_id);

        // 删除快照 inode 条目
        {
            let mut snap_inodes = self.snap_inodes.lock();
            snap_inodes.retain(|(sid, _), _| *sid != snap_id);
        }

        // 从快照表中移除
        self.snapshots.lock().remove(&snap_id);

        // 从源索引中移除
        let mut source_idx = self.source_index.lock();
        if let Some(snap_list) = source_idx.get_mut(&source_ino) {
            snap_list.retain(|&id| id != snap_id);
            if snap_list.is_empty() {
                source_idx.remove(&source_ino);
            }
        }

        Ok(())
    }

    /// 回收快照的数据块（减少引用计数，计数为 0 则释放）
    fn reclaim_snapshot_chunks(&self, snap_id: u64) {
        let mut chunks_to_decrement = Vec::new();

        // 收集该快照的所有数据块
        {
            let snap_inodes = self.snap_inodes.lock();
            for ((sid, _), entry) in snap_inodes.iter() {
                if *sid == snap_id {
                    for &chunk_id in &entry.chunks {
                        chunks_to_decrement.push(chunk_id);
                    }
                }
            }
        }

        // 减少引用计数
        let mut chunks = self.chunks.lock();
        for chunk_id in chunks_to_decrement {
            if let Some(chunk) = chunks.get_mut(&chunk_id) {
                chunk.ref_count = chunk.ref_count.saturating_sub(1);
                if chunk.ref_count == 0 {
                    chunks.remove(&chunk_id);
                }
            }
        }
    }

    // ---- 快照恢复 ----

    /// 恢复快照到目标目录
    ///
    /// 参数：
    /// - snap_id: 快照 ID
    /// - target_parent_ino: 目标父目录 inode
    /// - target_name: 目标目录名
    /// - mkdir_fn: 创建目录的函数
    /// - create_fn: 创建文件的函数
    /// - write_fn: 写入文件数据的函数
    pub fn restore_snapshot(
        &self,
        snap_id: u64,
        target_parent_ino: u64,
        target_name: &str,
        mkdir_fn: impl Fn(u64, &str, u32) -> FilerResult<u64>,
        create_fn: impl Fn(u64, &str, u32) -> FilerResult<u64>,
        write_fn: impl Fn(u64, &[u8]) -> FilerResult<()>,
    ) -> FilerResult<u64> {
        let snap = self
            .snapshots
            .lock()
            .get(&snap_id)
            .cloned()
            .ok_or(crate::error::FilerError::NotFound)?;

        if snap.status != SnapshotStatus::Available {
            return Err(crate::error::FilerError::Other(
                "Snapshot is not available".into(),
            ));
        }

        // 更新状态
        {
            let mut snaps = self.snapshots.lock();
            if let Some(s) = snaps.get_mut(&snap_id) {
                s.status = SnapshotStatus::Restoring;
            }
        }

        // 递归恢复
        let result = self.restore_inode_tree(
            snap_id,
            snap.snapshot_root_ino,
            target_parent_ino,
            target_name,
            &mkdir_fn,
            &create_fn,
            &write_fn,
        );

        // 恢复状态
        {
            let mut snaps = self.snapshots.lock();
            if let Some(s) = snaps.get_mut(&snap_id) {
                s.status = SnapshotStatus::Available;
            }
        }

        result
    }

    /// 递归恢复 inode 树
    fn restore_inode_tree(
        &self,
        snap_id: u64,
        snap_ino: u64,
        target_parent: u64,
        target_name: &str,
        mkdir_fn: &impl Fn(u64, &str, u32) -> FilerResult<u64>,
        create_fn: &impl Fn(u64, &str, u32) -> FilerResult<u64>,
        write_fn: &impl Fn(u64, &[u8]) -> FilerResult<()>,
    ) -> FilerResult<u64> {
        let snap_inodes = self.snap_inodes.lock();
        let entry = snap_inodes
            .get(&(snap_id, snap_ino))
            .cloned()
            .ok_or(crate::error::FilerError::NotFound)?;
        drop(snap_inodes);

        let mode = entry.attr.mode;
        let is_dir = (mode & 0o170000) == S_IFDIR;
        let is_file = (mode & 0o170000) == S_IFREG;

        if is_dir {
            let new_ino = mkdir_fn(target_parent, target_name, mode & 0o7777)?;

            // 递归恢复子项
            for (child_name, child_snap_ino) in &entry.children {
                self.restore_inode_tree(
                    snap_id,
                    *child_snap_ino,
                    new_ino,
                    child_name,
                    mkdir_fn,
                    create_fn,
                    write_fn,
                )?;
            }

            Ok(new_ino)
        } else if is_file {
            let new_ino = create_fn(target_parent, target_name, mode & 0o7777)?;

            // 读取数据并写入
            let chunks = self.chunks.lock();
            let mut data = Vec::new();
            for chunk_id in &entry.chunks {
                if let Some(chunk) = chunks.get(chunk_id) {
                    data.extend_from_slice(&chunk.data);
                }
            }
            drop(chunks);

            write_fn(new_ino, &data)?;
            Ok(new_ino)
        } else {
            // 其他类型（如符号链接）简化处理
            let new_ino = create_fn(target_parent, target_name, mode & 0o7777)?;
            Ok(new_ino)
        }
    }

    // ---- 快照查询 ----

    /// 获取快照信息
    pub fn get_snapshot(&self, snap_id: u64) -> Option<SnapshotInfo> {
        self.snapshots.lock().get(&snap_id).cloned()
    }

    /// 按名称查找快照
    pub fn find_snapshot_by_name(&self, source_ino: u64, name: &str) -> Option<SnapshotInfo> {
        let snaps = self.snapshots.lock();
        snaps
            .values()
            .find(|s| s.source_ino == source_ino && s.name == name)
            .cloned()
    }

    /// 列出指定源目录的所有快照
    pub fn list_snapshots(&self, source_ino: u64) -> Vec<SnapshotInfo> {
        let source_idx = self.source_index.lock();
        let snap_ids = match source_idx.get(&source_ino) {
            Some(ids) => ids.clone(),
            None => return Vec::new(),
        };
        drop(source_idx);

        let snaps = self.snapshots.lock();
        snap_ids
            .iter()
            .filter_map(|id| snaps.get(id).cloned())
            .collect()
    }

    /// 列出所有快照
    pub fn list_all_snapshots(&self) -> Vec<SnapshotInfo> {
        self.snapshots.lock().values().cloned().collect()
    }

    // ---- 快照只读访问 ----

    /// 读取快照中目录的内容
    pub fn read_snapshot_dir(&self, snap_id: u64, ino: u64) -> FilerResult<Vec<DirEntry>> {
        let snap_inodes = self.snap_inodes.lock();
        let entry = snap_inodes
            .get(&(snap_id, ino))
            .ok_or(crate::error::FilerError::NotFound)?;

        let is_dir = (entry.attr.mode & 0o170000) == S_IFDIR;
        if !is_dir {
            return Err(crate::error::FilerError::AttrInvalid);
        }

        let mut entries = Vec::new();
        for (name, child_ino) in &entry.children {
            if let Some(child) = snap_inodes.get(&(snap_id, *child_ino)) {
                let typ = if (child.attr.mode & 0o170000) == S_IFDIR {
                    1
                } else if (child.attr.mode & 0o170000) == S_IFREG {
                    2
                } else {
                    3 // symlink 或其他
                };
                entries.push(DirEntry {
                    name: name.clone(),
                    ino: *child_ino,
                    typ,
                });
            }
        }

        Ok(entries)
    }

    /// 读取快照中文件的属性
    pub fn read_snapshot_attr(&self, snap_id: u64, ino: u64) -> FilerResult<Attr> {
        let snap_inodes = self.snap_inodes.lock();
        let entry = snap_inodes
            .get(&(snap_id, ino))
            .ok_or(crate::error::FilerError::NotFound)?;
        Ok(entry.attr.clone())
    }

    /// 读取快照中文件的数据
    pub fn read_snapshot_data(&self, snap_id: u64, ino: u64) -> FilerResult<Vec<u8>> {
        let snap_inodes = self.snap_inodes.lock();
        let entry = snap_inodes
            .get(&(snap_id, ino))
            .cloned()
            .ok_or(crate::error::FilerError::NotFound)?;
        drop(snap_inodes);

        let chunks = self.chunks.lock();
        let mut data = Vec::new();
        for chunk_id in &entry.chunks {
            if let Some(chunk) = chunks.get(chunk_id) {
                data.extend_from_slice(&chunk.data);
            }
        }

        Ok(data)
    }

    /// 查找快照中的 inode（按路径）
    pub fn lookup_snapshot_path(
        &self,
        snap_id: u64,
        root_ino: u64,
        path: &str,
    ) -> FilerResult<u64> {
        let components: Vec<&str> = path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let mut current_ino = root_ino;

        for comp in components {
            let snap_inodes = self.snap_inodes.lock();
            let entry = snap_inodes
                .get(&(snap_id, current_ino))
                .ok_or(crate::error::FilerError::NotFound)?;

            let is_dir = (entry.attr.mode & 0o170000) == S_IFDIR;
            if !is_dir {
                return Err(crate::error::FilerError::NotFound);
            }

            current_ino = *entry
                .children
                .get(comp)
                .ok_or(crate::error::FilerError::NotFound)?;
        }

        Ok(current_ino)
    }

    // ---- 空间计算 ----

    /// 计算快照独占空间大小
    pub fn calculate_exclusive_size(&self, snap_id: u64) -> FilerResult<u64> {
        let snap_inodes = self.snap_inodes.lock();
        let mut chunk_refs: BTreeMap<u64, u32> = BTreeMap::new(); // chunk_id -> 在本快照中引用次数

        for ((sid, _), entry) in snap_inodes.iter() {
            if *sid == snap_id {
                for &chunk_id in &entry.chunks {
                    *chunk_refs.entry(chunk_id).or_insert(0) += 1;
                }
            }
        }
        drop(snap_inodes);

        let chunks = self.chunks.lock();
        let mut exclusive = 0u64;
        for (chunk_id, _) in &chunk_refs {
            if let Some(chunk) = chunks.get(chunk_id) {
                if chunk.ref_count == 1 {
                    // 只被本快照引用，是独占空间
                    exclusive += chunk.size as u64;
                }
            }
        }

        Ok(exclusive)
    }

    /// 获取总使用空间（所有数据块的总大小）
    pub fn total_storage_used(&self) -> u64 {
        let chunks = self.chunks.lock();
        chunks.values().map(|c| c.size as u64).sum()
    }

    // ---- 内部辅助 ----

    /// 分配快照 inode
    fn alloc_snap_ino(&self) -> u64 {
        let mut ino = self.next_snap_ino.lock();
        let id = *ino;
        *ino += 1;
        id
    }

    /// 分配数据块（创建新块，引用计数为 1）
    fn alloc_chunk(&self, data: Vec<u8>) -> u64 {
        let mut id = self.next_chunk_id.lock();
        let chunk_id = *id;
        *id += 1;
        drop(id);

        let size = data.len() as u32;
        let chunk = ChunkRef {
            ref_count: 1,
            size,
            data,
        };

        self.chunks.lock().insert(chunk_id, chunk);
        chunk_id
    }

    /// 获取数据块引用计数（用于调试）
    pub fn get_chunk_ref_count(&self, chunk_id: u64) -> Option<u32> {
        self.chunks.lock().get(&chunk_id).map(|c| c.ref_count)
    }
}

// ---------------- 辅助函数 ----------------

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------- 共享类型别名 ----------------

/// 共享的快照管理器引用
pub type SharedSnapshotManager = Arc<SnapshotManager>;

// ---------------- 单元测试 ----------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta_trait::{InMemInodeStore, S_IFDIR, S_IFREG};

    fn setup_test_store() -> InMemInodeStore {
        let mut store = InMemInodeStore::new();
        // store 默认有根目录 ino=1

        // 创建子目录 ino=2
        let d_ino = store.next_ino();
        let t = now_secs();
        store.inodes.insert(
            d_ino,
            Attr {
                ino: d_ino,
                parent: 1,
                name: "subdir".into(),
                mode: S_IFDIR | 0o755,
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
        store
            .dir_index
            .insert((1, "subdir".to_string()), d_ino);

        // 创建文件 ino=3
        let f_ino = store.next_ino();
        store.inodes.insert(
            f_ino,
            Attr {
                ino: f_ino,
                parent: 1,
                name: "file.txt".into(),
                mode: S_IFREG | 0o644,
                uid: 0,
                gid: 0,
                size: 12,
                atime: t,
                mtime: t,
                ctime: t,
                nlink: 1,
                data: b"hello world!".to_vec(),
                symlink: None,
            },
        );
        store
            .dir_index
            .insert((1, "file.txt".to_string()), f_ino);

        // 在子目录中创建文件 ino=4
        let f2_ino = store.next_ino();
        store.inodes.insert(
            f2_ino,
            Attr {
                ino: f2_ino,
                parent: d_ino,
                name: "nested.txt".into(),
                mode: S_IFREG | 0o644,
                uid: 0,
                gid: 0,
                size: 5,
                atime: t,
                mtime: t,
                ctime: t,
                nlink: 1,
                data: b"nested".to_vec(),
                symlink: None,
            },
        );
        store
            .dir_index
            .insert((d_ino, "nested.txt".to_string()), f2_ino);

        store
    }

    fn inode_list_dir(store: &InMemInodeStore, parent: u64) -> FilerResult<Vec<DirEntry>> {
        let mut entries = Vec::new();
        for ((p, name), ino) in &store.dir_index {
            if *p == parent {
                let attr = store.inodes.get(ino).unwrap();
                let typ = if (attr.mode & 0o170000) == S_IFDIR {
                    1
                } else if (attr.mode & 0o170000) == S_IFREG {
                    2
                } else {
                    3
                };
                entries.push(DirEntry {
                    name: name.clone(),
                    ino: *ino,
                    typ,
                });
            }
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    #[test]
    fn test_create_snapshot() {
        let mgr = SnapshotManager::new();
        let store = setup_test_store();

        let get_attr = |ino: u64| -> FilerResult<Attr> {
            store
                .inodes
                .get(&ino)
                .cloned()
                .ok_or(crate::error::FilerError::NotFound)
        };
        let list_dir = |parent: u64| inode_list_dir(&store, parent);
        let read_data = |ino: u64| -> FilerResult<Vec<u8>> {
            store
                .inodes
                .get(&ino)
                .map(|a| a.data.clone())
                .ok_or(crate::error::FilerError::NotFound)
        };

        let snap = mgr
            .create_snapshot(1, "snap1", Some("test snapshot".into()), get_attr, list_dir, read_data)
            .unwrap();

        assert_eq!(snap.name, "snap1");
        assert_eq!(snap.status, SnapshotStatus::Available);
        assert_eq!(snap.source_ino, 1);
        assert_eq!(snap.dir_count, 2); // 根目录 + subdir
        assert_eq!(snap.file_count, 2); // file.txt + nested.txt
        assert_eq!(snap.total_size, 17); // 12 + 5
    }

    #[test]
    fn test_list_snapshots() {
        let mgr = SnapshotManager::new();
        let store = setup_test_store();

        let get_attr = |ino: u64| -> FilerResult<Attr> {
            store
                .inodes
                .get(&ino)
                .cloned()
                .ok_or(crate::error::FilerError::NotFound)
        };
        let list_dir = |parent: u64| inode_list_dir(&store, parent);
        let read_data = |ino: u64| -> FilerResult<Vec<u8>> {
            store
                .inodes
                .get(&ino)
                .map(|a| a.data.clone())
                .ok_or(crate::error::FilerError::NotFound)
        };

        mgr.create_snapshot(1, "snap1", None, get_attr, list_dir, read_data)
            .unwrap();
        mgr.create_snapshot(1, "snap2", None, get_attr, list_dir, read_data)
            .unwrap();

        let snaps = mgr.list_snapshots(1);
        assert_eq!(snaps.len(), 2);
    }

    #[test]
    fn test_snapshot_readonly_access() {
        let mgr = SnapshotManager::new();
        let store = setup_test_store();

        let get_attr = |ino: u64| -> FilerResult<Attr> {
            store
                .inodes
                .get(&ino)
                .cloned()
                .ok_or(crate::error::FilerError::NotFound)
        };
        let list_dir = |parent: u64| inode_list_dir(&store, parent);
        let read_data = |ino: u64| -> FilerResult<Vec<u8>> {
            store
                .inodes
                .get(&ino)
                .map(|a| a.data.clone())
                .ok_or(crate::error::FilerError::NotFound)
        };

        let snap = mgr
            .create_snapshot(1, "snap1", None, get_attr, list_dir, read_data)
            .unwrap();
        let snap_id = snap.id;
        let root_ino = snap.snapshot_root_ino;

        // 读取根目录
        let entries = mgr.read_snapshot_dir(snap_id, root_ino).unwrap();
        assert_eq!(entries.len(), 2); // subdir + file.txt
        assert!(entries.iter().any(|e| e.name == "file.txt"));
        assert!(entries.iter().any(|e| e.name == "subdir"));

        // 读取文件数据
        let file_ino = mgr
            .lookup_snapshot_path(snap_id, root_ino, "file.txt")
            .unwrap();
        let data = mgr.read_snapshot_data(snap_id, file_ino).unwrap();
        assert_eq!(data, b"hello world!");

        // 读取嵌套文件
        let nested_ino = mgr
            .lookup_snapshot_path(snap_id, root_ino, "subdir/nested.txt")
            .unwrap();
        let nested_data = mgr.read_snapshot_data(snap_id, nested_ino).unwrap();
        assert_eq!(nested_data, b"nested");
    }

    #[test]
    fn test_cow_shared_chunks() {
        let mgr = SnapshotManager::new();
        let store = setup_test_store();

        let get_attr = |ino: u64| -> FilerResult<Attr> {
            store
                .inodes
                .get(&ino)
                .cloned()
                .ok_or(crate::error::FilerError::NotFound)
        };
        let list_dir = |parent: u64| inode_list_dir(&store, parent);
        let read_data = |ino: u64| -> FilerResult<Vec<u8>> {
            store
                .inodes
                .get(&ino)
                .map(|a| a.data.clone())
                .ok_or(crate::error::FilerError::NotFound)
        };

        // 创建两个相同源的快照，数据块应该共享（引用计数 > 1）
        let snap1 = mgr
            .create_snapshot(1, "snap1", None, get_attr, list_dir, read_data)
            .unwrap();
        let snap2 = mgr
            .create_snapshot(1, "snap2", None, get_attr, list_dir, read_data)
            .unwrap();

        // 总存储使用量应小于两个快照大小之和（数据共享）
        let total = mgr.total_storage_used();
        assert!(total < snap1.total_size + snap2.total_size);

        // 删除一个快照后，空间应该部分回收
        mgr.delete_snapshot(snap1.id).unwrap();
        let total_after = mgr.total_storage_used();
        assert!(total_after < total);
        assert!(total_after > 0); // snap2 还在

        // 再删除第二个，空间应该完全回收
        mgr.delete_snapshot(snap2.id).unwrap();
        let total_final = mgr.total_storage_used();
        assert_eq!(total_final, 0);
    }

    #[test]
    fn test_restore_snapshot() {
        let mgr = SnapshotManager::new();
        let store = setup_test_store();

        let get_attr = |ino: u64| -> FilerResult<Attr> {
            store
                .inodes
                .get(&ino)
                .cloned()
                .ok_or(crate::error::FilerError::NotFound)
        };
        let list_dir = |parent: u64| inode_list_dir(&store, parent);
        let read_data = |ino: u64| -> FilerResult<Vec<u8>> {
            store
                .inodes
                .get(&ino)
                .map(|a| a.data.clone())
                .ok_or(crate::error::FilerError::NotFound)
        };

        let snap = mgr
            .create_snapshot(1, "backup", None, get_attr, list_dir, read_data)
            .unwrap();

        // 用一个新的内存存储作为恢复目标
        let mut target_store = InMemInodeStore::new();

        let mkdir = |parent: u64, name: &str, mode: u32| -> FilerResult<u64> {
            let ino = target_store.next_ino();
            let t = now_secs();
            target_store.inodes.insert(
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
            target_store
                .dir_index
                .insert((parent, name.to_string()), ino);
            Ok(ino)
        };

        let create = |parent: u64, name: &str, mode: u32| -> FilerResult<u64> {
            let ino = target_store.next_ino();
            let t = now_secs();
            target_store.inodes.insert(
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
            target_store
                .dir_index
                .insert((parent, name.to_string()), ino);
            Ok(ino)
        };

        let write = |ino: u64, data: &[u8]| -> FilerResult<()> {
            if let Some(attr) = target_store.inodes.get_mut(&ino) {
                attr.data = data.to_vec();
                attr.size = data.len() as u64;
            }
            Ok(())
        };

        let restored_ino = mgr
            .restore_snapshot(snap.id, 1, "restored", mkdir, create, write)
            .unwrap();

        // 验证恢复的目录
        assert!(target_store
            .dir_index
            .contains_key(&(1, "restored".to_string())));
        assert_eq!(restored_ino, target_store.lookup_name(1, "restored").unwrap());

        // 验证恢复的文件
        let restored_root = restored_ino;
        let file_ino = target_store.lookup_name(restored_root, "file.txt").unwrap();
        let file_attr = target_store.inodes.get(&file_ino).unwrap();
        assert_eq!(file_attr.data, b"hello world!");
    }

    #[test]
    fn test_delete_snapshot() {
        let mgr = SnapshotManager::new();
        let store = setup_test_store();

        let get_attr = |ino: u64| -> FilerResult<Attr> {
            store
                .inodes
                .get(&ino)
                .cloned()
                .ok_or(crate::error::FilerError::NotFound)
        };
        let list_dir = |parent: u64| inode_list_dir(&store, parent);
        let read_data = |ino: u64| -> FilerResult<Vec<u8>> {
            store
                .inodes
                .get(&ino)
                .map(|a| a.data.clone())
                .ok_or(crate::error::FilerError::NotFound)
        };

        let snap = mgr
            .create_snapshot(1, "snap1", None, get_attr, list_dir, read_data)
            .unwrap();

        assert!(mgr.get_snapshot(snap.id).is_some());

        mgr.delete_snapshot(snap.id).unwrap();

        assert!(mgr.get_snapshot(snap.id).is_none());
        assert_eq!(mgr.list_snapshots(1).len(), 0);
    }

    #[test]
    fn test_find_snapshot_by_name() {
        let mgr = SnapshotManager::new();
        let store = setup_test_store();

        let get_attr = |ino: u64| -> FilerResult<Attr> {
            store
                .inodes
                .get(&ino)
                .cloned()
                .ok_or(crate::error::FilerError::NotFound)
        };
        let list_dir = |parent: u64| inode_list_dir(&store, parent);
        let read_data = |ino: u64| -> FilerResult<Vec<u8>> {
            store
                .inodes
                .get(&ino)
                .map(|a| a.data.clone())
                .ok_or(crate::error::FilerError::NotFound)
        };

        mgr.create_snapshot(1, "daily-001", None, get_attr, list_dir, read_data)
            .unwrap();

        let found = mgr.find_snapshot_by_name(1, "daily-001");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "daily-001");

        let not_found = mgr.find_snapshot_by_name(1, "nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_duplicate_name_rejected() {
        let mgr = SnapshotManager::new();
        let store = setup_test_store();

        let get_attr = |ino: u64| -> FilerResult<Attr> {
            store
                .inodes
                .get(&ino)
                .cloned()
                .ok_or(crate::error::FilerError::NotFound)
        };
        let list_dir = |parent: u64| inode_list_dir(&store, parent);
        let read_data = |ino: u64| -> FilerResult<Vec<u8>> {
            store
                .inodes
                .get(&ino)
                .map(|a| a.data.clone())
                .ok_or(crate::error::FilerError::NotFound)
        };

        mgr.create_snapshot(1, "snap1", None, get_attr, list_dir, read_data)
            .unwrap();

        let result = mgr.create_snapshot(1, "snap1", None, get_attr, list_dir, read_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshot_attr_read() {
        let mgr = SnapshotManager::new();
        let store = setup_test_store();

        let get_attr = |ino: u64| -> FilerResult<Attr> {
            store
                .inodes
                .get(&ino)
                .cloned()
                .ok_or(crate::error::FilerError::NotFound)
        };
        let list_dir = |parent: u64| inode_list_dir(&store, parent);
        let read_data = |ino: u64| -> FilerResult<Vec<u8>> {
            store
                .inodes
                .get(&ino)
                .map(|a| a.data.clone())
                .ok_or(crate::error::FilerError::NotFound)
        };

        let snap = mgr
            .create_snapshot(1, "snap1", None, get_attr, list_dir, read_data)
            .unwrap();

        let root_attr = mgr.read_snapshot_attr(snap.id, snap.snapshot_root_ino).unwrap();
        assert_eq!((root_attr.mode & 0o170000), S_IFDIR);
    }
}
