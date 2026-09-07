# MOX 系统总览（SYSTEM OVERVIEW）

> 数据基准日：2026-09-07 | 注册表 223 条 | 43 域 / 9 能力组 | 143 crates
> 本文档是 MOX 系统的顶层总览，整合架构、能力、运行、模块化、治理与演进。
> 配套文档：`API-REGISTRY.md`（接口映射）、`BUSINESS-FLOWS.md`（处理流程）、
> `MODULARITY.md`（模块化分析）、`ROADMAP-DOMAINS.md`（域排产）、`CORE-CAPABILITIES.md`（产品定位）。

---

## 1. 系统定位

**MOX 是一个 Rust 原生的企业级 AI 服务平台。**

它不是 LangChain/CrewAI 那种"AI 应用开发 SDK"，而是一个把知识图谱、多智能体编排、知识库、AI 引擎、云存储、语音转谱、权限治理统一收敛到一个网关入口下的**模块化平台**。

**差异化定位**：在「Rust 原生 + gRPC 服务化 + 知识图谱原生 + 多智能体编排 + K8s 部署」这个交叉区间，全行业无直接竞品。

---

## 2. 架构全景

### 2.1 六层单向依赖（清洁架构变体）

```
foundation（横切基座）
  → api（域契约）
    → proto（gRPC 契约层）
      → core（纯计算，无 IO）
        → svc（服务实现）
          → gateway（唯一入口 :8080）
```

依赖方向严格单向，核心层无 IO 副作用，可独立测试。

### 2.2 十二域驱动（业务能力切分）

`kg / ai / flow / data / cloud / voice / market / alliance / kb / base / project / platform`

### 2.3 四十三域描述符 → 九能力组

| 能力组 | 域 | 数量 |
| --- | --- | --- |
| **platform**（平台治理） | Health, Metrics, IAM, Auth, Tenant, RBAC, Enterprise, Platform, Audit | 9 |
| **knowledge**（知识图谱） | KG, Graph, KB, Cypher, nGQL | 5 |
| **ai**（AI 智能） | AIEngine, AI-Core, Expert, Intent, Alliance | 5 |
| **orchestration**（编排） | Flow, Workflow, BPM, Pipeline | 4 |
| **storage**（存储） | Cloud, S3, Volume, FS | 4 |
| **data**（数据） | Data, ETL, Norm, Standard | 4 |
| **media**（媒体） | Voice, MIDI, Melody, TTS | 4 |
| **commerce**（商业） | Market, Shop, Order, Billing | 4 |
| **streaming**（流式） | Streams, Kafka, WebSocket, Event | 4 |
| **合计** | | **43** |

### 2.4 二百二十三条已登记 API

actuator ROUTES 为单一权威源，`scripts/gen-api-registry.py` 自动生成 `docs/API-REGISTRY.md`，声明与实现一一对应。

---

## 3. 核心能力（七大已就绪能力域）

| 能力域 | 状态 | 核心能力 |
| --- | --- | --- |
| **知识图谱** | ready | 内存图引擎 + 算法（密度/平均度/聚类系数/SCC/CNM 社区检测），KG 与 Graph 同源 |
| **专家联盟** | ready | 48 接口：注册中心/协作/图谱算法/编排/调度/会话；goal→需求规则提取 + 加权集合覆盖组队 |
| **联盟调度** | ready | 20 接口：scheduler:3100 + executor:3200，DAG 编排，任务仓储可插拔（file 快照默认持久化） |
| **知识库** | ready | mox-kb-server:8104 独立进程，文档 CRUD/分析/挂图/检索，100% 自研 |
| **AI 引擎** | ready | 统一编排 4 接口 |
| **云存储** | ready | 6 接口：本地磁盘对象存储，S3 兼容语义，路径穿越防护，空桶删除 |
| **语音转谱** | ready | 10 接口：桥接 melody2score:8012，音频识别/简谱/歌谱/下载 |
| **RBAC 治理** | ready | 3 接口：角色/权限/当前用户，IAM SQLite 真实仓储 |

**就绪率**：12 ready / 1 beta / 30 stub（stub 域如实标注，不虚标）。

---

## 4. 运行架构（五进程企业级部署）

