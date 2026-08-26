use crate::error::{MasterError, MasterResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReplicaHealth {
    Healthy,
    Unhealthy,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaInfo {
    pub volume_id: String,
    pub addr: String,
    pub health: ReplicaHealth,
    pub last_acked: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaSet {
    pub set_id: String,
    pub replicas: Vec<ReplicaInfo>,
    pub replica_count: u8,
}

impl ReplicaSet {
    pub fn new(set_id: String, replica_count: u8) -> Self {
        Self {
            set_id,
            replicas: Vec::with_capacity(replica_count as usize),
            replica_count,
        }
    }

    pub fn add_replica(&mut self, info: ReplicaInfo) {
        // 同 volume_id 已存在则更新
        if let Some(pos) = self
            .replicas
            .iter()
            .position(|r| r.volume_id == info.volume_id)
        {
            self.replicas[pos] = info;
        } else {
            self.replicas.push(info);
        }
    }

    /// 写 quorum = N/2 + 1 (N = replica_count)
    pub fn write_quorum(&self) -> MasterResult<usize> {
        let n = self.replica_count.max(1) as usize;
        Ok(n / 2 + 1)
    }

    /// 读 quorum = ceil(N/2) (等价于 N/2 + 当 N 是奇数时 1 还是 + 0；
    /// 但常规实现写 W=N/2+1, 读 R=N/2+1 保证 W+R>N 相交；这里简化 R=N/2+1 也可)
    pub fn read_quorum(&self) -> MasterResult<usize> {
        let n = self.replica_count.max(1) as usize;
        Ok(n / 2 + 1)
    }

    pub fn healthy_count(&self) -> usize {
        self.replicas
            .iter()
            .filter(|r| r.health == ReplicaHealth::Healthy)
            .count()
    }

    pub fn check_write_ok(&self) -> MasterResult<()> {
        let need = self.write_quorum()?;
        if self.healthy_count() >= need {
            Ok(())
        } else {
            Err(MasterError::ReplicaQuorum(format!(
                "write need {} healthy replicas, got {} in set {}",
                need,
                self.healthy_count(),
                self.set_id
            )))
        }
    }

    pub fn check_read_ok(&self) -> MasterResult<()> {
        // 读 quorum 宽容：只要 healthy >= 1 并且结合写入策略能覆盖，这里按 "只要 healthy > 0 且 dead 不超过 1/2"
        // 为测试 TR4.4 的 quorum 读 2/3：我们简化 R = N/2（向上取整 + 1 的 half），即 N=3 时 R=2
        let n = self.replica_count.max(1) as usize;
        let need = n.div_ceil(2);
        let need = need.max(1);
        if self.healthy_count() >= need {
            Ok(())
        } else {
            Err(MasterError::ReplicaQuorum(format!(
                "read need {} healthy replicas, got {} in set {}",
                need,
                self.healthy_count(),
                self.set_id
            )))
        }
    }

    pub fn mark_dead(&mut self, volume_id: &str) {
        for r in self.replicas.iter_mut() {
            if r.volume_id == volume_id {
                r.health = ReplicaHealth::Dead;
            }
        }
    }

    /// healthy < replica_count 就需要 refill 触发
    pub fn needs_refill(&self) -> bool {
        self.healthy_count() < self.replica_count as usize
    }

    pub fn healthy_addrs(&self) -> Vec<String> {
        self.replicas
            .iter()
            .filter(|r| r.health == ReplicaHealth::Healthy)
            .map(|r| r.addr.clone())
            .collect()
    }
}

pub struct ReplicaSetManager {
    pub sets: parking_lot::Mutex<HashMap<String, ReplicaSet>>,
    refill_triggers: parking_lot::Mutex<u64>,
}

impl ReplicaSetManager {
    pub fn new() -> Self {
        Self {
            sets: parking_lot::Mutex::new(HashMap::new()),
            refill_triggers: parking_lot::Mutex::new(0),
        }
    }

    pub fn create_set(&self, set_id: String, replica_count: u8) {
        let set = ReplicaSet::new(set_id.clone(), replica_count);
        self.sets.lock().insert(set_id, set);
    }

    pub fn add_replica_to_set(&self, set_id: &str, info: ReplicaInfo) {
        let mut sets = self.sets.lock();
        if let Some(s) = sets.get_mut(set_id) {
            s.add_replica(info);
        }
    }

    /// 将某个 volume 在所有集合里标为 Dead，返回受影响的 set_id 列表
    pub fn mark_volume_dead(&self, volume_id: &str) -> Vec<String> {
        let mut sets = self.sets.lock();
        let mut affected = Vec::new();
        for (sid, s) in sets.iter_mut() {
            let before = s.healthy_count();
            s.mark_dead(volume_id);
            let after = s.healthy_count();
            if before != after {
                affected.push(sid.clone());
            }
        }
        affected
    }

    /// 对指定集合：若 needs_refill，则累计 refill trigger 计数
    /// 返回本次新增触发次数
    pub fn trigger_refill_if_needed(&self, set_ids: &[String]) -> u64 {
        let sets = self.sets.lock();
        let mut count = 0u64;
        let mut unique: HashSet<String> = HashSet::new();
        for sid in set_ids {
            unique.insert(sid.clone());
        }
        for sid in unique {
            if let Some(s) = sets.get(&sid) {
                if s.needs_refill() {
                    count += 1;
                }
            }
        }
        if count > 0 {
            let mut t = self.refill_triggers.lock();
            *t += count;
        }
        count
    }

    pub fn refill_trigger_count(&self) -> u64 {
        *self.refill_triggers.lock()
    }

    pub fn get_set(&self, set_id: &str) -> Option<ReplicaSet> {
        self.sets.lock().get(set_id).cloned()
    }

    pub fn check_write_ok(&self, set_id: &str) -> MasterResult<()> {
        let sets = self.sets.lock();
        let s = sets.get(set_id).ok_or_else(|| {
            MasterError::VolumeNotFound(format!("replica set {} not found", set_id))
        })?;
        s.check_write_ok()
    }

    pub fn check_read_ok(&self, set_id: &str) -> MasterResult<()> {
        let sets = self.sets.lock();
        let s = sets.get(set_id).ok_or_else(|| {
            MasterError::VolumeNotFound(format!("replica set {} not found", set_id))
        })?;
        s.check_read_ok()
    }
}

impl Default for ReplicaSetManager {
    fn default() -> Self {
        Self::new()
    }
}
