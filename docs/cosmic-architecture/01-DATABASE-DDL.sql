-- ============================================================
-- MOX 元架构 - 企业级数据库 DDL v1.0
-- 数据库: PostgreSQL 15+ (推荐) / MySQL 8+ / SQLite 3
-- 多租户: L1逻辑隔离 (所有表含 tenant_id)
-- 设计规范: 详见 03-DATABASE-DESIGN-SPEC.md
-- ============================================================
-- 表分组:
--   第一组: 租户与组织 (4表)
--   第二组: 权限体系   (5表)
--   第三组: 元数据引擎 (3表)
--   第四组: 字典       (2表)
--   第五组: 审计日志   (1表)
--   第六组: 流程引擎   (3表)
--   第七组: 专家联盟   (5表)
-- ============================================================

-- ===== 第一组：租户与组织 =====

-- 1. 租户表（璇玑/Mox）—— 多租户隔离的顶层单位
-- 隔离级别: logical(逻辑前缀) / schema(独立Schema) / cluster(独立集群)
-- 套餐: free(免费) / pro(专业) / enterprise(企业)
CREATE TABLE sys_tenant (
    id              VARCHAR(64)  PRIMARY KEY,            -- 租户全局唯一ID，格式 tnt_UUID
    tenant_code     VARCHAR(64)  NOT NULL UNIQUE,        -- 租户业务编码，唯一，用于URL前缀/隔离前缀/子域名
    tenant_name     VARCHAR(255) NOT NULL,               -- 租户显示名称
    isolation_level VARCHAR(16)  DEFAULT 'logical',      -- 隔离级别: logical/schema/cluster
    status          VARCHAR(16)  DEFAULT 'active',       -- 状态: active(活跃)/suspended(暂停)/expired(过期)
    plan            VARCHAR(32)  DEFAULT 'pro',          -- 套餐: free/pro/enterprise
    expire_at       TIMESTAMP,                            -- 套餐过期时间，NULL=永不过期
    config          JSONB,                                -- 租户级配置: 主题/语言/时区/功能开关/自定义品牌
    created_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,  -- 创建时间(UTC)
    updated_at      TIMESTAMP,                            -- 更新时间(UTC)
    deleted_at      TIMESTAMP                             -- 软删除时间，NULL=未删除
);
COMMENT ON TABLE  sys_tenant IS '租户表（璇玑/Mox）—— 多租户隔离的顶层单位，每个租户数据完全隔离';
COMMENT ON COLUMN sys_tenant.id IS '租户全局唯一ID，格式 tnt_UUID';
COMMENT ON COLUMN sys_tenant.tenant_code IS '租户业务编码，唯一，用于URL前缀/隔离前缀/子域名';
COMMENT ON COLUMN sys_tenant.isolation_level IS '隔离级别: logical(逻辑前缀)/schema(独立Schema)/cluster(独立集群)';
COMMENT ON COLUMN sys_tenant.config IS '租户级配置JSON: 主题/语言/时区/功能开关/自定义品牌';
CREATE INDEX idx_tenant_status ON sys_tenant(status);

-- 2. 部门表（无限层级组织树）—— 支持公司/部门/团队/项目组/虚拟组织
-- 用物化路径(path)加速子树查询，避免递归CTE性能问题
CREATE TABLE sys_department (
    id          VARCHAR(64)  PRIMARY KEY,                 -- 部门全局唯一ID，格式 dept_UUID
    tenant_id   VARCHAR(64)  NOT NULL,                    -- 租户ID，多租户隔离
    parent_id   VARCHAR(64),                               -- 父部门ID，NULL=根部门
    dept_code   VARCHAR(64)  NOT NULL,                    -- 部门业务编码，租户内唯一
    dept_name   VARCHAR(255) NOT NULL,                    -- 部门显示名称
    dept_type   VARCHAR(32)  DEFAULT 'org',               -- 部门类型: org(组织)/team(团队)/project(项目)/virtual(虚拟)
    leader_id   VARCHAR(64),                               -- 部门负责人用户ID
    sort_order  INT          DEFAULT 0,                    -- 排序号，同级部门排序
    path        VARCHAR(512),                              -- 物化路径，如 /root/tech/ai/，加速子树查询
    status      VARCHAR(16)  DEFAULT 'active',            -- 状态: active/inactive
    created_at  TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP,
    deleted_at  TIMESTAMP
);
COMMENT ON TABLE  sys_department IS '部门表（无限层级组织树）—— 支持公司/部门/团队/项目组/虚拟组织，物化路径加速子树查询';
COMMENT ON COLUMN sys_department.path IS '物化路径，如 /root/tech/ai/，加速子树查询，避免递归CTE';
COMMENT ON COLUMN sys_department.dept_type IS '部门类型: org(组织)/team(团队)/project(项目)/virtual(虚拟)';
CREATE INDEX idx_dept_tenant_parent ON sys_department(tenant_id, parent_id);
CREATE INDEX idx_dept_path ON sys_department(path);
CREATE UNIQUE INDEX uk_dept_tenant_code ON sys_department(tenant_id, dept_code);