| 进程 | 端口 | 职责 |
| --- | --- | --- |
| **mox-server** | :8080 | 网关唯一入口，21 路由单元装配，中间件链（可观测→CORS→限流→鉴权） |
| **operator-server** | :3001 | OUS 编排器，全业务域路由分发 |
| **mox-kb-server** | :8104 | 知识库独立进程 |
| **mox-alliance-scheduler** | :3100 | 联盟任务调度（DAG 构建/节点调度） |
| **mox-alliance-executor** | :3200 | 联盟任务执行（mock/expert 模式） |

**一键启停**：`scripts/start-mox-enterprise.ps1` / `stop-mox-enterprise.ps1`（幂等，不动独立服务）。

---

## 5. 模块化模式

**核心模式：模块化单体优先 + 服务化接口 + 渐进式拆分。**

- **模块化单体**：网关进程内装配 21 个路由单元，共享状态单例注入（防数据分裂）
- **服务化接口**：gRPC 契约层（proto）+ HTTP REST，多语言接入预留
- **渐进拆分**：KB/Scheduler/Executor 已示范进程拆分；联盟 remote/off 模式切换即拆分预演
- **可替换后端**：上游 URL 可覆盖（VOICE/PRIMIFLOW/ORCHESTRATOR），任务仓储可插拔（file/memory）

**智能评分 4.3/5**（详见 MODULARITY.md）：内聚 4.5 / 解耦 4.5 / 可演进 4.5 / 可替换 4.0 / 可观测 4.0 / 一致性 4.5。

---

## 6. 可观测性与治理

### 6.1 全链路追踪

`x-request-id` 端到端闭环：
- 客户端携带 → 原样透传 + 响应头回写
- 客户端未携带 → 网关生成 UUID → 写回请求头 → proxy 全量转发到上游（:3001/:8000）
- 环形日志关联（`rid=`），`/actuator/logs` 可查

### 6.2 治理门禁

| 门禁 | 工具 | 作用 |
| --- | --- | --- |
| API 注册表 | actuator ROUTES + gen-api-registry.py | 声明↔实现一一对应，223 条 |
| 端口注册表 | docs/api/PORT-REGISTRY.md + verify-ports.py | 端口漂移检测 |
| 域归并测试 | tests/domain_grouping_integration.rs（7 条） | 域数量/能力组/映射/端点输出门禁 |
| 代码规范 | cargo clippy --all-targets | CI 门禁 |
| 运行时预检 | start.sh --dry-run | 启动前配置校验 |

---

## 7. 技术栈

| 层 | 技术 |
| --- | --- |
| 语言 | Rust（edition 2021，143 crates） |
| Web 框架 | axum 0.7（tokio 异步运行时） |
| 序列化 | serde / serde_json |
| 存储 | SQLite（IAM）+ 本地磁盘对象存储（Cloud）+ 内容寻址去重 FS（kb-svc） |
| 认证 | JWT HS256 + API Key（恒定时间验签） |
| 可观测 | 运行时统计 + Prometheus 指标 + 环形日志 + x-request-id 追踪 |
| 部署 | Docker / K8s / Helm / 一键 PowerShell 脚本 |
| 前端 | Vite + Vue3（dev :3020） |

---

## 8. 演进路径

| 阶段 | 目标 | 关键动作 |
| --- | --- | --- |
| **近期**（已完成） | 治理闭环 + 模块化归一 | 域归并 9 组 / x-request-id 全链路 / 集成测试门禁 / 预存编译错误修复 |
| **中期** | 能力深化 + 服务拆分 | 专家/联盟域按负载拆独立进程；proto 契约版本化；GraphRAG 高层抽象 |
| **远期** | 平台化对外 | gRPC 客户端 SDK（Python/TS）；多租户治理产品化；可观测性面板（对标 LangSmith） |

> **原则**：拆分是演进而非目标。任何拆分必须由真实负载/团队边界驱动，不因"听起来企业级"而拆。

---

## 9. 文档索引

| 文档 | 内容 |
| --- | --- |
| `docs/API-REGISTRY.md` | 223 条接口↔实现映射（自动生成） |
| `docs/architecture/BUSINESS-FLOWS.md` | 全模块业务处理流程 |
| `docs/architecture/MODULARITY.md` | 模块化模式智能分析（4.3/5） |
| `docs/ROADMAP-DOMAINS.md` | 43 域排产与就绪状态 |
| `docs/CORE-CAPABILITIES.md` | 产品定位与核心能力 |
| `docs/api/PORT-REGISTRY.md` | 端口权威注册表 |
| `AGENTS.md` | 仓库维护指南（常用命令/架构/治理规则） |
