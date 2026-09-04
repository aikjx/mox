# API-INDEX · 接口契约归一化（API-xx）

> 编号：**DOC-NORM-API-V1.0** · 归属：[README.md](README.md)（SSoT 枢纽）
> 内容：mox 前端 17 个 API 模块、191 端点、REST/事件/SPI 契约归一化。

---

## 1. API 模块清单（SSoT = `frontend-ui/src/MODULE-MANIFEST.md` §4）

| 文件 | 主要导出 | 后端端点域 | 分组 |
|------|----------|-----------|------|
| `http.js` | axios 实例 + 拦截器 | 基础封装 | 基础 |
| `alliance.js` | `runAllianceFullSSE`, `getAllianceCapabilities` | `/api/alliance` | 专家联盟 SSE |
| `experts.api.js` | `getExperts`, `expertDebate`, `multiExpertConsult`, `routeExperts`, `registerExpert` | `/api/experts` | 专家 |
| `kb.api.js` | `kbListDocuments`, `kbGetCategories`, `kbSearch` | `/api/kb` | 知识库 |
| `projects.api.js` | `getProjects` | `/api/projects` | 项目 |
| `graph.api.js` | 图谱数据操作 | `/api/graph` | 图谱 |
| `ai.api.js` | `aiChat`, `aiFullComplete`, `aiDevTestFix`, `aiGenerateDoc` | `/api/ai` | AI |
| `llm.api.js` | `getLlmProviders`, `setActiveProvider`, `updateLlmRouting` | `/api/llm` | 大模型 |
| `workflow.api.js` | `getWorkflows`, `executeWorkflowDef`, `validateFlow` | `/api/workflow` | 工作流 |
| `operators.api.js` | `getOperators`, `registerOperator` | `/api/operators` | 算子 |
| `market.api.js` | `marketList`, `marketUpload` | `/api/market` | 应用市场 |
| `mox.api.js` | `moxHealth`, `moxOptimize`, `moxPublish` | `/api/mox` | 融合平台 |
| `system.api.js` | 系统配置/用户/角色/菜单 | `/api/system` | 系统管理（mox_sys） |
| `monitor.api.js` | 监控指标/日志 | `/api/monitor` | 监控 |
| `actuator.api.js` | 健康检查/指标 | `/actuator` | 运维 |
| `caomei.api.js` | `caomeiCompile`, `caomeiRefine` | `/api/caomei` | 需求编译 |
| `melody.api.js` | `melodyRecognize`, `melodyExportSheet` | `/api/melody` | 旋律转谱 |
| `workspace.api.js` | 工作台数据 | `/api/workspace` | 工作台 |

### 1.1 meta.codegen 出码端点（2026-09-04 新增 · 后端 Rust）

| 端点 | 落点 | 说明 |
|------|------|------|
| `POST /api/enterprise/v1/entities/codegen` | `mox-platform-enterprise-svc` routes.rs | 输入 `{tenant_id?, entity_code, detail_entity_code?, template}`（实体须已 define），输出 `{entity_code, template, artifacts}`；TPL-01~06 |
| `POST /api/mox/codegen-publish` | `mox-platform-orchestrator-svc` codegen_gate.rs | 出码 + 闸门：内联实体元数据 → 出码 → 与 `/api/mox/publish` 同链（⛨verify + 8 闸门 + I-05 双验收），`released=false` 时拦截不放行 |

---

## 2. 端点覆盖（191 接口分组，事实来源：功能图谱 §10.2）

| 分组 | 接口数 | 验证点 |
|------|:--:|------|
| 系统健康 | 5 | `getHealth` → healthy |
| 知识图谱 | 18 | `getGraphStats` 可调用 |
| AI 核心 | 18 | `aiChat` 可调用 |
| 专家联盟 | 15 | `getExperts` 可调用 |
| 大模型 | 15 | `getLlmProviders` 可调用 |
| 项目管理 | 12 | `getProjects` 可调用 |
| 任务管理 | 6 | `getTasks` 可调用 |
| 工作流 | 11 | `getWorkflows` 可调用 |
| 知识库 | 8 | `kbListDocuments` 可调用 |
| 其他 | 83 | 按模块分组验证 |

---

## 3. 契约规范（归一化约定）

- **分层**：页面只经 `api/*.api.js` 调 `http.js`，禁止页面直接用 axios。
- **事件/SSE**：专家联盟、工作台走 SSE（`alliance.js` / `useAlliance.js`）。
- **SPI（插件）**：Plugins/MCP/算子 经 manifest + SPI 接入，契约见 `ARC-INDEX` §插件。
- **命名**：端点域 `/api/{domain}`；函数名 `动词+名词` 小驼峰。

---

## 4. 登记规则

- 新接口契约 `API-{两位序号}-{中文短名}.md` 放 `docs/normalization/api/`，须含请求/响应样例与错误码（错误码参考 `docs/architecture/04-error-code-reference.md`）。
- 跨文档引用 `docs/normalization/API-INDEX.md#章节`。
