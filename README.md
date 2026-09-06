# Infotopograph · MOX 平台

> 企业级动态 SQL 管理 + 自研知识图谱 + 字段级权限 + AI 驱动
> 当前版本: 3.0.0-ai-powered | 架构: 6 层企业级架构（Rust workspace）

---

## 产品定位

**MOX 是「Rust 原生 · 企业级 AI 基础设施平台」，不是通用 AI 应用 SDK。**

与 LangChain / CrewAI / LlamaIndex 等库模式框架不同，MOX 解决的是**服务化部署层**的问题：gRPC 契约 + 8080 统一网关入口、多智能体编排（专家联盟 scheduler/executor）、自研知识图谱引擎（kg 域）、Prometheus 指标端点（`/metrics`）、K8s/Helm 一键部署。其差异化定位为：

| 维度 | MOX | Python 库模式框架（LangChain 等） |
|---|---|---|
| 运行形态 | gRPC 服务 + 统一网关（多语言客户端） | 嵌入应用进程的 SDK |
| 多智能体 + 知识图谱 | 双原生（alliance 域 + kg 域） | 需集成第三方（Neo4j 等） |
| 性能 | Rust 级开销、无 GC、单二进制 | ~5-10ms 框架层开销 |
| 场景 | 企业级内部 AI 服务平台 | 快速搭建 AI 应用 |

技术栈为 Rust 时、需要知识图谱与多智能体深度集成、且面向生产服务化部署的团队，MOX 是当前唯一同时具备以上能力的自研方案。

---

## 架构速览

MOX 平台采用 **6 层分层架构**（`platform/domains/` 域驱动），核心为自研 Rust 高性能知识图谱引擎，支持知识图谱与 SQL 融合查询、字段级权限、AI 智能助手。完整架构说明见 [ARCHITECTURE.md](ARCHITECTURE.md)（v3.0.0-ai-powered）。

```
L6  Gateway      mox-platform-gateway-svc（8080 唯一对外 HTTP 入口）
L5  Services     mox-*-svc（kg / ai / flow / data / cloud / voice / platform…）
L4  SvcAPI       domains/*/proto（gRPC 契约）
L3  Core         mox-*-core（纯计算 · 无 IO）
L2  API          domains/*/api（REST DTO）
L1  Foundation   mox-platform-foundation / mox-cloud-foundation
```

## 目录结构

```
infotopograph/
├── platform/           # 后端主体（Rust workspace，143 crates）
│   ├── domains/        #   业务域（kg/ai/flow/data/cloud/voice/market/alliance/base…）
│   ├── foundation/     #   基础层（error/audit/paths/observability…）
│   ├── gateway/        #   网关（mox-platform-gateway-svc）
│   ├── legacy/         #   历史 Python 版服务归档（勿用）
│   └── shared/         #   跨模块共享 core
├── frontend-ui/        # 前端（Vite Vue3，3020 dev）
├── projects/           # 独立子项目（melody2score、xiaobai_voice 等）
├── proto/              # protobuf 契约定义
├── deploy/             # 部署配置（Docker / Nginx / Systemd / Helm）
├── config/             # 运行时配置
├── scripts/            # 统一运维脚本（verify-ports.py 等）
├── tools/              # 开发/运维工具
├── docs/               # 文档中心（权威治理，见 docs/README.md）
├── reports/            # 报告归档（html/ + markdown/ + data/，共享 _shared/）
├── prototypes/         # HTML 原型（共享 _shared/）
├── observability/      # Prometheus / Grafana 配置
├── nginx/              # Nginx 配置
├── ais/ third_party/   # 第三方参考代码（不入库）
├── docker-compose.yml  # 编排：nginx / api-gateway / llm-inference-svc / ollama / postgres / redis / prometheus / grafana
└── start.sh            # 一键启动（--with-services / --build-rust / --verify）
```

## 快速开始

```bash
# Docker 一体化部署（推荐）
docker-compose up -d --build

# 或本地启动（POSIX / WSL）
./start.sh --with-services --build-rust

# Rust workspace 构建与测试
cargo build                # 默认 members
cargo test                 # 单元测试
cargo clippy --all-targets # lint（CI 门禁）
```

## 端口约定

| 服务 | 端口 |
|------|------|
| api（Rust 网关） | 8080 |
| frontend（Vite dev） | 3020 |
| gRPC（专家联盟内部） | 50051 |
| redis / postgres / ollama / prometheus / grafana | 6379 / 5432 / 11434 / 9090 / 3000 |

完整端口注册表见 `docs/api/PORT-REGISTRY.md`（端口漂移由 `scripts/verify-ports.py` 门禁校验）。

## 文档索引

- 文档中心（权威入口）: [docs/README.md](docs/README.md)
- 架构规范: [ARCHITECTURE.md](ARCHITECTURE.md) · [docs/architecture/](docs/architecture/)
- 部署指南: [DEPLOYMENT-GUIDE.md](DEPLOYMENT-GUIDE.md)
- 报告归档: [reports/README.md](reports/README.md)

## 仓库治理规范（企业级）

- **报告**一律归档至 `reports/`（html/ markdown/ data/），共享字体/JS 统一放 `reports/_shared/`，禁止根目录散落报告。
- **文档**按分类放 `docs/`（architecture / enterprise / modules / working-reports / _archive）。
- **运行时数据**（target/ .logs/ .runtime/ data/ workspace/）已由 `.gitignore` 管理，不得提交。
- **提交规范**: conventional commits（feat/fix/docs/chore/refactor + scope）。

## 技术栈

Rust（axum / tonic / sqlx）· Vue3（Vite）· PostgreSQL · Redis · Docker · Nginx · Prometheus/Grafana · Python（子项目与工具链）

## License

[MIT OR Apache-2.0](LICENSE)
