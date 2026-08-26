# 数据库设计规范（ID 统一 UUID v7）

> 版本：v2.0（2026-08-26）
> **核心变更**：全库主键由「bigint 自增 / 雪花 ID」统一迁移为 **UUID v7**。
> UUID v7 时间前缀排序特性（时间有序、索引友好），由应用层生成，天然全局唯一、不受软删除影响。

---

## 1. 命名规范

### 1.1 通用原则
- 全部 **小写**，单词间用**下划线 `_`** 分隔，不使用驼峰、连字符。
- 见名知意，使用**业务域英文名词**，禁止拼音（地名/专有名词拼音缩写需团队共识，如 `foshan`）。
- 长度控制：`表名 ≤ 50`、`字段名 ≤ 30`、`索引名 ≤ 50`。
- 禁止使用 MySQL 保留字（`order`、`group`、`desc`、`status` 等为保留字，需加业务前缀或 `_` 后缀，如 `order_no`、`task_status`）。

### 1.2 表命名
| 类型 | 规则 | 示例 |
|------|------|------|
| 业务主表 | `模块_实体` | `rpa_flow_run_config`、`sys_sso_user_mapping` |
| 关联/明细表 | `主表_从表` 或 `主表_关系` | `flow_run_group_member`、`rpa_matter_flow_bind` |
| 日志/记录表 | `实体_record` | `rpa_flow_run_config_record` |
| 分布式/中间件表 | `rpa_业务` | `rpa_distributed_task`、`rpa_distributed_lock` |
| 字典/配置表 | `sys_*` / `*_config` | `sys_param`、`flow_run_group_config` |

### 1.3 字段命名
| 语义 | 固定列名 | 类型 |
|------|----------|------|
| 主键 | `id`（**`char(36)` 存 UUID v7**）或 `id_`（兼容历史） | `char(36)` |
| 逻辑删除 | `del_at` | `char(1)` 默认 `'N'`，取值 `'N'`(正常) / `'Y'`(已删除) |
| 创建人 | `create_by` | `varchar(32)` |
| 创建时间 | `create_time` | **`bigint unsigned`**（注意：无 `(20)` 显示宽度，MySQL `bigint(n)` 的 n 仅为显示宽度、不限制存储长度，8.0 已不推荐书写） |
| 修改人 | `update_by` | `varchar(32)` |
| 修改时间 | `update_time` | **`bigint unsigned`**（同上，无显示宽度） |
| 租户/部门 | `sys_org_code` | `varchar(64)` |
| 状态 | `status` / `xxx_status` | `varchar(20)`（大写英文缩写） |
| 备注 | `remark` | `varchar(255)` |

### 1.4 索引命名
| 类型 | 规则 | 示例 |
|------|------|------|
| 主键 | `PRIMARY KEY` | — |
| 唯一索引 | `uk_字段` | `uk_group_code`、`uk_version` |
| 普通索引 | `idx_字段` | `idx_group_id`、`idx_status` |
| 组合索引 | `idx_字段1_字段2` | `idx_task_id_status` |

### 1.5 UUID v7 主键规范（强制）

> **所有新表主键必须使用 UUID v7，禁止 bigint 自增 / 雪花 ID / UUID v4。** 存量表按「冻结」原则不回改，但**新表、新字段、新变更脚本必须遵循**。

| 规则 | 说明 |
|------|------|
| **生成位置** | **应用层生成**（Java `UUIDv7` / Rust `uuid::Uuid::now_v7()`），**禁止数据库端 `UUID()` / 触发器 / 自增** |
| **存储格式** | `char(36)` 小写、标准横线格式（`8-4-4-4-12`，如 `0192f2b0-6f5a-7a00-8000-000000000001`）；禁止二进制 `binary(16)`（排障不可读） |
| **时间有序** | UUID v7 高 48 位为毫秒时间戳，天然时间有序 → B+ 树插入顺序化，页填充率高，**无 UUID v4 随机插入的页分裂问题** |
| **全局唯一** | 无需任何中心化发号器 / 分布式锁，软删除后新记录主键不冲突（天然规避第 6.7 章问题） |
| **外键引用** | 所有 `xxx_id` 外键字段与主键同规格：`char(36)` |
| **与时间字段关系** | UUID v7 内嵌时间戳**仅用于排序/索引友好**，不作为 `create_time` 业务字段的替代，`create_time` 仍按第 3 章用 bigint 毫秒戳 |
| **显示** | 对外展示建议截断/缩写（如列表页），排查日志记录完整值 |

