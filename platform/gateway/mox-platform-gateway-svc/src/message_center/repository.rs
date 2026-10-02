//! Transactional scoped inbox storage. SQLite supports same-host processes, not a multi-host cluster.
use super::{Message, MessageSendRecord, MessageStats, MessageStatus};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::path::PathBuf;

#[derive(Debug)]
pub enum StoreError {
    Conflict,
    Forbidden,
    Unavailable(String),
}
impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Unavailable(error.to_string())
    }
}
impl From<serde_json::Error> for StoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Unavailable(error.to_string())
    }
}

#[derive(Clone)]
pub struct InboxRepository {
    path: PathBuf,
}

impl InboxRepository {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn open(&self) -> Result<Connection, StoreError> {
        if self.path.as_os_str().is_empty() || self.path == std::path::Path::new(":memory:") {
            return Err(StoreError::Unavailable("A persistent database path is required".into()));
        }
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| StoreError::Unavailable(e.to_string()))?;
        }
        let mut conn = Connection::open(&self.path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        // Concurrent first-open can return SQLITE_BUSY without invoking busy_timeout.
        // Retry only initialization locks; never replay a business transaction here.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match conn.query_row("PRAGMA journal_mode=WAL", [], |row| row.get::<_, String>(0)) {
                Ok(mode) if mode.eq_ignore_ascii_case("wal") => break,
                Ok(mode) => {
                    return Err(StoreError::Unavailable(format!("WAL unavailable: {mode}")))
                },
                Err(error)
                    if matches!(
                        error.sqlite_error_code(),
                        Some(
                            rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                        )
                    ) && std::time::Instant::now() < deadline =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(25));
                },
                Err(error) => {
                    return Err(StoreError::Unavailable(format!("Initialize WAL: {error}")))
                },
            }
        }
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.execute_batch("CREATE TABLE IF NOT EXISTS inbox_messages (
            tenant TEXT NOT NULL, receiver TEXT NOT NULL, id TEXT NOT NULL,
            created_at TEXT NOT NULL, message_type TEXT NOT NULL, status TEXT NOT NULL,
            payload TEXT NOT NULL, idempotency_key TEXT, fingerprint TEXT NOT NULL,
            PRIMARY KEY(tenant, receiver, id), UNIQUE(tenant, receiver, idempotency_key));
            CREATE INDEX IF NOT EXISTS inbox_scope_time ON inbox_messages(tenant, receiver, created_at DESC, id DESC);
            CREATE TABLE IF NOT EXISTS inbox_receipts (
            tenant TEXT NOT NULL, receiver TEXT NOT NULL, message_id TEXT NOT NULL, payload TEXT NOT NULL,
            PRIMARY KEY(tenant, receiver, message_id),
            FOREIGN KEY(tenant, receiver, message_id) REFERENCES inbox_messages(tenant, receiver, id));
            CREATE TABLE IF NOT EXISTS inbox_dispatch_keys (
                tenant TEXT NOT NULL, sender TEXT NOT NULL, key TEXT NOT NULL,
                message_id TEXT NOT NULL, fingerprint TEXT NOT NULL,
                PRIMARY KEY(tenant,sender,key));
            CREATE TABLE IF NOT EXISTS inbox_schema_migrations (version INTEGER PRIMARY KEY);")?;
        let migrated: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM inbox_schema_migrations WHERE version=2)",
            [],
            |r| r.get(0),
        )?;
        if !migrated {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch(
                "INSERT OR IGNORE INTO inbox_dispatch_keys
                SELECT tenant,receiver,idempotency_key,id,fingerprint FROM inbox_messages
                WHERE idempotency_key IS NOT NULL AND json_extract(payload,'$.sender_id')=receiver;
                INSERT OR IGNORE INTO inbox_schema_migrations VALUES (2);",
            )?;
            tx.commit()?;
        }
        Ok(conn)
    }

    /// Message, receipt and deduplication identity commit together.
    pub fn send(
        &self,
        message: &Message,
        receipt: &MessageSendRecord,
        key: Option<&str>,
        fingerprint: &str,
    ) -> Result<String, StoreError> {
        self.send_many(message, std::slice::from_ref(receipt), key, fingerprint)
    }

    pub fn send_many(
        &self,
        message: &Message,
        receipts: &[MessageSendRecord],
        key: Option<&str>,
        fingerprint: &str,
    ) -> Result<String, StoreError> {
        if receipts.is_empty() || receipts.len() > 100 {
            return Err(StoreError::Unavailable("Invalid delivery batch".into()));
        }
        let mut conn = self.open()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(key) = key {
            let existing: Option<(String, String)> = tx.query_row(
                "SELECT message_id, fingerprint FROM inbox_dispatch_keys WHERE tenant=?1 AND sender=?2 AND key=?3",
                params![message.tenant_id, message.sender_id, key], |r| Ok((r.get(0)?, r.get(1)?))).optional()?;
            if let Some((id, prior)) = existing {
                return if prior == fingerprint { Ok(id) } else { Err(StoreError::Conflict) };
            }
        }
        for receipt in receipts {
            let mut personal = message.clone();
            personal.receiver_ids = vec![receipt.receiver_id.clone()];
            tx.execute(
                "INSERT INTO inbox_messages VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    message.tenant_id,
                    receipt.receiver_id,
                    message.message_id,
                    message.created_at,
                    enum_name(&message.message_type)?,
                    enum_name(&message.status)?,
                    serde_json::to_string(&personal)?,
                    Option::<&str>::None,
                    fingerprint
                ],
            )?;
            tx.execute(
                "INSERT INTO inbox_receipts VALUES (?1,?2,?3,?4)",
                params![
                    message.tenant_id,
                    receipt.receiver_id,
                    message.message_id,
                    serde_json::to_string(receipt)?
                ],
            )?;
        }
        if let Some(key) = key {
            tx.execute(
                "INSERT INTO inbox_dispatch_keys VALUES (?1,?2,?3,?4,?5)",
                params![message.tenant_id, message.sender_id, key, message.message_id, fingerprint],
            )?;
        }
        tx.commit()?;
        Ok(message.message_id.clone())
    }

    pub fn get(
        &self,
        tenant: &str,
        receiver: &str,
        id: &str,
    ) -> Result<Option<Message>, StoreError> {
        let conn = self.open()?;
        let payload: Option<String> = conn
            .query_row(
                "SELECT payload FROM inbox_messages WHERE tenant=?1 AND receiver=?2 AND id=?3",
                params![tenant, receiver, id],
                |r| r.get(0),
            )
            .optional()?;
        payload.map(|p| serde_json::from_str(&p).map_err(Into::into)).transpose()
    }

    pub fn list(
        &self,
        tenant: &str,
        receiver: &str,
        limit: i64,
        offset: i64,
        kind: Option<&str>,
        status: Option<&str>,
    ) -> Result<(Vec<Message>, i64), StoreError> {
        let mut conn = self.open()?;
        let tx = conn.transaction()?;
        let total = tx.query_row("SELECT COUNT(*) FROM inbox_messages WHERE tenant=?1 AND receiver=?2 AND (?3 IS NULL OR message_type=?3) AND (?4 IS NULL OR status=?4)",
            params![tenant, receiver, kind, status], |r| r.get(0))?;
        let mut query = tx.prepare("SELECT payload FROM inbox_messages WHERE tenant=?1 AND receiver=?2 AND (?3 IS NULL OR message_type=?3) AND (?4 IS NULL OR status=?4) ORDER BY created_at DESC, id DESC LIMIT ?5 OFFSET ?6")?;
        let rows = query
            .query_map(params![tenant, receiver, kind, status, limit, offset], |r| {
                r.get::<_, String>(0)
            })?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(serde_json::from_str(&row?)?);
        }
        Ok((messages, total))
    }

    pub fn receipt(
        &self,
        tenant: &str,
        receiver: &str,
        id: &str,
    ) -> Result<Option<MessageSendRecord>, StoreError> {
        let conn = self.open()?;
        let payload: Option<String> = conn.query_row("SELECT payload FROM inbox_receipts WHERE tenant=?1 AND receiver=?2 AND message_id=?3",
            params![tenant, receiver, id], |r| r.get(0)).optional()?;
        payload.map(|p| serde_json::from_str(&p).map_err(Into::into)).transpose()
    }

    pub fn mark_read(&self, tenant: &str, receiver: &str, id: &str) -> Result<bool, StoreError> {
        let mut conn = self.open()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let payload: Option<String> = tx
            .query_row(
                "SELECT payload FROM inbox_messages WHERE tenant=?1 AND receiver=?2 AND id=?3",
                params![tenant, receiver, id],
                |r| r.get(0),
            )
            .optional()?;
        let Some(payload) = payload else {
            return Ok(false);
        };
        let mut message: Message = serde_json::from_str(&payload)?;
        if matches!(message.status, MessageStatus::Read) {
            return Ok(true);
        }
        let payload: String = tx.query_row(
            "SELECT payload FROM inbox_receipts WHERE tenant=?1 AND receiver=?2 AND message_id=?3",
            params![tenant, receiver, id],
            |r| r.get(0),
        )?;
        let mut receipt: MessageSendRecord = serde_json::from_str(&payload)?;
        let now = chrono::Utc::now().to_rfc3339();
        message.status = MessageStatus::Read;
        message.updated_at = now.clone();
        receipt.status = MessageStatus::Read;
        receipt.read_at = Some(now);
        tx.execute("UPDATE inbox_messages SET status='read', payload=?4 WHERE tenant=?1 AND receiver=?2 AND id=?3",
            params![tenant, receiver, id, serde_json::to_string(&message)?])?;
        tx.execute("UPDATE inbox_receipts SET payload=?4 WHERE tenant=?1 AND receiver=?2 AND message_id=?3",
            params![tenant, receiver, id, serde_json::to_string(&receipt)?])?;
        tx.commit()?;
        Ok(true)
    }

    pub fn mark_all_read(&self, tenant: &str, receiver: &str) -> Result<usize, StoreError> {
        let mut conn = self.open()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now = chrono::Utc::now().to_rfc3339();
        let receipts = tx.execute(
            "UPDATE inbox_receipts SET payload=json_set(payload,'$.status','read','$.read_at',?3)
            WHERE tenant=?1 AND receiver=?2 AND message_id IN
            (SELECT id FROM inbox_messages WHERE tenant=?1 AND receiver=?2 AND status='sent')",
            params![tenant, receiver, now],
        )?;
        let messages = tx.execute("UPDATE inbox_messages SET status='read', payload=json_set(payload,'$.status','read','$.updated_at',?3)
            WHERE tenant=?1 AND receiver=?2 AND status='sent'", params![tenant,receiver,now])?;
        if messages != receipts {
            return Err(StoreError::Unavailable("Inbox receipt integrity failure".into()));
        }
        tx.commit()?;
        Ok(messages)
    }

    pub fn unread_counts(
        &self,
        tenant: &str,
        receiver: &str,
    ) -> Result<std::collections::BTreeMap<String, i64>, StoreError> {
        let conn = self.open()?;
        let mut query = conn.prepare("SELECT CASE message_type WHEN 'task' THEN 'task' WHEN 'alert' THEN 'alert' ELSE 'message' END AS category, COUNT(*)
            FROM inbox_messages WHERE tenant=?1 AND receiver=?2 AND status='sent' GROUP BY category")?;
        let rows = query.query_map(params![tenant, receiver], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut result = std::collections::BTreeMap::from([
            ("message".into(), 0),
            ("task".into(), 0),
            ("alert".into(), 0),
        ]);
        for row in rows {
            let (kind, count) = row?;
            result.insert(kind, count);
        }
        Ok(result)
    }

    pub fn stats(&self, tenant: &str, receiver: &str) -> Result<MessageStats, StoreError> {
        let conn = self.open()?;
        let mut query = conn.prepare("SELECT status, COUNT(*) FROM inbox_messages WHERE tenant=?1 AND receiver=?2 GROUP BY status")?;
        let rows = query.query_map(params![tenant, receiver], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        let mut stats = MessageStats::default();
        for row in rows {
            let (status, count) = row?;
            stats.total += count;
            match status.as_str() {
                "sent" => stats.sent = count,
                "read" => stats.read = count,
                "failed" => stats.failed = count,
                "pending" => stats.pending = count,
                _ => {},
            }
        }
        Ok(stats)
    }
}

fn enum_name(value: &impl serde::Serialize) -> Result<String, StoreError> {
    serde_json::from_value(serde_json::to_value(value)?).map_err(Into::into)
}
