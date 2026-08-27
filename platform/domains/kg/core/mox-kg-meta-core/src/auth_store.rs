// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! AuthStore：用户名 + 加盐 SHA-256 密码 + role + policy。
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;

use crate::error::{MetaError, MetaResult};

pub type UserId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Role {
    #[default]
    ReadOnly,
    User,
    SpaceAdmin,
    Admin,
}

impl Role {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "ReadOnly" | "read_only" | "readOnly" => Role::ReadOnly,
            "User" | "user" => Role::User,
            "SpaceAdmin" | "space_admin" | "spaceAdmin" => Role::SpaceAdmin,
            "Admin" | "admin" => Role::Admin,
            _ => return None,
        })
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::ReadOnly => "ReadOnly",
            Role::User => "User",
            Role::SpaceAdmin => "SpaceAdmin",
            Role::Admin => "Admin",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resource(pub String);

impl Resource {
    pub fn space(name: &str) -> Self {
        Self(format!("space:{}", name))
    }
    pub fn all() -> Self {
        Self("*:*".to_string())
    }
    // 通配符匹配："*:*" 或 "*" 匹配一切；"space:*" 匹配任意空间；
    // "space:s1" 仅匹配 space:s1。
    pub fn matches(&self, other: &Resource) -> bool {
        let (pa, pr) = self.split();
        let (oa, or) = other.split();
        let part_ok = |p: &str, o: &str| -> bool { p == "*" || p == o };
        part_ok(&pa, &oa) && part_ok(&pr, &or)
    }
    fn split(&self) -> (String, String) {
        match self.0.find(':') {
            Some(i) => (self.0[..i].to_string(), self.0[i + 1..].to_string()),
            None => (self.0.clone(), "*".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserDef {
    pub username: String,
    pub salt_hex: String,
    pub password_hash_hex: String,
    pub role: Role,
    pub policies: Vec<Policy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    pub action: String, // 如 "tag.create" / "space.*" / "*.read"
    pub resource: Resource,
    pub allow: bool,
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn gen_salt_hex() -> String {
    let mut s = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut s);
    hex::encode(s)
}

fn hash_password(pw: &str, salt_hex: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt_hex.as_bytes());
    hasher.update(pw.as_bytes());
    hex::encode(hasher.finalize())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthStore {
    users: BTreeMap<String, UserDef>,
    pub(super) created_at: u64,
}

impl AuthStore {
    pub fn new() -> Self {
        Self {
            users: BTreeMap::new(),
            created_at: now_ms(),
        }
    }

    pub fn create_user(
        &mut self,
        username: &str,
        password: &str,
        role: Role,
    ) -> MetaResult<UserDef> {
        if self.users.contains_key(username) {
            return Err(MetaError::InvalidArgument(format!(
                "user {} exists",
                username
            )));
        }
        let salt = gen_salt_hex();
        let ph = hash_password(password, &salt);
        let u = UserDef {
            username: username.to_string(),
            salt_hex: salt,
            password_hash_hex: ph,
            role,
            policies: Self::default_policies_for_role(role),
        };
        self.users.insert(username.to_string(), u.clone());
        Ok(u)
    }

    fn default_policies_for_role(role: Role) -> Vec<Policy> {
        // 默认策略：Admin 全空间；SpaceAdmin 必须在空间维度 grant 才能拿到权限
        match role {
            Role::Admin => vec![Policy {
                action: "*".to_string(),
                resource: Resource::all(),
                allow: true,
            }],
            Role::ReadOnly => vec![Policy {
                action: "*.read".to_string(),
                resource: Resource::all(),
                allow: true,
            }],
            Role::User | Role::SpaceAdmin => vec![],
        }
    }

    pub fn grant_role(
        &mut self,
        username: &str,
        role: Role,
        resource: &Resource,
    ) -> MetaResult<()> {
        let u = self
            .users
            .get_mut(username)
            .ok_or_else(|| MetaError::UserNotFound(username.to_string()))?;
        u.role = role;
        // 授予 role 对应空间级 policy：SpaceAdmin → space.<res>.*
        let policy = match role {
            Role::SpaceAdmin => Policy {
                action: "space.*".to_string(),
                resource: resource.clone(),
                allow: true,
            },
            Role::Admin => Policy {
                action: "*".to_string(),
                resource: Resource::all(),
                allow: true,
            },
            Role::User => Policy {
                action: "*.write".to_string(),
                resource: resource.clone(),
                allow: true,
            },
            Role::ReadOnly => Policy {
                action: "*.read".to_string(),
                resource: resource.clone(),
                allow: true,
            },
        };
        // 去重
        if !u.policies.iter().any(|p| p == &policy) {
            u.policies.push(policy);
        }
        Ok(())
    }

    pub fn revoke_role(
        &mut self,
        username: &str,
        role: Role,
        resource: &Resource,
    ) -> MetaResult<()> {
        let u = self
            .users
            .get_mut(username)
            .ok_or_else(|| MetaError::UserNotFound(username.to_string()))?;
        let expected_action = match role {
            Role::SpaceAdmin => "space.*",
            Role::Admin => "*",
            Role::User => "*.write",
            Role::ReadOnly => "*.read",
        };
        u.policies
            .retain(|p| !(p.action == expected_action && p.resource.0 == resource.0));
        if matches!(role, Role::SpaceAdmin) && u.role == Role::SpaceAdmin {
            // 如果没有任何 SpaceAdmin policy，就回退为 User
            let any_space_admin = u.policies.iter().any(|p| p.action == "space.*");
            if !any_space_admin {
                u.role = Role::User;
            }
        }
        Ok(())
    }

    pub fn authenticate_user(&self, username: &str, password: &str) -> MetaResult<UserId> {
        let u = self
            .users
            .get(username)
            .ok_or_else(|| MetaError::AuthenticationFailed(username.to_string()))?;
        let ph = hash_password(password, &u.salt_hex);
        if ph != u.password_hash_hex {
            return Err(MetaError::AuthenticationFailed(username.to_string()));
        }
        Ok(username.to_string())
    }

    pub fn get_user(&self, username: &str) -> Option<&UserDef> {
        self.users.get(username)
    }

    fn action_matches(policy_action: &str, request_action: &str) -> bool {
        if policy_action == "*" {
            return true;
        }
        if request_action == policy_action {
            return true;
        }
        // "space.*" 匹配 "tag.create" / "edge.create" / "space.drop" 等资源级操作
        if policy_action == "space.*" {
            return request_action.starts_with("tag.")
                || request_action.starts_with("edge.")
                || request_action.starts_with("space.")
                || request_action.starts_with("partition.");
        }
        if policy_action == "*.read" {
            return request_action.ends_with(".read")
                || request_action == "list_spaces"
                || request_action == "list_tags"
                || request_action == "list_edges"
                || request_action == "show_hosts";
        }
        if policy_action == "*.write" {
            return request_action.ends_with(".create")
                || request_action.ends_with(".alter")
                || request_action.ends_with(".drop");
        }
        false
    }

    pub fn authorize(&self, who: &UserId, action: &str, res: &Resource) -> MetaResult<()> {
        let u = self
            .users
            .get(who)
            .ok_or_else(|| MetaError::UserNotFound(who.clone()))?;
        let mut allowed = false;
        for p in &u.policies {
            if Self::action_matches(&p.action, action) && p.resource.matches(res) {
                allowed = p.allow;
            }
        }
        if allowed {
            return Ok(());
        }
        Err(MetaError::AuthDenied {
            user: who.clone(),
            action: action.to_string(),
            resource: res.0.clone(),
        })
    }

    pub fn list_users(&self) -> Vec<UserDef> {
        self.users.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn auth_create_authenticate_roundtrip() {
        let mut a = AuthStore::new();
        a.create_user("alice", "pw123", Role::User).unwrap();
        let uid = a.authenticate_user("alice", "pw123").unwrap();
        assert_eq!(uid, "alice");
        assert!(matches!(
            a.authenticate_user("alice", "wrong"),
            Err(MetaError::AuthenticationFailed(_))
        ));
    }
    #[test]
    fn admin_always_allowed() {
        let mut a = AuthStore::new();
        a.create_user("admin0", "pw", Role::Admin).unwrap();
        let uid = a.authenticate_user("admin0", "pw").unwrap();
        a.authorize(&uid, "tag.create", &Resource::space("s1"))
            .unwrap();
    }
}
