//! Committed state and durable request receipts share a writer transaction.
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

#[derive(Debug)]
pub enum FavoriteError {
    NotFound,
    Conflict,
    Storage(String),
}

#[derive(Debug)]
pub struct FavoriteReceipt {
    pub favorite: bool,
    pub current_favorite: bool,
    pub committed_at: String,
    pub replayed: bool,
}

impl From<rusqlite::Error> for FavoriteError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(error.to_string())
    }
}

/// Bounded batch read from one database snapshot; never use the process cache as authority.
pub fn read(tenant: &str, experts: &[String]) -> Result<Vec<(String, bool)>, FavoriteError> {
    let mut conn = super::experts_db::open_experts_db().map_err(FavoriteError::Storage)?;
    read_on_connection(&mut conn, tenant, experts)
}

pub fn read_on_connection(
    conn: &mut Connection,
    tenant: &str,
    experts: &[String],
) -> Result<Vec<(String, bool)>, FavoriteError> {
    let tx = conn.transaction()?;
    let mut states = Vec::with_capacity(experts.len());
    {
        let mut query = tx.prepare("SELECT EXISTS(SELECT 1 FROM favorites WHERE tenant_id=?1 AND expert_id=?2) WHERE EXISTS(SELECT 1 FROM experts WHERE tenant_id=?1 AND id=?2)")?;
        for expert in experts {
            let favorite = query.query_row(params![tenant, expert], |row| row.get(0)).optional()?;
            match favorite {
                Some(favorite) => states.push((expert.clone(), favorite)),
                None => return Err(FavoriteError::NotFound),
            }
        }
    }
    tx.commit()?;
    Ok(states)
}

pub fn toggle(tenant: &str, expert: &str) -> Result<bool, FavoriteError> {
    let mut conn = super::experts_db::open_experts_db().map_err(FavoriteError::Storage)?;
    toggle_on_connection(&mut conn, tenant, expert)
}

pub fn toggle_with_key(
    tenant: &str,
    actor: &str,
    expert: &str,
    key: Option<&str>,
) -> Result<FavoriteReceipt, FavoriteError> {
    let mut conn = super::experts_db::open_experts_db().map_err(FavoriteError::Storage)?;
    mutate(&mut conn, tenant, actor, expert, key)
}

pub fn toggle_on_connection(
    conn: &mut Connection,
    tenant: &str,
    expert: &str,
) -> Result<bool, FavoriteError> {
    mutate(conn, tenant, "", expert, None).map(|receipt| receipt.favorite)
}

pub fn toggle_idempotent_on_connection(
    conn: &mut Connection,
    tenant: &str,
    actor: &str,
    expert: &str,
    key: &str,
) -> Result<FavoriteReceipt, FavoriteError> {
    mutate(conn, tenant, actor, expert, Some(key))
}

fn mutate(
    conn: &mut Connection,
    tenant: &str,
    actor: &str,
    expert: &str,
    key: Option<&str>,
) -> Result<FavoriteReceipt, FavoriteError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if let Some(key) = key {
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS favorite_request_receipts (
            tenant_id TEXT NOT NULL, actor_id TEXT NOT NULL, request_key TEXT NOT NULL,
            expert_id TEXT NOT NULL, favorite INTEGER NOT NULL, committed_at TEXT NOT NULL,
            PRIMARY KEY(tenant_id,actor_id,request_key))",
        )?;
        let previous: Option<(String,bool,String)> = tx.query_row(
            "SELECT expert_id,favorite,committed_at FROM favorite_request_receipts WHERE tenant_id=?1 AND actor_id=?2 AND request_key=?3",
            params![tenant,actor,key], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
        ).optional()?;
        if let Some((previous_expert, favorite, committed_at)) = previous {
            if previous_expert != expert {
                return Err(FavoriteError::Conflict);
            }
            let current_favorite = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM favorites WHERE tenant_id=?1 AND expert_id=?2)",
                params![tenant, expert],
                |row| row.get(0),
            )?;
            tx.commit()?;
            return Ok(FavoriteReceipt {
                favorite,
                current_favorite,
                committed_at,
                replayed: true,
            });
        }
    }
    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM experts WHERE tenant_id=?1 AND id=?2)",
        params![tenant, expert],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(FavoriteError::NotFound);
    }
    let present: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM favorites WHERE tenant_id=?1 AND expert_id=?2)",
        params![tenant, expert],
        |row| row.get(0),
    )?;
    let committed_at = super::experts_common::now_iso();
    if present {
        tx.execute(
            "DELETE FROM favorites WHERE tenant_id=?1 AND expert_id=?2",
            params![tenant, expert],
        )?;
    } else {
        tx.execute(
            "INSERT INTO favorites(tenant_id,expert_id,created_at) VALUES(?1,?2,?3)",
            params![tenant, expert, &committed_at],
        )?;
    }
    if let Some(key) = key {
        tx.execute("INSERT INTO favorite_request_receipts(tenant_id,actor_id,request_key,expert_id,favorite,committed_at) VALUES(?1,?2,?3,?4,?5,?6)",
            params![tenant,actor,key,expert,!present,&committed_at])?;
    }
    tx.commit()?;
    Ok(FavoriteReceipt {
        favorite: !present,
        current_favorite: !present,
        committed_at,
        replayed: false,
    })
}
