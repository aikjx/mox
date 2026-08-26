-- ============================================================
-- Meta Core DDL (SQLite)
-- 元数据驱动真源：动态实体/字段/视图/工作流/规则/行业包
-- 日期全部用 TEXT 存 ISO8601
-- 所有索引用独立 CREATE INDEX IF NOT EXISTS
-- ============================================================

-- 1. meta_industry_package
CREATE TABLE IF NOT EXISTS meta_industry_package (
  package_id      TEXT PRIMARY KEY,
  package_code    TEXT NOT NULL,
  package_name    TEXT NOT NULL,
  package_version TEXT NOT NULL DEFAULT '1.0.0',
  description     TEXT,
  icon            TEXT,
  banner          TEXT,
  features        TEXT,
  seed_entities   TEXT,
  seed_workflows  TEXT,
  seed_rules      TEXT,
  compliance      TEXT,
  is_official     INTEGER NOT NULL DEFAULT 1,
  status          TEXT NOT NULL DEFAULT 'active',
  metadata        TEXT,
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_meta_industry_code   ON meta_industry_package(package_code);
CREATE INDEX IF NOT EXISTS idx_meta_industry_status ON meta_industry_package(status);

-- 2. meta_tenant_industry
CREATE TABLE IF NOT EXISTS meta_tenant_industry (
  ti_id           TEXT PRIMARY KEY,
  tenant_id       TEXT NOT NULL,
  package_id      TEXT NOT NULL,
  install_version TEXT,
  installed_at    TEXT NOT NULL,
  installed_by    TEXT,
  config          TEXT,
  status          TEXT NOT NULL DEFAULT 'active'
);
CREATE INDEX IF NOT EXISTS idx_meta_ti_tenant  ON meta_tenant_industry(tenant_id);
CREATE INDEX IF NOT EXISTS idx_meta_ti_package ON meta_tenant_industry(package_id);
CREATE INDEX IF NOT EXISTS idx_meta_ti_tp      ON meta_tenant_industry(tenant_id, package_id);

-- 3. meta_entity
CREATE TABLE IF NOT EXISTS meta_entity (
  entity_id        TEXT PRIMARY KEY,
  tenant_id        TEXT NOT NULL,
  entity_code      TEXT NOT NULL,
  entity_name      TEXT NOT NULL,
  entity_plural    TEXT,
  table_name       TEXT,
  description      TEXT,
  icon             TEXT,
  color            TEXT,
  entity_category  TEXT NOT NULL DEFAULT 'master',
  storage_mode     TEXT NOT NULL DEFAULT 'universal',
  shard_key        TEXT,
  history_strategy TEXT NOT NULL DEFAULT 'snapshot',
  extends_entity_id TEXT,
  mixin_ids        TEXT,
  tags             TEXT,
  list_view_id     TEXT,
  form_view_id     TEXT,
  detail_view_id   TEXT,
  workflow_id      TEXT,
  is_system        INTEGER NOT NULL DEFAULT 0,
  status           TEXT NOT NULL DEFAULT 'active',
  metadata         TEXT,
  created_at       TEXT NOT NULL,
  updated_at       TEXT NOT NULL,
  created_by       TEXT,
  updated_by       TEXT,
  deleted_at       TEXT,
  deleted_by       TEXT,
  version          INTEGER NOT NULL DEFAULT 1,
  _hash            TEXT
);
CREATE INDEX IF NOT EXISTS idx_meta_entity_tenant ON meta_entity(tenant_id);
CREATE INDEX IF NOT EXISTS idx_meta_entity_code   ON meta_entity(tenant_id, entity_code);
CREATE INDEX IF NOT EXISTS idx_meta_entity_cat    ON meta_entity(entity_category);

-- 4. meta_field
CREATE TABLE IF NOT EXISTS meta_field (
  field_id         TEXT PRIMARY KEY,
  tenant_id        TEXT NOT NULL,
  entity_id        TEXT NOT NULL,
  field_code       TEXT NOT NULL,
  field_name       TEXT NOT NULL,
  field_type       TEXT NOT NULL,
  is_required      INTEGER NOT NULL DEFAULT 0,
  is_unique        INTEGER NOT NULL DEFAULT 0,
  is_indexed       INTEGER NOT NULL DEFAULT 0,
  is_searchable    INTEGER NOT NULL DEFAULT 0,
  is_sortable      INTEGER NOT NULL DEFAULT 0,
  is_filterable    INTEGER NOT NULL DEFAULT 0,
  is_exportable    INTEGER NOT NULL DEFAULT 1,
  is_importable    INTEGER NOT NULL DEFAULT 1,
  is_readonly      INTEGER NOT NULL DEFAULT 0,
  is_hidden        INTEGER NOT NULL DEFAULT 0,
  is_system        INTEGER NOT NULL DEFAULT 0,
  default_value    TEXT,
  default_expr     TEXT,
  auto_fill_on     TEXT,
  max_length       INTEGER,
  min_value        REAL,
  max_value        REAL,
  decimal_places   INTEGER,
  step             REAL,
  currency_code    TEXT,
  unit             TEXT,
  options_source   TEXT NOT NULL DEFAULT 'inline',
  options_inline   TEXT,
  options_sql      TEXT,
  options_api      TEXT,
  options_dict_code TEXT,
  relation_config  TEXT,
  validations      TEXT,
  formula_expr     TEXT,
  formula_deps     TEXT,
  ui_component     TEXT,
  ui_props         TEXT,
  ui_placeholder   TEXT,
  ui_hint          TEXT,
  ui_group         TEXT,
  ui_sort_order    INTEGER NOT NULL DEFAULT 0,
  ui_span          INTEGER NOT NULL DEFAULT 24,
  ui_newline       INTEGER NOT NULL DEFAULT 0,
  ui_dynamic_cond  TEXT,
  field_permission TEXT,
  storage_slot     TEXT,
  description      TEXT,
  tags             TEXT,
  status           TEXT NOT NULL DEFAULT 'active',
  metadata         TEXT,
  created_at       TEXT NOT NULL,
  updated_at       TEXT NOT NULL,
  created_by       TEXT,
  updated_by       TEXT,
  deleted_at       TEXT,
  deleted_by       TEXT,
  version          INTEGER NOT NULL DEFAULT 1,
  _hash            TEXT
);
CREATE INDEX IF NOT EXISTS idx_meta_field_tenant ON meta_field(tenant_id);
CREATE INDEX IF NOT EXISTS idx_meta_field_entity ON meta_field(entity_id);
CREATE INDEX IF NOT EXISTS idx_meta_field_code   ON meta_field(tenant_id, entity_id, field_code);
CREATE INDEX IF NOT EXISTS idx_meta_field_type   ON meta_field(field_type);
CREATE INDEX IF NOT EXISTS idx_meta_field_slot   ON meta_field(storage_slot);

-- 5. meta_view
CREATE TABLE IF NOT EXISTS meta_view (
  view_id          TEXT PRIMARY KEY,
  tenant_id        TEXT NOT NULL,
  entity_id        TEXT,
  view_code        TEXT NOT NULL,
  view_name        TEXT NOT NULL,
  view_type        TEXT NOT NULL,
  view_mode        TEXT NOT NULL DEFAULT 'default',
  roles_whitelist  TEXT,
  roles_blacklist  TEXT,
  permission_codes TEXT,
  view_config      TEXT NOT NULL,
  filter_presets   TEXT,
  sort_order       INTEGER NOT NULL DEFAULT 0,
  is_default       INTEGER NOT NULL DEFAULT 0,
  description      TEXT,
  status           TEXT NOT NULL DEFAULT 'active',
  metadata         TEXT,
  created_at       TEXT NOT NULL,
  updated_at       TEXT NOT NULL,
  created_by       TEXT,
  updated_by       TEXT,
  deleted_at       TEXT,
  deleted_by       TEXT,
  version          INTEGER NOT NULL DEFAULT 1,
  _hash            TEXT
);
CREATE INDEX IF NOT EXISTS idx_meta_view_tenant ON meta_view(tenant_id);
CREATE INDEX IF NOT EXISTS idx_meta_view_entity ON meta_view(entity_id);
CREATE INDEX IF NOT EXISTS idx_meta_view_code   ON meta_view(tenant_id, view_code);
CREATE INDEX IF NOT EXISTS idx_meta_view_type   ON meta_view(view_type);

-- 6. meta_view_column
CREATE TABLE IF NOT EXISTS meta_view_column (
  vc_id          TEXT PRIMARY KEY,
  tenant_id      TEXT NOT NULL,
  view_id        TEXT NOT NULL,
  field_code     TEXT NOT NULL,
  column_title   TEXT,
  column_width   INTEGER,
  column_fixed   TEXT,
  is_sortable    INTEGER NOT NULL DEFAULT 0,
  is_filterable  INTEGER NOT NULL DEFAULT 0,
  is_visible     INTEGER NOT NULL DEFAULT 1,
  formatter      TEXT,
  component      TEXT,
  align          TEXT,
  sort_order     INTEGER NOT NULL DEFAULT 0,
  created_at     TEXT NOT NULL,
  updated_at     TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_meta_vc_view ON meta_view_column(view_id);

-- 7. meta_workflow
CREATE TABLE IF NOT EXISTS meta_workflow (
  workflow_id       TEXT PRIMARY KEY,
  tenant_id         TEXT NOT NULL,
  workflow_code     TEXT NOT NULL,
  workflow_name     TEXT NOT NULL,
  workflow_category TEXT,
  description       TEXT,
  icon              TEXT,
  entity_id         TEXT,
  trigger_events    TEXT,
  trigger_condition TEXT,
  workflow_version  INTEGER NOT NULL DEFAULT 1,
  version_tag       TEXT,
  is_main_version   INTEGER NOT NULL DEFAULT 1,
  process_def       TEXT NOT NULL,
  notification      TEXT,
  start_roles       TEXT,
  admin_roles       TEXT,
  viewer_roles      TEXT,
  is_draft          INTEGER NOT NULL DEFAULT 1,
  is_suspended      INTEGER NOT NULL DEFAULT 0,
  status            TEXT NOT NULL DEFAULT 'draft',
  metadata          TEXT,
  created_at        TEXT NOT NULL,
  updated_at        TEXT NOT NULL,
  created_by        TEXT,
  updated_by        TEXT,
  deleted_at        TEXT,
  deleted_by        TEXT,
  version           INTEGER NOT NULL DEFAULT 1,
  _hash             TEXT
);
CREATE INDEX IF NOT EXISTS idx_meta_wf_tenant ON meta_workflow(tenant_id);
CREATE INDEX IF NOT EXISTS idx_meta_wf_entity ON meta_workflow(entity_id);
CREATE INDEX IF NOT EXISTS idx_meta_wf_code   ON meta_workflow(tenant_id, workflow_code, workflow_version);

-- 8. meta_workflow_node
CREATE TABLE IF NOT EXISTS meta_workflow_node (
  wfn_id       TEXT PRIMARY KEY,
  tenant_id    TEXT NOT NULL,
  workflow_id  TEXT NOT NULL,
  node_id      TEXT NOT NULL,
  node_type    TEXT NOT NULL,
  node_name    TEXT,
  node_config  TEXT,
  sort_order   INTEGER NOT NULL DEFAULT 0,
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_meta_wfn_workflow ON meta_workflow_node(workflow_id);

-- 9. meta_workflow_transition
CREATE TABLE IF NOT EXISTS meta_workflow_transition (
  wft_id       TEXT PRIMARY KEY,
  tenant_id    TEXT NOT NULL,
  workflow_id  TEXT NOT NULL,
  transition_id TEXT NOT NULL,
  from_node_id TEXT NOT NULL,
  to_node_id   TEXT NOT NULL,
  condition    TEXT,
  label        TEXT,
  sort_order   INTEGER NOT NULL DEFAULT 0,
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_meta_wft_workflow ON meta_workflow_transition(workflow_id);
CREATE INDEX IF NOT EXISTS idx_meta_wft_from     ON meta_workflow_transition(workflow_id, from_node_id);
CREATE INDEX IF NOT EXISTS idx_meta_wft_to       ON meta_workflow_transition(workflow_id, to_node_id);

-- 10. meta_workflow_instance
CREATE TABLE IF NOT EXISTS meta_workflow_instance (
  wfi_id             TEXT PRIMARY KEY,
  tenant_id          TEXT NOT NULL,
  workflow_id        TEXT NOT NULL,
  workflow_version   INTEGER,
  entity_id          TEXT,
  biz_id             TEXT,
  biz_code           TEXT,
  biz_title          TEXT,
  instance_status    TEXT NOT NULL DEFAULT 'running',
  current_node_id    TEXT,
  current_task_ids   TEXT,
  initiator_id       TEXT NOT NULL,
  initiator_dept_id  TEXT,
  admin_user_ids     TEXT,
  cc_user_ids        TEXT,
  started_at         TEXT NOT NULL,
  ended_at           TEXT,
  due_at             TEXT,
  suspended_at       TEXT,
  last_active_at     TEXT,
  total_duration_ms  INTEGER,
  form_data          TEXT,
  variables          TEXT,
  context            TEXT,
  final_decision     TEXT,
  final_comment      TEXT,
  completed_count    INTEGER,
  rejected_count     INTEGER,
  metadata           TEXT,
  created_at         TEXT NOT NULL,
  updated_at         TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_meta_wfi_tenant   ON meta_workflow_instance(tenant_id);
CREATE INDEX IF NOT EXISTS idx_meta_wfi_workflow ON meta_workflow_instance(workflow_id);
CREATE INDEX IF NOT EXISTS idx_meta_wfi_status   ON meta_workflow_instance(instance_status);
CREATE INDEX IF NOT EXISTS idx_meta_wfi_biz      ON meta_workflow_instance(entity_id, biz_id);

-- 11. meta_workflow_instance_state
CREATE TABLE IF NOT EXISTS meta_workflow_instance_state (
  wfis_id       TEXT PRIMARY KEY,
  tenant_id     TEXT NOT NULL,
  instance_id   TEXT NOT NULL,
  node_id       TEXT NOT NULL,
  node_status   TEXT NOT NULL DEFAULT 'pending',
  entered_at    TEXT,
  leaved_at     TEXT,
  duration_ms   INTEGER,
  candidate_users TEXT,
  candidate_roles TEXT,
  assignee_user_id TEXT,
  claim_user_id TEXT,
  decision      TEXT,
  decision_comment TEXT,
  decision_data TEXT,
  decision_at   TEXT,
  decision_user_id TEXT,
  metadata      TEXT,
  created_at    TEXT NOT NULL,
  updated_at    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_meta_wfis_instance ON meta_workflow_instance_state(instance_id);
CREATE INDEX IF NOT EXISTS idx_meta_wfis_status   ON meta_workflow_instance_state(node_status);

-- 12. meta_rule
CREATE TABLE IF NOT EXISTS meta_rule (
  rule_id         TEXT PRIMARY KEY,
  tenant_id       TEXT NOT NULL,
  rule_code       TEXT NOT NULL,
  rule_name       TEXT NOT NULL,
  rule_category   TEXT NOT NULL,
  entity_id       TEXT,
  workflow_id     TEXT,
  rule_scope      TEXT NOT NULL DEFAULT 'global',
  scope_config    TEXT,
  trigger_event   TEXT,
  trigger_cron    TEXT,
  trigger_condition TEXT,
  priority        INTEGER NOT NULL DEFAULT 0,
  mutex_group     TEXT,
  rule_body       TEXT NOT NULL,
  failure_policy  TEXT NOT NULL DEFAULT 'log',
  retry_count     INTEGER NOT NULL DEFAULT 0,
  retry_interval  INTEGER NOT NULL DEFAULT 1000,
  rule_version    INTEGER NOT NULL DEFAULT 1,
  is_enabled      INTEGER NOT NULL DEFAULT 1,
  status          TEXT NOT NULL DEFAULT 'active',
  description     TEXT,
  tags            TEXT,
  metadata        TEXT,
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL,
  created_by      TEXT,
  updated_by      TEXT,
  version         INTEGER NOT NULL DEFAULT 1,
  _hash           TEXT
);
CREATE INDEX IF NOT EXISTS idx_meta_rule_tenant   ON meta_rule(tenant_id);
CREATE INDEX IF NOT EXISTS idx_meta_rule_code     ON meta_rule(tenant_id, rule_code, rule_version);
CREATE INDEX IF NOT EXISTS idx_meta_rule_entity   ON meta_rule(entity_id);
CREATE INDEX IF NOT EXISTS idx_meta_rule_category ON meta_rule(rule_category);
CREATE INDEX IF NOT EXISTS idx_meta_rule_event    ON meta_rule(trigger_event);

-- 13. meta_page
CREATE TABLE IF NOT EXISTS meta_page (
  page_id      TEXT PRIMARY KEY,
  tenant_id    TEXT NOT NULL,
  page_code    TEXT NOT NULL,
  page_name    TEXT NOT NULL,
  page_type    TEXT NOT NULL,
  entity_id    TEXT,
  route_path   TEXT,
  layout       TEXT,
  page_config  TEXT NOT NULL,
  description  TEXT,
  is_system    INTEGER NOT NULL DEFAULT 0,
  status       TEXT NOT NULL DEFAULT 'active',
  metadata     TEXT,
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL,
  created_by   TEXT,
  updated_by   TEXT,
  version      INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_meta_page_tenant ON meta_page(tenant_id);
CREATE INDEX IF NOT EXISTS idx_meta_page_code   ON meta_page(tenant_id, page_code);

-- 14. meta_component
CREATE TABLE IF NOT EXISTS meta_component (
  component_id    TEXT PRIMARY KEY,
  tenant_id       TEXT NOT NULL,
  component_code  TEXT NOT NULL,
  component_name  TEXT NOT NULL,
  component_type  TEXT NOT NULL,
  entity_id       TEXT,
  page_id         TEXT,
  component_lib   TEXT,
  component_props TEXT,
  data_source     TEXT,
  event_handlers  TEXT,
  slot_config     TEXT,
  style_config    TEXT,
  responsive_cfg  TEXT,
  sort_order      INTEGER NOT NULL DEFAULT 0,
  is_system       INTEGER NOT NULL DEFAULT 0,
  status          TEXT NOT NULL DEFAULT 'active',
  description     TEXT,
  metadata        TEXT,
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL,
  created_by      TEXT,
  updated_by      TEXT,
  version         INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_meta_component_tenant ON meta_component(tenant_id);
CREATE INDEX IF NOT EXISTS idx_meta_component_code   ON meta_component(tenant_id, component_code);
CREATE INDEX IF NOT EXISTS idx_meta_component_page   ON meta_component(page_id);

-- 15. meta_field_option_dict
CREATE TABLE IF NOT EXISTS meta_field_option_dict (
  dict_id        TEXT PRIMARY KEY,
  tenant_id      TEXT NOT NULL,
  dict_code      TEXT NOT NULL,
  dict_name      TEXT NOT NULL,
  dict_category  TEXT,
  is_system      INTEGER NOT NULL DEFAULT 0,
  description    TEXT,
  status         TEXT NOT NULL DEFAULT 'active',
  metadata       TEXT,
  created_at     TEXT NOT NULL,
  updated_at     TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_meta_dict_tenant ON meta_field_option_dict(tenant_id);
CREATE INDEX IF NOT EXISTS idx_meta_dict_code   ON meta_field_option_dict(tenant_id, dict_code);

-- 16. meta_field_option_dict_item
CREATE TABLE IF NOT EXISTS meta_field_option_dict_item (
  item_id        TEXT PRIMARY KEY,
  tenant_id      TEXT NOT NULL,
  dict_id        TEXT NOT NULL,
  item_value     TEXT NOT NULL,
  item_label     TEXT NOT NULL,
  parent_item_id TEXT,
  sort_order     INTEGER NOT NULL DEFAULT 0,
  item_level     INTEGER NOT NULL DEFAULT 0,
  item_path      TEXT,
  color          TEXT,
  icon           TEXT,
  tag_type       TEXT,
  ext_data       TEXT,
  is_default     INTEGER NOT NULL DEFAULT 0,
  is_disabled    INTEGER NOT NULL DEFAULT 0,
  status         TEXT NOT NULL DEFAULT 'active',
  created_at     TEXT NOT NULL,
  updated_at     TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_meta_dict_item_dict   ON meta_field_option_dict_item(dict_id);
CREATE INDEX IF NOT EXISTS idx_meta_dict_item_parent ON meta_field_option_dict_item(parent_item_id);
CREATE INDEX IF NOT EXISTS idx_meta_dict_item_tidv   ON meta_field_option_dict_item(tenant_id, dict_id, item_value);
