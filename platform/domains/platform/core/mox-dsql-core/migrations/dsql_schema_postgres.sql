-- =============================================================================
-- MOX DSQL 数据库完整 Schema（PostgreSQL 兼容版）
-- =============================================================================
-- 适用于生产环境 PostgreSQL 部署。
-- 包含：SQL模板定义 / 业务逻辑 / 动态流程 / 数据源 / 审计日志
--
-- 执行：psql -U postgres -d mox -f dsql_schema_postgres.sql
-- =============================================================================

-- =============================================================================
-- 1. SQL 模板定义表
-- =============================================================================

CREATE TABLE IF NOT EXISTS dsql_definition (
    id              BIGSERIAL PRIMARY KEY,
    sql_code        VARCHAR(128) UNIQUE NOT NULL,
    sql_name        VARCHAR(256) NOT NULL,
    description     TEXT,
    datasource_code VARCHAR(128) NOT NULL DEFAULT 'default',
    sql_template    TEXT NOT NULL,
    param_defs      JSONB NOT NULL DEFAULT '[]'::jsonb,
    result_type     VARCHAR(16) NOT NULL DEFAULT 'LIST',
    operation_type  VARCHAR(8) NOT NULL DEFAULT 'READ',
    cache_enabled   BOOLEAN NOT NULL DEFAULT true,
    cache_ttl       INTEGER NOT NULL DEFAULT 300,
    permission_code VARCHAR(128),
    entity_code     VARCHAR(128),
    status          VARCHAR(16) NOT NULL DEFAULT 'DRAFT',
    version         INTEGER NOT NULL DEFAULT 1,
    version_hash    VARCHAR(64),
    created_by      VARCHAR(128),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE dsql_definition IS '动态SQL模板定义表';
COMMENT ON COLUMN dsql_definition.sql_template IS 'SQL模板，支持 {{param}} 和 {?if cond?}...{?endif?} 语法';
COMMENT ON COLUMN dsql_definition.param_defs IS '参数定义JSON数组：[{name,data_type,required,default_value,validation}]';
COMMENT ON COLUMN dsql_definition.status IS 'DRAFT / ACTIVE / DEPRECATED';
COMMENT ON COLUMN dsql_definition.version_hash IS '模板+参数的SHA256哈希，用于缓存失效';

-- =============================================================================
-- 2. SQL 版本历史表
-- =============================================================================

CREATE TABLE IF NOT EXISTS dsql_version_history (
    id              BIGSERIAL PRIMARY KEY,
    sql_code        VARCHAR(128) NOT NULL,
    version         INTEGER NOT NULL,
    sql_template    TEXT NOT NULL,
    param_defs      JSONB NOT NULL DEFAULT '[]'::jsonb,
    change_note     TEXT,
    created_by      VARCHAR(128),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(sql_code, version)
);

-- =============================================================================
-- 3. 业务逻辑定义表
-- =============================================================================

CREATE TABLE IF NOT EXISTS dsql_logic (
    id              BIGSERIAL PRIMARY KEY,
    logic_code      VARCHAR(128) UNIQUE NOT NULL,
    logic_name      VARCHAR(256) NOT NULL,
    description     TEXT,
    sql_code        VARCHAR(128),
    logic_type      VARCHAR(32) NOT NULL DEFAULT 'WASM',
    logic_code_body TEXT NOT NULL,
    entry_point     VARCHAR(128),
    timeout_ms      INTEGER NOT NULL DEFAULT 5000,
    version         INTEGER NOT NULL DEFAULT 1,
    version_hash    VARCHAR(64),
    status          VARCHAR(16) NOT NULL DEFAULT 'DRAFT',
    permission_code VARCHAR(128),
    created_by      VARCHAR(128),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE dsql_logic IS '业务逻辑定义表（WASM/脚本，与SQL模板关联，动态加载执行）';
COMMENT ON COLUMN dsql_logic.logic_type IS 'WASM / SCRIPT / BUILTIN';
COMMENT ON COLUMN dsql_logic.logic_code_body IS 'WASM二进制base64 / 脚本源码';

-- =============================================================================
-- 4. 业务逻辑版本历史表
-- =============================================================================

CREATE TABLE IF NOT EXISTS dsql_logic_version (
    id              BIGSERIAL PRIMARY KEY,
    logic_code      VARCHAR(128) NOT NULL,
    version         INTEGER NOT NULL,
    logic_type      VARCHAR(32) NOT NULL,
    logic_code_body TEXT NOT NULL,
    entry_point     VARCHAR(128),
    change_note     TEXT,
    created_by      VARCHAR(128),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(logic_code, version)
);

-- =============================================================================
-- 5. 数据源定义表
-- =============================================================================

CREATE TABLE IF NOT EXISTS dsql_datasource (
    id              BIGSERIAL PRIMARY KEY,
    datasource_code VARCHAR(128) UNIQUE NOT NULL,
    name            VARCHAR(256) NOT NULL,
    db_type         VARCHAR(32) NOT NULL,
    connection_str  TEXT NOT NULL,
    username        VARCHAR(128),
    password_enc    TEXT,
    pool_max_size   INTEGER NOT NULL DEFAULT 10,
    pool_min_size   INTEGER NOT NULL DEFAULT 2,
    status          VARCHAR(16) NOT NULL DEFAULT 'ACTIVE',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- =============================================================================
-- 6. 动态流程定义表
-- =============================================================================

CREATE TABLE IF NOT EXISTS dsql_process_definition (
    id              BIGSERIAL PRIMARY KEY,
    process_code    VARCHAR(128) UNIQUE NOT NULL,
    process_name    VARCHAR(256) NOT NULL,
    description     TEXT,
    version         INTEGER NOT NULL DEFAULT 1,
    status          VARCHAR(16) NOT NULL DEFAULT 'DRAFT',
    steps           JSONB NOT NULL DEFAULT '[]'::jsonb,
    permission_code VARCHAR(128),
    entity_code     VARCHAR(128),
    created_by      VARCHAR(128),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON COLUMN dsql_process_definition.steps IS '流程步骤JSON数组：[{step_code,sql_code,param_mapping,condition,on_error}]';

-- =============================================================================
-- 7. 审计日志表
-- =============================================================================

CREATE TABLE IF NOT EXISTS dsql_audit_log (
    id              BIGSERIAL PRIMARY KEY,
    trace_id        VARCHAR(64),
    sql_code        VARCHAR(128) NOT NULL,
    datasource_code VARCHAR(128),
    params          TEXT,
    row_count       BIGINT,
    duration_ms     BIGINT,
    success         BOOLEAN NOT NULL,
    error_msg       TEXT,
    is_slow         BOOLEAN NOT NULL DEFAULT false,
    cache_hit       BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- =============================================================================
-- 8. 流程审计日志表
-- =============================================================================

CREATE TABLE IF NOT EXISTS dsql_process_audit (
    id              BIGSERIAL PRIMARY KEY,
    trace_id        VARCHAR(64),
    process_code    VARCHAR(128) NOT NULL,
    status          VARCHAR(16) NOT NULL,
    duration_ms     BIGINT,
    step_results    TEXT,
    error_msg       TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- =============================================================================
-- 索引
-- =============================================================================

CREATE INDEX IF NOT EXISTS idx_dsql_definition_status_entity
    ON dsql_definition(status, entity_code, datasource_code);
CREATE INDEX IF NOT EXISTS idx_dsql_definition_updated_at
    ON dsql_definition(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_dsql_definition_datasource
    ON dsql_definition(datasource_code);

CREATE INDEX IF NOT EXISTS idx_dsql_version_history_sql_code
    ON dsql_version_history(sql_code, version DESC);

CREATE INDEX IF NOT EXISTS idx_dsql_logic_status_type
    ON dsql_logic(status, logic_type);
CREATE INDEX IF NOT EXISTS idx_dsql_logic_sql_code
    ON dsql_logic(sql_code);
CREATE INDEX IF NOT EXISTS idx_dsql_logic_version
    ON dsql_logic_version(logic_code, version DESC);

CREATE INDEX IF NOT EXISTS idx_dsql_audit_log_sql_time
    ON dsql_audit_log(sql_code, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_dsql_audit_log_slow
    ON dsql_audit_log(is_slow, duration_ms DESC);
CREATE INDEX IF NOT EXISTS idx_dsql_audit_log_cache
    ON dsql_audit_log(cache_hit, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_dsql_audit_log_trace
    ON dsql_audit_log(trace_id);

CREATE INDEX IF NOT EXISTS idx_dsql_process_audit_process
    ON dsql_process_audit(process_code, created_at DESC);

-- =============================================================================
-- 分区建议（生产环境大数据量）
-- =============================================================================
-- dsql_audit_log 建议按月分区：
-- CREATE TABLE dsql_audit_log_2026_09 PARTITION OF dsql_audit_log
--   FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');
