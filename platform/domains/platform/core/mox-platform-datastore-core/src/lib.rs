//! MOX Platform Datastore Core
//!
//! Multi-backend data store abstraction: SQLite (default), PostgreSQL, MySQL.
//! Unified connection, migrations, transactions, KV store, and Repository trait.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

pub use sea_query;

#[derive(Debug, Error)]
pub enum DatastoreError {
    #[error("connection error: {0}")]
    Connection(String),
    #[error("query error: {0}")]
    Query(String),
    #[error("migration error: {0}")]
    Migration(String),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("backend not supported: {0}")]
    UnsupportedBackend(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseBackend {
    Sqlite,
    Postgres,
    Mysql,
}

impl Default for DatabaseBackend {
    fn default() -> Self { DatabaseBackend::Sqlite }
}

impl DatabaseBackend {
    pub fn from_env() -> Self {
        match std::env::var("MOX_BACKEND").as_deref() {
            Ok("postgres") | Ok("pg") => DatabaseBackend::Postgres,
            Ok("mysql") => DatabaseBackend::Mysql,
            _ => DatabaseBackend::Sqlite,
        }
    }
    pub fn scheme(&self) -> &'static str {
        match self { DatabaseBackend::Sqlite => "sqlite", DatabaseBackend::Postgres => "postgres", DatabaseBackend::Mysql => "mysql" }
    }
    pub fn supports_returning(&self) -> bool { matches!(self, DatabaseBackend::Postgres | DatabaseBackend::Sqlite) }
    pub fn upsert_keyword(&self) -> &'static str {
        match self { DatabaseBackend::Sqlite => "INSERT OR REPLACE", DatabaseBackend::Postgres => "ON CONFLICT DO UPDATE", DatabaseBackend::Mysql => "ON DUPLICATE KEY UPDATE" }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatastoreConfig {
    pub backend: DatabaseBackend,
    pub url: String,
    pub max_connections: u32,
    pub connect_timeout_secs: u64,
    pub strict_persist: bool,
    pub auto_migrate: bool,
}

impl Default for DatastoreConfig {
    fn default() -> Self {
        Self { backend: DatabaseBackend::default(), url: "sqlite:mox.db?mode=rwc".into(), max_connections: 10, connect_timeout_secs: 5, strict_persist: false, auto_migrate: true }
    }
}

impl DatastoreConfig {
    pub fn from_env() -> Result<Self, DatastoreError> {
        let backend = DatabaseBackend::from_env();
        let default_url = match backend {
            DatabaseBackend::Sqlite => "sqlite:mox.db?mode=rwc".to_string(),
            DatabaseBackend::Postgres => "postgres://localhost:5432/mox".to_string(),
            DatabaseBackend::Mysql => "mysql://localhost:3306/mox".to_string(),
        };
        Ok(Self {
            backend,
            url: std::env::var("MOX_DB_URL").unwrap_or(default_url),
            max_connections: std::env::var("MOX_DB_MAX_CONN").ok().and_then(|v| v.parse().ok()).unwrap_or(10),
            connect_timeout_secs: 5,
            strict_persist: std::env::var("MOX_STRICT_PERSIST").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false),
            auto_migrate: std::env::var("MOX_AUTO_MIGRATE").map(|v| v != "0" && !v.eq_ignore_ascii_case("false")).unwrap_or(true),
        })
    }
    pub fn validate(&self) -> Result<(), DatastoreError> {
        if self.url.is_empty() { return Err(DatastoreError::Config("database URL is empty".into())); }
        if self.max_connections == 0 { return Err(DatastoreError::Config("max_connections must be > 0".into())); }
        Ok(())
    }
}

#[derive(Clone)]
pub struct DatastoreConnection {
    pub config: DatastoreConfig,
    inner: Arc<DatastoreInner>,
}

enum DatastoreInner {
    Sqlite(rusqlite::Connection),
    Postgres,
    Mysql,
}

