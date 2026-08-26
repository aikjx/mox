# MOX 元架构 — 数据库设计规范 v1.0

> 本规范是所有数据库设计的强制标准，适用于 PostgreSQL 15+ / MySQL 8+ / SQLite 3。
> 任何表结构变更必须遵循本规范，并通过架构约束 CI 测试。

---

## 一、命名规范

### 1.1 表命名

| 规则 | 规范 | 示例 |
|------|------|------|
| **前缀分层** | 系统表 `sys_` / 元数据表 `meta_` / 流程表 `wf_` / 专家联盟表 `ea_` / 业务表 `biz_` | `sys_user`, `meta_entity`, `wf_instance`, `ea_expert`, `biz_order` |
| **命名风格** | 全小写 + 下划线分隔（snake_case） | `sys_user_role` ✅ / `SysUserRole` ❌ / `sysUserRole` ❌ |
| **单数形式** | 表名用单数名词 | `sys_user` ✅ / `sys_users` ❌ |
| **长度限制** | 表名 ≤ 48 字符（Oracle兼容） | — |
| **关联表命名** | 多对多中间表 = `主表_从表` 或 `主表_relation` | `sys_user_role`, `meta_relation` |
| **业务表命名** | `biz_` + 实体编码（来自 meta_entity.entity_code） | `biz_customer`, `biz_order`, `biz_product` |

### 1.2 字段命名

| 规则 | 规范 | 示例 |
|------|------|------|
| **命名风格** | 全小写 + 下划线分隔 | `created_at` ✅ / `createdAt` ❌ |
| **主键** | 统一 `id`，类型 VARCHAR(64)，格式 `前缀_UUID` | `usr_a1b2c3d4e5f6` |
| **外键** | `关联表单数_id` | `user_id`, `dept_id`, `tenant_id` |
| **时间字段** | 后缀 `_at`（时间点）/ `_date`（日期） | `created_at`, `updated_at`, `expire_date` |
| **布尔字段** | 前缀 `is_` / `has_` / `can_` | `is_active`, `has_child`, `can_edit` |
| **状态字段** | 后缀 `_status` | `user_status`, `order_status` |
| **类型字段** | 后缀 `_type` / `_kind` | `user_type`, `channel_kind` |
| **金额字段** | 后缀 `_amount` / `_price` / `_fee`，类型 DECIMAL | `total_amount`, `unit_price` |
| **数量字段** | 后缀 `_count` / `_num` / `_qty` | `view_count`, `item_qty` |
| **JSON字段** | 后缀 `_data` / `_config` / `_extra` / `_props` | `extra_data`, `config_json` |
| **编码字段** | 后缀 `_code`（业务唯一编码） | `tenant_code`, `role_code`, `dict_code` |
| **路径字段** | 后缀 `_path`（物化路径/URL/文件路径） | `dept_path`, `file_path` |
| **排序字段** | `sort_order`（统一命名，类型 INT） | `sort_order` |
| **保留字规避** | 禁止使用 SQL 保留字（user/order/group/desc/level等），必须加前缀 | `sys_user` ✅ / `user` ❌ |

### 1.3 索引命名

| 索引类型 | 命名规范 | 示例 |
|----------|----------|------|
| 主键索引 | `pk_表名`（自动生成，不手动命名） | `pk_sys_user` |
| 唯一索引 | `uk_表名_字段1_字段2` | `uk_sys_user_tenant_username` |
| 普通索引 | `idx_表名_字段1_字段2` | `idx_sys_user_tenant_dept` |
| 联合索引 | `idx_表名_字段1_字段2_字段3`（按查询频率排序） | `idx_wf_task_tenant_assignee_status` |
| 全文索引 | `ft_表名_字段` | `ft_ea_expert_description` |
| 向量索引 | `vec_表名_字段`（pgvector ivfflat/hnsw） | `vec_ea_expert_embedding` |
| 分区索引 | 同普通索引，分区键自动包含 | — |

---

## 二、字段类型规范

### 2.1 通用类型映射