-- 3. 用户表（成员/Member）—— 系统用户，支持多类型用户
-- 密码用 bcrypt/argon2 单向哈希存储，不可逆
CREATE TABLE sys_user (
    id              VARCHAR(64)  PRIMARY KEY,             -- 用户全局唯一ID，格式 usr_UUID
    tenant_id       VARCHAR(64)  NOT NULL,                -- 租户ID
    username        VARCHAR(128) NOT NULL,                -- 登录名，租户内唯一
    password_hash   VARCHAR(255) NOT NULL,                -- 密码哈希(bcrypt/argon2)，单向不可逆
    real_name       VARCHAR(128) NOT NULL,                -- 真实姓名
    email           VARCHAR(255),                          -- 邮箱，用于通知/找回密码
    phone           VARCHAR(32),                           -- 手机号，用于通知/双因素认证
    avatar          VARCHAR(512),                          -- 头像URL
    dept_id         VARCHAR(64),                           -- 主部门ID
    user_type       VARCHAR(32)  DEFAULT 'employee',      -- 用户类型: employee(员工)/partner(伙伴)/customer(客户)/system(系统)
    status          VARCHAR(16)  DEFAULT 'active',        -- 状态: active(活跃)/suspended(暂停)/left(离职)
    last_login_at   TIMESTAMP,                             -- 最后登录时间
    last_login_ip   VARCHAR(64),                           -- 最后登录IP
    extra           JSONB,                                 -- 扩展字段，元数据驱动的自定义属性
    created_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP,
    deleted_at      TIMESTAMP
);
COMMENT ON TABLE  sys_user IS '用户表（成员/Member）—— 系统用户，密码bcrypt/argon2哈希，支持多类型用户';
COMMENT ON COLUMN sys_user.password_hash IS '密码哈希(bcrypt/argon2)，单向不可逆，禁止明文存储';
COMMENT ON COLUMN sys_user.extra IS '扩展字段JSONB，元数据驱动的自定义用户属性';
CREATE UNIQUE INDEX uk_user_tenant_username ON sys_user(tenant_id, username);
CREATE INDEX idx_user_tenant_dept ON sys_user(tenant_id, dept_id);
CREATE INDEX idx_user_status ON sys_user(status);

-- 4. 用户-部门关系表—— 一个用户可属于多个部门，每个部门可有不同职位
CREATE TABLE sys_user_dept (
    id          VARCHAR(64) PRIMARY KEY,                   -- 关系ID
    tenant_id   VARCHAR(64) NOT NULL,                      -- 租户ID
    user_id     VARCHAR(64) NOT NULL,                      -- 用户ID
    dept_id     VARCHAR(64) NOT NULL,                      -- 部门ID
    is_primary  BOOLEAN     DEFAULT FALSE,                 -- 是否主部门，一个用户只有一个主部门
    position    VARCHAR(128),                              -- 在该部门的职位
    created_at  TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP
);
COMMENT ON TABLE  sys_user_dept IS '用户-部门关系表—— 一个用户可属于多个部门，每个部门可有不同职位';
CREATE UNIQUE INDEX uk_user_dept ON sys_user_dept(user_id, dept_id);
CREATE INDEX idx_user_dept_tenant ON sys_user_dept(tenant_id);

-- ===== 第二组：权限体系 =====

-- 5. 角色表（RBAC核心）—— 权限的集合，用户通过角色获得权限
-- data_scope 控制数据权限范围: all/dept/dept_sub/self/custom
CREATE TABLE sys_role (
    id          VARCHAR(64)  PRIMARY KEY,                 -- 角色ID，格式 role_UUID
    tenant_id   VARCHAR(64)  NOT NULL,                    -- 租户ID
    role_code   VARCHAR(64)  NOT NULL,                    -- 角色编码，租户内唯一，如 admin/manager/editor/viewer
    role_name   VARCHAR(128) NOT NULL,                    -- 角色显示名称
    role_type   VARCHAR(32)  DEFAULT 'business',          -- 角色类型: system(系统内置)/business(业务)/custom(自定义)
    data_scope  VARCHAR(32)  DEFAULT 'self',              -- 数据权限范围: all(全部)/dept(本部门)/dept_sub(本部门及子部门)/self(仅本人)/custom(自定义)
    description VARCHAR(512),                              -- 角色描述
    status      VARCHAR(16)  DEFAULT 'active',            -- 状态
    sort_order  INT          DEFAULT 0,                    -- 排序号
    created_at  TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP,
    deleted_at  TIMESTAMP
);
COMMENT ON TABLE  sys_role IS '角色表（RBAC核心）—— 权限的集合，用户通过角色获得权限，data_scope控制数据权限范围';
COMMENT ON COLUMN sys_role.data_scope IS '数据权限范围: all(全部数据)/dept(本部门)/dept_sub(本部门及子部门)/self(仅本人)/custom(自定义规则)';
CREATE UNIQUE INDEX uk_role_tenant_code ON sys_role(tenant_id, role_code);
CREATE INDEX idx_role_tenant ON sys_role(tenant_id);

-- 6. 用户-角色关系表—— 多对多，一个用户可有多角色，一个角色可有多用户
CREATE TABLE sys_user_role (
    id          VARCHAR(64) PRIMARY KEY,
    tenant_id   VARCHAR(64) NOT NULL,
    user_id     VARCHAR(64) NOT NULL,                      -- 用户ID
    role_id     VARCHAR(64) NOT NULL,                      -- 角色ID
    created_at  TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP
);
COMMENT ON TABLE  sys_user_role IS '用户-角色关系表—— 多对多，一个用户可有多角色，一个角色可有多用户';
CREATE UNIQUE INDEX uk_user_role ON sys_user_role(user_id, role_id);
CREATE INDEX idx_user_role_tenant ON sys_user_role(tenant_id);

