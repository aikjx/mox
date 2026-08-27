// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Bucket Versioning 状态机：Off / Enabled / Suspended。
//! 同一对象多次写入 → 保留多版本（VersionId = 随机 32 hex + 加盐）。

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VersioningStatus {
    #[default]
    Off,
    Enabled,
    Suspended,
}

impl VersioningStatus {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Enabled" => Some(VersioningStatus::Enabled),
            "Suspended" => Some(VersioningStatus::Suspended),
            "Off" | "" => Some(VersioningStatus::Off),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            VersioningStatus::Off => "Off",
            VersioningStatus::Enabled => "Enabled",
            VersioningStatus::Suspended => "Suspended",
        }
    }

    pub fn should_generate_version(&self) -> bool {
        matches!(self, VersioningStatus::Enabled)
    }
}

/// 生成新的 VersionId：32 hex（128 bit，加盐哈希）。
pub fn generate_version_id(key: &str, ts_ms: u64, counter: u64) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut h1 = Sha256::new();
    h1.update(b"mox-rand-entropy-v1");
    h1.update(key.as_bytes());
    h1.update(ts_ms.to_be_bytes());
    h1.update(counter.to_be_bytes());
    h1.update(nanos.to_be_bytes());
    let tid = format!("{:?}", std::thread::current().id());
    h1.update(tid.as_bytes());
    let r1 = h1.finalize();
    let mut random_bytes = [0u8; 16];
    random_bytes.copy_from_slice(&r1[..16]);
    let mut h = Sha256::new();
    h.update(b"mox-version-salt-v1");
    h.update(key.as_bytes());
    h.update(ts_ms.to_be_bytes());
    h.update(counter.to_be_bytes());
    h.update(random_bytes);
    let d = h.finalize();
    hex::encode(&d[..16])
}

#[derive(Debug, Default)]
pub struct VersioningManager {
    pub statuses: Mutex<BTreeMap<String, VersioningStatus>>,
}

impl VersioningManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, bucket: &str) -> VersioningStatus {
        self.statuses
            .lock()
            .get(bucket)
            .copied()
            .unwrap_or(VersioningStatus::Off)
    }

    pub fn set(&self, bucket: &str, status: VersioningStatus) {
        self.statuses.lock().insert(bucket.to_string(), status);
    }
}
