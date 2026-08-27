// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Mox-System 独占的 rusqlite PersistenceProvider 实现（L5-Infra 层）
//!
//! 本文件是仓库中**唯一**被允许直接 `use rusqlite` 的 provider 实现；
//! 其他 crate（ai-agent / primiflow-core / hermes…）只通过抽象 trait 调用。

use std::sync::{Arc, Mutex};

use crate::persistence_provider::{PersistenceProvider, SqlRow, SqlValue};

/// Rusqlite Connection 包装 + Send+Sync 互斥（与旧 rusqlite 用法完全一致的共享语义）
pub struct SqlitePersistence {
    inner: Arc<Mutex<rusqlite::Connection>>,
}

impl SqlitePersistence {
    pub fn file(path: &str) -> anyhow::Result<Self> {
        let conn = rusqlite::Connection::open(path)?;
        Ok(Self::wrap(conn))
    }

    pub fn memory() -> anyhow::Result<Self> {
        let conn = rusqlite::Connection::open_in_memory()?;
        Ok(Self::wrap(conn))
    }

    fn wrap(conn: rusqlite::Connection) -> Self {
        Self {
            inner: Arc::new(Mutex::new(conn)),
        }
    }

    /// 允许同一进程内共享同一底层 SQLite 连接（保留 现有 ai-agent/db:Arc<Mutex<Connection>> 使用习惯）
    pub fn as_shared(&self) -> Arc<Mutex<rusqlite::Connection>> {
        self.inner.clone()
    }
}

fn to_rusqlite_params<'a>(
    params: &'a [SqlValue],
    scratch: &'a mut Vec<rusqlite::types::Value>,
) -> Vec<&'a dyn rusqlite::ToSql> {
    scratch.clear();
    scratch.extend(params.iter().cloned().map(rusqlite_value_from));
    scratch.iter().map(|v| v as &dyn rusqlite::ToSql).collect()
}

fn rusqlite_value_from(v: SqlValue) -> rusqlite::types::Value {
    use rusqlite::types::Value as RV;
    match v {
        SqlValue::Null => RV::Null,
        SqlValue::Int(x) => RV::Integer(x),
        SqlValue::Real(x) => RV::Real(x),
        SqlValue::Text(x) => RV::Text(x),
        SqlValue::Blob(x) => RV::Blob(x),
        SqlValue::Bool(x) => RV::Integer(if x { 1 } else { 0 }),
    }
}

fn sql_value_from_rusqlite(v: rusqlite::types::Value) -> SqlValue {
    use rusqlite::types::Value as RV;
    match v {
        RV::Null => SqlValue::Null,
        RV::Integer(i) => SqlValue::Int(i),
        RV::Real(f) => SqlValue::Real(f),
        RV::Text(s) => SqlValue::Text(s),
        RV::Blob(b) => SqlValue::Blob(b),
    }
}

impl PersistenceProvider for SqlitePersistence {
    fn exec(&self, sql: &str, params: &[SqlValue]) -> anyhow::Result<usize> {
        let mut scratch = Vec::with_capacity(params.len());
        let refs = to_rusqlite_params(params, &mut scratch);
        let conn = self
            .inner
            .lock()
            .map_err(|e| anyhow::anyhow!("sqlite mutex: {e}"))?;
        let n = conn.execute(sql, refs.as_slice())?;
        Ok(n)
    }

    fn exec_batch(&self, sql: &str) -> anyhow::Result<()> {
        let conn = self
            .inner
            .lock()
            .map_err(|e| anyhow::anyhow!("sqlite mutex: {e}"))?;
        conn.execute_batch(sql)?;
        Ok(())
    }

    fn query(&self, sql: &str, params: &[SqlValue]) -> anyhow::Result<Vec<SqlRow>> {
        let mut scratch = Vec::with_capacity(params.len());
        let refs = to_rusqlite_params(params, &mut scratch);
        let conn = self
            .inner
            .lock()
            .map_err(|e| anyhow::anyhow!("sqlite mutex: {e}"))?;
        let mut stmt = conn.prepare(sql)?;
        let cols: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
        let rows = stmt.query_map(refs.as_slice(), |row| {
            let mut map = SqlRow::new();
            for (i, col) in cols.iter().enumerate() {
                let v: rusqlite::types::Value = row.get(i)?;
                map.insert(col.clone(), sql_value_from_rusqlite(v));
            }
            Ok(map)
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

/// 工具：把 `serde_json::Value`（ai-agent DatabaseTool 已经用）转成统一 SqlValue。
/// （位于本 crate 意味着不用让上层自己写 rusqlite ToSql 实现。）
pub fn json_to_sql_value(v: &serde_json::Value) -> SqlValue {
    match v {
        serde_json::Value::Null => SqlValue::Null,
        serde_json::Value::Bool(b) => SqlValue::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SqlValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                SqlValue::Real(f)
            } else {
                SqlValue::Text(n.to_string())
            }
        }
        serde_json::Value::String(s) => SqlValue::Text(s.clone()),
        serde_json::Value::Array(a) => SqlValue::Text(serde_json::to_string(a).unwrap_or_default()),
        serde_json::Value::Object(o) => {
            SqlValue::Text(serde_json::to_string(o).unwrap_or_default())
        }
    }
}
