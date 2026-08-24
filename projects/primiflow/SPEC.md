# PrimiFlow MVP 工程规格书 · Release‑1【原点】

> 版本：R1‑SPEC‑v1.0 ｜ 状态：规格待评审（开发前冻结）
> 范式来源：PT‑Primi 全域拓扑原语架构（三维业务原语 κ/τ/Q + 全局常数 C）
> 定位：验证「自然语言需求 → 拓扑涌现 → 可视化画布 → 全套文档自动生成」主链路闭环。
> **范围声明**：本 MVP **不**做全自动代码生成（代码以骨架/桩形式产出），优先打通主链路与六维溯源。

---

## 0. 与核验报告的映射（承上启下）

| 核验命题 | 本报告落点 |
|----------|-----------|
| 命题1 范式自洽 | κ/τ/C/Q 在本规格中定义为**工程调度参数 + 资产复用机制**，非物理定律；正则化算子 ℛ̂ 定义为预算残差裁剪算法。 |
| 命题2 仅靠对话完成 | 主链路输入 = 自然语言需求 + 稳定/探索滑块；输出 = 拓扑 + 画布 + 8 份文档。✅ 主链路覆盖。 |
| 命题3 边界短板 | 幻觉传导 →  schema 校验 + 冒烟测试双重兜底；κ‑τ 自动寻优未收敛 → 本期用「规则 + 人工滑块 + 日志回灌」；仅限业务软件域。⚠️ 已在 §9 登记缓解。 |
| 命题4 非终极最优 | 本期为三维原语低维投影；不自动判定需求本源最优；公理固定不可自改。❌ 已声明。 |
| 命题5 MVP 可行 | 本文件即最小可落地版完整规格。✅ |

---

## 1. 核心范式落地定义（工程语义，去玄学）

把核验报告的几何隐喻翻译为**可计算约束**：

- **κ（曲率 / 收敛权重）** ∈ [0,1]：越高越偏好复用历史资产、收敛到稳定拓扑。
- **τ（挠率 / 裂变权重）** ∈ [0,1]：越高越偏好新建拓扑、探索新结构。
- **C（全局拓扑常数 / 资源上界）** > 0：单次生成的算力‑规模‑风险预算上限（归一到「拓扑节点数 + 边数的加权代价」）。
- **Q（拓扑荷）**：经 ℛ̂ 校验合格、被冻结的拓扑沉淀为**永久可复用资产**，存入向量库供后续 κ 复用检索。
- **正则化算子 ℛ̂**：生成后计算残差 `Δ = C² − κ² − τ²`（其中 κ²+τ² 在滑块归一化下恒等于 1，故 Δ 实际反映「实际消耗代价 vs 预算 C」与「κ/τ 配比合法性」），对 `Δ<0` 的拓扑做**最低优先级边/节点裁剪**直至 `Δ≥0`，抑制拓扑爆炸、死锁、矛盾环。

**滑块 → 参数映射**（稳定优先 ↔ 探索优先）：
```
θ ∈ [0, π/2]，由滑块 s∈[0,1] 线性映射：θ = s · π/2
κ = cos(θ)，τ = sin(θ)   ⇒   κ² + τ² = 1   （即 C=1 归一化下的守恒）
C = C_base · (1 + budget_factor)   # 由用户/租户预算配置决定
```
> 注：引入 C≠1 是为了让「资源上界」独立可调；此时 `κ²+τ²=1 ≤ C²` 恒成立，ℛ̂ 的裁剪主要作用于「实际代价 vs C」维度。

---

## 2. 系统架构

```
┌──────────────────────────────────────────────────────────────┐
│ 前端 (React + Cytoscape‑js)                                   │
│  ChatPanel(需求输入+滑块)  ·  Canvas(DAG渲染/编辑)            │
│  DocViewer(8份文档)       ·  AssetLibrary(复用浏览)           │
└───────────────┬──────────────────────────┬───────────────────┘
                │  REST/JSON (HTTPS)        │
                ▼                           │
┌──────────────────────────────┐    ┌──────────────────────────┐
│ Go 编排层 (api + orchestrator)│    │ Python 算子层             │
│  - api‑gateway (鉴权/校验)    │    │  - llm‑gateway           │
│  - orchestrator (状态机)      │◀──▶│  - topology‑operator     │
│  - scheduler (κ/τ预算+ℛ̂)     │    │  - doc‑generator         │
│  - asset‑service             │    │  - smoke‑tester          │
└───────────────┬──────────────┘    └──────────────────────────┘
                │
                ▼
┌──────────────────────────────────────────────────────────────┐
│ PostgreSQL + pgvector                                          │
│  projects / conversations / topologies / assets /             │
│  artifacts / trace_links / embeddings(pgvector)               │
└──────────────────────────────────────────────────────────────┘
```

**主链路数据流**：
`NL需求+滑块` → llm‑gateway(需求结构化) → topology‑operator(产出 DAG) → scheduler.regularize(ℛ̂ 裁剪) → asset‑service(κ 检索复用/τ 新建) → doc‑generator(8 文档) → trace_links(六维绑定) → 前端画布 + DocViewer。

