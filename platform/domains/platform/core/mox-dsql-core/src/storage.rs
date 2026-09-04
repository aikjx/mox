// mox-dsql-core 存储层：SQL定义的CRUD操作
use crate::error::{DsqlError, DsqlResult};
use crate::model::*;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Arc;
use parking_lot::Mutex;

/// 存储引擎：管理SQL定义的持久化存储
pub struct DsqlStorage {
    conn: Arc<Mutex<Connection>>,
}

impl DsqlStorage {
    /// 打开存储（文件路径或:memory:）
    pub fn open<P: AsRef<Path>>(path: P) -> DsqlResult<Self> {
        let conn = Connection::open(path.as_ref())
            .map_err(|e| DsqlError::StorageError(format!("open db: {e}")))?;
        // 启用WAL模式提升并发性能
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| DsqlError::StorageError(format!("pragma: {e}")))?;
        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        storage.init_schema()?;
        Ok(storage)
    }

    /// 内存模式（用于测试）
    pub fn open_memory() -> DsqlResult<Self> {
        Self::open(":memory:")
    }

    /// 初始化数据库Schema
    fn init_schema(&self) -> DsqlResult<()> {
        let conn = self.conn.lock();
        let schema = include_str!("../migrations/001_init.sql");
        conn.execute_batch(schema)
            .map_err(|e| DsqlError::StorageError(format!("init schema: {e}")))?;
        let process_schema = include_str!("../migrations/002_process.sql");
        conn.execute_batch(process_schema)
            .map_err(|e| DsqlError::StorageError(format!("init process schema: {e}")))?;
        Ok(())
    }

    /// 获取原始连接（用于执行SQL）
    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    // ==================== SQL定义CRUD ====================

    /// 创建SQL定义
    pub fn create_sql(&self, req: &CreateSqlRequest) -> DsqlResult<SqlDefinition> {
        let param_defs_json = serde_json::to_string(&req.param_defs)
            .map_err(|e| DsqlError::Internal(format!("serialize params: {e}")))?;
        let version_hash = compute_version_hash(&req.sql_template, &param_defs_json);

        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO dsql_definition (
                sql_code, sql_name, description, datasource_code, sql_template,
                param_defs, result_type, operation_type, cache_enabled, cache_ttl,
                permission_code, entity_code, status, version, version_hash, created_by
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,'DRAFT',1,?13,?14)",
            params![
                req.sql_code,
                req.sql_name,
                req.description,
                req.datasource_code,
                req.sql_template,
                param_defs_json,
                format!("{:?}", req.result_type).to_uppercase(),
                format!("{:?}", req.operation_type).to_uppercase(),
                req.cache_enabled.unwrap_or(true),
                req.cache_ttl.unwrap_or(300),
                req.permission_code,
                req.entity_code,
                version_hash,
                req.created_by,
            ],
        ).map_err(|e| DsqlError::StorageError(format!("insert sql: {e}")))?;

        drop(conn);
        self.get_sql(&req.sql_code)?
            .ok_or_else(|| DsqlError::Internal("created sql not found".to_string()))
    }

    /// 获取SQL定义
    pub fn get_sql(&self, sql_code: &str) -> DsqlResult<Option<SqlDefinition>> {
        let conn = self.conn.lock();
        let sql = conn.query_row(
            "SELECT * FROM dsql_definition WHERE sql_code = ?1",
            params![sql_code],
            row_to_sql_definition,
        ).optional()
        .map_err(|e| DsqlError::StorageError(format!("get sql: {e}")))?;
        Ok(sql)
    }

    /// 获取活跃的SQL定义
    pub fn get_active_sql(&self, sql_code: &str) -> DsqlResult<SqlDefinition> {
        let sql = self.get_sql(sql_code)?
            .ok_or_else(|| DsqlError::SqlNotFound(sql_code.to_string()))?;
        if sql.status != SqlStatus::Active {
            return Err(DsqlError::SqlNotActive(
                sql_code.to_string(),
                sql.status.as_str().to_string(),
            ));
        }
        Ok(sql)
    }

    /// 更新SQL定义（自动创建版本历史）
    pub fn update_sql(&self, sql_code: &str, req: &UpdateSqlRequest) -> DsqlResult<SqlDefinition> {
        let existing = self.get_sql(sql_code)?
            .ok_or_else(|| DsqlError::SqlNotFound(sql_code.to_string()))?;

        let conn = self.conn.lock();

        // 保存版本历史
        let old_params = serde_json::to_string(&existing.param_defs).unwrap_or_default();
        conn.execute(
            "INSERT INTO dsql_version_history (sql_code, version, sql_template, param_defs, change_note, created_by)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                sql_code,
                existing.version,
                existing.sql_template,
                old_params,
                req.change_note,
                existing.created_by,
            ],
        ).map_err(|e| DsqlError::StorageError(format!("save history: {e}")))?;

        // 构建更新字段
        let new_version = existing.version + 1;
        let sql_template = req.sql_template.clone().unwrap_or(existing.sql_template);
        let param_defs = req.param_defs.clone().unwrap_or(existing.param_defs);
        let param_defs_json = serde_json::to_string(&param_defs).unwrap_or_default();
        let version_hash = compute_version_hash(&sql_template, &param_defs_json);

        conn.execute(
            "UPDATE dsql_definition SET
                sql_name = COALESCE(?1, sql_name),
                description = COALESCE(?2, description),
                datasource_code = COALESCE(?3, datasource_code),
                sql_template = ?4,
                param_defs = ?5,
                result_type = COALESCE(?6, result_type),
                operation_type = COALESCE(?7, operation_type),
                cache_enabled = COALESCE(?8, cache_enabled),
                cache_ttl = COALESCE(?9, cache_ttl),
                permission_code = COALESCE(?10, permission_code),
                entity_code = COALESCE(?11, entity_code),
                status = COALESCE(?12, status),
                version = ?13,
                version_hash = ?14,
                updated_at = CURRENT_TIMESTAMP
             WHERE sql_code = ?15",
            params![
                req.sql_name,
                req.description,
                req.datasource_code,
                sql_template,
                param_defs_json,
                req.result_type.map(|r| format!("{:?}", r).to_uppercase()),
                req.operation_type.map(|o| format!("{:?}", o).to_uppercase()),
                req.cache_enabled,
                req.cache_ttl,
                req.permission_code,
                req.entity_code,
                req.status.map(|s| s.as_str().to_string()),
                new_version,
                version_hash,
                sql_code,
            ],
        ).map_err(|e| DsqlError::StorageError(format!("update sql: {e}")))?;

        drop(conn);
        self.get_sql(sql_code)?
            .ok_or_else(|| DsqlError::Internal("updated sql not found".to_string()))
    }

    /// 删除SQL定义（软删除，标记为DEPRECATED）
    pub fn delete_sql(&self, sql_code: &str) -> DsqlResult<()> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "UPDATE dsql_definition SET status = 'DEPRECATED', updated_at = CURRENT_TIMESTAMP WHERE sql_code = ?1",
            params![sql_code],
        ).map_err(|e| DsqlError::StorageError(format!("delete sql: {e}")))?;
        if affected == 0 {
            return Err(DsqlError::SqlNotFound(sql_code.to_string()));
        }
        Ok(())
    }

    /// 分页查询SQL列表
    pub fn list_sql(&self, query: &PageQuery) -> DsqlResult<PageResult<SqlDefinition>> {
        let conn = self.conn.lock();
        let offset = (query.page - 1).max(0) * query.page_size;

        // 构建WHERE条件
        let mut conditions = Vec::new();
        let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();

        if let Some(kw) = &query.keyword {
            conditions.push("(sql_code LIKE ?1 OR sql_name LIKE ?1 OR description LIKE ?1)");
            params_vec.push(rusqlite::types::Value::Text(format!("%{kw}%")));
        }
        if let Some(status) = &query.status {
            conditions.push("status = ?");
            params_vec.push(rusqlite::types::Value::Text(status.clone()));
        }
        if let Some(entity) = &query.entity_code {
            conditions.push("entity_code = ?");
            params_vec.push(rusqlite::types::Value::Text(entity.clone()));
        }
        if let Some(ds) = &query.datasource_code {
            conditions.push("datasource_code = ?");
            params_vec.push(rusqlite::types::Value::Text(ds.clone()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // 查询总数
        let count_sql = format!("SELECT COUNT(*) FROM dsql_definition {where_clause}");
        let total: i64 = conn.query_row(&count_sql, params_from_iter(params_vec.iter()), |r| r.get(0))
            .map_err(|e| DsqlError::StorageError(format!("count: {e}")))?;

        // 查询分页数据
        let data_sql = format!(
            "SELECT * FROM dsql_definition {where_clause} ORDER BY updated_at DESC LIMIT ? OFFSET ?"
        );
        let mut final_params = params_vec.clone();
        final_params.push(rusqlite::types::Value::Integer(query.page_size));
        final_params.push(rusqlite::types::Value::Integer(offset));

        let mut stmt = conn.prepare(&data_sql)
            .map_err(|e| DsqlError::StorageError(format!("prepare list: {e}")))?;
        let items: Vec<SqlDefinition> = stmt.query_map(params_from_iter(final_params.iter()), row_to_sql_definition)
            .map_err(|e| DsqlError::StorageError(format!("query list: {e}")))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(PageResult {
            items,
            total,
            page: query.page,
            page_size: query.page_size,
        })
    }

    // ==================== 数据源管理 ====================

    /// 获取数据源
    pub fn get_datasource(&self, code: &str) -> DsqlResult<Option<Datasource>> {
        let conn = self.conn.lock();
        let ds = conn.query_row(
            "SELECT * FROM dsql_datasource WHERE datasource_code = ?1",
            params![code],
            |r| Ok(Datasource {
                id: r.get(0)?,
                datasource_code: r.get(1)?,
                name: r.get(2)?,
                db_type: r.get(3)?,
                connection_str: r.get(4)?,
                username: r.get(5)?,
                password_enc: r.get(6)?,
                pool_max_size: r.get(7)?,
                pool_min_size: r.get(8)?,
                status: r.get(9)?,
            }),
        ).optional()
        .map_err(|e| DsqlError::StorageError(format!("get ds: {e}")))?;
        Ok(ds)
    }

    // ==================== 审计日志 ====================

    /// 记录审计日志
    pub fn write_audit_log(&self, log: &AuditLog) -> DsqlResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO dsql_audit_log (
                trace_id, sql_code, datasource_code, params, row_count,
                duration_ms, success, error_msg, is_slow, cache_hit
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                log.trace_id,
                log.sql_code,
                log.datasource_code,
                log.params,
                log.row_count,
                log.duration_ms,
                log.success,
                log.error_msg,
                log.is_slow,
                log.cache_hit,
            ],
        ).map_err(|e| DsqlError::StorageError(format!("audit log: {e}")))?;
        Ok(())
    }

    /// 记录动态流程执行审计。
    pub fn write_process_audit(
        &self,
        trace_id: Option<&str>,
        process_code: &str,
        success: bool,
        duration_ms: u64,
        step_results: &str,
        error_msg: Option<&str>,
    ) -> DsqlResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO dsql_process_audit
             (trace_id, process_code, status, duration_ms, step_results, error_msg)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                trace_id,
                process_code,
                if success { "SUCCEEDED" } else { "FAILED" },
                duration_ms as i64,
                step_results,
                error_msg,
            ],
        )
        .map_err(|e| DsqlError::StorageError(format!("process audit: {e}")))?;
        Ok(())
    }

    // ==================== 动态流程管理 ====================

    /// 创建动态流程定义。流程步骤以 JSON 保存，便于跨数据库迁移和版本化。
    pub fn create_process(&self, req: &CreateProcessRequest) -> DsqlResult<ProcessDefinition> {
        validate_process_request(req)?;
        let steps = serde_json::to_string(&req.steps)
            .map_err(|e| DsqlError::Internal(format!("serialize process steps: {e}")))?;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO dsql_process_definition (
                process_code, process_name, description, version, status, steps,
                permission_code, entity_code, created_by
            ) VALUES (?1,?2,?3,1,'DRAFT',?4,?5,?6,?7)",
            params![
                req.process_code,
                req.process_name,
                req.description,
                steps,
                req.permission_code,
                req.entity_code,
                req.created_by,
            ],
        )
        .map_err(|e| DsqlError::StorageError(format!("insert process: {e}")))?;
        drop(conn);
        self.get_process(&req.process_code)?
            .ok_or_else(|| DsqlError::Internal("created process not found".to_string()))
    }

    /// 获取流程定义。
    pub fn get_process(&self, process_code: &str) -> DsqlResult<Option<ProcessDefinition>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT * FROM dsql_process_definition WHERE process_code = ?1",
            params![process_code],
            row_to_process_definition,
        )
        .optional()
        .map_err(|e| DsqlError::StorageError(format!("get process: {e}")))
    }

    /// 获取可执行的流程定义。
    pub fn get_active_process(&self, process_code: &str) -> DsqlResult<ProcessDefinition> {
        let process = self
            .get_process(process_code)?
            .ok_or_else(|| DsqlError::Internal(format!("Process not found: {process_code}")))?;
        if process.status != ProcessStatus::Active {
            return Err(DsqlError::SqlNotActive(
                process_code.to_string(),
                process.status.as_str().to_string(),
            ));
        }
        Ok(process)
    }

    /// 发布流程。
    pub fn activate_process(&self, process_code: &str) -> DsqlResult<ProcessDefinition> {
        let process = self
            .get_process(process_code)?
            .ok_or_else(|| DsqlError::Internal(format!("Process not found: {process_code}")))?;
        validate_process_steps(&process.steps)?;
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE dsql_process_definition
             SET status = 'ACTIVE', version = version + 1, updated_at = CURRENT_TIMESTAMP
             WHERE process_code = ?1",
            params![process_code],
        )
        .map_err(|e| DsqlError::StorageError(format!("activate process: {e}")))?;
        drop(conn);
        self.get_process(process_code)?
            .ok_or_else(|| DsqlError::Internal("activated process not found".to_string()))
    }
}

