/**
 * ============================================================
 *  璇玑 RelGraph · 宇宙级元数据引擎 Schema 定义
 * ============================================================
 *
 *  架构层级：L5Domain · 全维可配置元数据驱动
 *  核心能力：
 *    1. 业务实体可配置（无需改表即可新增业务对象）
 *    2. 动态字段可配置（字符串/数字/日期/枚举/关联/附件/JSON...）
 *    3. 页面布局可配置（表单/列表/详情/看板/甘特/卡片）
 *    4. 工作流可配置（BPMN2.0 简化模型 + 审批流转 + 条件分支）
 *    5. 业务规则可配置（校验规则/计算规则/联动规则/触发规则）
 *    6. 行业包可配置（行业模板一键导入：政务/金融/医疗/制造/教育/零售...）
 *
 *  设计哲学：把"需要改代码才能改业务"变成"改配置即改业务"
 *            所有行业差异都收敛为配置差异
 * ============================================================
 */

'use strict';

const MetaDDL = `
-- ============================================================
-- 1. 行业包 (Industry Package) —— 快速融合不同行业的基础
-- ============================================================
CREATE TABLE IF NOT EXISTS meta_industry_package (
  package_id      CHAR(36) PRIMARY KEY,
  package_code    VARCHAR(64)  NOT NULL UNIQUE,               -- 行业编码: gov/finance/medical/mfg/edu/retail/...
  package_name    VARCHAR(255) NOT NULL,                       -- 行业名称: 政务/金融/医疗/制造/教育/零售/...
  package_version VARCHAR(32)  NOT NULL DEFAULT '1.0.0',       -- 语义版本
  description     VARCHAR(1024),
  icon            VARCHAR(255),
  banner          VARCHAR(1024),
  -- 行业特性开关
  features        JSON,                                        -- 启用的特性清单
  -- 行业预置数据
  seed_entities   JSON,                                        -- 预置实体列表
  seed_workflows  JSON,                                        -- 预置流程列表
  seed_rules      JSON,                                        -- 预置规则列表
  -- 行业合规配置
  compliance      JSON,                                        -- 行业合规要求（等保/ISO/PCI-DSS/HIPAA...）
  -- 状态
  is_official     TINYINT(1)   DEFAULT 1,                      -- 是否官方认证
  status          VARCHAR(16)  NOT NULL DEFAULT 'active',
  metadata        JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)
);

-- 租户-行业包安装记录
CREATE TABLE IF NOT EXISTS meta_tenant_industry (
  ti_id           CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  package_id      CHAR(36)     NOT NULL,
  install_version VARCHAR(32),
  installed_at    DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  installed_by    CHAR(36),
  config          JSON,                                        -- 行业包个性化配置
  status          VARCHAR(16)  NOT NULL DEFAULT 'active',      -- active/disabled/upgrading
  UNIQUE KEY uk_tenant_industry (tenant_id, package_id)
);

-- ============================================================
-- 2. 业务实体 (Business Entity) —— 元数据驱动的业务对象
-- ============================================================
CREATE TABLE IF NOT EXISTS meta_entity (
  entity_id       CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  entity_code     VARCHAR(64)  NOT NULL,                       -- 实体编码(英文): order/customer/project/...
  entity_name     VARCHAR(255) NOT NULL,                       -- 实体名称(中文): 订单/客户/项目
  entity_plural   VARCHAR(255),                                -- 复数名称: 订单们/客户们
  table_name      VARCHAR(64),                                 -- 实际存储表名(可选，默认用biz_data通用表)
  description     VARCHAR(1024),
  icon            VARCHAR(255),
  color           VARCHAR(16),
  -- 分类: 主数据/交易数据/分析数据
  entity_category VARCHAR(32)  DEFAULT 'master',               -- master/transaction/analytics/config
  -- 存储策略
  storage_mode    VARCHAR(16)  NOT NULL DEFAULT 'universal',   -- universal(通用JSON表)/dedicated(独立表)/hybrid(混合)
  shard_key       VARCHAR(64),                                 -- 分库分表键
  history_strategy VARCHAR(16) DEFAULT 'snapshot',             -- none/snapshot/versioned/audit/chronicle
  -- 继承与扩展
  extends_entity_id CHAR(36),                                  -- 继承自哪个实体
  mixin_ids       JSON,                                        -- 混入的Trait ID列表
  -- 标签
  tags            JSON,
  -- UI配置
  list_view_id    CHAR(36),                                    -- 默认列表视图
  form_view_id    CHAR(36),                                    -- 默认表单视图
  detail_view_id  CHAR(36),                                    -- 默认详情视图
  -- 工作流绑定
  workflow_id     CHAR(36),                                    -- 默认审批流程
  -- 权限
  is_system       TINYINT(1)   DEFAULT 0,                      -- 系统内置不可删
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
  UNIQUE KEY uk_tenant_entity_code (tenant_id, entity_code)
);
CREATE INDEX IF NOT EXISTS idx_meta_entity_tenant ON meta_entity(tenant_id);
CREATE INDEX IF NOT EXISTS idx_meta_entity_category ON meta_entity(entity_category);

-- 实体索引配置
CREATE TABLE IF NOT EXISTS meta_entity_index (
  index_id        CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  entity_id       CHAR(36)     NOT NULL,
  index_name      VARCHAR(64)  NOT NULL,
  index_type      VARCHAR(16)  NOT NULL DEFAULT 'normal',      -- normal/unique/fulltext/geo/spatial/gin
  field_codes     JSON        NOT NULL,                        -- 索引字段列表: [["field1","asc"],["field2","desc"]]
  include_fields  JSON,                                        -- 覆盖索引包含字段
  where_condition VARCHAR(512),                                -- 部分索引条件
  status          VARCHAR(16)  NOT NULL DEFAULT 'active',
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_tenant_entity_index (tenant_id, entity_id, index_name)
);

-- ============================================================
-- 3. 动态字段 (Dynamic Field) —— 无需ALTER TABLE即可增删字段
-- ============================================================
CREATE TABLE IF NOT EXISTS meta_field (
  field_id        CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  entity_id       CHAR(36)     NOT NULL,
  field_code      VARCHAR(64)  NOT NULL,
  field_name      VARCHAR(255) NOT NULL,
  -- 字段类型（最全的类型体系）
  field_type      VARCHAR(32)  NOT NULL,
  -- ┌──────────────────────────────────────────────────────┐
  -- │ field_type 完整枚举:
  -- │ 基础: string/text/rich_text/html/markdown/keyword
  -- │ 数字: integer/bigint/decimal/float/double/percentage/money
  -- │ 布尔: boolean/toggle
  -- │ 日期时间: date/time/datetime/timestamp/timerange/daterange
  -- │ 枚举: enum/enum_multi/rating/stars
  -- │ 关联: relation/relation_multi/lookup/reference/parent_child
  -- │ 媒体: image/images/file/files/video/audio/avatar/signature
  -- │ 结构: json/json_array/object/array/map
  -- │ 特殊: user/user_multi/dept/dept_multi/tenant/location/address/
  -- │       phone/email/url/id_card/bank_card/domain/ip
  -- │ 计算: formula/aggregate/auto_increment/barcode/qrcode
  -- │ 系统: id/tenant_id/created_at/updated_at/created_by/...
  -- └──────────────────────────────────────────────────────┘

  -- 基础约束
  is_required     TINYINT(1)   DEFAULT 0,
  is_unique       TINYINT(1)   DEFAULT 0,
  is_indexed      TINYINT(1)   DEFAULT 0,
  is_searchable   TINYINT(1)   DEFAULT 0,
  is_sortable     TINYINT(1)   DEFAULT 0,
  is_filterable   TINYINT(1)   DEFAULT 0,
  is_exportable   TINYINT(1)   DEFAULT 1,
  is_importable   TINYINT(1)   DEFAULT 1,
  is_readonly     TINYINT(1)   DEFAULT 0,
  is_hidden       TINYINT(1)   DEFAULT 0,
  is_system       TINYINT(1)   DEFAULT 0,

  -- 默认值
  default_value   JSON,
  default_expr    VARCHAR(512),                                -- 表达式默认值: {{current_user_id}}/{{now}}/uuid()/...
  auto_fill_on    VARCHAR(16),                                 -- create/update/both (系统自动填充时机)

  -- 长度/范围
  max_length      INTEGER,
  min_value       DECIMAL(30,10),
  max_value       DECIMAL(30,10),
  decimal_places  INTEGER,
  step            DECIMAL(30,10),

  -- 精度/货币
  currency_code   VARCHAR(8),
  unit            VARCHAR(32),

  -- 枚举选项
  options_source  VARCHAR(16)  DEFAULT 'inline',               -- inline/sql/api/dictionary/custom
  options_inline  JSON,                                        -- inline选项: [{label,value,color,icon,disabled}]
  options_sql     TEXT,                                        -- SQL查询返回 options
  options_api     VARCHAR(512),                                -- API URL
  options_dict_code VARCHAR(64),                               -- 数据字典编码

  -- 关联类型配置
  relation_config JSON,
  -- {
  --   target_entity_id, target_field_code, display_fields[],
  --   search_fields[], filter_condition, sort, multiple, cascade_rules
  -- }

  -- 校验规则
  validations     JSON,
  -- [
  --   { type: "regex", pattern: "^1[3-9]\\d{9}$", message: "手机号格式错误" },
  --   { type: "custom", script: "return value > 0", message: "必须大于0" },
  --   { type: "range", min: 0, max: 100 },
  --   { type: "unique", scope: ["tenant_id"] }
  -- ]

  -- 公式/计算
  formula_expr    VARCHAR(2048),                               -- 公式表达式 (字段计算)
  formula_deps    JSON,                                        -- 依赖字段列表

  -- UI 渲染配置
  ui_component    VARCHAR(64),                                 -- 渲染组件名
  ui_props        JSON,                                        -- 组件额外属性
  ui_placeholder  VARCHAR(255),
  ui_hint         VARCHAR(512),
  ui_group        VARCHAR(64),                                 -- 表单分组
  ui_sort_order   INTEGER      DEFAULT 0,
  ui_span         INTEGER      DEFAULT 24,                     -- 栅格宽度 1-24
  ui_newline      TINYINT(1)   DEFAULT 0,                      -- 是否换行
  ui_dynamic_cond VARCHAR(1024),                               -- 动态显示条件表达式

  -- 数据权限
  field_permission JSON,                                       -- 字段级权限 { read_roles:[], edit_roles:[] }

  -- 扩展
  description     VARCHAR(1024),
  tags            JSON,
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
  UNIQUE KEY uk_tenant_entity_field (tenant_id, entity_id, field_code)
);
CREATE INDEX IF NOT EXISTS idx_meta_field_tenant ON meta_field(tenant_id);
CREATE INDEX IF NOT EXISTS idx_meta_field_entity ON meta_field(entity_id);
CREATE INDEX IF NOT EXISTS idx_meta_field_type   ON meta_field(field_type);

-- ============================================================
-- 4. 页面视图 (View) —— 列表/表单/详情/看板/甘特/卡片 全配置
-- ============================================================
CREATE TABLE IF NOT EXISTS meta_view (
  view_id         CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  entity_id       CHAR(36),                                    -- 所属实体(可空，全局视图)
  view_code       VARCHAR(128) NOT NULL,
  view_name       VARCHAR(255) NOT NULL,
  view_type       VARCHAR(32)  NOT NULL,
  -- view_type 枚举: list/table/form/detail/dashboard/kanban/gantt/card/tree/calendar/map/pivot/report/workspace

  -- 视图模式
  view_mode       VARCHAR(16)  DEFAULT 'default',              -- default/embedded/dialog/drawer/fullscreen

  -- 权限
  roles_whitelist JSON,
  roles_blacklist JSON,
  permission_codes JSON,

  -- 视图配置（按view_type差异化存储）
  view_config     JSON        NOT NULL,
  -- ┌──────────────────────────────────────────────────────┐
  -- │ list/table 配置:
  -- │ {
  -- │   columns: [ {field_code, title, width, fixed, sortable,
  -- │               filterable, formatter, component, slot,
  -- │               cell_render, header_render, align} ],
  -- │   toolbar: { showSearch, showFilter, showExport,
  -- │              showImport, actions:[...] },
  -- │   pagination: { pageSize, pageSizeOptions, showTotal },
  // │   rowSelection: { type, keys },
  -- │   expandable: { type, field_code },
  -- │   treeConfig: { childrenField, idField, parentField },
  -- │   advancedFilter: {...},
  -- │   sticky: { header, actionBar }
  -- │ }
  -- │
  -- │ form 配置:
  -- │ {
  -- │   layout: "horizontal/vertical/inline",
  -- │   labelWidth: "100px",
  -- │   labelPosition: "left/right/top",
  -- │   size: "large/default/small",
  -- │   disabled: false,
  -- │   groups: [
  -- │     {
  -- │       title, icon, collapsed, span,
  -- │       fields: [ {field_code, span, required, disabled,
  -- │                 readonly, dynamic_cond, component, props} ]
  -- │     }
  -- │   ],
  -- │   tabs: [ {title, fields:[]} ],
  -- │   footer: { showSubmit, showCancel, showReset,
  -- │              extraActions:[...] }
  -- │ }
  -- │
  -- │ kanban 配置:
  -- │ {
  // │   groupField: "status",
  // │   groups: [{value, title, color, wipLimit}],
  // │   cardFields: ["title","assignee","due_date"],
  // │   draggable: true,
  // │   swimlaneField: "priority"
  // │ }
  -- │
  // │ gantt 配置:
  // │ { startField, endField, progressField, parentField,
  // │   dependencyField, milestones, scales:["day","week","month"] }
  -- └──────────────────────────────────────────────────────┘

  -- 快速过滤/查询方案
  filter_presets  JSON,                                        -- 预设查询方案列表

  -- 排序
  sort_order      INTEGER      DEFAULT 0,
  is_default      TINYINT(1)   DEFAULT 0,

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
  UNIQUE KEY uk_tenant_view_code (tenant_id, view_code)
);
CREATE INDEX IF NOT EXISTS idx_meta_view_tenant ON meta_view(tenant_id);
CREATE INDEX IF NOT EXISTS idx_meta_view_entity ON meta_view(entity_id);
CREATE INDEX IF NOT EXISTS idx_meta_view_type   ON meta_view(view_type);

-- 用户个人视图方案（保存的筛选/列配置）
CREATE TABLE IF NOT EXISTS meta_view_preset (
  preset_id       CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  view_id         CHAR(36)     NOT NULL,
  user_id         CHAR(36)     NOT NULL,
  preset_name     VARCHAR(255) NOT NULL,
  preset_type     VARCHAR(16)  DEFAULT 'filter',               -- filter/column/layout/all
  preset_data     JSON        NOT NULL,                        -- 保存的配置
  is_default      TINYINT(1)   DEFAULT 0,                      -- 是否默认方案
  is_shared       TINYINT(1)   DEFAULT 0,                      -- 是否共享给所有人
  shared_roles    JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)
);
CREATE INDEX IF NOT EXISTS idx_meta_preset_view_user ON meta_view_preset(view_id, user_id);

-- ============================================================
-- 5. 工作流 (Workflow) —— BPMN2.0 简化模型 + 审批流转
-- ============================================================
CREATE TABLE IF NOT EXISTS meta_workflow (
  workflow_id     CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  workflow_code   VARCHAR(128) NOT NULL,
  workflow_name   VARCHAR(255) NOT NULL,
  workflow_category VARCHAR(64),                               -- 审批/业务/通知/数据同步
  description     VARCHAR(1024),
  icon            VARCHAR(255),

  -- 绑定实体
  entity_id       CHAR(36),
  trigger_events  JSON,                                        -- 触发事件: ['CREATE','UPDATE:status']
  trigger_condition VARCHAR(2048),                             -- 触发条件表达式

  -- 流程版本（支持多版本并行/灰度）
  workflow_version INTEGER      NOT NULL DEFAULT 1,
  version_tag     VARCHAR(32),                                 -- v1.0/beta/prod
  is_main_version TINYINT(1)   DEFAULT 1,                      -- 是否主版本

  -- 流程定义（BPMN兼容JSON）
  process_def     JSON        NOT NULL,
  -- ┌──────────────────────────────────────────────────────┐
  -- │ process_def 结构:
  -- │ {
  -- │   nodes: [
  // │     { id, type: "start/end/task/gateway/parallel/event/subprocess",
  // │       name, config: {...} }
  // │   ],
  // │   edges: [ { id, from, to, condition, label } ],
  // │   global: {
  // │     initiator: "{{creator}}",
  // │     due_duration: "P3D",
  // │     escalate_policy: {...},
  // │     notification_policy: {...}
  // │   }
  // │ }
  // │
  // │ node.config by type:
  // │ - start: { form_fields:[] }
  // │ - task: {
  // │     assignee_type: "user/role/dept/position/expression/candidate/auto",
  // │     assignees: ["user_id"] | "{{leader}}",
  // │     candidate_type: "all/any/order/vote",
  // │     vote_ratio: 0.5,
  // │     form_permission: { editable_fields:[], readonly_fields:[] },
  // │     actions: ["同意","驳回","转交","加签","减签","退回到发起人","终止"],
  // │     due_duration: "P1D",
  // │     reminder_policy: {...},
  // │     auto_pass_if: "{{expr}}"
  // │   }
  // │ - gateway: {
  // │     type: "exclusive/inclusive/parallel/event",
  // │     conditions: [ {edge_id, expr: "{{amount > 10000}}"} ]
  // │   }
  // │ - subprocess: { workflow_id: "..." }
  // │ - event: {
  // │     type: "timer/message/signal/error",
  // │     config: { duration: "P1D", webhook: "..." }
  // │   }
  -- └──────────────────────────────────────────────────────┘

  -- 通知配置
  notification    JSON,
  -- {
  // │   on_start: { channels:["in_app","email","sms"], template:"wf_start" },
  // │   on_task: { channels:["in_app","dingtalk"], template:"wf_task" },
  // │   on_end:   { channels:["in_app","email"], template:"wf_end" }
  -- }

  -- 权限
  start_roles     JSON,                                        -- 可发起的角色
  admin_roles     JSON,                                        -- 流程管理员(可干预)
  viewer_roles    JSON,                                        -- 可查看全部流程实例

  -- 状态
  is_draft        TINYINT(1)   DEFAULT 1,                      -- 是否草稿
  is_suspended    TINYINT(1)   DEFAULT 0,                      -- 是否挂起
  status          VARCHAR(16)  NOT NULL DEFAULT 'draft',
  metadata        JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  created_by      CHAR(36),
  updated_by      CHAR(36),
  deleted_at      DATETIME(3),
  deleted_by      CHAR(36),
  version         INTEGER     NOT NULL DEFAULT 1,
  _hash           CHAR(64),
  UNIQUE KEY uk_tenant_wf_code (tenant_id, workflow_code, workflow_version)
);
CREATE INDEX IF NOT EXISTS idx_meta_wf_tenant ON meta_workflow(tenant_id);
CREATE INDEX IF NOT EXISTS idx_meta_wf_entity ON meta_workflow(entity_id);

-- 工作流实例（运行时）
CREATE TABLE IF NOT EXISTS flow_workflow_instance (
  instance_id     CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  workflow_id     CHAR(36)     NOT NULL,
  workflow_version INTEGER,

  -- 业务关联
  entity_id       CHAR(36),
  biz_id          CHAR(36),
  biz_code        VARCHAR(128),
  biz_title       VARCHAR(255),

  -- 状态
  instance_status VARCHAR(32)  NOT NULL DEFAULT 'running',     -- running/completed/revoked/suspended/terminated/error
  current_node_id VARCHAR(64),
  current_task_ids JSON,

  -- 人员
  initiator_id    CHAR(36)     NOT NULL,
  initiator_dept_id CHAR(36),
  admin_user_ids  JSON,
  cc_user_ids     JSON,                                        -- 抄送人

  -- 时间
  started_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  ended_at        DATETIME(3),
  due_at          DATETIME(3),
  suspended_at    DATETIME(3),
  last_active_at  DATETIME(3),
  total_duration_ms BIGINT,

  -- 数据
  form_data       JSON,                                        -- 流程表单数据快照
  variables       JSON,                                        -- 流程变量
  context         JSON,                                        -- 运行上下文

  -- 结果
  final_decision  VARCHAR(16),                                 -- approved/rejected/revoked/terminated
  final_comment   VARCHAR(1024),
  completed_count INTEGER,
  rejected_count  INTEGER,

  metadata        JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)
);
CREATE INDEX IF NOT EXISTS idx_flow_inst_tenant    ON flow_workflow_instance(tenant_id);
CREATE INDEX IF NOT EXISTS idx_flow_inst_workflow  ON flow_workflow_instance(workflow_id);
CREATE INDEX IF NOT EXISTS idx_flow_inst_biz       ON flow_workflow_instance(entity_id, biz_id);
CREATE INDEX IF NOT EXISTS idx_flow_inst_status    ON flow_workflow_instance(instance_status);
CREATE INDEX IF NOT EXISTS idx_flow_inst_initiator ON flow_workflow_instance(initiator_id);

-- 工作流节点实例（运行时）
CREATE TABLE IF NOT EXISTS flow_node_instance (
  node_inst_id    CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  instance_id     CHAR(36)     NOT NULL,
  workflow_node_id VARCHAR(64) NOT NULL,

  node_type       VARCHAR(32)  NOT NULL,                        -- start/end/task/gateway/parallel/event/subprocess
  node_name       VARCHAR(255),

  -- 状态
  node_status     VARCHAR(32)  NOT NULL DEFAULT 'pending',     -- pending/running/completed/skipped/terminated/error
  entered_at      DATETIME(3),
  leaved_at       DATETIME(3),
  duration_ms     BIGINT,

  -- 候选人/处理人
  candidate_users JSON,
  candidate_roles JSON,
  assignee_user_id CHAR(36),
  claim_user_id   CHAR(36),

  -- 决策
  decision        VARCHAR(16),                                 -- approve/reject/transfer/skip/add_sign/back
  decision_comment VARCHAR(1024),
  decision_data   JSON,
  decision_at     DATETIME(3),
  decision_user_id CHAR(36),

  -- 加签/转交历史
  sign_chain      JSON,
  transfer_chain  JSON,

  -- 子流程
  sub_instance_id CHAR(36),

  -- 退回记录
  back_from_id    CHAR(36),                                    -- 从哪个节点退回
  back_target_id  CHAR(36),                                    -- 退回到哪个节点
  back_count      INTEGER      DEFAULT 0,

  metadata        JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)
);
CREATE INDEX IF NOT EXISTS idx_flow_node_instance ON flow_node_instance(instance_id);
CREATE INDEX IF NOT EXISTS idx_flow_node_status   ON flow_node_instance(node_status);
CREATE INDEX IF NOT EXISTS idx_flow_node_assignee ON flow_node_instance(assignee_user_id);

-- 审批意见/操作记录
CREATE TABLE IF NOT EXISTS flow_approval_record (
  record_id       CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  instance_id     CHAR(36)     NOT NULL,
  node_inst_id    CHAR(36),
  workflow_node_id VARCHAR(64),

  operator_id     CHAR(36)     NOT NULL,
  operator_name   VARCHAR(64),
  operator_dept   VARCHAR(255),

  action          VARCHAR(32)  NOT NULL,
  -- 操作类型:
  -- start/claim/unclaim/approve/reject/transfer/add_sign/
  -- minus_sign/back/revoke/suspend/resume/terminate/cc/comment
  -- auto_pass/timeout_pass/escalate/delegate

  decision        VARCHAR(16),                                 -- approve/reject/none
  comment         VARCHAR(2048),                               -- 审批意见
  attachments     JSON,                                        -- 附件
  extra_data      JSON,                                        -- 额外数据(表单字段变更等)

  -- 签名(防篡改审批)
  signature       VARCHAR(255),                                -- 电子签名/数字签名
  sign_cert       VARCHAR(1024),                               -- 签名证书
  sign_ts         DATETIME(3),                                 -- 签名时间戳(TSA)

  -- 关联
  target_user_ids JSON,                                        -- 转交给谁/加签给谁/抄送谁
  from_node_id    VARCHAR(64),
  to_node_id      VARCHAR(64),

  client_ip       VARCHAR(64),
  location        VARCHAR(255),
  device_info     JSON,

  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3)
);
CREATE INDEX IF NOT EXISTS idx_flow_record_instance ON flow_approval_record(instance_id);
CREATE INDEX IF NOT EXISTS idx_flow_record_operator ON flow_approval_record(operator_id);
CREATE INDEX IF NOT EXISTS idx_flow_record_action   ON flow_approval_record(action);
CREATE INDEX IF NOT EXISTS idx_flow_record_created  ON flow_approval_record(created_at);

-- ============================================================
-- 6. 业务规则 (Business Rule) —— 校验/计算/联动/触发
-- ============================================================
CREATE TABLE IF NOT EXISTS meta_rule (
  rule_id         CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  rule_code       VARCHAR(128) NOT NULL,
  rule_name       VARCHAR(255) NOT NULL,
  rule_category   VARCHAR(32)  NOT NULL,
  -- rule_category:
  --   validation  校验规则    (保存前校验: 金额必须 > 0)
  --   calculation 计算规则    (总价 = 单价 × 数量 × 折扣)
  --   linkage     联动规则    (A字段变化 → B字段值/选项变化)
  --   trigger     触发规则    (创建订单后 → 自动扣库存 + 生成发货单)
  --   derivation  派生规则    (客户等级 = 近12个月消费额映射)
  --   assignment  分配规则    (新线索按地区/行业自动分配销售)
  --   escalation  升级规则    (超时未审批自动升级至上级)

  entity_id       CHAR(36),                                    -- 绑定实体
  workflow_id     CHAR(36),                                    -- 绑定流程

  -- 生效范围
  rule_scope      VARCHAR(16)  DEFAULT 'global',               -- global/view/form/action/workflow_node
  scope_config    JSON,

  -- 触发时机
  trigger_event   VARCHAR(32),
  -- CREATE_BEFORE / CREATE_AFTER /
  -- UPDATE_BEFORE / UPDATE_AFTER / UPDATE_FIELD(field_name) /
  -- DELETE_BEFORE / DELETE_AFTER /
  -- LOAD_AFTER / EXPORT_BEFORE / IMPORT_AFTER /
  -- WORKFLOW_ENTER / WORKFLOW_LEAVE / WORKFLOW_TIMEOUT /
  -- SCHEDULE_CRON

  trigger_cron    VARCHAR(64),                                 -- 定时调度: cron表达式
  trigger_condition VARCHAR(2048),                             -- 触发条件表达式

  -- 规则优先级与互斥
  priority        INTEGER      DEFAULT 0,
  mutex_group     VARCHAR(64),                                 -- 互斥组（同组只执行优先级最高的）

  -- 规则体
  rule_body       JSON        NOT NULL,
  -- ┌──────────────────────────────────────────────────────┐
  // │ validation 规则体:
  // │ { checks: [ { field, type, params, message, severity: error/warn/info } ] }
  // │
  // │ calculation 规则体:
  // │ { target: "field_code", expr: "{{unit_price}} * {{quantity}} * (1 - {{discount}}/100)" }
  // │ 或 { targets: [], script: "return {a: x + 1, b: y * 2}" }
  // │
  // │ linkage 规则体:
  // │ {
  // │   watch_fields: ["province"],
  // │   effects: [
  // │     { target_field: "city", type: "filter_options",
  // │       params: { parent_field: "province" } },
  // │     { target_field: "manager", type: "set_value",
  // │       expr: "{{dept_manager_of_assignee}}" }
  // │   ]
  // │ }
  // │
  // │ trigger 规则体:
  // │ {
  // │   actions: [
  // │     { type: "update_related", target_entity, target_filter, updates },
  // │     { type: "create_record",   target_entity, data_expr },
  // │     { type: "call_api",        url, method, headers, body_template },
  // │     { type: "send_notification", channels, template, recipients },
  // │     { type: "start_workflow",  workflow_code, variables },
  // │     { type: "schedule_job",    delay_expr, payload }
  // │   ]
  // │ }
  -- └──────────────────────────────────────────────────────┘

  -- 失败处理
  failure_policy  VARCHAR(16)  DEFAULT 'log',                  -- abort/log/ignore/retry
  retry_count     INTEGER      DEFAULT 0,
  retry_interval  INTEGER      DEFAULT 1000,                   -- ms

  -- 版本/状态
  rule_version    INTEGER      NOT NULL DEFAULT 1,
  is_enabled      TINYINT(1)   DEFAULT 1,
  status          VARCHAR(16)  NOT NULL DEFAULT 'active',
  description     VARCHAR(1024),
  tags            JSON,
  metadata        JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  created_by      CHAR(36),
  updated_by      CHAR(36),
  version         INTEGER     NOT NULL DEFAULT 1,
  _hash           CHAR(64),
  UNIQUE KEY uk_tenant_rule_code (tenant_id, rule_code, rule_version)
);
CREATE INDEX IF NOT EXISTS idx_meta_rule_tenant   ON meta_rule(tenant_id);
CREATE INDEX IF NOT EXISTS idx_meta_rule_entity   ON meta_rule(entity_id);
CREATE INDEX IF NOT EXISTS idx_meta_rule_category ON meta_rule(rule_category);
CREATE INDEX IF NOT EXISTS idx_meta_rule_event    ON meta_rule(trigger_event);

-- ============================================================
-- 7. 通用业务数据表 (Universal Business Data Table)
--    —— 绝大多数实体用此表存储，无需建表
-- ============================================================
CREATE TABLE IF NOT EXISTS biz_data (
  biz_id          CHAR(36) PRIMARY KEY,                        -- UUIDv7
  tenant_id       CHAR(36)     NOT NULL,
  entity_id       CHAR(36)     NOT NULL,

  -- 系统字段
  biz_code        VARCHAR(128),                                -- 业务编码(自动生成/自定义)
  parent_biz_id   CHAR(36),                                    -- 父记录ID(树形结构)
  biz_level       INTEGER      DEFAULT 0,
  biz_path        VARCHAR(1024),                               -- 物化路径

  -- 状态
  biz_status      VARCHAR(32)  DEFAULT 'draft',                -- 业务状态(draft/approving/active/archived/...)
  workflow_status VARCHAR(32),                                 -- 审批状态(none/pending/approved/rejected)
  workflow_inst_id CHAR(36),

  -- 归属
  owner_user_id   CHAR(36),
  owner_dept_id   CHAR(36),
  assignee_user_id CHAR(36),
  collaborator_user_ids JSON,

  -- 扩展字段(通用)
  ext_str_01      VARCHAR(255),
  ext_str_02      VARCHAR(255),
  ext_str_03      VARCHAR(255),
  ext_str_04      VARCHAR(255),
  ext_str_05      VARCHAR(255),
  ext_str_06      VARCHAR(255),
  ext_str_07      VARCHAR(255),
  ext_str_08      VARCHAR(255),
  ext_str_09      VARCHAR(512),
  ext_str_10      VARCHAR(1024),
  ext_str_11      VARCHAR(2048),
  ext_str_12      TEXT,
  ext_text_01     MEDIUMTEXT,
  ext_text_02     LONGTEXT,
  ext_json_01     JSON,
  ext_json_02     JSON,
  ext_json_03     JSON,
  ext_json_04     JSON,
  ext_int_01      INTEGER,
  ext_int_02      INTEGER,
  ext_int_03      INTEGER,
  ext_int_04      BIGINT,
  ext_int_05      BIGINT,
  ext_dec_01      DECIMAL(30,10),
  ext_dec_02      DECIMAL(30,10),
  ext_dec_03      DECIMAL(30,10),
  ext_dec_04      DECIMAL(30,10),
  ext_dec_05      DECIMAL(30,10),
  ext_date_01     DATE,
  ext_date_02     DATE,
  ext_date_03     DATE,
  ext_datetime_01 DATETIME(3),
  ext_datetime_02 DATETIME(3),
  ext_datetime_03 DATETIME(3),
  ext_datetime_04 DATETIME(3),
  ext_bool_01     TINYINT(1),
  ext_bool_02     TINYINT(1),
  ext_bool_03     TINYINT(1),
  ext_bool_04     TINYINT(1),

  -- 自由字段存储(JSON全量，字段映射通过 meta_field.field_storage)
  dynamic_data    JSON,

  -- 审计追踪
  version         INTEGER     NOT NULL DEFAULT 1,
  changelog       JSON,                                        -- 最近变更摘要
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  created_by      CHAR(36),
  updated_by      CHAR(36),
  deleted_at      DATETIME(3),
  deleted_by      CHAR(36),
  _hash           CHAR(64)                                     -- 行内容SHA-256
);
CREATE INDEX IF NOT EXISTS idx_biz_tenant_entity  ON biz_data(tenant_id, entity_id);
CREATE INDEX IF NOT EXISTS idx_biz_owner          ON biz_data(tenant_id, owner_user_id);
CREATE INDEX IF NOT EXISTS idx_biz_dept           ON biz_data(tenant_id, owner_dept_id);
CREATE INDEX IF NOT EXISTS idx_biz_status         ON biz_data(tenant_id, biz_status);
CREATE INDEX IF NOT EXISTS idx_biz_wf             ON biz_data(workflow_inst_id);
CREATE INDEX IF NOT EXISTS idx_biz_parent         ON biz_data(parent_biz_id);
CREATE INDEX IF NOT EXISTS idx_biz_path           ON biz_data(biz_path(255));
CREATE INDEX IF NOT EXISTS idx_biz_code           ON biz_data(tenant_id, biz_code);
CREATE INDEX IF NOT EXISTS idx_biz_ext_str_01     ON biz_data(ext_str_01);
CREATE INDEX IF NOT EXISTS idx_biz_ext_str_02     ON biz_data(ext_str_02);
CREATE INDEX IF NOT EXISTS idx_biz_ext_int_01     ON biz_data(ext_int_01);
CREATE INDEX IF NOT EXISTS idx_biz_ext_dec_01     ON biz_data(ext_dec_01);
CREATE INDEX IF NOT EXISTS idx_biz_ext_date_01    ON biz_data(ext_date_01);
CREATE INDEX IF NOT EXISTS idx_biz_ext_datetime_01 ON biz_data(ext_datetime_01);

-- 版本历史（每次更新快照一份，用于数据血缘/回滚/审计）
CREATE TABLE IF NOT EXISTS biz_data_version (
  version_id      CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  biz_id          CHAR(36)     NOT NULL,
  entity_id       CHAR(36)     NOT NULL,
  version_number  INTEGER      NOT NULL,
  version_label   VARCHAR(64),
  is_major        TINYINT(1)   DEFAULT 0,
  change_summary  VARCHAR(512),
  change_type     VARCHAR(32),                                 -- create/update/delete/restore/rollback/import
  changed_fields  JSON,
  snapshot_before JSON,
  snapshot_after  JSON,
  diff_patch      JSON,                                        -- JSON Patch 格式
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  created_by      CHAR(36),
  comment         VARCHAR(1024)
);
CREATE INDEX IF NOT EXISTS idx_bizv_biz     ON biz_data_version(biz_id);
CREATE INDEX IF NOT EXISTS idx_bizv_version ON biz_data_version(biz_id, version_number);
CREATE INDEX IF NOT EXISTS idx_bizv_user    ON biz_data_version(created_by);

-- ============================================================
-- 8. 数据字典 (Data Dictionary)
-- ============================================================
CREATE TABLE IF NOT EXISTS meta_dictionary (
  dict_id         CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  dict_code       VARCHAR(128) NOT NULL,
  dict_name       VARCHAR(255) NOT NULL,
  dict_category   VARCHAR(64),                                 -- 系统/业务/地区/行业
  is_system       TINYINT(1)   DEFAULT 0,
  description     VARCHAR(1024),
  status          VARCHAR(16)  NOT NULL DEFAULT 'active',
  metadata        JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_tenant_dict_code (tenant_id, dict_code)
);

CREATE TABLE IF NOT EXISTS meta_dictionary_item (
  item_id         CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  dict_id         CHAR(36)     NOT NULL,
  item_value      VARCHAR(255) NOT NULL,
  item_label      VARCHAR(255) NOT NULL,
  parent_item_id  CHAR(36),
  sort_order      INTEGER      DEFAULT 0,
  item_level      INTEGER      DEFAULT 0,
  item_path       VARCHAR(1024),
  color           VARCHAR(16),
  icon            VARCHAR(255),
  tag_type        VARCHAR(16),
  ext_data        JSON,
  is_default      TINYINT(1)   DEFAULT 0,
  is_disabled     TINYINT(1)   DEFAULT 0,
  status          VARCHAR(16)  NOT NULL DEFAULT 'active',
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_tenant_dict_item (tenant_id, dict_id, item_value)
);
CREATE INDEX IF NOT EXISTS idx_meta_dict_item_dict   ON meta_dictionary_item(dict_id);
CREATE INDEX IF NOT EXISTS idx_meta_dict_item_parent ON meta_dictionary_item(parent_item_id);

-- ============================================================
-- 9. 编号规则 (Auto Numbering) —— 业务编码自动生成
-- ============================================================
CREATE TABLE IF NOT EXISTS meta_auto_number (
  number_id       CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  number_code     VARCHAR(128) NOT NULL,
  number_name     VARCHAR(255) NOT NULL,
  entity_id       CHAR(36),
  field_code      VARCHAR(64),

  -- 编号格式模板
  -- 变量支持:
  --   {{YYYY}} {{YY}} {{MM}} {{DD}} {{HH}} {{mm}} {{ss}}
  --   {{tenant_code}} {{dept_code}} {{user_code}}
  --   {{entity_code}} {{field:xxx}}
  --   {{SEQ:6}}  6位序号
  --   {{RAND:4}} 4位随机字母数字
  format_template VARCHAR(255) NOT NULL,                       -- 例: SO{{YYYY}}{{MM}}{{SEQ:6}}

  -- 序号策略
  seq_reset_cycle VARCHAR(16)  DEFAULT 'none',                -- none/year/month/day
  seq_padding     TINYINT(1)   DEFAULT 1,                     -- 是否补零
  seq_start       INTEGER      DEFAULT 1,
  seq_step        INTEGER      DEFAULT 1,
  seq_current     INTEGER      DEFAULT 0,
  seq_max         INTEGER,                                     -- 最大序号(溢出处理)

  -- 幂等/并发控制
  is_lock_enabled TINYINT(1)   DEFAULT 1,                     -- 是否启用分布式锁

  status          VARCHAR(16)  NOT NULL DEFAULT 'active',
  metadata        JSON,
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_tenant_number_code (tenant_id, number_code)
);

-- 序号分配日志（防止重复/跳号审计）
CREATE TABLE IF NOT EXISTS meta_auto_number_log (
  log_id          CHAR(36) PRIMARY KEY,
  tenant_id       CHAR(36)     NOT NULL,
  number_id       CHAR(36)     NOT NULL,
  seq_value       INTEGER      NOT NULL,
  reset_key       VARCHAR(64),                                 -- 重置周期键: 2026 / 2026-08 / 2026-08-26
  generated_code  VARCHAR(255) NOT NULL,
  used_by         CHAR(36),                                    -- 使用的记录ID
  used_for_entity CHAR(36),
  created_at      DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3)
);
CREATE INDEX IF NOT EXISTS idx_meta_num_log_num ON meta_auto_number_log(number_id, reset_key);
`;