| 业务含义 | PostgreSQL | MySQL | SQLite | 说明 |
|----------|-----------|-------|--------|------|
| 主键/ID | VARCHAR(64) | VARCHAR(64) | TEXT | 前缀+UUID，不用自增ID |
| 短字符串(≤64) | VARCHAR(64) | VARCHAR(64) | TEXT | 编码/名称/类型 |
| 中字符串(≤255) | VARCHAR(255) | VARCHAR(255) | TEXT | 标题/描述/路径 |
| 长文本 | TEXT | TEXT | TEXT | 内容/备注/JSON |
| 布尔 | BOOLEAN | TINYINT(1) | INTEGER | is_/has_/can_前缀 |
| 整数(小) | SMALLINT | SMALLINT | INTEGER | 状态/类型枚举 |
| 整数(中) | INTEGER | INT | INTEGER | 数量/排序/版本 |
| 整数(大) | BIGINT | BIGINT | INTEGER | 金额(分)/时间戳 |
| 金额 | DECIMAL(18,6) | DECIMAL(18,6) | REAL | 统一精度，不用FLOAT |
| 百分比 | DECIMAL(5,2) | DECIMAL(5,2) | REAL | 0.00-100.00 |
| 日期 | DATE | DATE | TEXT | 纯日期(YYYY-MM-DD) |
| 时间 | TIMESTAMP | DATETIME | TEXT | 带时区，统一UTC |
| JSON | JSONB | JSON | TEXT | PG用JSONB(支持索引) |
| 向量 | vector(1536) | — | — | pgvector扩展 |
| 枚举 | VARCHAR(32)+CHECK | VARCHAR(32) | TEXT | 不用ENUM类型(难扩展) |

### 2.2 公共字段规范（所有表必须包含）

```sql
-- 以下字段是所有表的标准公共字段，通过元数据引擎自动注入
id              VARCHAR(64)  PRIMARY KEY,   -- 全局唯一ID(前缀+UUID)
tenant_id       VARCHAR(64)  NOT NULL,       -- 租户ID(多租户隔离)
created_by      VARCHAR(64)  NOT NULL,       -- 创建人ID
created_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,  -- 创建时间(UTC)
updated_by      VARCHAR(64),                  -- 更新人ID
updated_at      TIMESTAMP,                    -- 更新时间(UTC)
deleted_at      TIMESTAMP,                    -- 软删除时间(NULL=未删除)
version         INT          DEFAULT 0         -- 乐观锁版本号
```

**公共字段规则**：
- 所有业务表必须包含以上8个公共字段
- `deleted_at` 软删除：查询自动加 `WHERE deleted_at IS NULL`
- `version` 乐观锁：更新时 `WHERE version = ?`，成功后 `version + 1`
- 所有时间字段统一 UTC 存储，应用层转换时区
- `tenant_id` 必须建联合索引（与高频查询字段组合）

---

## 三、索引设计规范

### 3.1 索引设计原则

| 原则 | 说明 |
|------|------|
| **查询驱动** | 索引只为查询服务，不为"可能会用到"建索引 |
| **联合优先** | 多条件查询优先建联合索引，而非多个单列索引 |
| **最左前缀** | 联合索引字段顺序按查询频率/区分度排序，区分度高的放前面 |
| **覆盖索引** | 高频查询尽量建覆盖索引（INCLUDE字段），避免回表 |
| **索引数量** | 单表索引 ≤ 5个（含主键），写入密集表 ≤ 3个 |
| **避免冗余** | 禁止建已有联合索引最左前缀的单列索引 |
| **字符串索引** | 长字符串建前缀索引（如 VARCHAR(512) 建前64字符） |

### 3.2 必须建索引的字段

| 字段类型 | 是否必须建索引 | 说明 |
|----------|---------------|------|
| 主键 | ✅ 自动 | PRIMARY KEY |
| 外键 | ✅ 必须 | 所有 `_id` 后缀字段 |
| `tenant_id` | ✅ 必须 | 多租户隔离，与高频字段建联合索引 |
| `deleted_at` | ⚠️ 建议 | 软删除查询，与 tenant_id 建联合索引 |
| 状态字段 | ⚠️ 建议 | `status` 低区分度，不单独建，与高频字段建联合索引 |
| 时间字段 | ⚠️ 建议 | `created_at` 范围查询，建BRIN或BTREE |
| 编码字段 | ✅ 必须 | `_code` 后缀，业务唯一编码建唯一索引 |
| JSON字段 | ❌ 不建 | PG用JSONB表达式索引，MySQL不建 |

### 3.3 索引类型选择

