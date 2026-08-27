-- ================================================================
-- mox-dsql-core 数据库初始化脚本
-- 动态SQL管理系统：所有SQL定义存储在数据库中，动态配置执行
-- ================================================================

-- 数据源配置表
CREATE TABLE IF NOT EXISTS dsql_datasource (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    datasource_code VARCHAR(64) UNIQUE NOT NULL,    -- 数据源唯一编码
    name            VARCHAR(256) NOT NULL,
    db_type         VARCHAR(32) NOT NULL,             -- sqlite/mysql/postgres/oracle/...
    connection_str  TEXT NOT NULL,                     -- 连接字符串
    username        VARCHAR(64),
    password_enc    VARCHAR(512),                      -- 加密存储
    pool_max_size   INTEGER DEFAULT 10,
    pool_min_size   INTEGER DEFAULT 2,
    status          VARCHAR(16) DEFAULT 'ACTIVE',      -- ACTIVE/DISABLED
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- SQL定义主表
CREATE TABLE IF NOT EXISTS dsql_definition (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    sql_code        VARCHAR(128) UNIQUE NOT NULL,      -- SQL唯一编码（业务标识）
    sql_name        VARCHAR(256) NOT NULL,
    description     TEXT,
    datasource_code VARCHAR(64) NOT NULL,               -- 关联数据源
    sql_template    TEXT NOT NULL,                       -- SQL模板（支持{{param}}和{?if?}语法）
    param_defs      TEXT NOT NULL,                       -- JSON数组：参数定义
    result_type     VARCHAR(16) NOT NULL DEFAULT 'LIST',-- MAP/LIST/SINGLE/COUNT/UPDATE
    operation_type  VARCHAR(8)  NOT NULL DEFAULT 'READ',-- READ/WRITE
    cache_enabled   BOOLEAN DEFAULT 1,
    cache_ttl       INTEGER DEFAULT 300,                 -- 秒
    permission_code VARCHAR(128),                         -- 权限点编码
    entity_code     VARCHAR(128),                         -- 关联业务实体
    status          VARCHAR(16) DEFAULT 'DRAFT',         -- DRAFT/ACTIVE/DEPRECATED
    version         INTEGER DEFAULT 1,
    version_hash    CHAR(64),                             -- SHA256(template+params)
    created_by      VARCHAR(64),
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- SQL版本历史表（支持回滚）
CREATE TABLE IF NOT EXISTS dsql_version_history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    sql_code    VARCHAR(128) NOT NULL,
    version     INTEGER NOT NULL,
    sql_template TEXT NOT NULL,
    param_defs  TEXT NOT NULL,
    change_note TEXT,
    created_by  VARCHAR(64),
    created_at  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(sql_code, version)
);

-- 执行审计日志表
CREATE TABLE IF NOT EXISTS dsql_audit_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    trace_id        VARCHAR(64),
    sql_code        VARCHAR(128) NOT NULL,
    datasource_code VARCHAR(64),
    params          TEXT,                                  -- JSON
    row_count       INTEGER,
    duration_ms     INTEGER,
    success         BOOLEAN DEFAULT 1,
    error_msg       TEXT,
    is_slow         BOOLEAN DEFAULT 0,
    cache_hit       BOOLEAN DEFAULT 0,
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_dsql_def_code ON dsql_definition(sql_code);
CREATE INDEX IF NOT EXISTS idx_dsql_def_status ON dsql_definition(status);
CREATE INDEX IF NOT EXISTS idx_dsql_def_entity ON dsql_definition(entity_code);
CREATE INDEX IF NOT EXISTS idx_dsql_audit_code ON dsql_audit_log(sql_code);
CREATE INDEX IF NOT EXISTS idx_dsql_audit_time ON dsql_audit_log(created_at);
CREATE INDEX IF NOT EXISTS idx_dsql_audit_slow ON dsql_audit_log(is_slow);

-- 内置默认数据源（SQLite内存模式，用于测试）
INSERT OR IGNORE INTO dsql_datasource (datasource_code, name, db_type, connection_str, status)
VALUES ('default', '默认SQLite数据源', 'sqlite', ':memory:', 'ACTIVE');
