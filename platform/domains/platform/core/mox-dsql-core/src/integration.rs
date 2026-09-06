// =============================================================================
// 企业级集成测试（端到端全链路验证）
// =============================================================================
//
// 验证多个组件协同工作的端到端流程：
// 1. DSQL执行全链路：注册SQL → 建表 → 执行读/写 → 缓存命中 → 审计记录 → 指标收集
// 2. 敏感数据脱敏全链路：含敏感字段的请求 → 执行 → 审计日志脱敏验证
// 3. 动态流程事务补偿全链路：多步骤流程 → 中间步骤失败 → 补偿执行验证
//
// 这些测试验证的是组件间的集成正确性，而非单个组件的单元功能。
// =============================================================================

#[cfg(test)]
mod integration_tests {
    use crate::*;
    use serde_json::json;
    use std::time::Duration;

    /// 测试辅助：创建临时目录和DsqlManager
    fn setup_test_manager() -> (DsqlManager, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let meta_path = dir.path().join("meta.db");
        let exec_path = dir.path().join("exec.db");
        let manager = DsqlManager::open(&meta_path, &exec_path).unwrap();
        // 初始化执行数据库表
        manager
            .execute_ddl("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT, email TEXT, password TEXT)")
            .unwrap();
        (manager, dir)
    }

    /// 测试辅助：激活SQL（create_sql默认创建为DRAFT状态）
    fn activate_sql(manager: &DsqlManager, code: &str) {
        manager
            .storage
            .connection()
            .lock()
            .execute(
                "UPDATE dsql_definition SET status = 'ACTIVE' WHERE sql_code = ?1",
                rusqlite::params![code],
            )
            .unwrap();
    }

    /// 测试辅助：注册一个读SQL
    fn register_read_sql(manager: &DsqlManager, code: &str, template: &str, param_defs: Vec<model::ParamDef>) {
        manager
            .storage
            .create_sql(&model::CreateSqlRequest {
                sql_code: code.to_string(),
                sql_name: format!("{code}_read"),
                description: None,
                datasource_code: "default".to_string(),
                sql_template: template.to_string(),
                param_defs,
                result_type: model::ResultType::List,
                operation_type: model::OperationType::Read,
                cache_enabled: Some(true),
                cache_ttl: Some(60),
                permission_code: None,
                entity_code: None,
                created_by: None,
            })
            .unwrap();
        activate_sql(manager, code);
    }

    /// 测试辅助：注册一个写SQL
    fn register_write_sql(manager: &DsqlManager, code: &str, template: &str, param_defs: Vec<model::ParamDef>) {
        manager
            .storage
            .create_sql(&model::CreateSqlRequest {
                sql_code: code.to_string(),
                sql_name: format!("{code}_write"),
                description: None,
                datasource_code: "default".to_string(),
                sql_template: template.to_string(),
                param_defs,
                result_type: model::ResultType::Update,
                operation_type: model::OperationType::Write,
                cache_enabled: Some(false),
                cache_ttl: None,
                permission_code: None,
                entity_code: None,
                created_by: None,
            })
            .unwrap();
        activate_sql(manager, code);
    }

    /// 测试辅助：创建字符串参数定义
    fn str_param(name: &str) -> model::ParamDef {
        model::ParamDef {
            name: name.to_string(),
            data_type: "STRING".to_string(),
            required: true,
            default_value: None,
            description: None,
            validation: None,
        }
    }

    /// 测试辅助：创建整数参数定义
    fn int_param(name: &str) -> model::ParamDef {
        model::ParamDef {
            name: name.to_string(),
            data_type: "INT".to_string(),
            required: true,
            default_value: None,
            description: None,
            validation: None,
        }
    }

    // =========================================================================
    // 测试1：DSQL执行全链路（注册→执行→缓存→审计→指标）
    // =========================================================================

