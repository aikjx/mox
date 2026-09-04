# 璇玑 mox · 企业级真实可运行落地 + mox 模块化系统架构自动化测试验收（V1.0）

## Overview
- **Summary**：把 `璇玑 RelGraph` 仓库当前的 Rust 16 crate 工作空间 + Node backend-node + 前端 frontend-ui + melody2score Python 桌面端 + 7×8 算法对账 + 三流程端点（graph_bulk / file_upload+link / ai_full_rag）+ AI 四端点（process/analyze/capabilities/metrics）从「文档声明完成」推进到「每一条声明都能通过一条可重复执行的自动化命令独立验证通过」，并修复所有验证暴露的编译/运行/对账/覆盖率缺口，形成企业级 SLO：**cargo test 全绿、clippy 零告警、前端构建 0 错误、Node test suite 70+ 全绿、RBAC 11 探针全过、7×8 对账 Δ≤1e-6、SixDim 绑定 ≥90% 护栏、full_gate 通过率 ≥90%**。
- **Purpose**：用户明确要求「代码开发好、真架构开发好、真实可用、真实完成、企业级」，反模式是只有文档与顶层设计而没有可运行可验证的代码事实。本 spec 以「自动化断言证据包」为唯一交付物，空对空的"更优架构"不算交付（All-04 铁律）。
- **Target Users**：三联盟签署方（产品 / 算法 / 开发）、企业审计、总设计师、后续接手的架构师与后端开发专家（分别走 docs/enterprise/19 与 docs/enterprise/20 主控提示词）。

