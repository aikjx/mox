-- MOX 专家联盟动态 SQL 注册表
--
-- 执行前提：目标库已经运行 mox-dsql-core/migrations/001_init.sql 和
-- deploy/sql/mox-expert-alliance.sql。所有写入均通过绑定参数执行。

INSERT OR IGNORE INTO dsql_definition
    (sql_code, sql_name, description, datasource_code, sql_template, param_defs,
     result_type, operation_type, cache_enabled, cache_ttl, permission_code,
     entity_code, status, version, version_hash, created_by)
VALUES
('alliance.expert.list', '查询专家', '按租户和领域查询可用专家', 'default',
 'SELECT expert_id, expert_code, display_name, domain, status, capabilities FROM mox_alliance_expert WHERE tenant_id = {{tenant_id}} AND status = ''ACTIVE'' {?if domain?}AND domain = {{domain}}{?endif?} ORDER BY display_name',
 '[{"name":"tenant_id","data_type":"STRING","required":true},{"name":"domain","data_type":"STRING","required":false}]',
 'LIST', 'READ', 1, 60, 'alliance.expert.read', 'alliance', 'ACTIVE', 1, NULL, 'mox');

INSERT OR IGNORE INTO dsql_definition
    (sql_code, sql_name, description, datasource_code, sql_template, param_defs,
     result_type, operation_type, cache_enabled, cache_ttl, permission_code,
     entity_code, status, version, version_hash, created_by)
VALUES
('alliance.task.get', '查询联盟任务', '租户范围内按任务 ID 查询任务', 'default',
 'SELECT task_id, title, requirement, status, priority, input_context, output_context, created_at, updated_at FROM mox_alliance_task WHERE tenant_id = {{tenant_id}} AND task_id = {{task_id}}',
 '[{"name":"tenant_id","data_type":"STRING","required":true},{"name":"task_id","data_type":"STRING","required":true}]',
 'MAP', 'READ', 0, 0, 'alliance.task.read', 'alliance', 'ACTIVE', 1, NULL, 'mox');

INSERT OR IGNORE INTO dsql_definition
    (sql_code, sql_name, description, datasource_code, sql_template, param_defs,
     result_type, operation_type, cache_enabled, cache_ttl, permission_code,
     entity_code, status, version, version_hash, created_by)
VALUES
('alliance.task.create', '创建联盟任务', '幂等创建联盟任务', 'default',
 'INSERT INTO mox_alliance_task (task_id, tenant_id, idempotency_key, title, requirement, trace_id, input_context, created_by) VALUES ({{task_id}}, {{tenant_id}}, {{idempotency_key}}, {{title}}, {{requirement}}, {{trace_id}}, {{input_context}}, {{created_by}})',
 '[{"name":"task_id","data_type":"STRING","required":true},{"name":"tenant_id","data_type":"STRING","required":true},{"name":"idempotency_key","data_type":"STRING","required":true},{"name":"title","data_type":"STRING","required":true},{"name":"requirement","data_type":"STRING","required":true},{"name":"trace_id","data_type":"STRING","required":false},{"name":"input_context","data_type":"STRING","required":false,"default_value":"{}"},{"name":"created_by","data_type":"STRING","required":false}]',
 'UPDATE', 'WRITE', 0, 0, 'alliance.task.create', 'alliance', 'ACTIVE', 1, NULL, 'mox');

INSERT OR IGNORE INTO dsql_definition
    (sql_code, sql_name, description, datasource_code, sql_template, param_defs,
     result_type, operation_type, cache_enabled, cache_ttl, permission_code,
     entity_code, status, version, version_hash, created_by)
VALUES
('alliance.task.transition', '推进联盟任务状态', '按当前版本推进任务状态，避免丢失更新', 'default',
 'UPDATE mox_alliance_task SET status = {{next_status}}, version = version + 1{?if output_context?}, output_context = {{output_context}}{?endif?}{?if finished_at?}, finished_at = CURRENT_TIMESTAMP{?endif?}, updated_at = CURRENT_TIMESTAMP WHERE tenant_id = {{tenant_id}} AND task_id = {{task_id}} AND version = {{expected_version}}',
 '[{"name":"next_status","data_type":"STRING","required":true},{"name":"output_context","data_type":"STRING","required":false},{"name":"finished_at","data_type":"STRING","required":false},{"name":"tenant_id","data_type":"STRING","required":true},{"name":"task_id","data_type":"STRING","required":true},{"name":"expected_version","data_type":"INT","required":true}]',
 'UPDATE', 'WRITE', 0, 0, 'alliance.task.transition', 'alliance', 'ACTIVE', 1, NULL, 'mox');
