# 璇玑 RelGraph · 企业级真实可运行落地 + 全维自动化测试验收 — 实施计划（tasks.md）

> **依赖图（总览）**：T1 基线盘点 → T2 clippy 修复 → T3 测试缺失补齐（graph-algorithms / mox-expert / primiflow-fusion / runtime）→ T4 前端构建 + T11 Node 并行 → T5 melody2score 桌面打包级回归 → T6 T4 依赖治理 rubric → T7 SLO 规模 rubric → T8 三流程端点 E2E → T9 汇总报告 + 独立 review。
> **优先级图例**：H = high（阻断），M = medium（推进），L = low（锦上添花）。
> **注意**：每任务至少 1 条 TR（rule 或 rubric）。完成必须附「Completion Evidence」。rubric TR 必须记录**分数 + 理由 + 证据**。

---

## Task 1: 企业级验收基线盘点 & 快速失败清单（先跑一遍，找缺口）
- **Status**: `pending`
- **Priority**: H
- **Depends On**: None
- **Description**:
  - 以仓库根为 cwd，一次性跑完基线命令，把失败点逐条登记到 `./target/enterprise-baseline-report.txt`
  - 命令包：`cargo check --workspace --all-targets` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo test --workspace 2>&1 | tail -60` → `node platform/services/graph-algorithms/scripts/reconcile_7x8.js` → `(cd platform/backend-node && npx mocha test --timeout 15000 2>&1 | tail -60)` → `(cd frontend-ui && pnpm build 2>&1 | tail -30)`
  - 失败点登记到本任务末尾「失败点→对应修复任务」映射表；供后续任务参考。
- **Acceptance Criteria Addressed**: AC-1, AC-2, AC-3, AC-10, AC-11（先行建立 true baseline 快照）
- **Test Requirements**:
  - `rule` TR-1.1: Baseline 报告完整记录 6 条命令的 exit code + 最后 30 行输出；任何失败都有可定位的错误片段
  - `rule` TR-1.2: 报告同时列出所有失败 crate / 测试用例全名 / 失败断言行，映射到 T2/T3/T4/T5/T11 中 ≥ 1 条修复任务
  - `rubric` TR-1.3: 失败点诊断完整度；Scale 1-5；1 = 仅有 exit 1 无信息；3 = 每个失败有 1 个报错；5 = 每个失败有「报错→可能根因→指向修复任务」三段；Threshold ≥ 4
- **Notes**: 本任务只做盘点，不做修复；禁止改任何源码。

---

## Task 2: `cargo clippy --workspace -- -D warnings` 清零
- **Status**: `pending`
- **Priority**: H
- **Depends On**: T1
- **Description**:
  - 根据 T1 基线报告的 clippy warnings 列表，在对应 crate 做定点修复；**禁止用 `#[allow(...)]` 批量压制**（All-04：你自验自验，不能用 allow 擦屁股）。
  - 修复顺序：runtime / mox-expert / primiflow-fusion / mox-system / graph-algorithms 这 5 个高权重 crate 先。
  - 外部 crate（ais/* 子仓）的 warnings 可由 `--exclude` 加白名单；不在企业验收主工作空间时不强求 0。
- **Acceptance Criteria Addressed**: AC-1 (NFR-2)
- **Test Requirements**:
  - `rule` TR-2.1: `cargo clippy --workspace --all-targets --exclude codex-rs --exclude claw-code --exclude hermes-agent --exclude cline --exclude openai-codex -- -D warnings 2>&1 | tee /tmp/clippy.log; rg -c "^warning: " /tmp/clippy.log` 输出 = 0 且 exit 0
  - `rule` TR-2.2: 主 5 crate（runtime/mox-expert/primiflow-fusion/mox-system/graph-algorithms）在各自 `Cargo.toml` 中不得新增 `lints.workspace=false` 或 lints 覆盖块
  - `rubric` TR-2.3: 修复方式合理性；Scale 1-5；1 = 全部用 allow；3 = 一半 allow 一半真改；5 = 0 个 allow（或仅 allow 上游问题并显式注释 sqlx-postgres 那一条）；Threshold ≥ 4
- **Notes**: sqlx-postgres future-incompat 属于上游，可用 `lints.clippy.xxx = allow` 单条精确注释并在 TR-2.3 评分时作为例外。

---

## Task 3: Rust 单元 / 集成测试缺失补齐（核心 5 crate 按优先级）
- **Status**: `pending`
- **Priority**: H
- **Depends On**: T2（先 clippy 清零，再补 tests，避免 warning 重复回流）
- **Description**:
  分 4 个子任务，互不写对方源文件，可顺序执行：
  - **3A graph-algorithms**：AC-3 / AC-13。对 5 类算法（CNM / Brandes / Harmonic / PageRank+转置 / 激活扩散）每类至少补到 ≥2 条 `#[test]`；断言内容来自 reconcile_7x8.js 8 数据集的已知节点数/介数最大节点名；保证 Δ≤1e-6 的对账在 Rust 侧也有独立断言。
  - **3B mox-expert verify + rbac + audit**：AC-7 / AC-8 / AC-14。verify 层 5 阻断级检查 = 至少 10 条 tests（5 通过 + 5 阻断）；rbac 66 组合至少 6 条表驱动 tests 覆盖 6×11=66；audit_hmac 至少 1 条伪造 payload 失败 + 1 条通过签名验证。
  - **3C primiflow-fusion full_gate + sixdim_coverage**：AC-5 / AC-6。补到 full_gate cases ≥ 50 且通过 ≥ 90%；SixDim 覆盖率统计命令输出 P ≥ 90.0%（如未达则修复悬空绑定，不允许造假报告）。
  - **3D runtime AI 四端点 + AC-10 路由语义 + 三流程 E2E**：AC-4 / AC-9。补 router_semantics 的三条断言（静态优先/参数少/同参数长路径）；补 ai_engine routes 的存在；补 mox_e2e 三条（graph_bulk / file_upload_link / ai_full_rag） ≤ 28s。
- **Acceptance Criteria Addressed**: AC-3, AC-4, AC-5, AC-6, AC-7, AC-8, AC-9, AC-13, AC-14
- **Test Requirements**:
  - `rule` TR-3A.1: `cargo test -p graph-algorithms` 0 failed；`--list` 输出中 5 类算法 test 名各 ≥2
  - `rule` TR-3B.1: `cargo test -p mox-expert verify rbac audit_hmac` → 汇总 0 failed；10 条阻断分支、66 cases、HMAC 正+反全部命中
  - `rule` TR-3C.1: `cargo test -p primiflow-fusion full_gate sixdim` → cases_total ≥ 50 且 pass_rate ≥ 90%；sixdim_coverage ≥ 90.0%
  - `rule` TR-3D.1: `cargo test -p runtime router_semantics ai_engine_e2e mox_e2e` → 0 failed；三条路由语义断言 + 三流程端点断言全部出现
  - `rubric` TR-3.2: 测试断言质量（可证伪性 + 不依赖随机）；Scale 1-5；1 = 仅 assert!(true)；3 = 断言数量 ≥ 要求的 80%；5 = 每条断言用具体数值且与 reconcile_7x8.js 基线一致；Threshold ≥ 4

---

## Task 4: frontend-ui 构建 0 错误 + 28 视图 + 5 面板 注册检查
- **Status**: `pending`
- **Priority**: M
- **Depends On**: None（可与 T1 并行启动；若依赖冲突则待 T1 结束后确认）
- **Description**:
  - `pnpm install`（或使用现有 pnpm-lock.yaml）；运行 `pnpm build`；修复任何 ERROR / WARN（WARN 允许但不得超过 10 条，ERROR 必须 0）。
  - 检查 `src/router/index.js` 中 path 注册数量 ≥ 33（28 视图 + admin 5 面板），任何缺失要补上。
  - 所有 Vue 文件须有 `<template>` 根与 `<script>`。
- **Acceptance Criteria Addressed**: AC-10
- **Test Requirements**:
  - `rule` TR-4.1: `(cd frontend-ui && pnpm build) && echo BUILD_OK` → 末尾 BUILD_OK 出现且 "ERROR" 命中 = 0
  - `rule` TR-4.2: `rg -c "<template>" frontend-ui/src/views/*.vue frontend-ui/src/views/admin/panels/*.vue` 的每个文件 ≥ 1；文件总数计数 ≥ 33
  - `rule` TR-4.3: `rg "^\\s*path:" frontend-ui/src/router/index.js` 匹配数 ≥ 33
  - `rubric` TR-4.4: 构建产物整洁度；Scale 1-5；1 = 产物 > 50MB 且有 20+ WARN；3 = ≤ 30MB 且 ≤ 10 WARN；5 = ≤ 20MB 且 WARN ≤ 3；Threshold ≥ 3

---

## Task 5: Melody2Score 打包级鲁棒性回归（stderr=None + 声卡缺失降级）
- **Status**: `pending`
- **Priority**: M
- **Depends On**: None
- **Description**:
  - 修复 `tests/test_score_sheet.py`（若失败）确保在 `sys.stderr=None` 模拟环境下 `jianpu-ly` / `music21` 不触发 AttributeError；`gui.py _ensure_windowed_streams` 兜底生效。
  - 修复 `tests/_run_frozen_selftest.py --selftest-full`：钢琴播放冒烟项在 PortAudioError 时降级输出 SKIP，禁止死锁（加入 30s 超时）；禁止 AttributeError: 'NoneType' has no attribute write。
- **Acceptance Criteria Addressed**: AC-12
- **Test Requirements**:
  - `rule` TR-5.1: `(cd melody2score && python -X utf8 tests/test_score_sheet.py)` exit 0；输出显式含 "stderr=None scenario: PASS"
  - `rule` TR-5.2: `(cd melody2score && python -X utf8 tests/_run_frozen_selftest.py --selftest-full --timeout 300)` exit 0；输出含 "piano deadlock regression: PASS"；如无音频设备含 "PortAudioError: SKIP" 不含 Traceback
  - `rubric` TR-5.3: 降级语义合理性；Scale 1-5；1 = 无声卡时抛错退出；3 = 跳过硬编码 PASS；5 = SKIP 语义写入 selftest 报告 JSON 中 `skipped: ["audio_play_unsupported"]` 字段；Threshold ≥ 4

---

## Task 6: backend-node 70+ tests 企业级修复（T12/T13/T14/three-flows 全 GREEN）
- **Status**: `pending`
- **Priority**: M
- **Depends On**: T1
- **Description**:
  - 按 T1 报告把 backend-node test 失败项分为「依赖外部服务（S3/PG）需降级 mock」与「真实代码 bug」两类。
  - 对 S3/PG 依赖用 MemoryStore / in-memory JSON 替换（如 `test-storage-postgres.js` 未配 DB 时打 SKIP 而非 fail）。
  - 对 T12 reconcile / T13 SLO / T14 HA / three-flows-trace 四条企业级用例，保证每条都能通过或显式 SKIP（SKIP 需有 `[SKIP]` 前缀与原因）。
- **Acceptance Criteria Addressed**: AC-11
- **Test Requirements**:
  - `rule` TR-6.1: `(cd platform/backend-node && npx mocha test --timeout 15000 2>&1 | tail -60)` 含 "passing: ≥ 70" + "failing: 0"
  - `rule` TR-6.2: 四条特殊用例名（algorithm-reconcile / enterprise-slo / enterprise-ha / three-flows-trace）要么 pass 要么 `[SKIP] no-xxx`；fail 数 = 0
  - `rubric` TR-6.3: mock 策略合理性；Scale 1-5；1 = 全 SKIP；3 = 一半 mock 一半真；5 = 企业级四条全 pass；Threshold ≥ 3

---

## Task 7: 工作空间依赖归一化一致性审计（对应 AC-16）
- **Status**: `pending`
- **Priority**: L
- **Depends On**: T2（clippy 之后，版本稳定了再跑）
- **Description**:
  - 写 1 个审计脚本（`scripts/check_workspace_deps.ps1` 或 `.js`），遍历所有 16 个平台主 crate 的 Cargo.toml，把 `serde / tokio / axum / reqwest / rusqlite / anyhow / thiserror / rayon / petgraph / serde_json / sea-query / sqlx` 12 个核心依赖的版本号与 `workspace.dependencies` 对比。
  - 任何不一致（包括仅 workspace 写了版本而下级 crate 写死具体版本）都记一条 violation；提交修复 PR 把所有下级改为 `workspace = true`。
- **Acceptance Criteria Addressed**: AC-16
- **Test Requirements**:
  - `rule` TR-7.1: 审计脚本 exit 0 → violations = 0 或者存在，给出具体修复；任务完成后必须 violations=0
  - `rubric` TR-7.2: AC-16 rubric；Scale 1-5；评估规则同 spec AC-16；Threshold ≥ 4；完成后须记录 **分数 / 理由 / 证据**（如报告 N 个 workspace=true 比例）

---

## Task 8: 算法 SLO 相对性能基线（500 节点介数 P95 ≤ baseline 1.2×，对应 AC-15）
- **Status**: `pending`
- **Priority**: L
- **Depends On**: T3A（graph-algorithms 测试已经稳定）
- **Description**:
  - 在 `graph-algorithms/benches/` 下补 1 个 brandes_er500 criterion benchmark；20 轮同 ER(500, p=0.01) 随机图（种子固定）。
  - 取冷启动第一次作为 baseline_0；后续 20 轮 P95 ≤ baseline_0 的 1.2×。结果写入 `target/brandes_er500_stats.json`。
- **Acceptance Criteria Addressed**: AC-15
- **Test Requirements**:
  - `rule` TR-8.1: `cargo bench -p graph-algorithms brandes_er500 -- --warm-up-time 0 --measurement-time 10s --sample-size 20` exit 0；`brandes_er500_stats.json` 存在并有 P50/P95/P99
  - `rubric` TR-8.2: AC-15 rubric；Scale 1-5；按 spec 打分；Threshold ≥ 4；须记录分数 + 理由 + 证据（统计表）

---

## Task 9: 企业级验收总报告 + 独立 Review 证据包
- **Status**: `pending`
- **Priority**: H
- **Depends On**: T1, T2, T3, T4, T5, T6, T7, T8 全部 completed
- **Description**:
  - 生成 `.trae/specs/<folder>/evidence/` 目录，保存 7 份核心命令的完整输出文本、rubric 评分表、16 条 AC 的逐一 pass/fail 状态。
  - 写一页总表 `evidence/SUMMARY.md`：每条 AC 编号 → pass/fail → 证据文件路径 → 关键断言值。
  - 报告同时对齐 docs/enterprise/11 全维测试报告 + 12 RBAC 验收 + 13 伪代码清零验收 + 16 P9 闸门验收，保证四者之间的测试数量不矛盾。
- **Acceptance Criteria Addressed**: 所有 17 条 AC 的汇总证据页
- **Test Requirements**:
  - `rule` TR-9.1: evidence/ 内对 17 条 AC 每条至少有 1 个文件对应；SUMMARY.md 每条 AC 都显式 PASS
  - `rule` TR-9.2: `rg "FAIL|block|Traceback" evidence/*.txt | rg -v "(SKIP|skip|ExpectedFail)"` 匹配数 = 0
  - `rubric` TR-9.3: 报告可读性 / 对齐性；Scale 1-5；1 = 只有乱码；3 = 有数字但无法追溯；5 = 每条断言数字→命令→输出三者一一反查；Threshold ≥ 4
- **Notes**: 本任务完成后，才能进入 Review 阶段

---

## 依赖矩阵（快速版）

```
T1(盘点) ─┬─> T2(clippy) ─> T3(补tests) ─┬─> T8(SLO perf)
          ├─> T6(node tests)            │
          └─> T4/ T5 可并行             ├─> T7(依赖归一)
                                          └─────────────> T9(汇总)
```

---

## 中止 / 重规划条件
- 若 T1 发现 cargo test 已经 ≥ 649 passed 0 failed（说明基线实际已经全绿），则 T2 / T3 降为「补齐缺失的 AC 对应 TR 断言报告」而不强行改代码。
- 若本地缺少 Python / pnpm / Node 工具链中的任一者，对应任务标为 blocked 并登记 Unblock Condition。
