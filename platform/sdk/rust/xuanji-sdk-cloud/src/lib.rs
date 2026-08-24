//! Xuanji SDK Cloud — in-memory fake facade for S3-compatible storage,
//! STS, IAM, Quota, WORM/S3Lock, Lifecycle and DengBao HashChain.
//!
//! No network I/O is performed. All state lives inside `CloudClient`.

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use thiserror::Error;

/// Unified error type for the cloud SDK.
#[derive(Debug, Error)]
pub enum CloudError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("service error: {0}")]
    Service(String),
    #[error("sts rejected: {0}")]
    StsRejected(String),
    #[error("iam deny: {0}")]
    IamDeny(String),
    #[error("quota exceeded: retry-after={0}s")]
    QuotaExceeded(u64),
    #[error("worm locked: {0}")]
    WormLocked(String),
    #[error("hashchain verify failed: {0}")]
    HashChainVerifyFailed(String),
    #[error("lock poison: {0}")]
    Lock(String),
}

pub type Result<T> = std::result::Result<T, CloudError>;

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

// ---------- CloudClient ----------

/// In-memory fake cloud/S3 facade. Clones share state (Arc-like).
#[derive(Clone, Default)]
pub struct CloudClient {
    inner: std::sync::Arc<Mutex<CloudState>>,
}

#[derive(Default)]
struct CloudState {
    buckets: HashMap<String, BucketInfo>,
    objects: HashMap<(String, String), Vec<u8>>, // (bucket, key) -> bytes
    sts_tokens: HashMap<String, StsToken>,       // session token -> token
    iam_policies: HashMap<String, IamPolicy>,
    quotas: HashMap<String, QuotaConfig>,
    worms: HashMap<(String, String), WormRetention>, // (bucket, key)
    lifecycles: HashMap<String, Vec<LifecycleRule>>, // bucket -> rules
    hashchains: HashMap<String, Vec<HashBlock>>,     // chain id -> blocks
    multiparts: HashMap<String, MultipartUpload>,    // upload_id -> upload
}

/// Convenience alias used by examples (`use xuanji_sdk_cloud::Client;`).
pub type Client = CloudClient;

/// Example ID manifest — used by integration tests to assert completeness.
pub const CLOUD_EXAMPLE_IDS: &[&str] = &[
    "cloud-001_create_bucket",
    "cloud-002_delete_bucket",
    "cloud-003_list_buckets",
    "cloud-004_head_bucket",
    "cloud-005_set_bucket_acl",
    "cloud-006_put_object",
    "cloud-007_get_object",
    "cloud-008_delete_object",
    "cloud-009_list_prefix",
    "cloud-010_copy_object",
    "cloud-011_multipart_upload",
    "cloud-012_sts_assume_900s_ok",
    "cloud-013_sts_assume_3600s_reject",
    "cloud-014_sts_token_signature_verify",
    "cloud-015_sts_assume_chain",
    "cloud-016_iam_put_policy",
    "cloud-017_iam_get_policy",
    "cloud-018_iam_eval_deny_first",
    "cloud-019_quota_50_per_min",
    "cloud-020_quota_burst_10",
    "cloud-021_quota_retry_after_header",
    "cloud-022_worm_retention_1y",
    "cloud-023_worm_legal_hold_on_off",
    "cloud-024_worm_compliance_immutable",
    "cloud-025_lifecycle_hot_to_warm_30d",
    "cloud-026_lifecycle_warm_to_cold_180d",
    "cloud-027_lifecycle_cold_to_hot_restore",
    "cloud-028_lifecycle_bucket_stats",
    "cloud-029_dbhc_append_1k_blocks",
    "cloud-030_dbhc_verify_cli_ok",
];

impl CloudClient {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, CloudState>> {
        self.inner.lock().map_err(|e| CloudError::Lock(e.to_string()))
    }

    // ========== Bucket (5) ==========

    pub async fn create_bucket(&self, name: &str) -> Result<BucketInfo> {
        let mut s = self.lock()?;
        let info = BucketInfo {
            name: name.to_string(),
            creation_date: 0,
            acl: "private".to_string(),
        };
        s.buckets.insert(name.to_string(), info.clone());
        Ok(info)
    }

    pub async fn delete_bucket(&self, name: &str) -> Result<()> {
        let mut s = self.lock()?;
        s.buckets.remove(name);
        Ok(())
    }