---

## 2. 字符集与排序规则（强制）

```sql
ENGINE = InnoDB
CHARACTER SET = utf8mb4
COLLATE = utf8mb4_unicode_ci
ROW_FORMAT = DYNAMIC
```

- **禁止** `utf8mb3` / `utf8mb3_general_ci`（非完整 UTF-8，emoji/生僻字会截断）。
- 连接层在建库脚本头部固定：
  ```sql
  SET NAMES utf8mb4;
  SET FOREIGN_KEY_CHECKS = 0;   -- 变更脚本使用，规避外键阻塞
  ```

---

## 3. 时间字段规范（核心：统一 bigint 毫秒时间戳）

### 3.1 设计原则
> **所有「记录创建/更新时间」以及「业务时间点（如过期、起止、心跳）」一律使用 `bigint unsigned` 存储毫秒时间戳**，不使用 `DATETIME` / `TIMESTAMP`。

理由：
1. **时区无关**：毫秒时间戳为绝对时间，跨时区、跨库迁移无歧义；`DATETIME` 无时区信息，运维排障易错。
2. **计算高效**：差值运算为纯整数减法，无需 `TIMESTAMPDIFF`，索引友好。
3. **与代码层对齐**：`BaseEntity.createTime`/`updateTime` 为 `String`，经 `toDatetimeLongStr()` 入库即毫秒串；VO 层用 `toDatetimeStr()` 转可读日期给前端。全链路自洽。
4. **无 2038 年溢出**：`TIMESTAMP` 上限 2038，`bigint` 可用至公元 2.9 亿年。

### 3.2 标准写法
```sql
`create_time` BIGINT UNSIGNED DEFAULT NULL COMMENT '创建时间',
`update_time` BIGINT UNSIGNED DEFAULT NULL COMMENT '更新时间',
```
> 注意：MySQL `BIGINT(20)` 的 `(20)` 仅为**显示宽度**，不限制 8 字节存储长度，MySQL 8.0 官方已不推荐书写该宽度。全文统一去掉 `(20)`，避免新人误以为其约束存储长度。

### 3.3 写入默认值（与代码生成器一致）
- **Java 自动填充**：`BaseEntity` 通过 `@ColumnField(..., format = "toDatetimeLongStr")` 在 insert/update 时由框架写入，**禁止在 SQL 手写 NOW()**。
- **纯 SQL 初始化/回填**（如升级脚本）：**唯一允许**的毫秒时间戳表达式为：
  ```sql
  UNIX_TIMESTAMP(NOW(3)) * 1000          -- 精确到毫秒（NOW(3) 提供微秒、*1000 转毫秒）
  ```
  > ⚠️ **已废弃并删除** `CAST(SYSDATE() AS UNSIGNED)` 写法：`SYSDATE()` 返回 `YYYYMMDDHHMMSS`（14 位，**不含毫秒**），被 `CAST` 成整数后毫秒部分丢失，无法得到 17 位毫秒值，线上回填会丢精度。代码生成器模板中的旧写法须一并移除（见第 9 章修订）。
  **禁止** `NOW()` / `CURRENT_TIMESTAMP()` 直接写入 `bigint` 列（值为 0 或报错）。

