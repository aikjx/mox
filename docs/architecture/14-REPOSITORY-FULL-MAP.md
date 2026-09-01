# 仓库全维全景图 · 模块化分层导航（最详细版）

> **文档编号**: 14-REPOSITORY-FULL-MAP
> **适用范围**: 仓库根目录全部内容（含隐藏目录）
> **依据**: 实测目录结构 + 根 `Cargo.toml` / `README.md` / `CLAUDE.md` / `ARCHITECTURE.md` + `docs/enterprise/00-INDEX.md` + 各子项目 README/Cargo/package.json
> **状态**: 权威 | 最后更新: 2026-08-29

---

## 0. 仓库身份

- **产品/项目名**：MOX 全维低代码平台（对外商业名：**璇玑（Aura）软件研发数字孪生中台**；对内研发代号：**关图 / 璇玑 RelGraph 算子统一全维治理系统**）
- **一句话定位**：企业级动态 SQL 管理 + 自研知识图谱 + 字段级权限 + AI 驱动 + 全维自动化治理
- **核心铁律**：四归三连（需求→架构→业务→文档四归一；联盟/流程/代码三连）、全维双收口、三联盟协同闭环（产品联盟收需求 / 算法联盟落算法 / 开发联盟交付代码）

### 0.1 技术栈总览

| 侧 | 技术栈 | 说明 |
|----|--------|------|
| 后端 | Rust edition 2021 · Axum + Tokio · Serde · thiserror/anyhow | 73 crate workspace · 6层8域DDD矩阵 · 模块化单体 |
| 中台 | Python · FastAPI + uvicorn · SQLite | mox-server（发布中台 8600）+ mox-store（应用商店 8601） |
| 前端 | Vue 3.4 · Vite 5 · Element Plus 2.4 · vue-router 4.3 · Axios 1.6 · ECharts 5.4 · three/3d-force-graph · mermaid · VexFlow | 用户端 + /admin 系统管理区 |
| 部署 | Docker · Helm · systemd · nginx · Istio | 边缘入口 Node :3000 → Rust 网关 :3001 |
| AI 生态 | ais/ 下 24 个第三方开源项目克隆 | 供参考/评估/复用 |

### 0.2 运行时架构（三层入口）

```
浏览器 ──→ frontend-ui (Vue/Vite, /api 代理)
                │
                ▼
        backend-node 边缘入口 (:3000, 零依赖 Node, 反向代理 /api)
                │
                ▼
        mox-platform-gateway-svc (:3001, Rust 聚合网关, 31域路由)
                ├── 8 域 api → 各域 svc → core
                ├── Python mox-server (:8600) / mox-store (:8601)
                └── 治理 8 闸门 + ⛨验证网关（出码/发布必经）
```

---

## 1. 目录分层总览

按「功能角色」将全仓库 60+ 顶层项划分为 **8 层**：

| 层 | 角色 | 目录 |
|----|------|------|
| **L1 核心平台** | 可运行的系统本体 | `platform/` · `frontend-ui/` · `shared/` · `proto/` · `config/` · `deploy/` · `scripts/` · `tools/` · `plugins/` · `tests/` · `data/` |
| **L2 独立项目** | 并列的独立应用/研究项目 | `projects/`（18 个子项目） |
| **L3 AI 生态** | 第三方开源克隆 | `ais/` · `third_party/` |
| **L4 文档体系** | 权威文档与治理中心 | `docs/` |
| **L5 可视化应用** | 独立 HTML 单页应用 | 7 个顶层 HTML 项目 + `data-vis/` |
| **L6 构建产物** | 编译/打包/运行产物 | `build/` · `dist/` · `release-pkg/` · `target/` · `outputs/` · `exports/` · `artifacts/` · `workspace/` · `temp/` |
| **L7 日志与缓存** | 运行期日志/缓存 | `log/` · `.logs/` · `.runtime/` · `__pycache__/` · `.pytest_cache/` |
| **L8 内部工具链** | AI 协作/记忆/规格 | `.trae/` · `.trae-html-share-packages/` · `.ous/` · `.ous_smoke/` · `.workbuddy/` · `my_projects/` · 根配置与隐藏文件 |

---

## 2. L1 核心平台

### 2.1 `platform/` — 后端工程主体（Rust 微服务 + Python 中台）

> 已单独成册：`docs/architecture/13-PLATFORM-CODEBASE-GUIDE.md`（含 9 域 crate 全表、foundation/framework/gateway 详表）。

