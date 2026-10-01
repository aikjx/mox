//! 文件元数据权威仓储：授权条件在所有查询和更新中执行。
use super::FileMetadata;
use mox_platform_api::UserInfo;
use rusqlite::{params, Connection, OptionalExtension};
use std::{path::Path, sync::Mutex};

pub(super) struct FileAccess {
    pub tenant: String,
    pub user: String,
    pub admin: bool,
}

impl From<&UserInfo> for FileAccess {
    fn from(user: &UserInfo) -> Self {
        Self {
            tenant: user.tenant_id.clone(),
            user: user.id.clone(),
            admin: user
                .roles
                .iter()
                .any(|role| matches!(role.as_str(), "super_admin" | "tenant_admin")),
        }
    }
}

pub(super) struct FileRepository(Mutex<Connection>);
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

impl FileRepository {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS file_resource (
                id TEXT PRIMARY KEY, tenant TEXT NOT NULL, owner TEXT NOT NULL, body TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS file_resource_scope ON file_resource(tenant, owner);",
        )?;
        Ok(Self(Mutex::new(conn)))
    }

    pub fn insert_many(&self, files: &[FileMetadata]) -> Result<()> {
        let mut conn = self.0.lock().map_err(|_| "file repository lock poisoned")?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        for file in files {
            let tenant =
                file.tenant_id.as_deref().filter(|s| !s.is_empty()).ok_or("missing tenant")?;
            tx.execute(
                "INSERT INTO file_resource(id,tenant,owner,body) VALUES (?1,?2,?3,?4)",
                params![file.file_id, tenant, file.uploaded_by, serde_json::to_string(file)?],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list(&self, access: &FileAccess) -> Result<Vec<FileMetadata>> {
        let conn = self.0.lock().map_err(|_| "file repository lock poisoned")?;
        let mut stmt =
            conn.prepare("SELECT body FROM file_resource WHERE tenant=?1 AND (owner=?2 OR ?3)")?;
        let rows = stmt.query_map(params![access.tenant, access.user, access.admin], |row| {
            row.get::<_, String>(0)
        })?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }

    pub fn get(&self, access: &FileAccess, id: &str) -> Result<Option<FileMetadata>> {
        let conn = self.0.lock().map_err(|_| "file repository lock poisoned")?;
        Self::read(&conn, access, id)
    }

    fn read(conn: &Connection, access: &FileAccess, id: &str) -> Result<Option<FileMetadata>> {
        let body: Option<String> = conn
            .query_row(
                "SELECT body FROM file_resource WHERE id=?1 AND tenant=?2 AND (owner=?3 OR ?4)",
                params![id, access.tenant, access.user, access.admin],
                |row| row.get(0),
            )
            .optional()?;
        body.map(|body| serde_json::from_str(&body).map_err(Into::into)).transpose()
    }

    pub fn change_status(&self, access: &FileAccess, id: &str, target: &str) -> Result<bool> {
        self.update(access, id, |file| {
            file.status = target.into();
            true
        })
    }

    pub fn record_download(&self, access: &FileAccess, id: &str) -> Result<bool> {
        self.update(access, id, |file| {
            if file.status != "normal" {
                return false;
            }
            file.download_count += 1;
            file.last_accessed_at = Some(chrono::Utc::now().to_rfc3339());
            true
        })
    }

    fn update(
        &self,
        access: &FileAccess,
        id: &str,
        mutate: impl FnOnce(&mut FileMetadata) -> bool,
    ) -> Result<bool> {
        let mut conn = self.0.lock().map_err(|_| "file repository lock poisoned")?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let Some(mut file) = Self::read(&tx, access, id)? else { return Ok(false) };
        if !mutate(&mut file) {
            return Ok(false);
        }
        tx.execute(
            "UPDATE file_resource SET body=?1 WHERE id=?2",
            params![serde_json::to_string(&file)?, id],
        )?;
        tx.commit()?;
        Ok(true)
    }
}
