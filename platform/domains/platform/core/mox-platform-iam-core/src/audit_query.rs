//! Bounded administrative audit metadata, with authorization and reads in one snapshot.
use crate::{
    permission_admin::{check_admin, AdminError},
    IamRepository,
};
use rusqlite::params;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct AuditQuery {
    pub page: u32,
    pub page_size: u32,
    pub action: Option<String>,
    pub actor: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub tenant_id: Option<String>,
}
impl Default for AuditQuery {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
            action: None,
            actor: None,
            since: None,
            until: None,
            tenant_id: None,
        }
    }
}
fn text(value: Option<String>) -> Result<Option<String>, AdminError> {
    let value = value.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    if value.as_ref().is_some_and(|s| s.chars().count() > 128) {
        return Err(AdminError::Invalid);
    }
    Ok(value)
}
fn date(
    value: Option<String>,
) -> Result<Option<chrono::DateTime<chrono::FixedOffset>>, AdminError> {
    text(value)?
        .map(|s| chrono::DateTime::parse_from_rfc3339(&s).map_err(|_| AdminError::Invalid))
        .transpose()
}

impl IamRepository {
    pub fn query_admin_audit(
        &self,
        tenant: &str,
        actor: &str,
        query: AuditQuery,
    ) -> Result<Value, AdminError> {
        let mut conn = self.conn.lock();
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let tx = conn.transaction()?;
        check_admin(&tx, tenant, actor)?;
        if query.tenant_id.as_deref().is_some_and(|t| t != tenant) {
            return Err(AdminError::Forbidden);
        }
        if query.page == 0 || !(1..=100).contains(&query.page_size) {
            return Err(AdminError::Invalid);
        }
        let action = text(query.action)?;
        let actor_filter = text(query.actor)?;
        let since = date(query.since)?;
        let until = date(query.until)?;
        if matches!((&since,&until),(Some(s),Some(u)) if s > u) {
            return Err(AdminError::Invalid);
        }
        let since = since.map(|d| d.to_rfc3339());
        let until = until.map(|d| d.to_rfc3339());
        let predicate = "tenant_id=?1 AND (?2 IS NULL OR action=?2) AND (?3 IS NULL OR user_id=?3) AND (?4 IS NULL OR julianday(created_at)>=julianday(?4)) AND (?5 IS NULL OR julianday(created_at)<=julianday(?5))";
        let total: i64 = tx.query_row(
            &format!("SELECT COUNT(*) FROM audit_log WHERE {predicate}"),
            params![tenant, action, actor_filter, since, until],
            |r| r.get(0),
        )?;
        let offset = i64::from(query.page - 1) * i64::from(query.page_size);
        let items = {
            let mut stmt = tx.prepare(&format!("SELECT log_id,action,action_detail,user_id,user_ip,resource_type,resource_id,status_code,http_method,http_path,latency_ms,created_at FROM audit_log WHERE {predicate} ORDER BY julianday(created_at) DESC,log_id ASC LIMIT ?6 OFFSET ?7"))?;
            let rows = stmt.query_map(params![tenant,action,actor_filter,since,until,query.page_size,offset],|r|Ok(json!({"id":r.get::<_,String>(0)?,"action":r.get::<_,String>(1)?,"actionDetail":r.get::<_,Option<String>>(2)?,"userId":r.get::<_,Option<String>>(3)?,"userIp":r.get::<_,Option<String>>(4)?,"resourceType":r.get::<_,Option<String>>(5)?,"resourceId":r.get::<_,Option<String>>(6)?,"statusCode":r.get::<_,Option<i64>>(7)?,"httpMethod":r.get::<_,Option<String>>(8)?,"httpPath":r.get::<_,Option<String>>(9)?,"latencyMs":r.get::<_,Option<i64>>(10)?,"createdAt":r.get::<_,String>(11)?})))?.collect::<Result<Vec<_>,_>>()?;
            rows
        };
        tx.commit()?;
        Ok(json!({"items":items,"total":total,"page":query.page,"page_size":query.page_size}))
    }
}
