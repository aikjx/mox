# T17 SDK + E/F 运维落地 实施任务清单 (tasks.md)

- 规格书: [spec.md](./spec.md)
- 生成日期: 2026-08-24
- 总里程碑:
  - M1 完成 C-01~C-06: SDK 跨语言 180 示例 + 80 tests
  - M2 完成 E-12~E-20: T12→T13→T15→T18→T19(≥706)→T20
  - M3 Run-T17-EF-All.ps1 一键 Grade S。

---

## Task 1: Rust mox-sdk-cloud 30 示例骨架

- **Priority**: high
- **Status**: pending
- **AC**: AC-C-01
- **Depends**: (none)
- **Outputs**:
  - `platform/sdk/rust/mox-sdk-cloud/Cargo.toml` (examples 配置)
  - `platform/sdk/rust/mox-sdk-cloud/src/lib.rs` (facade traits + FakeClient)
  - `platform/sdk/rust/mox-sdk-cloud/examples/cloud-001_*.rs` … `cloud-030_*.rs`
- **Test Requirements**
  - rule: `cargo build -p mox-sdk-cloud --examples` exit 0
  - rule: `cargo run --example cloud-001_*` … 抽 10 个 examples exit 0 + 打印 `XJ-OK`
  - rule: tests/test_sdk_cloud.rs (≥15 cases) 覆盖 30 示例 id 存在性 + facade trait 方法
  - rubric: 示例主题多样性 (0-10, 及格 7)：桶/对象/STS/IAM/Quota/WORM/Lifecycle/DBHC 8 大类中至少 7 类被覆盖

---

## Task 2: Rust mox-sdk-graph 30 示例骨架

- **Priority**: high
- **Status**: pending
- **AC**: AC-C-02
- **Depends**: Task 1
- **Outputs**:
  - `platform/sdk/rust/mox-sdk-graph/Cargo.toml`
  - `platform/sdk/rust/mox-sdk-graph/src/lib.rs` (GraphClient facade)
  - 30 examples `graph-001_*` … `graph-030_*`
- **Test Requirements**
  - rule: `cargo build -p mox-sdk-graph --examples` exit 0
  - rule: tests/test_sdk_graph.rs (≥15 cases) CDC/Spark/Projection/Faults 每类 ≥ 3
  - rule: 抽 10 个 examples run，stdout 包含 `XJ-OK`

---

## Task 3: Node.js SDK cloud 30 示例

- **Priority**: high
- **Status**: pending
- **AC**: AC-C-03 / id对齐 C-05
- **Depends**: Task 1
- **Outputs**:
  - `platform/sdk/nodejs/mox-sdk-cloud/package.json` (name=mox-cloud) + `index.js` (CloudClient 伪类)
  - `platform/sdk/nodejs/examples/cloud/cloud-001_*.js` … `cloud-030_*.js`（与 Rust cloud IDs 一一对应）
  - `platform/sdk/nodejs/test/t17-node-cloud-15.test.js` (≥ 15 Mocha)
- **Test Requirements**
  - rule: 每个 example 执行 `node xxx.js` exit 0 + stdout `XJ-OK: cloud-XXX`
  - rule: `npx mocha test/t17-node-cloud-15.test.js` pass 全部
  - rule: 30 示例 ID 集合 == Rust Task1 的 {cloud-001..cloud-030}

---

## Task 4: Node.js SDK graph 30 示例

- **Priority**: high
- **Status**: pending
- **AC**: AC-C-03 / C-05
- **Depends**: Task 2, Task 3
- **Outputs**:
  - `platform/sdk/nodejs/mox-sdk-graph/package.json` + `index.js` (GraphClient)
  - `examples/graph/graph-001_*` … `graph-030_*`（与 Rust graph IDs 对齐）
  - `test/t17-node-graph-15.test.js` (≥ 15 Mocha)
- **Test Requirements**
  - rule: 30 node examples exit 0 / `XJ-OK: graph-XXX`
  - rule: Mocha 15 pass 全部
  - rule: 30 示例 ID 与 Rust Task2 一致

---

