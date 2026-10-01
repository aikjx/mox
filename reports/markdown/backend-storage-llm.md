# 后端补路由：Storage / LLM 端点族真实能力实现报告

> 日期：2026-09-26 · 网关：mox-platform-gateway-svc（3080）· 构建：`cargo build -p mox-platform-gateway-svc` 通过（42.78s，无 error / 无 unused warning）
> 硬约束：**禁桩**——只接真实底层能力，没有的如实报告，绝不造硬编码全零/模板文案占位。

---

## 0. 结论速览

| 族 | 前端期望端点数 | 本次真实实现 | 如实报告做不了（需前置工程） |
| --- | --- | --- | --- |
| Storage `/storage/*` + `/modules` | 4 | **4（全部真实）** | 多后端切换抽象层（S3/OSS/COS） |
| LLM `/llm/*` | 16（含写/测试/统计/日志） | **4 只读（真实 env 投影）** | Provider CRUD/启停/设默认/连通性测试/模型发现/用量/日志/路由写 |
| Web-search `/web-search/*` | 4 | **0** | 互联网搜索引擎适配层（无任何后端） |

- 新增路由：**8 条**，网关注册表 `ROUTES` 由 228 → **236**，`/actuator/mappings` 实测 `total=236`。
- 信封：统一 `{code:0,msg:"ok",data:{...}}`（`mox_api_protocol::ApiResponse`），与 experts.*/alliance.* 一致；前端 http.js 自动剥 `data`。
- 鉴权：全部挂受保护路由组，无 token → 401（实测）；`Bearer dev-secret-token` 放行（dev_mode）。

---

## 1. 能力来源分析（每个端点：有/无底层能力 + 证据源码路径）

### 1.1 前端 API 清单提取（AdminStorage.vue 120 行 / AdminLlm.vue 1364 行）

前端 `baseURL=/api`，经 Vite 代理到网关 3080，故网关侧路径 = `/api` + 前端路径。

**Storage（system.api.js）：**
- `GET /storage/status` → `{provider, name, totalEntities, entitiesByType:[{entity_type,cnt}], features:{k:bool}}`
- `GET /storage/providers` → `[{name|id, description|desc|type}]`
- `POST /storage/switch` body `{provider}`
- `GET /modules` → `[{name, description, version, routes}]`

**LLM（llm.api.js）：**
- `GET /llm/providers` → `[{id,name,type,base_url,model,enabled,active,has_key,api_key?,...}]`
- `GET /llm/providers/presets` → `[{id,name,base_url,models:[],description}]`
- `GET /llm/providers/:id` · `POST /llm/providers` · `PUT /llm/providers/:id` · `DELETE /llm/providers/:id`
- `POST /llm/providers/active{provider_id}` · `POST .../:id/enable|disable|test|discover`
- `GET /llm/health` · `GET|PUT /llm/routing` · `GET /llm/usage` · `GET /llm/logs?limit` · `GET /llm/stats`

**Web-search（ai.api.js）：**
- `GET /web-search/config` → `{config:{enabled,engine,api_key_masked,base_url,max_results,timeout_ms}, engines:[{id,name,needKey}], ready}`
- `POST /web-search/config` · `POST /web-search/test` → `{success,message}` · `POST /web-search{query}`

### 1.2 后端真实能力探查结论

| 底层能力 | 证据路径 | 结论 |
| --- | --- | --- |
| 本地磁盘对象存储 | `platform/gateway/mox-platform-gateway-svc/src/cloud.rs`（`CloudState`，根 `data/storage`，bucket/object 增删查，`put_object_text`） | **真实存在**，可投影 status/providers |
| 多 Provider 存储后端抽象（S3/OSS/COS/MinIO） | 全仓检索无任何远程 client/抽象 trait | **不存在** |
| LLM Provider 配置 | `platform/domains/ai/svc/mox-ai-expert-svc/src/llm/chat.rs`（`LlmConfig::from_env()` / `ProviderConfig` / `RoutingStrategy`） | **真实存在但 env 驱动**：`MOX_LLM_PROVIDERS` + `MOX_LLM_{ID}_*` / `MOX_LLM_API_KEY` / `DEEPSEEK_API_KEY` / `OPENAI_API_KEY` / `MOX_LLM_ROUTING_STRATEGY` |
| LLM 配置持久化（DB/文件写层） | 全仓无；`llm_governance.rs` 仅为后验治理纯计算，无路由 | **不存在**（写操作族无根） |
| LLM 调用遥测/日志/用量 | 网关无 LLM 调用日志采集 | **不存在** |
| 编排器 3001 的 `/api/llm/*` `/web-search/*` | `mox-ai-expert-svc/src/server.rs` 路由仅 `/api/alliance/*` `/api/optimize` 等，无 llm/web-search | **不存在** |
| 互联网搜索引擎客户端（DuckDuckGo/SearXNG/Tavily） | 全仓 grep 命中均为 KG/语义/索引 `search()`，无 HTTP 联网搜索 | **不存在** |