impl DatastoreConnection {
    pub fn new(config: DatastoreConfig) -> Result<Self, DatastoreError> {
        config.validate()?;
        let inner = match config.backend {
            DatabaseBackend::Sqlite => {
                let path = config.url.strip_prefix("sqlite:").unwrap_or(&config.url).split('?').next().unwrap_or("mox.db");
                let conn = if path == ":memory:" { rusqlite::Connection::open_in_memory() } else { rusqlite::Connection::open(path) }
                    .map_err(|e| DatastoreError::Connection(e.to_string()))?;
                conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA synchronous=NORMAL;")
                    .map_err(|e| DatastoreError::Connection(e.to_string()))?;
                DatastoreInner::Sqlite(conn)
            }
            DatabaseBackend::Postgres => { if config.strict_persist { return Err(DatastoreError::UnsupportedBackend("PostgreSQL requires sqlx feature (planned)".into())); } DatastoreInner::Postgres }
            DatabaseBackend::Mysql => { if config.strict_persist { return Err(DatastoreError::UnsupportedBackend("MySQL requires sqlx feature (planned)".into())); } DatastoreInner::Mysql }
        };
        Ok(Self { config, inner: Arc::new(inner) })
    }
    pub fn from_env() -> Result<Self, DatastoreError> { Self::new(DatastoreConfig::from_env()?) }
    pub fn backend(&self) -> DatabaseBackend { self.config.backend }

    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<usize, DatastoreError> {
        match &*self.inner {
            DatastoreInner::Sqlite(conn) => conn.execute(sql, params).map_err(|e| DatastoreError::Query(e.to_string())),
            _ => Err(DatastoreError::UnsupportedBackend("execute requires SQLite".into())),
        }
    }

    pub fn query<T, F>(&self, sql: &str, params: &[&dyn rusqlite::ToSql], mapper: F) -> Result<Vec<T>, DatastoreError>
    where F: Fn(&rusqlite::Row<'_>) -> rusqlite::Result<T> {
        match &*self.inner {
            DatastoreInner::Sqlite(conn) => {
                let mut stmt = conn.prepare(sql).map_err(|e| DatastoreError::Query(e.to_string()))?;
                let rows = stmt.query_map(params, mapper).map_err(|e| DatastoreError::Query(e.to_string()))?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|e| DatastoreError::Query(e.to_string()))
            }
            _ => Err(DatastoreError::UnsupportedBackend("query requires SQLite".into())),
        }
    }

    pub fn query_one<T, F>(&self, sql: &str, params: &[&dyn rusqlite::ToSql], mapper: F) -> Result<Option<T>, DatastoreError>
    where F: Fn(&rusqlite::Row<'_>) -> rusqlite::Result<T> {
        Ok(self.query(sql, params, mapper)?.into_iter().next())
    }

    pub fn migrate(&self, name: &str, sql: &str) -> Result<(), DatastoreError> {
        self.execute("CREATE TABLE IF NOT EXISTS _mox_migrations (name TEXT PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT (datetime('now')))", &[])?;
        let already: Option<String> = self.query_one("SELECT name FROM _mox_migrations WHERE name = ?1", &[&name], |r| r.get(0))?;
        if already.is_none() {
            self.execute(sql, &[])?;
            self.execute("INSERT INTO _mox_migrations (name) VALUES (?1)", &[&name])?;
            tracing::info!(migration = name, "migration applied");
        }
        Ok(())
    }

    pub fn transaction<F, T>(&self, f: F) -> Result<T, DatastoreError>
    where F: FnOnce(&TransactionCtx<'_>) -> Result<T, DatastoreError> {
        match &*self.inner {
            DatastoreInner::Sqlite(conn) => {
                let mut conn = conn.lock();
                let tx = conn.transaction().map_err(|e| DatastoreError::Query(e.to_string()))?;
                let ctx = TransactionCtx { tx: &tx };
                let result = f(&ctx)?;
                tx.commit().map_err(|e| DatastoreError::Query(e.to_string()))?;
                Ok(result)
            }
            _ => Err(DatastoreError::UnsupportedBackend("transaction requires SQLite".into())),
        }
    }
}