### 3.4 业务时间点字段同样用 bigint
| 字段 | 类型 | 示例 |
|------|------|------|
| `expire_time`（过期） | `bigint unsigned` | `flow_run_group_invite`、`mox_encryption_key` |
| `start_time` / `end_time`（任务起止） | `bigint unsigned` | `rpa_distributed_task` |
| `last_heartbeat`（心跳） | `bigint unsigned` | `rpa_distributed_node`、`rpa_client` |
| `bind_time` / `unbind_time` | `bigint unsigned` | `sys_sso_user_mapping` |

### 3.5 例外（保留非 bigint 时间）
- **业务时段（一天内的时刻，非绝对时间）**：如 `rpa_time_slot.start_time` / `end_time` 表示 `09:00:00` 这类「每天重复的执行窗口」，语义为 `TIME` 类型，**保留 `TIME`**，不改为 bigint。
- **仅用于展示的日期（无时间精度需求）**：如 `birthday`，可用 `DATE`，但需团队评审。

---

## 4. 字段类型选型规范

| 场景 | 推荐类型 | 说明 |
|------|----------|------|
| 主键 | **`char(36)`**（UUID v7，小写带横线） | **禁止自增**、禁止 `int`；全局唯一，见 1.5 |
| 外部系统 ID / 历史雪花 ID | `bigint` 或 `varchar(36)` | 仅用于外部系统透传/存量兼容，不作为本库主键 |
| 短字符串（编码/状态） | `varchar(20~64)` | 状态/编码用 `varchar` 便于扩展 |
| 名称/标题 | `varchar(64~255)` | 超长用 `text` |
| 长文本/JSON | `text` / `json` | MySQL 8 原生 `json` 支持索引 |
| 金额 | `decimal(12,2)` 或 `decimal(10,3)` | **禁止** `float`/`double`（精度丢失） |
| 布尔/标志 | `char(1)` 大写英文缩写（如 `'N'/'Y'`） | 仅限二值语义（如 `del_at`）。**禁止数字** |
| 计数/数量 | `int` / `int UNSIGNED` | — |
| 状态/类型（多值枚举） | **`varchar(20)` 大写英文简写** | **禁止 `tinyint`/`int` 数字枚举**，见 4.1 |
| 时间戳 | **`bigint unsigned`** | 见第 3 章 |

### 4.1 状态字段规范（强制）

> **所有业务「状态 / 类型」字段禁止用数字（tinyint/int）表达多值语义，必须使用 `varchar(20)` 存储大写英文字母或英文简写。**

理由：
1. **可读性**：`status = 'RUNNING'` 比 `status = 2` 自解释，无需查码表，SQL 排查、日志、跨团队沟通零歧义。
2. **可演进**：新增状态只需扩展取值，无需改类型或担心数字越界；数字枚举一旦 0/1/2 语义变动极易引发线上 bug。
3. **跨语言一致**：Java 用 `String`/`enum`、前端 `el-select` 直接绑定，避免各端维护数字映射。
4. **与本项目一致**：`sys_ftp_task.status`、`rpa_distributed_node.status`、`rpa_distributed_task.status`、`rpa_client.status` 等已采用 `VARCHAR(20)` 英文简写（`PENDING/RUNNING/COMPLETED/FAILED/CANCELLED`、`ONLINE/OFFLINE`）。

**标准写法**：
```sql
`status` VARCHAR(20) NOT NULL DEFAULT 'NORMAL' COMMENT '状态 {NORMAL:正常, DISABLED:停用}',
```

**通用取值词典（推荐复用，保持全库统一）**：
| 语义 | 取值 |
|------|------|
| 启用/停用 | `NORMAL` / `DISABLED` |
| 有效/失效 | `ENABLED` / `DISABLED` |
| 在线/离线 | `ONLINE` / `OFFLINE` |
| 任务状态 | `PENDING` / `RUNNING` / `COMPLETED` / `FAILED` / `CANCELLED` |
| 邀请/申请 | `PENDING` / `ACCEPTED` / `REJECTED` / `EXPIRED` |
| 删除 | `del_at` 用 `char(1)` `'N'`(正常) / `'Y'`(已删除)（二值） |
| 性别等固定二元 | `varchar(1)` 大写英文缩写 `'M'/'F'`，需 COMMENT 注明 |

