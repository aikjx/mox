// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use crate::error::{MasterError, MasterResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub id: String,
    pub addr: String,
    pub capacity: u64,
    pub used: u64,
    pub is_alive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeAllocation {
    pub volume_id: String,
    pub replica_addresses: Vec<String>,
    pub replica_ids: Vec<String>,
    pub size: u64,
    pub replica_count: u8,
}

pub struct VolumeAllocator {
    volumes: parking_lot::Mutex<HashMap<String, VolumeInfo>>,
    rr_index: parking_lot::Mutex<usize>,
    order: parking_lot::Mutex<Vec<String>>,
}

impl VolumeAllocator {
    pub fn new() -> Self {
        Self {
            volumes: parking_lot::Mutex::new(HashMap::new()),
            rr_index: parking_lot::Mutex::new(0),
            order: parking_lot::Mutex::new(Vec::new()),
        }
    }

    pub fn register_volume(&self, id: String, addr: String, capacity: u64) {
        let info = VolumeInfo {
            id: id.clone(),
            addr,
            capacity,
            used: 0,
            is_alive: true,
        };
        self.volumes.lock().insert(id.clone(), info);
        let mut order = self.order.lock();
        if !order.contains(&id) {
            order.push(id);
        }
    }

    pub fn update_heartbeat(&self, id: &str, alive: bool) {
        if let Some(v) = self.volumes.lock().get_mut(id) {
            v.is_alive = alive;
        }
    }

    pub fn update_used(&self, id: &str, used: u64) {
        if let Some(v) = self.volumes.lock().get_mut(id) {
            v.used = used.min(v.capacity);
        }
    }

    /// round-robin + 容量最空优先 双策略
    /// 约束：replica 副本必须分配在 **不同** volume server 上；replica<=3
    pub fn allocate(&self, size: u64, replica: u8) -> MasterResult<VolumeAllocation> {
        if replica == 0 || replica > 3 {
            return Err(MasterError::InvalidReplicaCount(format!(
                "replica must be 1..=3, got {}",
                replica
            )));
        }

        let vols_snap: Vec<VolumeInfo> = {
            let v = self.volumes.lock();
            v.values().cloned().collect()
        };

        // 1. 筛选 alive 且 (capacity - used >= size) 的候选
        let mut candidates: Vec<VolumeInfo> = vols_snap
            .into_iter()
            .filter(|v| v.is_alive && v.capacity.saturating_sub(v.used) >= size)
            .collect();

        if (candidates.len() as u8) < replica {
            return Err(MasterError::NoCapacity(format!(
                "need {} alive volumes each with >= {} bytes free, only {} qualified",
                replica,
                size,
                candidates.len()
            )));
        }

        // 2. 双策略：按 `free` 最空优先 (primary key)；若 free 相同用 RR index 轮询
        candidates.sort_by(|a, b| {
            let fa = a.capacity.saturating_sub(a.used);
            let fb = b.capacity.saturating_sub(b.used);
            fb.cmp(&fa) // 最空在前
        });

        // 取前 N 个（replica 个）不同节点
        let mut picked_ids = HashSet::new();
        let mut picked = Vec::with_capacity(replica as usize);
        // 从 RR 起点开始绕，保证在同等 free 时 RR
        let rr = *self.rr_index.lock();
        let n = candidates.len();
        for offset in 0..n {
            let idx = (rr + offset) % n;
            let info = &candidates[idx];
            if picked_ids.insert(info.id.clone()) {
                picked.push(info.clone());
                if picked.len() == replica as usize {
                    break;
                }
            }
        }
        // 若 RR 绕一圈没拿够 replica 个（理论上不会），回退从 sorted 头部顺序拿
        if (picked.len() as u8) < replica {
            picked.clear();
            picked_ids.clear();
            for info in candidates.iter() {
                if picked_ids.insert(info.id.clone()) {
                    picked.push(info.clone());
                    if picked.len() == replica as usize {
                        break;
                    }
                }
            }
        }

        // 更新 RR
        let mut rr_mut = self.rr_index.lock();
        *rr_mut = (*rr_mut + 1) % n.max(1);
        drop(rr_mut);

        // 正式把 used 加上（分配即记账）
        let mut replica_ids = Vec::with_capacity(replica as usize);
        let mut replica_addrs = Vec::with_capacity(replica as usize);
        {
            let mut vols = self.volumes.lock();
            for p in &picked {
                if let Some(v) = vols.get_mut(&p.id) {
                    v.used = v.used.saturating_add(size);
                }
                replica_ids.push(p.id.clone());
                replica_addrs.push(p.addr.clone());
            }
        }

        Ok(VolumeAllocation {
            volume_id: format!("vol-{}-{}", replica, rand_suffix()),
            replica_addresses: replica_addrs,
            replica_ids,
            size,
            replica_count: replica,
        })
    }

    pub fn list_volumes(&self) -> Vec<VolumeInfo> {
        self.volumes.lock().values().cloned().collect()
    }

    pub fn is_alive(&self, id: &str) -> bool {
        self.volumes
            .lock()
            .get(id)
            .map(|v| v.is_alive)
            .unwrap_or(false)
    }

    pub fn get_addr(&self, id: &str) -> Option<String> {
        self.volumes.lock().get(id).map(|v| v.addr.clone())
    }

    pub fn get_used(&self, id: &str) -> Option<u64> {
        self.volumes.lock().get(id).map(|v| v.used)
    }
}

impl Default for VolumeAllocator {
    fn default() -> Self {
        Self::new()
    }
}

fn rand_suffix() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut s = String::with_capacity(8);
    for _ in 0..8 {
        s.push(std::char::from_digit(rng.gen_range(0..16), 16).unwrap());
    }
    s
}
