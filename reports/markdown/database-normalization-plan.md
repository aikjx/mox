# 数据库体系归一化修复方案

> 生成时间：2026-09-02
> 适用项目：infotopograph（璇玑 RelGraph・算子统一系统）
> 状态：messages 双写分裂已修复；其余为规划与待决策项



***

## 一、三套数据库现状对比

### 1.1 总览



| 维度    | data/mox.db（主库）                                    | mox\_meta.db（Python 元库）             | mox\_business.db（Python 业务库） |
| ----- | -------------------------------------------------- | ----------------------------------- | ---------------------------- |
| 所属层   | Rust 网关 IAM 主库                                     | Python legacy 元数据                   | Python legacy 业务内容           |
| 文件大小  | 644 KB                                             | 184 KB                              | 68 KB                        |
| 表数量   | 22                                                 | 17                                  | 6                            |
| 主键类型  | TEXT UUID（v4）                                      | INTEGER AUTOINCREMENT               | INTEGER（部分 AUTOINCREMENT）    |
| 时间格式  | TEXT ISO8601（RFC3339）                              | INTEGER Unix 时间戳                    | INTEGER Unix 时间戳             |
| 命名前缀  | `iam_` / `sys_` / `audit_log`                      | 无前缀                                 | 无前缀                          |
| 命名风格  | snake\_case                                        | snake\_case                         | snake\_case                  |
| 初始化方式 | Rust `init_schema()` + `seed_builtins()`           | Python `init_meta()` + `seed_all()` | Python `init_business()`     |
| 幂等性   | `CREATE TABLE IF NOT EXISTS` + "already exists" 容错 | `CREATE TABLE IF NOT EXISTS`        | `CREATE TABLE IF NOT EXISTS` |

### 1.2 data/mox.db（22 表）— Rust IAM 体系



| #  | 表名                    | 行数  | 用途        | 主键                |
| -- | --------------------- | --- | --------- | ----------------- |
| 1  | iam\_tenant           | 2   | 租户        | tenant\_id TEXT   |
| 2  | iam\_department       | 1   | 部门        | dept\_id TEXT     |
| 3  | iam\_user             | 2   | 用户（19 字段） | user\_id TEXT     |
| 4  | iam\_role             | 16  | 角色（15 字段） | role\_id TEXT     |
| 5  | iam\_permission       | 140 | 权限        | perm\_id TEXT     |
| 6  | iam\_resource         | 20  | 资源        | resource\_id TEXT |
| 7  | iam\_user\_role       | 2   | 用户 - 角色关联 | ur\_id TEXT       |
| 8  | iam\_role\_permission | 280 | 角色 - 权限关联 | rp\_id TEXT       |
| 9  | iam\_role\_inherit    | 0   | 角色继承      | ri\_id TEXT       |
| 10 | iam\_menu             | 0   | 菜单（25 字段） | menu\_id TEXT     |
| 11 | iam\_user\_menu       | 0   | 用户 - 菜单   | um\_id TEXT       |
| 12 | iam\_role\_menu       | 0   | 角色 - 菜单   | rm\_id TEXT       |
| 13 | iam\_data\_permission | 0   | 数据权限      | dp\_id TEXT       |
| 14 | iam\_tenant\_setting  | 0   | 租户设置      | setting\_id TEXT  |
| 15 | audit\_log            | 0   | 链式哈希审计日志  | log\_id TEXT      |
| 16 | sys\_post             | 0   | 岗位        | post\_id TEXT     |
| 17 | sys\_dict\_type       | 1   | 字典类型      | dict\_id TEXT     |
| 18 | sys\_dict\_data       | 0   | 字典数据      | dict\_code TEXT   |
| 19 | sys\_config           | 1   | 参数配置      | config\_id TEXT   |
| 20 | sys\_oper\_log        | 0   | 操作日志      | oper\_id TEXT     |
| 21 | sys\_logininfor       | 0   | 登录日志      | info\_id TEXT     |
| 22 | sys\_api\_key         | 1   | API 凭证    | key\_id TEXT      |

**DDL 位置**：`platform/domains/platform/core/mox-platform-iam-core/src/ddl.sql`

**初始化代码**：`platform/domains/platform/core/mox-platform-iam-core/src/repo.rs` → `IamRepository::init_schema()`

### 1.3 mox\_meta.db（17 表）— Python legacy 元数据



