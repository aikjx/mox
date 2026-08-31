# 开发专家联盟·核心服务端口规划规范（PORT-NORM-001）

> **标题**：开发专家联盟·核心服务端口规划规范
> **版本**：V1.0
> **权威等级**：🟢权威
> **编号**：PORT-NORM-001
> **最后更新日期**：2026-08-31
> **单源声明**：本文档是"开发专家联盟"全部核心服务与插件/小服务端口分配的**唯一权威规范**。凡涉及端口规划、端口分配、端口避让、端口迁移的决策与文档，均以本文档为准。本文档冲突时以 `docs/enterprise/18-全域顶层总设计-三联盟模式-V1.0.md`（TOP-MASTER）为准。
> **主责联盟**：开发联盟 R（架构·代码·文档治理）
> **编制依据**：`docs/expert-alliance/00-INTEGRATED-INDEX.md`、`docs/expert-alliance/02-DUAL-PLATFORM-RELATIONSHIP.md`、`docs/standards/expert-alliance-normalization-mode.md`（EA-NORM-001）

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
| **3010** | Node.js 平台 API（api） | 平台层（30xx） | Node.js 层统一入口（Express） | 已启用 |
| **3020** | Node.js 前端（frontend） | 平台层（30xx） | 前端开发/静态服务 | 已启用 |
| **3100** | **scheduler-svc**（调度编排） | 编排（31xx） | 任务调度、专家匹配、计划生成 | 已启用 |
| **3200** | **executor-svc**（执行引擎） | 执行（32xx） | DAG 执行、节点调度 | 已启用 |
| **3300** | AI 专家服务（ai-expert 桥接） | 领域专家（33xx） | scheduler 内部桥接的专家服务基地址 | 已启用 |

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
| **30010** | 语音服务（xiaobai_voice，原 3717） | **规划迁移**（落段 30010） |
| 30001–30009 | 通用插件保留 | 预留 |

> 迁移说明：语音服务为辅助小服务，按本规范应落入 30000+ 段。当前物理端口仍为 3717（Node.js 平台 `platform_config.json` 配置），**纳入迁移计划**，迁移时同步更新本文档与 `docs/expert-alliance/00-INTEGRATED-INDEX.md`。

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
| Node.js api / frontend | 3010 / 3020 | 不变 | 已在 3xxx 段 | — |
| 语音服务 xiaobai_voice | 3717 | 30010（规划） | 小服务归 30000+ 段 | `platform_config.json`（待迁移） |

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

---

## 附录A 快速记忆口诀

> **核心服务 3 打头**：平台 30、编排 31、执行 32、专家 33 —— 两位段号记业务，00 结尾是主口。
> **小服务 3 万起**：插件 30000+，语音 30010。

---

*PORT-NORM-001 V1.0 · 开发联盟 R · 2026-08-31*
