# PrimiFlow-Fusion · 企业级融合归一化一体化平台

> 把 **GR-STD 关图规范** 与 **PT-Primi 架构规范** 熔铸为**一张守恒统一图**，并提供
> 企业级 REST 服务、六维溯源注册表（R06）、PT-DOC 标准文档自生成（R08）与全局治理闸门
> （守恒 R07 / 六维零孤儿 A4 / GR-STD 8 闸门）。

本 crate 是 OUS（算子统一系统）的「归一化事实源」层：所有能力、数据、文档都从统一图派生，
保证 需求↔功能↔业务↔算法↔任务↔代码 全链路可溯源、可审计、可治理。

---

## 架构分层

| 层 | 模块 | 职责 |
|----|------|------|
| L1 需求语义 | `unified` | 归一化节点/边/原语坐标 (κ,τ,C,Q) |
| L2 六维映射 | `sixdim` | R06 六维绑定注册表（累积/查询/持久化） |
| L3 API 服务 | `server` | 企业级 REST（Bearer 鉴权 / CORS / 溯源查询 / PT-DOC） |
| L4 平台编排 | `platform` | 主链路八模块 + 统一图 + 全局闸门闭环 |
| L5 文档自生成 | `ptdoc` | R08 PT-DOC 标准文档集（10 份） |
| L6 能力融合 | `registry` | 13 crate 能力 + 6 数据表融合 |
| L7 治理 | `unified::full_gate` | 守恒残差 + 零孤儿 + GR-STD 8 闸门 |

---

## 构建与测试

```bash
# 构建（lib + 二进制）
cargo build -p primiflow-fusion

# 全部测试（单元 + 集成 + 基准）
cargo test -p primiflow-fusion --all-targets

# 性能基线（P4 · benches）
cargo bench -p primiflow-fusion --bench development_experts

# 代码质量门禁（CI 同款）
cargo fmt -p primiflow-fusion -- --check
cargo clippy -p primiflow-fusion --all-targets -- -D warnings
```

---

## 运行

二进制提供两个子命令：

```bash
# 启动 REST 服务（默认 0.0.0.0:8080）
primiflow-fusion serve [--config config.json] [--addr 0.0.0.0:9090]

# 仅跑全局治理闸门（供 CI 门禁；通过退出 0，否则 1）
primiflow-fusion verify
```

### 配置（12-factor）

配置优先级：**默认值 < 配置文件(JSON) < 环境变量**（`OUS_FUSION_*`）。

| 字段 | 环境变量 | 默认 | 说明 |
|------|----------|------|------|
| `bind_addr` | `OUS_FUSION_BIND_ADDR` | `0.0.0.0:8080` | HTTP 监听地址 |
| `persistence_path` | `OUS_FUSION_PERSISTENCE_PATH` | 无（仅内存） | 六维注册表 JSON 落盘路径（跨重启复用） |
| `docs_dir` | `OUS_FUSION_DOCS_DIR` | `data/fusion_docs` | PT-DOC 导出目录 |
| `log_level` | `OUS_FUSION_LOG_LEVEL` | `info` | trace/debug/info/warn/error |
| `auth_token` | `OUS_FUSION_AUTH_TOKEN` | 无（关闭鉴权） | Bearer 令牌（**仅建议环境变量注入**） |
| `access_log` | `OUS_FUSION_ACCESS_LOG` | `true` | 请求访问日志 |
| `json_log` | `OUS_FUSION_JSON_LOG` | `false` | 结构化(JSON)日志（对接 Loki/ELK） |

示例 `config.json`：
```json
{
  "bind_addr": "0.0.0.0:8080",
  "persistence_path": "/data/fusion_registry.json",
  "docs_dir": "/data/fusion_docs",
  "log_level": "info",
  "access_log": true,
  "json_log": false
}
```

> 生产环境务必设置 `OUS_FUSION_AUTH_TOKEN`，否则 `/api/v1/*` 端点将无鉴权暴露。

---

## REST API

除 `/api/health` 与 `/api/version` 外，所有 `/api/v1/*` 端点需 `Authorization: Bearer <token>`。

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/health` | 探活：注册表统计 + 全局闸门状态 |
| GET | `/api/version` | 服务版本 |
| POST | `/api/v1/synthesize` | 提交需求，跑一体化合成并导出 PT-DOC |
| GET | `/api/v1/registry/by-code?code=` | code→需求 溯源反查 |
| GET | `/api/v1/registry/by-requirement?req=` | 按需求 id 查询绑定 |
| GET | `/api/v1/registry/stats` | 注册表统计 |
| POST | `/api/v1/persist` | 落盘注册表（JSON） |
| GET | `/api/v1/gate` | 跑全局治理闸门 |
| GET | `/api/v1/docs` | 列出已导出 PT-DOC |
| GET | `/api/v1/docs/:id` | 读取某 PT-DOC 内容 |

`POST /api/v1/synthesize` 请求体：
```json
{ "requirement": "抓取销售数据生成月度经营分析报告", "slider_s": 0.5 }
```

---

## 容器化部署

```bash
# 构建镜像
docker build -f crates/primiflow-fusion/Dockerfile -t primiflow-fusion .

# 运行（持久化挂载到 /data）
docker run -d -p 8080:8080 \
  -e OUS_FUSION_AUTH_TOKEN=change-me \
  -v $(pwd)/data:/data \
  primiflow-fusion serve
```

---

## 治理与合规

- **R06 六维绑定注册表**：跨需求累积 `REQ→FUN→BIZ→ALG→TSK→COD`，支持按 code/需求/工程/实体反查溯源。
- **R07 守恒闸门**：统一图全局守恒残差 `|C² - (κ²+τ²)| < 1e-3`。
- **A4 六维零孤儿**：每个绑定六维实体齐全、Bind 边闭合，无悬空节点。
- **R08 PT-DOC 自生成**：合成后自动产出 10 份 PT-Primi 标准文档（六维溯源矩阵 / 守恒合规 / 零孤儿 / 关图治理 / 能力融合 / 注册表统计 / 拓扑涌现 / PT-Primi 合规 / κ 复用 / 术语表）。
- **GR-STD 8 闸门**：信息分类、命名、关联、演进、治理、可观测、安全、合规 八项校验。

`primiflow-fusion verify` 可在 CI 中以非零退出码阻断不合规的合并。
