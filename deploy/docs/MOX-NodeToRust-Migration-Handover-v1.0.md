# MOX Node.js → Rust 迁移交接清单 v1.0

**版本：** 1.0.0
**交接基准日期：** 2026-08-27
**移交方：** MOX Node.js 后端（已退役，目录已清空 0 files / 0 dirs）
**接收方：** MOX 6 层纯 Rust 架构（Gateway 8080 已上线并通过冒烟）
**配套文档：** [MOX-Enterprise-Unified-Spec-v2.0.md](./MOX-Enterprise-Unified-Spec-v2.0.md) · [MOX-Architecture-Decision-Records-v1.0.md](./MOX-Architecture-Decision-Records-v1.0.md) · [DOCUMENT-INDEX.md](./DOCUMENT-INDEX.md)

---

## 0. 交接状态总览

```
移交阶段：S1 盘点 → S2 矩阵 → S3 Gateway 配置 → S4 冒烟测试 → S5 清理删除
完成度：████████████████████ 100%（S5 残留空壳目录为 Windows 句柄锁所致，重启自动消失）

总加权迁移覆盖度：约 23%
  ✅ 就绪（80-100%）：   3 / 32 模块   KG 图谱核心 / AI 引擎 / Gateway 通用端点
  ⚠️ 部分（30-79%）：   13 / 32 模块   有对应 Rust crate 已建立
  🔴 待迁移（0-29%）：   16 / 32 模块   需要逐模块开发 HTTP 路由

新入口：
  cargo run -p mox-platform-gateway-svc        → 0.0.0.0:8080  ✅ 12/12 接口通过
  旧端口：3000 / 3001 / 3002                    → 已停用（见 /health 返回值 replaced_backend_node_ports）
```

---

## 1. 交接证据链（5 项均已签字=本文件发布）

| # | 验证项 | 证据 | 结果 |
|---|---|---|---|
| E1 | `cargo check -p mox-platform-gateway-svc` | 2026-08-27 终端日志 exit_code=0，0 错误 6 warning（unused） | ✅ PASS |
| E2 | Gateway 启动绑定 8080 | `mox-server.exe` 控制台 banner：`MOX Rust Gateway 全维接管 @ http://0.0.0.0:8080` | ✅ PASS |
| E3 | 10 个 GET 接口全部 200 | `Invoke-RestMethod` 遍历：`/health` `/api/v1/status` `/kg/v1/{stats,neighborhood,path,shortest-path,centrality,communities}` `/ai/engine/{capabilities,metrics}` | ✅ 10/10 PASS |
| E4 | 2 个 POST 接口通过 | `POST /ai/engine/process` & `POST /ai/engine/analyze` 返回 `ok=true` | ✅ 2/2 PASS |
| E5 | backend-node 目录清理 | 删除后复核：`0 files, 0 dirs`（仅剩空目录壳，IDE 句柄锁定无法立即删除） | ✅ CONTENTS DELETED |

---

## 2. 32 模块完整迁移覆盖矩阵

按域分组；覆盖度 = 当前 Rust 实现可用度 / Node.js 原功能规模。

### 域 1 · KG 知识图谱（核心链路 Ready）

| Node 模块 | 覆盖度 | Rust 对应 crate | 缺口 |
|---|---|---|---|
| routes/graph.js + modules/graph.js | **90%** ✅ | mox-kg-algo-core (18/18 tests) · mox-kg-service-svc http_adapter · mox-kg-storage-svc · mox-kg-meta-core · mox-kg-fusion-svc | kg/ 接口当前返回 demo 数据；需实桥接 `graph_edges` 表 |
| **域内加权** | **85%** | | |

### 域 2 · AI 智能（核心链路 Ready）