---

## 3. 技术栈（展开版，全部成熟开源）

| 层 | 选型 | 职责 |
|----|------|------|
| 后端编排 | **Go 1.22+** (gin/echo + gRPC) | API 网关、状态机编排、κ/τ 调度、ℛ̂ 裁剪 |
| 大模型算子 | **Python 3.11+** (FastAPI + httpx) | LLM 网关、拓扑生成、文档生成、冒烟测试 |
| 向量/关系库 | **PostgreSQL 16** + **pgvector** | 主存储 + 资产语义检索（κ 复用） |
| 前端画布 | **React 18** + **Cytoscape‑js** | 对话、DAG 画布、文档、资产库 |
| 模型接入 | OpenAI/Claude/本地 vLLM 抽象 | 经 llm‑gateway 统一接口，可替换 |
| 任务编排 | Go 内部状态机 + 可选 Temporal/Asynq | 长链路异步生成 |
| 观测 | OpenTelemetry + Prometheus | 链路追踪、κ/τ 消耗指标 |

---

## 4. 数据模型（PostgreSQL + pgvector）

```sql
-- 项目
CREATE TABLE projects (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name          TEXT NOT NULL,
  tenant_id     TEXT,
  k_t_pref      JSONB NOT NULL DEFAULT '{"k":0.7,"t":0.3}', -- 默认滑块
  budget_c      REAL NOT NULL DEFAULT 1.0,
  created_at    TIMESTAMPTZ DEFAULT now()
);

-- 对话与会话消息
CREATE TABLE conversations (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id    UUID REFERENCES projects(id),
  role          TEXT NOT NULL,            -- user/assistant/system
  content       TEXT NOT NULL,
  meta          JSONB,                    -- 含滑块 s、κ、τ、C
  created_at    TIMESTAMPTZ DEFAULT now()
);

-- 拓扑(DAG)
CREATE TABLE topologies (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id    UUID REFERENCES projects(id),
  status        TEXT NOT NULL DEFAULT 'draft', -- draft|regularized|frozen|rejected
  k             REAL, t REAL, c REAL,
  residual_delta REAL,                    -- ℛ̂ 后的 Δ
  graph_json    JSONB NOT NULL,           -- {nodes:[],edges:[]}
  created_at    TIMESTAMPTZ DEFAULT now()
);

-- 冻结资产（拓扑荷 Q）
CREATE TABLE assets (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  topology_id   UUID REFERENCES topologies(id),
  name          TEXT NOT NULL,
  domain        TEXT,                     -- 业务域标签，用于 κ 检索
  graph_json    JSONB NOT NULL,
  embedding     VECTOR(1536),             -- pgvector，语义检索
  frozen_at     TIMESTAMPTZ DEFAULT now()
);

-- 产物（8 份文档 + 代码骨架）
CREATE TABLE artifacts (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id    UUID REFERENCES projects(id),
  kind          TEXT NOT NULL,            -- 见 §8 八类
  title         TEXT NOT NULL,
  content       TEXT NOT NULL,            -- Markdown / 代码
  created_at    TIMESTAMPTZ DEFAULT now()
);

-- 六维溯源绑定
CREATE TABLE trace_links (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id    UUID REFERENCES projects(id),
  requirement_id TEXT,
  feature_id     TEXT,
  business_id    TEXT,
  algorithm_id   TEXT,
  task_id        TEXT,
  code_id        TEXT,
  UNIQUE (requirement_id, feature_id, business_id, algorithm_id, task_id, code_id)
);
```

---

## 5. κ/τ 调度算法（scheduler.regularize）

```
输入: proposal(DAG), k, t, C, project.domain
1. # κ 复用：在 assets 中按 domain + embedding 相似度检索 Top‑K 候选
2. # τ 新建：topology‑operator 在 proposal 中标记「新建节点」(new=true)
3. 预算代价 cost = Σ node_weight + Σ edge_weight(node,edge)
4. Δ = C*C - (k*k + t*t)            # k²+t²=1 ⇒ Δ = C²-1；若实际 cost 超 C 则额外裁剪
5. while Δ < 0 OR cost > C:
       candidate = argmin priority(edge or node)   # priority 由 τ‑新建度、耦合度给出
       prune(candidate); cost -= weight(candidate); Δ += 1
6. 若仍存在矛盾环(cycle not in DAG) → 标记 rejected，回写 conversation 让 LLM 重生成
7. 输出 regularized DAG + Δ + 复用/新建统计
```

> **κ‑τ 自动寻优（命题3 短板）本期方案**：不做 RL。记录每次 `s→k,t→用户采纳/驳回` 的反馈日志，提供「建议滑块」基于历史采纳分布；未来接入强化学习闭环（V2）。

---