**COMMENT 必须写明取值映射**（见 4.2）。

### 4.1.1 枚举取值约束与脏数据防控（强制）

> **重大隐患**：数据库排序规则 `utf8mb4_unicode_ci` **大小写不敏感**，`RUNNING` = `running`；但 **Java 代码字符串区分大小写**！若数据库存入小写 `'running'`，SQL 层 `WHERE status='RUNNING'` 能命中，而 Java 层 `StatusEnum.RUNNING.name().equals(rs)` 会判失败，产生线上状态判断 bug。

约束：
1. **取值全部大写英文**，不允许小写、驼峰、`snake_case`。
2. **Java 枚举常量名与数据库字符串完全一致**（如 `enum TaskStatus { RUNNING, PENDING }` ↔ `status = 'RUNNING'`）。
3. **禁止在 SQL 硬编码小写状态值**；SQL 中引用状态统一大写。
4. 数据库层取值约束（二选一）：
   - **方案 A（推荐，业务可控）**：Java 层校验，非法状态禁止入库（枚举 `valueOf` 拦截 / 入库前断言）。
   - **方案 B（强约束）**：MySQL 8.0 使用 `CHECK` 约束限制取值范围（DDL 明确枚举域）：
     ```sql
     `status` VARCHAR(20) NOT NULL DEFAULT 'NORMAL' COMMENT '状态 {NORMAL:正常, DISABLED:停用}',
     CONSTRAINT chk_status CHECK (status IN ('NORMAL','DISABLED'))
     ```
     > 注意：MySQL 8.0 默认 `sql_mode` 含 `CHECK` 校验，约束生效；低版本/关闭校验时需依赖方案 A。
5. 禁止自由新增状态字符串：新增取值须走评审 + 同步更新 COMMENT 取值映射 + 同步 Java 枚举（见 4.2）。

### 4.2 枚举注释约定
所有状态/类型字段，COMMENT 必须写明取值映射：
```sql
`status` VARCHAR(20) NOT NULL DEFAULT 'NORMAL' COMMENT '状态 {NORMAL:正常, DISABLED:停用}',
`invite_status` VARCHAR(20) NOT NULL DEFAULT 'PENDING' COMMENT '邀请状态 {PENDING:待接受, ACCEPTED:已接受, REJECTED:已拒绝, EXPIRED:已过期}',
```

---

## 5. 表结构设计规范

### 5.1 必备字段（每张业务表）

必备字段分两类：**全局审计字段（无例外）+ 可选多租户字段**。

#### 5.1.1 全局审计字段（每张业务表强制必备，无例外）
```sql
`id`          char(36)      NOT NULL COMMENT '主键(UUID v7)',
`create_by`   varchar(32)   DEFAULT NULL COMMENT '创建人',
`create_time` bigint UNSIGNED DEFAULT NULL COMMENT '创建时间',
`update_by`   varchar(32)   DEFAULT NULL COMMENT '修改人',
`update_time` bigint UNSIGNED DEFAULT NULL COMMENT '更新时间',
`del_at`      char(1)       NOT NULL DEFAULT 'N' COMMENT '删除状态(N-正常,Y-已删除)',
PRIMARY KEY (`id`)
```
> `id` 由应用层生成 UUID v7（`0192f2b0-6f5a-7a00-...`），**无 `AUTO_INCREMENT`、无数据库默认值**；插入时应用层必须显式赋值。

#### 5.1.2 多租户字段（边界说明）
- **多租户隔离业务表强制附加**：`sys_org_code varchar(64) DEFAULT NULL COMMENT '租户/部门编码'`。
- **不加租户字段的表**：全局字典表（`sys_*` 字典）、系统配置表（如 `sys_param`、`*_config`）、分布式锁/任务等中间件表（如 `rpa_distributed_lock`、`rpa_distributed_task`）。这些表为全局共享、不按租户隔离，多加租户字段反而造成查询与维护负担。
- ⚠️ 租户字段**漏加代价极高**（后期补字段需全表 DDL + 历史数据回填），新建业务表立项时须明确是否多租户隔离，并在评审清单确认（见第 10 章）。

