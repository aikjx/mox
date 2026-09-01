# 开发专家联盟·核心服务端口规划规范（PORT-NORM-001）

> **标题**：开发专家联盟·核心服务端口规划规范
> **版本**：V1.3
> **权威等级**：🟢权威
> **编号**：PORT-NORM-001
> **最后更新日期**：2026-09-01
> **单源声明**：本文档是"开发专家联盟"全部核心服务与插件/小服务端口分配的**唯一权威规范**。凡涉及端口规划、端口分配、端口避让、端口迁移的决策与文档，均以本文档为准。本文档冲突时以 `docs/enterprise/18-全域顶层总设计-三联盟模式-V1.0.md`（TOP-MASTER）为准。
> **主责联盟**：开发联盟 R（架构·代码·文档治理）
> **编制依据**：`docs/expert-alliance/00-INTEGRATED-INDEX.md`、`docs/expert-alliance/02-DUAL-PLATFORM-RELATIONSHIP.md`、`docs/standards/expert-alliance-normalization-mode.md`（EA-NORM-001）
> **V1.1 变更**：新增 §7.7 配置错误 fail-fast、§7.8 环境变量归一化（deprecated 兼容）、§7.9 命名约定
> **V1.2 变更**：新增 §7.10 HTTP 专家桥接激活（expert_service.enabled）、env 映射表补 EXPERT_SERVICE_ENABLED
> **V1.3 变更**：新增 §7.11 Nacos 阶段二（ConfigStore 抽象 + NacosConfigStore，boot-config `nacos` feature）、§7.12 executor 生产专家模式真实 LLM 调用链（blocking client 延迟创建修复）；registry.rs 双套专家集归一化（删除废弃 domain_experts，单一权威=config-core `build_domain_experts`）

---

## 第1章 总则与强制约束

### 1.1 端口段强制规则

本规范对开发专家联盟**全部核心服务**实施端口强制约束：

| 段位 | 范围 | 用途 | 约束级别 |
|---|---|---|---|
| **核心服务段** | **3000–3999** | 所有核心服务（调度、执行、专家、网关等）HTTP 监听端口 | **强制** |
| **插件/小服务段** | **30000–39999** | 插件、辅助小服务、工具进程 | 强制（新增小服务必须落入此段） |
| 其他段位 | 4000+ / 8000+ / 0–2999 | 禁止核心服务占用 | 禁止 |

**强制约束**：
1. 任何新增/修改的核心服务端口必须落在 `3000–3999`，否则拒绝合并。
2. 任何新增插件/小服务端口必须落在 `30000+`。
3. 端口分配必须通过本规范第5章的避让校验，不得占用常用软件端口。

### 1.2 设计原则

1. **易记**：端口按"段号（两位）+ 序号（两位）"编码，00 结尾为主端口，语义与业务域对应（31=编排、32=执行、33=领域专家）。
2. **避让**：绝不占用 3000–3999 区间内的常用软件端口（见第3章避让清单）。
3. **唯一**：同一端口全局唯一，禁止一端口多服务。
4. **单源**：端口分配以本文档为准，代码与文档必须与本文档一致（EA-NORM-001 §6 文档-代码对齐要求）。

---

## 第2章 核心服务端口分配表（3000–3999）

### 2.1 当前已分配

| 端口 | 服务 | 业务域 | 说明 | 状态 |
|---|---|---|---|---|
| **3010** | Node.js 平台 API（api） | 平台层（30xx） | Node.js 层统一入口（Express）；已由 Rust 网关 **:8080** 取代 | 已退役 |
| **3020** | Node.js 前端（frontend） | 平台层（30xx） | 前端开发/静态服务 | 已启用 |
| **3100** | **scheduler-svc**（调度编排） | 编排（31xx） | 任务调度、专家匹配、计划生成 | 已启用 |
| **3200** | **executor-svc**（执行引擎） | 执行（32xx） | DAG 执行、节点调度 | 已启用 |
| **3300** | AI 专家服务（ai-expert 桥接） | 领域专家（33xx） | scheduler 内部桥接的专家服务基地址 | 已启用 |
| **8080** ⚠️例外 | **Rust 平台网关**（api / mox-gateway） | 兼容段（例外） | Rust axum 单二进制 HTTP 入口，为全平台唯一对外 API；端口因历史兼容（原 Python mox-server / 部署链路）**钉死为 8080**，详见注 2.1a | 已启用（例外） |