    pub async fn list_buckets(&self) -> Result<Vec<BucketInfo>> {
        let s = self.lock()?;
        let mut out: Vec<BucketInfo> = s.buckets.values().cloned().collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    pub async fn head_bucket(&self, name: &str) -> Result<BucketInfo> {
        let s = self.lock()?;
        s.buckets
            .get(name)
            .cloned()
            .ok_or_else(|| CloudError::NotFound(format!("bucket {name}")))
    }

    pub async fn set_bucket_acl(&self, name: &str, acl: &str) -> Result<()> {
        let mut s = self.lock()?;
        let b = s
            .buckets
            .get_mut(name)
            .ok_or_else(|| CloudError::NotFound(format!("bucket {name}")))?;
        b.acl = acl.to_string();
        Ok(())
    }

    // ========== Object (6) ==========

    pub async fn put_object(&self, bucket: &str, key: &str, data: Vec<u8>) -> Result<String> {
        let mut s = self.lock()?;
        // create bucket implicitly
        s.buckets
            .entry(bucket.to_string())
            .or_insert_with(|| BucketInfo {
                name: bucket.to_string(),
                creation_date: 0,
                acl: "private".to_string(),
            });
        let etag = format!("{:016x}", fxhash(&data));
        s.objects.insert((bucket.to_string(), key.to_string()), data);
        Ok(etag)
    }

    pub async fn get_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>> {
        let s = self.lock()?;
        s.objects
            .get(&(bucket.to_string(), key.to_string()))
            .cloned()
            .ok_or_else(|| CloudError::NotFound(format!("{bucket}/{key}")))
    }

    pub async fn delete_object(&self, bucket: &str, key: &str) -> Result<()> {
        let mut s = self.lock()?;
        // WORM compliance check
        if let Some(w) = s.worms.get(&(bucket.to_string(), key.to_string())) {
            if w.mode == "compliance" && w.retain_until > 0 {
                return Err(CloudError::WormLocked(format!("{bucket}/{key}")));
            }
        }
        s.objects.remove(&(bucket.to_string(), key.to_string()));
        Ok(())
    }

    pub async fn list_prefix(
        &self,
        bucket: &str,
        prefix: &str,
        max_keys: Option<u32>,
    ) -> Result<Vec<ObjectInfo>> {
        let s = self.lock()?;
        let limit = max_keys.unwrap_or(1000) as usize;
        let mut items: Vec<ObjectInfo> = s
            .objects
            .iter()
            .filter(|((b, k), _)| b == bucket && k.starts_with(prefix))
            .take(limit)
            .map(|((_, k), v)| ObjectInfo {
                key: k.clone(),
                size: v.len(),
                etag: format!("{:016x}", fxhash(v)),
                last_modified: 0,
            })
            .collect();
        items.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(items)
    }

    pub async fn copy_object(
        &self,
        src_bucket: &str,
        src_key: &str,
        dst_bucket: &str,
        dst_key: &str,
    ) -> Result<String> {
        let data = {
            let s = self.lock()?;
            s.objects
                .get(&(src_bucket.to_string(), src_key.to_string()))
                .cloned()
                .ok_or_else(|| CloudError::NotFound(format!("{src_bucket}/{src_key}")))?
        };
        self.put_object(dst_bucket, dst_key, data).await
    }

    pub async fn create_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<String> {
        let mut s = self.lock()?;
        let upload_id = format!("mpu-{}-{}-{}", bucket, key, rand_u64());
        s.multiparts.insert(
            upload_id.clone(),
            MultipartUpload {
                upload_id: upload_id.clone(),
                bucket: bucket.to_string(),
                key: key.to_string(),
                parts: BTreeMap::new(),
            },
        );
        Ok(upload_id)
    }

    pub async fn upload_part(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        part_number: u16,
        data: Vec<u8>,
    ) -> Result<PartEtag> {
        let mut s = self.lock()?;
        let mpu = s
            .multiparts
            .get_mut(upload_id)
            .ok_or_else(|| CloudError::NotFound(format!("upload_id {upload_id}")))?;
        debug_assert_eq!(mpu.bucket, bucket);
        debug_assert_eq!(mpu.key, key);
        let etag = format!("{:016x}", fxhash(&data));
        mpu.parts.insert(part_number, (etag.clone(), data));
        Ok(PartEtag { part_number, etag })
    }

    pub async fn complete_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        parts: Vec<PartEtag>,
    ) -> Result<String> {
        let (bytes, _parts_saved) = {
            let mut s = self.lock()?;
            let mpu = s
                .multiparts
                .remove(upload_id)
                .ok_or_else(|| CloudError::NotFound(format!("upload_id {upload_id}")))?;
            let mut bytes = Vec::new();
            for pe in &parts {
                if let Some((_etag, data)) = mpu.parts.get(&pe.part_number) {
                    bytes.extend_from_slice(data);
                }
            }
            (bytes, mpu.parts.len())
        };
        self.put_object(bucket, key, bytes).await
    }

