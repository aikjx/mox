-- MOX 动态业务流程基线
-- 流程只保存“声明式步骤 + 参数映射”，不允许保存任意可执行代码。
CREATE TABLE IF NOT EXISTS dsql_process_definition (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    process_code    VARCHAR(128) UNIQUE NOT NULL,
    process_name    VARCHAR(256) NOT NULL,
    description     TEXT,
    version         INTEGER NOT NULL DEFAULT 1,
    status          VARCHAR(16) NOT NULL DEFAULT 'DRAFT',
    steps           TEXT NOT NULL,
    permission_code VARCHAR(128),
    entity_code     VARCHAR(128),
    created_by      VARCHAR(64),
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS dsql_process_audit (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    trace_id        VARCHAR(64),
    process_code    VARCHAR(128) NOT NULL,
    status          VARCHAR(16) NOT NULL,
    duration_ms     INTEGER NOT NULL DEFAULT 0,
    step_results    TEXT,
    error_msg       TEXT,
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_dsql_process_status
    ON dsql_process_definition(status);
CREATE INDEX IF NOT EXISTS idx_dsql_process_audit_code
    ON dsql_process_audit(process_code, created_at);
