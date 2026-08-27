// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use async_trait::async_trait;
use std::collections::BTreeMap;
use std::error::Error;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShardInfo {
    pub shard_id: u64,
    pub leader_addr: String,
    pub replica_addrs: Vec<String>,
    pub status: String,
    pub vid_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RebalancePlan {
    pub moves: Vec<RebalanceMove>,
    pub expected_migration_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RebalanceMove {
    pub shard_id: u64,
    pub from_addr: String,
    pub to_addr: String,
    pub estimated_vids: u64,
}

#[derive(Debug, Clone, Default)]
struct PartitionState {
    shard_count: u64,
    shards: BTreeMap<u64, ShardInfo>,
}

#[async_trait]
pub trait PartitionRouterProvider: Send + Sync {
    async fn vid_to_shard(&self, vid: &str) -> Result<u64, Box<dyn Error + Send + Sync>>;
    async fn shard_to_storage_addr(
        &self,
        shard_id: u64,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
    async fn list_shards(&self) -> Result<Vec<ShardInfo>, Box<dyn Error + Send + Sync>>;
    async fn rebalance_plan(
        &self,
        targets: Vec<String>,
    ) -> Result<RebalancePlan, Box<dyn Error + Send + Sync>>;
    async fn apply_rebalance(
        &self,
        plan: RebalancePlan,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn total_shard_count(&self) -> Result<u64, Box<dyn Error + Send + Sync>>;
    async fn update_storage_host(
        &self,
        old: &str,
        new: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}

pub struct MockPartitionRouterProvider {
    st: parking_lot::Mutex<PartitionState>,
}
impl Default for MockPartitionRouterProvider {
    fn default() -> Self {
        let mut s = PartitionState {
            shard_count: 8,
            ..Default::default()
        };
        for i in 0..8u64 {
            s.shards.insert(
                i,
                ShardInfo {
                    shard_id: i,
                    leader_addr: format!("127.0.0.1:{}", 9669 + (i % 3)),
                    status: "ACTIVE".into(),
                    ..Default::default()
                },
            );
        }
        Self {
            st: parking_lot::Mutex::new(s),
        }
    }
}
fn hash_vid(vid: &str, n: u64) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    vid.hash(&mut h);
    h.finish() % n.max(1)
}

#[async_trait]
impl PartitionRouterProvider for MockPartitionRouterProvider {
    async fn vid_to_shard(&self, vid: &str) -> Result<u64, Box<dyn Error + Send + Sync>> {
        let n = self.total_shard_count().await?;
        Ok(hash_vid(vid, n))
    }
    async fn shard_to_storage_addr(
        &self,
        sid: u64,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let st = self.st.lock();
        Ok(st
            .shards
            .get(&sid)
            .ok_or("shard missing")?
            .leader_addr
            .clone())
    }
    async fn list_shards(&self) -> Result<Vec<ShardInfo>, Box<dyn Error + Send + Sync>> {
        let st = self.st.lock();
        let mut v: Vec<ShardInfo> = st.shards.values().cloned().collect();
        v.sort_by_key(|s| s.shard_id);
        Ok(v)
    }
    async fn rebalance_plan(
        &self,
        targets: Vec<String>,
    ) -> Result<RebalancePlan, Box<dyn Error + Send + Sync>> {
        if targets.is_empty() {
            return Ok(RebalancePlan::default());
        }
        let st = self.st.lock();
        let mut moves = vec![];
        for (idx, (sid, sh)) in st.shards.iter().enumerate() {
            let t = &targets[idx % targets.len()];
            if sh.leader_addr != *t {
                moves.push(RebalanceMove {
                    shard_id: *sid,
                    from_addr: sh.leader_addr.clone(),
                    to_addr: t.clone(),
                    estimated_vids: sh.vid_count,
                });
            }
        }
        let bytes = moves
            .iter()
            .map(|m| m.estimated_vids.saturating_mul(128))
            .sum();
        Ok(RebalancePlan {
            moves,
            expected_migration_bytes: bytes,
        })
    }
    async fn apply_rebalance(
        &self,
        plan: RebalancePlan,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut st = self.st.lock();
        for mv in plan.moves {
            if let Some(sh) = st.shards.get_mut(&mv.shard_id) {
                sh.leader_addr = mv.to_addr;
            }
        }
        Ok(())
    }
    async fn total_shard_count(&self) -> Result<u64, Box<dyn Error + Send + Sync>> {
        Ok(self.st.lock().shard_count)
    }
    async fn update_storage_host(
        &self,
        old: &str,
        new: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut st = self.st.lock();
        for sh in st.shards.values_mut() {
            if sh.leader_addr == old {
                sh.leader_addr = new.into();
            }
            for r in sh.replica_addrs.iter_mut() {
                if r == old {
                    *r = new.into();
                }
            }
        }
        Ok(())
    }
}
