// ================================================================
// mox-dsql-core: 动态SQL管理核心
// 数据库SQL管理低代码平台：所有SQL定义存储在数据库中，动态配置执行
// 支持模板渲染、参数校验、多级缓存、多数据源、审计日志
// ================================================================

pub mod cache;
pub mod engine;
pub mod error;
pub mod model;
pub mod storage;

pub use cache::DsqlCache;
pub use engine::SqlEngine;
pub use error::{DsqlError, DsqlResult};
pub use model::*;
pub use storage::DsqlStorage;

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// 动态SQL管理系统：高层API，整合存储+引擎+缓存
pub struct DsqlManager {
    storage: Arc<DsqlStorage>,
    cache: Arc<DsqlCache>,
    /// 执行连接（MVP阶段使用SQLite内存/文件，后续扩展多数据源）
    exec_conn: Arc<parking_lot::Mutex<rusqlite::Connection>>,
}

impl DsqlManager {
    /// 创建动态SQL管理系统
    pub fn open<P: AsRef<Path>>(meta_path: P, exec_path: P) -> DsqlResult<Self> {
        let storage = DsqlStorage::open(meta_path)?;
        let exec_conn = rusqlite::Connection::open(exec_path)
            .map_err(|e| DsqlError::StorageError(format!("open exec db: {e}")))?;
        exec_conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| DsqlError::StorageError(format!("exec pragma: {e}")))?;