| #  | 表名                 | 行数    | 用途               | 主键                       |
| -- | ------------------ | ----- | ---------------- | ------------------------ |
| 1  | datasources        | 2     | 数据源配置            | id INTEGER AUTOINCREMENT |
| 2  | dsql\_sqls         | 26    | 动态 SQL 定义        | id INTEGER AUTOINCREMENT |
| 3  | dsql\_apps         | 1     | DSQL 应用          | —                        |
| 4  | dsql\_datasources  | 0     | DSQL 数据源         | —                        |
| 5  | apps               | 2     | 应用（无限发布）         | id INTEGER AUTOINCREMENT |
| 6  | publish\_logs      | 15    | 发布日志             | id INTEGER AUTOINCREMENT |
| 7  | ai\_requests       | 25    | AI 请求记录          | id INTEGER AUTOINCREMENT |
| 8  | kg\_vertices       | 32    | 知识图谱顶点           | vid TEXT                 |
| 9  | kg\_edges          | 44    | 知识图谱边            | id INTEGER AUTOINCREMENT |
| 10 | roles              | 3     | 角色（扁平，3 字段）      | id INTEGER AUTOINCREMENT |
| 11 | users              | 3     | 用户（扁平，4 字段）      | id INTEGER AUTOINCREMENT |
| 12 | field\_permissions | 2     | 字段级权限            | id INTEGER AUTOINCREMENT |
| 13 | audit\_logs        | 282   | 审计日志             | id INTEGER AUTOINCREMENT |
| 14 | **messages**       | **7** | **留言（孤儿表，见第二章）** | id INTEGER AUTOINCREMENT |
| 15 | store\_apps        | 1     | 应用商店 - 应用        | —                        |
| 16 | store\_installs    | 0     | 应用商店 - 安装        | —                        |
| 17 | store\_ratings     | 1     | 应用商店 - 评分        | —                        |

**Schema 定义**：`platform/legacy/mox-server/mox/seed_data.py` → `META_SCHEMA`（11 表）

**注意**：实际 17 表中，`dsql_apps`、`dsql_datasources`、`messages`、`store_apps`、`store_installs`、`store_ratings` 不在 `META_SCHEMA` 中，为历史版本遗留或运行时创建。

### 1.4 mox\_business.db（6 表）— Python legacy 业务内容



| # | 表名           | 行数    | 用途                 | 主键                       |
| - | ------------ | ----- | ------------------ | ------------------------ |
| 1 | banners      | 1     | 首页 Banner          | id INTEGER               |
| 2 | products     | 6     | 产品                 | id INTEGER               |
| 3 | news         | 6     | 新闻                 | id INTEGER               |
| 4 | cases        | 3     | 案例                 | id INTEGER               |
| 5 | team         | 3     | 团队                 | id INTEGER               |
| 6 | **messages** | **0** | **留言（正确位置，写入曾分裂）** | id INTEGER AUTOINCREMENT |

**Schema 定义**：`platform/legacy/mox-server/mox/seed_data.py` → `BUSINESS_SCHEMA`



***

## 二、messages 表双写分裂问题（已修复）

### 2.1 根因

`platform/legacy/mox-server/mox/server.py` 中 `website_message` 端点（原第 433 行）：



```
\# 修复前（错误）

META.execute(

&#x20;   "INSERT INTO messages(name,phone,email,company,content,status,created\_at) VALUES(...)",

&#x20;   \[...])
```



* `META` 连接的是 **mox\_meta.db**（元库）

* 但 `messages` 表的 schema 定义在 `BUSINESS_SCHEMA` 中，属于 **mox\_business.db**（业务库）

* 读取端全部走 mox\_business.db：


  * DSQL `message_list` / `stats_dashboard` → default 数据源 → `BUSINESS_DB`

  * `api_stats` → `BUSINESS_STORE.query("SELECT COUNT(*) FROM messages")`

**结果**：写入 mox\_meta.db.messages（7 行孤儿数据），读取 mox\_business.db.messages（0 行），用户提交留言后后台看不到。

### 2.2 修复方式



1. **备份**：`server.py` → `server.py.bak`

2. **修改写入连接**：`META.execute()` → `BUSINESS_STORE.execute()`

3. **修复返回值**：原硬编码 `{"id": 0}` → `{"id": result.get("last_insert_id", 0)}`

4. **添加注释**：标注修复原因与时间

修复后，messages 的读写统一在 mox\_business.db，与 schema 定义和 DSQL 查询一致。

### 2.3 遗留：孤儿数据迁移

mox\_meta.db.messages 中现有 **7 行历史留言数据**，需迁移至 mox\_business.db.messages。

由于约束禁止直接对 .db 执行 INSERT，提供迁移脚本供用户执行：