**结构**：`arch-test`（架构合规检查官）· `backend-rust`（早期独立 Rust 后端，含 aiops/zero_trust/data_quality）· `crates/bindings`（预留 FFI 绑定，空）· `domains/`（9 域五层：ai/cloud/data/flow/kg/market/platform/project/voice，每域 api/core/sdk/svc/svcapi）· `foundation/`（5 底座 crate）· `framework/`（mox-framework 横切框架）· `gateway/mox-platform-gateway-svc`（L1 网关 8080）· `mox-server/`（Python 发布中台 8600）· `mox-store/`（Python 应用商店 8601）· `scripts/`（空）· `shared/`（config/constants/schemas）

**8 域 DDD 矩阵核心**（来自 CLAUDE.md 权威）：
- **core（L2）**：ai-core（LLM 抽象）/ ai-intent-core（意图识别）/ kg-algo-core（八大算法 A1~A8：CNM·Brandes·Harmonic·PageRank·激活扩散·RRF·CEM·CPM）/ kg-meta-core（14 节点族×19 边族）/ flow-operator-core（算子代数·守恒律·范畴论）/ flow-optimizer-core（CPM·RCPSP·CEM）/ data-formula-core（高精度公式引擎）/ data-norm-core（归一化 IR）/ data-standards-core（数据标准）/ platform-system-core（成员·任务·权限·RBAC·EventBus）/ platform-iam-core（身份·令牌·访问控制）/ platform-meta-core（AisLayer·CrateMeta·all_crate_metas）/ platform-datastore-core（多后端 SQLite/PG/MySQL 方言归一化）/ platform-orchestrator-core（DAG 编排·事件反应器·鉴权闸门）/ voice-dsp-core（响度·软限幅·Aho-Corasick 热词·SIMD）
- **svc（L3）**：ai-agent-svc（对话·浏览器自动化·MultiAgent·ProviderRegistry）/ ai-expert-svc（⛨璇玑 14 专家·归一化 IR·裁决·验证 5 项·审计三汇·RBAC·租户分层）/ ai-flow-svc（流程 AI 9 模块·代码生成）/ kg-storage/service/streams/spark/hub/fusion-svc（混合索引+URN+8 段 5 连接器·RRF 融合）/ flow-operator-wasm-svc（WASM 沙箱·wasmer·热加载）/ flow-primiflow-svc（解析·代码生成·8 类骨架）/ flow-fusion-svc（六维融合·守恒闸门·Registry）/ flow-bridge-svc（Hermes 桥接）/ data-plane/etl/compliance/catalog-svc（PII 检测·脱敏·6 预置 FlowGraph）/ platform-enterprise-svc / platform-orchestrator-svc / cloud-master/volume/s3/filer-svc / market-template-svc（发布·评分·Fork·2 种子）/ voice-core/asr/intent/operator/desktop-app-svc（**voice-desktop-app = 独立产品形态·全局热键·BallWidget·键鼠自动化**）
- **sdk（L4）**：kg-sdk / cloud-sdk / platform-test-harness / data-formula-native（napi-rs Node FFI）/ data-norm-intent-native（napi-rs）/ voice-dsp-py（PyO3 abi3-py39）
- **api（L5）**：8 域 api/svcapi 已建目录，Phase 3 填充（规划中）

### 2.2 `frontend-ui/` — 前端工程

**package.json 定位**：`operator-unified-system-frontend` v1.0.0 · Vue3+Vite5+ElementPlus · pnpm 11.15 · vitest+storybook+playwright+lighthouse 测试栈。

**子应用/目录**：
- `src/`（主源码）：`api`（统一 fetcher，Axios 实例+Bearer 注入+异常拦截，**组件禁止直接 fetch**）· `views`（业务页面，按域组织）· `components`（通用 UI，无业务逻辑）· `router`（vue-router 路由表）· `styles`（设计 token，禁止硬编码颜色）· `utils`（工具，含 hitl-ws.js）· `composables` · `themes` · `admin`（系统管理区 5 面板：AdminOverview/Access/Audit/Storage/Hitl）
- `mox-website/`：企业官网（低代码能力落地页，已对接真实后端）
- `mox-console/`：SQL 定义管理 / 字段权限 / 数据源 / 图谱 / 缓存审计
- `mox-store/`：应用商店前端
- `chip-website/`：芯片行业网站
- 测试与 CI：`tests/` · `playwright-report/` · `test-results/` · `.storybook/` · `PORTAL_README.md`（企业门户外壳说明：PortalHome/Login/Workbench/BusinessHall）