### 5.2 表 COMMENT 必填
每张表必须有 `COMMENT '中文说明'`，明确表用途与归属模块。

### 5.3 存储参数
```sql
ENGINE = InnoDB
CHARACTER SET = utf8mb4
COLLATE = utf8mb4_unicode_ci
ROW_FORMAT = DYNAMIC
```
- 大字段（`text`/`blob`）多时 `ROW_FORMAT=DYNAMIC` 避免 8KB 行溢出限制。
- 单表行数预估超 500 万或容量超 2GB，需评估分表/分区（见第 8 章）。

---

## 6. 索引规范

1. **外键/关联字段必建索引**：如 `group_id`、`flow_id`、`task_id`、`node_id`（`char(36)` 等值查询，BTREE 即可，UUID v7 时间有序进一步降低页分裂）。
2. **高频查询条件建索引**：状态、时间范围、业务编码。
3. **组合索引最左前缀**：区分度高的列前置；`(status, create_time)` 优于 `(create_time, status)`。
4. **禁止冗余索引**：`idx_a` + `idx_a_b` 中 `idx_a` 冗余，删之。
5. **索引数量控制**：单表索引 ≤ 5 个，避免写放大。
6. **长字段索引**：`varchar` 超 191 字符需指定前缀长度 `idx_col(191)`。

### 6.7 逻辑删除场景唯一索引规则（政务高频坑）

> 本项目使用逻辑删除（`del_at`），**单一业务编码唯一索引会与软删除冲突**：删除一条记录（`del_at='Y'`）后，相同业务编码无法再插入新记录（唯一索引报错）。
> **UUID v7 主键天然全局唯一**，本身不参与该冲突；本条仅约束「业务编码」类唯一性。

规则：
- ❌ **禁止**：仅对业务编码建单一唯一索引 `uk_business_code`，删除后无法新建相同编码记录。
- ✅ **方案 A（推荐）**：业务唯一性使用独立「全局唯一编号」`xxx_no`（如 `flow_no`、`order_no`），由代码生成器/雪花算法/UUID 生成，不受删除影响，对 `xxx_no` 建唯一索引。
- ⚠️ **方案 B（不推荐，仅历史兼容）**：若必须「编码唯一 + 软删除」共存，唯一索引须包含 `del_at`，即 `UNIQUE KEY uk_code_del (business_code, del_at)`；但 `del_at` 取值少会导致唯一键区分度低、并发写入易冲突，慎用。

### 6.8 外键约束规范（物理外键禁令）

> **项目禁止创建 InnoDB 物理外键（`FOREIGN KEY`）**。表与表之间的关联一致性全部由 Java 业务代码保障。

理由：
- 物理外键在高并发、分布式、分表、DDL 在线变更（`gh-ost`/`pt-osc`）场景下会引发锁表、主从延迟、迁移失败。
- 政务后端项目几乎全部禁用物理外键，改由应用层逻辑关联 + 必要的数据校验。

> 注意：第 2/7 章脚本头部 `SET FOREIGN_KEY_CHECKS = 0;` **仅用于导入初始化数据、规避外键阻塞，不等于允许建立物理外键**。建表 DDL 中不得出现 `FOREIGN KEY ... REFERENCES` 子句。

---

## 7. 变更脚本（DDL/DML）规范

### 7.1 升级脚本形态
- 采用「**一键升级.sql**」模式：`CREATE TABLE IF NOT EXISTS` + `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`（MySQL 8 不支持 `ADD COLUMN IF NOT EXISTS`，改用存储过程或幂等判断，见 7.3）。
- 脚本需**可重复执行**（幂等）：先判断表/列是否存在，再操作。
- 头部固定：
  ```sql
  SET NAMES utf8mb4;
  SET FOREIGN_KEY_CHECKS = 0;
  ```