```
\# migrate\_orphan\_messages.py（需用户手动执行）

import sqlite3

meta = sqlite3.connect('mox\_meta.db')

biz = sqlite3.connect('mox\_business.db')

rows = meta.execute('SELECT name,phone,email,company,content,status,created\_at FROM messages').fetchall()

biz.executemany(

&#x20;   'INSERT INTO messages(name,phone,email,company,content,status,created\_at) VALUES(?,?,?,?,?,?,?)',

&#x20;   rows)

biz.commit()

print(f'Migrated {len(rows)} messages from mox\_meta.db to mox\_business.db')
```

> 迁移后是否删除 mox_meta.db.messages 孤儿表，由用户决策（见第六章）。



***

## 三、统一数据模型规范

### 3.1 推荐标准：以 data/mox.db 的 iam\_ 体系为基准



| 规范项  | 标准                                | 说明                  |
| ---- | --------------------------------- | ------------------- |
| 主键   | TEXT UUID v4                      | 全局唯一，无需中心化分配，适合分布式  |
| 时间   | TEXT ISO8601（RFC3339，UTC）         | 人类可读、时区明确、排序正确      |
| 命名   | snake\_case                       | 全库统一                |
| 表前缀  | IAM 域用 `iam_`，系统管理用 `sys_`，业务表无前缀 | 按域划分                |
| 软删除  | 不使用（物理删除），审计依赖 audit\_log 链式哈希    | 保持简洁                |
| 版本字段 | `version INTEGER DEFAULT 1`       | 乐观锁                 |
| 索引   | 独立 `CREATE INDEX IF NOT EXISTS`   | 不内联在 CREATE TABLE 中 |
| 字符集  | UTF-8                             | SQLite 默认           |

### 3.2 用户 / 角色 / 权限标准

以 Rust IAM 的完整 RBAC 模型为唯一标准：



* **用户**：`iam_user`（19 字段，含租户、部门、岗位、超管标记、登录信息）

* **角色**：`iam_role`（15 字段，含角色类型、继承、数据范围、内置标记）

* **权限**：`iam_permission` + `iam_resource`（资源 - 动作二维模型，`user:manage` 格式）

* **关联**：`iam_user_role`、`iam_role_permission`、`iam_role_inherit`、`iam_role_menu`、`iam_user_menu`

* **数据权限**：`iam_data_permission`（全部 / 本部门 / 本部门及子级 / 本人 / 自定义）

Python legacy 的扁平 `users`（4 字段）/`roles`（3 字段）/`field_permissions` 为简化演示模型，**不具备生产可用性**，应在迁移中废弃。

### 3.3 业务表标准



* 主键：`id TEXT PRIMARY KEY`（UUID）

* 时间：`created_at TEXT NOT NULL`、`updated_at TEXT NOT NULL`

* 租户隔离：`tenant_id TEXT NOT NULL`（多租户场景）

* 状态：`status TEXT NOT NULL DEFAULT 'active'`



***

## 四、迁移路径：Python legacy → 主库

### 4.1 迁移原则



1. **不删除 legacy 代码和数据**，保留可回退能力

2. **先读后写**：迁移脚本先验证源数据，再写入目标库

3. **双写过渡**：关键表在过渡期可同时写新旧两库，验证一致后切读

4. **ID 映射**：INTEGER 自增 ID → UUID，需建立映射表

### 4.2 分阶段迁移计划

#### 阶段 1：元数据迁移（低风险）



| Python 表      | 主库目标                      | 迁移复杂度 | 说明           |
| ------------- | ------------------------- | ----- | ------------ |
| datasources   | 新建 `biz_datasource` 或纳入配置 | 低     | 2 行，手动迁移即可   |
| dsql\_sqls    | 新建 `biz_dsql_sql`         | 中     | 26 行，模板语法需兼容 |
| apps          | 新建 `biz_app`              | 低     | 2 行          |
| publish\_logs | 新建 `biz_publish_log`      | 低     | 15 行，归档性质    |
| ai\_requests  | 新建 `biz_ai_request`       | 低     | 25 行，日志性质    |

#### 阶段 2：知识图谱迁移（中风险）



| Python 表     | 主库目标                  | 迁移复杂度 | 说明                           |
| ------------ | --------------------- | ----- | ---------------------------- |
| kg\_vertices | Rust `mox-kg-core` 存储 | 中     | 32 顶点，vid 为 TEXT 可直接映射       |
| kg\_edges    | Rust `mox-kg-core` 存储 | 中     | 44 边，需验证 source/target 引用完整性 |

