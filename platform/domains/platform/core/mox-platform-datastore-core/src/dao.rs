use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;
use dashmap::DashMap;
use rusqlite::{params, OptionalExtension, Row};
use serde_json::{Map, Value};
use uuid::Uuid;
use chrono::Utc;

use crate::port::{EntityWithFields, MetaRepository, IamRepository};
use crate::audit_chain::compute_hash;
use crate::slot_allocator::{FieldSlotAllocator, SlotCategory};

pub const DDL_SQL: &str = include_str!("ddl.sql");

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Filter {
    pub field_code: String,
    pub operator: String,
    pub value: Value,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SortSpec {
    pub field_code: Option<String>,
    pub desc: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ListResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

pub struct UniversalBizDAO {
    pub conn: Arc<Mutex<rusqlite::Connection>>,
    pub meta_cache: DashMap<(String, String), EntityWithFields>,
}

impl UniversalBizDAO {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self {
            conn,
            meta_cache: DashMap::new(),
        }
    }

    pub fn init_schema(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(DDL_SQL)
            .map_err(|e| anyhow::anyhow!("Init schema failed: {}", e))?;
        Ok(())
    }

    fn get_cached_entity(
        &self,
        meta_repo: &dyn MetaRepository,
        tenant_id: &str,
        entity_code: &str,
    ) -> anyhow::Result<EntityWithFields> {
        let key = (tenant_id.to_string(), entity_code.to_string());
        if let Some(e) = self.meta_cache.get(&key) {
            return Ok(e.clone());
        }
        let entity = meta_repo.get_entity(tenant_id, entity_code)?;
        self.meta_cache.insert(key.clone(), entity.clone());
        Ok(entity)
    }

    fn map_value_to_slot(
        val: &Value,
        cat: SlotCategory,
    ) -> (
        Option<String>,
        Option<i64>,
        Option<f64>,
    ) {
        match cat {
            SlotCategory::Bool => {
                let b = val.as_bool().unwrap_or(false);
                (None, Some(if b { 1 } else { 0 }), None)
            }
            SlotCategory::Int => {
                let i = val.as_i64().or_else(|| val.as_f64().map(|f| f as i64)).unwrap_or(0);
                (None, Some(i), None)
            }
            SlotCategory::Decimal => {
                let d = val.as_f64().unwrap_or(0.0);
                (None, None, Some(d))
            }
            SlotCategory::Json | SlotCategory::Overflow => {
                let s = serde_json::to_string(val).unwrap_or_default();
                (Some(s), None, None)
            }
            SlotCategory::Date | SlotCategory::DateTime | SlotCategory::Text | SlotCategory::String => {
                let s = match val {
                    Value::String(s) => s.clone(),
                    other => serde_json::to_string(other).unwrap_or_default(),
                };
                (Some(s), None, None)
            }
        }
    }

    fn slot_from_row(slot: &str, row: &Row) -> anyhow::Result<Option<Value>> {
        let prefix: String = slot.chars().take(8).collect();
        match prefix.as_str() {
            "ext_str_" | "ext_text" | "ext_json" | "ext_date" | "ext_da" => {
                let v: Option<String> = row.get(slot).ok();
                if let Some(ref s) = v {
                    if slot.starts_with("ext_json") {
                        if let Ok(jv) = serde_json::from_str::<Value>(s) {
                            return Ok(Some(jv));
                        }
                    }
                }
                Ok(v.map(Value::String))
            }
            "ext_int_" => {
                let v: Option<i64> = row.get(slot).ok();
                Ok(v.map(Value::from))
            }
            "ext_dec_" => {
                let v: Option<f64> = row.get(slot).ok();
                Ok(v.map(Value::from))
            }
            "ext_bool" => {
                let v: Option<i64> = row.get(slot).ok();
                Ok(v.map(|n| Value::Bool(n != 0)))
            }
            _ => Ok(None),
        }
    }

    fn slot_value_for_sql(
        slot: &str,
        s: &Option<String>,
        i: &Option<i64>,
        d: &Option<f64>,
    ) -> rusqlite::types::ToSqlOutput<'static> {
        use rusqlite::types::{ToSqlOutput, Value as RV};
        if slot.starts_with("ext_int") {
            if let Some(n) = i {
                return ToSqlOutput::Owned(RV::Integer(*n));
            }
        } else if slot.starts_with("ext_dec") {
            if let Some(n) = d {
                return ToSqlOutput::Owned(RV::Real(*n));
            }
        } else if slot.starts_with("ext_bool") {
            if let Some(n) = i {
                return ToSqlOutput::Owned(RV::Integer(*n));
            }
        } else {
            if let Some(ref ss) = s {
                return ToSqlOutput::Owned(RV::Text(ss.clone()));
            }
        }
        ToSqlOutput::Owned(RV::Null)
    }

    pub fn create(
        &self,
        meta_repo: &dyn MetaRepository,
        _iam_repo: &dyn IamRepository,
        tenant_id: &str,
        entity_code: &str,
        user_id: &str,
        data: &Map<String, Value>,
        workflow_instance_id: Option<&str>,
        biz_code_input: Option<&str>,
    ) -> anyhow::Result<(String, String, i64)> {
        let entity = self.get_cached_entity(meta_repo, tenant_id, entity_code)?;
        let alloc = FieldSlotAllocator::allocate(entity_code, &entity.fields);

        let mut cols: Vec<String> = vec![
            "biz_id".into(), "tenant_id".into(), "entity_id".into(), "biz_code".into(),
            "biz_type".into(), "biz_status".into(), "dynamic_data".into(),
            "creator_user_id".into(), "owner_user_id".into(), "created_at".into(),
            "updated_at".into(), "created_by".into(), "updated_by".into(),
            "version".into(), "trace_id".into(), "curr_hash".into(), "version_group_id".into(),
            "workflow_instance_id".into(), "workflow_status".into(),
        ];
        let mut values: Vec<rusqlite::types::ToSqlOutput<'static>> = Vec::new();
        let mut param_placeholders: Vec<String> = Vec::new();

        let biz_id = Uuid::now_v7().to_string();
        let version: i64 = 1;
        let now = Utc::now().to_rfc3339();
        let biz_code = biz_code_input
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{}-{}", entity_code, Uuid::now_v7().simple()));
        let trace_id = Uuid::new_v4().to_string();
        let version_group_id = Uuid::now_v7().to_string();

        let mut slot_values: HashMap<String, (Option<String>, Option<i64>, Option<f64>)> = HashMap::new();
        let mut dynamic_data_map: Map<String, Value> = Map::new();

        for (field_code, val) in data.iter() {
            if let Some(slot) = alloc.get(field_code) {
                let (s, i, d) = Self::map_value_to_slot(val, slot.category);
                if slot.slot_name == "dynamic_data" {
                    dynamic_data_map.insert(field_code.clone(), val.clone());
                } else {
                    slot_values.insert(slot.slot_name.clone(), (s, i, d));
                    if !cols.iter().any(|c| c == &slot.slot_name) {
                        cols.push(slot.slot_name.clone());
                    }
                }
            } else {
                dynamic_data_map.insert(field_code.clone(), val.clone());
            }
        }

        for c in cols.iter() {
            match c.as_str() {
                "biz_id" => { values.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(biz_id.clone()))); }
                "tenant_id" => { values.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(tenant_id.to_string()))); }
                "entity_id" => { values.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(entity.entity_id.clone()))); }
                "biz_code" => { values.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(biz_code.clone()))); }
                "biz_type" => { values.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(entity_code.to_string()))); }
                "biz_status" => {
                    let status = data.get("status").and_then(|v| v.as_str()).unwrap_or("active").to_string();
                    values.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(status)));
                }
                "dynamic_data" => {
                    let s = serde_json::to_string(&Value::Object(dynamic_data_map.clone())).unwrap_or_default();
                    values.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(s)));
                }
                "creator_user_id" | "owner_user_id" => {
                    values.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(user_id.to_string())));
                }
                "created_at" | "updated_at" => {
                    values.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(now.clone())));
                }
                "created_by" | "updated_by" => {
                    values.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(user_id.to_string())));
                }
                "version" => {
                    values.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Integer(version)));
                }
                "trace_id" => {
                    values.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(trace_id.clone())));
                }
                "curr_hash" => {
                    values.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Null));
                }
                "version_group_id" => {
                    values.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(version_group_id.clone())));
                }
                "workflow_instance_id" => {
                    let v = workflow_instance_id.map(|s| s.to_string());
                    values.push(match v {
                        Some(s) => rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(s)),
                        None => rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Null),
                    });
                }
                "workflow_status" => {
                    values.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Null));
                }
                other => {
                    let sv = slot_values.get(other).cloned().unwrap_or((None, None, None));
                    values.push(Self::slot_value_for_sql(other, &sv.0, &sv.1, &sv.2));
                }
            }
            param_placeholders.push("?".to_string());
        }

        let sql = format!(
            "INSERT OR IGNORE INTO biz_data ({}) VALUES ({})",
            cols.join(","),
            param_placeholders.join(",")
        );

        let conn = self.conn.lock();
        let refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        conn.execute(&sql, refs.as_slice())
            .map_err(|e| anyhow::anyhow!("INSERT biz_data failed: {}", e))?;

        let snapshot_after: Value = Value::Object(data.clone());
        let curr_hash = compute_hash(None, &biz_id, version, &snapshot_after, user_id, &now);

        conn.execute(
            "UPDATE biz_data SET curr_hash = ?1 WHERE biz_id = ?2",
            params![curr_hash.clone(), biz_id],
        ).ok();

        let version_id = Uuid::now_v7().to_string();
        let changed_fields: Vec<String> = data.keys().cloned().collect();
        conn.execute(
            "INSERT INTO biz_data_version (version_id, biz_id, tenant_id, entity_id, version_num, snapshot_before, snapshot_after, changed_fields, change_note, operation_type, operator_user_id, prev_hash, curr_hash, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                version_id,
                biz_id,
                tenant_id,
                entity.entity_id,
                version,
                "",
                serde_json::to_string(&snapshot_after).unwrap_or_default(),
                serde_json::to_string(&changed_fields).unwrap_or_default(),
                "create",
                "CREATE",
                user_id,
                "",
                curr_hash,
                now,
            ],
        ).map_err(|e| anyhow::anyhow!("INSERT version failed: {}", e))?;

        Ok((biz_id, biz_code, version))
    }

    pub fn get(
        &self,
        meta_repo: &dyn MetaRepository,
        tenant_id: &str,
        entity_code: &str,
        biz_id: &str,
    ) -> anyhow::Result<Option<Value>> {
        let entity = self.get_cached_entity(meta_repo, tenant_id, entity_code)?;
        let conn = self.conn.lock();
        let row = conn.query_row(
            "SELECT * FROM biz_data WHERE biz_id = ?1 AND tenant_id = ?2 AND deleted_at IS NULL",
            params![biz_id, tenant_id],
            |row| self.row_to_map(row, &entity),
        ).optional().map_err(|e| anyhow::anyhow!("SELECT failed: {}", e))?;

        Ok(row)
    }

    fn row_to_map(&self, row: &Row, entity: &EntityWithFields) -> Result<Value, rusqlite::Error> {
        let alloc = FieldSlotAllocator::allocate(&entity.entity_code, &entity.fields);
        let mut result = Map::new();

        let biz_id: String = row.get("biz_id")?;
        result.insert("biz_id".to_string(), Value::String(biz_id));
        let biz_code: Option<String> = row.get("biz_code").ok();
        if let Some(bc) = biz_code { result.insert("biz_code".to_string(), Value::String(bc)); }
        let version: Option<i64> = row.get("version").ok();
        if let Some(v) = version { result.insert("version".to_string(), Value::from(v)); }
        let status: Option<String> = row.get("biz_status").ok();
        if let Some(s) = status { result.insert("biz_status".to_string(), Value::String(s)); }
        let wf: Option<String> = row.get("workflow_instance_id").ok();
        if let Some(w) = wf { result.insert("workflow_instance_id".to_string(), Value::String(w)); }
        let created_at: Option<String> = row.get("created_at").ok();
        if let Some(s) = created_at { result.insert("created_at".to_string(), Value::String(s)); }
        let updated_at: Option<String> = row.get("updated_at").ok();
        if let Some(s) = updated_at { result.insert("updated_at".to_string(), Value::String(s)); }
        let hash: Option<String> = row.get("curr_hash").ok();
        if let Some(h) = hash { result.insert("curr_hash".to_string(), Value::String(h)); }

        for field in &entity.fields {
            if let Some(slot) = alloc.get(&field.field_code) {
                if slot.slot_name == "dynamic_data" {
                    continue;
                }
                if let Ok(Some(v)) = Self::slot_from_row(&slot.slot_name, row) {
                    result.insert(field.field_code.clone(), v);
                }
            }
        }

        let dyn_text: Option<String> = row.get("dynamic_data").ok();
        if let Some(ref dt) = dyn_text {
            if let Ok(Value::Object(dyn_map)) = serde_json::from_str::<Value>(dt) {
                for (k, v) in dyn_map {
                    result.insert(k, v);
                }
            }
        }

        Ok(Value::Object(result))
    }

    pub fn list(
        &self,
        meta_repo: &dyn MetaRepository,
        tenant_id: &str,
        entity_code: &str,
        filters: Vec<Filter>,
        sort: SortSpec,
        page: i64,
        page_size: i64,
    ) -> anyhow::Result<ListResult<Value>> {
        let entity = self.get_cached_entity(meta_repo, tenant_id, entity_code)?;
        let alloc = FieldSlotAllocator::allocate(entity_code, &entity.fields);

        let mut where_clauses: Vec<String> = vec![
            "tenant_id = ?".to_string(),
            "entity_id = ?".to_string(),
            "deleted_at IS NULL".to_string(),
        ];
        let mut args: Vec<rusqlite::types::ToSqlOutput<'static>> = vec![
            rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(tenant_id.to_string())),
            rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(entity.entity_id.clone())),
        ];

        for f in filters {
            let slot = alloc.get(&f.field_code);
            let col = match slot {
                Some(s) if s.slot_name != "dynamic_data" => s.slot_name.clone(),
                _ => format!("json_extract(dynamic_data, '$.{}')", f.field_code),
            };
            let placeholder_idx = args.len() + 1;
            match f.operator.as_str() {
                "eq" | "=" => {
                    where_clauses.push(format!("{} = ?{}", col, placeholder_idx));
                }
                "ne" | "!=" => {
                    where_clauses.push(format!("{} != ?{}", col, placeholder_idx));
                }
                "gt" => {
                    where_clauses.push(format!("{} > ?{}", col, placeholder_idx));
                }
                "gte" => {
                    where_clauses.push(format!("{} >= ?{}", col, placeholder_idx));
                }
                "lt" => {
                    where_clauses.push(format!("{} < ?{}", col, placeholder_idx));
                }
                "lte" => {
                    where_clauses.push(format!("{} <= ?{}", col, placeholder_idx));
                }
                "like" => {
                    where_clauses.push(format!("{} LIKE ?{}", col, placeholder_idx));
                }
                _ => {
                    where_clauses.push(format!("{} = ?{}", col, placeholder_idx));
                }
            }
            let av = match f.value {
                Value::String(s) => rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(s)),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Integer(i))
                    } else if let Some(fv) = n.as_f64() {
                        rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Real(fv))
                    } else {
                        rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Null)
                    }
                }
                Value::Bool(b) => rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Integer(if b { 1 } else { 0 })),
                _ => rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Null),
            };
            args.push(av);
        }

        let where_sql = format!("WHERE {}", where_clauses.join(" AND "));

        let order_sql = if let Some(ref fc) = sort.field_code {
            let slot = alloc.get(fc);
            let col = match slot {
                Some(s) if s.slot_name != "dynamic_data" => s.slot_name.clone(),
                _ => format!("json_extract(dynamic_data, '$.{}')", fc),
            };
            format!("ORDER BY {} {}", col, if sort.desc { "DESC" } else { "ASC" })
        } else {
            "ORDER BY created_at DESC".to_string()
        };

        let limit = page_size.max(1);
        let offset = (page.max(1) - 1) * limit;

        let conn = self.conn.lock();
        let total_sql = format!("SELECT COUNT(*) FROM biz_data {}", where_sql);
        let refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        let total: i64 = conn.query_row(&total_sql, refs.as_slice(), |r| r.get(0))
            .map_err(|e| anyhow::anyhow!("COUNT failed: {}", e))?;

        let select_sql = format!("SELECT * FROM biz_data {} {} LIMIT ? OFFSET ?", where_sql, order_sql);
        let mut stmt = conn.prepare(&select_sql)
            .map_err(|e| anyhow::anyhow!("PREPARE list failed: {}", e))?;

        let mut all_args: Vec<rusqlite::types::ToSqlOutput<'static>> = args.clone();
        all_args.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Integer(limit)));
        all_args.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Integer(offset)));
        let refs2: Vec<&dyn rusqlite::ToSql> = all_args.iter().map(|v| v as &dyn rusqlite::ToSql).collect();

        let rows = stmt.query_map(refs2.as_slice(), |row| self.row_to_map(row, &entity))
            .map_err(|e| anyhow::anyhow!("QUERY list failed: {}", e))?;

        let mut items: Vec<Value> = Vec::new();
        for r in rows {
            items.push(r.map_err(|e| anyhow::anyhow!("row error: {}", e))?);
        }

        Ok(ListResult {
            items,
            total,
            page: page.max(1),
            page_size: limit,
        })
    }

    pub fn update(
        &self,
        meta_repo: &dyn MetaRepository,
        tenant_id: &str,
        entity_code: &str,
        biz_id: &str,
        user_id: &str,
        patch: &Map<String, Value>,
    ) -> anyhow::Result<i64> {
        let entity = self.get_cached_entity(meta_repo, tenant_id, entity_code)?;
        let alloc = FieldSlotAllocator::allocate(entity_code, &entity.fields);
        let now = Utc::now().to_rfc3339();

        let conn = self.conn.lock();

        let (old_json, old_version, prev_hash): (Option<String>, i64, Option<String>) = conn.query_row(
            "SELECT dynamic_data, version, curr_hash FROM biz_data WHERE biz_id = ?1 AND tenant_id = ?2 AND deleted_at IS NULL",
            params![biz_id, tenant_id],
            |r| Ok((r.get("dynamic_data").ok(), r.get::<_, i64>("version").unwrap_or(0), r.get("curr_hash").ok())),
        ).map_err(|e| anyhow::anyhow!("Fetch before update failed: {}", e))?;

        let new_version = old_version + 1;

        let mut set_parts: Vec<String> = Vec::new();
        let mut args: Vec<rusqlite::types::ToSqlOutput<'static>> = Vec::new();

        let slot_vals: HashMap<String, (Option<String>, Option<i64>, Option<f64>)> = HashMap::new();
        let _ = slot_vals;

        for (fc, val) in patch.iter() {
            if let Some(slot) = alloc.get(fc) {
                if slot.slot_name != "dynamic_data" {
                    let (s, i, d) = Self::map_value_to_slot(val, slot.category);
                    set_parts.push(format!("{} = ?{}", slot.slot_name, args.len() + 1));
                    args.push(Self::slot_value_for_sql(&slot.slot_name, &s, &i, &d));
                }
            }
        }

        let existing_dyn: Map<String, Value> = old_json
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .and_then(|v| match v {
                Value::Object(m) => Some(m),
                _ => None,
            })
            .unwrap_or_default();

        let mut new_dyn = existing_dyn.clone();
        for (fc, val) in patch.iter() {
            let should_dyn = if let Some(slot) = alloc.get(fc) {
                slot.slot_name == "dynamic_data"
            } else {
                true
            };
            if should_dyn {
                new_dyn.insert(fc.clone(), val.clone());
            }
        }

        if !new_dyn.is_empty() {
            set_parts.push(format!("dynamic_data = ?{}", args.len() + 1));
            args.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(
                serde_json::to_string(&Value::Object(new_dyn.clone())).unwrap_or_default()
            )));
        }

        set_parts.push(format!("version = ?{}", args.len() + 1));
        args.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Integer(new_version)));

        set_parts.push(format!("updated_at = ?{}", args.len() + 1));
        args.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(now.clone())));

        set_parts.push(format!("updated_by = ?{}", args.len() + 1));
        args.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(user_id.to_string())));

        if let Some(status_v) = patch.get("status") {
            if let Some(status) = status_v.as_str() {
                set_parts.push(format!("biz_status = ?{}", args.len() + 1));
                args.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(status.to_string())));
            }
        }

        args.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(biz_id.to_string())));
        args.push(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(tenant_id.to_string())));

        let sql = format!(
            "UPDATE biz_data SET {} WHERE biz_id = ?{} AND tenant_id = ?{} AND deleted_at IS NULL",
            set_parts.join(","),
            args.len() - 1,
            args.len()
        );

        let refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        let affected = conn.execute(&sql, refs.as_slice())
            .map_err(|e| anyhow::anyhow!("UPDATE failed: {}", e))?;
        if affected == 0 {
            anyhow::bail!("Update failed: no rows affected for biz_id={}", biz_id);
        }

        let snapshot_before: Value = Value::Object(existing_dyn);
        let snapshot_after: Value = Value::Object(new_dyn);
        let curr_hash = compute_hash(prev_hash.as_deref(), biz_id, new_version, &snapshot_after, user_id, &now);

        conn.execute(
            "UPDATE biz_data SET curr_hash = ?1 WHERE biz_id = ?2",
            params![curr_hash.clone(), biz_id],
        ).ok();

        let changed_fields: Vec<String> = patch.keys().cloned().collect();
        let version_id = Uuid::now_v7().to_string();
        conn.execute(
            "INSERT INTO biz_data_version (version_id, biz_id, tenant_id, entity_id, version_num, snapshot_before, snapshot_after, changed_fields, change_note, operation_type, operator_user_id, prev_hash, curr_hash, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                version_id,
                biz_id,
                tenant_id,
                entity.entity_id,
                new_version,
                serde_json::to_string(&snapshot_before).unwrap_or_default(),
                serde_json::to_string(&snapshot_after).unwrap_or_default(),
                serde_json::to_string(&changed_fields).unwrap_or_default(),
                "update",
                "UPDATE",
                user_id,
                prev_hash.unwrap_or_default(),
                curr_hash,
                now,
            ],
        ).map_err(|e| anyhow::anyhow!("INSERT version (update) failed: {}", e))?;

        Ok(new_version)
    }

    pub fn delete(
        &self,
        tenant_id: &str,
        _entity_code: &str,
        biz_id: &str,
        user_id: &str,
        change_note: Option<&str>,
    ) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock();

        let (entity_id, version, prev_hash): (Option<String>, i64, Option<String>) = conn.query_row(
            "SELECT entity_id, version, curr_hash FROM biz_data WHERE biz_id = ?1 AND tenant_id = ?2 AND deleted_at IS NULL",
            params![biz_id, tenant_id],
            |r| Ok((r.get("entity_id").ok(), r.get::<_, i64>("version").unwrap_or(0), r.get("curr_hash").ok())),
        ).map_err(|e| anyhow::anyhow!("Fetch before delete failed: {}", e))?;

        let new_version = version + 1;

        let affected = conn.execute(
            "UPDATE biz_data SET deleted_at = ?1, deleted_by = ?2, updated_at = ?1, updated_by = ?2, version = ?3 WHERE biz_id = ?4 AND tenant_id = ?5 AND deleted_at IS NULL",
            params![now, user_id, new_version, biz_id, tenant_id],
        ).map_err(|e| anyhow::anyhow!("DELETE (soft) failed: {}", e))?;
        if affected == 0 {
            anyhow::bail!("Delete failed: no rows affected");
        }

        let version_id = Uuid::now_v7().to_string();
        let snapshot_after = serde_json::json!({"deleted": true});
        let curr_hash = compute_hash(prev_hash.as_deref(), biz_id, new_version, &snapshot_after, user_id, &now);

        conn.execute(
            "INSERT INTO biz_data_version (version_id, biz_id, tenant_id, entity_id, version_num, snapshot_before, snapshot_after, changed_fields, change_note, operation_type, operator_user_id, prev_hash, curr_hash, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                version_id,
                biz_id,
                tenant_id,
                entity_id.unwrap_or_default(),
                new_version,
                "",
                serde_json::to_string(&snapshot_after).unwrap_or_default(),
                "[\"deleted_at\"]",
                change_note.unwrap_or("soft delete"),
                "DELETE",
                user_id,
                prev_hash.unwrap_or_default(),
                curr_hash,
                now,
            ],
        ).ok();

        Ok(())
    }
}
