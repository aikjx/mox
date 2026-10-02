# AGENTS.md — Infotopograph 仓库维护指南

面向在此仓库工作的 AI 代理与开发者的持久约定。其余项目级说明见 `README.md` 与 `docs/README.md`。

## 常用命令

```bash
cargo build                 # 构建 workspace 默认 members
cargo test                  # 运行单元测试（默认 members）
cargo clippy --all-targets  # lint（CI 门禁，workspace.lints 已配置）
python scripts/gate/verify-ports.py   # 端口漂移校验（CI 门禁）
python scripts/gate/check-frontend-module.py  # 前端模块化门禁（CI 门禁；规范见 docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md）
python scripts/gate/check-locale-format-outlets.py  # 时间/locale 口径闸门（CI 门禁，§5.32；无参 toLocaleString 棘轮 + 出口写法唯一性 + locale pin + 扫描集分母）
python scripts/gate/check-script-paths.py           # 脚本路径解析门禁（CI 门禁，§5.62；代码/CI/仓根脚本里写死的仓内路径必须解析得到，先跑 --selftest 再跑体检）
python scripts/gate/check-doc-formulas.py --selftest   # 核心公式识别器的针（CI 门禁；口径与裁决点见 docs/architecture/API-SURFACE-AUTHORITY-PLAN-v0.1.md，其 --census/--dircheck/--recognizer 因裁决点 6 未裁故意不进 rc）
python scripts/gate/check-api-surface.py --selftest     # 网关对外表面普查＋核心公式单一算源台账（CI 门禁；--ledger 判文档托管块与现渲染逐字节相同，普查判决模式未接 CI）
python frontend-ui/scripts/gate/check-api-binding-kinds.py --check  # @/api 的函数型导出被当对象取属性调用（CI 门禁，零容忍；api 层在测试里是桩，全量 vitest 看不见这一类 TypeError）
python frontend-ui/scripts/gate/check-ep-feedback-imports.py --check # Element Plus 反馈 API 缺 import（CI 门禁；探针不是棘轮，只有判据自身失效才打红）
python frontend-ui/scripts/gate/check-framework-imports.py --check  # vue/vue-router/pinia 具名导出用而未绑（CI 门禁，零容忍；缺绑即首屏 ReferenceError）
python frontend-ui/scripts/gate/check-theme-tokens.py --check        # 主题令牌覆盖审计与孤儿名棘轮（CI 门禁；存量登记在 ORPHAN_BASELINE）
scripts/gate/check-all.ps1            # 一键质量检查（8 项：secret/fmt/clippy/test/前端构建/前端门禁/端口/其余闸门同账）——本地一键，项数由脚本里的 $Total 单源，由 scripts/gate/check-doc-formulas.py 的 夹具R 逐条对账
docker-compose up -d --build     # 一键部署
./start.sh --dry-run             # 启动前预检
scripts/startup/start-mox-enterprise.ps1 # 企业级四进程一键启动（编排器3001/联盟调度3100/执行3200/模块化网关3080）
scripts/startup/stop-mox-enterprise.ps1  # 对应一键停止（不动前端3020/primiflow8000/melody2score8012）
python tools/mox-governance-mcp/server.py --selftest   # 治理 MCP 自检（端口/文档门禁 + CI 组合）
python tools/alliance-demo/alliance_demo.py             # 专家联盟端到端链路演示（产出 reports/ 证据 + 治理体检）
```

注意：部分 crate（napi/PyO3 绑定）不在 `default-members` 中，需单独 `cargo check -p <crate>`。

## 架构速览

- **6 层架构**：`platform/foundation` → `domains/*/api` → `domains/*/proto`（gRPC 契约）→ `domains/*/core`（纯计算）→ `domains/*/svc`（服务）→ `platform/gateway`（3080 唯一入口）。
- **workspace 149 crates**，域驱动：kg / ai / flow / data / cloud / voice / market / alliance / kb / base / project / platform。
- 前端 `frontend-ui/`（Vite Vue3，dev 3020）；子项目在 `projects/`。
- 旧 Python 版服务已归档至 `platform/legacy/`（**勿用**，Rust 为唯一实现）。

## 仓库治理规则

- **报告**只允许进入 `reports/`（html/ markdown/ data/）。HTML 报告放 `reports/html/<报告名>/`，自带 `assets/`（独有资源），共享字体/JS 引用 `../../_shared/`（即 `reports/_shared/`），**禁止**在报告目录内复制 `_shared/`。
- **文档**按 `docs/` 分层归档，**结构权威为 `docs/ARCHITECTURE-OF-DOCS.md`（DOC-GOV-ARC-V1.0）**：L0 `docs/README.md` · `docs-hub/`｜L1 `CORE-CAPABILITIES.md` `ROADMAP-DOMAINS.md` `API-REGISTRY.md`｜L2 `architecture/`（含 meta/microservices/rust-enterprise/ai/graph/plugin/full-dimensional/assets）｜L3 `modules/` `expert-alliance/`｜L4 `api/` `database/`｜L5 `standards/` `normalization/`｜L6 `enterprise/` `specifications/`｜L7 `working-reports/`（含 verification/audits/）｜L8 `_archive/`。**一个主题一个目录，一个事实一个权威源，路径即契约**；根目录禁止散落报告文档，目录迁移须登记映射并跑 `python scripts/gate/check-doc-links.py`。
- **提交规范**：conventional commits（feat/fix/docs/chore/refactor/build + scope），中文描述。
- **运行时目录**（`target/` `.logs/` `.runtime/` `data/` `workspace/` `log/`）为本地运行态，已被 `.gitignore` 管理；`target/` 含本地服务数据（mox.db 等），**不要删除**，也不要提交。
- `.trae/`（IDE 工作区数据）不入库；`ais/` `third_party/` 为第三方参考，不入库。
- 端口权威来源：`docs/api/PORT-REGISTRY.md`；端口变更必须同步该文档并通过 `verify-ports.py`。

## 高频注意点

- 修改 workspace 配置后 `cargo check` 以根 `Cargo.toml`（149 members）为准，缺失目录会导致构建失败。
- 共享报告资源改动会波及全部 `reports/html/*`，修改后运行引用校验（`src/href` 指向 `reports/_shared/` 必须可解析）。
- Windows 下文本文件为 UTF-8（部分带 BOM）；PowerShell 读取中文显示乱码属正常显示问题，勿据此改写文件。