    #[test]
    fn test_dsql_full_pipeline_read_write_cache() {
        let (manager, _dir) = setup_test_manager();

        // 步骤1：注册写SQL（插入用户）
        register_write_sql(
            &manager,
            "insert_user",
            "INSERT INTO users (name, email, password) VALUES ({{name}}, {{email}}, {{password}})",
            vec![str_param("name"), str_param("email"), str_param("password")],
        );

        // 步骤2：注册读SQL（查询用户）
        register_read_sql(
            &manager,
            "get_user_by_id",
            "SELECT id, name, email FROM users WHERE id = {{id}}",
            vec![int_param("id")],
        );

        // 步骤3：执行写SQL（插入数据）
        let write_result = manager
            .execute(&model::ExecuteRequest {
                sql_code: "insert_user".to_string(),
                params: json!({"name": "Alice", "email": "alice@test.com", "password": "secret123"}),
                trace_id: Some("trace-001".to_string()),
            })
            .unwrap();
        assert!(write_result.success);
        assert_eq!(write_result.row_count, Some(1));

        // 步骤4：执行读SQL（第一次，缓存未命中）
        let read_result1 = manager
            .execute(&model::ExecuteRequest {
                sql_code: "get_user_by_id".to_string(),
                params: json!({"id": 1}),
                trace_id: Some("trace-002".to_string()),
            })
            .unwrap();
        assert!(read_result1.success);
        assert!(!read_result1.cache_hit); // 第一次应该未命中缓存

        // 步骤5：再次执行读SQL（第二次，缓存命中）
        let read_result2 = manager
            .execute(&model::ExecuteRequest {
                sql_code: "get_user_by_id".to_string(),
                params: json!({"id": 1}),
                trace_id: Some("trace-003".to_string()),
            })
            .unwrap();
        assert!(read_result2.success);
        assert!(read_result2.cache_hit); // 第二次应该命中缓存

        // 步骤6：验证指标已收集
        let metrics = manager.gather_metrics();
        assert!(metrics.contains("dsql_execute_total"));
        assert!(metrics.contains("dsql_cache_hits_total"));
        assert!(metrics.contains("dsql_cache_misses_total"));
        // 至少有1次缓存命中（带标签格式）
        assert!(metrics.contains("dsql_cache_hits_total{"));

        // 步骤7：验证审计日志已写入（等待异步写入）
        std::thread::sleep(Duration::from_secs(1));
        let audit_count: i64 = manager
            .storage
            .connection()
            .lock()
            .query_row(
                "SELECT COUNT(*) FROM dsql_audit_log WHERE trace_id IN ('trace-001', 'trace-002', 'trace-003')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        assert!(audit_count >= 3, "审计日志应该至少有3条记录，实际: {audit_count}");
    }

    // =========================================================================
    // 测试2：敏感数据脱敏全链路
    // =========================================================================

    #[test]
    fn test_sensitive_data_masking_full_pipeline() {
        let (manager, _dir) = setup_test_manager();

        // 注册写SQL（包含敏感字段password）
        register_write_sql(
            &manager,
            "insert_user_sensitive",
            "INSERT INTO users (name, email, password) VALUES ({{name}}, {{email}}, {{password}})",
            vec![str_param("name"), str_param("email"), str_param("password")],
        );

        // 执行包含敏感数据的请求
        manager
            .execute(&model::ExecuteRequest {
                sql_code: "insert_user_sensitive".to_string(),
                params: json!({
                    "name": "Bob",
                    "email": "bob@test.com",
                    "password": "SuperSecret123!",
                    "api_key": "sk-1234567890abcdef"
                }),
                trace_id: Some("trace-sensitive-001".to_string()),
            })
            .unwrap();

        // 等待异步审计写入
        std::thread::sleep(Duration::from_secs(1));

        // 验证审计日志中的敏感数据已被脱敏
        let audit_params: String = manager
            .storage
            .connection()
            .lock()
            .query_row(
                "SELECT params FROM dsql_audit_log WHERE trace_id = 'trace-sensitive-001' LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or_default();

        // password不应该以明文出现
        assert!(
            !audit_params.contains("SuperSecret123!"),
            "审计日志中不应该包含明文password: {audit_params}"
        );
        // api_key不应该以明文出现
        assert!(
            !audit_params.contains("sk-1234567890abcdef"),
            "审计日志中不应该包含明文api_key: {audit_params}"
        );
        // 应该包含脱敏标记
        assert!(
            audit_params.contains("***") || audit_params.contains("****"),
            "审计日志中应该包含脱敏标记: {audit_params}"
        );
    }

    // =========================================================================
    // 测试3：连接池健康检查与统计
    // =========================================================================

    #[test]
    fn test_connection_pool_health_and_stats() {
        let (manager, _dir) = setup_test_manager();

        // 获取连接池统计
        let stats = manager.exec_pool.stats();
        assert!(stats.max_size > 0);
        assert!(stats.idle_count > 0, "预创建的空闲连接应该大于0");
        assert_eq!(stats.active_count, 0);

        // 获取一个连接
        let conn = manager.exec_pool.get_default().unwrap();
        let stats2 = manager.exec_pool.stats();
        assert_eq!(stats2.active_count, 1);
        assert_eq!(stats2.idle_count, stats.idle_count - 1);

        // 归还连接
        drop(conn);
        let stats3 = manager.exec_pool.stats();
        assert_eq!(stats3.active_count, 0);
        assert_eq!(stats3.idle_count, stats.idle_count);

        // 验证创建计数
        assert!(stats3.create_count >= stats.idle_count);
    }

    // =========================================================================
    // 测试4：写操作自动失效缓存
    // =========================================================================

    #[test]
    fn test_write_operation_invalidates_cache() {
        let (manager, _dir) = setup_test_manager();

        // 注册读SQL（查询所有用户）
        register_read_sql(
            &manager,
            "list_users",
            "SELECT id, name FROM users ORDER BY id",
            vec![],
        );

        // 注册写SQL（插入用户）
        register_write_sql(
            &manager,
            "add_user",
            "INSERT INTO users (name) VALUES ({{name}})",
            vec![str_param("name")],
        );

        // 第一次读（缓存未命中）
        let r1 = manager
            .execute(&model::ExecuteRequest {
                sql_code: "list_users".to_string(),
                params: json!({}),
                trace_id: None,
            })
            .unwrap();
        assert!(!r1.cache_hit);

        // 第二次读（缓存命中）
        let r2 = manager
            .execute(&model::ExecuteRequest {
                sql_code: "list_users".to_string(),
                params: json!({}),
                trace_id: None,
            })
            .unwrap();
        assert!(r2.cache_hit);

        // 执行写操作
        manager
            .execute(&model::ExecuteRequest {
                sql_code: "add_user".to_string(),
                params: json!({"name": "Charlie"}),
                trace_id: None,
            })
            .unwrap();

        // 写操作后读（缓存应该已失效，未命中）
        let r3 = manager
            .execute(&model::ExecuteRequest {
                sql_code: "list_users".to_string(),
                params: json!({}),
                trace_id: None,
            })
            .unwrap();
        assert!(!r3.cache_hit, "写操作后缓存应该已失效");
    }

    // =========================================================================
    // 测试5：慢查询检测与指标收集
    // =========================================================================

    #[test]
    fn test_slow_query_detection() {
        let (manager, _dir) = setup_test_manager();

        // 注册读SQL
        register_read_sql(
            &manager,
            "slow_query",
            "SELECT 1 as value",
            vec![],
        );

        // 执行多次
        for _ in 0..5 {
            manager
                .execute(&model::ExecuteRequest {
                    sql_code: "slow_query".to_string(),
                    params: json!({}),
                    trace_id: None,
                })
                .unwrap();
        }

        // 手动记录慢查询（模拟慢查询触发）
        manager.metrics().record_slow_query("slow_query");

        // 验证指标中包含慢查询计数器
        let metrics = manager.gather_metrics();
        assert!(metrics.contains("dsql_slow_queries_total"));
        assert!(metrics.contains("dsql_execute_total"));
    }
}
