//! Tenant-scoped, fresh administrator checks and transactional direct permission grants.
use crate::IamRepository;
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    #[error("Forbidden")]
    Forbidden,
    #[error("Not found")]
    NotFound,
    #[error("Conflict")]
    Conflict,
    #[error("Invalid input")]
    Invalid,
    #[error("Storage unavailable: {0}")]
    Storage(#[from] rusqlite::Error),
}

pub enum PermissionCommand {
    Catalog,
    RegisterMessageSend,
    Role(String),
    ReplaceRole { role: String, version: i64, permissions: Vec<String> },
}

pub(crate) fn check_admin(
    tx: &Transaction<'_>,
    tenant: &str,
    actor: &str,
) -> Result<(), AdminError> {
    let allowed: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM iam_user u JOIN iam_tenant t ON t.tenant_id=u.tenant_id
        WHERE u.tenant_id=?1 AND u.user_id=?2 AND u.user_status='active' AND u.is_superuser=1 AND t.tenant_status='active')",params![tenant,actor],|r|r.get(0))?;
    if allowed {
        Ok(())
    } else {
        Err(AdminError::Forbidden)
    }
}

fn direct_permissions(
    tx: &Transaction<'_>,
    tenant: &str,
    role: &str,
) -> Result<Vec<String>, AdminError> {
    let mut stmt = tx.prepare("SELECT DISTINCT perm_id FROM iam_role_permission WHERE tenant_id=?1 AND role_id=?2 ORDER BY perm_id")?;
    let values = stmt.query_map(params![tenant, role], |r| r.get(0))?.collect::<Result<_, _>>()?;
    Ok(values)
}

