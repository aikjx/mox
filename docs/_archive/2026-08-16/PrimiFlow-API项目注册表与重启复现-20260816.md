# PrimiFlow 项目注册表 + 跨重启复现 Q（API 增强 · 2026-08-16）

> 工程：`operator-unified-system/crates/primiflow`
> 目标：在既有 层2 需求解析 / 层6 真实持久化 / 层3 API 服务 三层之上，补齐 API 闭环缺口（能建不能查）并实现**重启后拓扑荷 Q 连续复用**（可复现/可审计）。

---

## 1. 本次交付

| 能力 | 端点 | 落点 |
|---|---|---|
| 项目审计清单 | `GET /api/projects` | `server.rs::list_projects` + `ProjectsListResp` |
| 项目详情 | `GET /api/projects/:id`（不存在 404） | `server.rs::get_project` + `ProjectDetailResp` |
| 单项目查询 | `Persistence::get_project(id)` | `persistence.rs`（Memory + Sqlite 双后端） |
| 启动重放 | 服务启动时自动 | `AppState::replay_from_store`，接入 `serve`/`spawn_serve` |

审计字段贯穿 `ProjectView`：id / name / policy / κ / τ / conserved / acyclic / reused / regularized / q_before / q_after / bound_nodes / bound_edges / created_at。

---

## 2. 关键设计

- **`row_to_record` 公共行映射**：从 `list_projects` 抽出 14 字段映射为 `ProjectRecord`，`list_projects` 与 `get_project` 共用，消除重复并修掉 clippy redundant closure。
- **`AppState::replay_from_store`**：启动时 `load_kb()` + `load_graph()` 恢复到 `PrimiEngine.kb` 与 `AssocGraph` 主图；用两个独立 Mutex 锁分段加锁避免死锁；失败（空库/损坏）静默跳过，不影响启动。
- **跨重启 Q 连续**：`server_demo` 已用 `Persistence::sqlite("./primiflow_runtime/primiflow.db")` 真实落盘；新进程启动即从该库重放，新需求可命中历史资产、不必从零探索。

---

## 3. 验证结果

| 项 | 结果 |
|---|---|
| `cargo test -p primiflow` | **46 lib + 8 API + 8 enterprise = 62 passed / 0 failed** |
| API 新增用例 | `l5_list_projects_after_create`、`l5_get_project_detail`（含 404）、`l5_replay_across_restart_continues_q` 全绿 |
| `cargo clippy -p primiflow --all-targets` | 仅 2 个非阻断 `new_without_default`（`gen/c1`、`gen/c7` 外部生成骨架） |
| `cargo run --example server_demo` | 监听 `0.0.0.0:3000` 正常 |

**`l5_replay_across_restart_continues_q`** 验证链路：进程 A 跑需求把资产 + 项目记录落盘到 SQLite → 进程 B 用同一 db 文件启动并重放 → `B 的 /api/assets` 与 `/api/projects` 均能恢复，证明拓扑荷 Q 在重启后连续。

---

## 4. 测试命令（复现）

```bash
cargo test -p primiflow --test api_server      # 8 个 L5 HTTP 用例
cargo test -p primiflow                         # 全量 62 用例
cargo run -p primiflow --example server_demo    # 起服务：0.0.0.0:3000
```