---

## 2. 本次实现的端点（8 条，全部真实）

新增源码：
- `platform/gateway/mox-platform-gateway-svc/src/admin_storage.rs`（存储管理面）
- `platform/gateway/mox-platform-gateway-svc/src/admin_llm.rs`（LLM 只读管理面）
- `cloud.rs` 增补 `CloudState::root_dir()` 只读访问器
- `lib.rs` 登记 `pub mod admin_storage / admin_llm`
- `modules.rs::build_module_routers` 挂载两个新路由单元
- `actuator.rs::ROUTES` 登记 8 条 + 数组尺寸 228→236

### 2.1 Storage 族

| ID | 方法 | 路径 | 数据来源（真实） |
| --- | --- | --- | --- |
| `storage.status` | GET | `/api/storage/status` | 实时读 `CloudState.root_dir()`：遍历 bucket（子目录）统计对象数/字节 |
| `storage.providers` | GET | `/api/storage/providers` | 真实可用提供方清单——当前**仅 local**（本地磁盘），远程后端不臆造为可用 |
| `storage.switch` | POST | `/api/storage/switch` | 仅接受 `local`（幂等无操作）；对无后端提供方返回 **409**，不假装切换成功 |
| `storage.modules` | GET | `/api/modules` | 投影 `routes::DOMAINS` 域自描述注册表（46 域），`routes` 列填挂载前缀 |

### 2.2 LLM 只读族

| ID | 方法 | 路径 | 数据来源（真实） |
| --- | --- | --- | --- |
| `llm.providers` | GET | `/api/llm/providers` | `LlmConfig::from_env()` 实时读取；**API Key 绝不回传**，仅 `has_key:bool` |
| `llm.provider_presets` | GET | `/api/llm/providers/presets` | 已知公开 Provider 参考数据（官方 Base URL/模型清单，静态元数据，非运行态） |
| `llm.health` | GET | `/api/llm/health` | env 是否配置 Provider/路由策略；未配置时如实报 `engine:local_rules_fallback` |
| `llm.routing` | GET | `/api/llm/routing` | 读 `MOX_LLM_ROUTING_STRATEGY` + provider id 列表 |

---

## 3. curl E2E 实测（网关 3080，Bearer dev-secret-token，2026-09-26 实测）

基线（重启前）：`GET /api/storage/status` → **404**（证实此前零路由）。重启新二进制后：

```text
[1] GET /api/storage/status
{"code":0,"msg":"ok","data":{"bucket_count":2,"entitiesByType":[{"cnt":0,"entity_type":"demo-bucket"},
 {"cnt":3,"entity_type":"dialogue"}],"features":{"object_storage":true,"multi_bucket":true,
 "s3_compatible_semantics":true,"s3_native_api":false,"aliyun_oss":false,"tencent_cos":false,"minio":false},
 "name":"本地磁盘对象存储","provider":"local","root":"D:\\...\\data/storage",
 "totalEntities":3,"used_bytes":9248}}
# ↑ 真实读盘：dialogue bucket 3 个对象、9248 字节，与 data/storage 实际内容一致

[2] GET /api/storage/providers
{"code":0,"data":[{"id":"local","name":"local","type":"disk","available":true,
 "description":"本地磁盘对象存储（S3 兼容语义；根目录 data/storage）"}]}

[3] POST /api/storage/switch  {"provider":"local"}   → 200
{"code":0,"data":{"provider":"local","switched":true,"note":"当前唯一真实后端即本地磁盘，切换为幂等无操作。"}}

[4] POST /api/storage/switch  {"provider":"s3"}      → 409
{"code":409,"msg":"存储提供方「s3」无后端实现（当前仅 local 本地磁盘可用）；需先建设存储抽象层与远程后端"}

[5] GET /api/modules            → code:0, 46 条（Health/Metrics/IAM/...，version=3.0.0-ai-powered）

[6] GET /api/llm/providers      → {"code":0,"data":[]}   （当前进程未配 MOX_LLM_* env，如实空）
[7] GET /api/llm/providers/presets → code:0, 8 条（deepseek/openai/qwen/zhipu/volcengine/anthropic/google/ollama）
[8] GET /api/llm/health
{"code":0,"data":{"configured":false,"providers":0,"enabled":0,"with_key":0,"routing_strategy":"priority",
 "engine":"local_rules_fallback","note":"未配置 MOX_LLM_* 凭据，专家链路回退本地规则引擎"}}
[9] GET /api/llm/routing
{"code":0,"data":{"strategy":"priority","providers":[],"fallback":true,"load_balance":false,
 "note":"未配置 LLM Provider，路由策略为默认 priority"}}

[鉴权] 无 Authorization 头 → GET /api/storage/status = 401

[注册表] GET /actuator/mappings → total=236；?q=storage 命中 storage.status/providers/switch/modules
```