pub(crate) fn audit(
    tx: &Transaction<'_>,
    tenant: &str,
    actor: &str,
    action: &str,
    id: &str,
    before: Value,
    after: Value,
) -> Result<(), AdminError> {
    let prev: Option<String> = tx.query_row("SELECT curr_hash FROM audit_log WHERE tenant_id=?1 ORDER BY created_at DESC,log_id DESC LIMIT 1",[tenant],|r|r.get(0)).optional()?;
    let now = chrono::Utc::now().to_rfc3339();
    let log_id = uuid::Uuid::new_v4().to_string();
    let before = before.to_string();
    let after = after.to_string();
    let hash = hex::encode(Sha256::digest(
        json!([prev, log_id, tenant, actor, action, id, before, after, now])
            .to_string()
            .as_bytes(),
    ));
    tx.execute("INSERT INTO audit_log (log_id,tenant_id,user_id,action,resource_type,resource_id,status_code,snapshot_before,snapshot_after,prev_hash,curr_hash,created_at)
        VALUES (?1,?2,?3,?4,?11,?5,200,?6,?7,?8,?9,?10)",params![log_id,tenant,actor,action,id,before,after,prev,hash,now,if action.starts_with("iam.api_key.") { "sys_api_key" } else if action == "iam.role.permissions.replace" { "iam_role" } else { "iam_permission" }])?;
    Ok(())
}

impl IamRepository {
    pub fn administer_permissions(
        &self,
        tenant: &str,
        actor: &str,
        command: PermissionCommand,
    ) -> Result<Value, AdminError> {
        let mut conn = self.conn.lock();
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_admin(&tx, tenant, actor)?;
        let result = match command {
            PermissionCommand::Catalog => {
                let mut stmt = tx.prepare("SELECT perm_id,tenant_id,perm_code,perm_name,status FROM iam_permission WHERE tenant_id IN (?1,'system') ORDER BY perm_code,tenant_id,perm_id")?;
                let rows = stmt.query_map([tenant],|r|Ok(json!({"id":r.get::<_,String>(0)?,"tenant_id":r.get::<_,String>(1)?,"code":r.get::<_,String>(2)?,"name":r.get::<_,String>(3)?,"status":r.get::<_,String>(4)?})))?.collect::<Result<Vec<_>,_>>()?;
                json!({"items":rows})
            },
            PermissionCommand::RegisterMessageSend => {
                let existing: Option<String> = tx.query_row("SELECT perm_id FROM iam_permission WHERE tenant_id=?1 AND perm_code='message:send' ORDER BY perm_id LIMIT 1",[tenant],|r|r.get(0)).optional()?;
                let id = if let Some(id) = existing {
                    id
                } else {
                    let id = uuid::Uuid::new_v4().to_string();
                    let now = chrono::Utc::now().to_rfc3339();
                    let resource: Option<String> = tx.query_row("SELECT resource_id FROM iam_resource WHERE tenant_id=?1 AND resource_code='message' ORDER BY resource_id LIMIT 1",[tenant],|r|r.get(0)).optional()?;
                    let resource = if let Some(resource) = resource {
                        resource
                    } else {
                        let resource = uuid::Uuid::new_v4().to_string();
                        tx.execute("INSERT INTO iam_resource (resource_id,tenant_id,resource_code,resource_name,resource_type,status,created_at,updated_at) VALUES (?1,?2,'message','In-app messages','api','active',?3,?3)",params![resource,tenant,now])?;
                        resource
                    };
                    tx.execute("INSERT INTO iam_permission (perm_id,tenant_id,perm_code,perm_name,resource_id,resource_type,perm_action,status,created_at,updated_at)
                        VALUES (?1,?2,'message:send','Send in-app messages',?4,'api','send','active',?3,?3)",params![id,tenant,now,resource])?;
                    audit(
                        &tx,
                        tenant,
                        actor,
                        "iam.permission.register",
                        &id,
                        Value::Null,
                        json!({"code":"message:send"}),
                    )?;
                    id
                };
                json!({"id":id,"code":"message:send"})
            },
            PermissionCommand::Role(role) => {
                let version: Option<i64> = tx
                    .query_row(
                        "SELECT version FROM iam_role WHERE tenant_id=?1 AND role_id=?2",
                        params![tenant, role],
                        |r| r.get(0),
                    )
                    .optional()?;
                let version = version.ok_or(AdminError::NotFound)?;
                json!({"role_id":role,"version":version,"permission_ids":direct_permissions(&tx,tenant,&role)?})
            },
            PermissionCommand::ReplaceRole { role, version, mut permissions } => {
                if version < 1 || permissions.len() > 200 {
                    return Err(AdminError::Invalid);
                }
                permissions.sort();
                permissions.dedup();
                let current: Option<(i64, String)> = tx
                    .query_row(
                        "SELECT version,status FROM iam_role WHERE tenant_id=?1 AND role_id=?2",
                        params![tenant, role],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()?;
                let (current, status) = current.ok_or(AdminError::NotFound)?;
                if current != version {
                    return Err(AdminError::Conflict);
                }
                if status != "active" {
                    return Err(AdminError::Invalid);
                }
                for id in &permissions {
                    let valid: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM iam_permission WHERE perm_id=?1 AND tenant_id IN (?2,'system') AND status='active')",params![id,tenant],|r|r.get(0))?;
                    if !valid {
                        return Err(AdminError::Invalid);
                    }
                }
                let before = direct_permissions(&tx, tenant, &role)?;
                tx.execute(
                    "DELETE FROM iam_role_permission WHERE tenant_id=?1 AND role_id=?2",
                    params![tenant, role],
                )?;
                let now = chrono::Utc::now().to_rfc3339();
                for id in &permissions {
                    tx.execute("INSERT INTO iam_role_permission (rp_id,tenant_id,role_id,perm_id,created_at,created_by) VALUES (?1,?2,?3,?4,?5,?6)",params![uuid::Uuid::new_v4().to_string(),tenant,role,id,now,actor])?;
                }
                tx.execute("UPDATE iam_role SET version=version+1,updated_at=?3 WHERE tenant_id=?1 AND role_id=?2",params![tenant,role,now])?;
                audit(
                    &tx,
                    tenant,
                    actor,
                    "iam.role.permissions.replace",
                    &role,
                    json!({"version":version,"permission_ids":before}),
                    json!({"version":version+1,"permission_ids":permissions}),
                )?;
                json!({"role_id":role,"version":version+1,"permission_ids":permissions})
            },
        };
        tx.commit()?;
        Ok(result)
    }

    /// Legacy administrative entry points must also reject tenant and object overrides.
    pub fn check_permission_admin_scope(
        &self,
        tenant: &str,
        actor: &str,
        kind: Option<&str>,
        id: Option<&str>,
    ) -> Result<(), AdminError> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        check_admin(&tx, tenant, actor)?;
        if let (Some(kind), Some(id)) = (kind, id) {
            let sql = match kind {
                "user" => "SELECT EXISTS(SELECT 1 FROM iam_user WHERE tenant_id=?1 AND user_id=?2)",
                "role" => "SELECT EXISTS(SELECT 1 FROM iam_role WHERE tenant_id=?1 AND role_id=?2)",
                _ => return Err(AdminError::Invalid),
            };
            if !tx.query_row(sql, params![tenant, id], |r| r.get::<_, bool>(0))? {
                return Err(AdminError::NotFound);
            }
        }
        Ok(())
    }
}
