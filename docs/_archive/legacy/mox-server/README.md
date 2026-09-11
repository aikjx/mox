# mox-server · 无限发布系统管理中心中台

企业级低代码平台的可运行实现。定位为**无限发布系统管理中心中台**：
多应用（企业官网/业务系统）的创建、SQL 动态配置、发布上线、在线处理与 AI 辅助全部在一个中台完成。
核心引擎：**mox-dsql-core（数据库管理动态 SQL）** + **mox-kg-core（自研知识图谱）** + **mox-apps（应用发布）** + **mox-ai（智能助手）**。

## 一、能力总览

| 能力 | 说明 | 实现 |
|---|---|---|
| 无限发布 · 多应用 | 创建/发布/下线任意数量企业官网应用，独立 SQL 集与域名，状态机 draft→prepared→published→running→offline | `mox/apps_core.py` |
| 数据库管理 SQL | 所有业务 SQL 以"定义"存于元数据库，动态配置/发布/版本化/启停，改 SQL 无需改代码 | `mox/dsql_core.py` |
| 动态模板 | `{{param}}` 参数占位、`{% if param %}…{% endif %}` 条件片段、`{{limit}}` 整数分页 | 轻量模板渲染器 |
| 多级缓存 | sql_code+参数+角色 维度缓存，命中即返回（比硬编码查询更快）；内存 LRU+TTL，可一键切 Redis | `mox/cache.py` |
| 支持所有数据库 | 中间层适配器驱动：SQLite/MySQL/PostgreSQL/DuckDB，改配置即可切换，业务零改动 | `mox/db_adapters.py` |
| 字段级权限 | 资源(SQL)×角色 配置可见字段白名单，结果集列过滤 + 手机/邮箱/证件自动脱敏 | `mox/dsql_core.py` |
| 自研知识图谱 | 实体关系图、邻接遍历、多跳可达、最短路径、跨行业(domain)融合，无限扩展 | `mox/kg_core.py` |
| AI 智能助手 | 自然语言→SQL 模板、SQL 结构解释、优化建议、一键试运行；内置规则引擎零依赖，配置 MOX_LLM_URL 可切换大模型 | `mox/ai_core.py` |
| 业务流程引擎 | 需求→创建→数据源→SQL→装配→测试→发布→监控→下线 全链路 9 阶段，每步输入/处理/输出/验收明确 | `mox/process.py` |
| 安全护栏 | 只读白名单（仅 SELECT/WITH）、拦截写语句/多语句、参数全部绑定防注入 | `mox/db_adapters.py` |
| 全链路可观测 | 每次请求返回 trace_id / duration_ms / cache_hit，审计日志 + AI 请求日志落库 | `mox/server.py` |

## 二、快速启动

```bash
cd platform/mox-server
pip install -r requirements.txt        # fastapi + uvicorn 已装则跳过
python run.py 8600                      # 默认 0.0.0.0:8600
```

- 健康检查：`GET http://127.0.0.1:8600/api/health`
- 平台概览：`GET http://127.0.0.1:8600/api/stats`
- 首次启动自动建库并注入种子：SQL 定义 16 条、图谱 16 顶点/15 关系、角色/字段权限、官网业务数据。

## 三、一键启动脚本（Windows PowerShell）

```powershell
# 启动并保持运行
cd D:\a10\aikjx\gitcode\infotopograph\platform\mox-server
Start-Process python -ArgumentList 'run.py','8600' -WindowStyle Hidden
# 停止
$c = Get-NetTCPConnection -LocalPort 8600 -State Listen
$c | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force }
```

## 四、配套前端

| 前端 | 路径 | 说明 |
|---|---|---|
| 企业官网 | `frontend-ui/mox-website/index.html` | 低代码能力落地页，已对接真实后端（后端离线自动回退 mock） |
| 低代码配置台 | `frontend-ui/mox-console/index.html` | SQL 定义管理 / 字段权限 / 数据源 / 图谱 / 缓存审计 |

> 官网与配置台均通过浏览器直接打开 file:// 即可，CORS 已全开，自动连接 `http://127.0.0.1:8600`。

## 五、API 一览