### 7.2 新增列（时间字段）
```sql
ALTER TABLE `flow_run_group`
  ADD COLUMN `create_time` BIGINT UNSIGNED DEFAULT NULL COMMENT '创建时间',
  ADD COLUMN `update_time` BIGINT UNSIGNED DEFAULT NULL COMMENT '更新时间';
```
回填存量数据：
```sql
UPDATE `flow_run_group`
   SET `create_time` = UNIX_TIMESTAMP(NOW(3)) * 1000,
       `update_time` = UNIX_TIMESTAMP(NOW(3)) * 1000
 WHERE `create_time` IS NULL;
```

### 7.3 幂等检查（推荐模板）
```sql
SET @db = DATABASE();
SELECT COUNT(*) INTO @c
FROM information_schema.COLUMNS
WHERE TABLE_SCHEMA = @db AND TABLE_NAME = 'flow_run_group' AND COLUMN_NAME = 'create_time';
SET @sql = IF(@c = 0,
  'ALTER TABLE `flow_run_group` ADD COLUMN `create_time` BIGINT UNSIGNED DEFAULT NULL COMMENT ''创建时间''',
  'SELECT 1');
PREPARE stmt FROM @sql; EXECUTE stmt; DEALLOCATE PREPARE stmt;
```

### 7.4 禁止事项
- ❌ 禁止 `DROP COLUMN` / `DROP TABLE` 生产直接执行（须备份 + 评审）。
- ❌ 禁止 `ALTER TABLE` 改列类型导致锁表超时的操作（大表用 `pt-online-schema-change` / `gh-ost`）。
- ❌ 禁止在变更脚本写 `NOW()` 写入 bigint 时间列。
- ❌ 禁止隐含事务的批量 DML 一次超 10 万行（分批 `LIMIT 5000`）。

---

## 8. 性能与规模规范

1. **单表行数**：预估 > 500 万行，按「时间/租户」分表；> 2000 万行评估分区表。
2. **大表 DDL**：一律在线变更工具，避免主从延迟与锁表。
3. **查询规范**：
   - `SELECT *` 禁止上生产，明确列。
   - `LIMIT` 必须配合 `ORDER BY` 有索引列。
   - 禁止 `LIKE '%xxx%'` 前缀模糊（索引失效），改用全文索引 `FULLTEXT` 或 ES。
4. **连接池**：Druid 监控开启，`maxActive` 按业务压测设定，配置 `validationQuery`。

### 8.1 分区方案（毫秒时间戳不能直接 RANGE 分区）

> ⚠️ **陷阱**：`create_time` 为 `bigint` 毫秒时间戳（数值极大，如 `1756000000000`），**不能直接 `PARTITION BY RANGE(create_time)` 按月/年分区**——RANGE 分区键需与可计算的时段边界对齐，原始毫秒戳无法表达「2026-08」这类年月区间。
> （UUID v7 内嵌时间戳同理，同样不能直接作 RANGE 分区键，一律用下方方案。）

可行方案：
- **方案一（推荐）：增加冗余分区辅助字段** `partition_date DATE`，由业务写入时根据时间 `FROM_UNIXTIME(create_time/1000)` 维护，再 `PARTITION BY RANGE (TO_DAYS(partition_date))` 按年月分区。
  ```sql
  `partition_date` DATE GENERATED ALWAYS AS (FROM_UNIXTIME(create_time/1000)) STORED COMMENT '分区辅助字段',
  PARTITION BY RANGE (TO_DAYS(partition_date)) (
    PARTITION p202601 VALUES LESS THAN (TO_DAYS('2026-02-01')),
    PARTITION p202602 VALUES LESS THAN (TO_DAYS('2026-03-01'))
  );
  ```
- **方案二（放弃原生分区）**：采用应用层分表（按年/月分物理表 `rpa_flow_log_2026_08`），由代码路由，规避分区键转换复杂度。