> **注 2.1a（8080 例外说明）**：本规范 1.1 要求核心服务端口落在 3000–3999，但 **Rust 平台网关 api=8080** 为**全平台唯一对外 HTTP 入口**，且已同步固化于 `platform_config.json`、`deploy/config/gateway.yaml`、`mox-workspace/.env.example`、`frontend-ui/vite.config.js`、docker-compose/helm 等全链路，迁移成本与风险极高。故**特批为例外**：8080 为网关保留端口，任何其他服务禁止占用；若未来整体迁移到 3xxx，须走第5章变更流程并同步全链路。

### 2.2 段位预留

| 段位 | 含义 | 状态 |
|---|---|---|
| 3000–3009 | 平台层保留（**3000 禁用于 Grafana 冲突**） | 预留 |
| 3030–3099 | 平台层扩展（网关、鉴权等） | 预留 |
| 3100–3199 | 调度编排域扩展 | 预留 |
| 3200–3299 | 执行域扩展 | 预留 |
| 3300–3399 | 领域专家/配置域扩展（**避开 3306 MySQL**） | 预留 |
| 3400–3999 | 其他核心域扩展 | 预留 |

---

## 第3章 端口避让清单（3000–3999 常用软件，禁止占用）

以下端口为常见软件/服务默认端口，**核心服务严禁占用**。本清单是第5章避让校验的硬性依据。

| 端口 | 常用软件/协议 | 端口 | 常用软件/协议 |
|---|---|---|---|
| 3000 | Grafana | 3333 | 各种开发服务器（如 Rails） |
| 3001 | Gitea | 3389 | RDP（远程桌面） |
| 3004 | 部分 IoT 网关 | 3443 | HTTPS 备用端口 |
| 3030 | 部分 API | 35729 | LiveReload |
| 3050 | Firebird 数据库 | 3690 | SVN |
| 3128 | Squid 代理 | 3894 | 部分消息服务 |
| 3268 | LDAP 全局编目 | 3900 | 部分管理服务 |
| 3306 | **MySQL** | 3968 | 部分监控服务 |
| 3310 | ClamAV | 4000 | 部分框架（超出 3xxx，仅参考） |

> 说明：避让以"与最常用的软件、应用不冲突"为总原则；若某端口在部署环境已占用，须走第5章变更流程重新分配。

---

## 第4章 插件/小服务段规划（30000+）

### 4.1 段位细分

| 子段 | 用途 | 示例 |
|---|---|---|
| 30000–30099 | 通用插件 / 工具小服务 | 语音服务、翻译代理、渲染插件 |
| 30100–30199 | 领域插件（按 31-39 领域细分） | 金融插件、医学插件 |
| 30200–30299 | 集成桥接 / 适配器 | 第三方系统适配器 |
| 30300–39999 | 扩展保留 | 预留 |

### 4.2 已规划分配

| 端口 | 服务 | 状态 |
|---|---|---|
| **30010** | 语音服务（xiaobai_voice，原 3717） | **已启用**（2026-09-01 由 3717 迁入，落段 30010） |
| 30001–30009 | 通用插件保留 | 预留 |

> 迁移说明：语音服务为辅助小服务，按本规范落入 30000+ 段。已按 PORT-REGISTRY-001 于 2026-09-01 完成 **3717 → 30010** 全链路迁移（Python 服务、Rust `mox-voice-operator-svc::voice_server`、orchestrator `voice_proxy` 上游、`platform_config.json`、`server-manage.py`、桌面端、校验脚本），旧端口 3717 标记为 DEPRECATED。历史文档（ARCHITECTURE/enterprise 报告等）仍可能显示旧端口 3717，以本文档与本表为准。

---

## 第5章 端口变更流程