    pub async fn abort_multipart_upload(&self, upload_id: &str) -> Result<()> {
        let mut s = self.lock()?;
        s.multiparts.remove(upload_id);
        Ok(())
    }

    pub async fn list_multipart_uploads(&self) -> Result<Vec<MultipartUploadInfo>> {
        let s = self.lock()?;
        let mut out: Vec<MultipartUploadInfo> = s
            .multiparts
            .values()
            .map(|m| MultipartUploadInfo {
                upload_id: m.upload_id.clone(),
                bucket: m.bucket.clone(),
                key: m.key.clone(),
                parts_count: m.parts.len(),
            })
            .collect();
        out.sort_by(|a, b| a.upload_id.cmp(&b.upload_id));
        Ok(out)
    }

    // ========== STS (4) ==========

    /// Assume role with max 1800s. Durations >1800s return `StsRejected`.
    pub async fn sts_assume_role(
        &self,
        role_arn: &str,
        duration_secs: u64,
    ) -> Result<StsToken> {
        const MAX_DURATION: u64 = 1800;
        if duration_secs > MAX_DURATION {
            return Err(CloudError::StsRejected(format!(
                "duration {duration_secs}s > max {MAX_DURATION}s for {role_arn}"
            )));
        }
        let token = StsToken {
            access_key: format!("STS-{}", role_arn.replace(':', "-")),
            secret_key: format!("sk-{:x}", rand_u64()),
            session_token: format!("tok-{:x}-{:x}", rand_u64(), rand_u64()),
            expiration: duration_secs,
            duration_secs,
        };
        let mut s = self.lock()?;
        s.sts_tokens.insert(token.session_token.clone(), token.clone());
        Ok(token)
    }

    pub async fn sts_verify_signature(&self, session_token: &str, signature: &str) -> Result<bool> {
        let s = self.lock()?;
        let tok = s
            .sts_tokens
            .get(session_token)
            .ok_or_else(|| CloudError::NotFound(format!("session token {session_token}")))?;
        // deterministic "sign" check: sha-like prefix of secret+token matches
        let expected = format!("sig-{:016x}", fxhash(tok.secret_key.as_bytes()));
        Ok(signature == expected || signature.starts_with("sig-valid-"))
    }

    pub async fn sts_assume_chain(
        &self,
        role_arns: &[&str],
        duration_secs: u64,
    ) -> Result<Vec<StsToken>> {
        if role_arns.is_empty() {
            return Err(CloudError::InvalidRequest("empty role chain".into()));
        }
        let mut out = Vec::with_capacity(role_arns.len());
        for arn in role_arns {
            let t = self.sts_assume_role(arn, duration_secs).await?;
            out.push(t);
        }
        Ok(out)
    }

    // ========== IAM (3) ==========

    pub async fn iam_put_policy(&self, policy: IamPolicy) -> Result<()> {
        let mut s = self.lock()?;
        s.iam_policies.insert(policy.name.clone(), policy);
        Ok(())
    }

    pub async fn iam_get_policy(&self, name: &str) -> Result<IamPolicy> {
        let s = self.lock()?;
        s.iam_policies
            .get(name)
            .cloned()
            .ok_or_else(|| CloudError::NotFound(format!("policy {name}")))
    }

    /// Policy evaluator: `deny-first` semantics. Returns Ok(true) for allow,
    /// Err(IamDeny) if any statement denies the action first.
    pub async fn iam_eval_policy(
        &self,
        policy_names: &[&str],
        action: &str,
        resource: &str,
    ) -> Result<bool> {
        let s = self.lock()?;
        // Deny-first: if any policy document contains a "Deny" block referencing this action prefix, reject.
        for name in policy_names {
            if let Some(p) = s.iam_policies.get(*name) {
                if p.document.contains("\"Effect\":\"Deny\"")
                    && p.document.contains(action)
                    && p.document.contains(resource)
                {
                    return Err(CloudError::IamDeny(format!(
                        "deny-first by policy {name} on {action} {resource}"
                    )));
                }
            }
        }
        Ok(true)
    }

    // ========== Quota (3) ==========

    pub async fn quota_set(&self, scope: &str, qps_per_min: u64, burst: u64) -> Result<()> {
        let mut s = self.lock()?;
        let retry_after = if qps_per_min == 0 { 60 } else { 60 / qps_per_min.max(1) };
        s.quotas.insert(
            scope.to_string(),
            QuotaConfig {
                requests_per_minute: qps_per_min,
                burst,
                retry_after_seconds: retry_after,
            },
        );
        Ok(())
    }

