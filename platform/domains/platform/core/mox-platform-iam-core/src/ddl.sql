-- ============================================================
-- IAM Core DDL (SQLite)
-- 列名严格对齐 Node 版 schema.js 的用户指定子集
-- 日期全部用 TEXT 存 ISO8601
-- 所有索引用独立 CREATE INDEX IF NOT EXISTS
-- ============================================================

-- 1. iam_tenant
CREATE TABLE IF NOT EXISTS iam_tenant (
  tenant_id     TEXT PRIMARY KEY,
  tenant_code   TEXT NOT NULL,
  tenant_name   TEXT NOT NULL,
  tenant_mode   TEXT NOT NULL DEFAULT 'logical',
  tenant_status TEXT NOT NULL DEFAULT 'trial',
  tenant_plan   TEXT NOT NULL DEFAULT 'free',
  config_json   TEXT,
  settings      TEXT,
  created_at    TEXT NOT NULL,
  updated_at    TEXT NOT NULL,
  version       INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_iam_tenant_code   ON iam_tenant(tenant_code);
CREATE INDEX IF NOT EXISTS idx_iam_tenant_status ON iam_tenant(tenant_status);
CREATE INDEX IF NOT EXISTS idx_iam_tenant_plan   ON iam_tenant(tenant_plan);

-- 2. iam_department
CREATE TABLE IF NOT EXISTS iam_department (
  dept_id         TEXT PRIMARY KEY,
  tenant_id       TEXT NOT NULL,
  parent_id       TEXT,
  dept_code       TEXT NOT NULL,
  dept_name       TEXT NOT NULL,
  dept_type       TEXT NOT NULL DEFAULT 'department',
  dept_level      INTEGER NOT NULL DEFAULT 0,
  dept_path       TEXT NOT NULL,
  sort_order      INTEGER DEFAULT 0,
  manager_user_id TEXT,
  status          TEXT NOT NULL DEFAULT 'active',
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL,
  version         INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_iam_dept_tenant ON iam_department(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_dept_parent ON iam_department(parent_id);
CREATE INDEX IF NOT EXISTS idx_iam_dept_path   ON iam_department(dept_path);
CREATE INDEX IF NOT EXISTS idx_iam_dept_code   ON iam_department(tenant_id, dept_code);

-- 3. iam_user
CREATE TABLE IF NOT EXISTS iam_user (
  user_id       TEXT PRIMARY KEY,
  tenant_id     TEXT NOT NULL,
  user_code     TEXT NOT NULL,
  username      TEXT NOT NULL,
  password_hash TEXT,
  real_name     TEXT,
  nickname      TEXT,
  email         TEXT,
  phone         TEXT,
  avatar        TEXT,
  dept_id       TEXT,
  position      TEXT,
  user_status   TEXT NOT NULL DEFAULT 'invited',
  is_superuser  INTEGER NOT NULL DEFAULT 0,
  last_login_at TEXT,
  last_login_ip TEXT,
  created_at    TEXT NOT NULL,
  updated_at    TEXT NOT NULL,
  version       INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_iam_user_tenant   ON iam_user(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_user_dept     ON iam_user(dept_id);
CREATE INDEX IF NOT EXISTS idx_iam_user_status   ON iam_user(user_status);
CREATE INDEX IF NOT EXISTS idx_iam_user_code     ON iam_user(tenant_id, user_code);
CREATE INDEX IF NOT EXISTS idx_iam_user_username ON iam_user(tenant_id, username);

-- 4. iam_role
CREATE TABLE IF NOT EXISTS iam_role (
  role_id      TEXT PRIMARY KEY,
  tenant_id    TEXT NOT NULL,
  role_code    TEXT NOT NULL,
  role_name    TEXT NOT NULL,
  role_type    TEXT NOT NULL DEFAULT 'custom',
  parent_id    TEXT,
  inherit_path TEXT,
  is_builtin   INTEGER NOT NULL DEFAULT 0,
  data_scope   TEXT NOT NULL DEFAULT 'self',
  description  TEXT,
  sort_order   INTEGER DEFAULT 0,
  status       TEXT NOT NULL DEFAULT 'active',
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL,
  version      INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_iam_role_tenant ON iam_role(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_role_parent ON iam_role(parent_id);
CREATE INDEX IF NOT EXISTS idx_iam_role_type   ON iam_role(role_type);
CREATE INDEX IF NOT EXISTS idx_iam_role_code   ON iam_role(tenant_id, role_code);

-- 5. iam_permission
CREATE TABLE IF NOT EXISTS iam_permission (
  perm_id       TEXT PRIMARY KEY,
  tenant_id     TEXT NOT NULL,
  perm_code     TEXT NOT NULL,
  perm_name     TEXT NOT NULL,
  resource_id   TEXT NOT NULL,
  resource_type TEXT NOT NULL,
  perm_action   TEXT NOT NULL,
  perm_category TEXT,
  description   TEXT,
  sort_order    INTEGER DEFAULT 0,
  status        TEXT NOT NULL DEFAULT 'active',
  created_at    TEXT NOT NULL,
  updated_at    TEXT NOT NULL,
  version       INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_iam_perm_tenant   ON iam_permission(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_perm_resource ON iam_permission(resource_id);
CREATE INDEX IF NOT EXISTS idx_iam_perm_code     ON iam_permission(tenant_id, perm_code);

-- 6. iam_user_role
CREATE TABLE IF NOT EXISTS iam_user_role (
  ur_id       TEXT PRIMARY KEY,
  tenant_id   TEXT NOT NULL,
  user_id     TEXT NOT NULL,
  role_id     TEXT NOT NULL,
  assigned_by TEXT,
  assigned_at TEXT,
  created_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_iam_ur_user ON iam_user_role(user_id);
CREATE INDEX IF NOT EXISTS idx_iam_ur_role ON iam_user_role(role_id);
CREATE INDEX IF NOT EXISTS idx_iam_ur_tenant_user_role ON iam_user_role(tenant_id, user_id, role_id);

-- 7. iam_role_permission
CREATE TABLE IF NOT EXISTS iam_role_permission (
  rp_id      TEXT PRIMARY KEY,
  tenant_id  TEXT NOT NULL,
  role_id    TEXT NOT NULL,
  perm_id    TEXT NOT NULL,
  created_at TEXT NOT NULL,
  created_by TEXT
);
CREATE INDEX IF NOT EXISTS idx_iam_rp_role ON iam_role_permission(role_id);
CREATE INDEX IF NOT EXISTS idx_iam_rp_perm ON iam_role_permission(perm_id);
CREATE INDEX IF NOT EXISTS idx_iam_rp_tenant_role_perm ON iam_role_permission(tenant_id, role_id, perm_id);

-- 8. iam_role_inherit
CREATE TABLE IF NOT EXISTS iam_role_inherit (
  ri_id           TEXT PRIMARY KEY,
  tenant_id       TEXT NOT NULL,
  parent_role_id  TEXT NOT NULL,
  child_role_id   TEXT NOT NULL,
  inherit_level   INTEGER NOT NULL DEFAULT 0,
  created_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_iam_ri_parent ON iam_role_inherit(parent_role_id);
CREATE INDEX IF NOT EXISTS idx_iam_ri_child  ON iam_role_inherit(child_role_id);
CREATE INDEX IF NOT EXISTS idx_iam_ri_tenant ON iam_role_inherit(tenant_id);

-- 9. iam_menu
CREATE TABLE IF NOT EXISTS iam_menu (
  menu_id         TEXT PRIMARY KEY,
  tenant_id       TEXT NOT NULL,
  parent_id       TEXT,
  menu_code       TEXT NOT NULL,
  menu_name       TEXT NOT NULL,
  menu_type       TEXT NOT NULL DEFAULT 'menu',
  menu_category   TEXT,
  route_path      TEXT,
  route_name      TEXT,
  component_path  TEXT,
  icon            TEXT,
  color           TEXT,
  sort_order      INTEGER DEFAULT 0,
  is_visible      INTEGER NOT NULL DEFAULT 1,
  is_cached       INTEGER NOT NULL DEFAULT 0,
  is_external     INTEGER NOT NULL DEFAULT 0,
  link_target     TEXT,
  permission_code TEXT,
  api_scope       TEXT,
  menu_config     TEXT,
  children_json   TEXT,
  status          TEXT NOT NULL DEFAULT 'active',
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL,
  version         INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_iam_menu_tenant ON iam_menu(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_menu_parent ON iam_menu(parent_id);
CREATE INDEX IF NOT EXISTS idx_iam_menu_code   ON iam_menu(tenant_id, menu_code);
CREATE INDEX IF NOT EXISTS idx_iam_menu_type   ON iam_menu(menu_type);

-- 10. iam_user_menu
CREATE TABLE IF NOT EXISTS iam_user_menu (
  um_id       TEXT PRIMARY KEY,
  tenant_id   TEXT NOT NULL,
  user_id     TEXT NOT NULL,
  menu_id     TEXT NOT NULL,
  is_favorite INTEGER NOT NULL DEFAULT 0,
  sort_order  INTEGER DEFAULT 0,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_iam_um_user ON iam_user_menu(user_id);
CREATE INDEX IF NOT EXISTS idx_iam_um_menu ON iam_user_menu(menu_id);

-- 11. iam_role_menu
CREATE TABLE IF NOT EXISTS iam_role_menu (
  rm_id      TEXT PRIMARY KEY,
  tenant_id  TEXT NOT NULL,
  role_id    TEXT NOT NULL,
  menu_id    TEXT NOT NULL,
  created_by TEXT,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_iam_rm_role ON iam_role_menu(role_id);
CREATE INDEX IF NOT EXISTS idx_iam_rm_menu ON iam_role_menu(menu_id);

-- 12. iam_data_permission
CREATE TABLE IF NOT EXISTS iam_data_permission (
  dp_id                        TEXT PRIMARY KEY,
  tenant_id                    TEXT NOT NULL,
  dp_code                      TEXT NOT NULL,
  dp_name                      TEXT NOT NULL,
  subject_type                 TEXT NOT NULL,
  subject_id                   TEXT,
  subject_uuids_json           TEXT,
  resource_code                TEXT NOT NULL,
  scope_type                   TEXT NOT NULL DEFAULT 'self',
  custom_rule_expression_sql   TEXT,
  custom_rule_expression_json  TEXT,
  status                       TEXT NOT NULL DEFAULT 'active',
  created_at                   TEXT NOT NULL,
  created_by                   TEXT,
  updated_at                   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_iam_dp_tenant   ON iam_data_permission(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_dp_code     ON iam_data_permission(tenant_id, dp_code);
CREATE INDEX IF NOT EXISTS idx_iam_dp_subject  ON iam_data_permission(subject_type, subject_id);
CREATE INDEX IF NOT EXISTS idx_iam_dp_resource ON iam_data_permission(resource_code);

-- 13. iam_resource
CREATE TABLE IF NOT EXISTS iam_resource (
  resource_id     TEXT PRIMARY KEY,
  tenant_id       TEXT NOT NULL,
  resource_code   TEXT NOT NULL,
  resource_name   TEXT NOT NULL,
  resource_type   TEXT NOT NULL,
  parent_id       TEXT,
  resource_category TEXT,
  api_methods_sql TEXT,
  api_paths_sql   TEXT,
  description     TEXT,
  sort_order      INTEGER DEFAULT 0,
  status          TEXT NOT NULL DEFAULT 'active',
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL,
  version         INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_iam_resource_tenant ON iam_resource(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_resource_code   ON iam_resource(tenant_id, resource_code);
CREATE INDEX IF NOT EXISTS idx_iam_resource_type   ON iam_resource(resource_type);
CREATE INDEX IF NOT EXISTS idx_iam_resource_parent ON iam_resource(parent_id);

-- 14. iam_tenant_setting
CREATE TABLE IF NOT EXISTS iam_tenant_setting (
  setting_id         TEXT PRIMARY KEY,
  tenant_id          TEXT NOT NULL,
  setting_key        TEXT NOT NULL,
  setting_value      TEXT,
  setting_value_type TEXT NOT NULL DEFAULT 'string',
  description        TEXT,
  updated_by         TEXT,
  updated_at         TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_iam_ts_tenant ON iam_tenant_setting(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_ts_key    ON iam_tenant_setting(tenant_id, setting_key);

-- 15. audit_log (链式哈希真源)
CREATE TABLE IF NOT EXISTS audit_log (
  log_id          TEXT PRIMARY KEY,
  tenant_id       TEXT NOT NULL,
  trace_id        TEXT,
  request_id      TEXT,
  user_id         TEXT,
  user_ip         TEXT,
  action          TEXT NOT NULL,
  action_detail   TEXT,
  resource_type   TEXT,
  resource_id     TEXT,
  resource_code   TEXT,
  biz_id          TEXT,
  biz_code        TEXT,
  status_code     INTEGER,
  http_method     TEXT,
  http_path       TEXT,
  latency_ms      INTEGER,
  snapshot_before TEXT,
  snapshot_after  TEXT,
  changed_fields  TEXT,
  prev_hash       TEXT,
  curr_hash       TEXT NOT NULL,
  created_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_audit_tenant  ON audit_log(tenant_id);
CREATE INDEX IF NOT EXISTS idx_audit_user    ON audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_action  ON audit_log(action);
CREATE INDEX IF NOT EXISTS idx_audit_trace   ON audit_log(trace_id);
CREATE INDEX IF NOT EXISTS idx_audit_request ON audit_log(request_id);
CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_log(created_at);
CREATE INDEX IF NOT EXISTS idx_audit_resource ON audit_log(resource_type, resource_id);

-- ============================================================
-- 系统管理扩展表（16-22）：岗位 / 字典 / 参数 / 操作日志 / 登录日志 / API Key
-- 日期全部用 TEXT 存 ISO8601
-- ============================================================

-- 16. sys_post 岗位
CREATE TABLE IF NOT EXISTS sys_post (
  post_id    TEXT PRIMARY KEY,
  tenant_id  TEXT NOT NULL,
  post_code  TEXT NOT NULL,
  post_name  TEXT NOT NULL,
  dept_id    TEXT,
  sort_order INTEGER DEFAULT 0,
  status     TEXT NOT NULL DEFAULT 'active',
  remark     TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sys_post_tenant ON sys_post(tenant_id);
CREATE INDEX IF NOT EXISTS idx_sys_post_dept   ON sys_post(dept_id);
CREATE INDEX IF NOT EXISTS idx_sys_post_code   ON sys_post(tenant_id, post_code);

-- 17. sys_dict_type 字典类型
CREATE TABLE IF NOT EXISTS sys_dict_type (
  dict_id    TEXT PRIMARY KEY,
  tenant_id  TEXT NOT NULL,
  dict_name  TEXT NOT NULL,
  dict_type  TEXT NOT NULL,
  status     TEXT NOT NULL DEFAULT 'active',
  remark     TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sys_dict_type_tenant ON sys_dict_type(tenant_id);
CREATE INDEX IF NOT EXISTS idx_sys_dict_type_type   ON sys_dict_type(tenant_id, dict_type);

-- 18. sys_dict_data 字典数据
CREATE TABLE IF NOT EXISTS sys_dict_data (
  dict_code  TEXT PRIMARY KEY,
  tenant_id  TEXT NOT NULL,
  dict_sort  INTEGER DEFAULT 0,
  dict_label TEXT NOT NULL,
  dict_value TEXT NOT NULL,
  dict_type  TEXT NOT NULL,
  css_class  TEXT,
  list_class TEXT,
  is_default TEXT DEFAULT 'N',
  status     TEXT NOT NULL DEFAULT 'active',
  remark     TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sys_dict_data_tenant ON sys_dict_data(tenant_id);
CREATE INDEX IF NOT EXISTS idx_sys_dict_data_type   ON sys_dict_data(tenant_id, dict_type);

-- 19. sys_config 参数配置
CREATE TABLE IF NOT EXISTS sys_config (
  config_id    TEXT PRIMARY KEY,
  tenant_id    TEXT NOT NULL,
  config_name  TEXT NOT NULL,
  config_key   TEXT NOT NULL,
  config_value TEXT,
  config_type  TEXT DEFAULT 'string',
  status       TEXT NOT NULL DEFAULT 'active',
  remark       TEXT,
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sys_config_tenant ON sys_config(tenant_id);
CREATE INDEX IF NOT EXISTS idx_sys_config_key    ON sys_config(tenant_id, config_key);

-- 20. sys_oper_log 操作日志
CREATE TABLE IF NOT EXISTS sys_oper_log (
  oper_id        TEXT PRIMARY KEY,
  tenant_id      TEXT NOT NULL,
  title          TEXT,
  business_type  INTEGER DEFAULT 0,
  method         TEXT,
  request_method TEXT,
  operator_type  INTEGER DEFAULT 0,
  oper_name      TEXT,
  dept_name      TEXT,
  oper_url       TEXT,
  oper_ip        TEXT,
  oper_location  TEXT,
  oper_param     TEXT,
  json_result    TEXT,
  status         INTEGER DEFAULT 0,
  error_msg      TEXT,
  oper_time      TEXT NOT NULL,
  cost_time      INTEGER DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_sys_oper_log_tenant ON sys_oper_log(tenant_id);
CREATE INDEX IF NOT EXISTS idx_sys_oper_log_time   ON sys_oper_log(oper_time);

-- 21. sys_logininfor 登录日志
CREATE TABLE IF NOT EXISTS sys_logininfor (
  info_id        TEXT PRIMARY KEY,
  tenant_id      TEXT NOT NULL,
  user_name      TEXT,
  ipaddr         TEXT,
  login_location TEXT,
  browser        TEXT,
  os             TEXT,
  status         TEXT DEFAULT '0',
  msg            TEXT,
  login_time     TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sys_logininfor_tenant ON sys_logininfor(tenant_id);
CREATE INDEX IF NOT EXISTS idx_sys_logininfor_time   ON sys_logininfor(login_time);

-- 22. sys_api_key API 凭证
CREATE TABLE IF NOT EXISTS sys_api_key (
  key_id       TEXT PRIMARY KEY,
  tenant_id    TEXT NOT NULL,
  name         TEXT NOT NULL,
  api_key      TEXT NOT NULL,
  user_id      TEXT,
  scopes       TEXT,
  status       TEXT NOT NULL DEFAULT 'active',
  expires_at   TEXT,
  last_used_at TEXT,
  created_at   TEXT NOT NULL,
  revoked_at   TEXT
);
CREATE INDEX IF NOT EXISTS idx_sys_api_key_tenant ON sys_api_key(tenant_id);
CREATE INDEX IF NOT EXISTS idx_sys_api_key_key    ON sys_api_key(api_key);
CREATE INDEX IF NOT EXISTS idx_sys_api_key_status ON sys_api_key(status);