> 分区为高风险 DDL，上线前须评估迁移成本与查询路由，严禁对核心事务表盲目分区。

---

## 9. 代码生成器与前端控件对应关系（重要说明）

代码生成器（`mox-parent/doc/代码生成模版`）与前端模板（`htmlType == "datetime"`）**已与本规范自洽，禁止误改**：

1. **建表 DDL 模板已用 bigint**：`数据库模版/常用.md` 定义 bigint 时间默认值为
   `UNIX_TIMESTAMP(NOW(3)) * 1000`（精确到毫秒）。
   > ⚠️ 旧模板中的 `(CAST(SYSDATE() AS UNSIGNED))` 写法**已废弃并移至第 3.3 章列为反例**——`SYSDATE()` 仅 14 位、丢失毫秒精度，须从模板中删除，统一用 `UNIX_TIMESTAMP(NOW(3)) * 1000`。
2. **实体类模板用 `toDatetimeLongStr()`**：`domain.java.vm` 对注释含「时间/日期」的字段自动覆写 setter，
   将入参经 `DateUtils.toDatetimeLongStr()` 转为毫秒时间戳字符串写入 bigint 列。
3. **VO 类模板用 `toDatetimeStr()`**：`domainVo.java.vm` 对同类字段用 `toDatetimeStr()` 转可读日期（`yyyy-MM-dd HH:mm:ss`）给前端展示。
4. **前端 `htmlType == "datetime"` 是 UI 控件类型，不是存储类型**：Vue 模板（`index.vue*`、`index-tree.vue*`）
   中 `el-date-picker type="datetime"` 仅表示「用日期时间选择器录入」，提交后由实体层转换入库。
   **此处 `datetime` 切勿改成 `bigint`**，否则前端控件失效。
5. **`${datetime}` 是生成日期变量**（如 `@date 2026-08-06`），与 SQL `DATETIME` 类型无关，勿混淆。
6. **主键生成**：代码生成器主键模板已改为 **UUID v7**（应用层生成 `char(36)`），实体类 insert 前调用 `IdGen.uuidV7()` 显式赋值；**禁止生成 `AUTO_INCREMENT` 主键**。

> 结论：存储层（DDL + 实体）已是 bigint 毫秒戳 + UUID v7 主键；展示层（VO + Vue 日期控件）负责可读化。
> 二者通过 `DateUtils` 双向转换，全链路自洽。调整时间相关逻辑只改 `DateUtils` / `BaseEntity`，不动模板类型。

---

## 10. 自检清单（Code Review 必过）

- [ ] 表/字段全小写 + 下划线，无保留字（**新表/新字段**）
- [ ] 字符集 utf8mb4 / utf8mb4_unicode_ci，引擎 InnoDB，ROW_FORMAT DYNAMIC（**新表/新字段**）
- [ ] **主键 `char(36)` UUID v7，应用层生成，禁止自增/雪花/`AUTO_INCREMENT`**（**新表**，见 1.5）
- [ ] `xxx_id` 外键字段与主键同规格 `char(36)` 且已建索引（**新表/新字段**）
- [ ] 所有时间字段为 `bigint unsigned`（**无 `(20)` 显示宽度**；时段除外），COMMENT 注明「毫秒时间戳」（**新表/新字段**）
- [ ] 时间写入用 `toDatetimeLongStr()` 或 `UNIX_TIMESTAMP(NOW(3))*1000`，**已废弃 `CAST(SYSDATE() AS UNSIGNED)`**，无 `NOW()`（**新代码/新脚本**）
- [ ] 必有 `create_by/create_time/update_by/update_time/del_at`（**新表**）
- [ ] 多租户业务表已加 `sys_org_code`；字典/配置/中间件表未多余加租户字段（见 5.1.2）
- [ ] 状态/类型字段为 `varchar(20)` **大写英文简写**（禁止 tinyint 数字枚举），COMMENT 含取值映射；**Java 枚举与库字符串一致、取值全大写**（**新字段**，见 4.1.1）
- [ ] 金额 `decimal`，二元标志（`del_at`）用 `char(1)` `'N'/'Y'`（N-正常,Y-已删除）
- [ ] 唯一索引已评估软删除冲突：业务唯一编号用 `xxx_no` 方案，未对单一业务编码裸建唯一索引（**新表**，见 6.7）
- [ ] 索引命名 `uk_/idx_`，外键/高频查询字段有索引（**新表/新字段**）
- [ ] 新建表**未建物理外键 FOREIGN KEY**（见 6.8）
- [ ] 计划分区：毫秒时间戳 / UUID v7 时间戳**未直接**作 RANGE 分区键，已加分区辅助字段（见 8.1）
- [ ] 变更脚本幂等、可重复执行、头部 `SET NAMES` + `FOREIGN_KEY_CHECKS=0`（**新脚本**）
- [ ] 大表变更走在线工具，批量 DML 分批

