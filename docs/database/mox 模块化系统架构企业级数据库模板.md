# mox 模块化系统架构企业级数据库通用母版（mox_sys Universal Template）

> 目标：一套**mox 模块化系统架构维度覆盖、归一化到 BCNF、企业级、人人可用、全方面可扩展**的关系数据库模板。
> 配套 DDL：`mox_sys/mox_sys-universal-template.sql`（本文件为其设计说明与归一化规范）。
> 权威契约：`mox_sys/module-contract.md`；跨库兼容：`mox_sys/cross-database.md`。

---

## 0. 一句话定位

旧 `mox_v3` 基线解决了"把 79 张乱表分层"的问题；本母版在其之上解决三件事：

1. **归一化（Normalization）**：所有短码、配置、审计、授权只定义一次，消灭散落 CHECK 约束、每表大 JSON、重复快照表。
2. **mox 模块化系统架构维度（Full-Dimension）**：身份 / 租户 / 组织 / 成员 / 授权 / 菜单 / 字典 / 配置 / 审计 / 事件总线 / 文件 / 通知 / 调度 / 特性开关 / 连接器 / 国际化 / 计量套餐 / 知识图谱 / 扩展钩子——19 个维度包一次就位。
3. **全方面可扩展（Extensible）**：每个维度是一个可独立安装/禁用的模块；业务零改表即可通过 EAV 自定义字段、Webhook、事件 outbox、模块注册表扩展。

---

## 1. mox 模块化系统架构现状分析（优化前 → 优化后）

| 维度 | 旧痛点 | 本母版做法 |
|---|---|---|
| 代码字典 | 各表 `status CHAR(1)` + 散落 `CHECK` 约束，魔法串难统一 | **`sys_enum_type` / `sys_enum_item`** 中心化，所有短码只定义一次（4NF） |
| 配置 | `settings JSON` 大字段，无法过滤/授权 | **`sys_setting`** 按 `G/T/E/U` scope 归一化为行；密文走 `secret_ref` |
| 审计 | 每业务表复制 `*_record` / `*_link_record` 快照 | **`sys_audit_event` + `sys_audit_change`** 统一事件头+逐字段，hash 链防篡改 |
| 授权 | 菜单=权限、角色塞数据范围、字符串外键伪装强一致 | **RBAC + ABAC + ReBAC** 三正交模型独立建表，互不混入 |
| 身份 | 自增/nanoid/UUID_SHORT 混用 | **UUID v7 `BINARY(16)`** 全局统一，应用层生成 |
| 多身份 | SSO/MFA 无归一承载 | `sys_user_identity` / `sys_user_mfa` / `sys_user_session` |
| 通知 | 模板/消息/偏好混在一列 | 三实体独立 |
| 调度 | 强绑 Quartz `QRTZ_*` | 可移植 `sys_job` / `sys_job_run`，与协议解耦 |
| 连接器 | FTP/SFTP/HTTP/S3 各一套 | `endpoint` / `credential` / `call` 三实体，密文不进调用日志 |
| 国际化 | 散落各表 locale 列 | `sys_i18n_bundle` / `sys_i18n_message` 按 locale 归一化行 |
| 计量 | 无 SaaS 计费支撑 | `sys_plan` / `sys_subscription` / `sys_usage_meter` |
| 扩展 | 加字段就改表 | `sys_custom_field_schema` / `sys_custom_field_value` + `sys_webhook` |
| 图谱 | 仅关系基座 | 保留 `mox_sys_*` 完整 10 表权威包 |

> 关键结论：旧库"79 张表"的复杂度，本质是**同一类概念被复制了 N 次**。归一化后，平台能力收敛为**单一权威母版** `mox_sys-universal-template.sql`（**56 张标准化表**：19 维度包系统内核 + P17 模块注册与知识图谱），**一键安装**即可。业务模块只装自己独有的表，不再反向复制系统能力。全目录只有这一个 DDL 真相，不存在并行/重复定义。