-- 7. 权限表（菜单/按钮/API/数据四级权限）—— 权限树结构
-- perm_type: menu(菜单)/button(按钮)/api(API接口)/data(数据权限)
CREATE TABLE sys_permission (
    id              VARCHAR(64)  PRIMARY KEY,             -- 权限ID
    tenant_id       VARCHAR(64)  NOT NULL,                -- 租户ID，系统权限tenant_id=system
    parent_id       VARCHAR(64),                           -- 父权限ID，构建权限树/菜单树
    perm_code       VARCHAR(128) NOT NULL,                -- 权限编码，如 system:user:list / system:user:create
    perm_name       VARCHAR(128) NOT NULL,                -- 权限显示名称
    perm_type       VARCHAR(16)  NOT NULL,                -- 权限类型: menu(菜单)/button(按钮)/api(API接口)/data(数据权限)
    path            VARCHAR(512),                          -- 前端路由路径(menu类型)
    component       VARCHAR(255),                          -- 前端组件路径(menu类型)
    icon            VARCHAR(64),                           -- 菜单图标
    api_method      VARCHAR(16),                           -- API方法: GET/POST/PUT/DELETE(api类型)
    api_path        VARCHAR(512),                          -- API路径(api类型)
    sort_order      INT          DEFAULT 0,                -- 排序号
    visible         BOOLEAN      DEFAULT TRUE,             -- 菜单是否可见，FALSE=隐藏菜单但权限仍生效
    status          VARCHAR(16)  DEFAULT 'active',
    created_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP
);
COMMENT ON TABLE  sys_permission IS '权限表（菜单/按钮/API/数据四级权限）—— 权限树结构，perm_code格式 模块:实体:操作';
COMMENT ON COLUMN sys_permission.perm_type IS '权限类型: menu(菜单)/button(按钮)/api(API接口)/data(数据权限)';
COMMENT ON COLUMN sys_permission.visible IS '菜单是否可见，FALSE=隐藏菜单但权限仍生效(用于隐藏入口但保留API权限)';
CREATE UNIQUE INDEX uk_perm_tenant_code ON sys_permission(tenant_id, perm_code);
CREATE INDEX idx_perm_tenant_parent ON sys_permission(tenant_id, parent_id);
CREATE INDEX idx_perm_type ON sys_permission(perm_type);

-- 8. 角色-权限关系表—— 多对多
CREATE TABLE sys_role_permission (
    id              VARCHAR(64) PRIMARY KEY,
    tenant_id       VARCHAR(64) NOT NULL,
    role_id         VARCHAR(64) NOT NULL,                  -- 角色ID
    permission_id   VARCHAR(64) NOT NULL,                  -- 权限ID
    created_at      TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP
);
COMMENT ON TABLE  sys_role_permission IS '角色-权限关系表—— 多对多，角色拥有的权限集合';
CREATE UNIQUE INDEX uk_role_perm ON sys_role_permission(role_id, permission_id);
CREATE INDEX idx_role_perm_tenant ON sys_role_permission(tenant_id);

