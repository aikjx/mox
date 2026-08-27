// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: String,
    pub username: String,
    pub created_at: u64,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PolicyStatement {
    pub sid: String,
    pub effect: String,
    pub actions: Vec<String>,
    pub resources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RoleInfo {
    pub role_id: String,
    pub role_name: String,
    pub assume_role_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StsCredentials {
    pub access_key: String,
    pub secret_key: String,
    pub session_token: String,
    pub expiration: u64,
}

#[derive(Debug, Clone, Default)]
struct UserRecord {
    info: UserInfo,
    password_hash: String,
    policies: Vec<PolicyStatement>,
}

#[async_trait]
pub trait IamProvider: Send + Sync {
    async fn create_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<UserInfo, Box<dyn Error + Send + Sync>>;
    async fn delete_user(&self, user_id: &str) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<UserInfo, Box<dyn Error + Send + Sync>>;
    async fn authorize_policy(
        &self,
        user_id: &str,
        action: &str,
        resource: &str,
    ) -> Result<bool, Box<dyn Error + Send + Sync>>;
    async fn list_roles(&self) -> Result<Vec<RoleInfo>, Box<dyn Error + Send + Sync>>;
    async fn sts_assume_role(
        &self,
        role_id: &str,
        session_name: &str,
    ) -> Result<StsCredentials, Box<dyn Error + Send + Sync>>;
    async fn attach_policy(
        &self,
        user_id: &str,
        policy: PolicyStatement,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn detach_policy(
        &self,
        user_id: &str,
        sid: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn list_user_policies(
        &self,
        user_id: &str,
    ) -> Result<Vec<PolicyStatement>, Box<dyn Error + Send + Sync>>;
}

pub struct MockIamProvider {
    users: parking_lot::Mutex<BTreeMap<String, UserRecord>>,
    names: parking_lot::Mutex<BTreeMap<String, String>>,
    roles: parking_lot::Mutex<BTreeMap<String, RoleInfo>>,
    ctr: parking_lot::Mutex<u64>,
}
impl Default for MockIamProvider {
    fn default() -> Self {
        Self {
            users: parking_lot::Mutex::new(BTreeMap::new()),
            names: parking_lot::Mutex::new(BTreeMap::new()),
            roles: parking_lot::Mutex::new(BTreeMap::new()),
            ctr: parking_lot::Mutex::new(1),
        }
    }
}
fn hash(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}
fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[async_trait]
impl IamProvider for MockIamProvider {
    async fn create_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<UserInfo, Box<dyn Error + Send + Sync>> {
        let mut c = self.ctr.lock();
        let uid = format!("user-{}", *c);
        *c += 1;
        let info = UserInfo {
            user_id: uid.clone(),
            username: username.into(),
            created_at: now_ms(),
            enabled: true,
        };
        let rec = UserRecord {
            info: info.clone(),
            password_hash: hash(password),
            policies: vec![],
        };
        self.users.lock().insert(uid.clone(), rec);
        self.names.lock().insert(username.into(), uid);
        Ok(info)
    }
    async fn delete_user(&self, user_id: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut us = self.users.lock();
        if let Some(rec) = us.remove(user_id) {
            self.names.lock().remove(&rec.info.username);
        }
        Ok(())
    }
    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<UserInfo, Box<dyn Error + Send + Sync>> {
        let ns = self.names.lock();
        let uid = ns.get(username).ok_or("no such user")?.clone();
        drop(ns);
        let us = self.users.lock();
        let rec = us.get(&uid).ok_or("user gone")?;
        if rec.password_hash != hash(password) {
            return Err("bad password".into());
        }
        if !rec.info.enabled {
            return Err("disabled".into());
        }
        Ok(rec.info.clone())
    }
    async fn authorize_policy(
        &self,
        user_id: &str,
        action: &str,
        resource: &str,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let us = self.users.lock();
        let rec = us.get(user_id).ok_or("no user")?;
        let mut allowed = false;
        for p in &rec.policies {
            let a_match = p.actions.iter().any(|a| a == action || a == "*");
            let r_match = p
                .resources
                .iter()
                .any(|r| r == "*" || resource.starts_with(r.trim_end_matches('*')));
            if a_match && r_match {
                if p.effect == "Deny" {
                    return Ok(false);
                }
                if p.effect == "Allow" {
                    allowed = true;
                }
            }
        }
        Ok(allowed)
    }
    async fn list_roles(&self) -> Result<Vec<RoleInfo>, Box<dyn Error + Send + Sync>> {
        Ok(self.roles.lock().values().cloned().collect())
    }
    async fn sts_assume_role(
        &self,
        role_id: &str,
        session_name: &str,
    ) -> Result<StsCredentials, Box<dyn Error + Send + Sync>> {
        let salt = format!("{}-{}", role_id, session_name);
        Ok(StsCredentials {
            access_key: format!("AK-{}", hash(&salt[..salt.len().min(16)])),
            secret_key: hash(&(salt.clone() + "-sk")),
            session_token: hash(&(salt + "-st")),
            expiration: now_ms() + 3_600_000,
        })
    }
    async fn attach_policy(
        &self,
        user_id: &str,
        policy: PolicyStatement,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut us = self.users.lock();
        let rec = us.get_mut(user_id).ok_or("no user")?;
        rec.policies.push(policy);
        Ok(())
    }
    async fn detach_policy(
        &self,
        user_id: &str,
        sid: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut us = self.users.lock();
        let rec = us.get_mut(user_id).ok_or("no user")?;
        rec.policies.retain(|p| p.sid != sid);
        Ok(())
    }
    async fn list_user_policies(
        &self,
        user_id: &str,
    ) -> Result<Vec<PolicyStatement>, Box<dyn Error + Send + Sync>> {
        let us = self.users.lock();
        Ok(us.get(user_id).ok_or("no user")?.policies.clone())
    }
}