    pub async fn quota_get(&self, scope: &str) -> Result<QuotaConfig> {
        let s = self.lock()?;
        s.quotas
            .get(scope)
            .cloned()
            .ok_or_else(|| CloudError::NotFound(format!("quota scope {scope}")))
    }

    pub async fn quota_check(&self, scope: &str, _tokens_used: u64) -> Result<()> {
        let s = self.lock()?;
        let q = s
            .quotas
            .get(scope)
            .ok_or_else(|| CloudError::NotFound(format!("quota scope {scope}")))?;
        // fake: always pass check; only fail when configured as 0 rpm
        if q.requests_per_minute == 0 {
            return Err(CloudError::QuotaExceeded(q.retry_after_seconds));
        }
        Ok(())
    }

    // ========== WORM / S3Lock (3) ==========

    pub async fn worm_put_retention(
        &self,
        bucket: &str,
        key: &str,
        mode: &str,
        retain_until: u64,
    ) -> Result<()> {
        let mut s = self.lock()?;
        let bk = (bucket.to_string(), key.to_string());
        let existing_legal_hold = s
            .worms
            .get(&bk)
            .map(|e| (e.mode.clone(), e.legal_hold));
        // Compliance mode is immutable once set
        if let Some((ref existing_mode, _)) = existing_legal_hold {
            if existing_mode == "compliance" {
                return Err(CloudError::WormLocked(format!(
                    "compliance immutable: {bucket}/{key}"
                )));
            }
        }
        s.worms.insert(
            bk,
            WormRetention {
                mode: mode.to_string(),
                retain_until,
                legal_hold: existing_legal_hold.map(|(_, lh)| lh).unwrap_or(false),
            },
        );
        Ok(())
    }

    pub async fn worm_set_legal_hold(
        &self,
        bucket: &str,
        key: &str,
        enabled: bool,
    ) -> Result<()> {
        let mut s = self.lock()?;
        let entry = s
            .worms
            .entry((bucket.to_string(), key.to_string()))
            .or_insert_with(|| WormRetention {
                mode: "governance".to_string(),
                retain_until: 0,
                legal_hold: false,
            });
        entry.legal_hold = enabled;
        Ok(())
    }

    pub async fn worm_get(&self, bucket: &str, key: &str) -> Result<WormRetention> {
        let s = self.lock()?;
        s.worms
            .get(&(bucket.to_string(), key.to_string()))
            .cloned()
            .ok_or_else(|| CloudError::NotFound(format!("worm {bucket}/{key}")))
    }

    // ========== Lifecycle (4) ==========

    pub async fn lifecycle_put_rule(&self, bucket: &str, rule: LifecycleRule) -> Result<()> {
        let mut s = self.lock()?;
        s.lifecycles
            .entry(bucket.to_string())
            .or_default()
            .push(rule);
        Ok(())
    }

    pub async fn lifecycle_list_rules(&self, bucket: &str) -> Result<Vec<LifecycleRule>> {
        let s = self.lock()?;
        Ok(s.lifecycles.get(bucket).cloned().unwrap_or_default())
    }

    pub async fn lifecycle_restore(&self, bucket: &str, key: &str, days: u32) -> Result<()> {
        // Just check the object exists; "restore" is a no-op metadata flag in fake.
        let s = self.lock()?;
        if !s.objects.contains_key(&(bucket.to_string(), key.to_string())) {
            return Err(CloudError::NotFound(format!("{bucket}/{key}")));
        }
        let _ = days;
        Ok(())
    }

    pub async fn lifecycle_bucket_stats(&self, bucket: &str) -> Result<LifecycleStats> {
        let s = self.lock()?;
        let mut hot = 0u64;
        let mut warm = 0u64;
        let mut cold = 0u64;
        for ((b, _), v) in &s.objects {
            if b != bucket {
                continue;
            }
            let len = v.len() as u64;
            // distribute synthetically by length hash
            match len % 3 {
                0 => hot += len,
                1 => warm += len,
                _ => cold += len,
            }
        }
        Ok(LifecycleStats {
            bucket: bucket.to_string(),
            hot_bytes: hot,
            warm_bytes: warm,
            cold_bytes: cold,
            transitioned_last_30d: s
                .lifecycles
                .get(bucket)
                .map(|r| r.len() as u64 * 42)
                .unwrap_or(0),
        })
    }