1. **申请**：提出新服务端口需求，说明服务名、业务域、用途。
2. **落段**：核心服务 → 按第2章段位表选 3000–3999 空闲端口；插件/小服务 → 选 30000+ 空闲端口。
3. **避让校验**：对照第3章避让清单 + 本机 `netstat -ano | findstr LISTENING` 实查占用，确认不冲突。
4. **登记**：在本文档第2章/第4章表中登记（端口、服务、域、状态）。
5. **同步**：同步更新代码（`main.rs` 监听端口、SDK 默认基址、桥接默认 URL）与 `docs/expert-alliance/00-INTEGRATED-INDEX.md`。
6. **验证**：重新编译 + 启动服务 + 健康检查确认新端口生效。

---

## 第6章 本次端口迁移记录（2026-08-31）

| 服务 | 原端口 | 新端口 | 迁移原因 | 涉及代码 |
|---|---|---|---|---|
| scheduler-svc | 8081 | **3100** | 核心服务强制 3xxx 段 | `svc/.../bin/main.rs`、SDK `client.rs` 默认基址 |
| executor-svc | 8082 | **3200** | 核心服务强制 3xxx 段 | `svc/.../bin/main.rs`、`scheduler-core/executor_bridge.rs`、`server.rs` 默认桥接 |
| AI 专家服务（桥接） | 8080 | **3300** | 核心服务强制 3xxx 段 | `scheduler-core/registry.rs` 默认基址 |
| Node.js api / frontend | 3010 / 3020 | api 退役→Rust 网关 **8080**（例外）；frontend 3020 不变 | Node BFF 迁移至 Rust 网关（8080 例外见 2.1a） | `platform_config.json`、`vite.config.js` |
| 语音服务 xiaobai_voice | 3717 | **30010**（已迁移） | 小服务归 30000+ 段 | `platform_config.json`、`cli.py`、Rust `voice_server`、orchestrator `voice_proxy`、桌面端、`verify_tts_rust_fullstack.py` 等全链路 |

> 迁移后同步更新：`docs/expert-alliance/00-INTEGRATED-INDEX.md`（6 处）、`02-DUAL-PLATFORM-RELATIONSHIP.md`（8 处）、`03-GLOSSARY.md`（2 处）、`v2/*`（56 处）——共 72 处文档端口引用，全部对齐新端口，引用审计零残留。

---

## 第7章 yml 配置外部化（Nacos 配置中心地基）

> 本规范要求核心服务配置**外部化到 yml 文件**（配置中心化第一层），为后续接入 Nacos 配置中心打地基。

### 7.1 配置文件

| 服务 | 配置文件 | 说明 |
|---|---|---|
| scheduler-svc | `config/alliance-scheduler.yml` | 服务器、调度参数、执行器桥接、专家服务、存储 |
| executor-svc | `config/alliance-executor.yml` | 服务器、执行器模式与参数、存储 |

### 7.2 加载优先级（从低到高）

1. **内置默认值**（与 PORT-NORM-001 端口一致：scheduler=3100 / executor=3200 / 专家=3300）
2. **yml 文件**（`config/alliance-*.yml`）
3. **环境变量** `MOX_ALLIANCE_*`（如 `MOX_ALLIANCE_SERVER_PORT`），yml 未配置的字段用默认，env 覆盖 yml

配置文件路径可用 `MOX_ALLIANCE_CONFIG_FILE` 覆盖（默认 `config/alliance-scheduler.yml` / `config/alliance-executor.yml`）。

### 7.3 环境变量映射表（MOX_ALLIANCE_*）

| 环境变量 | 配置路径 | 说明 |
|---|---|---|
| `MOX_ALLIANCE_SERVER_HOST` / `_PORT` | `server.host` / `server.port` | 监听地址/端口 |
| `MOX_ALLIANCE_SCHEDULER_MAX_CONCURRENT_TASKS` | `scheduler.max_concurrent_tasks` | 最大并发任务 |
| `MOX_ALLIANCE_SCHEDULER_QUEUE_CAPACITY` | `scheduler.queue_capacity` | 队列容量 |
| `MOX_ALLIANCE_SCHEDULER_DEFAULT_PRIORITY` / `_MODE` / `_FUSION_STRATEGY` | `scheduler.default_*` | 默认优先级/模式/融合 |
| `MOX_ALLIANCE_EXECUTOR_BRIDGE_BASE_URL` / `_TIMEOUT_MS` | `executor_bridge.*` | 执行器桥接地址/超时 |
| `MOX_ALLIANCE_EXPERT_SERVICE_BASE_URL` / `_TIMEOUT_MS` | `expert_service.*` | AI 专家服务地址/超时 |
| `MOX_ALLIANCE_EXPERT_SERVICE_ENABLED` | `expert_service.enabled` | 是否启用 HTTP 专家桥接（true/false/1/0） |
| `MOX_ALLIANCE_STORAGE_MODE` / `_PATH` | `storage.mode` / `storage.path` | 存储模式/路径 |
| `MOX_ALLIANCE_EXECUTOR_MODE` | `executor.mode` | 执行器模式（expert/mock） |
| `MOX_ALLIANCE_EXECUTOR_MAX_CONCURRENT_NODES` 等 | `executor.*` | 执行器参数 |