**管理区能力**：安全状态 / API Key 凭证 / 审计日志 / 存储提供方 / HITL 人机协同审批（WebSocket `/ws/hitl`）。

### 2.3 `shared/` — 跨模块共享

`config/` · `constants/`（index.js）· `schemas/`（config.js）——前端跨应用共享的常量与配置 schema。

### 2.4 `proto/` — protobuf 契约定义

`expert-alliance/v1/`：`alliance_executor.proto` · `alliance_fusion.proto` · `alliance_scheduler.proto` · `common.proto` · `expert_agent.proto` · `expert_memory.proto` · `expert_registry.proto`（专家联盟系统 gRPC 契约 7 个文件）。

### 2.5 `config/` — 运行时配置

`gateway.yaml`（网关配置）· `meta_latest.json` · `paths.env.example` · `.guantu_baseline.json`（关图基线）。

### 2.6 `deploy/` — 部署体系

- `Dockerfile` · `nginx.conf` · `systemd.service` · `docker-compose.yml`（根）
- `helm/mox` + `helm/mox-dr`（Chart + values + templates，主备双部署）
- `sql/mox-step1-graph-edges.sql`（图谱边初始化）
- `docs/`（运维手册 ops-manual.md、FS-S3 全生命周期、HA 容量 TCO、ADR 记录、NodeToRust 迁移交接、全栈自动交付计划、信创矩阵）

### 2.7 `scripts/` — 统一运维脚本

**主入口** `server-manage.py`（服务生命周期 + Web 面板 + 公理验证；`manage.py` 为兼容别名）。子目录：`ci/`（CI 脚本·git 打 tag）· `deploy/`（Windows 一键启动 start.ps1、smoke_test.sh、KG 存储部署、灰度预热）· `tests/`（T10/T11/T17/T19/T1 系列验收、企业级 7 闸门、8 大规范测试执行）· `validation/`（单节点校验等）。

### 2.8 `tools/` — 开发/运维工具脚本

- `info-graph/`：**关图 CLI**（Rust，Cargo 工程，加载领域子图）
- 顶层 Python 工具：`deploy.py`（本地一体化部署）· `package.py`（打包发布）· `export_data.py` / `import_data.py` / `validate_export.py`（数据导出导入校验）· `install_app.py` / `publish_app.py`（应用商店运维）· `arch_test.py` / `architecture_audit.py` / `architecture_constraint_test.py`（架构合规）· `migrate_architecture.py` / `migrate_to_domain_first.py` / `update_cargo_paths.py` / `fix_path_deps.py`（架构迁移）· `build_chip_kg.py` / `init_chip_website.py` / `fix_chip_mox.py` / `verify_chip_full.py` / `verify_chip_website.py`（芯片站）· `inspect_mox.py` / `check_mox_api.py` / `guantu_gate.py`（巡检/关图闸门）

### 2.9 `plugins/` — 插件目录

`extensions/` · `scripts/` · `wasm/`——均为空（仅 `.gitkeep`），WASM 插件预留位。

### 2.10 `tests/` — 仓库级测试

`governance_api.rs`（治理 API 测试，Rust）。

### 2.11 `data/` — 运行时数据

`cache/` · `exports/` · `logs/` · `storage/` · `uploads/`（均 gitkeep）+ `graph.json` / `graph.enterprise.json`（图谱数据，企业版与全量版）。

---

## 3. L2 独立项目（projects/，18 项）