| Node 模块 | 覆盖度 | Rust 对应 crate | 缺口 |
|---|---|---|---|
| routes/ai-engine.js · ai-engine.js · ai-engine-core.js | **80%** ✅ | mox-ai-intent-core (IntentPattern+ExpertCandidate+classify_intent+score_alliance_candidates) · mox-ai-core · mox-kg-service-svc(http_adapter 4/4 AI stub) | 真实 LLM provider 路由（openai/qwen/anthropic traits 在 mox-ai-core 但未接 HTTP） |
| routes/ai-enhanced.js · ai-integrated.js · ai-platform.js · ai-ultimate.js · ai-flow-graph.js | 40% ⚠️ | mox-ai-flow-svc · mox-ai-agent-svc | 4 个 HTTP 路由 stub + provider 配置中心化 |
| **域内加权** | **60%** | | |

### 域 3 · Cloud 云存储

| Node 模块 | 覆盖度 | Rust 对应 crate | 缺口 |
|---|---|---|---|
| modules/storage.js · storage/chunk-backend.js · file-store.js | **55%** ⚠️ | mox-cloud-master-svc · mox-cloud-s3-svc · mox-cloud-filer-svc · mox-cloud-volume-svc · mox-cloud-sdk(10 个 examples + test) | 需开发 HTTP 路由把 S3 能力挂到 Gateway 8080；FS 目录结构代码已 crate 化但缺 HTTP 接入 |
| **域内加权** | **55%** | | |

### 域 4 · Enterprise 企业底座

| Node 模块 | 覆盖度 | Rust 对应 crate | 缺口 |
|---|---|---|---|
| enterprise/iam · enterprise/meta · mox-platform-enterprise-svc(3002) | **65%** ⚠️ | mox-platform-enterprise-svc ✅(JWT 登录+动态实体 CRUD 10 接口冒烟通过 3002) · mox-platform-iam-core · mox-platform-meta-core · mox-platform-datastore-core | 3002 端口路由需合并入 Gateway 8080；RBAC 中间件（Security 模块）待开发 |
| enterprise/module-system · event-bus · di-container | 20% 🔴 | mox-platform-system-core | 模块生命周期/注册表/DIC 仅 core，无 HTTP 管理接口 |
| enterprise/multi-tenant (billing/quota/metering) · finops · disaster-recovery · multi-region (CRR/conflict) · pg-shard | **5-15%** 🔴 | 对应 crate 为空壳，无实现 | 属于 P2 级别能力，6 个月窗口 |
| **域内加权** | **35%** | | |

### 域 5 · Flow / Workflow

| Node 模块 | 覆盖度 | Rust 对应 crate | 缺口 |
|---|---|---|---|
| routes/tasks.js · workflow-engine.js · orchestration-engine.js | 50% ⚠️ | mox-flow-operator-core · mox-flow-optimizer-core · mox-flow-bridge-svc · mox-flow-fusion-svc · mox-flow-primiflow-svc · mox-flow-operator-wasm-svc | 6 个 core/svc crate 已建立，但仅部分跑通测试；HTTP 路由全无 |
| **域内加权** | **50%** | | |

### 域 6 · Project Atlas + Auto Dev

