// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use async_trait::async_trait;
use std::collections::BTreeMap;
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuotaInfo {
    pub max_bytes: u64,
    pub max_objects: u64,
    pub used_bytes: u64,
    pub used_objects: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UserQuota {
    pub user_id: String,
    pub quota: QuotaInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DirectoryQuota {
    pub path: String,
    pub quota: QuotaInfo,
}

#[derive(Debug, Clone, Default)]
struct QuotaEntry {
    max_bytes: u64,
    max_objects: u64,
    used_bytes: u64,
    used_objects: u64,
}
fn to_info(e: &QuotaEntry) -> QuotaInfo {
    QuotaInfo {
        max_bytes: e.max_bytes,
        max_objects: e.max_objects,
        used_bytes: e.used_bytes,
        used_objects: e.used_objects,
    }
}

#[async_trait]
pub trait QuotaProvider: Send + Sync {
    async fn get_user_quota(
        &self,
        user_id: &str,
    ) -> Result<QuotaInfo, Box<dyn Error + Send + Sync>>;
    async fn set_user_quota(
        &self,
        user_id: &str,
        max_bytes: u64,
        max_objects: u64,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn check_put_allowed(
        &self,
        user_id: &str,
        incoming_bytes: u64,
        incoming_objects: u64,
    ) -> Result<bool, Box<dyn Error + Send + Sync>>;
    async fn get_directory_quota(
        &self,
        path: &str,
    ) -> Result<QuotaInfo, Box<dyn Error + Send + Sync>>;
    async fn set_directory_quota(
        &self,
        path: &str,
        max_bytes: u64,
        max_objects: u64,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn check_directory_write_allowed(
        &self,
        path: &str,
        incoming_bytes: u64,
        incoming_objects: u64,
    ) -> Result<bool, Box<dyn Error + Send + Sync>>;
    async fn list_user_quotas(&self) -> Result<Vec<UserQuota>, Box<dyn Error + Send + Sync>>;
    async fn list_directory_quotas(
        &self,
    ) -> Result<Vec<DirectoryQuota>, Box<dyn Error + Send + Sync>>;
}

pub struct MockQuotaProvider {
    uq: parking_lot::Mutex<BTreeMap<String, QuotaEntry>>,
    dq: parking_lot::Mutex<BTreeMap<String, QuotaEntry>>,
}
impl Default for MockQuotaProvider {
    fn default() -> Self {
        Self {
            uq: parking_lot::Mutex::new(BTreeMap::new()),
            dq: parking_lot::Mutex::new(BTreeMap::new()),
        }
    }
}

#[async_trait]
impl QuotaProvider for MockQuotaProvider {
    async fn get_user_quota(
        &self,
        user_id: &str,
    ) -> Result<QuotaInfo, Box<dyn Error + Send + Sync>> {
        Ok(self.uq.lock().get(user_id).map(to_info).unwrap_or_default())
    }
    async fn set_user_quota(
        &self,
        user_id: &str,
        max_bytes: u64,
        max_objects: u64,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.uq.lock().entry(user_id.into()).or_default().max_bytes = max_bytes;
        self.uq
            .lock()
            .entry(user_id.into())
            .or_default()
            .max_objects = max_objects;
        Ok(())
    }
    async fn check_put_allowed(
        &self,
        user_id: &str,
        inc_b: u64,
        inc_o: u64,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let q = self.uq.lock();
        let e = q.get(user_id).ok_or("no user quota")?;
        Ok(e.used_bytes
            .checked_add(inc_b)
            .map(|v| v <= e.max_bytes)
            .unwrap_or(false)
            && e.used_objects
                .checked_add(inc_o)
                .map(|v| v <= e.max_objects)
                .unwrap_or(false))
    }
    async fn get_directory_quota(
        &self,
        path: &str,
    ) -> Result<QuotaInfo, Box<dyn Error + Send + Sync>> {
        Ok(self.dq.lock().get(path).map(to_info).unwrap_or_default())
    }
    async fn set_directory_quota(
        &self,
        path: &str,
        max_bytes: u64,
        max_objects: u64,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut dq = self.dq.lock();
        let e = dq.entry(path.into()).or_default();
        e.max_bytes = max_bytes;
        e.max_objects = max_objects;
        Ok(())
    }
    async fn check_directory_write_allowed(
        &self,
        path: &str,
        inc_b: u64,
        inc_o: u64,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let dq = self.dq.lock();
        let e = dq.get(path).ok_or("no dir quota")?;
        Ok(e.used_bytes
            .checked_add(inc_b)
            .map(|v| v <= e.max_bytes)
            .unwrap_or(false)
            && e.used_objects
                .checked_add(inc_o)
                .map(|v| v <= e.max_objects)
                .unwrap_or(false))
    }
    async fn list_user_quotas(&self) -> Result<Vec<UserQuota>, Box<dyn Error + Send + Sync>> {
        let uq = self.uq.lock();
        Ok(uq
            .iter()
            .map(|(k, v)| UserQuota {
                user_id: k.clone(),
                quota: to_info(v),
            })
            .collect())
    }
    async fn list_directory_quotas(
        &self,
    ) -> Result<Vec<DirectoryQuota>, Box<dyn Error + Send + Sync>> {
        let dq = self.dq.lock();
        Ok(dq
            .iter()
            .map(|(k, v)| DirectoryQuota {
                path: k.clone(),
                quota: to_info(v),
            })
            .collect())
    }
}
