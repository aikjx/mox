# 璇玑系统 · 全局端口注册表（PORT-REGISTRY-001）

> **标题**：璇玑系统 · 全局端口注册表
> **版本**：V1.0
> **权威等级**：🟢权威
> **编号**：PORT-REGISTRY-001
> **最后更新日期**：2026-09-01
> **适用范围**：**整个 infotopograph 仓库**（全部运行服务、附属服务、遗留服务、测试服务、历史/已退役端口、第三方基础设施引用）
> **单源声明**：本文档是**全仓库端口分配的唯一权威来源**。凡涉及端口规划、分配、迁移、避让的决策与文档，均以本文档为准；与本文档冲突时，以本文档为准并修复冲突来源。专家联盟核心服务段（3000–3999）同时受 `docs/standards/expert-alliance-port-norm.md`（PORT-NORM-001）约束，两者一致，PORT-NORM-001 为本表 3000–3999 段的细粒度权威。

---

## 第1章 分类总览

| 分类 | 说明 | 端口段/示例 |
|---|---|---|
| **RUNTIME** | 由 `scripts/server-manage.py` 统一管理、`platform_config.json` 登记的**当前运行服务** | 8080 / 3020 / 30010 / 8012 / 8000 / 3999 |
| **ALLIANCE** | 专家联盟核心服务（PORT-NORM-001 强制 3000–3999 段） | 3100 / 3200 / 3300 |
| **ANCILLARY** | 运行期附属端口（gRPC、内网控制面、OUS 边缘、前端预览等） | 50051 / 50052 / 9080 / 9081 / 4173 / 3998 / 7000 / 3000 / 3001 / 3002 |
| **LEGACY** | 遗留模块，自洽但不再纳入统一运维（Python mox-server / mox-store / docker） | 8600 / 8601 / 6379(infra) |
| **DEPRECATED** | 已退役/历史端口，**禁止新服务复用** | 3010 / 3021 / 3717 |
| **TEST-ONLY** | 仅供测试/内存 mock 的端口，不进入运行链路 | 8001–8003、9000–9003、9101–9103、9201–9203、9301–9303、9333、9401–9403、9501–9503、9669、9779–9781、9998/9999、12345、13130、19601–19603、19876、19999、35432、65528–65530 等 |
| **THIRD-PARTY** | 第三方基础设施/中间件默认端口（部署引用，非本项目监听） | 3306 / 5432 / 2379 / 2380 / 4222 / 4317 / 6379 / 7687 / 8200 / 8848 / 9000 / 9001 / 9090 / 9093 / 5236 / 54321 / 7480 等 |

---

## 第2章 快速速查表（开发常用）

| 服务 | 端口 | 协议 | 绑定 | 入口/访问 | 配置权威来源 |
|---|---|---|---|---|---|
| **api**（Rust 网关 mox-gateway / mox-server） | **8080** | HTTP | 0.0.0.0 | `http://localhost:8080/health` | `platform_config.json`、`deploy/config/gateway.yaml`、`mox-workspace/.env.example` |
| **frontend**（Vite Vue3 dev server） | **3020** | HTTP | 0.0.0.0 | `http://localhost:3020/` | `frontend-ui/vite.config.js`、`platform_config.json` |
| **xiaobai_voice**（ASR+TTS） | **30010** | HTTP/WS | 127.0.0.1 | `http://localhost:30010/voice/health` | `projects/xiaobai_voice/xiaobai_voice/config/default_config.yaml`、`cli.py`、`platform_config.json` |
| **melody2score**（旋律转谱 WebUI） | **8012** | HTTP | 0.0.0.0 | `http://localhost:8012/` | `projects/melody2score/app/webui.py`、`platform_config.json` |
| **primiflow**（低代码拓扑引擎） | **8000** | HTTP | 0.0.0.0 | `http://localhost:8000/` | `projects/primiflow/backend/main.py`、`platform_config.json` |
| **dashboard**（运维管理面板） | **3999** | HTTP | 0.0.0.0 | `http://localhost:3999/` | `platform_config.json` → `dashboard_port` |
| **alliance scheduler-svc** | **3100** | HTTP | 0.0.0.0 | `http://localhost:3100/health` | `config/alliance-scheduler.yml` |
| **alliance executor-svc** | **3200** | HTTP | 0.0.0.0 | `http://localhost:3200/health` | `config/alliance-executor.yml` |
| **alliance expert 桥接** | **3300** | HTTP（内部基址） | — | scheduler → 3300 | `config/alliance-scheduler.yml` → `expert_service.base_url` |