| 项目 | 类型 | 说明 |
|------|------|------|
| `melody2score/` | Rust+Python 桌面应用 | **哼唱旋律转歌谱**端到端应用：录音/音频→提取旋律→生成简谱/musicxml。PC 原型 + 开发板移植（`board/`）+ 信息图谱融合（`graph/`，领域子图 D13，可被 tools/info-graph 加载）。目录：app/audio/board/build/core/dist/docs/graph/lib/results/tests/video |
| `mox-dualrpc/` | Rust 框架 | **企业级双协议 RPC 框架**：gRPC + JSON-RPC 零配置自动转码。含 mox-dualrpc-macro（过程宏）、examples、ARCHITECTURE_AUDIT.md、EXPERT_ALLIANCE_INTEGRATION.md |
| `primiflow/` | Python+Web MVP | **客户语音/文字→自动拓扑→拖拽编辑流程图→出 8 份说明书**主链路闭环。backend（uvicorn :8000）+ web。规则化拓扑生成器代替 LLM（离线可跑），生产按 SPEC.md 拆 Go+Python |
| `xiaobai_voice/` | Python 语音应用 | **璇玑离线语音 & 桌面小白 AI 助手**。ASR=Paraformer-zh+sherpa-onnx（离线 CPU）；TTS=Fish-Speech-S2-Pro（默认）→CosyVoice2（信创回退）→浏览器 SpeechSynthesis（兜底）。语音服务 :3717。含 xiaobai_core / xiaobai_voice / models / reports / build |
| `market-games/` | Python 小游戏 | 贪吃蛇等（src/snake.py）+ t5 游戏产物 |
| `mox-official-site/` | 官网 | `docs/需求.md`（官网需求文档） |
| `t10-cloud-artifacts/` | 验收产物 | T10 云盘 M4：冷热分层迁移 / IAM 10×10 判定矩阵 / STS TTL=900 校验 |
| `t11-graph-artifacts/` | 验收产物 | T11 关系图 R4：CDC 10 万事件 / projection 20 算子 oracle 哈希 / AC-15 14 故障注入 |
| `t17-ef-runs/` | 验收产物 | EF 运行批次（20260824-xxxxxx 多次运行 + latest） |
| `t17-sdk-examples/` | 验收产物 | 官方 SDK 示例输出（Rust/Node/Python ≥15 JSON） |
| `t19-regression/` | 验收产物 | 全量回归：build-mox-server.log / 单节点校验报告 |
| `t19-regression-report/` | 验收产物 | 回归汇总 last_summary.json（rust/mocha/vitest 通过数） |
| `t20-canary-metrics/` | 验收产物 | Helm 灰度 4 阶段 metrics（warmup 100×healthz + 10×metrics） |
| `t22-simd-artifacts/` | 验收产物 | SIMD 产物 runs/ |
| `t23-projection-artifacts/` | 验收产物 | 投影产物 runs/ |
| `t24-gm-artifacts/` | 验收产物 | GM 产物 runs/ |
| `t25-glacier-artifacts/` | 验收产物 | Glacier 产物 runs/ |
| `vendor-eval/` | 评估 | 供应商评估（当前为空） |

---

## 4. L3 AI 生态与第三方

### 4.1 `ais/` — 第三方开源 AI 项目克隆（24 项，均带 .git）

**分类**：
- **AI 编程 Agent**：aider / claude-code / claude-code-rust / claw-code / cline / openai-codex / opencode / openhands / gemini-cli / pi / deepseek-harness / hermes-agent / superpowers / system-prompts-and-models-of-ai-tools
- **LLM 应用框架**：langchain / dify / awesome-llm-apps / browser-use
- **存储与云**：ceph / juicefs / minio / seaweedfs / RustFS / nebula（图数据库）/ Cloudreve（网盘）

**根级文档**：`AI_AUTOMATION_BEST_PRACTICES.md`（AI 自动化最佳实践）· `AI_TOOLS_COMPARISON.md`（AI 工具对比）· `技术评估报告.md` · `git_pull_all.py`（批量拉取脚本）· `start_deepseek_harness.py`

### 4.2 `third_party/CosyVoice/` — 第三方语音

阿里 CosyVoice 语音合成项目克隆（含 cosyvoice / runtime / examples / docker / asset）。

---

## 5. L4 文档体系（docs/，权威治理中心）

> 唯一权威入口：`docs/README.md`（MOX 文档中心）· 最高级权威：`docs/enterprise/18-全域顶层总设计-三联盟模式-V1.0.md`（TOP-MASTER）

### 5.1 子目录（14 个）

| 目录 | 文件数 | 内容 |
|------|-------|------|
| `enterprise/` | 49 | **企业级文档治理中心**：28 份编号文档 00~27（TOP-MASTER 顶层设计 / Aura 对外 SRS / 架构 / 设计 / 业务 / 路线图 / 需求-架构映射 / 全维自动化 / 归一化总控卡 / 竞品对比 / 测试评测主控 等）+ 配套 |
| `architecture/` | 15 | 架构文档编号系列：01-overview ~ 12 专题 + 归一化架构 + 13/14（本次新增的 platform 指南与仓库全景） |
| `modules/` | 22 | 模块级文档 |
| `microservices/` | 8 | 微服务文档 |
| `rust-enterprise/` | 9 | Rust 企业级实践 |
| `standards/` | 8 | 标准规范 |
| `full-dimensional/` | 5 | 全维分析 |
| `cosmic-architecture/` | 6 | 宇宙级架构 |
| `ai-architecture/` | 2 | AI 架构 |
| `expert-alliance/` | 3 | 专家联盟 |
| `graph/` | 2 | 图谱 |
| `specs/` | 3 | 规格 |
| `enterprise-architecture/` | 1 | 企业架构分析 |
| `_archive/` | 0 | 归档（空） |