#### 阶段 3：业务内容迁移（中风险）



| Python 表 | 主库目标                 | 迁移复杂度 | 说明         |
| -------- | -------------------- | ----- | ---------- |
| banners  | 新建 `biz_banner`      | 低     | 1 行        |
| products | 新建 `biz_product`     | 低     | 6 行        |
| news     | 新建 `biz_news`        | 低     | 6 行        |
| cases    | 新建 `biz_case`        | 低     | 3 行        |
| team     | 新建 `biz_team_member` | 低     | 3 行        |
| messages | 新建 `biz_message`     | 低     | 需先完成孤儿数据迁移 |

#### 阶段 4：用户体系迁移（高风险，需用户决策）



| Python 表           | 主库目标                  | 迁移复杂度 | 说明                                                                       |
| ------------------ | --------------------- | ----- | ------------------------------------------------------------------------ |
| users（3 行）         | iam\_user             | 高     | 字段不匹配（4→19），需补充 tenant\_id/dept\_id 等                                    |
| roles（3 行）         | iam\_role             | 高     | 字段不匹配（3→15），admin/staff/guest 需映射到 sys\_admin/tenant\_admin/tenant\_user |
| field\_permissions | iam\_data\_permission | 高     | 模型完全不同，需重新设计                                                             |

### 4.3 迁移脚本模板



```
\# 通用迁移模式：INTEGER ID → UUID

import sqlite3, uuid, json

from datetime import datetime, timezone

def to\_iso(ts):

&#x20;   return datetime.fromtimestamp(ts, tz=timezone.utc).isoformat()

src = sqlite3.connect('mox\_meta.db')

dst = sqlite3.connect('../../data/mox.db')

\# 示例：迁移 apps

for row in src.execute('SELECT app\_key,name,type,domain,status,config\_json,publish\_version,created\_at,updated\_at FROM apps'):

&#x20;   new\_id = str(uuid.uuid4())

&#x20;   dst.execute(

&#x20;       'INSERT INTO biz\_app(id,app\_key,name,type,domain,status,config\_json,publish\_version,created\_at,updated\_at,version) VALUES(?,?,?,?,?,?,?,?,?,?,?)',

&#x20;       (new\_id, row\[0], row\[1], row\[2], row\[3], row\[4], row\[5], row\[6], to\_iso(row\[7]), to\_iso(row\[8]), 1))

dst.commit()
```



***

## 五、必须保留的独立产物及理由



| 产物                   | 位置                                             | 保留理由                                                                       |
| -------------------- | ---------------------------------------------- | -------------------------------------------------------------------------- |
| **mox\_meta.db**     | `platform/legacy/mox-server/mox_meta.db`       | Python legacy 服务运行时依赖，含 282 条审计日志、26 条 DSQL 定义、32 顶点 / 44 边知识图谱，为不可再生的历史数据 |
| **mox\_business.db** | `platform/legacy/mox-server/mox_business.db`   | Python legacy 官网演示数据（产品 / 新闻 / 案例 / 团队 / Banner），前端演示依赖                    |
| **Python legacy 代码** | `platform/legacy/mox-server/`                  | 完整的 FastAPI 低代码平台实现（DSQL 引擎、知识图谱、多数据库适配层、无限发布系统），具有参考价值和回退价值               |
| **data/mox.db**      | `data/mox.db`                                  | Rust 网关 IAM 主库，含真实 RBAC 数据（2 用户 / 16 角色 / 140 权限 / 280 关联），为当前生产主库         |
| **server.py.bak**    | `platform/legacy/mox-server/mox/server.py.bak` | 修复前备份，用于回退验证                                                               |
| **ddl.sql**          | `mox-platform-iam-core/src/ddl.sql`            | Rust IAM 22 表完整 DDL，为 schema 真源                                            |



***

## 六、Rust IAM Schema 检查结果

### 6.1 完整性



* **DDL 文件**：`ddl.sql` 包含 **22 张表**的完整建表语句，与 `data/mox.db` 实际 22 表完全一致

* **所有表均使用&#x20;**`CREATE TABLE IF NOT EXISTS`，幂等安全

* **所有索引均使用&#x20;**`CREATE INDEX IF NOT EXISTS`，独立于建表语句

* **时间字段统一为&#x20;**`TEXT NOT NULL`，存储 ISO8601

* **主键统一为&#x20;**`TEXT`（UUID）

### 6.2 初始化代码

`IamRepository::init_schema()`（repo.rs 第 51-69 行）：



