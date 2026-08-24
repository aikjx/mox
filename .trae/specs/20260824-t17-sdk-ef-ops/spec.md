# T17 官方 SDK + E/F 批次运维落地（规格书 v1.0）

- **项目**: 玄机 XUANJI 信息图谱一体化平台
- **批次**: 批次 C (T17 官方 SDK) + 批次 E/F (运维落地 T12/T13→T15→T18→T19→T20)
- **生成日期**: 2026-08-24
- **语言**: 中文
- **总验收目标**:
  - T17 SDK：3 语言 (Rust/Node.js/Python) × (cloud + graph) = 6 个子 SDK，每子 SDK 30 示例，共 **180 示例文件**；单元/集成测试 ≥ **80**。
  - E/F 运维：T12 Helm DR → T13 信创 + 手册 → T15 HA+容量+TCO → T18 8 阶段 Trace → T19 全量回归 **≥706 tests** → T20 Helm 一键 + 灰度 1→10→50→100 warmup。
  - 全部通过后：Grade S ≥ 90 / 100，一键验收脚本可复现。

---

## 1. Problem / Users / Goals

### 1.1 问题
T10（云盘 M4）与 T11（关系图 R4）的后端能力已落地，但 **缺少对外消费这些能力的官方 SDK**，导致客户和二次开发方需裸调用 HTTP 接口。同时 **生产运维体系缺失**：无 Helm DR 容灾方案、无信创适配矩阵、无 HA 容量与 TCO 评估、无分布式 8 阶段 Trace、无 706+ 全量回归门禁、无一键 Helm 发布 + 灰度 warmup。

### 1.2 用户
- **集成开发者**：Rust/Node.js/Python 语言构建上层应用的客户工程师。
- **平台运维**：企业 SRE / DBA / K8s 管理员。
- **合规/信创团队**：三级等保 + 信创国产环境验收。

### 1.3 目标
1. 3 语言官方 SDK 覆盖云盘 IAM/STS/Lifecycle/Quota/WORM/HashChain 与 关系图 CDC/Spark/Projection/Faults。
2. Helm 双活 DR 可一键切换主备；信创 OS + CPU + 数据库适配矩阵文档化。
3. HA 3 主 3 从部署 + 容量规划（100M 顶点 / 500M 边）+ 3 年 TCO Rubric。
4. 8 阶段（Emit→CDC-Next→Dedup→Spark-Write→Projection→Audit→CB→Sink）OTel 埋点 + Zipkin 导出。
5. 全量回归矩阵 **T19 ≥ 706 tests**，覆盖 T10/T11/T17/HA/DR/SDK 组合。
6. T20 Helm 一键发布，灰度 warmup: 1% → 10% → 50% → 100%，每阶段健康检查。

### 1.4 非目标
- 不改动 T10/T11 已经验收的 Rust 核心实现（仅扩展 SDK wrap 和测试 harness）。
- 不引入真实 K8s 集群；Helm Chart 以本地 helm lint + values schema 验证。
- Python/Node.js SDK 为纯语义化 API 骨架（不做真实网络），通过同构断言 + 可执行伪客户端验证。

---

## 2. 功能需求

### 2.1 批次 C / T17 官方 SDK

