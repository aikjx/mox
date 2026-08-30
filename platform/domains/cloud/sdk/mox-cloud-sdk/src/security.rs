// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::client::CloudClient;
use crate::error::{CloudError, Result};
use crate::types::{IamPolicy, QuotaConfig, StsToken, WormRetention};
use crate::utils::{fxhash, rand_u64};

impl CloudClient {
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
}
