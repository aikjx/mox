// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::client::CloudClient;
use crate::error::{CloudError, Result};
use crate::types::{LifecycleRule, LifecycleStats};

impl CloudClient {
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
}