## Task 5: Python SDK cloud 30 示例

- **Priority**: high
- **Status**: pending
- **AC**: AC-C-04 / C-05
- **Depends**: Task 3
- **Outputs**:
  - `platform/sdk/python/mox_sdk_cloud/__init__.py` (CloudClient 类)
  - `examples/cloud/cloud-001_*.py` … `cloud-030_*.py`（ID 对齐 Rust/Node）
  - `test/test_cloud_15.py` (≥ 15 pytest)
- **Test Requirements**
  - rule: python3 每个脚本 exit 0 / `XJ-OK: cloud-XXX`
  - rule: `pytest test/test_cloud_15.py -q` 全部 pass
  - rule: 30 示例 ID 一致

---

## Task 6: Python SDK graph 30 示例

- **Priority**: high
- **Status**: pending
- **AC**: AC-C-04 / C-05
- **Depends**: Task 4, Task 5
- **Outputs**:
  - `platform/sdk/python/mox_sdk_graph/__init__.py`
  - `examples/graph/graph-001_*` … `graph-030_*`（ID 对齐）
  - `test/test_graph_15.py` (≥ 15 pytest)
- **Test Requirements**
  - rule: 30 py scripts OK / `XJ-OK: graph-XXX`
  - rule: pytest 15 全绿

---

## Task 7: T17 SDK 矩阵 JSON + SDK 跨语言测试汇总 Rubric 脚本

- **Priority**: high
- **Status**: pending
- **AC**: AC-C-05 / AC-C-06 (总测试 ≥ 80)
- **Depends**: Task 1..6
- **Outputs**:
  - `projects/t17-sdk-examples/matrix.json` (180 entries)
  - `projects/t17-sdk-examples/runs/<id>/rubric_t17.json` (6 维评分，Grade S/A/B/C/D)
  - `scripts/Run-T17-SDK-All.ps1`（Rust examples build + tests + Node test + Python test + Rubric）
- **Test Requirements**
  - rule: matrix.json entries == 180 且 ID 集合完全对齐
  - rule: 所有 tests (Rust 30 + Node 30 + Python 30) 总和 ≥ 80 且 0 failing
  - rubric: T17 SDK 质量 (0-100，及格 80)：功能完整度(40) / 跨语言一致性(25) / 示例可执行(20) / 覆盖率(15)

---

## Task 8: T12 Helm DR Chart（双区域双活）

- **Priority**: medium
- **Status**: pending
- **AC**: AC-E-12
- **Depends**: (none)
- **Outputs**:
  - `deploy/helm/mox-dr/Chart.yaml` + `values.yaml`
  - `templates/deployment-primary.yaml`
  - `templates/deployment-secondary.yaml`
  - `templates/_helpers.tpl`
  - `templates/service-primary.yaml` / `service-secondary.yaml`
  - `templates/pdb.yaml` (PodDisruptionBudget minAvailable=2)
  - `templates/hpa.yaml` (HPA min=2 max=10)
  - `templates/region-selector.yaml` (节点亲和)
  - `templates/NOTES.txt` (failover 说明)
- **Test Requirements**
  - rule: `helm lint deploy/helm/mox-dr` exit 0
  - rule: helm template 输出中 deployment 数量 ≥ 2，PDB/HPA/Svc 各 ≥ 1
  - rule: AC-E-12 rubric 打分 ≥ 70

---

## Task 9: T13 信创矩阵 + 运维手册

- **Priority**: medium
- **Status**: pending
- **AC**: AC-E-13
- **Depends**: (none)
- **Outputs**:
  - `deploy/docs/xinchuang-matrix.md` (3 OS × 4 CPU × 3 DB = 36 单元格 + 5 条环境指令)
  - `deploy/docs/ops-manual.md` (13 章：架构/部署/升级/回滚/备份/恢复/监控/告警/配额/安全/审计/容灾/FAQ)
- **Test Requirements**
  - rule: xinchuang-matrix.md 中 "fully" / "partial" / "planned" 出现次数之和 ≥ 36
  - rule: ops-manual.md 包含 13 个 H2 标题（`## 1.` … `## 13.`）