---

## 第3章 端口分配细则

### 3.1 RUNTIME —— 统一运维服务（`platform_config.json` 登记，`manage.py` 管理）

| 端口 | 服务 key | 名称 | 协议 | 绑定 | 健康检查 | 状态 |
|---|---|---|---|---|---|---|
| 8080 | `api` | API 后端服务（Rust mox-gateway） | HTTP | 0.0.0.0 | `/health` | 🟢运行中 |
| 3020 | `frontend` | 用户前端界面（Vite + Vue3） | HTTP | 0.0.0.0 | `/` | 🟢运行中 |
| 30010 | `xiaobai_voice` | 小白语音服务（ASR + TTS） | HTTP/WS | 127.0.0.1 | `/voice/health` | 🟢运行中 |
| 8012 | `melody2score` | 旋律转谱服务（FastAPI WebUI） | HTTP | 0.0.0.0 | `/` | 🟢运行中 |
| 8000 | `primiflow` | PrimiFlow 低代码拓扑引擎 | HTTP | 0.0.0.0 | `/` | 🟢运行中 |
| 3999 | `dashboard` | Web 管理面板 | HTTP | 0.0.0.0 | `/` | 🟢运行中 |

> **依赖关系**：`frontend`(3020) `depends_on` `api`(8080)；`api` 的 `/voice/**` 路由代理到 `xiaobai_voice`(30010)。
> **前端代理**：`frontend-ui/vite.config.js` 中 `/api` → `http://localhost:8080`；`/ai/engine`、`/voice`、`/ws` 默认 → `http://localhost:8080`（可用 `GATEWAY_URL` 环境变量覆盖）。

### 3.2 ALLIANCE —— 专家联盟核心服务（3000–3999 段，PORT-NORM-001）

| 端口 | 服务 | crate | 协议 | 配置 | 状态 |
|---|---|---|---|---|---|
| 3100 | scheduler-svc（调度编排） | `mox-alliance-scheduler-svc` | HTTP | `config/alliance-scheduler.yml` | 🟢已启用 |
| 3200 | executor-svc（执行引擎） | `mox-alliance-executor-svc` | HTTP | `config/alliance-executor.yml` | 🟢已启用 |
| 3300 | AI 专家服务（桥接基址） | scheduler 内部桥接 | HTTP | `config/alliance-scheduler.yml` → `expert_service` | 🟢已启用 |

> 配置加载优先级：内置默认 < `config/alliance-*.yml` < 环境变量 `MOX_ALLIANCE_*`（如 `MOX_ALLIANCE_SERVER_PORT=3100`）。

### 3.3 ANCILLARY —— 运行期附属端口

| 端口 | 用途 | 绑定/默认 | 权威来源 |
|---|---|---|---|
| 50051 | gRPC（`mox-dualrpc` / framework 默认 / 专家联盟内部 gRPC） | 0.0.0.0 | `mox-workspace/.env.example` → `MOX_GRPC_PORT`；`platform/framework/src/config.rs` |
| 50052 | gRPC 备用端口（架构文档提及） | — | `docs/architecture/OPTIMAL_ARCHITECTURE.md` |
| 9080 | data-plane-svc 内网控制面（ctrl） | 127.0.0.1 | `platform/domains/data/svc/mox-data-plane-svc/src/listeners.rs` |
| 9081 | data-plane-svc 内网数据面（data） | 127.0.0.1 | `platform/domains/data/svc/mox-data-plane-svc/src/listeners.rs` |
| 4173 | frontend 生产预览（Vite preview） | 0.0.0.0 | `frontend-ui/vite.config.js` → `preview.port` |
| 3998 | operator API（mox-ai-agent-svc `OPERATOR_API_BASE` 默认；`runtime --port` 测试） | 127.0.0.1 | `platform/domains/ai/svc/mox-ai-agent-svc/src/workflow_engine.rs`、`scripts/tests/verify_tests.*` |
| 7000 | mox-dr raft（helm `containerPort`；历史文档亦见 8200） | — | `deploy/helm/mox-dr/templates/NOTES.txt` |
| 3000 | OUS 算子统一系统边缘（`mox-platform-system-core` 默认绑定；曾为 Node 边缘入口；注意 Grafana 默认同为 3000，部署需避让） | 0.0.0.0 | `platform/domains/platform/core/mox-platform-system-core/src/config.rs` |
| 3001 | orchestrator-svc（operator-unified-system）HTTP 默认绑定（`--port` 默认 3001） | 0.0.0.0 | `platform/domains/platform/svc/mox-platform-orchestrator-svc/src/main.rs` |
| 3002 | enterprise-svc 默认绑定（休眠/备用服务） | 0.0.0.0 | `platform/domains/platform/svc/mox-platform-enterprise-svc/src/main.rs` |

