//! Persistent user credentials. Administrative mutations and audit share one transaction.
use crate::{
    permission_admin::{audit, check_admin, AdminError},
    IamRepository,
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub enum KeyCommand {
    List { page: u32, page_size: u32 },
    Create { name: String, expires_at: Option<String> },
    Revoke(String),
}

#[derive(Debug)]
pub struct KeyPrincipal {
    pub id: String,
    pub tenant_id: String,
    pub username: String,
    pub email: String,
}

fn digest(key: &str) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(key.as_bytes())))
}

fn expiry_state(value: Option<&str>, now: chrono::DateTime<chrono::Utc>) -> Option<&'static str> {
    let value = value?;
    match chrono::DateTime::parse_from_rfc3339(value) {
        Err(_) => Some("invalid_expiry"),
        Ok(expiry) if expiry <= now => Some("expired"),
        Ok(_) => None,
    }
}

impl IamRepository {
    /// Newly issued credentials belong to the authenticated administrator, never a default user.
    /// Scoped credentials are deliberately unsupported until route-level enforcement exists.
    pub fn administer_api_keys(
        &self,
        tenant: &str,
        actor: &str,
        command: KeyCommand,
    ) -> Result<Value, AdminError> {
        let mut conn = self.conn.lock();
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_admin(&tx, tenant, actor)?;
        let now = chrono::Utc::now();
        let result = match command {
            KeyCommand::List { page, page_size } => {
                if page == 0 || !(1..=100).contains(&page_size) {
                    return Err(AdminError::Invalid);
                }
                let total: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM sys_api_key WHERE tenant_id=?1",
                    [tenant],
                    |r| r.get(0),
                )?;
                let offset = i64::from(page - 1) * i64::from(page_size);
                let mut stmt = tx.prepare("SELECT k.key_id,k.name,k.user_id,k.status,k.expires_at,k.last_used_at,k.created_at,k.scopes,u.user_status,t.tenant_status
                    FROM sys_api_key k LEFT JOIN iam_user u ON u.user_id=k.user_id AND u.tenant_id=k.tenant_id LEFT JOIN iam_tenant t ON t.tenant_id=k.tenant_id
                    WHERE k.tenant_id=?1 ORDER BY k.created_at DESC,k.key_id ASC LIMIT ?2 OFFSET ?3")?;
                let items = stmt.query_map(params![tenant,page_size,offset], |r| {
                    let status: String = r.get(3)?;
                    let expires: Option<String> = r.get(4)?;
                    let scopes: Option<String> = r.get(7)?;
                    let user_status: Option<String> = r.get(8)?;
                    let tenant_status: Option<String> = r.get(9)?;
                    let eligibility = if status != "active" { "revoked_or_inactive" }
                        else if scopes.as_deref().is_some_and(|s|!s.is_empty()) { "unsupported_scope" }
                        else if user_status.is_none() { "missing_identity" }
                        else if user_status.as_deref() != Some("active") { "inactive_user" }
                        else if tenant_status.as_deref() != Some("active") { "inactive_tenant" }
                        else { expiry_state(expires.as_deref(),now).unwrap_or("eligible") };
                    Ok(json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"user_id":r.get::<_,Option<String>>(2)?,"active":status == "active","status":status,"eligibility":eligibility,"expires_at":expires,"last_used_at":r.get::<_,Option<String>>(5)?,"createdAt":r.get::<_,String>(6)?}))
                })?.collect::<Result<Vec<_>,_>>()?;
                json!({"items":items,"total":total,"page":page,"page_size":page_size})
            },
            KeyCommand::Create { name, expires_at } => {
                if name.trim().is_empty() || name.chars().count() > 100 {
                    return Err(AdminError::Invalid);
                }
                let expires_at = match expires_at {
                    Some(value) => {
                        let date = chrono::DateTime::parse_from_rfc3339(&value)
                            .map_err(|_| AdminError::Invalid)?;
                        if date <= now {
                            return Err(AdminError::Invalid);
                        }
                        Some(date.to_rfc3339())
                    },
                    None => None,
                };
                let id = uuid::Uuid::new_v4().to_string();
                let key = format!(
                    "mox_{}{}",
                    uuid::Uuid::new_v4().simple(),
                    uuid::Uuid::new_v4().simple()
                );
                tx.execute("INSERT INTO sys_api_key(key_id,tenant_id,name,api_key,user_id,status,expires_at,created_at) VALUES (?1,?2,?3,?4,?5,'active',?6,?7)",params![id,tenant,name.trim(),digest(&key),actor,expires_at,now.to_rfc3339()])?;
                audit(
                    &tx,
                    tenant,
                    actor,
                    "iam.api_key.create",
                    &id,
                    Value::Null,
                    json!({"name":name.trim(),"user_id":actor,"expires_at":expires_at}),
                )?;
                json!({"id":id,"name":name.trim(),"api_key":key,"user_id":actor,"active":true,"expires_at":expires_at,"createdAt":now.to_rfc3339()})
            },
            KeyCommand::Revoke(id) => {
                let status: Option<String> = tx
                    .query_row(
                        "SELECT status FROM sys_api_key WHERE tenant_id=?1 AND key_id=?2",
                        params![tenant, id],
                        |r| r.get(0),
                    )
                    .optional()?;
                let status = status.ok_or(AdminError::NotFound)?;
                if status != "revoked" {
                    tx.execute("UPDATE sys_api_key SET status='revoked',revoked_at=?3 WHERE tenant_id=?1 AND key_id=?2",params![tenant,id,now.to_rfc3339()])?;
                    audit(
                        &tx,
                        tenant,
                        actor,
                        "iam.api_key.revoke",
                        &id,
                        json!({"status":status}),
                        json!({"status":"revoked"}),
                    )?;
                }
                Value::Null
            },
        };
        tx.commit()?;
        Ok(result)
    }

    /// Every request rechecks the persisted key, expiry, user and tenant. No identity cache.
    pub fn authenticate_api_key(&self, key: &str) -> Result<Option<KeyPrincipal>, AdminError> {
        self.authenticate_key_in_scope(key, None)
    }

    pub fn authenticate_api_key_in_tenant(
        &self,
        key: &str,
        tenant: &str,
    ) -> Result<Option<KeyPrincipal>, AdminError> {
        self.authenticate_key_in_scope(key, Some(tenant))
    }

    fn authenticate_key_in_scope(
        &self,
        key: &str,
        tenant: Option<&str>,
    ) -> Result<Option<KeyPrincipal>, AdminError> {
        if key.is_empty() || key.len() > 256 || key.starts_with("sha256:") {
            return Ok(None);
        }
        let mut conn = self.conn.lock();
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let hash = digest(key);
        let record = tx.query_row("SELECT k.key_id,k.api_key,k.expires_at,u.user_id,u.tenant_id,u.username,COALESCE(u.email,'') FROM sys_api_key k JOIN iam_user u ON u.user_id=k.user_id AND u.tenant_id=k.tenant_id JOIN iam_tenant t ON t.tenant_id=u.tenant_id WHERE k.api_key IN (?1,?2) AND (?3 IS NULL OR k.tenant_id=?3) AND (SELECT COUNT(*) FROM sys_api_key WHERE api_key IN (?1,?2))=1 AND k.status='active' AND u.user_status='active' AND t.tenant_status='active' AND (k.scopes IS NULL OR k.scopes='') ORDER BY k.key_id LIMIT 1",params![hash,key,tenant],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,KeyPrincipal{id:r.get(3)?,tenant_id:r.get(4)?,username:r.get(5)?,email:r.get(6)?}))).optional()?;
        let Some((id, stored, expiry, principal)) = record else {
            return Ok(None);
        };
        if expiry_state(expiry.as_deref(), chrono::Utc::now()).is_some() {
            return Ok(None);
        }
        // Valid legacy plaintext credentials are upgraded without exposing the hash as a key.
        tx.execute(
            "UPDATE sys_api_key SET api_key=?2,last_used_at=?3 WHERE key_id=?1",
            params![
                id,
                if stored == hash { stored } else { hash },
                chrono::Utc::now().to_rfc3339()
            ],
        )?;
        tx.commit()?;
        Ok(Some(principal))
    }
}
