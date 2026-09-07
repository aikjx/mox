# MOX 域路线图（Domain Roadmap）

> 权威状态基准：`platform/gateway/mox-platform-gateway-svc/src/routes.rs` 的 43 域描述符 + `docs/API-REGISTRY.md`（199 条路由）。
> 本文档回答：**35 个 stub 域按什么顺序、以什么路径、达到什么标准落地**。与注册表一一对应，不做模糊承诺。

## 1. 现状总览（43 域）

| 状态 | 数量 | 域 |
| --- | --- | --- |
| ready（有真实路由+handler） | 7 | Health · Metrics · KG · KB · AIEngine · Alliance · Expert |
| beta（可用但依赖外部进程） | 1 | IAM（依赖编排器 :3001） |
| **stub（仅规划声明）** | **35** | 见下表 |

## 2. 35 个 stub 域排产总表

> 排产原则：① 能直接复用现有 ready 能力/独立服务的域最优先（低投入高收益）；② 治理闭环（认证/租户/审计）先于业务域；③ 数据管道先于商业化；④ 每个域落地必须满足"**声明状态与实际路由/实现一一对应**"（注册表门禁），禁止"声明 ready 但无路由"。

### Phase 0 —— 对接现有能力（低投入高收益，建议 1 个迭代）

| 域 | 所在层（目录） | 现成可对接能力 | 落地路径 | 验收标准（路由） |
| --- | --- | --- | --- | --- |
| RBAC | platform | system 域已实现 `/api/system/*`、`/api/security/*`（46 条，含权限相关） | 在 system 域之上声明 RBAC 描述符，补齐权限/角色 CRUD 路由 | `/api/rbac/*` 全部 200，非空实现 |
| Graph | kg | kg 域已 ready：`/kg/v1/*`（图存储/查询/节点边 CRUD） | 高层图 API（图谱构建/图算法入口） | `/api/graph/*` 与 `/kg/v1/*` 数据一致 |
| Voice | voice | melody2score 独立服务 :8012 已运行 | 桥接网关 `/api/voice/*` → :8012 | `/api/voice/*` 200 |
| Melody / MIDI | voice | 同 Voice | 语音域子能力，随 Voice 一并承接 | `/api/voice/melody*`、`/api/voice/midi*` 200 |
| S3 / Volume / FS | cloud | 无现成实现（S3 已如实降 stub） | 统一文件/对象存储抽象，MinIO 兼容 S3 优先 | `/api/cloud/s3/*` 200（本地磁盘 fallback） |

### Phase 1 —— 治理闭环（企业级地基，建议 2 个迭代）

| 域 | 所在层 | 依赖 | 落地路径 | 验收标准 |
| --- | --- | --- | --- | --- |
| Auth | platform | IAM(beta) | 统一认证：网关 JWT 校验已工作，补齐 token 签发/刷新/吊销管理 API | `/api/auth/*`（login/refresh/revoke/keys）200 |
| Tenant | platform | Auth → IAM | 多租户隔离：租户 CRUD + 数据隔离策略 | `/api/tenant/*` 200，鉴权生效 |
| Audit | platform | Auth → IAM | 操作审计：事件落库 + 查询 API（网关已有 trace_id 注入基础） | `/api/audit/*` 200，事件真实可查 |
| Enterprise | platform | Tenant → RBAC | 企业组织/部门/成员（编排器 `/api/system/user` 已有雏形） | `/api/enterprise/*` 200 |

### Phase 2 —— 智能编排增强（承接 alliance 任务流）

| 域 | 所在层 | 依赖 | 落地路径 | 验收标准 |
| --- | --- | --- | --- | --- |
| AI-Core | ai | AIEngine(ready) | AI 引擎统一抽象：模型/推理/上下文管理层 | `/api/ai/core/*` 200 |
| Intent | ai | 编排器已暴露 `intent_recognition` 能力 | 意图识别服务化 | `/api/ai/intent/*` 200 |
| Flow | flow | alliance 任务流(ready, 已实测 DAG 执行) | 把联盟 DAG 提升为通用流程域 | `/api/flow/*`（定义/实例/状态）200 |
| Workflow | flow | Flow | 审批/编排工作流（承接 BPM 概念） | `/api/workflow/*` 200 |
| BPM | flow | Workflow | 业务流程管理（模型/部署/实例） | `/api/bpm/*` 200 |
| Pipeline | flow | Workflow → Data | 数据/任务管道定义与执行 | `/api/pipeline/*` 200 |

### Phase 3 —— 数据管道与流式

| 域 | 所在层 | 依赖 | 落地路径 | 验收标准 |
| --- | --- | --- | --- | --- |
| Data | data | kb 域(ready) | 数据集管理：导入/版本/血缘 | `/api/data/*` 200 |
| ETL / Norm / Standard | data | Data | 抽取-转换-加载 + 数据规范/标准化 | `/api/etl/*`、`/api/norm/*`、`/api/standard/*` 200 |
| Streams / Kafka | data | Event | 流式数据接入（Kafka 客户端封装） | `/api/streams/*` 200 |
| Event / WebSocket | platform | — | 事件总线 + 实时通道（网关可先行 `/ws/*`） | `/api/event/*`、`/ws/*` 握手 101 |

### Phase 4 —— 商业化（最后，依赖治理+数据就绪）

| 域 | 所在层 | 依赖 | 落地路径 | 验收标准 |
| --- | --- | --- | --- | --- |
| Market | market | Shop → Auth | 市场/应用商店 | `/api/market/*` 200 |
| Shop / Order / Billing | market | Tenant + Billing 规则 | 店铺/订单/计费结算 | `/api/shop/*`、`/api/order/*`、`/api/billing/*` 200 |

### 特殊：图查询语言适配

| 域 | 所在层 | 说明 | 落地路径 |
| --- | --- | --- | --- |
| Cypher / nGQL | kg | 图查询语言方言层 | 随 Graph 域（Phase 0）一并设计：SQL 方言抽象 → Cypher 适配器 → nGQL 适配器，验收 = 同一查询两方言结果一致 |

### 特殊：Platform（平台域自身）

| 域 | 所在层 | 说明 | 落地路径 |
| --- | --- | --- | --- |
| Platform | platform | 平台自治能力（配置/升级/插件） | 编排器 `mox-viz`/`mox-system` 模块已有雏形，收敛为 `/api/platform/*` |

## 3. 落地门禁（每个域开工前必须满足）

1. **路由先行登记**：新域路由先写 `actuator.rs` `ROUTES`，再写 handler（`/actuator/mappings` 为唯一权威视图）。
2. **前缀唯一**：kg=`/kg/v1/*`、ai=`/ai/engine/*`、kb=`/api/kb/*`、alliance=`/api/alliance/*`、其余=`/api/<域>/*`。
3. **状态如实**：实现未完成前声明 `stub`；`beta` = 依赖外部进程；只有"有路由 + 有 handler + 实测 200"才可声明 `ready`。
4. **文档同步**：改完跑 `scripts/gen-api-registry.py` 重新生成 `docs/API-REGISTRY.md`，并在本文档变更记录登记。
5. **门禁命令**：`cargo check -p mox-platform-gateway-svc` + `scripts/verify-ports.py`（ERROR=0）+ 新前缀实测 200。

## 4. 变更记录

| 日期 | 变更 |
| --- | --- |
| 2026-09-07 | 初版：35 stub 域按"对接现有能力→治理→编排→数据→商业化"五阶段排产，落地门禁与验收标准一一对应 |