| 场景 | PostgreSQL | MySQL | 说明 |
|------|-----------|-------|------|
| 等值/范围查询 | BTREE | BTREE | 默认，最通用 |
| 多值/数组 | GIN | — | JSONB/数组包含查询 |
| 全文搜索 | GIN(to_tsvector) | FULLTEXT | 全文检索 |
| 空间数据 | GiST/SP-GiST | SPATIAL | GIS坐标 |
| 向量相似度 | ivfflat/hnsw | — | pgvector RAG检索 |
| 大表时间范围 | BRIN | — | 按时间排序的大表，体积极小 |

---

## 四、分区设计规范

### 4.1 分区策略

| 表类型 | 分区方式 | 分区键 | 分区粒度 | 说明 |
|--------|----------|--------|----------|------|
| 审计日志 | RANGE | `created_at` | 按月 | `sys_audit_log`，超大数据量 |
| 流程实例 | RANGE | `created_at` | 按季 | `wf_instance`，历史数据归档 |
| 专家执行 | RANGE | `created_at` | 按月 | `ea_expert_execution`，高频写入 |
| 业务流水 | RANGE | `created_at` | 按月 | `biz_order` 等流水表 |
| 租户数据 | LIST | `tenant_id` | 按租户 | 超大租户独立分区(L2隔离) |

### 4.2 分区管理

- **自动创建**：通过 pg_partman 或定时任务自动创建未来3个月分区
- **自动归档**：超过保留期的分区自动归档到冷存储（对象存储）
- **保留策略**：审计日志保留2年，流程实例保留5年，业务流水按行业合规要求
- **分区裁剪**：所有查询必须包含分区键，确保分区裁剪生效

---

## 五、安全设计规范

### 5.1 数据加密

| 数据类型 | 加密方式 | 说明 |
|----------|----------|------|
| 密码 | bcrypt/argon2 | 单向哈希，不可逆，`password_hash` |
| 敏感字段 | AES-256-GCM | 身份证/手机号/银行卡，应用层加密 |
| 传输层 | TLS 1.3 | 所有数据库连接强制TLS |
| 备份加密 | AES-256 | 备份文件加密存储 |
| 列级加密 | pgcrypto | PG可选，敏感字段透明加密 |

### 5.2 访问控制

| 层级 | 控制方式 | 说明 |
|------|----------|------|
| 网络层 | VPC/安全组 | 数据库仅内网访问，不暴露公网 |
| 账号层 | 最小权限 | 应用账号只有SELECT/INSERT/UPDATE，DDL用独立账号 |
| 租户层 | tenant_id过滤 | 所有查询自动注入租户条件，应用层不可绕过 |
| 行级安全 | RLS (PG) | PostgreSQL Row Level Security，数据库层强制租户隔离 |
| 列级权限 | GRANT COLUMN | 敏感字段仅授权角色可查询 |
| 审计层 | 全量审计 | 所有DDL/DCL/敏感DML记录审计日志 |

### 5.3 多租户隔离三档

| 级别 | 实现 | 数据安全 | 性能隔离 | 成本 | 适用 |
|------|------|----------|----------|------|------|
| L1 逻辑隔离 | tenant_id字段 + RLS | 中 | 共享 | 低 | 中小企业/SaaS标准版 |
| L2 Schema隔离 | 每租户独立Schema | 高 | 共享连接池 | 中 | 中大型/SaaS专业版 |
| L3 集群隔离 | 每租户独立数据库集群 | 极高 | 完全隔离 | 高 | 金融/政府/大型企业 |

---

## 六、性能设计规范

### 6.1 查询性能

| 规则 | 说明 |
|------|------|
| **避免SELECT *** | 只查需要的字段，减少网络传输和内存 |
| **分页优化** | 深分页用游标分页（WHERE id > ? LIMIT N），不用OFFSET |
| **N+1查询** | 列表页用批量查询（IN）或JOIN，禁止循环查单条 |
| **大表COUNT** | 用估算计数（pg_stat_user_tables）或计数器表，不用COUNT(*)全表 |
| **超时控制** | 所有查询设置statement_timeout（默认30s，报表5min） |
| **慢查询监控** | log_min_duration_statement = 1000ms，自动记录慢查询 |

### 6.2 写入性能

