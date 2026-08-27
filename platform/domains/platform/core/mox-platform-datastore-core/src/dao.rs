//! 通用业务数据访问对象（UniversalBizDAO）
//!
//! 企业级通用数据层：一张 biz_data 表 + 预定义扩展槽位列 + 版本链哈希 + 软删除，
//! 支持任意业务实体的 CRUD、过滤、排序、分页，无需为每种实体建表。

use crate::field::{FieldSlotAllocator, FieldSpec, SlotAllocation, SlotType};
use crate::hash::compute_hash;
use crate::memory_repos::{InMemoryIamRepo, InMemoryMetaRepo};
use crate::query::{Filter, ListResult, SortSpec};
use parking_lot::Mutex;
use rusqlite::{params, types::Value as SqlValue, Connection, OptionalExtension};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// 将 serde_json::Value 转换为 rusqlite::types::Value
fn json_to_sql(value: &Value) -> SqlValue {
    match value {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Integer(if *b { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SqlValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                SqlValue::Real(f)
            } else {
                SqlValue::Null
            }
        }
        Value::String(s) => SqlValue::Text(s.clone()),
        Value::Array(_) | Value::Object(_) => SqlValue::Text(value.to_string()),
    }
}

/// 通用业务数据访问对象
#[derive(Clone)]
pub struct UniversalBizDAO {
    conn: Arc<Mutex<Connection>>,
}

impl UniversalBizDAO {
    /// 创建新的 DAO 实例
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 获取底层数据库连接（企业级编排器便捷方法使用）
    pub fn conn(&self) -> &Arc<Mutex<Connection>> {
        &self.conn
    }

    /// 初始化数据库 schema（biz_data 表 + 扩展槽位列 + 索引）
    pub fn init_schema(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock();

        // 构建扩展槽位列定义
        let mut slot_columns = Vec::new();
        for i in 0..SlotType::Str.count() {
            slot_columns.push(format!("ext_str_{} TEXT", i));
        }
        for i in 0..SlotType::Int.count() {
            slot_columns.push(format!("ext_int_{} INTEGER", i));
        }
        for i in 0..SlotType::Dec.count() {
            slot_columns.push(format!("ext_dec_{} REAL", i));
        }
        for i in 0..SlotType::Bool.count() {
            slot_columns.push(format!("ext_bool_{} INTEGER", i));
        }
        for i in 0..SlotType::Ts.count() {
            slot_columns.push(format!("ext_ts_{} TEXT", i));
        }

        let create_sql = format!(
            r#"CREATE TABLE IF NOT EXISTS biz_data (
                id TEXT PRIMARY KEY,
                tenant_id TEXT NOT NULL,
                biz_type TEXT NOT NULL,
                biz_code TEXT NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                prev_hash TEXT,
                curr_hash TEXT NOT NULL,
                data_json TEXT NOT NULL DEFAULT '{{}}',
                created_by TEXT,
                updated_by TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                is_deleted INTEGER NOT NULL DEFAULT 0,
                deleted_by TEXT,
                deleted_at TEXT,
                delete_reason TEXT,
                {}
            )"#,
            slot_columns.join(",\n                ")
        );
        conn.execute_batch(&create_sql)?;

        // 创建索引
        conn.execute_batch(
            r#"CREATE INDEX IF NOT EXISTS idx_biz_tenant_type ON biz_data(tenant_id, biz_type, is_deleted);
               CREATE INDEX IF NOT EXISTS idx_biz_code ON biz_data(tenant_id, biz_type, biz_code);
               CREATE INDEX IF NOT EXISTS idx_biz_created ON biz_data(tenant_id, biz_type, created_at DESC);
               CREATE INDEX IF NOT EXISTS idx_biz_str_0 ON biz_data(ext_str_0);
               CREATE INDEX IF NOT EXISTS idx_biz_str_1 ON biz_data(ext_str_1);
               CREATE INDEX IF NOT EXISTS idx_biz_int_0 ON biz_data(ext_int_0);
               CREATE INDEX IF NOT EXISTS idx_biz_dec_0 ON biz_data(ext_dec_0);"#,
        )?;