---

## 11. JSON 与大字段存储规范

1. **结构化扩展属性优先用 `JSON` 类型**，禁止用 `varchar` 存 JSON 字符串（无法校验结构、无法索引）。
2. **JSON 字段不建普通索引**；MySQL 8.0 如需检索内部字段，使用 **JSON 二级索引**（`CREATE INDEX ... ON t ((CAST(json_col->>'$.field' AS ...)))`）。
3. **超大文本 / 富文本用 `text` / `longtext`**，禁止用 `varchar(4000)` 之类的超大定长列（浪费行内空间、超出 65535 字节行限制）。
4. JSON 高频更新字段（如大对象内嵌数组）会产生行膨胀，评估是否拆独立表。

## 12. 数据安全与脱敏字段规范（政务刚需）

1. **敏感字段范围**：手机号、身份证号、银行卡号、密码/密钥、人脸/生物特征等。
2. **存储**：密码/密钥类必须**不可逆加密**（如 `bcrypt`/`Argon2`）；身份证/手机号/银行卡等若需可逆，使用项目统一加密组件（如 `mox_encryption_key` 体系），禁止明文落库。
3. **查询/展示脱敏**：列表/接口返回的敏感字段须脱敏（如手机号 `138****8000`、身份证 `4406**********1234`），脱敏在 VO/接口层完成，数据库层仍存密文/明文。
4. **日志**：禁止将敏感明文打印到应用日志、SQL 日志（Druid 监控开启 `filter` 脱敏）。
5. **导出**：含敏感字段的数据导出须走权限审批 + 水印。

## 13. SQL 开发红线清单（慢 SQL 与高危写法）

1. ❌ 禁止 `SELECT *` 上生产（明确列，减少 IO 与回表）。
2. ❌ 禁止 `UPDATE/DELETE` 不带 `WHERE` 或 `WHERE` 无索引（全表锁/全表扫）。
3. ❌ 禁止 `SELECT ... FOR UPDATE` 命中无索引条件（锁全表，引发死锁与雪崩）。
4. ❌ 禁止单条 SQL `JOIN` 超过 3 张表；超过须拆分或下沉 ES/数仓。
5. ❌ 禁止 `LIKE '%xxx%'` 前缀模糊（索引失效）；改用 `FULLTEXT` / ES。
6. ❌ 禁止在 `WHERE` 对字段套函数（如 `WHERE DATE(create_time/1000)=...` 导致索引失效），改为范围比较。
7. ❌ 禁止 `ORDER BY` 非索引字段 + 大 `OFFSET` 深翻页（改用游标/主键游标分页）。
8. ❌ 禁止大事务（单事务跨多网络调用 / 批量超 10 万行）；分批 `LIMIT 5000`。
9. ❌ 禁止在循环内逐条 `INSERT`，使用 `batch` / `INSERT ... VALUES (...),(...)`。
10. ⚠️ 线上慢 SQL 阈值（如 > 1s）须接入监控告警，定期 `EXPLAIN` 复核执行计划。
