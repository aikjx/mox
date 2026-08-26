/**
 * ============================================================
 *  璇玑 RelGraph · 宇宙级企业级 IAM 数据库 Schema 定义
 * ============================================================
 *
 *  架构层级：L7Infrastructure · 企业级身份与访问管理
 *  适用模式：多租户（Logical/Schema/Cluster 三档隔离）
 *  设计原则：
 *    1. RBAC + ABAC + 数据权限 三维鉴权
 *    2. 部门-用户-角色-权限-菜单 全链路闭环
 *    3. 全维可配置：字段、页面、工作流、审批流均元数据驱动
 *    4. 审计全链路不可篡改（SHA-256 哈希链式签名）
 *    5. 软删除 + 版本化 + 数据血缘
 *
 *  命名规范：
 *    - 表名：snake_case，业务域前缀（iam_ / meta_ / biz_ / flow_ / audit_）
 *    - 主键：<table>_id (UUIDv7，时间有序，支持分库分表)
 *    - 租户：所有业务表单列 tenant_id（逻辑隔离模式）
 *    - 审计字段：created_by / updated_by / deleted_by / created_at / updated_at / deleted_at
 *    - 版本字段：version (乐观锁) + _hash (SHA-256 行内容哈希)
 *
 * ============================================================
 */

'use strict';

/**
 * 领域枚举定义 — 与 Rust framework 层严格对齐
 */
const Enums = {
  // 租户隔离模式（对齐 TenantMode）
  TenantMode: { NONE: 'none', LOGICAL: 'logical', SCHEMA: 'schema', CLUSTER: 'cluster' },
  // 租户状态
  TenantStatus: { TRIAL: 'trial', ACTIVE: 'active', SUSPENDED: 'suspended', EXPIRED: 'expired', TERMINATED: 'terminated' },
  // 套餐级别
  TenantPlan: { FREE: 'free', PRO: 'pro', ENTERPRISE: 'enterprise', ULTIMATE: 'ultimate' },
  // 用户状态
  UserStatus: { INVITED: 'invited', ACTIVE: 'active', SUSPENDED: 'suspended', DISABLED: 'disabled', LEFT: 'left' },
  // 认证类型
  AuthType: { PASSWORD: 'password', SSO_SAML: 'sso_saml', SSO_OIDC: 'sso_oidc', LDAP: 'ldap', API_KEY: 'api_key', MFA: 'mfa' },
  // 性别
  Gender: { UNKNOWN: 'unknown', MALE: 'male', FEMALE: 'female', OTHER: 'other' },
  // 部门类型
  DeptType: { COMPANY: 'company', DIVISION: 'division', DEPARTMENT: 'department', GROUP: 'group', TEAM: 'team', VIRTUAL: 'virtual' },
  // 岗位级别
  PositionLevel: { L1: 'L1', L2: 'L2', L3: 'L3', L4: 'L4', L5: 'L5', L6: 'L6', L7: 'L7', L8: 'L8', L9: 'L9', L10: 'L10' },
  // 角色类型
  RoleType: { SYSTEM: 'system', TENANT: 'tenant', CUSTOM: 'custom', BUSINESS: 'business' },
  // 资源类型
  ResourceType: { MENU: 'menu', API: 'api', BUTTON: 'button', COLUMN: 'column', DATA: 'data', FIELD: 'field', WORKFLOW: 'workflow' },
  // 权限动作
  PermissionAction: { VIEW: 'view', CREATE: 'create', EDIT: 'edit', DELETE: 'delete', EXPORT: 'export', IMPORT: 'import', AUDIT: 'audit', APPROVE: 'approve', EXECUTE: 'execute', MANAGE: 'manage' },
  // 数据权限范围
  DataScope: { ALL: 'all', DEPT: 'dept', DEPT_AND_SUB: 'dept_and_sub', SELF: 'self', CUSTOM: 'custom' },
  // 菜单类型
  MenuType: { DIRECTORY: 'directory', MENU: 'menu', BUTTON: 'button', LINK: 'link', IFRAME: 'iframe' },
  // 菜单显示端
  MenuTarget: { PC: 'pc', MOBILE: 'mobile', BOTH: 'both', MINIAPP: 'miniapp' },
  // 审计动作结果
  AuditResult: { SUCCESS: 'success', FAIL: 'fail', TIMEOUT: 'timeout', BLOCKED: 'blocked' },
  // 业务状态通用
  CommonStatus: { DRAFT: 'draft', PENDING: 'pending', APPROVING: 'approving', ACTIVE: 'active', INACTIVE: 'inactive', CANCELLED: 'cancelled', ARCHIVED: 'archived' },
};