#### AC-C-01 Rust xuanji-sdk-cloud（30 示例）
rule: examples/cloud/*.rs 数量 == 30，覆盖：
- 桶操作 (5): create/delete/list/head/acl-set
- 对象上传下载 (6): put/get/delete/list-prefix/copy/multipart
- STS AssumeRole (4): 申请 900s、超 900s 拒绝、session 签名校验、assume-role chain
- IAM Policy (3): put/get/evaluate_deny_first
- Quota 429 (3): rate=50/min, burst=10, retry-after header
- WORM + S3 Object Lock (3): retention 1y, legal-hold on/off, COMPLIANCE mode 不可提前删
- Lifecycle 冷热分层 (4): HOT→WARM 30d、WARM→COLD 180d、COLD→HOT 回温 1h、bucket stats
- DengBao HashChain (2): append 1k blocks & verify CLI verify

#### AC-C-02 Rust xuanji-sdk-graph（30 示例）
rule: examples/graph/*.rs 数量 == 30，覆盖：
- Flink CDC Source (7): new, next_blocking, resume(offset), 100k via Writer, dedup stats, lag monitor, consumer_id rotate
- Spark Connector (7): Reader paged nodes/edges, Writer bulk, idempotent upsert, roundtrip 2k/3k, stats accumulate
- Projection 20 (8, top12 代表式): proj_type_out_1/2, community_in_1/2, attr_out/in, degree_out_2, label_in_1 + 组合 pipeline
- AC-15 Fault (8): F1 double idempotent, F3 lost==0, F6 partial, F7 diskfull Err, F8 CB+audit, F12 timeout dedup, F13 lag, F14 audit+CB

#### AC-C-03 Node.js SDK 云盘 + 关系图（各 30 示例，共 60）
rule: platform/sdk/nodejs/examples/cloud/ 30 .js, platform/sdk/nodejs/examples/graph/ 30 .js。
每个示例是可执行脚本，打印 "XJ-OK: <name>" 到 stdout，返回 exit 0；无未捕获异常。
npm package 骨架：xuanji-sdk-cloud/package.json (name=xuanji-cloud, version=3.0.0)，xuanji-sdk-graph/package.json。
Mocha 测试：platform/sdk/nodejs/test/ 下至少 30 个 passing cases（≥80% 覆盖率 180 示例名称抽样）。

#### AC-C-04 Python SDK 云盘 + 关系图（各 30 示例，共 60）
rule: platform/sdk/python/examples/cloud/ 30 .py, platform/sdk/python/examples/graph/ 30 .py。
每个示例是 `if __name__ == "__main__": ...`，print("XJ-OK: <name>")，exit 0。
pyproject 骨架：xuanji_sdk_cloud/ + xuanji_sdk_graph/ __init__.py 暴露 Client。
pytest：platform/sdk/python/test/ 至少 30 passing cases（pytest -q 全绿）。

#### AC-C-05 SDK 跨语言示例 ID 对齐
rule: 云盘 30 主题在 Rust/Node/Python 中共享 id 集 {cloud-001..cloud-030}；
关系图 30 主题共享 id 集 {graph-001..graph-030}。
生成 projects/t17-sdk-examples/matrix.json 列出 6×30=180 示例 id × 语言 × 路径。

#### AC-C-06 SDK 总测试 ≥80
rule: `cargo test -p xuanji-sdk-cloud -p xuanji-sdk-graph --test '*'` +
`npx mocha platform/sdk/nodejs/test/` + `pytest platform/sdk/python/test/ -q` 的
**通过用例数之和 ≥ 80**，且 **0 failing**。

---

### 2.2 批次 E/F 运维落地

#### AC-E-12 Helm DR（双区域容灾）
rule: deploy/helm/xuanji-dr/ 存在 Chart.yaml + values.yaml + templates/{primary,secondary,region-Selector,service,pdb,hpa}.yaml 共 ≥ 9 个 yaml。
`helm lint deploy/helm/xuanji-dr` exit 0。values 中 `primaryRegion`, `failoverRegion`, `dr: enabled: true`。

rubric: Helm DR 成熟度 (0-100，及格 70)
- 维度：Chart 结构完整度(30) / values 可配置度(20) / 双活策略清晰(20) / PDB+HPA(15) / helm lint 通过(15)

#### AC-E-13 信创适配 + 运维手册
rule: deploy/docs/ 下产出 `xinchuang-matrix.md` 与 `ops-manual.md`。
xinchuang-matrix.md：OS (银河麒麟V10/UOS/统信) × CPU (飞腾/鲲鹏/海光/龙芯) × DB (达梦8/人大金仓 GaussDB) = **至少 36 单元格**，支持状态标注 fully/partial/planned。
ops-manual.md：13 章节（架构/部署/升级/回滚/备份/恢复/监控/告警/配额/安全/审计/容灾/FAQ）。

#### AC-E-15 HA 3 主 3 从 + 容量 + TCO Rubric
rule: deploy/docs/ha-capacity-tco.md 必须包含：
- HA 部署拓扑图：3 主 (graph/storage/iam) × 3 从跨 AZ
- 容量：100M 顶点 / 500M 边 → 内存、磁盘、网络带宽、CPU cores 数字规划
- 3 年 TCO：CAPEX（服务器×数量、单价）+ OPEX（电费、带宽、SRE 人力）= 总金额（人民币，分项合计）

rubric: HA 与 TCO (0-100，及格 70)
- 维度：HA 拓扑清晰(25) / 容量有理有据(25) / TCO 分 3 年合计(25) / 跨 AZ 说明(15) / 回滚策略(10)

#### AC-E-18 8 阶段 Trace 埋点
rule: 8 阶段常量 TraceStage {Emit, CdcNext, Dedup, SparkWrite, Projection, Audit, CircuitBreaker, Sink}。
platform/services/xuanji-graph-service/src/trace_8stages.rs：
- emit_span(stage, id, attrs) 函数，span_count_atomic 计数
- OTel-compatible JSON export：vector of {trace_id, span_id, stage, start_ms, end_ms, attrs}
- dashboard JSON：每阶段 p50/p95/p99、错误率、饱和度 (≥ 12 指标)
- Rust unit tests ≥ 8。

#### AC-E-19 全量回归 ≥706 tests
rule: 运行 scripts/Run-T19-Regression-706.ps1 或等价，打印的总用例数数字 ≥ 706，且 0 failing。
tests 覆盖矩阵：
- T10 云盘 M4: 118 tests（Rust 54 + Node 64）
- T11 关系图 R4: 126 tests（Rust 80 + Node 46）
- T17 SDK: Rust 60 + Node 30 + Python 30 = 120 tests
- HA/DR/Trace/TCO/Canary harness tests: ≥ 342 (含参数化组合)
- **合计 ≥ 706**。
输出 projects/t19-regression/report.json：{total, pass, fail, suites, duration_ms, rubric_ok}。

#### AC-E-20 Helm 一键 + 灰度 warmup
rule: deploy/helm/xuanji/ 是 "一键伞图" Chart.yaml，依赖 xuanji-dr、xuanji-core、xuanji-observability。
values.yaml 字段：global.gray.enabled, global.gray.stages: [1,10,50,100]。
`helm lint deploy/helm/xuanji` exit 0。
scripts/Gray-Warmup.ps1：4 阶段脚本，每阶段 sleep + health-check URL，任何阶段 < 95% 健康自动回滚并 exit 1。

---

## 3. 非功能需求
1. **可移植**: 所有脚本 UTF-8 无 BOM，PowerShell 5.1 兼容；所有 Rust 代码无 `unwrap()` 在非 test 生产路径。
2. **可复现**: 一键脚本 Run-T17-EF-All.ps1 每次产出新 RUN_ID，artifacts 归档 projects/t17-sdk-examples/runs/<id>、projects/t19-regression/runs/<id>。
3. **可扩展**: 示例 ID 命名规范可追加 (cloud-031+, graph-031+) 兼容。
4. **可审核**: 所有 Trace/CB/Audit 记录写入 JSONL 或 stdout 审计链。

## 4. 约束 / 依赖 / 假设
- **约束**: 不重写 T10/T11 核心逻辑；SDK 是 facade / adapter 层。
- **依赖**: Rust 1.77+ / Node 20+ / Python 3.11+ / Helm 3.13+ / PowerShell 5.1 (已在当前环境)。
- **假设**: 用户无需真实 K8s 集群；helm lint 足够。无需真实 OTel Collector；JSON export 验证。

## 5. 开放问题（已内置假设决策）
Q1: Python/Node 示例是伪客户端？→ **A1: 是，同构 API 断言 + 打印 XJ-OK，与 Rust 语义对齐。**
Q2: T19 706 tests 的参数化组合来源？→ **A2: T10/T11/T17 基础 tests × (语言 × ha_mode × dr_mode × gray_stage) 笛卡尔子集，参数化生成。**
Q3: 信创矩阵单元格内容？→ **A3: 自动化 smoke 通过 → full；部分 → partial；未测 → planned。**