### 3.4 LEGACY —— 遗留模块（自洽，不纳入统一运维）

| 端口 | 模块 | 说明 | 权威来源 | 状态 |
|---|---|---|---|---|
| 8600 | legacy Python `mox-server`（低代码平台旧后端） | docker-compose / systemd / nginx 反向代理指向此端口；**与 Rust 网关 api=8080 是两个不同服务**，注意同名“mox-server”易混淆 | `docker-compose.yml`、`deploy/Dockerfile`、`deploy/systemd.service`、`tools/deploy.py`、`platform/legacy/mox-server/run.py` | 🟡遗留（仍在部署链路） |
| 8601 | legacy `mox-store`（应用商店） | FastAPI 商店服务 | `platform/legacy/mox-store/store_server.py` | 🟡遗留 |
| 6379 | **redis**（基础设施） | docker-compose / systemd 引用 | `docker-compose.yml`、`deploy/systemd.service`、`mox-workspace/.env.example` | 🟢基础设施 |

> **注意**：legacy `mox-server`(8600) 与 Rust 网关 `mox-server`(8080) **二进制同名不同物**。新代码一律以 8080 为唯一 API 入口；8600 仅服务遗留静态站点（`mox-website` / `mox-console` / `chip-website`）。

### 3.5 DEPRECATED —— 已退役/历史端口（禁止新服务复用）

| 端口 | 原归属 | 退役原因 | 备注 |
|---|---|---|---|
| 3010 | Node.js 平台 API / Node sidecar（`platform/backend-node`） | `backend-node` 已删除 | orchestrator 侧车默认已改指向 Rust 网关 8080（2026-09-01，见 §6.2）；历史文档仍可能引用 |
| 3021 | 前端旧端口（AI 对话 UI 曾用） | 前端端口统一为 **3020**（vite `server.port`） | 桌面端/shared 常量/校验脚本均已改为 3020 |
| 3717 | xiaobai_voice 旧端口（ASR+TTS） | 2026-09-01 按 PORT-NORM-001 4.2 迁至 **30010** | 历史文档（ARCHITECTURE/enterprise 报告）仍可能显示 3717，以本表为准 |
| 8081 / 8082 | 专家联盟 scheduler-svc / executor-svc 旧端口（已迁 **3100 / 3200**） | 按 PORT-NORM-001 迁移 | 现仅作 `mox-dualrpc` 测试端口（TEST-ONLY） |
| 18080 / 19080 / 19081 | single-node 验证模式（public / ctrl / data） | 仅 t19 回归验证产物使用 | `projects/t19-regression/`、`scripts/validation/validate-single-node.js` |

### 3.6 TEST-ONLY —— 测试/内存 mock 保留端口（不进入运行链路）

> 以下端口仅出现在单元测试 / 集成测试 / 内存 mock / 本地探针中，**并非真实运行服务**，禁止在生产启动链路使用，也**禁止与其他服务分配冲突**（测试端口可跨服务重复使用，因不同时监听）。

| 端口 | 用途 |
|---|---|
| 8001–8003 | mox-cloud-master-svc 卷节点测试 |
| 9000–9003 | 云 master / kg-storage 节点测试；MinIO S3 默认（9000/9001） |
| 9101–9103 / 9201–9203 / 9301–9303 / 9401–9403 / 9501–9503 | kg-storage-svc 各分片/查询/基准测试 |
| 9333 | mox-cloud-master-svc raft 测试 |
| 9669 / 9779–9781 | kg-meta-core 存储宿主测试 |
| 8999 | alliance executor expert 模式 e2e：mock OpenAI 兼容服务（tools/mock_openai.py，仅测试） |
| 9848 / 10848 | rnacos（Nacos Rust 服务端）gRPC / 独立控制台端口（本地 e2e，tools/rnacos/；HTTP 8848 已登记 THIRD-PARTY） |
| 9998 / 9999 | legacy backend-rust 网关 target 测试 |
| 12345 | glacier-adapter 测试 endpoint |
| 13130 | xiaobai_voice 语音代理 WS 测试 |
| 19601–19603 | kg-meta-core 集群节点测试 |
| 19876 | mox-dualrpc 测试 |
| 19999 | kg-connector 不可用降级测试 |
| 35432 | 本地 PostgreSQL 测试库（`MOX_TEST_PG_URL`） |
| 65528–65530 | alliance-sdk 客户端连通性测试（高位端口） |
| 8081 / 8082 | mox-dualrpc jsonrpc 测试端口（曾为 alliance 旧端口，已迁 3100/3200） |
| 18080 / 19080 / 19081 | single-node 验证模式 public/ctrl/data（t19） |
| 8787 | primiflow demo（`Server::serve 0.0.0.0:8787`） |
| 8788 | primiflow-fusion demo（fusion-server 默认） |
| 8333 | mox-cloud-filer-svc 挂载测试 |
| 3079 | mox-flow-bridge serve demo（`mox serve --port 3079`） |
| 3123 | alliance boot-config 测试 fixture |
| 8307 / 63001 | voice-operator netstat 解析测试样本 |