### 1.4 硬红线：类型字段一律显式字符串，禁止数值型

为达到「最通用、最明确、跨库可移植」，本母版对所有**类别性字段**（状态/类型/种类/范围/效果/角色/代码/二值）执行以下铁律：

- **禁止数值型魔法值**：`status` / `kind` / `type` / `scope` / `effect` / `code` 一律 `VARCHAR`（如 `VARCHAR(24)` / `ascii_bin`），取值为可读英文短码（`active` / `allow` / `api` / `global` …）。**绝不用 `0/1/2`、`TINYINT` 状态位、或数值型 `ENUM`**。
- **二值用 `Y/N`**：布尔语义统一 `CHAR(1)` 的 `Y`/`N`（如 `is_secret` / `enabled`），不用 `TINYINT(1)`。
- **A 级 · 封闭值域用 DDL `CHECK` 枚举**：仅用于**永不加项**的封闭值域（`effect allow/deny`、`scope_kind G/T/E/U`、`flag.percent ≤100`、P17 全部状态集），DDL 直接写 `CHECK (col IN (...))` + `COMMENT 'a/b/c'`，由 DB 强制、跨库保真。
- **B 级 · 可演进值域用字典治理**：状态/类型/渠道等**可能加项**的类别字段，用 `COMMENT 'a/b/c'` 自描述并**必须**登记 `sys_enum_type` / `sys_enum_item`（4NF），作为应用层运行时校验与 UI 下拉源；**不写死 CHECK**——否则改枚举值需 `ALTER TABLE` 锁表，违背字典热维护原则。
- **度量/序号允许数值**：`row_version`、`size_bytes`、`quantity`、`percent`、`confidence`、`priority`、`sort_no`、`version_no`、`execution_ms` 等是「量」不是「类」，保留 `BIGINT` / `DECIMAL` / `TINYINT`，不受本红线约束。
- 判定一句话：**将来可能加项 → B 级字典；值域永锁死 → A 级 CHECK**。二者互补而非替代；文档凡言“每个类别字段都带 CHECK”一律以本两级规则为准。

> **精算审计结论（复核于 2026-09）**：母版 56 表 653 列，全部类别字段为显式字符串、**零数值型类别**（红线达标）。`CHECK` 共 14 条，均属 A 级封闭值域：`sys_policy.effect` / `sys_setting.scope_kind` / `sys_feature_flag_rollout.percent` 3 条 + P17 状态/基数 11 条；其余可演进类别列（B 级）以 `COMMENT 'a/b/c'` 自描述，取值登记于字典种子 `mox_sys-seed.sql`。

---

## 2. 归一化规范（BCNF 基线）

本母版对每张表强制以下规则；违反即视为不合格迁移。

### 2.1 范式检查清单
- **1NF**：无重复列组、每列原子、每张表有主键、无无主键关系表。
- **2NF**：无复合主键的部分依赖——凡是"多对多/一对多"关联（角色-权限、用户-角色、文件-对象、通知-偏好）都拆成独立关联实体，并以 `(owner, target)` 为唯一键。
- **3NF**：无传递依赖——字典值（`sys_enum_*`）、配置（`sys_setting`）、资源（`sys_resource`）一律外提为独立实体，业务表只存 ID 或短码。
- **BCNF**：每个决定因子都是候选键——短码含义由 `sys_enum_type` 唯一定义，禁止在应用层各自硬编码；配置按 scope 外提，不让"配置"成为某张业务表的列。
- **4NF**：消除多值依赖——用户多身份、用户多 MFA、通知多偏好、国际化多语言，全部拆独立表，不塞 JSON 数组。
- **5NF**：ReBAC 关系投影为 `(subject, relation, object)` 三元组（`sys_relation`），不把"能看/能改"算作用户事实。