---

## Task 10: T15 HA 3 主 3 从 + 容量 + TCO Rubric

- **Priority**: medium
- **Status**: pending
- **AC**: AC-E-15
- **Depends**: Task 8
- **Outputs**: `deploy/docs/ha-capacity-tco.md`
- **Test Requirements**
  - rule: 文档中 HA 拓扑图 (ASCII art / Mermaid) 节点数 ≥ 6
  - rule: 容量数字段："内存(GB)" / "磁盘(TB)" / "CPU cores" / "QPS 峰值" 4 项各 ≥ 1 次
  - rule: TCO 至少列出 2027 / 2028 / 2029 分项，有年度合计 + 3 年总计(元)
  - rubric: AC-E-15 rubric ≥ 70

---

## Task 11: T18 8 阶段 Trace 埋点

- **Priority**: medium
- **Status**: pending
- **AC**: AC-E-18
- **Depends**: (none)
- **Outputs**:
  - `platform/services/mox-graph-service/src/trace_8stages.rs` (+ lib.rs mod)
  - `deploy/docs/trace-8stages-dashboard.json` (12+ metrics)
- **Test Requirements**
  - rule: TraceStage 枚举包含全部 8 阶段
  - rule: Rust tests ≥ 8：每阶段 emit_span 增加 span_count
  - rule: export_json() 输出条数 ≥ 8，字段齐全 trace_id/span_id/stage/start_ms/end_ms
  - rule: dashboard.json 中 metrics 名称数 ≥ 12

---

## Task 12: T20 Helm 一键伞图 + Gray-Warmup 4 阶段脚本

- **Priority**: medium
- **Status**: pending
- **AC**: AC-E-20
- **Depends**: Task 8
- **Outputs**:
  - `deploy/helm/mox/Chart.yaml` (umbrella: 依赖 mox-dr + mox-core + mox-observability)
  - `deploy/helm/mox/values.yaml` (global.gray.enabled + stages: [1,10,50,100])
  - `deploy/helm/mox/templates/_gray.tpl` (百分比/节点亲和)
  - `scripts/Gray-Warmup.ps1`
- **Test Requirements**
  - rule: `helm lint deploy/helm/mox` exit 0
  - rule: Gray-Warmup.ps1 4 阶段，每阶段 health-check 模拟 ≥ 95% 通过，exit 0
  - rule: 强制 health-check 80% 时 exit 1 + 自动回滚日志

---

## Task 13: T19 全量回归 ≥706 tests（参数化 harness）

- **Priority**: medium
- **Status**: pending
- **AC**: AC-E-19
- **Depends**: Task 7, Task 8..12
- **Outputs**:
  - `scripts/Run-T19-Regression-706.ps1`
  - `projects/t19-regression/runs/<id>/report.json`
- **Test Requirements**
  - rule: 脚本输出 Total Tests 数字 ≥ 706，Failing == 0
  - rule: report.json.suites 含 6 大项：T10-cloud、T11-graph、T17-SDK-Rust/Node/Py、E12-E20-Ops
  - rule: report.json.total ≥ 706 且 pass ≥ total - 0

---

## Task 14: Run-T17-EF-All.ps1 总控 + 终极 Rubric

- **Priority**: high
- **Status**: pending
- **AC**: 全部 AC 聚合
- **Depends**: Task 1..13
- **Outputs**:
  - `scripts/Run-T17-EF-All.ps1`（按依赖序：T17 → T12 → T13 → T15 → T18 → T19 ≥ 706 → T20）
  - `projects/t17-ef-runs/<id>/rubric-all.json`：T17(40%) + E/ops(60%)
- **Test Requirements**
  - rule: 一键脚本 exit 0
  - rubric: 综合评分 ≥ 90 Grade S，否则 A(≥80)/B(≥70)/C(≥60)/D(<60)
  - 维度权重：T17 SDK (0.40) / E12 DR (0.08) / E13 信创手册 (0.07) / E15 HA+TCO (0.10) / E18 Trace (0.07) / E19 ≥706 回归 (0.18) / E20 Helm+灰度 (0.10)