### 3.7 THIRD-PARTY —— 第三方基础设施默认端口（部署引用）

> 以下为部署文档 / Helm / docker-compose 中引用的中间件默认端口，**非本项目进程监听**，仅作避让与排障参考。

| 端口 | 中间件 | 端口 | 中间件 |
|---|---|---|---|
| 2379 / 2380 | etcd client / peer | 4317 | OpenTelemetry OTLP |
| 3306 | MySQL | 50051 | gRPC（见 3.3） |
| 4222 | NATS | 5236 | 达梦 DM |
| 5432 | PostgreSQL | 54321 | KingbaseES |
| 6379 | Redis（见 3.4） | 7687 / 7688 | Neo4j Bolt / mox-dr 部署映射（7688→7687） |
| 7480 | Ceph RGW | 8200 / 7000 | raft（mox-dr；helm 用 7000，历史文档 8200） |
| 8848 | Nacos | 9000 / 9001 | MinIO API / Console |
| 9090 | Prometheus | 9093 | Alertmanager |
| 6006 | Storybook（dev） | 33060 / 33306 | MySQL X / 映射端口（本机） |

---

## 第4章 归一化原则（强制约束）

1. **单一事实源**：运行服务端口一律以 `platform_config.json` 为准；启动脚本（`start.sh` / `scripts/deploy/start.ps1`）**禁止硬编码端口**，必须从 `platform_config.json` 读取（已完成）。
2. **核心服务归 3xxx**：专家联盟核心服务必须落在 `3000–3999`（PORT-NORM-001 1.1）；**唯一例外**是 Rust 网关 `api=8080`（见 PORT-NORM-001 注 2.1a），任何其他服务禁止占用 8080。
3. **一端口一服务**：同一端口全局唯一，禁止一端口多服务；DEPRECATED 端口禁止复用。
4. **禁止占用常见软件端口**：新增端口须对照第3章避让清单（PORT-NORM-001 第3章）与 `netstat -ano | findstr LISTENING` 实查。
5. **测试端口隔离**：测试/mock 端口（TEST-ONLY）不得与运行端口混用，不得出现在启动链路配置中。
6. **文档-代码对齐**：任何文档中出现的端口号必须与本文档一致（含 `:8081/:8082` 等历史遗留引用，一律按第3.5 节解释为过期）。

---

## 第5章 端口变更流程

1. **申请**：说明服务名、业务域、用途、协议。
2. **落段**：RUNTIME → 避开已占用与保留段，选空闲端口并同步 `platform_config.json`；ALLIANCE 核心服务 → 3000–3999；附属/插件 → 30000+（PORT-NORM-001 第4章）。
3. **避让校验**：对照第3章避让清单 + 本机实查占用。
4. **登记**：在本文档第3章登记（端口、服务、用途、状态）。
5. **同步**：同步更新 `platform_config.json`、`config/alliance-*.yml`、启动脚本、前端代理、docker/helm、部署文档、PORT-NORM-001（如涉 3xxx 段）。
6. **验证**：运行 `python scripts/verify-ports.py` 确认无漂移；重新编译 + 启动 + 健康检查。

---

## 第6章 一致性保障

### 6.1 自动校验

仓库内置 `scripts/verify-ports.py` 端口漂移校验脚本：

```bash
python scripts/verify-ports.py            # 全量校验，任何漂移/冲突返回非零退出码
python scripts/verify-ports.py --json     # 输出机器可读 JSON 报告
```