/**
 * DDL 定义：企业级 IAM 核心表
 * 顺序严格按照外键依赖：租户 → 部门 → 岗位 → 用户 → 角色 → 权限 → 菜单 → 绑定关系
 */
const DDL = `
-- ============================================================
-- 1. 租户域 (Tenant Domain) —— 多租户隔离的最高边界
-- ============================================================
CREATE TABLE IF NOT EXISTS iam_tenant (
  tenant_id       CHAR(36) PRIMARY KEY,                      -- UUIDv7
  tenant_code     VARCHAR(64)  NOT NULL UNIQUE,              -- 租户编码（唯一，英文短码）
  tenant_name     VARCHAR(255) NOT NULL,                     -- 租户名称
  tenant_mode     VARCHAR(16)  NOT NULL DEFAULT 'logical',   -- 隔离模式: logical/schema/cluster
  tenant_status   VARCHAR(16)  NOT NULL DEFAULT 'trial',     -- 状态
  tenant_plan     VARCHAR(16)  NOT NULL DEFAULT 'free',      -- 套餐
  industry        VARCHAR(64),                               -- 行业分类（用于行业融合引擎）
  region          VARCHAR(64),                               -- 部署区域
  timezone        VARCHAR(32)  DEFAULT 'Asia/Shanghai',
  locale          VARCHAR(16)  DEFAULT 'zh-CN',
  logo_url        VARCHAR(1024),
  contact_name    VARCHAR(64),
  contact_phone   VARCHAR(32),
  contact_email   VARCHAR(128),
  license_key     VARCHAR(255),                              -- License 校验
  license_expire  DATETIME(3),
  trial_expire    DATETIME(3),
  max_users       INTEGER       DEFAULT 10,
  max_storage_gb  INTEGER       DEFAULT 5,
  max_api_per_min INTEGER       DEFAULT 1000,
  custom_schema   VARCHAR(128),                              -- Schema隔离模式下的schema名
  cluster_id      VARCHAR(64),                               -- Cluster隔离模式下的集群ID
  config          JSON,                                      -- 租户级配置（主题、集成、安全策略等）
  metadata        JSON,                                      -- 扩展元数据
  -- 审计字段
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  created_by      CHAR(36),
  updated_by      CHAR(36),
  deleted_at      DATETIME(3),
  deleted_by      CHAR(36),
  version         INTEGER     NOT NULL DEFAULT 1,
  _hash           CHAR(64)                                   -- 行内容 SHA-256
);
CREATE INDEX IF NOT EXISTS idx_iam_tenant_status ON iam_tenant(tenant_status);
CREATE INDEX IF NOT EXISTS idx_iam_tenant_plan   ON iam_tenant(tenant_plan);
CREATE INDEX IF NOT EXISTS idx_iam_tenant_code   ON iam_tenant(tenant_code);

-- ============================================================
-- 2. 部门域 (Department Domain) —— 树形组织架构 + 岗位
-- ============================================================
CREATE TABLE IF NOT EXISTS iam_department (
  dept_id         CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  dept_code       VARCHAR(64)  NOT NULL,
  dept_name       VARCHAR(255) NOT NULL,
  dept_type       VARCHAR(16)  NOT NULL DEFAULT 'department', -- company/division/department/group/team/virtual
  parent_dept_id  CHAR(36),                                   -- 父部门（根=NULL）
  dept_level      INTEGER      NOT NULL DEFAULT 0,            -- 层级（根=0）
  dept_path       VARCHAR(1024) NOT NULL,                     -- 物化路径: /root_id/child_id/grandchild_id
  sort_order      INTEGER      DEFAULT 0,
  leader_id       CHAR(36),                                   -- 部门负责人
  vice_leader_id  CHAR(36),                                   -- 副负责人
  dept_hr_id      CHAR(36),                                   -- HRBP
  description     VARCHAR(1024),
  location        VARCHAR(255),
  area_code       VARCHAR(32),
  status          VARCHAR(16)  NOT NULL DEFAULT 'active',
  metadata        JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  created_by      CHAR(36),
  updated_by      CHAR(36),
  deleted_at      DATETIME(3),
  deleted_by      CHAR(36),
  version         INTEGER     NOT NULL DEFAULT 1,
  _hash           CHAR(64),
  UNIQUE KEY uk_tenant_dept_code (tenant_id, dept_code)
);
CREATE INDEX IF NOT EXISTS idx_iam_dept_tenant   ON iam_department(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_dept_parent   ON iam_department(parent_dept_id);
CREATE INDEX IF NOT EXISTS idx_iam_dept_path     ON iam_department(dept_path(255));
CREATE INDEX IF NOT EXISTS idx_iam_dept_leader   ON iam_department(leader_id);

-- 岗位定义
CREATE TABLE IF NOT EXISTS iam_position (
  position_id     CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  position_code   VARCHAR(64)  NOT NULL,
  position_name   VARCHAR(255) NOT NULL,
  position_level  VARCHAR(8)   DEFAULT 'L3',
  dept_id         CHAR(36)     NOT NULL,
  job_family      VARCHAR(64),                                 -- 职族: 研发/产品/销售/...
  job_category    VARCHAR(64),                                 -- 职类: 前端/后端/算法/...
  report_to_id    CHAR(36),                                    -- 汇报线岗位ID
  sort_order      INTEGER      DEFAULT 0,
  description     VARCHAR(1024),
  requirements    JSON,                                        -- 任职要求
  kpi_template    JSON,                                        -- KPI模板
  status          VARCHAR(16)  NOT NULL DEFAULT 'active',
  metadata        JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  created_by      CHAR(36),
  updated_by      CHAR(36),
  deleted_at      DATETIME(3),
  deleted_by      CHAR(36),
  version         INTEGER     NOT NULL DEFAULT 1,
  _hash           CHAR(64),
  UNIQUE KEY uk_tenant_pos_code (tenant_id, position_code)
);
CREATE INDEX IF NOT EXISTS idx_iam_pos_tenant ON iam_position(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_pos_dept   ON iam_position(dept_id);

-- ============================================================
-- 3. 用户域 (User Domain) —— 用户档案 + 认证凭证 + 组织归属
-- ============================================================
CREATE TABLE IF NOT EXISTS iam_user (
  user_id         CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  user_code       VARCHAR(64)  NOT NULL,                       -- 工号
  username        VARCHAR(64)  NOT NULL,                       -- 登录名
  nickname        VARCHAR(64),
  real_name       VARCHAR(64),
  avatar_url      VARCHAR(1024),
  gender          VARCHAR(8)   DEFAULT 'unknown',
  birthday        DATE,
  mobile          VARCHAR(32),
  email           VARCHAR(128),
  wechat_openid   VARCHAR(64),
  dingtalk_uid    VARCHAR(64),
  id_card_no      VARCHAR(64),
  nationality     VARCHAR(64),
  home_address    VARCHAR(512),
  emergency_contact VARCHAR(64),
  emergency_phone   VARCHAR(32),
  -- 组织归属
  dept_id         CHAR(36)     NOT NULL,                       -- 主部门
  position_id     CHAR(36),                                    -- 主岗位
  leader_id       CHAR(36),                                    -- 直属上级
  entry_date      DATE,                                        -- 入职日期
  regular_date    DATE,                                        -- 转正日期
  leave_date      DATE,                                        -- 离职日期
  probation_months INTEGER     DEFAULT 3,
  work_location   VARCHAR(255),
  work_status     VARCHAR(32)  DEFAULT 'on_job',               -- on_job/business_trip/leave/remote/...
  user_status     VARCHAR(16)  NOT NULL DEFAULT 'invited',
  last_login_at   DATETIME(3),
  last_login_ip   VARCHAR(64),
  last_login_device VARCHAR(255),
  pwd_expire_at   DATETIME(3),                                 -- 密码过期时间
  pwd_changed_at  DATETIME(3),
  pwd_error_count INTEGER     DEFAULT 0,
  locked_until    DATETIME(3),
  -- 配置
  theme           VARCHAR(32)  DEFAULT 'default',
  language        VARCHAR(16)  DEFAULT 'zh-CN',
  timezone        VARCHAR(32)  DEFAULT 'Asia/Shanghai',
  preferences     JSON,                                        -- 用户偏好
  skills          JSON,                                        -- 技能标签
  tags            JSON,
  metadata        JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  created_by      CHAR(36),
  updated_by      CHAR(36),
  deleted_at      DATETIME(3),
  deleted_by      CHAR(36),
  version         INTEGER     NOT NULL DEFAULT 1,
  _hash           CHAR(64),
  UNIQUE KEY uk_tenant_usercode (tenant_id, user_code),
  UNIQUE KEY uk_tenant_username (tenant_id, username),
  KEY idx_iam_user_mobile (mobile),
  KEY idx_iam_user_email  (email)
);
CREATE INDEX IF NOT EXISTS idx_iam_user_tenant   ON iam_user(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_user_dept     ON iam_user(dept_id);
CREATE INDEX IF NOT EXISTS idx_iam_user_leader   ON iam_user(leader_id);
CREATE INDEX IF NOT EXISTS idx_iam_user_status   ON iam_user(user_status);

-- 用户部门归属（一人多部门）
CREATE TABLE IF NOT EXISTS iam_user_dept (
  ud_id           CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  user_id         CHAR(36)     NOT NULL,
  dept_id         CHAR(36)     NOT NULL,
  position_id     CHAR(36),
  is_primary      TINYINT(1)   DEFAULT 0,                      -- 是否主部门
  part_time_ratio DECIMAL(5,2) DEFAULT 100.00,                 -- 投入比例
  start_date      DATE,
  end_date        DATE,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_tenant_user_dept (tenant_id, user_id, dept_id)
);
CREATE INDEX IF NOT EXISTS idx_iam_ud_user ON iam_user_dept(user_id);
CREATE INDEX IF NOT EXISTS idx_iam_ud_dept ON iam_user_dept(dept_id);

-- 用户认证凭证（支持多认证方式）
CREATE TABLE IF NOT EXISTS iam_user_auth (
  auth_id         CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  user_id         CHAR(36)     NOT NULL,
  auth_type       VARCHAR(16)  NOT NULL,                       -- password/sso_saml/sso_oidc/ldap/api_key/mfa
  auth_identifier VARCHAR(255) NOT NULL,                       -- 标识: 用户名/SSO ID/LDAP DN/API Key前缀
  auth_secret     VARCHAR(255),                                -- 密码哈希/Token哈希 (仅存哈希)
  auth_salt       VARCHAR(128),
  auth_config     JSON,                                        -- 额外配置（MFA密钥、SSO配置等）
  last_verified   DATETIME(3),
  expire_at       DATETIME(3),
  status          VARCHAR(16)  NOT NULL DEFAULT 'active',
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_tenant_auth (tenant_id, auth_type, auth_identifier)
);
CREATE INDEX IF NOT EXISTS idx_iam_auth_user ON iam_user_auth(user_id);

-- ============================================================
-- 4. 权限域 (RBAC + ABAC + DataScope)
-- ============================================================

-- 4.1 角色定义（系统角色 + 租户自定义角色 + 业务角色）
CREATE TABLE IF NOT EXISTS iam_role (
  role_id         CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  role_code       VARCHAR(64)  NOT NULL,
  role_name       VARCHAR(255) NOT NULL,
  role_type       VARCHAR(16)  NOT NULL DEFAULT 'custom',      -- system/tenant/custom/business
  parent_role_id  CHAR(36),                                    -- 角色继承（父子角色）
  inherit_level   INTEGER      DEFAULT 0,                      -- 继承深度（防止循环）
  sort_order      INTEGER      DEFAULT 0,
  description     VARCHAR(1024),
  icon            VARCHAR(255),
  color           VARCHAR(16),
  is_builtin      TINYINT(1)   DEFAULT 0,                      -- 是否内置（内置不可删）
  status          VARCHAR(16)  NOT NULL DEFAULT 'active',
  metadata        JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  created_by      CHAR(36),
  updated_by      CHAR(36),
  deleted_at      DATETIME(3),
  deleted_by      CHAR(36),
  version         INTEGER     NOT NULL DEFAULT 1,
  _hash           CHAR(64),
  UNIQUE KEY uk_tenant_role_code (tenant_id, role_code)
);
CREATE INDEX IF NOT EXISTS idx_iam_role_tenant ON iam_role(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_role_parent ON iam_role(parent_role_id);
CREATE INDEX IF NOT EXISTS idx_iam_role_type   ON iam_role(role_type);

-- 4.2 资源定义（菜单/API/按钮/列/字段/数据/工作流）
CREATE TABLE IF NOT EXISTS iam_resource (
  resource_id     CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  resource_code   VARCHAR(128) NOT NULL,
  resource_name   VARCHAR(255) NOT NULL,
  resource_type   VARCHAR(16)  NOT NULL,                       -- menu/api/button/column/data/field/workflow
  resource_module VARCHAR(128),                                -- 所属模块
  resource_path   VARCHAR(512),                                -- 资源路径: menu_id / api_uri / table.column
  http_method     VARCHAR(8),                                  -- API: GET/POST/...
  component_path  VARCHAR(255),                                -- 前端组件路径
  description     VARCHAR(1024),
  is_public       TINYINT(1)   DEFAULT 0,                      -- 是否公开资源（无需鉴权）
  is_critical     TINYINT(1)   DEFAULT 0,                      -- 是否关键资源（操作必审计）
  status          VARCHAR(16)  NOT NULL DEFAULT 'active',
  metadata        JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  version         INTEGER     NOT NULL DEFAULT 1,
  _hash           CHAR(64),
  UNIQUE KEY uk_tenant_res_code (tenant_id, resource_code)
);
CREATE INDEX IF NOT EXISTS idx_iam_res_tenant ON iam_resource(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_res_type   ON iam_resource(resource_type);
CREATE INDEX IF NOT EXISTS idx_iam_res_path   ON iam_resource(resource_path(255));

-- 4.3 权限定义（资源+动作的笛卡尔积）
CREATE TABLE IF NOT EXISTS iam_permission (
  perm_id         CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  perm_code       VARCHAR(256) NOT NULL,                       -- 格式: resource_code:action
  resource_id     CHAR(36)     NOT NULL,
  perm_action     VARCHAR(16)  NOT NULL,                       -- view/create/edit/delete/export/import/audit/approve/execute/manage
  description     VARCHAR(1024),
  sort_order      INTEGER      DEFAULT 0,
  status          VARCHAR(16)  NOT NULL DEFAULT 'active',
  metadata        JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_tenant_perm_code (tenant_id, perm_code)
);
CREATE INDEX IF NOT EXISTS idx_iam_perm_tenant   ON iam_permission(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_perm_resource ON iam_permission(resource_id);

-- 4.4 角色-权限绑定（多对多）
CREATE TABLE IF NOT EXISTS iam_role_permission (
  rp_id           CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  role_id         CHAR(36)     NOT NULL,
  perm_id         CHAR(36)     NOT NULL,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  created_by      CHAR(36),
  UNIQUE KEY uk_tenant_rp (tenant_id, role_id, perm_id)
);
CREATE INDEX IF NOT EXISTS idx_iam_rp_role ON iam_role_permission(role_id);
CREATE INDEX IF NOT EXISTS idx_iam_rp_perm ON iam_role_permission(perm_id);

-- 4.5 用户-角色绑定（多对多，支持按部门/岗位/数据范围生效）
CREATE TABLE IF NOT EXISTS iam_user_role (
  ur_id           CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  user_id         CHAR(36)     NOT NULL,
  role_id         CHAR(36)     NOT NULL,
  -- 生效范围
  scope_type      VARCHAR(16)  DEFAULT 'all',                  -- all/dept/dept_and_sub/self/custom
  scope_dept_ids  JSON,                                        -- custom模式下的部门ID列表
  scope_conditions JSON,                                       -- ABAC条件表达式
  -- 生效时间
  effective_from  DATETIME(3),
  effective_to    DATETIME(3),
  -- 来源
  grant_source    VARCHAR(32)  DEFAULT 'manual',               -- manual/dept_inherit/position_inherit/api/sso
  grant_reason    VARCHAR(255),
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  created_by      CHAR(36),
  UNIQUE KEY uk_tenant_ur (tenant_id, user_id, role_id, scope_type)
);
CREATE INDEX IF NOT EXISTS idx_iam_ur_user ON iam_user_role(user_id);
CREATE INDEX IF NOT EXISTS idx_iam_ur_role ON iam_user_role(role_id);

-- 4.6 数据权限策略（行级过滤）
CREATE TABLE IF NOT EXISTS iam_data_permission (
  dp_id           CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  dp_name         VARCHAR(255) NOT NULL,
  dp_code         VARCHAR(128) NOT NULL,
  target_entity   VARCHAR(128) NOT NULL,                       -- 目标业务表/实体名
  role_id         CHAR(36),                                    -- 绑定角色（NULL=全局策略）
  scope_type      VARCHAR(16)  NOT NULL DEFAULT 'self',        -- all/dept/dept_and_sub/self/custom
  -- 规则表达式（WHERE子句模板，用 {{user_xxx}} 变量注入）
  filter_expr     VARCHAR(2048),                               -- 例: dept_id IN ({{user_dept_path_ids}})
  filter_params   JSON,                                        -- 静态参数
  custom_sql      TEXT,                                        -- 自定义SQL片段
  priority        INTEGER      DEFAULT 0,                      -- 优先级（大的优先）
  status          VARCHAR(16)  NOT NULL DEFAULT 'active',
  metadata        JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  created_by      CHAR(36),
  updated_by      CHAR(36),
  UNIQUE KEY uk_tenant_dp_code (tenant_id, dp_code)
);
CREATE INDEX IF NOT EXISTS idx_iam_dp_tenant  ON iam_data_permission(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_dp_entity  ON iam_data_permission(target_entity);
CREATE INDEX IF NOT EXISTS idx_iam_dp_role    ON iam_data_permission(role_id);

-- ============================================================
-- 5. 菜单域 (Menu Domain) —— 树形菜单 + 权限联动
-- ============================================================
CREATE TABLE IF NOT EXISTS iam_menu (
  menu_id         CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  menu_code       VARCHAR(128) NOT NULL,
  menu_name       VARCHAR(255) NOT NULL,
  menu_type       VARCHAR(16)  NOT NULL DEFAULT 'menu',        -- directory/menu/button/link/iframe
  menu_target     VARCHAR(16)  NOT NULL DEFAULT 'pc',          -- pc/mobile/both/miniapp
  parent_menu_id  CHAR(36),
  menu_level      INTEGER      DEFAULT 0,
  menu_path       VARCHAR(1024) NOT NULL,                      -- 物化路径
  sort_order      INTEGER      DEFAULT 0,
  -- 前端路由
  route_path      VARCHAR(255),                                -- Vue Router path
  route_name      VARCHAR(128),                                -- Vue Router name
  component_path  VARCHAR(512),                                -- 组件文件路径
  redirect_path   VARCHAR(255),                                -- 重定向路径
  -- 显示配置
  icon            VARCHAR(255),
  badge           VARCHAR(64),
  color           VARCHAR(16),
  is_hidden       TINYINT(1)   DEFAULT 0,                      -- 是否隐藏菜单
  is_cached       TINYINT(1)   DEFAULT 1,                      -- 是否缓存页面
  is_affix        TINYINT(1)   DEFAULT 0,                      -- 是否固定标签页
  is_breadcrumb   TINYINT(1)   DEFAULT 1,                      -- 是否显示面包屑
  -- 外链/iframe
  link_url        VARCHAR(1024),
  iframe_src      VARCHAR(1024),
  -- 扩展
  permission_codes JSON,                                       -- 所需权限码列表（AND/OR逻辑）
  roles_whitelist JSON,                                        -- 角色白名单
  roles_blacklist JSON,                                        -- 角色黑名单
  description     VARCHAR(1024),
  status          VARCHAR(16)  NOT NULL DEFAULT 'active',
  metadata        JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  created_by      CHAR(36),
  updated_by      CHAR(36),
  deleted_at      DATETIME(3),
  deleted_by      CHAR(36),
  version         INTEGER     NOT NULL DEFAULT 1,
  _hash           CHAR(64),
  UNIQUE KEY uk_tenant_menu_code (tenant_id, menu_code)
);
CREATE INDEX IF NOT EXISTS idx_iam_menu_tenant ON iam_menu(tenant_id);
CREATE INDEX IF NOT EXISTS idx_iam_menu_parent ON iam_menu(parent_menu_id);
CREATE INDEX IF NOT EXISTS idx_iam_menu_path   ON iam_menu(menu_path(255));
CREATE INDEX IF NOT EXISTS idx_iam_menu_target ON iam_menu(menu_target);
CREATE INDEX IF NOT EXISTS idx_iam_menu_type   ON iam_menu(menu_type);

-- ============================================================
-- 6. 审计域 (Audit Domain) —— 全链路不可篡改审计
-- ============================================================
CREATE TABLE IF NOT EXISTS audit_log (
  audit_id        CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  user_id         CHAR(36),
  username        VARCHAR(64),
  -- 动作
  action_domain   VARCHAR(64)  NOT NULL,                       -- 操作域: iam/meta/biz/flow/system
  action_module   VARCHAR(128) NOT NULL,                       -- 操作模块
  action_name     VARCHAR(128) NOT NULL,                       -- 操作名
  action_desc     VARCHAR(512),
  -- 对象
  target_type     VARCHAR(64),
  target_id       CHAR(36),
  target_name     VARCHAR(255),
  -- 请求信息
  request_id      VARCHAR(64),
  request_method  VARCHAR(16),
  request_uri     VARCHAR(1024),
  request_params  JSON,
  request_body    JSON,
  -- 响应
  result          VARCHAR(16)  NOT NULL DEFAULT 'success',     -- success/fail/timeout/blocked
  result_code     INTEGER,
  result_message  VARCHAR(1024),
  response_time_ms INTEGER,
  -- 定位
  client_ip       VARCHAR(64),
  user_agent      VARCHAR(512),
  location        VARCHAR(255),
  server_node     VARCHAR(128),
  -- 安全上下文
  auth_type       VARCHAR(16),
  session_id      VARCHAR(64),
  trace_id        VARCHAR(64),
  -- 变更快照（用于数据回滚/溯源）
  snapshot_before JSON,
  snapshot_after  JSON,
  changed_fields  JSON,
  -- 链式签名（防止篡改）
  prev_hash       CHAR(64),                                    -- 前一条审计的hash
  curr_hash       CHAR(64),                                    -- 本条的 SHA-256
  -- 时间
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3)
);
CREATE INDEX IF NOT EXISTS idx_audit_tenant    ON audit_log(tenant_id);
CREATE INDEX IF NOT EXISTS idx_audit_user      ON audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_action    ON audit_log(action_domain, action_module);
CREATE INDEX IF NOT EXISTS idx_audit_target    ON audit_log(target_type, target_id);
CREATE INDEX IF NOT EXISTS idx_audit_result    ON audit_log(result);
CREATE INDEX IF NOT EXISTS idx_audit_created   ON audit_log(created_at);
CREATE INDEX IF NOT EXISTS idx_audit_trace     ON audit_log(trace_id);
CREATE INDEX IF NOT EXISTS idx_audit_req       ON audit_log(request_id);
`;