> 兼容：旧 `EXECUTOR_MODE` 仍生效（显式设置时优先于 yml）；实现见 `mox-alliance-boot-config` crate。

### 7.4 与 Nacos 的演进关系

本规范第7章的 yml 文件即未来 Nacos 配置中心的 **dataId**（`mox-alliance-scheduler.yml` / `mox-alliance-executor.yml`）：
- 阶段一（当前）：本地 yml + 环境变量覆盖（`mox-alliance-boot-config`）
- 阶段二：`ConfigStore` 抽象 → `NacosConfigStore`（nacos-sdk-rust `ConfigService` 拉取 + watch 热更新），见 `docs/microservices/02-communication.md` §6.3
- 阶段三：`NamingService` 注册中心（服务发现，规模化）

### 7.5 yml 配置清单（2026-08-31 落地）

- ✅ `config/alliance-scheduler.yml`、`config/alliance-executor.yml`（本规范 7.1）
- ✅ 新 crate `mox-alliance-boot-config`（yml 加载 + `MOX_ALLIANCE_*` env 覆盖，含 3 单测）
- ✅ 两个 `main.rs` 已接入 yml（端口/调度参数/桥接/存储/执行器模式全部外部化）
- ✅ 修复 executor-core `fusion.rs` 历史编译问题（Node 导入 + 无 `Task.fusion_result` 字段），全量回归 251 passed / 0 failed
- ✅ 端到端验证：yml 启动双服务（3100/3200）、env 覆盖（3199/memory/3299 桥接）、任务 completed 5/5

### 7.6 专家配置外部化（config/alliance-experts.yml）

> 在服务引导配置之外，将**专家模块配置**（全局 LLM + 领域专家）从代码写死改为 yml 覆盖式合并，
> 实现配置外部化的"专家维度"闭环。

| 项 | 说明 |
|---|---|
| 配置文件 | `config/alliance-experts.yml`（路径可用 `MOX_ALLIANCE_EXPERTS_FILE` 覆盖） |
| 全局 LLM | `global_llm` **局部覆盖**（primary_provider/primary_model/fallback_chain/routing_strategy/model_config），未写字段继承内置 |
| 模块覆盖 | `modules` 按 `module_id` 与内置 10 大专家**合并**（name/version/llm_config/graph_config/capability_weights/matching_weights/enabled/tags 可覆盖） |
| 新增模块 | yml 可引入内置不存在的 `module_id`（以默认值 + 覆盖字段创建） |
| 时间戳 | `created_at`/`updated_at` 由系统生成，yml 免填 |
| 实现 | `mox-alliance-boot-config::load_experts` + `ExpertsBootConfig::merge_into`（6 单测） |
| 接线 | scheduler `build_app`：`effective_global` + `merge_into` 应用于全局 LLM 与模块注册 |

**验证（2026-08-31）**：临时覆盖文件将 expert-code name 改为独特标记 → `/experts/search` 返回
`code-expert-001 | 代码专家-YML覆盖验证`（覆盖生效、10 专家总数不变、其他专家保留内置）。

### 7.7 配置错误处理：显式报错（fail-fast）

> 配置错误**必须显式暴露，禁止静默降级为默认值**（用户明确要求 + 企业级原则）。

| 场景 | 行为 |
|---|---|
| 配置文件**不存在** | 输出警告，使用内置默认值（默认配置可运行） |
| 配置文件**存在但解析失败** | 返回错误、启动失败（fail-fast），报错含文件名与解析原因 |
| 环境变量**解析失败**（如端口非数字） | 输出警告，保持原值（yml/默认），不中断 |