### 2.2 全局基线约定
| 项 | 规则 |
|---|---|
| 主键 | `BINARY(16)` UUID v7，应用层生成；禁止自增 / `UUID_SHORT()` / 触发器 |
| 租户键 | 每个租户业务表必有 `tenant_id`；`tenant_id IS NULL` 仅表示平台级共享资源（scope=G） |
| 隔离三键 | `tenant_id`（租户） / `enterprise_id`（企业主体） / `org_unit_id`（组织节点）**不混用**，禁用含义不明的 `org_id` |
| 时间 | `DATETIME(3)` UTC；API 统一 RFC3339；`deleted_at` 软删除 |
| 状态 | **英文短码** `VARCHAR(24)` 引用 `sys_enum_type[status]`；二值字段 `Y/N` |
| 金额 | 只用 `DECIMAL`，禁止 `double` / `float` |
| JSON | 只承载可演进配置/非过滤属性；可过滤、可排序、可授权的属性必须提升为列 |
| 密文 | 密码/凭证/密钥不落地明文，存 `password_hash VARBINARY` 或 `secret_ref` 引用 |
| 版本 | `row_version BIGINT UNSIGNED` 乐观锁；不可变记录（审计/凭证签名/迁移）禁止 UPDATE 覆盖 |
| 外键 | 默认无跨服务物理外键；单体可在发布期追加；跨模块一致性由 outbox + 事件保证 |

### 2.3 短码字典（归一化核心）
所有 `status` / `mode` / `type` / `kind` / `scope` 短码，**必须**先在 `sys_enum_type` 注册，再在 `sys_enum_item` 列举取值。应用层只读字典，不写死魔法串。这样既统一语义，又支持运维在界面上维护，且不会因改 CHECK 约束而锁表。

---

## 3. mox 模块化系统架构维度包地图（19 包）

```
P01 身份    sys_user / sys_user_identity / sys_user_mfa / sys_user_session
P02 租户组织 sys_tenant / sys_enterprise / sys_org_unit
P03 成员    sys_tenant_member / sys_org_member
P04 授权    sys_resource / sys_permission / sys_role / sys_role_permission
            / sys_user_role / sys_policy(ABAC) / sys_relation(ReBAC)
P05 菜单    sys_menu
P06 字典    sys_enum_type / sys_enum_item            ← 归一化核心
P07 配置    sys_setting
P08 审计    sys_audit_event / sys_audit_change
P09 事件    sys_outbox_event / sys_inbox_event / sys_idempotency_key
P10 文件    sys_file_object / sys_file_link
P11 通知    sys_notification_template / sys_notification_message / sys_notification_pref
P12 调度    sys_job / sys_job_run
P13 开关    sys_feature_flag / sys_feature_flag_rollout
P14 连接器  sys_connector_endpoint / sys_connector_credential / sys_connector_call
P15 国际化  sys_i18n_bundle / sys_i18n_message
P16 计量    sys_plan / sys_subscription / sys_usage_meter
P17 图谱    mox_sys_module / *_module_version / *_module_dependency / *_schema_version
            / *_graph / *_node_type / *_relation_type / *_node / *_edge / *_evidence
P18 扩展    sys_custom_field_schema / sys_custom_field_value / sys_webhook
P19 视图    v_tenant_user / v_user_effective_permission
```

每个包遵循同一套生命周期列（`id/tenant_id/created_at/updated_at/deleted_at/row_version/created_at/updated_by/deleted_by`），高吞吐表额外具备 `occurred_at / trace_id / status / retention` 语义。

---

## 4. 企业级能力