pub struct TransactionCtx<'a> { tx: &'a rusqlite::Transaction<'a> }
impl<'a> TransactionCtx<'a> {
    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<usize, DatastoreError> {
        self.tx.execute(sql, params).map_err(|e| DatastoreError::Query(e.to_string()))
    }
    pub fn query<T, F>(&self, sql: &str, params: &[&dyn rusqlite::ToSql], mapper: F) -> Result<Vec<T>, DatastoreError>
    where F: Fn(&rusqlite::Row<'_>) -> rusqlite::Result<T> {
        let mut stmt = self.tx.prepare(sql).map_err(|e| DatastoreError::Query(e.to_string()))?;
        let rows = stmt.query_map(params, mapper).map_err(|e| DatastoreError::Query(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| DatastoreError::Query(e.to_string()))
    }
}

pub trait Repository {
    type Entity;
    type Id;
    fn find_by_id(&self, id: &Self::Id) -> Result<Option<Self::Entity>, DatastoreError>;
    fn find_all(&self) -> Result<Vec<Self::Entity>, DatastoreError>;
    fn save(&self, entity: &Self::Entity) -> Result<(), DatastoreError>;
    fn delete(&self, id: &Self::Id) -> Result<bool, DatastoreError>;
    fn count(&self) -> Result<u64, DatastoreError>;
}

#[derive(Clone)]
pub struct KvStore { conn: DatastoreConnection, table: String }

impl KvStore {
    pub fn new(conn: DatastoreConnection, table: &str) -> Result<Self, DatastoreError> {
        let t = if table.is_empty() { "kv_store" } else { table };
        conn.execute(&format!("CREATE TABLE IF NOT EXISTS {} (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL DEFAULT (datetime('now')))", t), &[])?;
        Ok(Self { conn, table: t.into() })
    }
    pub fn get(&self, key: &str) -> Result<Option<String>, DatastoreError> {
        self.conn.query_one(&format!("SELECT value FROM {} WHERE key = ?1", self.table), &[&key], |r| r.get(0))
    }
    pub fn set(&self, key: &str, value: &str) -> Result<(), DatastoreError> {
        self.conn.execute(&format!("INSERT OR REPLACE INTO {} (key, value) VALUES (?1, ?2)", self.table), &[&key, &value])?;
        Ok(())
    }
    pub fn delete(&self, key: &str) -> Result<bool, DatastoreError> {
        Ok(self.conn.execute(&format!("DELETE FROM {} WHERE key = ?1", self.table), &[&key])? > 0)
    }
    pub fn keys(&self, prefix: &str) -> Result<Vec<String>, DatastoreError> {
        let pattern = format!("{}%", prefix);
        self.conn.query(&format!("SELECT key FROM {} WHERE key LIKE ?1 ORDER BY key", self.table), &[&pattern.as_str()], |r| r.get(0))
    }
    pub fn get_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>, DatastoreError> {
        match self.get(key)? {
            Some(v) => serde_json::from_str(&v).map(Some).map_err(|e| DatastoreError::Query(format!("JSON parse: {}", e))),
            None => Ok(None),
        }
    }
    pub fn set_json<T: Serialize>(&self, key: &str, value: &T) -> Result<(), DatastoreError> {
        let json = serde_json::to_string(value).map_err(|e| DatastoreError::Query(format!("JSON serialize: {}", e)))?;
        self.set(key, &json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_conn() -> DatastoreConnection {
        DatastoreConnection::new(DatastoreConfig { backend: DatabaseBackend::Sqlite, url: "sqlite::memory:".into(), ..Default::default() }).unwrap()
    }
    #[test]
    fn kv_set_get() {
        let kv = KvStore::new(test_conn(), "t").unwrap();
        kv.set("k", "v").unwrap();
        assert_eq!(kv.get("k").unwrap(), Some("v".into()));
    }
    #[test]
    fn migration_idempotent() {
        let c = test_conn();
        c.migrate("m1", "CREATE TABLE t1(id INTEGER PRIMARY KEY)").unwrap();
        c.migrate("m1", "CREATE TABLE t1(id INTEGER PRIMARY KEY)").unwrap();
    }
    #[test]
    fn tx_commit() {
        let c = test_conn();
        c.execute("CREATE TABLE tx(id INTEGER PRIMARY KEY, v TEXT)", &[]).unwrap();
        c.transaction(|ctx| { ctx.execute("INSERT INTO tx(v) VALUES(?1)", &[&"a"])?; Ok(()) }).unwrap();
        let n: i64 = c.query_one("SELECT COUNT(*) FROM tx", &[], |r| r.get(0)).unwrap().unwrap();
        assert_eq!(n, 1);
    }
}