实现：`mox-alliance-boot-config::load_from_file`（`load_scheduler` / `load_executor` / `load_experts` 统一语义）。
回归测试：`invalid_yaml_fails_fast`（lib）验证坏 yml → `Err`。

### 7.8 环境变量归一化（统一 `MOX_ALLIANCE_*`）

**规则**：所有配置覆盖统一使用前缀 `MOX_ALLIANCE_` + 配置路径全大写蛇形。
历史遗留的旧变量**保留兼容但标记 deprecated**（命中即打警告），新代码一律使用新变量。

| 旧变量（deprecated） | 新变量 | 说明 |
|---|---|---|
| `EXECUTOR_MODE` | `MOX_ALLIANCE_EXECUTOR_MODE` | 执行器模式；新变量优先，旧变量仅在新变量未设时生效 |
| `ALLIANCE_TASK_STORE` | `MOX_ALLIANCE_STORAGE_MODE` | 任务仓库模式（server.rs fallback）；显式注入优先于两者 |

回归测试：`tests/env_deprecated.rs`（独立进程）验证"新优先 / 旧兼容 / 默认保持"三条规则。

### 7.9 命名约定（module_id / expert_id）

> 归一化命名约定：**模块标识**与**专家实例标识**各司其职、方向相反是语义差异（模块 vs 实例），
> 但每类标识内部必须一致（kebab-case 小写）。

| 标识 | 约定 | 示例 |
|---|---|---|
| `module_id` | `expert-<domain>`（模块标识，统一 `expert-` 前缀） | `expert-code` / `expert-arch` / `expert-math` |
| `expert_id` | `<domain>-expert-<seq>`（专家实例标识，`-expert-` 中缀 + 序号） | `code-expert-001` / `arch-expert-001` |

- 新增领域模块必须遵循上述模式；两者由配置引擎以 `module_id → expert_id` 映射关联（config_sync 维护 `module_to_expert`）。
- 本次为文档化约定，不改动既有标识（避免破坏 config 快照与存量文档引用）。

### 7.10 HTTP 专家桥接（生产专家服务接线，2026-09-01 激活）

> 将此前"已实现未接线"的 `HttpExpertRegistryBridge`（scheduler-core `http-bridge` feature）真正激活：
> 通过 `expert_service.enabled=true` 显式启用，启动时从远程 AI 专家服务（默认 3300）拉取专家并入匹配器。

| 项 | 说明 |
|---|---|
| 启用开关 | `expert_service.enabled`（yml）或 `MOX_ALLIANCE_EXPERT_SERVICE_ENABLED`（env，true/false/1/0） |
| 默认 | `false`（不连接，仅内置 10 大领域专家） |
| 拉取接口 | `GET {base_url}/api/v1/experts?page_size=100&domain={tenant_id}`，响应 `{total, experts:[{id,name,domain,capabilities,description}]}` |
| 拉取策略 | 启动时拉取一次，成功则并入匹配器（与内置共存）；**失败优雅降级**——仅告警、使用内置继续，不阻断启动 |
| 实现 | scheduler-svc 依赖 scheduler-core 启用 `http-bridge` feature；`build_app` §3.1 接线 |
| 回归测试 | boot-config：`expert_service_enabled_env_override`（env 布尔解析）+ `defaults_match_port_norm`（默认 false） |

**端到端验证（2026-09-01，真实执行）**：

| 场景 | 结果 |
|---|---|
| 默认 enabled=false | 启动正常，10 内置专家，无 HTTP 拉取 |
| enabled=true + 无 3300 服务 | 优雅降级：`WARN HTTP 专家桥接拉取失败`，10 内置专家继续可用，不崩溃 |
| enabled=true + mock 3300 | `INFO HTTP 专家桥接启用：拉取 2 位专家`，专家总数 **10→12**（远程专家A/B 已并入） |

### 7.11 Nacos 阶段二：ConfigStore 抽象 + NacosConfigStore（2026-09-01）

> 把「配置从哪里来」从「直接读 yml 文件」抽象为**可插拔配置源链**，Nacos 成为 yml 的托管方，本地 yml 降级为离线兜底。基于官方 `nacos-group/nacos-sdk-rust`（crates.io `nacos-sdk` 0.8，`config` feature）。

