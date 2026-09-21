# 专家联盟端到端链路演示（alliance_demo.py）

面向「2026 移动云杯 AI Coding 赛 · 智算方向」的可复现部署验证脚本。
把"联盟管线跑通了"从口头描述变成**任何人一条命令即可复现、且带机器可读证据**的演示链路。

## 它做什么

对网关 `:3080` 的 `/api/alliance/*` 真实调用，遍历多种 `AllianceMode`（协作模式），完整走一遍：

```
创建任务 → 读取 DAG → 列出节点 → 状态流转(resume) →
人工干预(skip 未终态节点) → 执行状态 → 融合结果 → 日志 → 任务问答 → toggle-done
```

最后再跑一次仓库治理门禁（`tools/mox-governance-mcp`），证明演示过程没有破坏治理约束。

产物（写入 `reports/`，符合仓库治理）：

- `reports/data/<时间戳>-alliance-demo.json` —— 机器可读证据
- `reports/markdown/专家联盟端到端演示报告.md` —— 人读报告（覆盖式，保留最新版）

## 用法

```bash
# 走默认 6 种模式（sequential/parallel/debate/hierarchical/iterative/voting）
python tools/alliance-demo/alliance_demo.py

# 只演示部分模式
python tools/alliance-demo/alliance_demo.py --modes parallel,debate

# 跳过演示后的治理体检（默认会跑；约需 2~3 分钟）
python tools/alliance-demo/alliance_demo.py --no-gate

# 不依赖 Rust 服务，用内置 HTTP 桩验证脚本逻辑闭环
python tools/alliance-demo/alliance_demo.py --selftest
```

常用参数：

| 参数 | 默认 | 说明 |
|---|---|---|
| `--base-url` | `http://127.0.0.1:3080` | 网关地址 |
| `--token` | `dev-secret-token` | 网关 dev 后门令牌 |
| `--modes` | 6 种全跑 | 逗号分隔的模式子集 |
| `--fusion` | 按模式自动匹配 | 统一指定融合策略 |
| `--node-wait-seconds` | 30 | 等待真实执行器推进节点到终态的秒数，超时后用人工干预 API 收尾 |
| `--no-gate` | 关 | 跳过演示后治理体检 |
| `--selftest` | 关 | 内置桩服务自检 |

## 前置

四进程需先启动（`scripts/start-mox-enterprise.ps1`，编排器 3001 / 联盟调度 3100 / 执行 3200 / 模块化网关 3080）。
脚本会等待健康探针（`/api/v1/status`）就绪，超时则退出码 2 并给出启动提示。

## 证据价值（对 AI Coding 赛）

- **真实场景部署验证**：直连运行中的四进程，125 次真实 HTTP 调用，全链路无 mock。
- **模式驱动编排**：6 种 `AllianceMode` 生成差异化 DAG（节点/边数不同），可作为"智算编排"能力佐证。
- **协议归一化可见**：报告同时呈现 `fusion_strategy` 的 serde 名与网关展示串（如 `weighted`→`weighted_voting`），证明契约层设计。
- **诚实边界**：本地模式无自动执行引擎，未终态节点经人工干预 API 推到 `skipped`，故融合状态为 `partial`、置信度 0.75——脚本如实记录，不粉饰。

## 依赖

仅 Python 3.8+ 标准库，无第三方依赖（与 `tools/mox-governance-mcp/server.py` 风格一致）。