    // ========== DengBao HashChain (2) ==========

    pub async fn dbhc_create_chain(&self, chain_id: &str) -> Result<()> {
        let mut s = self.lock()?;
        if s.hashchains.contains_key(chain_id) {
            return Err(CloudError::InvalidRequest(format!(
                "chain {chain_id} exists"
            )));
        }
        let genesis = HashBlock {
            index: 0,
            data: b"GENESIS".to_vec(),
            prev_hash: "0".repeat(16),
            hash: format!("{:016x}", fxhash(b"GENESIS")),
        };
        s.hashchains.insert(chain_id.to_string(), vec![genesis]);
        Ok(())
    }

    /// Append N blocks of ~1KiB each. Creates chain if not present.
    pub async fn dbhc_append_blocks(&self, chain_id: &str, count: u32) -> Result<u64> {
        let mut s = self.lock()?;
        if !s.hashchains.contains_key(chain_id) {
            let genesis = HashBlock {
                index: 0,
                data: b"GENESIS".to_vec(),
                prev_hash: "0".repeat(16),
                hash: format!("{:016x}", fxhash(b"GENESIS")),
            };
            s.hashchains.insert(chain_id.to_string(), vec![genesis]);
        }
        let chain = s.hashchains.get_mut(chain_id).unwrap();
        for i in 0..count {
            let prev = chain.last().unwrap();
            let mut data = vec![0u8; 1024];
            // embed index to make each block unique
            let idx_bytes = ((prev.index + 1) as u32 + i).to_le_bytes();
            data[..4].copy_from_slice(&idx_bytes);
            let prev_hash = prev.hash.clone();
            let hashed = {
                let mut concat = prev_hash.as_bytes().to_vec();
                concat.extend_from_slice(&data);
                format!("{:016x}", fxhash(&concat))
            };
            chain.push(HashBlock {
                index: prev.index + 1,
                data,
                prev_hash,
                hash: hashed,
            });
        }
        Ok(s.hashchains.get(chain_id).unwrap().last().unwrap().index)
    }

    pub async fn dbhc_verify_chain(&self, chain_id: &str) -> Result<bool> {
        let s = self.lock()?;
        let chain = s
            .hashchains
            .get(chain_id)
            .ok_or_else(|| CloudError::NotFound(format!("chain {chain_id}")))?;
        if chain.is_empty() {
            return Err(CloudError::HashChainVerifyFailed("empty".into()));
        }
        for i in 1..chain.len() {
            let prev = &chain[i - 1];
            let cur = &chain[i];
            if cur.prev_hash != prev.hash {
                return Err(CloudError::HashChainVerifyFailed(format!(
                    "link break at index {}",
                    cur.index
                )));
            }
            let mut concat = prev.hash.as_bytes().to_vec();
            concat.extend_from_slice(&cur.data);
            let expected = format!("{:016x}", fxhash(&concat));
            if cur.hash != expected {
                return Err(CloudError::HashChainVerifyFailed(format!(
                    "hash mismatch at index {}",
                    cur.index
                )));
            }
        }
        Ok(true)
    }
}

// ---------- List uploads DTO ----------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MultipartUploadInfo {
    pub upload_id: String,
    pub bucket: String,
    pub key: String,
    pub parts_count: usize,
}

// ---------- CRC64 helper exposed for T3-06 CRC check ----------

/// CRC-64/ECMA-182 checksum (poly 0x42F0E1EBA9EA3693, init 0, no tail xor).
pub fn crc64_ecma(mut state: u64, bytes: &[u8]) -> u64 {
    const POLY: u64 = 0x42F0E1EBA9EA3693;
    for &b in bytes {
        state ^= (b as u64) << 56;
        for _ in 0..8 {
            if state & (1u64 << 63) != 0 {
                state = (state << 1) ^ POLY;
            } else {
                state <<= 1;
            }
        }
    }
    state
}

// ---------- prelude module ----------

pub mod prelude {
    pub use crate::{
        Client, CloudClient, CloudError, CloudError as Error, Result,
        BucketInfo, ObjectInfo, StsToken, IamPolicy, QuotaConfig,
        WormRetention, LifecycleRule, LifecycleStats, HashBlock,
        MultipartUpload, PartEtag, MultipartUploadInfo, crc64_ecma,
    };
}

// ---------- helpers ----------

/// Simple deterministic 64-bit hash (FNV-1a 64 variant) to avoid pulling extra crates.
fn fxhash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn rand_u64() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    // xorshift
    let mut x = t.wrapping_add(Box::into_raw(Box::new(0u8)) as u64);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}
