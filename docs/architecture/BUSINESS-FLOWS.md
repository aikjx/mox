# MOX 全架构业务处理流程（BUSINESS FLOWS）

> 数据基准日：2026-09-07（提交 a609bdc0 + 53986b18 之后，注册表 223 条）
> 本文档与 `docs/API-REGISTRY.md`（222 条接口↔实现映射）、`docs/ROADMAP-DOMAINS.md`（43 域排产）互为配套：
> 注册表回答"有什么"，路线图回答"何时做"，本文档回答"请求怎么走"。

## 0. 总览：唯一入口 + 分层处理

```
客户端 ──▶ :8080 网关（唯一入口）
              │
              ├─ ① 可观测中间件（trace/指标/日志 + API 启停拦截）
              ├─ ② CORS（具体 origin 白名单，禁 Any）
              ├─ ③ 限流（令牌桶）
              ├─ ④ 鉴权（JWT HS256 验签 / API Key → UserInfo 注入）
              │
              ├─ ⑤ 路由匹配（按具体度）：
              │     Actuator 管理面 → L0 公共端点 → 21 个域路由单元 → Proxy 兜底
              │
              ├─ 进程内域（RBAC/KG/Graph/Expert/Alliance 网关侧/System/Security/
              │            Voice/Melody/Cloud/Monitor/Workspace/Projects）
              ├─ 独立进程（KB :8104 · Scheduler :3100 · Executor :3200 · 编排器 :3001）
              └─ 用户独立服务（melody2score :8012 · primiflow :8000 · 前端 :3020）
```

### 运行时中间件链（由外到内，lib.rs 中 layer 后注册者先执行）

| 序 | 中间件 | 职责 | 失败语义 |
| --- | --- | --- | --- |
| 1 | `observability_middleware` | 运行时统计（QPS/延迟分位）、指标采集、环形日志、**API 启停拦截**（被停用路由→403） | 403（停用路由） |
| 2 | `CorsLayer` | 白名单 origin + 限定 methods/headers + credentials | 浏览器跨域拒绝 |
| 3 | `rate_limit_middleware` | 令牌桶限流（按路由维度） | 429 |
| 4 | `auth_middleware` | JWT HS256 恒定时间验签 + `iss`/`exp` 校验 / `x-api-key`；`public_paths`（/health 等）放行；UserInfo 注入请求扩展 | 401/403 |

## 1. 基础设施域（L0 · 公共端点）

| 端点 | 处理流程 | 数据来源 |
| --- | --- | --- |
| `GET /health` | 进程存活探针 → 200 `{"status":"ok"}` | 静态 |
| `GET /api/v1/status` | GatewayState 聚合 → 域状态/注册表/服务矩阵 | runtime + actuator ROUTES |
| `GET /api/v1/domains` | 遍历 43 域描述符 → 分层分组 JSON | routes.rs 描述符 |
| `GET /metrics` | Prometheus 文本格式指标（QPS/延迟/活跃请求） | metrics 采集器 |

## 2. Actuator 管理面（L0 · 强制鉴权）

| 端点 | 处理流程 | 语义 |
| --- | --- | --- |
| `GET /actuator/mappings` | ROUTES 静态表 → 222+ 路由（含 id/方法/路径/层/域/状态/描述） | 注册表单一权威源 |
| `POST /actuator/api/:id/enable\|disable` | 匹配路由 → 原子切换 enabled 标志（持久化）→ 后续请求经可观测中间件拦截 | 运行时治理 |

> **治理闭环**：actuator 是声明源 → `scripts/gen-api-registry.py` 重生成 `docs/API-REGISTRY.md` → 实测 223 条一一对应。

## 3. RBAC 域（L1 · `/rbac/v1/*`）

```
请求 → auth_middleware（Bearer → UserInfo）
     → rbac handler → GatewayState.iam（Arc<IamRepository>，SQLite）
     → list_roles(&tenant) / get_user_permissions(&tenant, &user) / 当前用户摘要
     → ApiResponse（角色/权限/当前用户）
```
- 默认租户 `T001`、默认用户 `admin-user`；`resolve_tenant` 支持 tenant_code/tenant_id 双兼容
- **优化点（2026-09-07 落地）**：`/rbac/v1/current` 从 Bearer 取真实 user_id（此前用占位默认值）

## 4. 知识图谱域（L2 · `/kg/v1/*` + `/graph/v1/*`，同源）

```
请求 → kg-svc router（mox-kg-service-svc 进程内）
     → KgAiState.kg（Arc<KgGraph>：内存 KnowledgeGraph + 节点/边 meta + source_path）
     → 端点算法：stats()（密度/平均度/聚类系数/SCC）/ detect_communities(50)（CNM）
     → /graph/v1/overview（标签/类型/关系分布聚合）
     → ApiResponse
```
- **同源一致**：graph 域与 kg 域共用同一图实例与算法，数据天然一致（不复制）

## 5. 知识库域（L2 · `/api/kb/*` → :8104 独立进程）

