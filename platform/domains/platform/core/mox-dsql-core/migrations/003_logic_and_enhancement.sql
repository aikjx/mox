-- =============================================================================
-- mox-dsql-core Migration 003: 业务逻辑表 + 索引增强
-- =============================================================================
-- 新增：
--   dsql_logic              业务逻辑定义（WASM/脚本，与SQL模板关联）
--   dsql_logic_version      业务逻辑版本历史
-- 增强：
--   dsql_audit_log          添加性能索引
--   dsql_definition         添加复合索引
-- =============================================================================

-- ── 业务逻辑定义表 ──────────────────────────────────────────────────────────
-- 存储与 SQL 模板关联的业务逻辑代码，支持 WASM 动态加载和脚本执行。
-- 新站开发时，只需在此表中插入逻辑代码，无需重新编译。

CREATE TABLE IF NOT EXISTS dsql_logic (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    logic_code      VARCHAR(128) UNIQUE NOT NULL,          -- 逻辑唯一标识
    logic_name      VARCHAR(256) NOT NULL,                  -- 逻辑名称
    description     TEXT,                                    -- 描述
    sql_code        VARCHAR(128),                            -- 关联的SQL模板（可选）
    logic_type      VARCHAR(32) NOT NULL DEFAULT 'WASM',   -- 逻辑类型: WASM / SCRIPT / BUILTIN
    logic_code_body TEXT NOT NULL,                           -- 逻辑代码体（WASM二进制base64 / 脚本源码）
    entry_point     VARCHAR(128),                            -- 入口函数名
    timeout_ms      INTEGER NOT NULL DEFAULT 5000,          -- 执行超时（毫秒）
    version         INTEGER NOT NULL DEFAULT 1,              -- 当前版本号
    version_hash    VARCHAR(64),                             -- 版本哈希（代码内容SHA256）
    status          VARCHAR(16) NOT NULL DEFAULT 'DRAFT',   -- DRAFT / ACTIVE / DEPRECATED
    permission_code VARCHAR(128),                            -- 权限码
    created_by      VARCHAR(128),
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ── 业务逻辑版本历史表 ──────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS dsql_logic_version (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    logic_code      VARCHAR(128) NOT NULL,
    version         INTEGER NOT NULL,
    logic_type      VARCHAR(32) NOT NULL,
    logic_code_body TEXT NOT NULL,
    entry_point     VARCHAR(128),
    change_note     TEXT,
    created_by      VARCHAR(128),
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(logic_code, version)
);

-- ── 索引增强 ────────────────────────────────────────────────────────────────

-- dsql_definition: 状态+实体+数据源复合索引（列表查询高频）
CREATE INDEX IF NOT EXISTS idx_dsql_definition_status_entity
    ON dsql_definition(status, entity_code, datasource_code);

-- dsql_definition: 更新时间倒序索引（分页排序）
CREATE INDEX IF NOT EXISTS idx_dsql_definition_updated_at
    ON dsql_definition(updated_at DESC);

-- dsql_audit_log: SQL代码+时间范围索引（审计查询高频）
CREATE INDEX IF NOT EXISTS idx_dsql_audit_log_sql_time
    ON dsql_audit_log(sql_code, created_at DESC);

-- dsql_audit_log: 慢查询索引（性能分析）
CREATE INDEX IF NOT EXISTS idx_dsql_audit_log_slow
    ON dsql_audit_log(is_slow, duration_ms DESC);

-- dsql_audit_log: 缓存命中统计索引
CREATE INDEX IF NOT EXISTS idx_dsql_audit_log_cache
    ON dsql_audit_log(cache_hit, created_at DESC);

-- dsql_logic: 状态+类型索引
CREATE INDEX IF NOT EXISTS idx_dsql_logic_status_type
    ON dsql_logic(status, logic_type);

-- dsql_logic: 关联SQL索引
CREATE INDEX IF NOT EXISTS idx_dsql_logic_sql_code
    ON dsql_logic(sql_code);

-- dsql_logic_version: 逻辑代码+版本索引
CREATE INDEX IF NOT EXISTS idx_dsql_logic_version
    ON dsql_logic_version(logic_code, version DESC);