## Goals
1. **可运行**：16 个 Rust crate 零编译告警 + `cargo test --workspace` 全绿（含 benches / bins 不失败）。
2. **可验证**：7 大算法 × 8 数据集对账脚本（reconcile_7x8.js）56/56 全 PASS，Δ≤1e-6。
3. **三流程端点真实可用**：graph_bulk / file_upload+link / ai_full_rag 三端点有独立 E2E 冒烟，28s 内返回非错误状态。
4. **AI 四端点契约对齐**：Rust runtime 的 `/ai/engine/{process,analyze,capabilities,metrics}` 有路由存在断言与 AC-10 语义用例回归（router_semantics.rs 全绿）。
5. **治理闸门真实生效**：primiflow-fusion full_gate 有 ≥50 个基准用例，通过率 ≥90%；六维绑定覆盖率 ≥90% 护栏不被击穿。
6. **RBAC + 审计链闭环**：mox-expert 14 位专家阻断级检查 5/5 通过；审计三汇（内存 + 文件 + 可选 S3）签名可验证。
7. **前端 28 视图 + 管理区 5 面板**：`npm run build`（或 pnpm build）0 error，所有路由对应文件存在。
8. **Node 侧 70+ JS 测试套件**：backend-node test/*.js 全部绿；特别 T13 SLO / T14 HA / T12 reconcile / three-flows-trace 四条企业级用例全绿。
9. **Melody2Score 打包级鲁棒性**：`tests/test_score_sheet.py`（模拟 `sys.stderr=None` 的 PyInstaller 打包环境）+ `tests/_run_frozen_selftest.py --selftest-full` 两条均 pass（无声卡时 PortAudioError 降级跳过）。
10. **CI 一致性**：`.github/workflows/enterprise-ci.yml` 的步骤在本地能等价跑通。

## Non-Goals
- 不新增算法家族（不引入 A9；18 §五 A1~A8 不可动）。
- 不重写 OUS 父系统架构 v7；不拆 frontend-ui 回双应用（ADR-DOC-005）。
- 不做跨语言运行时的破坏性迁移（Node sidecar 不删，M0 仍保留）。
- 不替换重型框架 Axum / 不引入 actix / Leptos（18 §六）。
- 不写 DB schema 外迁（SQLite 仍是默认；PG/MySQL 零代码切换保留，不新增迁移脚本族）。

## Background & Context
- 工作空间 Cargo.toml 注册 16 members（含 runtime = gateway；15 主 crate + mox-common-meta）已在 `cargo check --workspace` 基线通过（exit 0；仅有 sqlx-postgres 0.8.0 future-incompat 告警，此为上游问题，不 block 企业级验收）。
- 7×8 对账 56/56 GREEN（baseline 已通过）。
- docs/enterprise/18 TOP-MASTER 是 L0 最高权威，所有 AC 不得与 18 §二~§八冲突。
- docs/enterprise/19 架构师主控提示词定义了 ⭕8 不可动，本 spec 严格继承。
- 企业文档基线（09-企业级mox 模块化系统架构维度完成归档.md §3）声明 649+ passed / 0 failed，本 spec 的目标是**复现 + 加固 + 把 649 上升到 ≥700**，并把"文档断言"翻译成"可重跑命令"。

## Functional Requirements
- **FR-1 三流程端点 E2E**：
  - `POST /api/graph/bulk`（graph_bulk）：接受 RAW 边数组 + 节点 JSON，返回写入后的 node/edge count 与关图 8 项门禁 score。
  - `POST /api/file/upload` + `POST /api/graph/link`（file_upload+link）：上传一个 Markdown 小文件 + 生成六维绑定 + 返回绑定覆盖率数。
  - `POST /api/ai/full_rag`（ai_full_rag）：发送 1 条中文用户查询 + 返回含 trace_id / provider_name / latency_ms / sources[] 的响应。
- **FR-2 AI 四端点**：Rust runtime 必须 4 条路由真实存在并注册 handlers/ai_engine.rs：`process`（意图识别后路由）、`analyze`（显式能力执行）、`capabilities`（能力矩阵 JSON）、`metrics`（成功率/降级率/延迟 JSON array）。
- **FR-3 算法侧真实输出**：graph-algorithms 必须暴露 `cnm_communities / brandes_betweenness / harmonic_closeness / pagerank_transpose / activation_spread` 公开函数，每个函数都有 2 个以上真实 `#[test]` 断言通过。
- **FR-4 璇玑 verify 五条件阻断**：mox-expert verify 层 `topology / data_dep / conflict / gains / code_rt` 五项各自有 ≥1 条"应阻断"和 1 条"应通过"的单测，vetoed=true/false 可观察。
- **FR-5 full_gate 八条件治理**：primiflow-fusion 的 G0/G1/G3 共 8 项，每个条件一条单测覆盖"开/关两种情况"，approved 五条件公式不可变。
- **FR-6 RBAC 六角色矩阵**：mox-expert rbac 层 policy.rs 中 super_admin / enterprise_admin / project_admin / developer / viewer / auditor 六角色 × 11 探针 = 66 组合全部有确定性断言。
- **FR-7 前端 28 视图真实渲染**：frontend-ui/src/views 下 28 个 Vue 文件 + admin/panels 下 5 面板，全部有 `<template>` 根元素、`<script>` 存在、router/index.js 注册对应路径。
- **FR-8 Melody2Score 打包鲁棒性回退**：PyInstaller console=False 环境模拟通过（test_score_sheet 对 stdin/stdout/stderr=None 场景输出正常）；音频播放无声卡时跳过（而非死锁 / AttributeError）。

## Non-Functional Requirements
- **NFR-1 测试基线**：`cargo test --workspace` → failed=0；ignored ≤ 10。
- **NFR-2 lint**：`cargo clippy --workspace -- -D warnings` → exit 0，warning 数 = 0。
- **NFR-3 性能**：graph-algorithms 500 节点 / 3 跳介数 P95 ≤ 420ms（与 18 基线一致）。
- **NFR-4 可用性**：AI 查询 P95 ≤ 1000ms；Rust runtime bootstrap 冷启动 ≤ 15s。
- **NFR-5 覆盖率护栏**：六维绑定（REQ/FUN/BIZ/ALG/TSK/COD）覆盖率 ≥ 90%，full_gate 通过率 ≥ 90%，任意一项低于护栏 = 阻断发布。
- **NFR-6 算法对账精度**：7×8 对账任何一项 Δ > 1e-6 = fail。
- **NFR-7 安全性**：审计事件每一条含 HMAC-SHA256 签名；RBAC 未登录访问 11 探针全返回 401/403。
- **NFR-8 兼容性**：SQLite 配置下所有 E2E 通过；PG/MySQL 仅需配置矩阵存在（零代码切换语义），不强求本 spec 在 CI 里同时跑三种。

## Constraints
- **Technical**：
  - 必须使用 workspace.dependencies 统一版本，禁止任何 crate 在自己 Cargo.toml 里写与 workspace 不一致的版本号（T4 依赖治理铁律）。
  - 所有图算法输入边必须 RAW 处理，库内展开为无向双向，禁止上层传"手动双条边"绕过 RAW 契约（度中心性正确铁律）。
  - PageRank 必须含转置图处理；激活扩散必须 d=0.85、30 轮收敛；CNM 为模块度贪心；Brandes = Brandes 2001；Harmonic = harmonic 算法实现。
  - AC-10 路由语义必须：静态路由优先 → 参数段少者优先 → 同参数数长路径优先。
  - ⛨璇玑 verify 最高权限不可被 RBAC 覆盖、不可被合规降级（All-04 铁律的自验=最高权限）。
- **Business**：三联盟铁律 All-01~04 严格执行。任何越权改动必须走 18 §十二 12 ADR 流程。
- **Dependencies**：
  - Rust 1.98-nightly（baseline 工具链已验证）；Node 22.23.x；npm 10.9.x；Python 3.10+（melody2score）。
  - 不升级 axum 主版本（保持 0.7）；不替换 wasmer 4.2。

## Assumptions
- 本地执行环境无 GPU / 无声卡 均不 block（降级跳过即可）。
- 未配置真实 LLM API key 的环境下 ai_full_rag 可以走 mock provider 并返回 `provider_name="mock"`，仍算满足 E2E 通过。
- S3 审计 sink 未配置时允许降级到文件 sink，仍算满足审计链。
- 本次 Spec 的全部验收命令必须能在一台 Windows 11 开发机（仓库工作目录直接 checkout）上 30 分钟内跑完。

## Open Questions
- [ ] ai_full_rag 的真实 LLM key 在企业部署中如何注入？（本 spec 允许 mock；在 AC 中写为 rule 的 mock 路径）
- [ ] NFR-3 / NFR-4 的性能基线若在本机因硬件差异达不到，是否以"CI cold baseline + 断言保留"为准？（建议：以自动化相对阈值代替绝对 ms，如「介数 P95 ≤ 相同规模 baseline 的 1.2×」）

---

## Acceptance Criteria

### AC-1: Workspace 编译全绿且 0 clippy 告警
- **Type**: `rule`
- **Given**: 仓库根目录为 cwd，Rust 工具链为 cargo 1.98+ nightly，已跑过一次 `cargo fetch`
- **When**: 执行 `cargo check --workspace --all-targets`
- **Then**: exit code = 0；再执行 `cargo clippy --workspace --all-targets -- -D warnings` exit code = 0，stderr 中 "warning:" 行数 = 0
- **Pass Condition**: 两条命令 exit 均为 0 且 clippy 0 warning
- **Evidence**: `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee /tmp/clippy.log && rg -c "^warning:" /tmp/clippy.log` 输出 = 0 + cargo check exit 0

### AC-2: cargo test --workspace 全绿（核心 16 crate）
- **Type**: `rule`
- **Given**: 同 AC-1
- **When**: `cargo test --workspace --exclude codex-rs --exclude claw-code --exclude hermes-agent --exclude cline --exclude openai-codex 2>&1 | tail -200`
- **Then**: 测试汇总行 `test result: ok. N passed; 0 failed; M ignored`，其中 failed = 0；N ≥ 600
- **Pass Condition**: 所有 target 的汇总全部 `ok`；0 failed
- **Evidence**: 末尾汇总表 + exit 0 截图/文本

### AC-3: 7 算法 × 8 数据集对账 Δ≤1e-6，56/56 GREEN
- **Type**: `rule`
- **Given**: Node 22.23 已安装；Rust graph-algorithms 已 build
- **When**: `node platform/services/graph-algorithms/scripts/reconcile_7x8.js`
- **Then**: 输出尾部 = `PASS: 56, FAIL: 0`；exit 0
- **Pass Condition**: 56 PASS / 0 FAIL，无任何 Δ 数字显式超过 1e-6
- **Evidence**: 完整脚本输出

### AC-4: AI 四端点路由注册 + AC-10 语义回归全绿
- **Type**: `rule`
- **Given**: Rust runtime 已编译
- **When**: `cargo test -p runtime router_semantics -- --nocapture` 与 `cargo test -p runtime ai_engine_e2e -- --nocapture`
- **Then**: 两个测试二进制 exit 0；router_semantics 中至少含 "static beats param / fewer-params beats many / same-params longer wins" 三句断言通过
- **Pass Condition**: 两测试 exit 0 + 三条路由语义命中
- **Evidence**: 两命令输出

### AC-5: full_gate ≥ 50 用例通过率 ≥ 90%
- **Type**: `rule`
- **Given**: primiflow-fusion 已编译
- **When**: `cargo test -p primiflow-fusion full_gate -- --nocapture 2>&1`
- **Then**: 输出里有一条统计 "cases_total=X, cases_passed=Y"，其中 X ≥ 50，Y/X ≥ 0.90
- **Pass Condition**: X ≥ 50 且 pass_rate ≥ 0.90
- **Evidence**: full_gate 测试最终统计行

### AC-6: 六维绑定覆盖率 ≥ 90% 护栏
- **Type**: `rule`
- **Given**: primiflow-fusion / kg-hub / mox-expert 已编译
- **When**: `cargo test -p primiflow-fusion sixdim_coverage -- --nocapture` 或等价命令
- **Then**: 输出 "sixdim_coverage = P%", P ≥ 90.0
- **Pass Condition**: 覆盖率数字 ≥ 90.0%
- **Evidence**: 覆盖率统计行

### AC-7: 璇玑 verify 五阻断级检查双分支覆盖（通过/阻断各至少 1 例）
- **Type**: `rule`
- **Given**: mox-expert 已编译
- **When**: `cargo test -p mox-expert verify -- --nocapture 2>&1`
- **Then**: "topology_pass=true / topology_block=true / data_dep_pass=true / data_dep_block=true / conflict_pass=true / conflict_block=true / gains_pass=true / gains_block=true / code_rt_pass=true / code_rt_block=true" 这 10 条日志/断言全部出现
- **Pass Condition**: 10 条全部出现
- **Evidence**: verify 测试输出截取

### AC-8: RBAC 六角色 × 11 探针 66 组合确定性断言
- **Type**: `rule`
- **Given**: mox-expert 已编译
- **When**: `cargo test -p mox-expert rbac -- --nocapture 2>&1`
- **Then**: rbac tests exit 0；显式统计 "66 cases, 0 violations"
- **Pass Condition**: exit 0 + 66 cases, 0 violations
- **Evidence**: rbac 测试尾部汇总

### AC-9: 三流程端点 E2E 冒烟（mock 后端接入）
- **Type**: `rule`
- **Given**: Rust runtime 能冷启动；sidecar Node 已 mock 或通过 feature 旁路
- **When**: `cargo test -p runtime mox_e2e -- --nocapture 2>&1` 或等价三流程 smoke
- **Then**: 三条断言 "graph_bulk ok / file_upload_link ok / ai_full_rag ok" 全部出现；每条延迟 ≤ 28s
- **Pass Condition**: 三条 ok + 每条 ≤ 28s
- **Evidence**: E2E 输出中三条断言

### AC-10: frontend-ui 构建 0 error + 28 视图 + 5 面板注册齐全
- **Type**: `rule`
- **Given**: frontend-ui 已 `npm install`（或 pnpm 已安装）
- **When**: (cd frontend-ui && pnpm build 2>&1 | tail -50) + (rg -l "<template>" frontend-ui/src/views/*.vue 2>&1 | wc -l) + (rg -l "path:" frontend-ui/src/router/index.js 2>&1)
- **Then**: build exit 0，无 "ERROR" 字样；Vue 文件数 ≥ 28；router 注册 path 数 ≥ 33（28 + admin 5）
- **Pass Condition**: 三条同时满足
- **Evidence**: build 末尾汇总 / Vue 数 / router 数

### AC-11: backend-node test suite ≥ 70 GREEN（含 T12/T13/T14/three-flows）
- **Type**: `rule`
- **Given**: backend-node 已 npm install；无外部 S3/PG 时允许环境变量降级
- **When**: `(cd platform/backend-node && npx mocha test --timeout 15000 2>&1 | tail -80)`
- **Then**: "passing: ≥ 70" / "failing: 0"；四条特殊测试 T12-algorithm-reconcile / T13-enterprise-slo / T14-enterprise-ha / three-flows-trace 结果均为 passing
- **Pass Condition**: ≥ 70 passing，0 failing，四条企业级用例全通过
- **Evidence**: mocha 尾部 + 四条用例名

### AC-12: Melody2Score 打包鲁棒性（stderr=None + 声卡缺失降级）
- **Type**: `rule`
- **Given**: Python 3.10+，melody2score 已安装 requirements
- **When**: (cd melody2score && python -X utf8 tests/test_score_sheet.py 2>&1) 与 (cd melody2score && python -X utf8 tests/_run_frozen_selftest.py --selftest-full 2>&1 | tail -30)
- **Then**: test_score_sheet 退出 0（打印 "stderr=None scenario: PASS"）；selftest-full 在无声卡时出现 "PortAudioError: SKIP (no audio device)" 而非 Error / Traceback；死锁回归项 "piano deadlock regression: PASS"
- **Pass Condition**: 三条场景全部符合
- **Evidence**: 两条命令尾部输出

### AC-13: 算法 API 可观测性（5 个算法公开函数 × ≥2 tests）
- **Type**: `rule`
- **Given**: graph-algorithms 已编译
- **When**: `cargo test -p graph-algorithms -- --list 2>&1`
- **Then**: 对 cnm / brandes / harmonic / pagerank_transpose / activation_spread 每个名字，相关 `#[test]` 个数 ≥ 2
- **Pass Condition**: 5 类算法各 ≥2 tests = ≥10 tests
- **Evidence**: test list 过滤后的统计

### AC-14: 审计三汇 HMAC 可验证性
- **Type**: `rule`
- **Given**: mox-expert 已编译；默认使用文件 sink
- **When**: `cargo test -p mox-expert audit_hmac -- --nocapture 2>&1`
- **Then**: 至少 1 条测试生成审计事件，并验证 HMAC(sha256, secret, payload) == signature 成功；篡改 1 byte 后验证失败
- **Pass Condition**: 正向通过 + 篡改失败 两条断言全部命中
- **Evidence**: audit_hmac 测试输出

### AC-15: 企业级 SLO 规模锚（500 节点介数 P95 ≤ baseline 1.2×）
- **Type**: `rubric`
- **Dimension**: 算法性能相对鲁棒性（相同 500-node ER 随机图 20 轮迭代取 P95）
- **Scale**: 1–5
- **Anchors**: 1 = 比 baseline 慢 2×+ 或 panic；3 = 在 baseline 的 1.0×~1.5× 之间；5 = ≤ baseline 的 1.05× 且 CV ≤ 5%
- **Pass Threshold**: ≥ 4（对应 ≤ baseline 1.2× 且抖动小）
- **Evidence**: `cargo bench -p graph-algorithms brandes_er500` 或 benchmark 等价脚本的 20 轮统计表

### AC-16: 工作空间依赖归一化一致性（T4 依赖治理回归）
- **Type**: `rubric`
- **Dimension**: 依赖归一化一致度
- **Scale**: 1–5
- **Anchors**: 1 = ≥ 10 个 crate 写了与 workspace.deps 不一致的版本号；3 = ≤ 3 个不一致，都在 dev-deps；5 = 0 个不一致，且所有 member 均使用 `workspace = true` 继承版本
- **Pass Threshold**: ≥ 4
- **Evidence**: 自定义脚本 `rg -n "^(serde|tokio|axum|reqwest|rusqlite|anyhow|thiserror|rayon|petgraph|serde_json|sea-query|sqlx) = " --glob Cargo.toml platform/ ais/claw-code ais/cline ais/hermes-agent ais/openai-codex 平台外也忽略` 或 `cargo tree --workspace -i` 一致性报告

### AC-17: 三联盟流程端点覆盖率（代码→文档→测试 双向绑定）
- **Type**: `rubric`
- **Dimension**: 六维绑定 REQ/FUN/BIZ/ALG/TSK/COD 对齐完整度
- **Scale**: 1–5
- **Anchors**: 1 = ≤ 70%；3 = 80%~90%；5 = ≥ 95% 且每条 BP-01~10 至少有 1 条自动断言对应
- **Pass Threshold**: ≥ 4（对应 ≥ 90%）
- **Evidence**: `primiflow-fusion` sixdim 覆盖率报告 + 与 `docs/enterprise/04 §3 BP-01~10` 逐条映射表
