-- MOX 企业级专家联盟基线（SQLite 3.37+）
--
-- 目标：联盟域只负责业务数据模型；读写行为通过 mox-dsql-core 注册的动态 SQL 执行。
-- 所有业务表都带 tenant_id、版本/状态和审计时间，满足多租户、软删除、幂等和追溯要求。

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS mox_alliance_tenant (
    tenant_id       TEXT PRIMARY KEY,
    tenant_code     TEXT NOT NULL UNIQUE,
    tenant_name     TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'ACTIVE'
                    CHECK (status IN ('ACTIVE', 'SUSPENDED', 'DELETED')),
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS mox_alliance_expert (
    expert_id       TEXT PRIMARY KEY,
    tenant_id       TEXT NOT NULL,
    expert_code     TEXT NOT NULL,
    display_name    TEXT NOT NULL,
    domain          TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'ACTIVE'
                    CHECK (status IN ('ACTIVE', 'PAUSED', 'DRAINING', 'DELETED')),
    endpoint        TEXT,
    capabilities    TEXT NOT NULL DEFAULT '[]',
    metadata        TEXT NOT NULL DEFAULT '{}',
    version         INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at      TEXT,
    FOREIGN KEY (tenant_id) REFERENCES mox_alliance_tenant(tenant_id),
    UNIQUE (tenant_id, expert_code)
);

CREATE TABLE IF NOT EXISTS mox_alliance_task (
    task_id             TEXT PRIMARY KEY,
    tenant_id           TEXT NOT NULL,
    idempotency_key     TEXT NOT NULL,
    title               TEXT NOT NULL,
    requirement         TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'PENDING'
                        CHECK (status IN ('PENDING', 'RUNNING', 'WAITING_HUMAN', 'SUCCEEDED', 'FAILED', 'CANCELLED')),
    priority            INTEGER NOT NULL DEFAULT 50,
    trace_id            TEXT,
    input_context       TEXT NOT NULL DEFAULT '{}',
    output_context      TEXT,
    version             INTEGER NOT NULL DEFAULT 1,
    created_by          TEXT,
    created_at          TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at         TEXT,
    FOREIGN KEY (tenant_id) REFERENCES mox_alliance_tenant(tenant_id),
    UNIQUE (tenant_id, idempotency_key)
);

CREATE TABLE IF NOT EXISTS mox_alliance_assignment (
    assignment_id   TEXT PRIMARY KEY,
    tenant_id       TEXT NOT NULL,
    task_id         TEXT NOT NULL,
    expert_id       TEXT NOT NULL,
    role            TEXT NOT NULL DEFAULT 'CONTRIBUTOR',
    status          TEXT NOT NULL DEFAULT 'ASSIGNED'
                    CHECK (status IN ('ASSIGNED', 'RUNNING', 'SUCCEEDED', 'FAILED', 'CANCELLED')),
    input_context   TEXT NOT NULL DEFAULT '{}',
    output_context  TEXT,
    error_message   TEXT,
    started_at      TEXT,
    finished_at     TEXT,
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES mox_alliance_tenant(tenant_id),
    FOREIGN KEY (task_id) REFERENCES mox_alliance_task(task_id),
    FOREIGN KEY (expert_id) REFERENCES mox_alliance_expert(expert_id)
);

CREATE TABLE IF NOT EXISTS mox_alliance_consensus (
    consensus_id    TEXT PRIMARY KEY,
    tenant_id       TEXT NOT NULL,
    task_id         TEXT NOT NULL,
    decision        TEXT NOT NULL CHECK (decision IN ('APPROVED', 'REJECTED', 'REVIEW')),
    confidence      REAL NOT NULL DEFAULT 0 CHECK (confidence >= 0 AND confidence <= 1),
    conclusion      TEXT NOT NULL,
    evidence        TEXT NOT NULL DEFAULT '[]',
    version         INTEGER NOT NULL DEFAULT 1,
    created_by      TEXT,
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES mox_alliance_tenant(tenant_id),
    FOREIGN KEY (task_id) REFERENCES mox_alliance_task(task_id)
);

-- 追加型事件日志：不更新、不物理删除，支撑重放、审计和问题定位。
CREATE TABLE IF NOT EXISTS mox_alliance_task_event (
    event_id        INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id       TEXT NOT NULL,
    task_id         TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    event_payload   TEXT NOT NULL DEFAULT '{}',
    trace_id        TEXT,
    actor           TEXT,
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES mox_alliance_tenant(tenant_id),
    FOREIGN KEY (task_id) REFERENCES mox_alliance_task(task_id)
);

CREATE INDEX IF NOT EXISTS idx_alliance_expert_tenant_status
    ON mox_alliance_expert(tenant_id, status, domain);
CREATE INDEX IF NOT EXISTS idx_alliance_task_tenant_status
    ON mox_alliance_task(tenant_id, status, priority, created_at);
CREATE INDEX IF NOT EXISTS idx_alliance_assignment_task
    ON mox_alliance_assignment(tenant_id, task_id, status);
CREATE INDEX IF NOT EXISTS idx_alliance_consensus_task
    ON mox_alliance_consensus(tenant_id, task_id, version);
CREATE INDEX IF NOT EXISTS idx_alliance_event_task_time
    ON mox_alliance_task_event(tenant_id, task_id, created_at);
