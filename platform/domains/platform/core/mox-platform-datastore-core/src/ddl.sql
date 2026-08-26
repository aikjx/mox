-- biz_data 万能业务数据表
CREATE TABLE IF NOT EXISTS biz_data (
    biz_id TEXT PRIMARY KEY,
    tenant_id TEXT,
    entity_id TEXT,
    biz_code TEXT,
    biz_type TEXT,
    biz_status TEXT,
    ext_str_01 TEXT,
    ext_str_02 TEXT,
    ext_str_03 TEXT,
    ext_str_04 TEXT,
    ext_str_05 TEXT,
    ext_str_06 TEXT,
    ext_str_07 TEXT,
    ext_str_08 TEXT,
    ext_str_09 TEXT,
    ext_str_10 TEXT,
    ext_str_11 TEXT,
    ext_str_12 TEXT,
    ext_text_01 TEXT,
    ext_text_02 TEXT,
    ext_json_01 TEXT,
    ext_json_02 TEXT,
    ext_json_03 TEXT,
    ext_json_04 TEXT,
    ext_int_01 INTEGER,
    ext_int_02 INTEGER,
    ext_int_03 INTEGER,
    ext_int_04 INTEGER,
    ext_int_05 INTEGER,
    ext_dec_01 REAL,
    ext_dec_02 REAL,
    ext_dec_03 REAL,
    ext_dec_04 REAL,
    ext_dec_05 REAL,
    ext_date_01 TEXT,
    ext_datetime_01 TEXT,
    ext_bool_01 INTEGER,
    ext_bool_02 INTEGER,
    ext_bool_03 INTEGER,
    dynamic_data TEXT,
    creator_user_id TEXT,
    creator_dept_id TEXT,
    owner_user_id TEXT,
    owner_dept_id TEXT,
    collaborator_user_ids TEXT,
    created_at TEXT,
    updated_at TEXT,
    created_by TEXT,
    updated_by TEXT,
    deleted_at TEXT,
    deleted_by TEXT,
    version INTEGER DEFAULT 1,
    tenant_shard_key INTEGER DEFAULT 0,
    region_code TEXT DEFAULT 'cn-north-1',
    trace_id TEXT,
    curr_hash TEXT,
    version_group_id TEXT,
    workflow_instance_id TEXT,
    workflow_status TEXT,
    audit_lock INTEGER DEFAULT 0,
    immutable INTEGER DEFAULT 0,
    snapshot_policy TEXT,
    CONSTRAINT uq_biz_code UNIQUE(tenant_id, entity_id, biz_code, deleted_at)
);

-- 独立索引
CREATE INDEX IF NOT EXISTS idx_biz_tenant_entity ON biz_data(tenant_id, entity_id);
CREATE INDEX IF NOT EXISTS idx_biz_creator ON biz_data(creator_user_id);
CREATE INDEX IF NOT EXISTS idx_biz_owner ON biz_data(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_biz_status ON biz_data(biz_status);
CREATE INDEX IF NOT EXISTS idx_biz_version_group ON biz_data(version_group_id);
CREATE INDEX IF NOT EXISTS idx_biz_deleted ON biz_data(deleted_at);
CREATE INDEX IF NOT EXISTS idx_biz_trace ON biz_data(trace_id);
CREATE INDEX IF NOT EXISTS idx_biz_workflow ON biz_data(workflow_instance_id);
CREATE INDEX IF NOT EXISTS idx_biz_region ON biz_data(region_code);

-- biz_data_version 版本链表
CREATE TABLE IF NOT EXISTS biz_data_version (
    version_id TEXT PRIMARY KEY,
    biz_id TEXT,
    tenant_id TEXT,
    entity_id TEXT,
    version_num INTEGER,
    snapshot_before TEXT,
    snapshot_after TEXT,
    changed_fields TEXT,
    change_note TEXT,
    operation_type TEXT,
    operator_user_id TEXT,
    prev_hash TEXT,
    curr_hash TEXT,
    created_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_ver_biz ON biz_data_version(biz_id);
CREATE INDEX IF NOT EXISTS idx_ver_op ON biz_data_version(operator_user_id);

-- biz_data_relation 关联关系表
CREATE TABLE IF NOT EXISTS biz_data_relation (
    relation_id TEXT PRIMARY KEY,
    tenant_id TEXT,
    entity_id_from TEXT,
    biz_id_from TEXT,
    entity_id_to TEXT,
    biz_id_to TEXT,
    relation_type TEXT,
    relation_strength INTEGER,
    extra_json TEXT,
    created_by TEXT,
    created_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_rel_from ON biz_data_relation(entity_id_from, biz_id_from);
CREATE INDEX IF NOT EXISTS idx_rel_to ON biz_data_relation(entity_id_to, biz_id_to);