-- 9. 数据权限规则表（自定义数据范围）—— 当角色data_scope=custom时，用此表定义过滤规则
-- operator: eq/ne/gt/lt/in/like/between
-- logic: AND/OR 多条规则之间的逻辑关系
CREATE TABLE sys_data_permission_rule (
    id          VARCHAR(64)  PRIMARY KEY,                 -- 规则ID
    tenant_id   VARCHAR(64)  NOT NULL,                    -- 租户ID
    role_id     VARCHAR(64)  NOT NULL,                    -- 角色ID
    entity_code VARCHAR(64)  NOT NULL,                    -- 实体编码，对应meta_entity.entity_code
    field_code  VARCHAR(64)  NOT NULL,                    -- 字段编码，对应meta_field.field_code
    operator    VARCHAR(16)  NOT NULL,                    -- 操作符: eq/ne/gt/lt/in/like/between
    value       VARCHAR(512),                              -- 比较值，in用逗号分隔，between用~分隔
    logic       VARCHAR(8)   DEFAULT 'AND',               -- 与同角色同实体其他规则的逻辑关系: AND/OR
    sort_order  INT          DEFAULT 0,                    -- 规则执行顺序
    created_at  TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
COMMENT ON TABLE  sys_data_permission_rule IS '数据权限规则表（自定义数据范围）—— 当角色data_scope=custom时，用此表定义字段级过滤规则，查询时自动注入WHERE条件';
COMMENT ON COLUMN sys_data_permission_rule.operator IS '操作符: eq(等于)/ne(不等于)/gt(大于)/lt(小于)/in(包含)/like(模糊)/between(范围)';
COMMENT ON COLUMN sys_data_permission_rule.logic IS '与同角色同实体其他规则的逻辑关系: AND(且)/OR(或)';
CREATE INDEX idx_data_rule_role ON sys_data_permission_rule(role_id);
CREATE INDEX idx_data_rule_entity ON sys_data_permission_rule(tenant_id, entity_code);

-- ===== 第三组：元数据引擎 =====

-- 10. 实体定义表（业务对象元数据）—— 元数据驱动的核心，定义业务实体
-- 实体引擎根据此表自动生成: 数据库表/REST API/gRPC服务/前端页面/权限/菜单
CREATE TABLE meta_entity (
    id              VARCHAR(64)  PRIMARY KEY,             -- 实体ID
    tenant_id       VARCHAR(64)  NOT NULL,                -- 租户ID
    entity_code     VARCHAR(64)  NOT NULL,                -- 实体编码，租户内唯一，如 customer/order/product
    entity_name     VARCHAR(128) NOT NULL,                -- 实体显示名称
    table_name      VARCHAR(128) NOT NULL,                -- 对应数据库表名，自动加biz_前缀
    entity_type     VARCHAR(32)  DEFAULT 'business',      -- 实体类型: business(业务)/system(系统)/dict(字典)/workflow(流程)
    description     VARCHAR(512),                          -- 实体描述
    icon            VARCHAR(64),                           -- 实体图标
    color           VARCHAR(16),                           -- 实体主题色
    is_auditable    BOOLEAN      DEFAULT TRUE,             -- 是否开启审计日志
    is_soft_delete  BOOLEAN      DEFAULT TRUE,             -- 是否软删除
    is_versioned    BOOLEAN      DEFAULT FALSE,            -- 是否版本控制(保留历史版本)
    search_fields   JSONB,                                 -- 可搜索字段列表，用于全局搜索
    list_layout     JSONB,                                 -- 列表页布局配置(列定义/筛选/排序/按钮)
    form_layout     JSONB,                                 -- 表单页布局配置(分组/字段/校验/联动)
    detail_layout   JSONB,                                 -- 详情页布局配置(Tab/关联/操作)
    status          VARCHAR(16)  DEFAULT 'active',
    created_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP
);
COMMENT ON TABLE  meta_entity IS '实体定义表（业务对象元数据）—— 元数据驱动核心，实体引擎据此自动生成: 数据库表/REST API/gRPC服务/前端页面/权限/菜单';
COMMENT ON COLUMN meta_entity.list_layout IS '列表页布局JSON: 列定义/筛选器/排序/批量操作/按钮权限';
COMMENT ON COLUMN meta_entity.form_layout IS '表单页布局JSON: 分组/字段顺序/校验规则/联动规则/组件类型';
COMMENT ON COLUMN meta_entity.detail_layout IS '详情页布局JSON: Tab页/关联实体/操作按钮/时间线';
CREATE UNIQUE INDEX uk_entity_tenant_code ON meta_entity(tenant_id, entity_code);
CREATE INDEX idx_entity_type ON meta_entity(entity_type);

-- 11. 字段定义表（实体字段元数据）—— 定义实体的每个字段，包括数据库列和前端组件
-- field_type: string/text/int/decimal/boolean/date/datetime/json/ref/file/image
-- component_type: input/textarea/select/datepicker/upload/richtext/switch/number
CREATE TABLE meta_field (
    id              VARCHAR(64)  PRIMARY KEY,             -- 字段ID
    tenant_id       VARCHAR(64)  NOT NULL,                -- 租户ID
    entity_id       VARCHAR(64)  NOT NULL,                -- 所属实体ID
    field_code      VARCHAR(64)  NOT NULL,                -- 字段编码，实体内唯一，如 name/amount/status
    field_name      VARCHAR(128) NOT NULL,                -- 字段显示名称
    column_name     VARCHAR(64)  NOT NULL,                -- 数据库列名
    field_type      VARCHAR(32)  NOT NULL,                -- 字段类型: string/text/int/decimal/boolean/date/datetime/json/ref/file/image
    length          INT,                                   -- 长度(string类型)
    precision       INT,                                   -- 精度(decimal类型，总位数)
    scale           INT,                                   -- 小数位(decimal类型)
    is_required     BOOLEAN      DEFAULT FALSE,            -- 是否必填
    is_unique       BOOLEAN      DEFAULT FALSE,            -- 是否唯一
    is_indexed      BOOLEAN      DEFAULT FALSE,            -- 是否建索引
    is_searchable   BOOLEAN      DEFAULT FALSE,            -- 是否可搜索(全局搜索包含此字段)
    default_value   VARCHAR(512),                          -- 默认值
    ref_entity_id   VARCHAR(64),                           -- 关联实体ID(ref类型)
    ref_display_field VARCHAR(64),                         -- 关联显示字段(ref类型下拉显示哪个字段)
    dict_code       VARCHAR(64),                           -- 字典编码(dict类型下拉选项来源)
    validation_rule JSONB,                                 -- 验证规则: 正则/范围/自定义脚本
    component_type  VARCHAR(32),                           -- 前端组件类型: input/textarea/select/datepicker/upload/richtext/switch/number
    component_props JSONB,                                 -- 前端组件属性: placeholder/options/disabled/readonly
    sort_order      INT          DEFAULT 0,                -- 字段排序
    status          VARCHAR(16)  DEFAULT 'active',
    created_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP
);
COMMENT ON TABLE  meta_field IS '字段定义表（实体字段元数据）—— 定义实体的每个字段，包括数据库列属性和前端组件配置，驱动自动建表和页面生成';
COMMENT ON COLUMN meta_field.field_type IS '字段类型: string(短文本)/text(长文本)/int(整数)/decimal(小数)/boolean(布尔)/date(日期)/datetime(时间)/json(JSON)/ref(关联)/file(文件)/image(图片)';
COMMENT ON COLUMN meta_field.component_type IS '前端组件类型: input/textarea/select/datepicker/upload/richtext/switch/number/cascader/radio/checkbox';
COMMENT ON COLUMN meta_field.validation_rule IS '验证规则JSON: regex(正则)/min/max(范围)/custom(自定义验证脚本)';
CREATE UNIQUE INDEX uk_field_entity_code ON meta_field(entity_id, field_code);
CREATE INDEX idx_field_tenant_entity ON meta_field(tenant_id, entity_id);

-- 12. 关系定义表（实体间关系元数据）—— 定义实体之间的关联关系
-- relation_type: one2one/one2many/many2one/many2many
CREATE TABLE meta_relation (
    id                VARCHAR(64)  PRIMARY KEY,           -- 关系ID
    tenant_id         VARCHAR(64)  NOT NULL,              -- 租户ID
    relation_code     VARCHAR(64)  NOT NULL,              -- 关系编码
    source_entity_id  VARCHAR(64)  NOT NULL,              -- 源实体ID
    target_entity_id  VARCHAR(64)  NOT NULL,              -- 目标实体ID
    relation_type     VARCHAR(16)  NOT NULL,              -- 关系类型: one2one/one2many/many2one/many2many
    source_field      VARCHAR(64),                         -- 源端关联字段(many2one时源实体的外键字段)
    target_field      VARCHAR(64),                         -- 目标端关联字段(one2many时目标实体的外键字段)
    junction_table    VARCHAR(128),                        -- 中间表名(many2many时)
    on_delete         VARCHAR(16)  DEFAULT 'restrict',    -- 删除级联: cascade(级联删除)/restrict(阻止)/set_null(置空)
    is_bidirectional  BOOLEAN      DEFAULT TRUE,           -- 是否双向关联(双方页面都显示关联)
    status            VARCHAR(16)  DEFAULT 'active',
    created_at        TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
COMMENT ON TABLE  meta_relation IS '关系定义表（实体间关系元数据）—— 定义实体之间的关联关系，驱动外键约束/关联查询/关联页面组件生成';
COMMENT ON COLUMN meta_relation.relation_type IS '关系类型: one2one(一对一)/one2many(一对多)/many2one(多对一)/many2many(多对多)';
COMMENT ON COLUMN meta_relation.on_delete IS '删除级联策略: cascade(级联删除)/restrict(阻止删除)/set_null(外键置空)';
CREATE UNIQUE INDEX uk_relation_tenant_code ON meta_relation(tenant_id, relation_code);
CREATE INDEX idx_relation_source ON meta_relation(source_entity_id);
CREATE INDEX idx_relation_target ON meta_relation(target_entity_id);

-- ===== 第四组：字典 =====

-- 13. 字典表—— 可配置的数据字典，用于下拉选项/状态枚举
CREATE TABLE sys_dict (
    id          VARCHAR(64)  PRIMARY KEY,                 -- 字典ID
    tenant_id   VARCHAR(64)  NOT NULL,                    -- 租户ID
    dict_code   VARCHAR(64)  NOT NULL,                    -- 字典编码，租户内唯一，如 order_status/priority
    dict_name   VARCHAR(128) NOT NULL,                    -- 字典显示名称
    description VARCHAR(512),                              -- 字典描述
    status      VARCHAR(16)  DEFAULT 'active',
    created_at  TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP
);
COMMENT ON TABLE  sys_dict IS '字典表—— 可配置的数据字典，用于下拉选项/状态枚举/类型分类，meta_field.dict_code引用此表';
CREATE UNIQUE INDEX uk_dict_tenant_code ON sys_dict(tenant_id, dict_code);

-- 14. 字典项表—— 字典的具体选项
CREATE TABLE sys_dict_item (
    id          VARCHAR(64)  PRIMARY KEY,                 -- 字典项ID
    tenant_id   VARCHAR(64)  NOT NULL,                    -- 租户ID
    dict_id     VARCHAR(64)  NOT NULL,                    -- 所属字典ID
    item_code   VARCHAR(64)  NOT NULL,                    -- 字典项编码，字典内唯一，如 pending/approved/rejected
    item_name   VARCHAR(128) NOT NULL,                    -- 字典项显示名称
    item_value  VARCHAR(255),                              -- 字典项值(默认=item_code)
    color       VARCHAR(16),                               -- 显示颜色(标签/状态点颜色)
    icon        VARCHAR(64),                               -- 图标
    sort_order  INT          DEFAULT 0,                    -- 排序号
    is_default  BOOLEAN      DEFAULT FALSE,                -- 是否默认选项
    status      VARCHAR(16)  DEFAULT 'active',
    created_at  TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP
);
COMMENT ON TABLE  sys_dict_item IS '字典项表—— 字典的具体选项，支持颜色/图标/默认值/排序，驱动前端下拉组件和状态标签渲染';
CREATE UNIQUE INDEX uk_dict_item_code ON sys_dict_item(dict_id, item_code);
CREATE INDEX idx_dict_item_tenant ON sys_dict_item(tenant_id, dict_id);

-- ===== 第五组：审计日志 =====

-- 15. 审计日志表—— 不可变，按时间追加，按月分区，所有关键操作留痕
-- 按 created_at 按月RANGE分区，超大数据量
CREATE TABLE sys_audit_log (
    id            VARCHAR(64)  PRIMARY KEY,               -- 审计日志ID
    tenant_id     VARCHAR(64)  NOT NULL,                  -- 租户ID
    user_id       VARCHAR(64),                             -- 操作用户ID，系统操作为NULL
    username      VARCHAR(128),                            -- 操作用户名(冗余，便于查询)
    action        VARCHAR(64)  NOT NULL,                  -- 操作类型: login/logout/create/update/delete/export/import/approve/reject
    module        VARCHAR(64),                             -- 操作模块: system/meta/wf/ea/biz_xxx
    entity_code   VARCHAR(64),                             -- 操作实体编码
    entity_id     VARCHAR(64),                             -- 操作实体记录ID
    method        VARCHAR(16),                             -- HTTP方法: GET/POST/PUT/DELETE
    url           VARCHAR(512),                            -- 请求URL
    ip            VARCHAR(64),                             -- 客户端IP
    user_agent    VARCHAR(512),                            -- 客户端User-Agent
    request_data  JSONB,                                   -- 请求数据(脱敏后)
    response_data JSONB,                                   -- 响应数据(脱敏后)
    status        VARCHAR(16),                             -- 操作结果: success/failure
    error_msg     TEXT,                                    -- 失败原因
    duration_ms   INT,                                     -- 耗时(毫秒)
    created_at    TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP  -- 操作时间(UTC)，分区键
);
COMMENT ON TABLE  sys_audit_log IS '审计日志表—— 不可变，按时间追加，按月RANGE分区，所有关键操作留痕，满足等保三级/合规审计要求';
COMMENT ON COLUMN sys_audit_log.action IS '操作类型: login/logout/create/update/delete/export/import/approve/reject/export/grant';
COMMENT ON COLUMN sys_audit_log.request_data IS '请求数据JSONB(脱敏后)，敏感字段(密码/身份证/手机号)自动掩码';
CREATE INDEX idx_audit_tenant_time ON sys_audit_log(tenant_id, created_at);
CREATE INDEX idx_audit_user ON sys_audit_log(user_id);
CREATE INDEX idx_audit_action ON sys_audit_log(action);
-- 分区: 按月RANGE分区，pg_partman自动管理

-- ===== 第六组：流程引擎 =====

-- 16. 流程定义表—— BPMN 2.0流程定义，支持多版本
CREATE TABLE wf_definition (
    id            VARCHAR(64)  PRIMARY KEY,               -- 流程定义ID
    tenant_id     VARCHAR(64)  NOT NULL,                  -- 租户ID
    def_code      VARCHAR(64)  NOT NULL,                  -- 流程编码，租户内唯一，如 order_approval/purchase_request
    def_name      VARCHAR(128) NOT NULL,                  -- 流程显示名称
    entity_code   VARCHAR(64),                             -- 关联实体编码(流程驱动哪个业务实体)
    version       INT          DEFAULT 1,                  -- 版本号，同def_code可有多版本并行
    bpmn_xml      TEXT,                                    -- BPMN 2.0 XML定义
    status        VARCHAR(16)  DEFAULT 'draft',           -- 状态: draft(草稿)/published(已发布)/deprecated(已废弃)
    is_active     BOOLEAN      DEFAULT FALSE,              -- 是否当前激活版本(同def_code只有一个active)
    created_by    VARCHAR(64),                             -- 创建人
    created_at    TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMP
);
COMMENT ON TABLE  wf_definition IS '流程定义表—— BPMN 2.0流程定义，支持多版本并行/灰度发布/回滚，is_active标记当前生效版本';
COMMENT ON COLUMN wf_definition.bpmn_xml IS 'BPMN 2.0标准XML，流程设计器可视化编辑后存储';
COMMENT ON COLUMN wf_definition.status IS '流程状态: draft(草稿)/published(已发布)/deprecated(已废弃)';
CREATE UNIQUE INDEX uk_wf_def_tenant_code_ver ON wf_definition(tenant_id, def_code, version);
CREATE INDEX idx_wf_def_entity ON wf_definition(tenant_id, entity_code);

-- 17. 流程实例表—— 一个流程的一次执行实例
CREATE TABLE wf_instance (
    id            VARCHAR(64)  PRIMARY KEY,               -- 流程实例ID
    tenant_id     VARCHAR(64)  NOT NULL,                  -- 租户ID
    def_id        VARCHAR(64)  NOT NULL,                  -- 流程定义ID
    entity_code   VARCHAR(64),                             -- 关联实体编码
    entity_id     VARCHAR(64),                             -- 关联实体记录ID
    title         VARCHAR(255),                            -- 实例标题(自动生成或自定义)
    status        VARCHAR(16)  DEFAULT 'running',         -- 状态: running(运行中)/completed(已完成)/cancelled(已取消)/suspended(已挂起)
    initiator_id  VARCHAR(64)  NOT NULL,                  -- 发起人ID
    current_node  VARCHAR(64),                             -- 当前节点ID(BPMN node id)
    started_at    TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,  -- 开始时间
    completed_at  TIMESTAMP,                               -- 完成时间
    due_at        TIMESTAMP,                               -- 截止时间(SLA)
    extra         JSONB                                    -- 扩展数据(流程变量快照)
);
COMMENT ON TABLE  wf_instance IS '流程实例表—— 一个流程的一次执行实例，关联业务实体记录，记录流程状态和SLA截止时间';
COMMENT ON COLUMN wf_instance.status IS '实例状态: running(运行中)/completed(已完成)/cancelled(已取消)/suspended(已挂起)';
CREATE INDEX idx_wf_inst_tenant_status ON wf_instance(tenant_id, status);
CREATE INDEX idx_wf_inst_entity ON wf_instance(tenant_id, entity_code, entity_id);
CREATE INDEX idx_wf_inst_initiator ON wf_instance(initiator_id);

-- 18. 流程任务表—— 流程中的用户任务/审批任务
CREATE TABLE wf_task (
    id            VARCHAR(64)  PRIMARY KEY,               -- 任务ID
    tenant_id     VARCHAR(64)  NOT NULL,                  -- 租户ID
    instance_id   VARCHAR(64)  NOT NULL,                  -- 所属流程实例ID
    node_id       VARCHAR(64)  NOT NULL,                  -- BPMN节点ID
    node_name     VARCHAR(128),                            -- 节点显示名称
    node_type     VARCHAR(32),                             -- 节点类型: user_task(用户任务)/service_task(服务任务)/gateway(网关)/event(事件)
    assignee_id   VARCHAR(64),                             -- 指派处理人ID
    candidate_users JSONB,                                  -- 候选用户ID列表(多人竞争领取)
    candidate_roles JSONB,                                  -- 候选角色编码列表(按角色动态分配)
    status        VARCHAR(16)  DEFAULT 'pending',         -- 状态: pending(待处理)/claimed(已领取)/completed(已完成)/rejected(已拒绝)/delegated(已转办)
    action        VARCHAR(16),                             -- 处理动作: approve(同意)/reject(拒绝)/delegate(转办)/transfer(移交)
    comment       TEXT,                                    -- 处理意见
    form_data     JSONB,                                   -- 表单数据(任务表单填写内容)
    claimed_at    TIMESTAMP,                               -- 领取时间
    completed_at  TIMESTAMP,                               -- 完成时间
    due_at        TIMESTAMP,                               -- 任务截止时间(SLA)
    created_at    TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
COMMENT ON TABLE  wf_task IS '流程任务表—— 流程中的用户任务/审批任务，支持指派/候选竞争/角色动态分配/转办移交/SLA超时';
COMMENT ON COLUMN wf_task.status IS '任务状态: pending(待处理)/claimed(已领取)/completed(已完成)/rejected(已拒绝)/delegated(已转办)/transferred(已移交)';
COMMENT ON COLUMN wf_task.candidate_roles IS '候选角色编码列表JSON，运行时动态查询拥有该角色的用户作为候选人';
CREATE INDEX idx_wf_task_instance ON wf_task(instance_id);
CREATE INDEX idx_wf_task_assignee ON wf_task(tenant_id, assignee_id, status);
CREATE INDEX idx_wf_task_status ON wf_task(tenant_id, status);

-- ===== 第七组：专家联盟 =====

-- 19. 专家注册表—— AI专家注册/能力定义/评分/版本管理
CREATE TABLE ea_expert (
    id              VARCHAR(64)  PRIMARY KEY,             -- 专家ID
    tenant_id       VARCHAR(64)  NOT NULL,                -- 租户ID
    expert_code     VARCHAR(64)  NOT NULL,                -- 专家编码，租户内唯一
    expert_name     VARCHAR(128) NOT NULL,                -- 专家显示名称
    description     TEXT,                                  -- 专家描述/能力介绍
    avatar          VARCHAR(512),                          -- 专家头像
    domain_tags     JSONB,                                 -- 领域标签列表，如 ["法律","合同审查"]
    capability_tags JSONB,                                 -- 能力标签列表，如 ["文本生成","代码审查","数据分析"]
    input_schema    JSONB,                                 -- 输入JSON Schema定义
    output_schema   JSONB,                                 -- 输出JSON Schema定义
    tools           JSONB,                                 -- 可用工具列表(工具名/描述/参数Schema)
    prompt_template TEXT,                                  -- 系统提示词模板
    model_config    JSONB,                                 -- 模型配置: model/temperature/max_tokens/api_endpoint/api_key(加密)
    version         VARCHAR(32)  DEFAULT '1.0.0',         -- 专家版本号，语义化版本
    rating          DECIMAL(3,2) DEFAULT 0,               -- 综合评分0-5，ELO等级分+用户评分加权
    rating_count    INT          DEFAULT 0,                -- 评分次数
    success_rate    DECIMAL(5,2) DEFAULT 0,               -- 任务成功率%
    avg_latency_ms  INT,                                   -- 平均响应延迟(毫秒)
    cost_per_call   DECIMAL(10,6),                        -- 平均每次调用成本(美元)
    status          VARCHAR(16)  DEFAULT 'active',        -- 状态: active/inactive/maintenance
    created_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP
);
COMMENT ON TABLE  ea_expert IS '专家注册表—— AI专家注册/能力定义/评分/版本管理，全维页面配置，专家匹配算法基于此表的embedding和评分';
COMMENT ON COLUMN ea_expert.model_config IS '模型配置JSON: model(模型名)/temperature(温度)/max_tokens(最大Token)/api_endpoint(API地址)/api_key(加密存储)';
COMMENT ON COLUMN ea_expert.rating IS '综合评分0-5，算法: 0.4*ELO等级分 + 0.3*用户评分 + 0.2*成功率 - 0.1*成本系数';
CREATE UNIQUE INDEX uk_ea_expert_tenant_code ON ea_expert(tenant_id, expert_code);
CREATE INDEX idx_ea_expert_domain ON ea_expert(tenant_id, status);

-- 20. 专家能力向量表—— 存储专家能力描述的embedding，用于语义匹配检索
-- 需要 pgvector 扩展: CREATE EXTENSION vector;
CREATE TABLE ea_expert_embedding (
    id          VARCHAR(64) PRIMARY KEY,                   -- 记录ID
    expert_id   VARCHAR(64) NOT NULL,                      -- 专家ID
    embedding   vector(1536),                              -- 能力描述embedding向量(1536维,text-embedding-3-small)
    created_at  TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP
);
COMMENT ON TABLE  ea_expert_embedding IS '专家能力向量表—— 存储专家能力描述的embedding，pgvector向量相似度检索，专家匹配算法第一步';
COMMENT ON COLUMN ea_expert_embedding.embedding IS '能力描述embedding向量1536维，由text-embedding-3-small模型生成，专家描述+领域标签+能力标签拼接后生成';
CREATE INDEX idx_ea_embed_expert ON ea_expert_embedding(expert_id);
CREATE INDEX idx_ea_embed_ivfflat ON ea_expert_embedding USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

-- 21. 联盟任务表—— 提交给专家联盟处理的任务
CREATE TABLE ea_alliance_task (
    id              VARCHAR(64)  PRIMARY KEY,             -- 任务ID
    tenant_id       VARCHAR(64)  NOT NULL,                -- 租户ID
    task_code       VARCHAR(64)  NOT NULL,                -- 任务业务编码
    title           VARCHAR(255) NOT NULL,                -- 任务标题
    description     TEXT,                                  -- 任务描述
    input_data      JSONB,                                 -- 任务输入数据
    context         JSONB,                                 -- 上下文信息(历史对话/相关文档/用户偏好)
    constraints     JSONB,                                 -- 约束: 预算上限/时间上限/质量要求/指定专家
    status          VARCHAR(16)  DEFAULT 'pending',       -- 状态: pending(待匹配)/matching(匹配中)/running(执行中)/completed(已完成)/failed(失败)/cancelled(已取消)
    matched_experts JSONB,                                 -- 匹配到的专家列表(专家ID+匹配度+预估成本)
    execution_mode  VARCHAR(16)  DEFAULT 'parallel',      -- 执行模式: parallel(并行)/serial(串行)/hybrid(混合)
    fusion_strategy VARCHAR(32)  DEFAULT 'weighted_vote', -- 融合策略: weighted_vote(加权投票)/stacking(元学习)/debate(辩论)/confidence(置信度)
    result          JSONB,                                 -- 最终融合结果
    confidence      DECIMAL(5,2),                          -- 最终结果置信度0-100
    initiator_id    VARCHAR(64),                           -- 发起人ID
    created_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at      TIMESTAMP,                               -- 开始执行时间
    completed_at    TIMESTAMP                                -- 完成时间
);
COMMENT ON TABLE  ea_alliance_task IS '联盟任务表—— 提交给专家联盟处理的任务，记录匹配/执行/融合全流程，支持并行/串行/混合执行和多种融合策略';
COMMENT ON COLUMN ea_alliance_task.execution_mode IS '执行模式: parallel(所有专家并行执行)/serial(按优先级串行执行)/hybrid(先并行初选再串行精修)';
COMMENT ON COLUMN ea_alliance_task.fusion_strategy IS '融合策略: weighted_vote(按评分加权投票)/stacking(元学习器融合)/debate(专家互相辩论收敛)/confidence(按置信度加权)';
CREATE UNIQUE INDEX uk_ea_task_tenant_code ON ea_alliance_task(tenant_id, task_code);
CREATE INDEX idx_ea_task_status ON ea_alliance_task(tenant_id, status);

-- 22. 专家执行记录表—— 每个专家在联盟任务中的执行详情
CREATE TABLE ea_expert_execution (
    id              VARCHAR(64)  PRIMARY KEY,             -- 执行记录ID
    tenant_id       VARCHAR(64)  NOT NULL,                -- 租户ID
    task_id         VARCHAR(64)  NOT NULL,                -- 联盟任务ID
    expert_id       VARCHAR(64)  NOT NULL,                -- 专家ID
    status          VARCHAR(16)  DEFAULT 'pending',       -- 状态: pending/running/completed/failed/timeout
    input_data      JSONB,                                 -- 实际输入数据(含上下文组装)
    output_data     JSONB,                                 -- 专家输出结果
    confidence      DECIMAL(5,2),                          -- 专家自评置信度0-100
    reasoning_trace JSONB,                                 -- ReAct思考轨迹(thought/action/observation序列)
    tool_calls      JSONB,                                 -- 工具调用记录(工具名/参数/结果/耗时)
    latency_ms      INT,                                   -- 执行耗时(毫秒)
    token_usage     JSONB,                                 -- Token用量: prompt_tokens/completion_tokens/total_tokens
    cost            DECIMAL(10,6),                         -- 本次调用成本(美元)
    error_msg       TEXT,                                  -- 失败原因
    created_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at    TIMESTAMP
);
COMMENT ON TABLE  ea_expert_execution IS '专家执行记录表—— 每个专家在联盟任务中的执行详情，包含ReAct思考轨迹/工具调用/Token用量/成本，用于结果溯源和评分更新';
COMMENT ON COLUMN ea_expert_execution.reasoning_trace IS 'ReAct思考轨迹JSON: [{type:thought/action/observation, content:..., timestamp:...}]，完整记录专家推理过程';
COMMENT ON COLUMN ea_expert_execution.token_usage IS 'Token用量JSON: {prompt_tokens, completion_tokens, total_tokens}，用于成本核算和模型选型';
CREATE INDEX idx_ea_exec_task ON ea_expert_execution(task_id);
CREATE INDEX idx_ea_exec_expert ON ea_expert_execution(expert_id);
CREATE INDEX idx_ea_exec_status ON ea_expert_execution(tenant_id, status);

-- 23. 专家记忆表—— 专家的短期/长期/语义/情景记忆，RAG检索
CREATE TABLE ea_expert_memory (
    id              VARCHAR(64)  PRIMARY KEY,             -- 记忆ID
    tenant_id       VARCHAR(64)  NOT NULL,                -- 租户ID
    expert_id       VARCHAR(64)  NOT NULL,                -- 专家ID
    memory_type     VARCHAR(16)  NOT NULL,                -- 记忆类型: short_term(短期)/long_term(长期)/semantic(语义)/episodic(情景)
    content         TEXT         NOT NULL,                 -- 记忆内容
    embedding       vector(1536),                          -- 内容embedding(语义记忆/RAG检索)
    metadata        JSONB,                                 -- 元数据: 来源/标签/实体/时间
    source_task_id  VARCHAR(64),                           -- 来源任务ID(从哪个任务提取的记忆)
    importance      DECIMAL(3,2) DEFAULT 0.5,             -- 重要度0-1，低于阈值不写入长期记忆
    access_count    INT          DEFAULT 0,                -- 访问次数(用于记忆强化/遗忘)
    last_accessed   TIMESTAMP,                             -- 最后访问时间
    expires_at      TIMESTAMP,                             -- 过期时间(短期记忆TTL)
    created_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
COMMENT ON TABLE  ea_expert_memory IS '专家记忆表—— 专家的短期/长期/语义/情景记忆，pgvector向量相似度RAG检索，支持记忆重要度评分/访问强化/TTL过期遗忘';
COMMENT ON COLUMN ea_expert_memory.memory_type IS '记忆类型: short_term(会话级,TTL过期)/long_term(永久,重要度筛选)/semantic(向量化,RAG检索)/episodic(情景,时间+实体关联)';
COMMENT ON COLUMN ea_expert_memory.importance IS '重要度0-1，任务完成后自动评分，低于阈值(默认0.3)不写入长期记忆，高重要度记忆优先检索';
CREATE INDEX idx_ea_mem_expert_type ON ea_expert_memory(tenant_id, expert_id, memory_type);
CREATE INDEX idx_ea_mem_ivfflat ON ea_expert_memory USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

-- ===== 初始化数据 =====

-- 初始化系统租户
INSERT INTO sys_tenant (id, tenant_code, tenant_name, plan, status)
VALUES ('t_system', 'system', 'System Tenant', 'enterprise', 'active');

-- 初始化超级管理员角色
INSERT INTO sys_role (id, tenant_id, role_code, role_name, role_type, data_scope, description)
VALUES ('r_super_admin', 't_system', 'super_admin', '超级管理员', 'system', 'all', '拥有所有权限，不可删除');

-- 初始化系统权限（菜单树）—— 详见 00-COSMIC-META-ARCHITECTURE.md 中的完整列表
-- 系统管理 / 元数据管理 / 流程管理 / 专家联盟 四大菜单组

-- ============================================================
-- 说明:
-- 1. 业务表由元数据引擎自动生成: CREATE TABLE biz_{entity_code} (...公共字段 + 配置字段...)
-- 2. 所有业务表自动包含公共字段: id/tenant_id/created_by/created_at/updated_by/updated_at/deleted_at/version
-- 3. 推荐扩展: pgvector(向量检索) / pg_partman(分区管理) / pgcrypto(加密) / pg_stat_statements(性能监控)
-- 4. 性能优化: 审计日志按月分区 / 大表建BRIN索引 / 热数据SSD / 冷数据对象存储 / 读写分离
-- 5. 安全: 密码bcrypt / 敏感字段AES-256 / 连接TLS / RLS行级安全 / 最小权限账号 / 全量审计
-- ============================================================