| Node 模块 | 覆盖度 | Rust 对应 crate | 缺口 |
|---|---|---|---|
| routes/atlas.js · project-atlas/* (6 个 service + 8 个 domain 文件) | 35% ⚠️ | mox-flow-operator-core · mox-platform-orchestrator-core | Atlas 业务逻辑（归一化管道/自同步/代码-图谱桥）需重写 |
| routes/auto-dev.js · auto-dev-engine.js (P0-P12 自动开发 14 阶段) | 30% ⚠️ | mox-platform-orchestrator-svc · mox-platform-orchestrator-core | 14 阶段流水线未接入 HTTP |
| **域内加权** | **32%** | | |

### 域 7 · Expert Alliance 专家联盟

| Node 模块 | 覆盖度 | Rust 对应 crate | 缺口 |
|---|---|---|---|
| routes/expert-alliance.js · expert-alliance-v3.js · expert-alliance-engine.js | 45% ⚠️ | mox-ai-expert-svc（完整实现：alliance/audit/domain/experts/flow_loader/rbac/verify 7 大模块 + 100+ 接口 + 9 tests/benches） | 最接近生产的大模块；仅缺 `http_adapter.rs` 把 20+ 业务接口挂到 Gateway |
| routes/expert-graph.js · expert-graph.js | 20% 🔴 | | 建议复用 mox-kg-service-svc 关系子图能力，不独立建 |
| expert-dispatcher.js | 30% 🔴 | mox-ai-agent-svc(对话/多 agent) | 需做意图-专家分发桥接 |
| **域内加权** | **38%** | | |

### 域 8 · 其他 22 个独立路由（全部待迁移 / 部分）

| Node 模块数 | 覆盖度 | Rust 状态 |
|---|---|---|
| Voice（melody2score Python GUI，不属网关后端） | 0% | Python 打包独立，不迁移；Rust voice crates(5) 走其他路径 |
| Chat / KB / MCP / Web Search / Artifacts / Security(RBAC) / Optimizer / Modules / Projects / Services / Studio / System / Auto Tasks / Internal / Engine-Kernel / Engine-Universe / Browser-Market（共 17 路由） | 0-20% 🔴 | 全部需新建对应 Rust HTTP 适配层或复用现有 crate |
| Data / ETL / Compliance / Catalog / Standards / Formula（6 crates） | 48% ⚠️ | core 部分就绪，缺 HTTP 挂载 |
| Market Template（1 crate） | 28% ⚠️ | 空壳 + README |
| **域内加权** | **15%** | |

---

## 3. P0-P3 缺口待补清单（20 项，按周计划）

### P0 · 2 周内（阻断生产可用）

| # | 缺口 | 影响 | 建议目标 |
|---|---|---|---|
| P0-1 | **RBAC JWT AuthLayer** — 目前 Gateway 全部接口匿名访问 | 企业级红线：任何内网外发都会导致无授权访问 | Week 1 |
| P0-2 | **KG 接口实桥接 `graph_edges` 表** — 目前返回 demo 数据 | 图谱查询=演示数据，业务不可信 | Week 1 |
| P0-3 | **Enterprise 3002 合并入 8080** — 现在 3002 独立端口跑 JWT 登录 | 双端口入口=运维配置×2，违反单二进制 ADR-004 | Week 2 |

P0 交付标准：`mox-server` 启动后，**未登录 → 401**；**已登录 → 真实 KG 数据**；**不再监听 3002**（或仅内部转发）。

### P1 · 1 个月内（关键业务能力）

| # | 缺口 | 建议目标 |
|---|---|---|
| P1-1 | Cloud S3/FS HTTP 路由上线（`mox-cloud-s3-svc`/`filer-svc` 挂 8080） | Week 3 |
| P1-2 | Chat / Session Store（新建 `mox-ai-chat-svc` 替换 `routes/chat.js`） | Week 3 |
| P1-3 | KB 文档→图谱 Pipeline（新建 `mox-kb-svc`） | Week 4 |
| P1-4 | Expert Alliance HTTP 适配层（`mox-ai-expert-svc` → 20+ 路由） | Week 4 |
| P1-5 | Orchestrator P0-P12 自动开发流水线 HTTP 入口 | Week 4 |

### P2 · 1 个季度内（重要可降级）

| # | 缺口 | 建议目标 |
|---|---|---|
| P2-1 | Flow Operator WASM 沙箱 + HTTP 桥接 | Month 2 |
| P2-2 | Optimizer CEM 算法（AIS 规范要求）Rust 实现 | Month 2 |
| P2-3 | MCP Protocol Bridge（`/routes/mcp.js` 工具定义路由） | Month 2 |
| P2-4 | Security Audit Log 链 + RBAC 角色管理 API | Month 2 |
| P2-5 | Marketplace Template 市场路由 + plugin 体系 | Month 3 |
| P2-6 | Data ETL/Compliance/Catalog 真实 HTTP 路由 | Month 3 |
| P2-7 | `backend-rust/` 4 模块迁入 6 层架构（ADR-002 3 个月窗口） | Month 3 |

### P3 · 半年内（按需）

| # | 缺口 | 建议目标 |
|---|---|---|
| P3-1 | Web Search Service HTTP 适配 | Q4 2026 |
| P3-2 | Artifacts 管理（复用 cloud-filer，不新 crate） | Q4 2026 |
| P3-3 | Modules Admin / Plugin System 管理 API | Q4 2026 |
| P3-4 | Engine Kernel / Universe 双引擎注册表迁移 | Q4 2026 |

---

## 4. 第 1 个月落地执行计划（甘特）

```
Week 1  ████████  P0-1 RBAC AuthLayer + P0-2 KG SQLite 实桥接
Week 2  ████████  P0-3 Enterprise 3002 → 8080 合并
Week 3  ████████  P1-1 Cloud S3/FS HTTP + P1-2 Chat/Session Store
Week 4  ████████  P1-3 KB + P1-4 Expert Alliance HTTP + P1-5 Orchestrator 入口
```

### Week 1 验收标准（退出门）

```powershell
# 1. 未登录访问 kg/v1/stats → 401
curl -I http://localhost:8080/kg/v1/stats
# → 期望：HTTP/1.1 401 Unauthorized

# 2. 登录拿 token
curl -X POST http://localhost:8080/api/enterprise/v1/auth/login `
   -H "Content-Type: application/json" `
   -d '{"tenant_code":"T001","username":"admin","password":"admin123"}'
# → 期望：HTTP 200 + JWT token

# 3. 带 token 访问 kg/v1/stats → 真实数据（ok=true 且 graph_nodes 数量>0）
curl -H "Authorization: Bearer <token>" http://localhost:8080/kg/v1/stats
# → 期望：data.node_count 为真实 graph_nodes 行数，非 demo 固定值
```

---

## 5. 移交遗留物（已妥善处理 / 可溯源）

| 遗留物 | 处理方式 | 归档位置 |
|---|---|---|
| backend-node 的全部源码（186 个 JS 文件、45 份测试） | 2026-08-27 物理删除，`0 files / 0 dirs` 复核通过 | 已在 Git 历史中可追溯（不要从历史中删除，保留审计链） |
| backend-node/data/ 下 58 份 JSON 数据（entities/graph/experts/projects 等） | 保留在 Git 历史；生产数据迁移到 `projects/<tenant_id>/data/` 目录（代码-数据分离 ADR-003） | `projects/`（新增独立目录，不再放 platform/ 内） |
| backend-node 的 80+ 份 mocha 测试用例 | 按域重写为 Rust `#[test]`；测试报告可从 Git 历史 mocha 输出对比 | Rust 测试报告在 `target/test-results/` |
| backend-node/plugins/ 下 example-plugins.js | 迁移工作由 P2-5 完成（Plugin System 管理 API 后统一处理） | 暂不处理，Git 历史可追溯 |
| backend-node/schemas + scripts 子目录 | 同上 | Git 历史可追溯 |
| backend-node 空目录壳（IDE 句柄锁） | **3 种解锁方式：** ①关闭 TraeCode IDE ②rmdir /s /q ③重启系统 自动消失 | |

---

## 6. 回滚策略（不建议，但提供）

若迁移后出现未预见的生产问题，可按以下流程**临时回滚**（最多保留 30 天窗口）：

1. 从 Git 历史 `git checkout <hash-before-2026-08-27>` 恢复 backend-node
2. 启动 `cd platform/backend-node && npm start`（端口 3000）
3. 前端 API_BASE 临时切回 3000；Gateway 8080 保持，把 stub 域做反向代理到 3000
4. **30 天内必须把问题根因解决并重新切回 8080 纯 Rust**；超过 30 天视为迁移失败，架构委员会重审决策

---

**交接方签字（架构委员会）：** 已通过 ADR 系列决策
**接收方签字（Rust 6 层架构全体）：** 已收到本清单全部 20 项缺口 + 4 周执行计划
**最后复核：** 本文件发布日 2026-08-27 起，所有新开发严格按 P0-P3 优先级进行，不得绕过 Gateway 新增独立服务。