校验内容：
- 全仓库（源码/配置，排除第三方参考库、构建产物、node_modules）扫描端口引用；
- 对照本文档注册表分类（RUNTIME / ALLIANCE / ANCILLARY / LEGACY / DEPRECATED / TEST-ONLY）；
- 检出**未知端口**（未登记）、**DEPRECATED 端口仍被活跃配置引用**、**一端口多服务**等漂移。

### 6.2 本次归一化已修复的不一致（2026-09-01）

| 文件 | 修复前 | 修复后 |
|---|---|---|
| `start.sh` | `API_PORT=3010`（硬编码，与 api=8080 冲突） | 从 `platform_config.json` 读取（回退 8080） |
| `scripts/deploy/start.ps1` | 提示 `api:3010 / localhost:3010/health` | 从 `platform_config.json` 读取 api/frontend 端口 |
| `frontend-ui/vite.config.js` | `/ai/engine`、`/voice`、`/ws` 默认代理到 `:3001`（已退役端口，代理会 502） | 默认 `http://localhost:8080` |
| `docs/standards/expert-alliance-normalization-mode.md` | scheduler/executor 记 `:8081/:8082`（7 处，与代码 3100/3200 冲突） | 全部改为 `:3100/:3200` |
| `docs/standards/expert-alliance-port-norm.md` | Node api 记 `3010 已启用`（实为 Rust 网关 8080 例外） | 3010 标退役、新增 8080 例外登记（注 2.1a） |
| `shared/constants/index.js` | `SERVICE_PORTS`：gateway:3000 / nodeBackend:3010 / frontendUI:3021（旧 Node 架构残留） | gateway:8080 / nodeBackend:0(退役) / frontendUI:3020 |
| `shared/schemas/config.js` | local-engine LLM 模板 endpoint `http://localhost:3010/api/local` | `http://localhost:8080/api/local` |
| `projects/xiaobai_voice/.../default_config.yaml` | `ai_dialog_url: http://localhost:3021/#/ai` | `http://localhost:3020/#/ai` |
| `projects/xiaobai_voice/.../desktop/app.py` | fallback URL `:3021` | `:3020` |
| `projects/xiaobai_voice/.../desktop/ball_widget.py` | 2 处 `:3021` | `:3020` |
| `projects/xiaobai_voice/.../desktop/main_window.py` | 提示语 + 前端探活 `:3021` | `:3020` |
| `scripts/validation/verify_tts_rust_fullstack.py` | E-4 三层代理链 `:3021 -> :3001 -> :3717` | `:3020 -> :8080 -> :30010`（3717→30010 迁移后同步） |
| `projects/xiaobai_voice/**`（config/cli/service/proxy/desktop/README） | 端口 `3717`（15 处） | **`30010`**（PORT-NORM-001 4.2 落段 30000+） |
| `platform/domains/voice/svc/mox-voice-operator-svc` | module/feature `server_3717`/`server-3717`，绑定 `127.0.0.1:3717` | `voice_server`/`voice-server`，绑定 `127.0.0.1:30010`（文件同步改名 `voice_server.rs`） |
| `platform/domains/voice/svc/mox-voice-desktop-app` | `server_3717` + `:3717`（30 处） | `voice_server` + `:30010` |
| `platform/domains/platform/svc/mox-platform-orchestrator-svc`（voice_proxy/subservers） | voice 上游 `127.0.0.1:3717` | `127.0.0.1:30010` |
| `platform_config.json` / `scripts/server-manage.py` | xiaobai_voice `port: 3717` | `port: 30010` |
| orchestrator 侧车（main.rs / ai_engine.rs / sidecar/*） | 默认 `http://127.0.0.1:3010`（指向已删除 backend-node） | 默认 `http://127.0.0.1:8080`（接管其职责的 Rust 网关，注释标注） |

### 6.3 遗留待办（不影响一致性，属演进建议）

- **alliance 3100/3200/3300 纳入 `server-manage.py`**：当前由 `config/alliance-*.yml` 独立管理，尚未登记进 `platform_config.json`；如需统一面板启停，按第5章流程补充登记。

> 已完结（2026-09-01）：xiaobai_voice **3717 → 30010** 全链路迁移（Python 服务 / Rust `voice_server` / orchestrator voice 代理 / `platform_config.json` / 桌面端 / 校验脚本）已完成并通过 `verify-ports.py`；orchestrator Node 侧车默认 `127.0.0.1:3010` 已清理为 Rust 网关 8080。旧端口 3717 与 3010 现仅存于历史文档（.md/.html 报告），属 DEPRECATED 文档引用，予以保留。

---

*PORT-REGISTRY-001 V1.0 · 全局端口唯一权威 · 2026-09-01*