        Ok(())
    }

    /// 创建业务记录
    ///
    /// 返回 (biz_id, biz_code, version)
    pub fn create(
        &self,
        meta: &InMemoryMetaRepo,
        iam: &InMemoryIamRepo,
        tenant_id: &str,
        biz_type: &str,
        user_id: &str,
        data: &Map<String, Value>,
        _biz_code_hint: Option<&str>,
        _parent_id: Option<&str>,
    ) -> anyhow::Result<(String, String, i64)> {
        // 权限检查
        if !iam.has_permission(tenant_id, user_id, "biz:create") {
            anyhow::bail!("permission denied: user {} lacks biz:create", user_id);
        }

        let biz_id = Uuid::new_v4().to_string();
        let biz_code = format!("{}-{}", biz_type, Uuid::new_v4().simple().to_string().get(..8).unwrap_or("00000000"));
        let version: i64 = 1;
        let now = chrono::Utc::now().to_rfc3339();

        // 获取字段规格并分配槽位
        let fields = meta.get_entity_fields(tenant_id, biz_type);
        let slot_map = FieldSlotAllocator::allocate(biz_type, &fields);

        // 分离槽位字段和 JSON 扩展字段
        let (slot_values, json_data) = Self::split_data_to_slots(data, &slot_map, &fields);

        // 计算哈希
        let data_value = Value::Object(json_data.clone());
        let curr_hash = compute_hash(None, &biz_id, version, &data_value, user_id, &now);

        // 构建 INSERT SQL
        let mut columns = vec![
            "id", "tenant_id", "biz_type", "biz_code", "version",
            "prev_hash", "curr_hash", "data_json", "created_by", "updated_by",
            "created_at", "updated_at",
        ];
        let mut values: Vec<SqlValue> = vec![
            SqlValue::Text(biz_id.clone()),
            SqlValue::Text(tenant_id.to_string()),
            SqlValue::Text(biz_type.to_string()),
            SqlValue::Text(biz_code.clone()),
            SqlValue::Integer(version),
            SqlValue::Null,
            SqlValue::Text(curr_hash),
            SqlValue::Text(serde_json::to_string(&data_value)?),
            SqlValue::Text(user_id.to_string()),
            SqlValue::Text(user_id.to_string()),
            SqlValue::Text(now.clone()),
            SqlValue::Text(now.clone()),
        ];

        for (slot_name, slot_value) in &slot_values {
            columns.push(slot_name.as_str());
            values.push(json_to_sql(slot_value));
        }

        let placeholders: Vec<String> = (0..values.len()).map(|i| format!("?{}", i + 1)).collect();
        let sql = format!(
            "INSERT INTO biz_data ({}) VALUES ({})",
            columns.join(", "),
            placeholders.join(", ")
        );

        let conn = self.conn.lock();
        let params_ref: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        conn.execute(&sql, params_ref.as_slice())?;

        Ok((biz_id, biz_code, version))
    }

    /// 获取业务记录
    pub fn get(
        &self,
        meta: &InMemoryMetaRepo,
        tenant_id: &str,
        biz_type: &str,
        biz_id: &str,
    ) -> anyhow::Result<Option<Value>> {
        let conn = self.conn.lock();

        let fields = meta.get_entity_fields(tenant_id, biz_type);
        let slot_map = FieldSlotAllocator::allocate(biz_type, &fields);

        // 构建查询列
        let mut select_cols = vec![
            "id", "tenant_id", "biz_type", "biz_code", "version",
            "prev_hash", "curr_hash", "data_json", "created_by", "updated_by",
            "created_at", "updated_at", "is_deleted",
        ];
        for slot in slot_map.values() {
            if !slot.slot_name.contains("_overflow_") {
                select_cols.push(slot.slot_name.as_str());
            }
        }

        let sql = format!(
            "SELECT {} FROM biz_data WHERE tenant_id = ?1 AND biz_type = ?2 AND id = ?3 AND is_deleted = 0",
            select_cols.join(", ")
        );

        let result = conn
            .query_row(&sql, params![tenant_id, biz_type, biz_id], |row| {
                Self::row_to_value(row, &select_cols, &slot_map, &fields)
            })
            .optional()?;

        Ok(result)
    }

    /// 更新业务记录（返回新版本号）
    pub fn update(
        &self,
        meta: &InMemoryMetaRepo,
        tenant_id: &str,
        biz_type: &str,
        biz_id: &str,
        user_id: &str,
        patch: &Map<String, Value>,
    ) -> anyhow::Result<i64> {
        let conn = self.conn.lock();

        // 获取当前记录
        let current: Option<(i64, String, String)> = conn
            .query_row(
                "SELECT version, curr_hash, data_json FROM biz_data WHERE tenant_id = ?1 AND biz_type = ?2 AND id = ?3 AND is_deleted = 0",
                params![tenant_id, biz_type, biz_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;

        let (old_version, old_hash, old_data_json) = match current {
            Some(v) => v,
            None => anyhow::bail!("record not found: {}/{}", biz_type, biz_id),
        };

        let new_version = old_version + 1;
        let now = chrono::Utc::now().to_rfc3339();

        // 合并 patch 到旧数据
        let mut merged: Map<String, Value> = serde_json::from_str(&old_data_json)?;
        for (k, v) in patch {
            merged.insert(k.clone(), v.clone());
        }

        // 获取字段规格和槽位映射
        let fields = meta.get_entity_fields(tenant_id, biz_type);
        let slot_map = FieldSlotAllocator::allocate(biz_type, &fields);

        // 分离槽位字段和 JSON 字段
        let (slot_values, json_data) = Self::split_data_to_slots(&merged, &slot_map, &fields);

        // 计算新哈希
        let data_value = Value::Object(json_data);
        let new_hash = compute_hash(Some(&old_hash), biz_id, new_version, &data_value, user_id, &now);

        // 构建 UPDATE SQL
        let mut set_clauses = vec![
            "version = ?".to_string(),
            "prev_hash = ?".to_string(),
            "curr_hash = ?".to_string(),
            "data_json = ?".to_string(),
            "updated_by = ?".to_string(),
            "updated_at = ?".to_string(),
        ];
        let mut values: Vec<SqlValue> = vec![
            SqlValue::Integer(new_version),
            SqlValue::Text(old_hash),
            SqlValue::Text(new_hash),
            SqlValue::Text(serde_json::to_string(&data_value)?),
            SqlValue::Text(user_id.to_string()),
            SqlValue::Text(now),
        ];

        for (slot_name, slot_value) in &slot_values {
            set_clauses.push(format!("{} = ?", slot_name));
            values.push(json_to_sql(slot_value));
        }

        values.push(SqlValue::Text(tenant_id.to_string()));
        values.push(SqlValue::Text(biz_type.to_string()));
        values.push(SqlValue::Text(biz_id.to_string()));

        let sql = format!(
            "UPDATE biz_data SET {} WHERE tenant_id = ? AND biz_type = ? AND id = ? AND is_deleted = 0",
            set_clauses.join(", ")
        );

        let params_ref: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        conn.execute(&sql, params_ref.as_slice())?;

        Ok(new_version)
    }

    /// 列表查询（支持过滤、排序、分页）
    pub fn list(
        &self,
        meta: &InMemoryMetaRepo,
        tenant_id: &str,
        biz_type: &str,
        filters: Vec<Filter>,
        sort: SortSpec,
        page: i64,
        page_size: i64,
    ) -> anyhow::Result<ListResult> {
        let conn = self.conn.lock();

        let fields = meta.get_entity_fields(tenant_id, biz_type);
        let slot_map = FieldSlotAllocator::allocate(biz_type, &fields);

        // 构建查询列
        let mut select_cols = vec![
            "id", "tenant_id", "biz_type", "biz_code", "version",
            "prev_hash", "curr_hash", "data_json", "created_by", "updated_by",
            "created_at", "updated_at", "is_deleted",
        ];
        for slot in slot_map.values() {
            if !slot.slot_name.contains("_overflow_") {
                select_cols.push(slot.slot_name.as_str());
            }
        }

        // 构建 WHERE 子句
        let mut where_clauses = vec!["tenant_id = ?".to_string(), "biz_type = ?".to_string(), "is_deleted = 0".to_string()];
        let mut param_values: Vec<SqlValue> = vec![
            SqlValue::Text(tenant_id.to_string()),
            SqlValue::Text(biz_type.to_string()),
        ];

        for filter in &filters {
            if let Some(slot) = slot_map.get(&filter.field_code) {
                if !slot.slot_name.contains("_overflow_") {
                    let (sql_frag, val) = filter.to_sql(&slot.slot_name);
                    where_clauses.push(sql_frag);
                    if let Some(v) = val {
                        param_values.push(SqlValue::Text(v));
                    }
                }
            }
        }

        // 排序
        let sort_col = sort.field.as_ref().and_then(|f| slot_map.get(f)).map(|s| s.slot_name.clone());
        let order_sql = sort.to_sql(sort_col.as_deref());

        // 分页
        let offset = (page - 1) * page_size;

        // COUNT 查询
        let count_sql = format!(
            "SELECT COUNT(*) FROM biz_data WHERE {}",
            where_clauses.join(" AND ")
        );
        let count_params: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        let total: i64 = conn.query_row(&count_sql, count_params.as_slice(), |r| r.get(0))?;

        // 数据查询
        let data_sql = format!(
            "SELECT {} FROM biz_data WHERE {} {} LIMIT ? OFFSET ?",
            select_cols.join(", "),
            where_clauses.join(" AND "),
            order_sql
        );
        let mut data_params = param_values.clone();
        data_params.push(SqlValue::Integer(page_size));
        data_params.push(SqlValue::Integer(offset));
        let data_params_ref: Vec<&dyn rusqlite::ToSql> = data_params.iter().map(|v| v as &dyn rusqlite::ToSql).collect();

        let mut stmt = conn.prepare(&data_sql)?;
        let items: Vec<Value> = stmt
            .query_map(data_params_ref.as_slice(), |row| {
                Self::row_to_value(row, &select_cols, &slot_map, &fields)
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ListResult { total, items, page, page_size })
    }

    /// 软删除业务记录
    pub fn delete(
        &self,
        tenant_id: &str,
        biz_type: &str,
        biz_id: &str,
        user_id: &str,
        reason: Option<&str>,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();

        let affected = conn.execute(
            "UPDATE biz_data SET is_deleted = 1, deleted_by = ?, deleted_at = ?, delete_reason = ?, updated_at = ? WHERE tenant_id = ? AND biz_type = ? AND id = ? AND is_deleted = 0",
            params![user_id, now, reason, now, tenant_id, biz_type, biz_id],
        )?;

        if affected == 0 {
            anyhow::bail!("record not found or already deleted: {}/{}", biz_type, biz_id);
        }

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════
    // 内部辅助方法
    // ═══════════════════════════════════════════════════════════════

    /// 将数据 Map 分离为槽位列值和 JSON 扩展字段
    fn split_data_to_slots(
        data: &Map<String, Value>,
        slot_map: &HashMap<String, SlotAllocation>,
        _fields: &[FieldSpec],
    ) -> (HashMap<String, Value>, Map<String, Value>) {
        let mut slot_values = HashMap::new();
        let mut json_data = Map::new();

        for (key, value) in data {
            if let Some(slot) = slot_map.get(key) {
                if slot.slot_name.contains("_overflow_") {
                    // 溢出槽位放入 JSON
                    json_data.insert(key.clone(), value.clone());
                } else {
                    // 根据槽位类型转换值
                    let converted = match slot.slot_type {
                        SlotType::Bool => match value {
                            Value::Bool(b) => Value::Number(if *b { 1.into() } else { 0.into() }),
                            Value::Number(n) => Value::Number(n.clone()),
                            _ => Value::Null,
                        },
                        SlotType::Int => match value {
                            Value::Number(n) => Value::Number(n.clone()),
                            Value::String(s) => s.parse::<i64>().ok().map(|n| Value::Number(n.into())).unwrap_or(Value::Null),
                            _ => Value::Null,
                        },
                        SlotType::Dec => match value {
                            Value::Number(n) => Value::Number(n.clone()),
                            Value::String(s) => s.parse::<f64>().ok().and_then(serde_json::Number::from_f64).map(Value::Number).unwrap_or(Value::Null),
                            _ => Value::Null,
                        },
                        _ => value.clone(),
                    };
                    slot_values.insert(slot.slot_name.clone(), converted);
                }
            } else {
                // 未分配槽位的字段放入 JSON
                json_data.insert(key.clone(), value.clone());
            }
        }

        (slot_values, json_data)
    }

    /// 将数据库行转换为 JSON Value（合并槽位列和 JSON 字段）
    fn row_to_value(
        row: &rusqlite::Row,
        select_cols: &[&str],
        slot_map: &HashMap<String, SlotAllocation>,
        _fields: &[FieldSpec],
    ) -> rusqlite::Result<Value> {
        let mut obj = Map::new();

        // 读取基础字段
        for (idx, col) in select_cols.iter().enumerate() {
            match *col {
                "id" => { obj.insert("id".to_string(), Value::String(row.get(idx)?)); }
                "tenant_id" => { obj.insert("tenant_id".to_string(), Value::String(row.get(idx)?)); }
                "biz_type" => { obj.insert("biz_type".to_string(), Value::String(row.get(idx)?)); }
                "biz_code" => { obj.insert("biz_code".to_string(), Value::String(row.get(idx)?)); }
                "version" => {
                    let v: i64 = row.get(idx)?;
                    obj.insert("version".to_string(), Value::Number(v.into()));
                }
                "prev_hash" => {
                    let v: Option<String> = row.get(idx)?;
                    obj.insert("prev_hash".to_string(), v.map(Value::String).unwrap_or(Value::Null));
                }
                "curr_hash" => { obj.insert("curr_hash".to_string(), Value::String(row.get(idx)?)); }
                "data_json" => {
                    let json_str: String = row.get(idx)?;
                    if let Ok(parsed) = serde_json::from_str::<Value>(&json_str) {
                        if let Value::Object(map) = parsed {
                            for (k, v) in map { obj.insert(k, v); }
                        }
                    }
                }
                "created_by" => { obj.insert("created_by".to_string(), Value::String(row.get(idx)?)); }
                "updated_by" => { obj.insert("updated_by".to_string(), Value::String(row.get(idx)?)); }
                "created_at" => { obj.insert("created_at".to_string(), Value::String(row.get(idx)?)); }
                "updated_at" => { obj.insert("updated_at".to_string(), Value::String(row.get(idx)?)); }
                "is_deleted" => {
                    let v: i64 = row.get(idx)?;
                    obj.insert("is_deleted".to_string(), Value::Bool(v != 0));
                }
                _ => {
                    // 槽位列：反查 field_code
                    if let Some(field_code) = slot_map.iter().find(|(_, s)| s.slot_name == *col).map(|(k, _)| k.clone()) {
                        let slot = slot_map.get(&field_code).unwrap();
                        let val: Value = match slot.slot_type {
                            SlotType::Str => {
                                let v: Option<String> = row.get(idx)?;
                                v.map(Value::String).unwrap_or(Value::Null)
                            }
                            SlotType::Int => {
                                let v: Option<i64> = row.get(idx)?;
                                v.map(|n| Value::Number(n.into())).unwrap_or(Value::Null)
                            }
                            SlotType::Dec => {
                                let v: Option<f64> = row.get(idx)?;
                                v.and_then(serde_json::Number::from_f64).map(Value::Number).unwrap_or(Value::Null)
                            }
                            SlotType::Bool => {
                                let v: Option<i64> = row.get(idx)?;
                                v.map(|n| Value::Bool(n != 0)).unwrap_or(Value::Null)
                            }
                            SlotType::Ts => {
                                let v: Option<String> = row.get(idx)?;
                                v.map(Value::String).unwrap_or(Value::Null)
                            }
                        };
                        obj.insert(field_code, val);
                    }
                }
            }
        }

        Ok(Value::Object(obj))
    }
}