### 5.2 根级文档（docs/*.md / .html / .mmd）

- **权威**：`architecture.md`（统一架构规范 v3.0-ai-powered）· `MOX-AI驱动全维平台-企业级设计-全维分析-v3.0.md` · `operations-manual.md`（操作手册 v2.0）· `GLOSSARY.md`（术语表）
- **专项**：`data-exchange-spec.md`（MXDEF v1.0）· `deployment-guide.md`（部署指南 v1.0）· `app-store-architecture.md`（应用商店架构）· `AI-UNIFIED-OPTIMIZATION-PLAN.md` · `ARCHITECTURE_SAAS_PRIVATE.md` · `enterprise-architecture-analysis.md`
- **可视化**：`mox-architecture.html` · `mox-system-business-architecture.html` · `璇玑-全维需求业务处理流程图-归一化企业级.html/.md` · `璇玑-全维流水线.mmd` · `对话开发系统-端到端流水线.mmd` · `对话开发系统-全维分析与业务流程图.md`
- **本次新增**：`ai-development-concepts.md`（AI 工具概念区分）+ `architecture/13/14`（platform 指南 + 本全景图）

---

## 6. L5 可视化应用（独立 HTML 单页项目）

| 目录 | 产物 | 说明 |
|------|------|------|
| `chat-project-generator/` | chat-project-generator.html | **对话驱动项目生成器**：AI 对话→自动编排→项目落地（含 flow.yaml + autoOrchestrator.js / flowEngine.js + examples.js） |
| `directory-audit-report/` | directory-audit-report.html | MOX 项目目录结构审计与整理方案（开发专家联盟） |
| `expert-alliance-cyber/` | expert-alliance-cyber.html | 专家联盟系统 CYBERPUNK 版 |
| `expert-alliance-design/` | expert-alliance-design.html | 专家联盟平台全维度设计方案（Element Plus 规范） |
| `kg-workflow-guide/` | kg-workflow-guide.html | 知识图谱数据处理工作流（采集到入库全链路，含 kg-pipeline-flow.yaml） |
| `mox-enterprise-optimization/` | mox-enterprise-optimization.html | 璇玑（MOX）企业级全维分析与优化设计报告 |
| `xuanji-ux-redesign/` | xuanji-ux-redesign.html | 璇玑系统 UX 重设计规划（信息架构与布局） |
| `data-vis/` | 全维分析流程.html + flow_data.json | 全维分析需求业务处理流程图可视化 |

> 每个 HTML 项目均含 `assets/`（静态资源）与 `_shared/`（共享资源）；`.trae-html-share-packages/` 存放其分享打包 zip。

---

## 7. L6 构建产物

| 目录 | 内容 |
|------|------|
| `build/` | `build_exe/`（可执行构建目录） |
| `dist/` | `Melody2Score/`（可执行 + _internal 依赖集）+ `mox-platform-1.0.0-20260828-103241.zip`（平台发布包） |
| `release-pkg/` | `xiaobai-desktop/`：Xiaobai.exe + sherpa-onnx DLL 族 + onnxruntime + 语音模型（asr-paraformer-streaming / tts-kokoro） |
| `target/` | cargo 构建产物（debug/release/doc/aarch64）+ 各类 clippy/测试日志 + sherpa-onnx-prebuilt |
| `outputs/` | d6-final-check-* 系列（12+ 批次，最终检查输出） |
| `exports/` | `mox-export-all-20260828-103141.json`（全量导出）+ `test-crm-1.0.0.mxap`（应用包） |
| `artifacts/` | TTS 验证音频（e3_direct_cosyvoice2.wav / e4a/e4b） |
| `workspace/artifacts/` | site-corp-site（企业官网静态站：index/about/contact/services + site.json） |
| `temp/` | tts_verify（临时验证） |

---

## 8. L7 日志与缓存