**配置源链（优先级高 → 低）**：`内置默认 < 本地 yml(FileConfigStore) < Nacos(远程,可选) < env`

| 项 | 说明 |
|---|---|
| ConfigStore trait | `load_raw(key) -> Result<Option<String>>`；`Ok(None)`=无此 key，`Err`=读取失败 |
| FileConfigStore | 本地 `{base_dir}/{key}.yml`（离线兜底） |
| MemoryConfigStore | 内置默认 / 测试桩 |
| ConfigStoreChain | 按序逐源尝试，**容错降级**：上游 Err 告警后落到下一源（配置中心不可达 → 自动用本地 yml） |
| NacosConfigStore | 绑定单个 dataId；启动 `get_config` 初拉 + `add_listener` watch 热更新（缓存 + 广播） |
| 启用开关 | boot-config Cargo `nacos` feature（默认关闭，不引入 SDK）+ yml `nacos.enabled: true` |
| Bootstrap | `load_scheduler_with_nacos(path)`：读本地（引导）→ 若启用则拉远程整体覆盖 → env 仍最高 |
| 认证 | username/password 需 nacos-sdk `auth-by-http` feature（当前 boot-config 仅 `config` 能力，无鉴权直连；接入时补 feature） |
| 回归测试 | boot-config：config_store 6 项（命中/未命中/链降级/全 None）+ nacos 3 项（disabled 不发请求 / 空 dataId / 不可达显式报错）；nacos feature 下 **19 passed** |

**诚实声明**：本地无 Nacos 服务端，未做真实服务端 e2e；`get_config`/`add_listener` 走官方 SDK 协议，真实链路需部署 rnacos 或 nacos-server 2.x 后验证。scheduler-svc/executor-svc 默认**不启用** nacos feature（保持轻量），部署配置中心时开启。

### 7.12 executor 生产专家模式：真实 LLM 调用链（2026-09-01）

> 验证并修复 `executor-svc` Expert 模式（生产）下**真实 LLM 调用链**，即 DAG 节点 → `ExpertNodeExecutor` → `LlmExpertConsultant` → `OpenAiChatClient` → `POST {base_url}/chat/completions`。

| 项 | 说明 |
|---|---|
| LLM 配置 | `MOX_LLM_ENABLED` / `MOX_LLM_API_KEY`（回退 `OPENAI_API_KEY`/`DEEPSEEK_API_KEY`）/ `MOX_LLM_BASE_URL` / `MOX_LLM_MODEL` / `MOX_LLM_TIMEOUT_MS`（见 alliance-executor.yml 注释） |
| 无 Key | `llm_consultant_from_env()` 返回 None → 回退本地专家引擎（离线可用） |
| 调用失败 | ReAct 循环异常 → 告警 + 回退本地引擎 |
| **修复（真实 bug）** | `OpenAiChatClient` 原在 `new()` 直接构建 `reqwest::blocking::Client`；但 `llm_consultant()` 在 axum `build_app`（async 上下文）调用，blocking client 自带 tokio runtime，进程退出 drop 时 panic `Cannot drop a runtime in a context where blocking is not allowed` → **有 Key 的生产模式启动即崩**。改为 `OnceLock` 延迟到首次 `complete()`（`spawn_blocking` 的 blocking 线程）才构建，彻底避开 async 上下文 |
| 端到端验证 | 真实执行（mock OpenAI 兼容 8999，响应固定）：任务 1/1 节点 completed；mock 收到 `POST /v1/chat/completions` + `Authorization: Bearer test-key-123` + `model=test-model` + 2 messages，解析评分/结论 → 节点 completed |

**诚实声明**：LLM 响应来自本地 mock OpenAI 服务（无真实 API Key），但 **HTTP 请求构造、认证、消息格式、响应解析走真实生产代码路径**；接入真实 Key 后同一链路即接真实模型。

---

## 附录A 快速记忆口诀

> **核心服务 3 打头**：平台 30、编排 31、执行 32、专家 33 —— 两位段号记业务，00 结尾是主口。
> **小服务 3 万起**：插件 30000+，语音 30010。

---

*PORT-NORM-001 V1.3 · 开发联盟 R · 2026-09-01*