/**
 * 内置数据定义：系统级超级管理员角色与基础权限
 * 与 Rust framework mox-system crate 的 5 角色模型严格对齐
 */
const SeedData = {
  // 系统内置角色（tenant_id=system）
  BuiltinRoles: [
    { role_code: 'sys_admin',   role_name: '超级管理员', role_type: 'system', is_builtin: 1, description: '拥有所有权限，仅限系统运维' },
    { role_code: 'tenant_admin',role_name: '租户管理员', role_type: 'system', is_builtin: 1, description: '租户全功能管理员，可管理租户下所有资源' },
    { role_code: 'coordinator', role_name: '协调者',     role_type: 'business', is_builtin: 1, description: '项目协调者，可分派任务、管理成员' },
    { role_code: 'expert',      role_name: '专家',       role_type: 'business', is_builtin: 1, description: '领域专家，可执行任务、编辑分配给自己的工作' },
    { role_code: 'member',      role_name: '普通成员',   role_type: 'business', is_builtin: 1, description: '基础成员，可查看、评论分配给自己的工作' },
    { role_code: 'auditor',     role_name: '审计员',     role_type: 'system', is_builtin: 1, description: '只读审计权限，可查看所有操作日志与数据' },
  ],
  // 内置权限动作与资源类型映射
  BuiltinPermissionMatrix: {
    // 资源类型 × 动作 = 权限码
    user:     ['view', 'create', 'edit', 'delete', 'import', 'export', 'reset_pwd', 'manage'],
    dept:     ['view', 'create', 'edit', 'delete', 'export', 'manage'],
    role:     ['view', 'create', 'edit', 'delete', 'assign_user', 'assign_perm', 'manage'],
    menu:     ['view', 'create', 'edit', 'delete', 'sort', 'manage'],
    meta:     ['view', 'create', 'edit', 'delete', 'import', 'export', 'manage'],
    workflow: ['view', 'create', 'edit', 'delete', 'start', 'approve', 'manage'],
    audit:    ['view', 'export', 'manage'],
    tenant:   ['view', 'create', 'edit', 'suspend', 'manage'],
  },
};

module.exports = {
  Enums,
  DDL,
  SeedData,
};