## 6. API 契约（REST，JSON）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/projects` | 建项目（含默认 k/t、预算 C） |
| POST | `/api/projects/:id/messages` | 发送自然语言需求 + 滑块 `s`；触发链路 |
| GET  | `/api/topologies/:id` | 取 DAG 供画布渲染 |
| POST | `/api/topologies/:id/regularize` | 显式触发 ℛ̂（滑块可调后重算） |
| GET  | `/api/projects/:id/artifacts` | 列出 8 份文档 |
| GET  | `/api/artifacts/:aid` | 取单份文档/代码 |
| POST | `/api/projects/:id/generate-docs` | 触发生成 8 文档 |
| GET  | `/api/assets?q=&domain=` | κ 复用检索（pgvector 相似度） |
| POST | `/api/topologies/:id/freeze` | 冻结为资产 Q（写 embedding） |

所有写接口需 `X-Actor` 头；错误统一 RFC9457（`application/problem+json`）。

---

## 7. 模块拆分与目录结构

```
primiflow/
├── go/                      # 编排层
│   ├── cmd/server/          # 启动入口
│   ├── internal/gateway/    # 鉴权/校验/REST
│   ├── internal/orchestrator/# 状态机: requirement→topology→docs
│   ├── internal/scheduler/  # κ/τ 预算 + ℛ̂ 裁剪
│   └── internal/asset/      # 资产检索/冻结
├── python/                  # 算子层
│   ├── llm_gateway/         # 模型抽象 + 提示链
│   ├── topology_operator/   # 需求→DAG
│   ├── doc_generator/       # 8 文档生成
│   └── smoke_tester/        # schema 校验 + 冒烟
├── web/                     # React + Cytoscape
│   ├── ChatPanel/  Canvas/  DocViewer/  AssetLibrary/
├── db/                      # 迁移脚本 (sql)
└── deploy/                  # docker-compose (pg+go+py+web)
```

Go↔Python 通过 gRPC（topology/document 生成）或 REST 内部调用；MVP 允许用 REST 简化。

---

## 8. 八份标准化说明书（artifacts.kind）

| # | 文档 | 内容要点 |
|---|------|---------|
| 1 | 需求规格说明书 | 结构化需求树、约束、验收标准 |
| 2 | 功能设计说明书 | 功能清单、模块划分、用例 |
| 3 | 业务流程说明书 | 主/子流程、角色、异常 |
| 4 | 数据模型说明书 | 表/字段/关系/索引 |
| 5 | 接口契约说明书 | REST/RPC 契约、错误码 |
| 6 | 定时任务说明书 | 调度周期、幂等、失败重试 |
| 7 | 代码工程说明书 | 目录结构、关键模块、代码骨架/桩 |
| 8 | 部署运维说明书 | 依赖、环境变量、观测、回滚 |

> MVP 中 #7 仅产出**骨架/桩代码**（承接命题5「暂不实现全自动代码生成」）。

---

## 9. 风险与缓解（承接核验命题3）

| 风险 | 缓解 |
|------|------|
| 大模型幻觉传导（需求/拓扑继承错误） | ℛ̂ 后接 schema 校验 + smoke‑tester 冒烟；失败回写对话重生成，不静默放行 |
| κ‑τ 自动寻优未收敛 | 本期规则+滑块+反馈日志；V2 接 RL |
| 仅限业务软件域 | 在 topology‑operator 入口做域白名单，超域任务显式拒绝 |
| 资产检索噪声 | pgvector 相似度阈值 + domain 硬过滤双保险 |
| 长链路超时 | orchestrator 异步化 + 可轮询状态，前端渐进渲染 |

---

## 10. 验收标准（DoD）

- [ ] 输入自然语言需求 + 滑块，端到端产出可渲染 DAG 画布（无人工画流程图）。
- [ ] ℛ̂ 对任意超预算/矛盾拓扑产出 `Δ≥0` 的合规 DAG，或显式 rejected 并触发重生成。
- [ ] 8 份文档可从拓扑自动生成并在 DocViewer 查看；#7 为骨架/桩。
- [ ] 六维 `trace_links` 对每条需求‑功能‑业务‑算法‑任务‑代码建立可追溯绑定。
- [ ] κ 复用：第二次同类需求能检索到首次冻结的资产 Q 并优先复用。
- [ ] `cargo`/Go test + Python pytest + 前端构建全绿；冒烟用例覆盖主链路。

---

## 11. 开发排期建议（全职 3 人，28‑35 天）

| 阶段 | 周 | 交付 |
|------|----|------|
| P0 基建 | 1‑2 | db 迁移 + Go/Py/Web 脚手架 + docker‑compose 跑通 |
| P1 对话→拓扑 | 2‑4 | llm‑gateway + topology‑operator + 画布渲染 |
| P2 正则化 | 3‑5 | scheduler κ/τ + ℛ̂ + 资产冻结/检索 |
| P3 文档链 | 4‑6 | doc‑generator 8 文档 + 六维溯源 |
| P4 收口 | 6‑7 | 冒烟测试 + 端到端验收 + 文档沉淀 |

---

*本规格为开发前冻结版本；评审通过后据此立项编码。任何范式参数（κ/τ/C 语义、ℛ̂ 算法）变更须回填本节并 bump 版本。*