/// 行映射：数据库行 → SqlDefinition
fn row_to_sql_definition(r: &rusqlite::Row) -> rusqlite::Result<SqlDefinition> {
    let param_defs_str: String = r.get(6)?;
    let param_defs: Vec<ParamDef> = serde_json::from_str(&param_defs_str).unwrap_or_default();

    Ok(SqlDefinition {
        id: r.get(0)?,
        sql_code: r.get(1)?,
        sql_name: r.get(2)?,
        description: r.get(3)?,
        datasource_code: r.get(4)?,
        sql_template: r.get(5)?,
        param_defs,
        result_type: match r.get::<_, String>(7)?.as_str() {
            "MAP" => ResultType::Map,
            "SINGLE" => ResultType::Single,
            "COUNT" => ResultType::Count,
            "UPDATE" => ResultType::Update,
            _ => ResultType::List,
        },
        operation_type: if r.get::<_, String>(8)?.as_str() == "WRITE" {
            OperationType::Write
        } else {
            OperationType::Read
        },
        cache_enabled: r.get(9)?,
        cache_ttl: r.get(10)?,
        permission_code: r.get(11)?,
        entity_code: r.get(12)?,
        status: match r.get::<_, String>(13)?.as_str() {
            "ACTIVE" => SqlStatus::Active,
            "DEPRECATED" => SqlStatus::Deprecated,
            _ => SqlStatus::Draft,
        },
        version: r.get(14)?,
        version_hash: r.get(15)?,
        created_by: r.get(16)?,
        created_at: r.get(17)?,
        updated_at: r.get(18)?,
    })
}