```
请求 → 网关路由匹配（/api/kb/* 具体度优先于 proxy 兜底）
     → mox-kb-server :8104（独立 Rust 进程）
     → 文档/知识库 CRUD + 检索 → 响应回传网关
```

## 6. 专家联盟域（L3 · `/api/experts*` 48 接口）

```
请求 → auth_middleware → ExpertsSharedState（注册中心 + 图谱 + 编排 + 会话）
     │
     ├─ 注册中心：ExpertRegistry（10 专家 descriptors，JSON 持久化）
     │     → 列表/能力/指标/总览/统计
     ├─ 协作：单/多专家咨询、辩论、智能路由（规则 + LLM 可选）
     ├─ 图谱算法：build_graph_from_registry → 邻域/路径/社区（label propagation）/
     │     │     最优团队（weighted_set_cover_greedy）
     │     └─ ★ goal→需求提取（2026-09-07 开发）：extract_requirements(goal, registry)
     │          仅产出 registry 真实域/技能 id → set-cover 组建团队
     ├─ 编排：plan 生成 → execute（步骤级执行，fusion 融合策略）→ 历史/统计
     ├─ 调度器：best_match 策略分发、熔断、重置
     └─ 会话：store_json 持久化 + 分页/过滤/搜索 + 语义检索 + 导出/归档
```

## 7. 联盟域（L4 · `/api/alliance/*` 20 接口）

```
请求 → gateway alliance_remote（MOX_ALLIANCE_SCHEDULER_URL/EXECUTOR_URL 激活）
     → scheduler :3100（任务编排：DAG 构建/节点调度）
     → executor :3200（节点执行：mock 模式走 Mock 节点执行器 / expert 模式需 LLM key）
     → 任务状态流转（pending→running→completed/failed）、DAG 节点/边、日志流、融合结果
```
- **运行模式**：`mode=remote`（远程调度/执行）或 `mode=off`（本地降级）
- **readiness 语义**：`EXECUTOR_MODE=mock` 时如实判定就绪（提交 95e715ae 修复）

## 8. 治理域（L1 · `/api/system/*` + `/api/security/*`，46 条）

```
请求 → 网关原生路由（受保护）→ IAM SQLite 真实链路（租户/用户/角色/策略）
     → 用户管理、角色权限、审计（audit/experts-audit.ndjson）、配置
```

## 9. 语音/转谱域（L7 · `/voice/v1/*` + `/melody/v1/*`，桥接模式）

```
请求 → 网关 Voice/Melody router（自含 reqwest client，MOX_VOICE_UPSTREAM_URL 覆盖）
     → melody2score :8012（webui.py FastAPI，用户独立服务）
     → 健康探测/样例列表/识别（透传）/保存简谱/导出歌谱/下载
     → 上游不可达 → 502（不静默降级）；上游 422/500/404 如实透传
```
- **对应关系**：voice 3 条 + melody 7 条 = 10 条桥接端点，逐条对应上游真实路由

## 10. 云存储域（L5 · `/cloud/v1/*`，本地磁盘对象存储）

```
请求 → CloudState（MOX_STORAGE_ROOT，默认 data/storage）
     → sanitize(bucket/key)：拒绝 ../、绝对路径、分隔符、前导点 → 400
     → bucket 列/建/删（S3 语义：非空 409）→ object 列/PUT/GET/DELETE
     → 真实磁盘读写（std::fs）
```
- **路径穿越防护**：非法段 400 实测（`../evil` 被拒）

## 11. 代理兜底（L6 · `/api/{*path}`）

```
请求 → proxy.rs（注册为 Router<()>，nest /api/projects 与 /api 两级）
     ├─ /api/projects/* → primiflow :8000（项目/拓扑/资产）
     └─ 其余 /api/* → operator-server :3001（OUS 编排器，OUS_API_TOKEN 校验）
     → 上游响应透传
```
- **优先级**：网关原生域路由（/api/kb、/api/experts、/api/alliance、/api/system 等）**先于** proxy 匹配
- 编排器内部再按 OUS 域路由分发（AI/图谱/算子/治理/商城等）

## 12. 通用业务域（注册中心注入）

`monitor`（运行监控）、`workspace`（工作区）、`projects_ext`（项目扩展）——状态由 `ModuleStates` 统一构造一次注入，杜绝数据分裂。

---

## 13. 流程优化点清单（2026-09-07 已落地 + 待办）

| # | 优化点 | 状态 |
| --- | --- | --- |
| 1 | goal→需求规则提取（专家联盟最优团队，之前只能显式传 required） | ✅ 已落地（53986b18） |
| 2 | 会话列表声明对齐（路由存在但注册表漏登记） | ✅ 已落地（53986b18） |
| 3 | Cloud bucket 删除（S3 语义缺口，空桶可删/非空 409） | ✅ 已落地（本轮） |
| 4 | 全局 trace_id 注入响应头（跨进程链路追踪） | ⏳ 待办（建议） |
| 5 | 代理目标健康探测（:3001/:8000 失联时网关快速失败而非超时） | ⏳ 待办（建议） |
| 6 | expert 真实执行闭环（需 LLM provider key） | ⏳ 依赖环境 |
