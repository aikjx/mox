// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::error::{MasterError, MasterResult};
use hex::ToHex;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    time::{SystemTime, UNIX_EPOCH},
};

pub type SnapshotId = String;
pub type VolumeId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub snapshot_id: SnapshotId,
    pub volume_id: VolumeId,
    pub created_at: u64,
    pub deleted_at: Option<u64>,
    pub chunk_count: u64,
    pub chunk_manifest: BTreeMap<String, Vec<u8>>,
}

pub struct SnapshotManager {
    snapshots: parking_lot::Mutex<HashMap<VolumeId, Vec<SnapshotMeta>>>,
    snapshots_taken: parking_lot::Mutex<u64>,
}

impl SnapshotManager {
    pub fn new() -> Self {
        Self {
            snapshots: parking_lot::Mutex::new(HashMap::new()),
            snapshots_taken: parking_lot::Mutex::new(0),
        }
    }

    pub fn create_salt() -> String {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 16];
        rng.fill(&mut bytes);
        bytes.encode_hex::<String>()
    }

    /// snapshot_id = hex(sha256(volume_id + salt + timestamp)) — 不可伪造
    pub fn generate_snapshot_id(volume_id: &str, salt: &str, timestamp: u64) -> SnapshotId {
        let mut h = Sha256::new();
        h.update(volume_id.as_bytes());
        h.update(b"|");
        h.update(salt.as_bytes());
        h.update(b"|");
        h.update(timestamp.to_le_bytes());
        h.finalize().encode_hex::<String>()
    }

    pub fn take_snapshot(
        &self,
        volume_id: &str,
        chunk_manifest: BTreeMap<String, Vec<u8>>,
    ) -> MasterResult<SnapshotId> {
        let ts = now_millis();
        let salt = Self::create_salt();
        let sid = Self::generate_snapshot_id(volume_id, &salt, ts);

        let chunk_count = chunk_manifest.len() as u64;
        let meta = SnapshotMeta {
            snapshot_id: sid.clone(),
            volume_id: volume_id.to_string(),
            created_at: ts,
            deleted_at: None,
            chunk_count,
            chunk_manifest,
        };

        let mut map = self.snapshots.lock();
        map.entry(volume_id.to_string()).or_default().push(meta);

        let mut cnt = self.snapshots_taken.lock();
        *cnt += 1;
        drop(cnt);

        Ok(sid)
    }

    pub fn get_snapshot(&self, volume_id: &str, snapshot_id: &str) -> MasterResult<SnapshotMeta> {
        let map = self.snapshots.lock();
        let list = map.get(volume_id).ok_or_else(|| {
            MasterError::SnapshotInvalid(format!("volume {} has no snapshots", volume_id))
        })?;
        list.iter()
            .find(|m| m.snapshot_id == snapshot_id && m.deleted_at.is_none())
            .cloned()
            .ok_or_else(|| {
                MasterError::SnapshotInvalid(format!(
                    "snapshot {} not found or deleted",
                    snapshot_id
                ))
            })
    }

    pub fn list_snapshots(&self, volume_id: &str) -> Vec<SnapshotMeta> {
        self.snapshots
            .lock()
            .get(volume_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|m| m.deleted_at.is_none())
            .collect()
    }

    pub fn soft_delete_snapshot(&self, volume_id: &str, snapshot_id: &str) -> MasterResult<()> {
        let mut map = self.snapshots.lock();
        let list = map.get_mut(volume_id).ok_or_else(|| {
            MasterError::SnapshotInvalid(format!("volume {} no snapshots", volume_id))
        })?;
        for m in list.iter_mut() {
            if m.snapshot_id == snapshot_id && m.deleted_at.is_none() {
                m.deleted_at = Some(now_millis());
                return Ok(());
            }
        }
        Err(MasterError::SnapshotInvalid(snapshot_id.to_string()))
    }

    /// 删除快照（软删除，标记 deleted_at 时间戳）
    pub fn delete_snapshot(&self, volume_id: &str, snapshot_id: &str) -> MasterResult<()> {
        self.soft_delete_snapshot(volume_id, snapshot_id)
    }

    pub fn snapshots_taken_count(&self) -> u64 {
        *self.snapshots_taken.lock()
    }
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
