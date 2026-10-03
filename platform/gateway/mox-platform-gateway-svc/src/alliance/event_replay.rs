//! Tenant-scoped, bounded durable SSE replay. SQLite append order is authoritative.
use rusqlite::{params, OptionalExtension};

use super::{experts_db::open_experts_db, experts_events::AllianceEvent};

#[derive(Debug)]
pub(super) enum ReplayError {
    CursorGone,
    RefreshRequired,
    Storage(String),
}

impl From<rusqlite::Error> for ReplayError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(error.to_string())
    }
}

pub(super) fn load_after(tenant: &str, cursor: &str) -> Result<Vec<AllianceEvent>, ReplayError> {
    let conn = open_experts_db().map_err(ReplayError::Storage)?;
    let tx = conn.unchecked_transaction()?;
    let rowid: i64 = tx
        .query_row(
            "SELECT rowid FROM alliance_event_log WHERE tenant_id=?1 AND event_id=?2",
            params![tenant, cursor],
            |row| row.get(0),
        )
        .optional()?
        .ok_or(ReplayError::CursorGone)?;
    let mut stmt = tx.prepare(
        "SELECT event_id,event_type,
         CASE WHEN length(CAST(payload AS BLOB))<=1048576 THEN payload ELSE NULL END
         FROM alliance_event_log WHERE tenant_id=?1 AND rowid>?2 ORDER BY rowid LIMIT 201",
    )?;
    let mut rows = stmt.query(params![tenant, rowid])?;
    let mut events = Vec::new();
    let mut bytes = 0;
    while let Some(row) = rows.next()? {
        if events.len() == 200 {
            return Err(ReplayError::RefreshRequired);
        }
        let payload: Option<String> = row.get(2)?;
        let payload = payload.ok_or(ReplayError::RefreshRequired)?;
        bytes += payload.len();
        if bytes > 2 * 1024 * 1024 {
            return Err(ReplayError::RefreshRequired);
        }
        let event: AllianceEvent = serde_json::from_str(&payload)
            .map_err(|error| ReplayError::Storage(error.to_string()))?;
        let id: String = row.get(0)?;
        let kind: String = row.get(1)?;
        if event.tenant != tenant
            || event.id != id
            || event.kind.type_name() != kind
            || event.id.is_empty()
            || event.id.len() > 256
            || !event.id.bytes().all(|byte| (33..=126).contains(&byte))
        {
            return Err(ReplayError::Storage(
                "event envelope does not match its tenant-scoped row".into(),
            ));
        }
        events.push(event);
    }
    Ok(events)
}