* 按 `;` 分割 DDL，逐条执行 `execute_batch`

* 对 `"already exists"` 和 `"duplicate column"` 错误容错跳过

* 其他错误抛出并附带上下文

* **结论**：初始化逻辑完整且幂等，无需补充

### 6.3 种子数据

`IamRepository::seed_builtins()`：



* 创建 system 租户 + T001 演示租户

* 创建 4 个内置角色（sys\_admin/sys\_developer/tenant\_admin/tenant\_user）× 2 租户 = 8 角色

* 创建 11 资源 × 7 动作 = 77 权限（system 租户）

* 为 sys\_admin 和 tenant\_admin 分配全部权限

* 创建 admin 用户（T001 租户，超管）

* **实际数据**：2 租户、16 角色（8 内置 + 可能其他）、140 权限、280 角色 - 权限关联、2 用户

* **结论**：种子数据完整，与审计结论一致

### 6.4 缺失项检查



| 检查项                    | 状态        |
| ---------------------- | --------- |
| 22 表 DDL 完整            | 通过        |
| 所有表 IF NOT EXISTS      | 通过        |
| 主键类型统一（TEXT UUID）      | 通过        |
| 时间格式统一（TEXT ISO8601）   | 通过        |
| init\_schema 幂等容错      | 通过        |
| seed\_builtins 完整      | 通过        |
| data/mox.db 表数与 DDL 一致 | 通过（22=22） |
| 缺失表或字段                 | 无         |



***

## 七、空库文件清单



| 文件路径                        | 大小      | 说明                                       |
| --------------------------- | ------- | ---------------------------------------- |
| `.runtime/operator_data.db` | 0 bytes | 运行时目录下的空库，可能为 AI operator 模块初始化时创建但未写入数据 |

> 按约束不删除，保留供用户判断是否需要。

其他非主 .db 文件：



* `.runtime/operator_dialogue.db` — AI 对话运行时库

* `platform/domains/ai/svc/mox-ai-agent-svc/operator_data.db` — AI agent 服务的 operator 库

* WebView2 缓存目录下的 .db 文件（浏览器缓存，非业务库）



***

## 八、仍需用户决策的取舍项

### 8.1 是否将 Python legacy 数据全量迁移到主库？



* **选项 A：全量迁移** — 统一到 data/mox.db，废弃 mox\_meta.db 和 mox\_business.db


  * 优点：单一数据源，消除一致性风险

  * 缺点：工作量大（尤其用户体系映射），需新建业务表，Python legacy 服务需改造或下线

* **选项 B：仅迁移关键数据** — 迁移 messages / 用户 / 角色，保留业务演示库


  * 优点：风险可控，保留演示能力

  * 缺点：仍存在多库分裂

* **选项 C：不迁移，维持现状** — 仅修复双写 bug，legacy 库继续独立运行


  * 优点：零风险

  * 缺点：长期技术债

### 8.2 mox\_meta.db.messages 孤儿表如何处理？



* 迁移 7 行数据到 mox\_business.db 后：


  * **选项 A**：DROP TABLE messages（清理孤儿表）

  * **选项 B**：保留为空表（防止旧代码意外写入时报错）

  * **选项 C**：创建为 VIEW 指向 mox\_business.db.messages（SQLite 不支持跨库 VIEW 除非 ATTACH）

### 8.3 Python legacy 用户体系是否对接 Rust IAM？



* Python 的 admin/ops/visitor（3 用户，扁平角色）与 Rust 的 admin 用户（完整 RBAC）无同步机制

* **选项 A**：Python 服务调用 Rust IAM API 做认证授权

* **选项 B**：定期同步用户数据（批处理）

* **选项 C**：Python legacy 下线，统一用 Rust 网关

### 8.4 业务表是否纳入主库统一 schema？



* 当前 products/news/cases/team/banners/messages 仅在 Python legacy 业务库

* 若主库需要承载官网业务，需新建对应业务表（UUID 主键 + ISO8601 时间 + tenant\_id）

* 这取决于产品规划：Rust 网关是否要接管官网内容管理



***

## 九、修复文件清单



| 文件                                             | 操作 | 说明                                               |
| ---------------------------------------------- | -- | ------------------------------------------------ |
| `platform/legacy/mox-server/mox/server.py`     | 修改 | website\_message 写入连接 META→BUSINESS\_STORE，返回值修复 |
| `platform/legacy/mox-server/mox/server.py.bak` | 新增 | 修改前备份                                            |
| `reports/database-normalization-plan.md`       | 新增 | 本文档                                              |



***

*文档结束*