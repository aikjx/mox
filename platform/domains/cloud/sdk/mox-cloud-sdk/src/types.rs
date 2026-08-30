// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use std::collections::BTreeMap;

// ---------- common DTOs ----------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct BucketInfo {
    pub name: String,
    pub creation_date: u64,
    pub acl: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ObjectInfo {
    pub key: String,
    pub size: usize,
    pub etag: String,
    pub last_modified: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StsToken {
    pub access_key: String,
    pub secret_key: String,
    pub session_token: String,
    pub expiration: u64,
    pub duration_secs: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IamPolicy {
    pub name: String,
    pub document: String,
    pub version: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuotaConfig {
    pub requests_per_minute: u64,
    pub burst: u64,
    pub retry_after_seconds: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WormRetention {
    pub mode: String, // "governance" | "compliance"
    pub retain_until: u64,
    pub legal_hold: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LifecycleRule {
    pub id: String,
    pub from_storage_class: String,
    pub to_storage_class: String,
    pub after_days: u32,
    pub prefix: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LifecycleStats {
    pub bucket: String,
    pub hot_bytes: u64,
    pub warm_bytes: u64,
    pub cold_bytes: u64,
    pub transitioned_last_30d: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HashBlock {
    pub index: u64,
    pub data: Vec<u8>,
    pub prev_hash: String,
    pub hash: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MultipartUpload {
    pub upload_id: String,
    pub bucket: String,
    pub key: String,
    pub parts: BTreeMap<u16, (String, Vec<u8>)>, // part_number -> (etag, bytes)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PartEtag {
    pub part_number: u16,
    pub etag: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MultipartUploadInfo {
    pub upload_id: String,
    pub bucket: String,
    pub key: String,
    pub parts_count: usize,
}
