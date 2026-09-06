# Examples · 快速开始与最小闭环

> 面向新开发者的**分层入门路径**：先用最小闭环跑通，再逐域深入。
> 完整架构见 [README](../README.md) 与 [docs/](../docs/README.md)。

## 0. 前置

- Rust 工具链（workspace 143 crates，`rust-toolchain.toml` 指定版本）
- 可选：Docker（`docker-compose up -d --build` 一键拉起 nginx / api-gateway / ollama / postgres / redis / prometheus / grafana）
- Windows PowerShell 下中文乱码属控制台显示问题，文件均为 UTF-8

## 1. 最小闭环（建议 5 分钟）

只依赖 3 个域 + 网关，验证「HTTP 入口 → 网关路由 → 服务 → 指标」全链路：

```powershell
# 1) 编译并启动网关（默认 0.0.0.0:8080，唯一对外入口）
cargo run -p mox-platform-gateway-svc

# 2) 健康检查
curl http://localhost:8080/health

# 3) 指标端点（Prometheus 文本格式，验证可观测性链路）
curl http://localhost:8080/metrics
#    期望出现 mox_gateway_requests_total / mox_gateway_request_duration_seconds
#    （访问 /health 数次后，requests_total 计数递增）

# 4) 管理面
curl http://localhost:8080/actuator/metrics   # JSON 概览
curl http://localhost:8080/actuator/mappings  # 全部路由注册表
```

启动前预检（CI 门禁同款）：

```powershell
python scripts/verify-ports.py   # 端口漂移校验，须 ERROR=0 WARN=0
./start.sh --dry-run             # 启动前预检
```

## 2. 分域深入路径

| 层 | 域 | 入口/示例 |
|---|---|---|
| 知识图谱 | `kg` | `GET /kg/v1/stats` 等；图算法在 `platform/domains/kg/core/`（pagerank / centrality / community / pathfinding） |
| AI 意图 | `ai` | `mox-ai-intent-svc`（默认监听 8765，`MOX_AI_INTENT_PORT` 可改） |
| 专家联盟 | `alliance` | 本地一键起：`scripts/start-alliance-local.ps1`（33080/33100/33200） |
| 前端 | — | `frontend-ui/`（dev 3020；专家联盟页面 dev 33020） |

## 3. 已有示例/探针

- `tests/test_stream_e2e_probe.py` — 流式 E2E 探针（31111/31112，测试端口）
- `scripts/verify-ports.py --json` — 端口漂移校验 JSON 报告
- `deploy/docs/trace-8stages-dashboard.json` — Grafana 面板（消费 `/metrics`）

## 4. 新手指南（分层路径）

1. **跑通**：完成第 1 节最小闭环。
2. **看路由**：`/actuator/mappings` 按层/域过滤，理解 31 域路由单元。
3. **改服务**：在 `platform/domains/<域>/svc/` 内新增 handler，网关自动代理（路由注册见 `platform/gateway/.../actuator.rs` RouteRegistry）。
4. **守规范**：新增端口必须先登记 `docs/api/PORT-REGISTRY.md` 并过 `verify-ports.py`。
