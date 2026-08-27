// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::error::{MasterError, MasterResult};
use crate::snapshot::{now_millis, SnapshotId, SnapshotManager};
use crate::volume_allocator::{VolumeAllocation, VolumeAllocator, VolumeInfo};
use crate::volume_replica::{ReplicaHealth, ReplicaInfo, ReplicaSetManager};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub type VolumeId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterConfig {
    pub heartbeat_timeout_ms: u64,
    pub max_replica: u8,
}

impl Default for MasterConfig {
    fn default() -> Self {
        MasterConfig {
            heartbeat_timeout_ms: 1500,
            max_replica: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeLoadReport {
    pub used_bytes: u64,
    pub chunk_count: u64,
    pub cpu_pct: u8,
    pub is_healthy: bool,
}

impl Default for VolumeLoadReport {
    fn default() -> Self {
        VolumeLoadReport {
            used_bytes: 0,
            chunk_count: 0,
            cpu_pct: 0,
            is_healthy: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VolumeStatusState {
    Alive,
    Dead,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeStatus {
    pub id: VolumeId,
    pub addr: String,
    pub capacity: u64,
    pub used: u64,
    pub state: VolumeStatusState,
    pub last_heartbeat_ms: u64,
}

pub struct Metrics {
    pub heartbeats_received: parking_lot::Mutex<u64>,
    pub volumes_allocations_total: parking_lot::Mutex<u64>,
    pub replicas_fill_triggers: parking_lot::Mutex<u64>,
    pub snapshots_taken: parking_lot::Mutex<u64>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            heartbeats_received: parking_lot::Mutex::new(0),
            volumes_allocations_total: parking_lot::Mutex::new(0),
            replicas_fill_triggers: parking_lot::Mutex::new(0),
            snapshots_taken: parking_lot::Mutex::new(0),
        }
    }

    pub fn inc_heartbeats(&self) {
        *self.heartbeats_received.lock() += 1;
    }

    pub fn inc_allocations(&self) {
        *self.volumes_allocations_total.lock() += 1;
    }

    pub fn inc_fill_triggers(&self, n: u64) {
        *self.replicas_fill_triggers.lock() += n;
    }

    pub fn inc_snapshots(&self) {
        *self.snapshots_taken.lock() += 1;
    }

    pub fn get_all(&self) -> HashMap<String, u64> {
        let mut m = HashMap::new();
        m.insert(
            "heartbeats_received".into(),
            *self.heartbeats_received.lock(),
        );
        m.insert(
            "volumes_allocations_total".into(),
            *self.volumes_allocations_total.lock(),
        );
        m.insert(
            "replicas_fill_triggers".into(),
            *self.replicas_fill_triggers.lock(),
        );
        m.insert("snapshots_taken".into(), *self.snapshots_taken.lock());
        m
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MasterServer {
    pub config: MasterConfig,
    pub allocator: VolumeAllocator,
    pub replica_manager: ReplicaSetManager,
    pub snapshot_manager: SnapshotManager,
    pub metrics: Arc<Metrics>,
    last_heartbeat: parking_lot::Mutex<HashMap<VolumeId, u64>>,
    volume_capacities: parking_lot::Mutex<HashMap<VolumeId, u64>>,
    /// M1 简化：volume_id -> 最近 snapshot_manifest（用于 restore 查回）
    #[allow(clippy::type_complexity)]
    // snapshot_manifest 仅在 restore 反查时用；避免大类型散落污染
    snapshot_manifest_store:
        parking_lot::Mutex<HashMap<(VolumeId, SnapshotId), BTreeMap<String, Vec<u8>>>>,
}

impl MasterServer {
    pub fn new(config: MasterConfig) -> Self {
        Self {
            config,
            allocator: VolumeAllocator::new(),
            replica_manager: ReplicaSetManager::new(),
            snapshot_manager: SnapshotManager::new(),
            metrics: Arc::new(Metrics::new()),
            last_heartbeat: parking_lot::Mutex::new(HashMap::new()),
            volume_capacities: parking_lot::Mutex::new(HashMap::new()),
            snapshot_manifest_store: parking_lot::Mutex::new(HashMap::new()),
        }
    }

    pub fn register_volume(&self, addr: String, capacity: u64) -> VolumeId {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let id = format!(
            "vol-{:04x}{:04x}{:04x}",
            rng.gen_range(0..=0xFFFFu32),
            rng.gen_range(0..=0xFFFFu32),
            rng.gen_range(0..=0xFFFFu32)
        );
        self.allocator
            .register_volume(id.clone(), addr.clone(), capacity);
        self.volume_capacities.lock().insert(id.clone(), capacity);
        self.last_heartbeat.lock().insert(id.clone(), now_millis());
        id
    }

    pub fn heartbeat(&self, volume_id: &str, load: VolumeLoadReport) -> MasterResult<()> {
        {
            let vols = self.allocator.list_volumes();
            if !vols.iter().any(|v| v.id == volume_id) {
                return Err(MasterError::VolumeNotFound(volume_id.to_string()));
            }
        }
        self.metrics.inc_heartbeats();
        let now = now_millis();
        self.last_heartbeat
            .lock()
            .insert(volume_id.to_string(), now);

        // alive 根据 load.is_healthy + 时间综合
        self.allocator.update_heartbeat(volume_id, load.is_healthy);
        self.allocator.update_used(volume_id, load.used_bytes);

        // 若 healthy=true，更新 replica set 中对应节点状态
        let sets_affected = {
            // 临时：需要 replica_manager 把这个 volume 在各个 set 的健康状态更新
            // 当前实现里 replica_manager 不公开按 volume_id 遍历的接口，
            // 用 mark_volume_dead 反向再恢复：为简洁 M1 实现 heartbeat 时直接把
            // 对应 volume 在 replica sets 里标成 Healthy（简化）
            let sets = self.replica_manager.sets.lock();
            let mut v: Vec<String> = Vec::new();
            for (sid, s) in sets.iter() {
                for r in &s.replicas {
                    if r.volume_id == volume_id {
                        v.push(sid.clone());
                        break;
                    }
                }
            }
            drop(sets);
            v
        };
        // 直接通过 add_replica_to_set 更新健康：每个 set 找到对应 info 改健康
        for sid in &sets_affected {
            if let Some(mut s) = self.replica_manager.get_set(sid) {
                for r in s.replicas.iter_mut() {
                    if r.volume_id == volume_id {
                        r.health = if load.is_healthy {
                            ReplicaHealth::Healthy
                        } else {
                            ReplicaHealth::Unhealthy
                        };
                        r.last_acked = now;
                    }
                }
                // 写回：删除重建（简化，M1 不需要性能）
                let rc = s.replica_count;
                let sid2 = s.set_id.clone();
                let replicas = s.replicas.clone();
                // 直接 set（通过重新 create 会清空 replicas 所以用下面技巧替换）：
                // 用 sets 字段直接拿
                let mut sets_map = self.replica_manager.sets.lock();
                if let Some(entry) = sets_map.get_mut(&sid2) {
                    entry.replicas = replicas;
                    let _ = rc;
                }
            }
        }

        Ok(())
    }

    pub fn allocate_volume(&self, size: u64, replica: u8) -> MasterResult<VolumeAllocation> {
        if replica > self.config.max_replica {
            return Err(MasterError::InvalidReplicaCount(format!(
                "replica {} exceeds max_replica {}",
                replica, self.config.max_replica
            )));
        }
        let alloc = self.allocator.allocate(size, replica)?;
        self.metrics.inc_allocations();

        // 创建 ReplicaSet
        let set_id = alloc.volume_id.clone();
        self.replica_manager.create_set(set_id.clone(), replica);
        for (i, vid) in alloc.replica_ids.iter().enumerate() {
            let addr = alloc.replica_addresses.get(i).cloned().unwrap_or_default();
            self.replica_manager.add_replica_to_set(
                &set_id,
                ReplicaInfo {
                    volume_id: vid.clone(),
                    addr,
                    health: ReplicaHealth::Healthy,
                    last_acked: now_millis(),
                },
            );
        }

        Ok(alloc)
    }

    pub fn list_volumes(&self) -> Vec<VolumeStatus> {
        let infos: Vec<VolumeInfo> = self.allocator.list_volumes();
        let now = now_millis();
        let hb = self.last_heartbeat.lock();
        infos
            .into_iter()
            .map(|v| {
                let last = *hb.get(&v.id).unwrap_or(&0);
                let timed_out = now.saturating_sub(last) > self.config.heartbeat_timeout_ms;
                let state = if !v.is_alive || timed_out {
                    VolumeStatusState::Dead
                } else {
                    VolumeStatusState::Alive
                };
                VolumeStatus {
                    id: v.id,
                    addr: v.addr,
                    capacity: v.capacity,
                    used: v.used,
                    state,
                    last_heartbeat_ms: last,
                }
            })
            .collect()
    }

    /// 无 manifest 版本：用 snapshot_manager 生成 id 并记录；M1 简化不实际存数据
    pub fn snapshot_volume(&self, volume_id: &str) -> MasterResult<SnapshotId> {
        // 校验 volume 存在
        let vols = self.allocator.list_volumes();
        if !vols.iter().any(|v| v.id == volume_id) {
            return Err(MasterError::VolumeNotFound(volume_id.to_string()));
        }
        let empty = BTreeMap::new();
        let sid = self.snapshot_manager.take_snapshot(volume_id, empty)?;
        self.metrics.inc_snapshots();
        // 同步放到 store（空 manifest）
        self.snapshot_manifest_store
            .lock()
            .insert((volume_id.to_string(), sid.clone()), BTreeMap::new());
        Ok(sid)
    }

    /// 带 manifest 的快照：volume 端导出真实 manifest，master 存储 + 生成不可伪造 sid
    pub fn store_snapshot_manifest(
        &self,
        volume_id: &str,
        manifest: BTreeMap<String, Vec<u8>>,
    ) -> MasterResult<SnapshotId> {
        let sid = self
            .snapshot_manager
            .take_snapshot(volume_id, manifest.clone())?;
        self.metrics.inc_snapshots();
        self.snapshot_manifest_store
            .lock()
            .insert((volume_id.to_string(), sid.clone()), manifest);
        Ok(sid)
    }

    pub fn get_snapshot_manifest(
        &self,
        volume_id: &str,
        snapshot_id: &str,
    ) -> MasterResult<BTreeMap<String, Vec<u8>>> {
        // 先让 snapshot_manager 校验合法性（存在且未删除）
        let _meta = self.snapshot_manager.get_snapshot(volume_id, snapshot_id)?;
        // 再从自己的 store 返回 manifest（若空则从 meta 拿）
        let key = (volume_id.to_string(), snapshot_id.to_string());
        let store = self.snapshot_manifest_store.lock();
        if let Some(m) = store.get(&key) {
            if !m.is_empty() {
                return Ok(m.clone());
            }
        }
        // 从 snapshot_manager meta 里拿
        drop(store);
        let meta = self.snapshot_manager.get_snapshot(volume_id, snapshot_id)?;
        Ok(meta.chunk_manifest)
    }

    pub fn restore_snapshot(&self, volume_id: &str, snapshot_id: &str) -> MasterResult<()> {
        // 只校验合法性，实际数据由 volume 端 restore_from_manifest 处理
        let _ = self.snapshot_manager.get_snapshot(volume_id, snapshot_id)?;
        Ok(())
    }

    /// 检查所有 volume 心跳是否超时，标记 dead + 触发 refill
    /// 返回新增 refill trigger 数
    pub fn check_dead_and_trigger_refill(&self) -> u64 {
        let now = now_millis();
        let timeout = self.config.heartbeat_timeout_ms;
        let ids_to_mark: Vec<VolumeId> = {
            let hb = self.last_heartbeat.lock();
            let mut dead = Vec::new();
            for (vid, last) in hb.iter() {
                if now.saturating_sub(*last) > timeout {
                    dead.push(vid.clone());
                }
            }
            dead
        };
        let mut set_ids: Vec<String> = Vec::new();
        for vid in &ids_to_mark {
            self.allocator.update_heartbeat(vid, false);
            let affected = self.replica_manager.mark_volume_dead(vid);
            set_ids.extend(affected);
        }
        let triggered = self.replica_manager.trigger_refill_if_needed(&set_ids);
        if triggered > 0 {
            self.metrics.inc_fill_triggers(triggered);
        }
        triggered
    }

    pub fn volume_state(&self, volume_id: &str) -> VolumeStatusState {
        let now = now_millis();
        let last = *self.last_heartbeat.lock().get(volume_id).unwrap_or(&0);
        let timed_out = now.saturating_sub(last) > self.config.heartbeat_timeout_ms;
        let alloc_alive = self.allocator.is_alive(volume_id);
        if timed_out || !alloc_alive {
            VolumeStatusState::Dead
        } else {
            VolumeStatusState::Alive
        }
    }

    pub fn get_metrics(&self) -> HashMap<String, u64> {
        let mut m = self.metrics.get_all();
        // 把 replica_manager 的 refill_count 也同步（保证最新）
        m.insert(
            "replicas_fill_triggers".into(),
            self.metrics
                .replicas_fill_triggers
                .lock()
                .saturating_add(self.replica_manager.refill_trigger_count()),
        );
        // snapshots_taken 同步 snapshot_manager 计数
        m.insert(
            "snapshots_taken".into(),
            self.snapshot_manager.snapshots_taken_count(),
        );
        m
    }

    /// 返回 refill 触发总计数（metrics + replica_manager 合计）
    pub fn start_replica_refill_count(&self) -> u64 {
        // 先主动跑一次 dead 检测，这样 TR4.3 停止 B 心跳后，调用方能拿到 count
        let _ = self.check_dead_and_trigger_refill();
        let m = self.get_metrics();
        m.get("replicas_fill_triggers").copied().unwrap_or(0)
    }
}
