# AGENTS.md — Infotopograph 仓库维护指南

面向在此仓库工作的 AI 代理与开发者的持久约定。其余项目级说明见 `README.md` 与 `docs/README.md`。

## 常用命令

```bash
cargo build                 # 构建 workspace 默认 members
cargo test                  # 运行单元测试（默认 members）
cargo clippy --all-targets  # lint（CI 门禁，workspace.lints 已配置）
python scripts/verify-ports.py   # 端口漂移校验（CI 门禁）
docker-compose up -d --build     # 一键部署
./start.sh --dry-run             # 启动前预检
```

注意：部分 crate（napi/PyO3 绑定）不在 `default-members` 中，需单独 `cargo check -p <crate>`。

## 架构速览

- **6 层架构**：`platform/foundation` → `domains/*/api` → `domains/*/proto`（gRPC 契约）→ `domains/*/core`（纯计算）→ `domains/*/svc`（服务）→ `platform/gateway`（8080 唯一入口）。
- **workspace 143 crates**，域驱动：kg / ai / flow / data / cloud / voice / market / alliance / kb / base / project / platform。
- 前端 `frontend-ui/`（Vite Vue3，dev 3020）；子项目在 `projects/`。
- 旧 Python 版服务已归档至 `platform/legacy/`（**勿用**，Rust 为唯一实现）。

## 仓库治理规则

- **报告**只允许进入 `reports/`（html/ markdown/ data/）。HTML 报告放 `reports/html/<报告名>/`，自带 `assets/`（独有资源），共享字体/JS 引用 `../../_shared/`（即 `reports/_shared/`），**禁止**在报告目录内复制 `_shared/`。
- **文档**按 `docs/` 子目录分类归档（architecture / enterprise / modules / working-reports / _archive），根目录禁止散落报告文档。
- **提交规范**：conventional commits（feat/fix/docs/chore/refactor/build + scope），中文描述。
- **运行时目录**（`target/` `.logs/` `.runtime/` `data/` `workspace/` `log/`）为本地运行态，已被 `.gitignore` 管理；`target/` 含本地服务数据（mox.db 等），**不要删除**，也不要提交。
- `.trae/`（IDE 工作区数据）不入库；`ais/` `third_party/` 为第三方参考，不入库。
- 端口权威来源：`docs/api/PORT-REGISTRY.md`；端口变更必须同步该文档并通过 `verify-ports.py`。

## 高频注意点

- 修改 workspace 配置后 `cargo check` 以根 `Cargo.toml`（143 members）为准，缺失目录会导致构建失败。
- 共享报告资源改动会波及全部 `reports/html/*`，修改后运行引用校验（`src/href` 指向 `reports/_shared/` 必须可解析）。
- Windows 下文本文件为 UTF-8（部分带 BOM）；PowerShell 读取中文显示乱码属正常显示问题，勿据此改写文件。
