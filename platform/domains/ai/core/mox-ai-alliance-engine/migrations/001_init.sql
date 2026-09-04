-- =============================================================================
-- MOX 联盟引擎数据库 Schema
-- =============================================================================
-- 版本：1.0.0
-- 数据库：PostgreSQL 15+
-- 说明：联盟引擎任务持久化、事件审计、专家元数据缓存
-- =============================================================================

-- 启用 UUID 扩展
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- =============================================================================
-- 1. 任务表（alliance_tasks）
-- =============================================================================

CREATE TABLE IF NOT EXISTS alliance_tasks (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    trace_id        UUID NOT NULL,
    session_id      VARCHAR(128),
    query           TEXT NOT NULL,
    status          VARCHAR(32) NOT NULL DEFAULT 'pending',
    -- pending / running / completed / failed / cancelled
    current_phase   VARCHAR(32) DEFAULT 'intent',
    -- intent / team / debate / synthesize / gate / learn / done

    -- 输入配置
    team_size       INTEGER NOT NULL DEFAULT 4,
    enable_llm      BOOLEAN NOT NULL DEFAULT false,
    options_json    JSONB DEFAULT '{}'::jsonb,
    context_json    JSONB DEFAULT '{}'::jsonb,

    -- 结果数据（各阶段输出）
    intent_result   JSONB,
    team_result     JSONB,
    debate_result   JSONB,
    synthesis_result JSONB,
    gate_result     JSONB,
    learn_result    JSONB,
    final_result    JSONB,

    -- 质量指标
    consensus       DOUBLE PRECISION,
    gate_score      DOUBLE PRECISION,
    gate_grade      VARCHAR(8),
    passed          BOOLEAN DEFAULT false,

    -- 运行时状态
    degraded        BOOLEAN DEFAULT false,
    degrade_reason  TEXT,
    error_message   TEXT,
    retry_count     INTEGER NOT NULL DEFAULT 0,

    -- 时间戳
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    duration_ms     BIGINT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- 租户隔离（企业级）
    tenant_id       VARCHAR(64) DEFAULT 'default',
    created_by      VARCHAR(64)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_alliance_tasks_trace_id ON alliance_tasks(trace_id);
CREATE INDEX IF NOT EXISTS idx_alliance_tasks_session_id ON alliance_tasks(session_id);
CREATE INDEX IF NOT EXISTS idx_alliance_tasks_status ON alliance_tasks(status);
CREATE INDEX IF NOT EXISTS idx_alliance_tasks_created_at ON alliance_tasks(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_alliance_tasks_tenant ON alliance_tasks(tenant_id);
CREATE INDEX IF NOT EXISTS idx_alliance_tasks_grade ON alliance_tasks(gate_grade);

-- =============================================================================
-- 2. 事件审计表（alliance_events）
-- =============================================================================

CREATE TABLE IF NOT EXISTS alliance_events (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    task_id         UUID NOT NULL REFERENCES alliance_tasks(id) ON DELETE CASCADE,
    trace_id        UUID NOT NULL,
    phase           VARCHAR(32) NOT NULL,
    event_type      VARCHAR(32) NOT NULL,
    -- phase_started / phase_data / progress / complete / error / audit

    payload         JSONB NOT NULL DEFAULT '{}'::jsonb,
    latency_ms      BIGINT NOT NULL DEFAULT 0,

    degraded        BOOLEAN DEFAULT false,
    degrade_reason  TEXT,

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    tenant_id       VARCHAR(64) DEFAULT 'default'
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_alliance_events_task_id ON alliance_events(task_id);
CREATE INDEX IF NOT EXISTS idx_alliance_events_trace_id ON alliance_events(trace_id);
CREATE INDEX IF NOT EXISTS idx_alliance_events_phase ON alliance_events(phase);
CREATE INDEX IF NOT EXISTS idx_alliance_events_created_at ON alliance_events(created_at);

-- =============================================================================
-- 3. 专家元数据表（alliance_experts）
-- =============================================================================

CREATE TABLE IF NOT EXISTS alliance_experts (
    expert_id       VARCHAR(128) PRIMARY KEY,
    dimension       VARCHAR(64) NOT NULL,
    description     TEXT,
    priority        INTEGER NOT NULL DEFAULT 5,
    avg_latency_ms  BIGINT NOT NULL DEFAULT 0,
    gate_a_rate_30d DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_runs      BIGINT NOT NULL DEFAULT 0,
    success_runs    BIGINT NOT NULL DEFAULT 0,
    supported_classes JSONB DEFAULT '[]'::jsonb,
    capabilities    JSONB DEFAULT '[]'::jsonb,
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_alliance_experts_dimension ON alliance_experts(dimension);
CREATE INDEX IF NOT EXISTS idx_alliance_experts_active ON alliance_experts(is_active);
CREATE INDEX IF NOT EXISTS idx_alliance_experts_priority ON alliance_experts(priority DESC);

-- =============================================================================
-- 4. 任务阶段耗时统计表（alliance_phase_stats）
-- =============================================================================

CREATE TABLE IF NOT EXISTS alliance_phase_stats (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    task_id         UUID NOT NULL REFERENCES alliance_tasks(id) ON DELETE CASCADE,
    phase           VARCHAR(32) NOT NULL,
    latency_ms      BIGINT NOT NULL DEFAULT 0,
    success         BOOLEAN NOT NULL DEFAULT true,
    error_message   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_phase_stats_task ON alliance_phase_stats(task_id);
CREATE INDEX IF NOT EXISTS idx_phase_stats_phase ON alliance_phase_stats(phase);

-- =============================================================================
-- 5. 自动更新 updated_at 触发器
-- =============================================================================

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

DROP TRIGGER IF EXISTS update_alliance_tasks_updated_at ON alliance_tasks;
CREATE TRIGGER update_alliance_tasks_updated_at
    BEFORE UPDATE ON alliance_tasks
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_alliance_experts_updated_at ON alliance_experts;
CREATE TRIGGER update_alliance_experts_updated_at
    BEFORE UPDATE ON alliance_experts
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- =============================================================================
-- 6. 初始化专家元数据（默认专家团队）
-- =============================================================================

INSERT INTO alliance_experts (expert_id, dimension, description, priority, supported_classes)
VALUES
    ('security', 'security', '安全架构专家', 9, '["code","architecture","security"]'::jsonb),
    ('performance', 'performance', '性能优化专家', 8, '["code","performance","architecture"]'::jsonb),
    ('architecture', 'architecture', '系统架构专家', 9, '["architecture","design","code"]'::jsonb),
    ('code', 'code', '代码质量专家', 7, '["code","review","testing"]'::jsonb),
    ('data', 'data', '数据架构专家', 7, '["data","database","etl"]'::jsonb),
    ('ai', 'ai', 'AI/ML专家', 8, '["ai","ml","model"]'::jsonb),
    ('product', 'product', '产品设计专家', 6, '["product","ux","requirements"]'::jsonb),
    ('devops', 'devops', 'DevOps专家', 6, '["devops","deployment","infrastructure"]'::jsonb)
ON CONFLICT (expert_id) DO NOTHING;

-- =============================================================================
-- Schema 版本记录
-- =============================================================================

CREATE TABLE IF NOT EXISTS schema_migrations (
    version         VARCHAR(32) PRIMARY KEY,
    description     TEXT,
    applied_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO schema_migrations (version, description)
VALUES ('1.0.0', '联盟引擎初始Schema：任务/事件/专家/阶段统计')
ON CONFLICT (version) DO NOTHING;
