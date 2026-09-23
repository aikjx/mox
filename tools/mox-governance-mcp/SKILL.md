---
name: mox-governance
version: 1.0.0
description: 璇玑（infotopograph）仓库治理门禁。当需要新增或改动服务端口、编写 docker-compose/nginx/健康检查/CORS 配置、移动或重命名 docs 下文件、或在提交与发布前做体检时使用；能力为查询权威端口注册表、执行端口漂移校验、执行文档链接校验、一次跑通两项门禁的组合体检。
argument-hint: "[端口号 | 服务关键字 | '体检']"
allowed-tools: mox_port_lookup, mox_port_verify, mox_doc_links_check, mox_ci_gate
---

# 璇玑仓库治理 Skill

本 skill 把仓库既有的两项 CI 门禁（唯一事实源）接到 AI 编码流程里：
`scripts/gate/verify-ports.py`（PORT-REGISTRY-001）与 `scripts/gate/check-doc-links.py`（DOC-GOV-ARC-V1.0 §7）。

## 何时必须调用

| 场景 | 调用 |
|---|---|
| 即将写一个端口（新服务、compose 映射、nginx 反代、CORS 白名单、健康检查） | `mox_port_lookup` 先取权威端口，**禁止凭记忆臆造端口** |
| 改动了端口相关代码/配置 | `mox_port_verify` |
| 移动、删除、重命名了 `docs/` 下文件或目录 | `mox_doc_links_check` |
| 提交 PR / 打 release 之前 | `mox_ci_gate` |

## 标准工作流

1. **先查后写**：任何端口字面量进入代码前，先 `mox_port_lookup`。若返回 `found=false`，说明未登记——
   须先在 `docs/api/PORT-REGISTRY.md` 与 `scripts/gate/verify-ports.py` 的 `CANONICAL` 中登记后再使用。
2. **改即校验**：改动落地后立即 `mox_port_verify`（关注 `error_count` 是否为 0）。
3. **文档迁移联动**：目录迁移后跑 `mox_doc_links_check`，并按仓库约定在 `docs/ARCHITECTURE-OF-DOCS.md` 登记映射。
4. **收口体检**：`mox_ci_gate` 返回 `ok=true` 才允许提交。

## 结果判读

- `mox_port_verify.passed == false`：
  - ERROR「已退役端口 … 仍被活跃文件引用」→ 删除/替换该端口引用，DEPRECATED 端口禁止复用；
  - ERROR「platform_config.json: services.X.port」→ 以注册表为准修正配置，切勿反过来改注册表去迁就配置；
  - WARN「发现未登记端口」→ 确认是新服务则登记，是笔误则修正。
- `mox_doc_links_check.broken_count > 0`：按 `broken` 里的 `file:line -> target` 逐条修；
  历史快照或「旧路径 → 新路径」迁移映射表可在文档中加 `<!-- check-doc-links:ignore -->` 显式豁免。
- `mox_ci_gate.ok == false`：按 `failures` 顺序修复，**先端口后文档**（端口漂移一旦存在会直接阻断 CI）。

## 约束

- 端口变更必须同步 `docs/api/PORT-REGISTRY.md` 并通过 PORT-REGISTRY-001 变更流程；MOX 自有进程端口须落在 3000–3999，插件端口落在 30000–39999。
- 不要为了让校验通过而放宽白名单或删除校验规则；本 MCP 的 `shell=False`、脚本白名单、超时上限是安全边界。

## 快速命令（无 MCP 时等价手动执行）

```
python scripts/gate/verify-ports.py
python scripts/gate/check-doc-links.py
python tools/mox-governance-mcp/server.py --selftest
```