/// 计算版本哈希
fn compute_version_hash(template: &str, params_json: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(template.as_bytes());
    hasher.update(params_json.as_bytes());
    hex::encode(hasher.finalize())
}

fn row_to_process_definition(r: &rusqlite::Row) -> rusqlite::Result<ProcessDefinition> {
    let steps_json: String = r.get(6)?;
    let steps = serde_json::from_str(&steps_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            6,
            rusqlite::types::Type::Text,
            Box::new(e),
        )
    })?;
    Ok(ProcessDefinition {
        id: r.get(0)?,
        process_code: r.get(1)?,
        process_name: r.get(2)?,
        description: r.get(3)?,
        version: r.get(4)?,
        status: match r.get::<_, String>(5)?.as_str() {
            "ACTIVE" => ProcessStatus::Active,
            "DEPRECATED" => ProcessStatus::Deprecated,
            _ => ProcessStatus::Draft,
        },
        steps,
        permission_code: r.get(7)?,
        entity_code: r.get(8)?,
        created_by: r.get(9)?,
        created_at: r.get(10)?,
        updated_at: r.get(11)?,
    })
}

fn validate_process_request(req: &CreateProcessRequest) -> DsqlResult<()> {
    if req.process_code.trim().is_empty() || req.process_name.trim().is_empty() {
        return Err(DsqlError::InvalidParam(
            "process_code and process_name are required".to_string(),
        ));
    }
    validate_process_steps(&req.steps)
}

fn validate_process_steps(steps: &[ProcessStep]) -> DsqlResult<()> {
    if steps.is_empty() {
        return Err(DsqlError::InvalidParam("process must contain a step".to_string()));
    }
    let mut seen = std::collections::HashSet::new();
    for step in steps {
        if step.step_code.trim().is_empty() || step.sql_code.trim().is_empty() {
            return Err(DsqlError::InvalidParam(
                "step_code and sql_code are required".to_string(),
            ));
        }
        if !seen.insert(&step.step_code) {
            return Err(DsqlError::InvalidParam(format!(
                "duplicate process step: {}",
                step.step_code
            )));
        }
    }
    Ok(())
}
