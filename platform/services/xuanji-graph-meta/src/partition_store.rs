//! PartitionStore：VID 哈希分片 + shard → storage host 映射。
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::error::{MetaError, MetaResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageHost {
    pub id: String,
    pub addr: String,   // host:port
    pub status: String, // ONLINE / OFFLINE
}

/// 计算 VID 的分片：SHA-256(hex(VID_bytes)) first 8 bytes mod partition_num → shard_id。
///
/// 对外公开，便于调用方路由计算。
pub fn vid_hash_partition(vid: &str, partition_num: u16) -> u64 {
    let mut hasher = Sha256::new();
    let vid_hex = hex::encode(vid.as_bytes());
    hasher.update(vid_hex.as_bytes());
    let result = hasher.finalize();
    // 取前 8 bytes，转成 u64（big-endian）
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&result[..8]);
    let v = u64::from_be_bytes(arr);
    v % (partition_num as u64)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PartitionStore {
    pub hosts: BTreeMap<String, StorageHost>,
    pub shard_leader: BTreeMap<(String, u64), String>, // (space, shard_id) -> host_id
}

impl PartitionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_storage_host(&mut self, id: &str, addr: &str) -> MetaResult<()> {
        if id.is_empty() || addr.is_empty() {
            return Err(MetaError::InvalidArgument("host id/addr empty".to_string()));
        }
        self.hosts.insert(
            id.to_string(),
            StorageHost {
                id: id.to_string(),
                addr: addr.to_string(),
                status: "ONLINE".to_string(),
            },
        );
        Ok(())
    }

    pub fn unregister_storage_host(&mut self, id: &str) {
        if let Some(h) = self.hosts.get_mut(id) {
            h.status = "OFFLINE".to_string();
        }
    }

    pub fn list_hosts(&self) -> Vec<StorageHost> {
        self.hosts.values().cloned().collect()
    }

    /// 返回 (shard_id, host_addr)
    pub fn get_partition_route(
        &self,
        space: &str,
        vid: &str,
        partition_num: u16,
    ) -> MetaResult<(u64, String)> {
        if partition_num == 0 || partition_num.count_ones() != 1 {
            return Err(MetaError::PartitionInvalid(format!(
                "partition_num {} invalid",
                partition_num
            )));
        }
        let shard = vid_hash_partition(vid, partition_num);
        let key = (space.to_string(), shard);
        let host_id = match self.shard_leader.get(&key) {
            Some(id) => id.clone(),
            None => {
                // round-robin 映射 shard -> host（一致映射）
                let hosts_online: Vec<&StorageHost> = self
                    .hosts
                    .values()
                    .filter(|h| h.status == "ONLINE")
                    .collect();
                if hosts_online.is_empty() {
                    return Err(MetaError::StorageHostMissing);
                }
                let idx = (shard as usize) % hosts_online.len();
                hosts_online[idx].id.clone()
            }
        };
        let host = self
            .hosts
            .get(&host_id)
            .ok_or(MetaError::StorageHostMissing)?;
        Ok((shard, host.addr.clone()))
    }

    /// 为某个空间分配全部分片 → host 的 leader 映射（rebalance 前的建议）。
    pub fn assign_all_shards(
        &mut self,
        space: &str,
        partition_num: u16,
        replica_factor: u8,
    ) -> MetaResult<()> {
        if partition_num.count_ones() != 1 || partition_num < 4 {
            return Err(MetaError::PartitionInvalid(format!(
                "partition_num {} not power of two or < 4",
                partition_num
            )));
        }
        let hosts_online: Vec<StorageHost> = self
            .hosts
            .values()
            .filter(|h| h.status == "ONLINE")
            .cloned()
            .collect();
        if hosts_online.is_empty() {
            return Err(MetaError::StorageHostMissing);
        }
        let _ = replica_factor; // replica 仅用于副本分配，此处只记 leader 映射
        for shard in 0..partition_num as u64 {
            let idx = (shard as usize) % hosts_online.len();
            let host = &hosts_online[idx];
            self.shard_leader
                .insert((space.to_string(), shard), host.id.clone());
        }
        Ok(())
    }

    pub fn rebalance_suggestion(&self, space: &str, partition_num: u16) -> Vec<(u64, String)> {
        // 简单 rebalance：统计每个 host 下 shard 数，差值 > 1 就建议迁移
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for shard in 0..partition_num as u64 {
            let key = (space.to_string(), shard);
            if let Some(h) = self.shard_leader.get(&key) {
                *counts.entry(h.clone()).or_default() += 1;
            }
        }
        let mut suggestions: Vec<(u64, String)> = Vec::new();
        if counts.is_empty() {
            return suggestions;
        }
        let avg: usize = (partition_num as usize) / counts.len().max(1);
        for (host_id, c) in &counts {
            if *c > avg + 1 {
                // 把第一部分 shard 迁移到最少的 host
                let least_host = counts
                    .iter()
                    .min_by_key(|(_, v)| **v)
                    .map(|(k, _)| k.clone())
                    .unwrap_or_default();
                if least_host != *host_id {
                    suggestions.push((0, least_host)); // 占位示例
                }
            }
        }
        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vid_hash_uniform_smoke() {
        let mut counts: BTreeMap<u64, usize> = BTreeMap::new();
        for i in 0..1000u64 {
            let vid = format!("user_{:08}", i);
            let shard = vid_hash_partition(&vid, 16);
            *counts.entry(shard).or_default() += 1;
        }
        let vals: Vec<f64> = counts.values().map(|v| *v as f64).collect();
        assert_eq!(vals.len(), 16);
        let mean = vals.iter().sum::<f64>() / vals.len() as f64;
        let variance: f64 =
            vals.iter().map(|v| (*v - mean).powi(2)).sum::<f64>() / vals.len() as f64;
        let stddev = variance.sqrt();
        let cv = stddev / mean;
        assert!(cv <= 0.15, "cv={} too large, counts={:?}", cv, counts);
    }
    #[test]
    fn register_host_and_route() {
        let mut ps = PartitionStore::new();
        ps.register_storage_host("h1", "127.0.0.1:9779").unwrap();
        ps.register_storage_host("h2", "127.0.0.1:9780").unwrap();
        ps.assign_all_shards("s1", 16, 3).unwrap();
        let (shard, addr) = ps.get_partition_route("s1", "user_001", 16).unwrap();
        assert!(shard < 16);
        assert!(addr.contains("127.0.0.1"));
    }
}