| 规则 | 说明 |
|------|------|
| **批量写入** | 批量INSERT（VALUES (...), (...)），单批≤1000行 |
| **异步写入** | 审计/日志/指标用异步队列批量写入，不阻塞主流程 |
| **事务粒度** | 事务尽量短，禁止事务内包含外部调用（HTTP/RPC） |
| **热点更新** | 高频更新字段拆到独立表（如计数器），减少主表锁竞争 |
| **连接池** | 应用层用连接池（PgBouncer），连接数 = CPU核数 * 2 + 磁盘数 |

### 6.3 缓存策略

| 数据类型 | 缓存方式 | TTL | 说明 |
|----------|----------|-----|------|
| 权限/角色 | Redis | 5min | 用户权限变更主动失效 |
| 字典数据 | Redis/本地缓存 | 1h | 字典变更主动失效 |
| 元数据 | 本地缓存(moka) | 永久+变更通知 | 实体/字段/关系定义 |
| 专家匹配 | Redis | 5min | 相似任务缓存匹配结果 |
| 热点业务 | Redis | 按业务 | 商品/用户等高频读 |
| 会话/Token | Redis | 会话有效期 | JWT黑名单/用户会话 |

---

## 七、设计审核清单（CI自动检测）

### 7.1 表结构审核

- [ ] 表名符合命名规范（前缀+snake_case+单数）
- [ ] 包含所有公共字段（id/tenant_id/created_by/created_at/updated_by/updated_at/deleted_at/version）
- [ ] 主键类型 VARCHAR(64)，不用自增ID
- [ ] 所有外键字段（_id后缀）已建索引
- [ ] tenant_id 已建联合索引
- [ ] 单表索引 ≤ 5个
- [ ] 无冗余索引（联合索引最左前缀不重复建单列）
- [ ] 金额字段用 DECIMAL，不用 FLOAT/DOUBLE
- [ ] 枚举用 VARCHAR+CHECK，不用 ENUM 类型
- [ ] 时间字段用 TIMESTAMP UTC，不用 DATETIME 无时区
- [ ] 布尔字段有 is_/has_/can_ 前缀
- [ ] 无 SQL 保留字作为字段名
- [ ] 大表（预估>1000万行）已设计分区策略
- [ ] 敏感字段已标注加密要求

### 7.2 索引审核

- [ ] 索引名符合规范（uk_/idx_/ft_/vec_）
- [ ] 联合索引字段顺序符合最左前缀原则
- [ ] 区分度低的字段（status/type）不单独建索引
- [ ] 高频查询有覆盖索引
- [ ] 长字符串字段建前缀索引
- [ ] JSONB查询建GIN表达式索引
- [ ] 向量字段建ivfflat/hnsw索引

### 7.3 性能审核

- [ ] 无 SELECT * 查询
- [ ] 深分页用游标分页
- [ ] 无 N+1 查询
- [ ] 大表 COUNT 用估算
- [ ] 查询有超时控制
- [ ] 写入用批量操作
- [ ] 事务内无外部调用
- [ ] 热点字段已拆表

### 7.4 安全审核

- [ ] 密码字段用 bcrypt/argon2 哈希
- [ ] 敏感字段已加密
- [ ] 数据库连接强制 TLS
- [ ] 应用账号最小权限
- [ ] 多租户隔离已实现（tenant_id + RLS）
- [ ] 审计日志已开启
- [ ] 备份加密已配置

---

## 八、变更管理规范

### 8.1 版本化迁移

- 所有表结构变更通过迁移脚本管理（Flyway/Liquibase/Refinery）
- 迁移脚本命名：`V{版本号}__{描述}.sql`（如 `V1.0.1__add_user_avatar.sql`）
- 迁移脚本必须可重复执行（幂等）
- 生产环境变更必须有回滚脚本

### 8.2 在线DDL

- 大表变更用在线DDL工具（pg_repack/gh-ost/pt-online-schema-change）
- 优先用扩展字段（JSONB extra）避免 ALTER TABLE
- 新增字段必须有默认值或允许NULL
- 删除字段先标记废弃（保留1个版本周期），再物理删除

### 8.3 数据迁移

- 数据迁移用异步任务，分批处理（每批≤10000行）
- 迁移过程双写（新旧表同时写），校验一致后切换读
- 迁移后数据校验：行数/抽样/关键字段对比
- 保留回滚窗口（至少7天）

---

*本规范是 MOX 元架构数据库设计的强制标准，所有表结构变更必须通过 CI 审核。版本 v1.0*