| 目录 | 内容 |
|------|------|
| `log/` | clippy/ · compliance/ · graph/ · test/（分类日志）+ README |
| `.logs/` | 运行期日志约 40 个：admin/api/fe/gateway-new/melody2score/primiflow/start_api/test_all/xiaobai_voice/xv 等 |
| `.runtime/` | 进程 pid（api/frontend/melody2score）+ 构建日志（my_*.log 系列）+ llm_gateway_full.js |
| `__pycache__/` | Python 缓存（platform_manager/service_manager/service_monitor） |
| `.pytest_cache/` | pytest 缓存 |

---

## 9. L8 内部工具链与根配置

### 9.1 AI 协作与记忆

| 目录 | 说明 |
|------|------|
| `.trae/` | Trae 工作区：`specs/`（20+ 规格文档，按日期 20260823~20260826：企业级验收/架构/云存储/图谱/迁移/验收批次/专家联盟/语音集成等）+ `documents/` |
| `.trae-html-share-packages/` | 各 HTML 项目分享打包 zip + 接口 500 分析报告 |
| `.ous/` | 算子统一系统：automation 工作流 JSON |
| `.ous_smoke/` | automation/ + market/ 冒烟 |
| `.workbuddy/` | memory/（MEMORY.md + 2026-08-16~18 记忆文件） |
| `my_projects/` | `business-court-docs/`：商务法庭文档库（Rust crate，src/lib.rs + examples/run_opt.rs + tests/integration.rs） |
| `.github/` | workflows/（CI） |
| `.git/` | git 元数据（filter-repo 过滤历史） |

### 9.2 根配置文件

| 文件 | 说明 |
|------|------|
| `Cargo.toml` / `Cargo.lock` | **根 workspace（73 crate）**：default-members 只构建核心；含 deny.toml（依赖许可）、tarpaulin.toml（覆盖率） |
| `README.md` | 快速开始（启动后端→开前端→docker 部署→数据导入导出） |
| `CLAUDE.md` | **AI 编码上下文规范**：6层8域 DDD 矩阵 · 分层约束 · 编码规范 · 禁止清单（Do NOT）· 测试规范 · 常用命令 |
| `ARCHITECTURE.md` | 架构文档 v3.0.0-ai-powered：6 层架构 / 模块清单 / 命名规范 / 零改动扩展 / 错误码 / 配置参考 |
| `docker-compose.yml` | Docker 一体化部署 |
| `start.sh` | 启动脚本 |
| `platform_config.json` | 平台配置 |
| `接口500错误全维分析报告.html` | 接口故障分析报告 |

---

## 10. 关键入口速查

| 目的 | 入口 |
|------|------|
| 启动后端 | `cd platform/mox-server && pip install -r requirements.txt && python run.py 8600` |
| 启动 Rust 聚合网关 | `cargo run -p mox-platform-gateway-svc`（:3001） |
| 启动前端 | `cd frontend-ui && npm run dev`（/api 代理 :3000） |
| 全量构建 | `cargo build --workspace`（沙箱须后台运行） |
| 静态检查 | `cargo clippy --workspace --all-targets`（目标零告警） |
| 全量测试 | `cargo test --workspace` |
| 架构一致性 | `cargo metadata | jq '.packages | length'`（应为 73） |
| 一键部署 | `docker-compose up -d --build` 或 `python tools/deploy.py local --start` |
| 打包发布 | `python tools/package.py --version 1.0.0 --with-data` |
| 运维面板 | `python scripts/server-manage.py` |
| 文档导航 | `docs/README.md` → `docs/enterprise/00-INDEX.md` → `18 TOP-MASTER` |

---

## 11. 权威信息源与依赖关系

- **架构设计**：`ARCHITECTURE.md`（v3.0.0-ai-powered）↔ `docs/architecture/` 编号系列
- **工程规范（AI 编码约束）**：`CLAUDE.md`
- **文档治理**：`docs/enterprise/00-INDEX.md`（28 份文档 00~27，L0~L4 四级权威）
- **Crate 真源**：`mox-platform-foundation`（all_crate_metas 硬编码 16 条真源）
- **Workspace 权威**：根 `Cargo.toml`（members 全量清单，本文档第 2/3 节所列表格均来自此处实测）

---

## 12. 文档维护

- 本图覆盖仓库全部顶层目录；新增/删除目录时同步更新第 1 节分层表与对应详表。
- `platform/` 深度细节以 `13-PLATFORM-CODEBASE-GUIDE.md` 为准；本图负责全局导航。
- 规格/验收批次（t10~t25）为临时产物目录，归档后可移入 `docs/_archive/` 或移除。

**归一化状态**: ✅ 已归一化