/**
 * 行业包定义 —— 用于快速融合不同行业
 */
const IndustryPackages = {
  // 通用基础包（所有行业默认安装）
  common: {
    package_code: 'common',
    package_name: '通用基础包',
    entities: [
      { code: 'employee',    name: '员工档案',   category: 'master' },
      { code: 'department',  name: '部门档案',   category: 'master' },
      { code: 'position',    name: '岗位档案',   category: 'master' },
      { code: 'project',     name: '项目',       category: 'transaction' },
      { code: 'task',        name: '任务',       category: 'transaction' },
      { code: 'notice',      name: '公告通知',   category: 'transaction' },
      { code: 'document',    name: '文档中心',   category: 'master' },
      { code: 'meeting',     name: '会议管理',   category: 'transaction' },
      { code: 'asset',       name: '资产档案',   category: 'master' },
      { code: 'contract',    name: '合同管理',   category: 'transaction' },
      { code: 'supplier',    name: '供应商档案', category: 'master' },
      { code: 'customer',    name: '客户档案',   category: 'master' },
    ],
  },
  // 政务
  gov: {
    package_code: 'gov',
    package_name: '政务服务包',
    entities: [
      { code: 'gov_org',         name: '机构编制' },
      { code: 'gov_cadre',       name: '干部档案' },
      { code: 'gov_document',    name: '公文管理' },
      { code: 'gov_meeting_min', name: '会议纪要' },
      { code: 'gov_approval',    name: '行政审批' },
      { code: 'gov_complaint',   name: '投诉举报' },
      { code: 'gov_event',       name: '应急事件' },
      { code: 'gov_assessment',  name: '绩效考核' },
      { code: 'gov_training',    name: '培训管理' },
      { code: 'gov_budget',      name: '预算管理' },
      { code: 'gov_procurement', name: '采购管理' },
    ],
    workflows: [
      { code: 'gov_doc_issue',   name: '公文签发流程' },
      { code: 'gov_approval',    name: '行政审批流程' },
      { code: 'gov_budget_adj',  name: '预算调整流程' },
    ],
  },
  // 金融
  finance: {
    package_code: 'finance',
    package_name: '金融业务包',
    entities: [
      { code: 'fin_product',    name: '金融产品' },
      { code: 'fin_customer',   name: '客户(CRM)' },
      { code: 'fin_account',    name: '账户管理' },
      { code: 'fin_loan',       name: '贷款业务' },
      { code: 'fin_guarantee',  name: '担保管理' },
      { code: 'fin_credit',     name: '授信审批' },
      { code: 'fin_risk',       name: '风险预警' },
      { code: 'fin_settlement', name: '资金清算' },
      { code: 'fin_reconcile',  name: '对账管理' },
      { code: 'fin_compliance', name: '合规检查' },
    ],
  },
  // 医疗
  medical: {
    package_code: 'medical',
    package_name: '医疗健康包',
    entities: [
      { code: 'med_patient',    name: '患者档案' },
      { code: 'med_doctor',     name: '医生排班' },
      { code: 'med_appointment',name: '预约挂号' },
      { code: 'med_visit',      name: '门诊就诊' },
      { code: 'med_order',      name: '医嘱处方' },
      { code: 'med_drug',       name: '药品库存' },
      { code: 'med_exam',       name: '检查检验' },
      { code: 'med_surgery',    name: '手术管理' },
      { code: 'med_ward',       name: '住院病房' },
      { code: 'med_insurance',  name: '医保结算' },
    ],
  },
  // 制造
  manufacturing: {
    package_code: 'manufacturing',
    package_name: '智能制造包',
    entities: [
      { code: 'mfg_product',    name: '产品BOM' },
      { code: 'mfg_material',   name: '物料档案' },
      { code: 'mfg_workorder',  name: '生产工单' },
      { code: 'mfg_process',    name: '工艺路线' },
      { code: 'mfg_equipment',  name: '设备台账' },
      { code: 'mfg_qc',         name: '质量管理' },
      { code: 'mfg_wip',        name: '在制品追踪' },
      { code: 'mfg_schedule',   name: '排产计划' },
      { code: 'mfg_mro',        name: '设备维保' },
      { code: 'mfg_scada',      name: 'SCADA采集' },
    ],
  },
  // 教育
  education: {
    package_code: 'education',
    package_name: '智慧教育包',
    entities: [
      { code: 'edu_student',    name: '学生档案' },
      { code: 'edu_teacher',    name: '教师档案' },
      { code: 'edu_class',      name: '班级管理' },
      { code: 'edu_course',     name: '课程体系' },
      { code: 'edu_schedule',   name: '课表排课' },
      { code: 'edu_exam',       name: '考试管理' },
      { code: 'edu_score',      name: '成绩分析' },
      { code: 'edu_attendance', name: '考勤管理' },
      { code: 'edu_elective',   name: '选课管理' },
      { code: 'edu_homework',   name: '作业管理' },
    ],
  },
  // 零售
  retail: {
    package_code: 'retail',
    package_name: '智慧零售包',
    entities: [
      { code: 'rtl_product',    name: '商品档案' },
      { code: 'rtl_category',   name: '商品分类' },
      { code: 'rtl_sku',        name: 'SKU管理' },
      { code: 'rtl_store',      name: '门店档案' },
      { code: 'rtl_stock',      name: '库存管理' },
      { code: 'rtl_order',      name: '销售订单' },
      { code: 'rtl_pos',        name: 'POS流水' },
      { code: 'rtl_promotion',  name: '促销活动' },
      { code: 'rtl_vip',        name: '会员管理' },
      { code: 'rtl_refund',     name: '退换货' },
    ],
  },
};

module.exports = {
  MetaDDL,
  IndustryPackages,
};