        Ok(Self {
            storage: Arc::new(storage),
            cache: Arc::new(DsqlCache::default()),
            exec_conn: Arc::new(parking_lot::Mutex::new(exec_conn)),
        })
    }

    /// 内存模式（用于测试）
    pub fn open_memory() -> DsqlResult<Self> {
        let storage = DsqlStorage::open_memory()?;
        let exec_conn = rusqlite::Connection::open_in_memory()
            .map_err(|e| DsqlError::StorageError(format!("open exec: {e}")))?;
        Ok(Self {
            storage: Arc::new(storage),
            cache: Arc::new(DsqlCache::default()),
            exec_conn: Arc::new(parking_lot::Mutex::new(exec_conn)),
        })
    }

    /// 获取存储层引用
    pub fn storage(&self) -> &DsqlStorage {
        &self.storage
    }

    /// 获取缓存引用
    pub fn cache(&self) -> &DsqlCache {
        &self.cache
    }

    // ==================== SQL定义管理 ====================

    /// 创建SQL定义
    pub fn create_sql(&self, req: &CreateSqlRequest) -> DsqlResult<SqlDefinition> {
        self.storage.create_sql(req)
    }

    /// 获取SQL定义
    pub fn get_sql(&self, sql_code: &str) -> DsqlResult<Option<SqlDefinition>> {
        self.storage.get_sql(sql_code)
    }

    /// 更新SQL定义
    pub fn update_sql(&self, sql_code: &str, req: &UpdateSqlRequest) -> DsqlResult<SqlDefinition> {
        let result = self.storage.update_sql(sql_code, req)?;
        // 更新后失效缓存
        self.cache.invalidate_sql(sql_code);
        Ok(result)
    }

    /// 删除SQL定义（软删除）
    pub fn delete_sql(&self, sql_code: &str) -> DsqlResult<()> {
        self.storage.delete_sql(sql_code)?;
        self.cache.invalidate_sql(sql_code);
        Ok(())
    }

    /// 分页查询SQL列表
    pub fn list_sql(&self, query: &PageQuery) -> DsqlResult<PageResult<SqlDefinition>> {
        self.storage.list_sql(query)
    }

    /// 激活SQL（从DRAFT变为ACTIVE）
    pub fn activate_sql(&self, sql_code: &str) -> DsqlResult<SqlDefinition> {
        self.update_sql(sql_code, &UpdateSqlRequest {
            sql_name: None,
            description: None,
            datasource_code: None,
            sql_template: None,
            param_defs: None,
            result_type: None,
            operation_type: None,
            cache_enabled: None,
            cache_ttl: None,
            permission_code: None,
            entity_code: None,
            status: Some(SqlStatus::Active),
            change_note: Some("Activate SQL".to_string()),
        })
    }

    // ==================== SQL执行 ====================

    /// 执行SQL
    pub fn execute(&self, req: &ExecuteRequest) -> DsqlResult<ExecuteResult> {
        let start = Instant::now();
        let sql_def = self.storage.get_active_sql(&req.sql_code)?;

        // 缓存检查（仅读操作且启用缓存）
        if sql_def.operation_type == OperationType::Read && sql_def.cache_enabled {
            let version_hash = sql_def.version_hash.clone().unwrap_or_default();
            let cache_key = DsqlCache::cache_key(&req.sql_code, &version_hash, &req.params);
            if let Some(mut cached) = self.cache.get(&cache_key) {
                cached.trace_id = req.trace_id.clone();
                cached.cache_hit = true;
                // 记录缓存命中审计
                self.write_audit_log(&sql_def, req, &cached, start.elapsed().as_millis() as u64, true)?;
                return Ok(cached);
            }
        }

        // 执行SQL
        let conn = self.exec_conn.lock();
        let mut result = SqlEngine::execute(&conn, &sql_def, &req.params)?;
        result.trace_id = req.trace_id.clone();

        // 写缓存（仅读操作且启用缓存）
        if sql_def.operation_type == OperationType::Read && sql_def.cache_enabled {
            let version_hash = sql_def.version_hash.clone().unwrap_or_default();
            let cache_key = DsqlCache::cache_key(&req.sql_code, &version_hash, &req.params);
            self.cache.set(cache_key, result.clone(), sql_def.cache_ttl);
        }

        // 记录审计日志
        self.write_audit_log(&sql_def, req, &result, start.elapsed().as_millis() as u64, false)?;

        Ok(result)
    }

    /// 执行SQL并返回数据（便捷方法）
    pub fn query(&self, sql_code: &str, params: serde_json::Value) -> DsqlResult<serde_json::Value> {
        let result = self.execute(&ExecuteRequest {
            sql_code: sql_code.to_string(),
            params,
            trace_id: None,
        })?;
        if !result.success {
            return Err(DsqlError::ExecutionError(result.error.unwrap_or_default()));
        }
        Ok(result.data.unwrap_or(serde_json::Value::Null))
    }

    // ==================== 执行连接管理 ====================

    /// 获取执行连接（用于DDL等操作）
    pub fn exec_connection(&self) -> Arc<parking_lot::Mutex<rusqlite::Connection>> {
        self.exec_conn.clone()
    }

    /// 在执行连接上执行DDL
    pub fn execute_ddl(&self, ddl: &str) -> DsqlResult<()> {
        let conn = self.exec_conn.lock();
        conn.execute_batch(ddl)
            .map_err(|e| DsqlError::ExecutionError(format!("ddl: {e}")))?;
        Ok(())
    }

    // ==================== 内部方法 ====================

    fn write_audit_log(
        &self,
        sql_def: &SqlDefinition,
        req: &ExecuteRequest,
        result: &ExecuteResult,
        duration_ms: u64,
        cache_hit: bool,
    ) -> DsqlResult<()> {
        let is_slow = duration_ms > 1000; // 1秒以上为慢SQL
        let log = AuditLog {
            id: 0,
            trace_id: req.trace_id.clone(),
            sql_code: sql_def.sql_code.clone(),
            datasource_code: Some(sql_def.datasource_code.clone()),
            params: Some(req.params.to_string()),
            row_count: result.row_count,
            duration_ms: Some(duration_ms as i64),
            success: result.success,
            error_msg: result.error.clone(),
            is_slow,
            cache_hit,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.storage.write_audit_log(&log)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_manager() -> DsqlResult<DsqlManager> {
        let manager = DsqlManager::open_memory()?;
        // 创建测试表
        manager.execute_ddl("CREATE TABLE IF NOT EXISTS test_users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER, email TEXT)")?;
        manager.execute_ddl("INSERT INTO test_users (name, age, email) VALUES ('Alice', 30, 'alice@test.com'), ('Bob', 25, 'bob@test.com'), ('Charlie', 35, 'charlie@test.com')")?;
        Ok(manager)
    }

    #[test]
    fn test_create_and_execute_sql() {
        let manager = setup_test_manager().unwrap();

        // 创建SQL
        let sql_def = manager.create_sql(&CreateSqlRequest {
            sql_code: "test_query_users".to_string(),
            sql_name: "查询用户".to_string(),
            description: Some("根据年龄查询用户".to_string()),
            datasource_code: "default".to_string(),
            sql_template: "SELECT * FROM test_users WHERE age >= {{min_age}} ORDER BY age".to_string(),
            param_defs: vec![ParamDef {
                name: "min_age".to_string(),
                data_type: "INT".to_string(),
                required: true,
                default_value: None,
                description: Some("最小年龄".to_string()),
                validation: None,
            }],
            result_type: ResultType::List,
            operation_type: OperationType::Read,
            cache_enabled: Some(true),
            cache_ttl: Some(60),
            permission_code: None,
            entity_code: Some("user".to_string()),
            created_by: Some("test".to_string()),
        }).unwrap();

        assert_eq!(sql_def.sql_code, "test_query_users");
        assert_eq!(sql_def.status, SqlStatus::Draft);

        // 激活SQL
        let active = manager.activate_sql("test_query_users").unwrap();
        assert_eq!(active.status, SqlStatus::Active);

        // 执行SQL
        let result = manager.execute(&ExecuteRequest {
            sql_code: "test_query_users".to_string(),
            params: serde_json::json!({ "min_age": 28 }),
            trace_id: Some("test-trace-001".to_string()),
        }).unwrap();

        assert!(result.success);
        assert!(!result.cache_hit); // 第一次不命中缓存
        let data = result.data.unwrap();
        let users = data.as_array().unwrap();
        assert_eq!(users.len(), 2); // Alice(30) + Charlie(35)

        // 第二次执行应命中缓存
        let result2 = manager.execute(&ExecuteRequest {
            sql_code: "test_query_users".to_string(),
            params: serde_json::json!({ "min_age": 28 }),
            trace_id: None,
        }).unwrap();
        assert!(result2.cache_hit);
    }

    #[test]
    fn test_template_if_directive() {
        let manager = setup_test_manager().unwrap();

        manager.execute_ddl("CREATE TABLE IF NOT EXISTS test_orders (id INTEGER PRIMARY KEY, user_id INTEGER, amount REAL, status TEXT)").unwrap();
        manager.execute_ddl("INSERT INTO test_orders (user_id, amount, status) VALUES (1, 100.0, 'PAID'), (1, 50.0, 'PENDING'), (2, 200.0, 'PAID')").unwrap();

        manager.create_sql(&CreateSqlRequest {
            sql_code: "test_query_orders".to_string(),
            sql_name: "查询订单".to_string(),
            description: None,
            datasource_code: "default".to_string(),
            sql_template: "SELECT * FROM test_orders WHERE user_id = {{user_id}} {?if status?}AND status = {{status}}{?endif?}".to_string(),
            param_defs: vec![
                ParamDef { name: "user_id".to_string(), data_type: "INT".to_string(), required: true, default_value: None, description: None, validation: None },
                ParamDef { name: "status".to_string(), data_type: "STRING".to_string(), required: false, default_value: None, description: None, validation: None },
            ],
            result_type: ResultType::List,
            operation_type: OperationType::Read,
            cache_enabled: Some(false),
            cache_ttl: None,
            permission_code: None,
            entity_code: None,
            created_by: None,
        }).unwrap();
        manager.activate_sql("test_query_orders").unwrap();

        // 不带status参数
        let result = manager.execute(&ExecuteRequest {
            sql_code: "test_query_orders".to_string(),
            params: serde_json::json!({ "user_id": 1 }),
            trace_id: None,
        }).unwrap();
        assert!(result.success);
        assert_eq!(result.data.unwrap().as_array().unwrap().len(), 2);

        // 带status参数
        let result = manager.execute(&ExecuteRequest {
            sql_code: "test_query_orders".to_string(),
            params: serde_json::json!({ "user_id": 1, "status": "PAID" }),
            trace_id: None,
        }).unwrap();
        assert!(result.success);
        assert_eq!(result.data.unwrap().as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_write_operation() {
        let manager = setup_test_manager().unwrap();

        manager.create_sql(&CreateSqlRequest {
            sql_code: "test_insert_user".to_string(),
            sql_name: "插入用户".to_string(),
            description: None,
            datasource_code: "default".to_string(),
            sql_template: "INSERT INTO test_users (name, age, email) VALUES ({{name}}, {{age}}, {{email}})".to_string(),
            param_defs: vec![
                ParamDef { name: "name".to_string(), data_type: "STRING".to_string(), required: true, default_value: None, description: None, validation: None },
                ParamDef { name: "age".to_string(), data_type: "INT".to_string(), required: true, default_value: None, description: None, validation: None },
                ParamDef { name: "email".to_string(), data_type: "STRING".to_string(), required: true, default_value: None, description: None, validation: None },
            ],
            result_type: ResultType::Update,
            operation_type: OperationType::Write,
            cache_enabled: Some(false),
            cache_ttl: None,
            permission_code: None,
            entity_code: None,
            created_by: None,
        }).unwrap();
        manager.activate_sql("test_insert_user").unwrap();

        let result = manager.execute(&ExecuteRequest {
            sql_code: "test_insert_user".to_string(),
            params: serde_json::json!({ "name": "Dave", "age": 28, "email": "dave@test.com" }),
            trace_id: None,
        }).unwrap();

        assert!(result.success);
        let data = result.data.unwrap();
        assert_eq!(data["affected_rows"], 1);
    }

    #[test]
    fn test_missing_required_param() {
        let manager = setup_test_manager().unwrap();

        manager.create_sql(&CreateSqlRequest {
            sql_code: "test_missing_param".to_string(),
            sql_name: "测试缺失参数".to_string(),
            description: None,
            datasource_code: "default".to_string(),
            sql_template: "SELECT * FROM test_users WHERE age = {{age}}".to_string(),
            param_defs: vec![ParamDef {
                name: "age".to_string(),
                data_type: "INT".to_string(),
                required: true,
                default_value: None,
                description: None,
                validation: None,
            }],
            result_type: ResultType::List,
            operation_type: OperationType::Read,
            cache_enabled: Some(false),
            cache_ttl: None,
            permission_code: None,
            entity_code: None,
            created_by: None,
        }).unwrap();
        manager.activate_sql("test_missing_param").unwrap();

        let result = manager.execute(&ExecuteRequest {
            sql_code: "test_missing_param".to_string(),
            params: serde_json::json!({}),
            trace_id: None,
        });

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DsqlError::MissingParam(_)));
    }

    #[test]
    fn test_list_sql_pagination() {
        let manager = setup_test_manager().unwrap();

        for i in 0..5 {
            manager.create_sql(&CreateSqlRequest {
                sql_code: format!("test_paging_{i}"),
                sql_name: format!("测试分页{i}"),
                description: None,
                datasource_code: "default".to_string(),
                sql_template: "SELECT 1".to_string(),
                param_defs: vec![],
                result_type: ResultType::Single,
                operation_type: OperationType::Read,
                cache_enabled: None,
                cache_ttl: None,
                permission_code: None,
                entity_code: None,
                created_by: None,
            }).unwrap();
        }

        let page1 = manager.list_sql(&PageQuery {
            page: 1,
            page_size: 3,
            keyword: Some("test_paging".to_string()),
            ..Default::default()
        }).unwrap();

        assert_eq!(page1.total, 5);
        assert_eq!(page1.items.len(), 3);
        assert_eq!(page1.page, 1);
        assert_eq!(page1.page_size, 3);

        let page2 = manager.list_sql(&PageQuery {
            page: 2,
            page_size: 3,
            keyword: Some("test_paging".to_string()),
            ..Default::default()
        }).unwrap();

        assert_eq!(page2.items.len(), 2);
    }
}