> 说明：当前运行进程未注入 `MOX_LLM_*` 凭据，故 `/llm/providers` 为空、`health.configured=false`——这是**真实状态**而非占位。配置方式：启动前设 `MOX_LLM_PROVIDERS=deepseek,openai` + 对应 `MOX_LLM_DEEPSEEK_API_KEY/_BASE_URL/_MODEL` 等，重启网关后即被 `/llm/providers` 真实读出（`has_key=true`，Key 不回传）。

---

## 4. 前端面板联通结论（代码级字段核对）

| 前端读取字段 | 后端返回 | 对齐 |
| --- | --- | --- |
| `status.provider / name / totalEntities / entitiesByType[].entity_type,cnt / features{}` | 同名 key（camelCase 为主，附 snake 别名） | ✅ |
| `providers[].name\|id / description / type` | `id,name,type,description,available` | ✅ |
| `modules[].name/description/version/routes` | `name,description,version,routes(前缀),layer,status` | ✅ |
| `llm providers[].id/name/type/base_url/model/enabled/active/has_key` | 同名；`active`=env 列表首个；`has_key` 替代明文 Key | ✅ |
| `presets[].id/name/base_url/models[]/description` | 同名 | ✅ |
| `routing.strategy/providers/fallback/load_balance` | 同名（`weights` 未回传，前端无权重时不渲染，安全） | ✅ |
| `webSearchConfig.config{}` + `engines[]` + `ready` | **未实现（缺口）** | ⛔ |

结论：**Storage 面板三卡片（存储状态/提供方切换/已加载模块）可真实联调**；LLM 面板的渠道列表、预设、健康概况、路由策略只读区可真实联调；联网搜索配置区因后端缺口保持"未就绪"降级（前端 `loadWebSearchConfig` 已 `catch` 静默降级，不阻塞页面）。

---

## 5. 遗留缺口（做不了 + 为什么 + 需要什么前置工程）

### 5.1 Storage：多后端切换
- **现状**：仅 local 本地磁盘一个真实后端；`/storage/switch` 对 S3/OSS 等返回 409。
- **前置工程**：建设存储后端抽象层（trait `ObjectStore`：put/get/list/delete）+ 各远程 adapter（AWS S3 SDK / Aliyun OSS / Tencent COS / MinIO）+ 连接配置（凭据/endpoint）持久化，再谈"切换"。

### 5.2 LLM：写操作族（CRUD/启停/设默认/测试/模型发现）
- **现状**：配置由进程 env 在启动时决定，**无运行时持久化层**；`std::env::set_var` 仅进程内生效、重启即丢、且与专家链路 `from_env()` 读取口径不一致。
- **前置工程**：LLM 配置持久化（SQLite/JSON 文件仓储）+ 运行时热加载 + 真实 `OpenAI /models` 发现与 `/chat/completions` 连通性探测（需 reqwest 异步调用，带超时与脱敏结果）。

### 5.3 LLM：用量 / 统计 / 日志
- **现状**：网关无 LLM 调用日志采集管线。
- **前置工程**：在 `llm_consultant`/`chat` 调用点埋点 → 环形缓冲/日志仓储，再暴露聚合。

### 5.4 Web-search `/web-search/*`
- **现状**：无任何互联网搜索引擎客户端（前端期望 DuckDuckGo/SearXNG/Tavily）。
- **前置工程**：引入搜索 adapter（HTTP client 调 SearXNG 实例或 Tavily/DuckDuckGo 接口）+ 配置持久化 + `ready` 探针。当前**不实现、不伪造**。

---

## 6. 改动文件清单（本次提交范围）

```
platform/gateway/mox-platform-gateway-svc/src/admin_storage.rs      (新增)
platform/gateway/mox-platform-gateway-svc/src/admin_llm.rs          (新增)
platform/gateway/mox-platform-gateway-svc/src/cloud.rs               (+root_dir() 只读访问器)
platform/gateway/mox-platform-gateway-svc/src/lib.rs               (+pub mod)
platform/gateway/mox-platform-gateway-svc/src/modules.rs             (+挂载两路)
platform/gateway/mox-platform-gateway-svc/src/actuator.rs          (+8 路由, ROUTES 228→236)
scripts/doc/gen-api-registry.py                                     (+storage/llm 域 order/IMPL)
docs/API-REGISTRY.md                                                (重生成, 236 条)
reports/markdown/backend-storage-llm.md                             (本报告)
```