- **多租户隔离**：`L` 逻辑 / `P` 物理 / `H` 混合三模式；`tenant_id` 永不省略，repository 自动注入租户谓词。
- **三模型授权**：RBAC（角色-权限）、ABAC（策略条件）、ReBAC（关系三元组）正交并存，访问判定顺序：租户边界 → 成员状态 → 组织数据范围 → 角色 → 资源关系 → 字段脱敏，任一层拒绝不可被下层放宽。
- **可审计**：统一事件头 + 逐字段变更 + hash 链，满足合规追溯；不复制业务快照。
- **最终一致**：outbox 同库提交、消费端幂等（inbox + idempotency key），跨模块不用分布式事务。
- **可观测**：文件/连接器/作业/通知均带 `trace_id` 与状态，可接入 APM；大表按时间归档。
- **可计费**：套餐/订阅/用量三表支撑 SaaS 计量与分级。
- **安全**：密码 Argon2id/bcrypt 摘要、凭证/密钥 `secret_ref` 外置、会话 `token_hash` 不存明文。
- **可移植**：仅用 ANSI 语法 + 参数绑定；`BINARY(16)/JSON/生成列/分区` 为 capability，经 adapter 接入 MySQL/PostgreSQL/SQLite/图库（见 `cross-database.md`）。

---

## 5. 全方面可扩展

### 5.1 加一个业务模块（不改核心表）
1. 在 `mox_sys_module` 注册 `module_code` + SemVer + `requires`（依赖 mox_sys/iam/…）。
2. 业务表前缀用 `<domain>_`，自带 `tenant_id` 与生命周期列。
3. 需要系统能力时只引用稳定 ID / 发事件 / 写图谱关系，绝不复制 `sys_user` 等核心表。
4. 增量 migration 写 `mox_sys_schema_version`，遵循 expand→backfill→switch→contract。

### 5.2 不改表加字段（EAV）
通过 `sys_custom_field_schema` + `sys_custom_field_value` 给任意 `entity_type` 挂载自定义字段；`field_type=enum` 时复用 `sys_enum_type`。高频查询字段建议提升为生成列或物化视图。

### 5.3 事件驱动扩展（Webhook / outbox）
业务写事务同时写 `sys_outbox_event`；下游通过 `sys_webhook` 订阅事件类型，或消费端用 `sys_inbox_event` 幂等落库。外部系统无需轮询。

### 5.4 模块依赖治理
`mox_sys_module_dependency` 记录版本区间；安装器校验依赖与 `requires_capability`（如某模块需要图库，缺失则拒绝静默降级）。

---

## 6. 迁移与兼容

- **一键安装（唯一入口）**：直接执行 `mox_sys-universal-template.sql`（它已含 `CREATE DATABASE IF NOT EXISTS mox_v3; USE mox_v3;` 并定义全部 56 张表，含 P17 模块注册与知识图谱）。同目录 `install.ps1` / `install.sh` 一行调用，无需任何顺序编排。
- 已有 mox_v3（baseline 现状库）想对齐到本归一化母版：先按 `mox-v3.0-migration-plan.md` 做迁移（baseline 是“现状落库”，本母版是“归一化目标”，二者不是安装关系），再用本文件补齐缺失表；不要与本文件叠加执行同名旧表。
- 旧库迁移见 `mox-v3.0-migration-plan.md`：先盘点 `information_schema`、建映射表、先迁全局用户再租户/组织/成员/权限、双写校验后切读，禁止生产直接 `DROP` / 关外键导入。
- 破坏性变更必须：备份 + 回滚脚本 + 数据校验 + 停机/在线说明。

---

## 7. 验收清单（提交新模块必须通过）

- [ ] 所有表满足 1NF~BCNF，无无主键关系表
- [ ] 所有短码已在 `sys_enum_type`/`sys_enum_item` 注册，无散落 `CHECK`
- [ ] 可过滤字段均为列，未塞进 JSON
- [ ] 租户业务表必有 `tenant_id`，无 `org_id` 混用
- [ ] 密文经 `secret_ref` 外置，无明文落地
- [ ] 跨模块仅引用稳定 ID / 事件，无直接改别模块表
- [ ] 通过 MySQL + SQLite migration smoke test（复杂查询加 PostgreSQL 矩阵）
- [ ] 通过租户泄露 / 幂等 / 迁移回滚检查

---

> 本母版随仓库 MIT 发布；提交须同时提供 DDL、迁移、验证 SQL、模块契约、示例数据与变更日志；**禁止**提交真实租户数据、密钥、连接串或内部生产配置。