### DSQL 执行
| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/api/dsql/execute` | `{sql_code, params, role, use_cache}` → 执行动态 SQL |
| POST | `/api/dsql/execute-batch` | `{items:[{sql_code,params}]}` 批量执行 |
| POST | `/api/dsql/explain` | 渲染后的 SQL + 绑定参数 + 字段权限预览（不执行） |

### SQL 定义管理
| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/api/admin/sqls` | 定义列表 |
| POST | `/api/admin/sqls` | 新建/更新（入库前自动语法+安全校验） |
| POST | `/api/admin/sqls/{code}/status` | draft/published/disabled |
| POST | `/api/admin/sqls/{code}/test` | 试运行 |
| DELETE | `/api/admin/sqls/{code}` | 删除 |

### 字段级权限
| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/api/admin/permissions` | 权限列表 |
| POST | `/api/admin/permissions` | `{resource, role, allowed_fields}` 设置（留空=全部） |
| GET | `/api/admin/roles` `/api/admin/users` | 角色/用户 |

### 数据源（中间层）
| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/api/admin/datasources` | 数据源列表 |
| POST | `/api/admin/datasources` | `{name, driver, config}` 新增 |
| POST | `/api/admin/datasources/{name}/reload` | 重载适配器 |

### 知识图谱
| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/api/kg/graph` | 全量可视化子图 |
| POST | `/api/kg/query` | `dsl: graph / neighbors:<vid> / reachable:<vid>:<hops> / path:<a>\|<b> / stats` |
| POST | `/api/kg/traverse` | `{vertex_id, direction}` |
| POST | `/api/admin/kg/vertices` | 顶点新增 |
| DELETE | `/api/admin/kg/vertices/{vid}` | 删除顶点(级联边) |
| POST | `/api/admin/kg/edges` | 关系新增 |

### 无限发布 · 应用管理
| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/api/apps` | 应用列表（含 SQL 数/发布版本） |
| POST | `/api/apps` | 创建应用（app_key 全局唯一） |
| PUT | `/api/apps/{app_key}` | 更新应用 |
| DELETE | `/api/apps/{app_key}` | 删除应用（默认 mox 受保护） |
| POST | `/api/apps/{app_key}/transition` | 状态机流转（发布即 version++） |
| GET | `/api/apps/{app_key}/logs` | 发布日志 |

### AI 智能助手 / 业务流程
| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/api/ai/assistant` | `{message, app_key}` → 自然语言生成 SQL / 解释 / 优化建议 |
| GET | `/api/ai/requests` | AI 请求日志 |
| GET | `/api/process/flow` | 无限发布系统全链路业务流程（9 阶段） |

### 缓存 / 审计 / 官网
| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/api/cache/stats` | 缓存命中统计 |
| POST | `/api/cache/clear` | 清空缓存 |
| GET | `/api/audit?limit=50` | 审计日志 |
| POST | `/api/website/message` `/resume` `/consultation` | 官网写接口（落业务库） |

## 六、模板语法速查

```sql
-- 参数占位（自动 ? 绑定，防注入）
SELECT * FROM products WHERE id = {{id}}

-- 条件片段（参数有值才包含）
SELECT id,name,category FROM products
WHERE 1=1
{% if category %} AND category = {{category}} {% endif %}

-- 分页（整数内联）
SELECT * FROM products ORDER BY id LIMIT {{limit}} OFFSET {{offset}}

-- 模糊搜索（同一参数出现几次即绑定几次）
SELECT ... FROM products WHERE name LIKE {{keyword}} OR summary LIKE {{keyword}}
```

## 七、切换数据库 / 缓存

1. 数据库：`POST /api/admin/datasources` 新增 `{name, driver:'mysql'|'postgres'|'duckdb', config:{host,port,user,password,database}}`，业务 SQL 零改动。
2. 缓存：改 `mox/server.py` 中 `build_cache(driver="redis")` 并 `pip install redis`，即切换 Redis 缓存。

## 八、测试

```bash
python smoke_test.py   # 引擎级冒烟（14 项：模板/权限/缓存/注入/图谱）
python api_test.py     # API 层全接口（26 项）
```
