// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::{CloudError, Result};
use crate::types::{
    BucketInfo, HashBlock, IamPolicy, LifecycleRule, MultipartUpload, QuotaConfig,
    StsToken, WormRetention,
};

/// In-memory fake cloud/S3 facade. Clones share state (Arc-like).
#[derive(Clone, Default)]
pub struct CloudClient {
    inner: std::sync::Arc<Mutex<CloudState>>,
}

#[derive(Default)]
pub(crate) struct CloudState {
    pub(crate) buckets: HashMap<String, BucketInfo>,
    pub(crate) objects: HashMap<(String, String), Vec<u8>>, // (bucket, key) -> bytes
    pub(crate) sts_tokens: HashMap<String, StsToken>,       // session token -> token
    pub(crate) iam_policies: HashMap<String, IamPolicy>,
    pub(crate) quotas: HashMap<String, QuotaConfig>,
    pub(crate) worms: HashMap<(String, String), WormRetention>, // (bucket, key)
    pub(crate) lifecycles: HashMap<String, Vec<LifecycleRule>>, // bucket -> rules
    pub(crate) hashchains: HashMap<String, Vec<HashBlock>>,     // chain id -> blocks
    pub(crate) multiparts: HashMap<String, MultipartUpload>,    // upload_id -> upload
}

/// Convenience alias used by examples (`use mox_cloud_sdk::Client;`).
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

    pub(crate) fn lock(&self) -> Result<std::sync::MutexGuard<'_, CloudState>> {
        self.inner.lock().map_err(|e| CloudError::Lock(e.to_string()))
    }
}
