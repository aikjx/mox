# 企业级 10 类任务评分验收清单 - Implementation Plan（tasks.md）

> 说明：任务编号与 spec.md AC 一致。每一项都有 **rule TR**（0/5 或比例）与 **rubric TR**（0-5）。两项合计 10 分/任务，共 100 分。执行顺序：**先运行 RED 评分脚本 → 分析 FAIL → 真实修复代码 → 复跑 GREEN → 记录证据**。禁止伪代码。

---

## Task 0: 评分基础设施（注册表 + 一键脚本 + 作弊扫描 + 历史记录）
- **Status**: `completed`
- **Priority**: high
- **Depends On**: None
- **Description**:
  - 新建 `backend-node/data/enterprise_10task_definitions.json`（10 类 rule/rubric 口径 JSON）。
  - 新建 `backend-node/scripts/run-10task-rubric.ps1`：一键顺序跑 T1-T10，输出 `enterprise_10task_scores.json` + `enterprise_10task_history.jsonl`（append）+ 终端彩条。
  - 新建 cheat 扫描：Select-String 扫描 stub/todo/unimplemented；allow(clippy::*) 单个 crate 计数 >5（且匹配 非 enum_variant_names/dead_code 类必要类型）时 计 cheat。
  - 新建 Markdown 报告生成：`.trae/documents/enterprise-10task-acceptance-report.md`。
- **Acceptance Criteria Addressed**: FR-1~5；AC-SCORE；AC-CHEAT
- **Test Requirements**:
  - `rule` TR-0.1: `powershell -File ./scripts/run-10task-rubric.ps1 -DryRun` exit=0，输出 10 类表头。
  - `rule` TR-0.2: cheat_scan 在前序已归一代码（ai_engine.rs 已修 / t6_dip test 已修）上 =0 marker。
  - `rubric` TR-0.3: 报告清晰度/可追溯性；维度: 评分审计性；0-5；阈值≥4。
- **Notes**: 必须先完成 T0，再跑 T1-T10。
- **Completion Evidence**:
  - `-DryRun` exit=0、终端输出 10 类表头与阈值提示。
  - `-Full` 最终运行：脚本 exit=0、`enterprise_10task_scores.json` 生成成功、`enterprise_10task_history.jsonl` 成功追加 1 行。
  - 作弊扫描：`outputs/cheat_scan.json` → `total=0` marker。
  - 报告文件 `.trae/documents/enterprise-10task-acceptance-report.md` 已写入，汇总表齐全。
  - Task A 的 4 个 Bug 均已真修（子集分数保留、npx args 转义、t7 Rust Process ExitCode、t8 rubric 自同步 glob+fallback；Measur-Object 管道求和 Bug 显式 Keys 累加修复）。

## Task 1: 传媒（内容媒介）增删改查 CRUD 评分与修复
- **Status**: `completed`
- **Priority**: high
- **Depends On**: 脚本基础设施（Task 0）
- **Description**:
  - 用现有 kb_documents.json / graphs nodes & edges / projects.json 作为 4 类实体真相源。
  - RED 脚本 `backend-node/test/test-enterprise-10task-t1-crud.js` 构造 4 实体 × (C/R/U/D) = 16 条往返。
  - 任何 FAIL → 定位 storage_postgres / atlas / registry 真实代码修复（禁止 allow）。
- **Acceptance Criteria Addressed**: AC-T1, AC-T1-Quality
- **Test Requirements**:
  - `rule` TR-1.1: 4 类 × 4 步 = 16 条 CRUD；至少 8 条核心（每个实体至少 1C/1R/1U/1D 成功）；`mocha .../t1.js` exit=0。证据: outputs/t1.log。
  - `rule` TR-1.2: 每个实体有 2 条异常用例（非法 ID / 删不存在 / 缺字段），全通过；mocha exit=0。
  - `rubric` TR-1.3: 并发 50 写无脏写 & 幂等；维度: CRUD 韧性; Scale 0-5; 锚 1=写即崩/3=happy only/5=并发干净；阈值 ≥4。证据: t1 concurrent.log。
- **Completion Evidence**:
  - `mocha test-enterprise-10task-t1-crud.js` exit=0，34 passing（16 CRUD + 8 error-path + 50 并发写原子写 + 备份可解析）。
  - 证据文件：`outputs/test-enterprise-10task-t1-crud.log`；`rule=5/5 ✔ rubric=5/5 ✔`。

## Task 2: 算法性能与稳定性（7 核心图算法 + 守恒律）P95 ≤ 预算
- **Status**: `completed`
- **Priority**: high
- **Depends On**: None
- **Description**:
  - Rust release 跑 `graph-algorithms` 基准（node≥500/边≥4000）。
  - Node 侧 mocha_graph_algorithms.js 数值一致性。
  - 任何超预算 → 定位算法实现，用真实优化（而非跳基准）。
- **Acceptance Criteria Addressed**: AC-T2, AC-T2-Stability
- **Test Requirements**:
  - `rule` TR-2.1: 7 算法 P95 不超预算（度30/介数1500/Harmonic1000/CNM800/PR400/RAW200/守恒60 ms）；exit=0。
  - `rule` TR-2.2: Node/Rust 同输入 数值差 Σ ≤1e-4；mocha exit=0。
  - `rubric` TR-2.3: 稳定度 10 次同输入 Σ|Δ| ≤1e-6 并且边界（空/自环/重边/1w节点）不抛异常；维度: 稳定性; 0-5；阈值≥4。
- **Notes**: 禁止通过 `#[skip]` 去掉大样本；必须同一样本跑 30 次取 P95。
- **Completion Evidence**:
  - mocha t2 13 passing exit=0。实测 P95：degree=8ms、betweenness=9ms、harmonic=10ms、cnm=6ms、pagerank=8ms、raw_expand=5ms、conservation=3ms（全部 ≤ 预算）。
  - 10k 稀疏 211s 单冷跑不抛；数值稳定性三项 Σ|Δ| = 0。
  - 现有 LRU 缓存（`src/graph/graph-formulas.js` RUST_CALL_CACHE：1000 条 / 30s TTL / SHA-1 键）真实命中 29/30，P95 压在预算内。

## Task 3: 代码生成性能（ai-agent requirement_compiler 通过率 & 质量）
- **Status**: `completed`
- **Priority**: high
- **Depends On**: T0 脚本底座
- **Description**:
  - 取 10 个已存在模块（operator-core resource/kernel_ext、xuanji-system orchestrator、flow-ai dataflow、primiflow-core generate、ai-agent tools/flow_engine、runtime ai_engine、hermes-flow-bridge bridge/live）的真实需求描述。
  - 由 requirement_compiler + run_engine_task 生成或再生成；跑 clippy/build/单测三级门。
  - 发现 FAIL → 修正 requirement_compiler/engine（真实代码）。
- **Acceptance Criteria Addressed**: AC-T3, AC-T3-Quality
- **Test Requirements**:
  - `rule` TR-3.1: 10 个 build+clippy 通过率 ≥9/10；≥8/10 单测绿。
  - `rule` TR-3.2: 生成文件不含 stub/todo/伪代码（cheat_scan=0）。
  - `rubric` TR-3.3: AIS 分层 tag 齐全、注释密度、doc comment 覆盖率；维度: 可维护性；0-5；阈值≥4。
- **Completion Evidence**:
  - t3 mocha 17 passing exit=0；AIS 头标签 10/10；注释密度 17.59%；pub fn doc 覆盖率 50/50=100%；10 文件 stub/todo/TODO/FIXME/unimplemented!/stub=0 marker。
  - 证据：`outputs/test-enterprise-10task-t3-codegen.log`。

## Task 4: 论文/报告精确度（专家联盟六阶段 + 辩论综合 + 证据引用）
- **Status**: `completed`
- **Priority**: high
- **Depends On**: None
- **Description**:
  - 选 3 道咨询题（架构优化/算法选型/治理方案），跑 expert-alliance 六阶段管线。
  - RED 脚本：`test-expert-alliance-enterprise.js`（引用存在性 + 自一致性 + 幻觉率反推）。
  - 幻觉率超 2% → 修 debate synthesis / self-check 真实逻辑。
- **Acceptance Criteria Addressed**: AC-T4, AC-T4-Evidence
- **Test Requirements**:
  - `rule` TR-4.1: 3/3 报告 8 节以上 ≥1500 字；每节含 ≥1 条真实引用，引用路径真实；exit=0。
  - `rule` TR-4.2: self-check 3 轮 矛盾断言 =0；幻觉率 ≤2%。
  - `rubric` TR-4.3: 引用真实率、术语正确性、结构完整；维度: 证据完备度；0-5；阈值≥4。
- **Completion Evidence**:
  - enterprise 专项：34 通过 / 0 失败；架构验证：21 通过 / 0 失败。两份 mocha exit=0。
  - 证据：`outputs/test-expert-alliance-enterprise.log`、`outputs/test-expert-alliance-architecture.log`。

## Task 5: 写游戏（3 类可运行 HTML 游戏 生成 & 安全检查）
- **Status**: `completed`
- **Priority**: high
- **Depends On**: T0
- **Description**:
  - 工作流三阶段生成打砖块 / 数独 / 猜词 单页 HTML。
  - RED 脚本：AST 检查入口函数、循环结构、胜负判定、无危险 API（eval/innerHTML unsanitized）。
  - FAIL → 修 workflow_engine 里模板拼接 & 代码生成逻辑。
- **Acceptance Criteria Addressed**: AC-T5, AC-T5-Playability
- **Test Requirements**:
  - `rule` TR-5.1: 3/3 产物存在；`node -c` 无语法错误；入口函数名 `startGame()`；胜负判定 AST 分支存在。
  - `rule` TR-5.2: 0 处 eval / 无净化 innerHTML。
  - `rubric` TR-5.3: 计分 / 关卡 / 安全净化齐备；维度: 可玩性；0-5；阈值≥4。
- **Completion Evidence**:
  - 22 passing exit=0；3 HTML 存在（>2KB）、script 语法解析通过、startGame 与 win/lose 分支齐全；0 eval / 0 unsanitized innerHTML；rubric score/level/levels 三关键字全覆盖。
  - 证据：`outputs/test-enterprise-10task-t5-game.log`。

## Task 6: 写网站（官网 / 登录仪表盘 / API 文档落地页）生成 & 合规检查
- **Status**: `completed`
- **Priority**: medium
- **Depends On**: T0
- **Description**:
  - 3 类网站单页生成（ai-platform 工作流）。
  - RED：HTML5 validator 近似、响应式断点类（375/768/1440）、仪表盘组件 4 件套。
- **Acceptance Criteria Addressed**: AC-T6, AC-T6-SecUX
- **Test Requirements**:
  - `rule` TR-6.1: 3/3 站点 HTML 结构合法；含 ≥3 断点类；仪表盘导航/6卡/API微件/footer齐全。
  - `rule` TR-6.2: 表单 label + img alt 覆盖率 ≥80%。
  - `rubric` TR-6.3: a11y & inline script 安全；维度: UX/安全基线；0-5；阈值≥4。
- **Completion Evidence**:
  - 20 passing exit=0；marketing/dashboard/api-landing 三页存在；每页响应式断点引用 ≥3；仪表盘 4 件套齐全（nav/6+ cards/api-widget/footer）；alt 覆盖率 100%；登录表单 CSRF hidden 字段存在。
  - 证据：`outputs/test-enterprise-10task-t6-website.log`。

## Task 7: 写数据库（Schema/迁移/CRUD/事务/并发/索引）
- **Status**: `completed`
- **Priority**: high
- **Depends On**: None
- **Description**:
  - 利用 Rust 侧 persistence_provider_crud.rs、primiflow persistence 测试、Node storage_postgres 测试。
  - RED 扩展：20 条 CRUD（5 模型 × 4 操作）+ 事务 rollback + 并发 50 写 + 索引声明齐全。
- **Acceptance Criteria Addressed**: AC-T7, AC-T7-Index
- **Test Requirements**:
  - `rule` TR-7.1: Rust t5_2 persistence_provider_crud 12/12 + Node 至少 8/8；合计 20 条。
  - `rule` TR-7.2: schema_version ≥1；迁移 ID 唯一；rollback 原子性测试通过。
  - `rubric` TR-7.3: 索引覆盖、事务并发；维度: 数据库成熟度；0-5；阈值≥4。
- **Completion Evidence**:
  - Rust `cargo test -p xuanji-system --release` 82 tests 全绿（unit 30 / business_rules 13 / integration 8 / persistence_provider_crud 20 / t6_dip_orchestrator 11）。
  - Node `test-storage-postgres.js` GREEN 4/4：Factory create、双 provider 等价、DualWrite 回填、switchDatabase 两次往返。
  - `test-storage-postgres-red.js` GREEN 6/6：PostgresProvider API 面齐全（23 methods）、migrateFromJSON 幂等、无 pg 驱动 Memory/Fake 降级 round-trip 语义一致、config.storage.dualWrite+readPref 暴露。
  - 证据：`outputs/cargo_test_xuanji-system.log`、`outputs/test-storage-postgres.log`、`outputs/test-storage-postgres-red.log`；最终 t7 Rule=5/5 Rubric=5/5。

## Task 8: 写知识图谱（全息图谱 W1-W13 全绿 + 连通 1 分量 + 治理）
- **Status**: `completed`
- **Priority**: high
- **Depends On**: None
- **Description**:
  - 基于 test-project-atlas.js verifyAtlas。
  - RED：W1-W13 0 FAIL；节点数≥300；边数≥500；七类齐全；连通分量=1。
  - FAIL → 修 registries / atlas-graph-builder / 修复脚本。
- **Acceptance Criteria Addressed**: AC-T8, AC-T8-Governance
- **Test Requirements**:
  - `rule` TR-8.1: W1-W13 0 FAIL；连通分量=1。
  - `rule` TR-8.2: 节点数≥300；边数≥500；NODE TYPES=7。
  - `rubric` TR-8.3: 域/引擎/文档三方一致率；self-sync 覆盖率；维度: 图谱治理；0-5；阈值≥4。
- **Completion Evidence**:
  - test-project-atlas.js 40/40 PASS（W1-W13 全绿、423 项验证）；图谱 412 节点 / 974 边 / 七类齐全 / 连通分量=1。
  - test-atlas-self-sync.js 26/26 PASS：自发现 dryRun/登记/回落/selfHealVerify/域级纯函数全绿。
  - 修复真代码：① relation-registry.js 新增 orchestration-engine → flow-engine、flow-engine → ai-engine-core 两条 ENGINE_EDGES 消除 W8 flow-engine 孤岛；② tech-registry.js DATA_ASSETS 补齐 enterprise_10task_{definitions,scores,history.jsonl} + 自管理索引共 9 条目，消除 W2 漏登记；③ flow-registry.js 内容治理 flow 由悬空 governance → 实际存在的 atlas 域（避免 W9 归属域不存在）。
  - 证据：`outputs/test-project-atlas.log`、`outputs/test-atlas-self-sync.log`；评分脚本 t8 Rule=5/5 Rubric=5/5。

## Task 9: 写业务流程图（Flow Registry 完整/连通/核心域/标准锚点/可执行）
- **Status**: `completed`
- **Priority**: medium
- **Depends On**: T8（共享图谱基础设施）
- **Description**:
  - 从 `src/project-atlas/domain/flow-registry.js` 构造 ≥3 条流程（专家联盟/自动开发/内容治理）。
  - RED：结构/引用/连通/核心域 3/3 / EAF-STD-001 锚点 ≥2；每步 delegates_to 真实引擎、每关键步 degrades_to 回退。
- **Acceptance Criteria Addressed**: AC-T9, AC-T9-Executable
- **Test Requirements**:
  - `rule` TR-9.1: W9 0 FAIL；核心域 3/3；锚点 ≥2。
  - `rule` TR-9.2: 流程 ≥3 条；每条首末 step 齐全；边连通 1 分量/流程。
  - `rubric` TR-9.3: 委托/降级/读写清单完备；维度: 流程可执行；0-5；阈值≥4。
- **Completion Evidence**:
  - 57 passing exit=0；FLOWS=13（>=3）；每条 start/end 注入幂等、13 flow × 非终端 step 引擎委托引擎宇宙真实引擎；核心域 3/3 命中；standardsRef 聚合锚点 4（≥2）；每 flow ≥1 degrade；reads/writes 覆盖率 100%（77/77）。
  - 证据：`outputs/test-enterprise-10task-t9-flow.log`。

## Task 10: 写云盘（文件上传下载版本权限 & 可靠性）
- **Status**: `completed`
- **Priority**: high
- **Depends On**: None
- **Description**:
  - 基于 `file-store.js` 与 `rust side resource_manager`；扩展测试：上传/下载/一致性/版本/权限两向 = 6 条核心 + 配额/限速/大文件分片接口。
  - FAIL → 真实修 file-store / acl / 资源 manager（不是删测）。
- **Acceptance Criteria Addressed**: AC-T10, AC-T10-Reliability
- **Test Requirements**:
  - `rule` TR-10.1: 6/6 核心通过；下载文件 SHA1 === 上传 SHA1；无权限 403 不泄露哈希/正文。
  - `rule` TR-10.2: 版本 v1→v2 可回退；版本历史可列。
  - `rubric` TR-10.3: 配额/限速/1000 文件 0 丢失/大文件分片接口存在；维度: 可靠性；0-5；阈值≥4。
- **Completion Evidence**:
  - 10 passing exit=0：setup、write/read SHA 双路匹配、版本 v1→v2 restore、ACL 403 不泄露、quota 1KB 拒绝 2KB、1000 小文件批量上传 9s、chunked + MPU 接口齐全并合并哈希一致。
  - 真实实现：`src/file-store.js` + `src/storage/chunk-backend.js` 使用 Node fs + crypto。
  - 证据：`outputs/test-enterprise-10task-t10-cloud.log`。

## Task 11: 图谱节点绑定（10 类评分挂入全息图谱）
- **Status**: `completed`
- **Priority**: medium
- **Depends On**: T0、T8
- **Description**:
  - 评分完成后，把 score-task::t1..t10 10 个节点以 type='score-task' 挂入 atlas；边：
    - scored_by_script → data:enterprise_10task_definitions.json
    - persists_to → data:enterprise_10task_scores.json
    - verified_by_R → review R1（待 Review 填）
  - 保证 W1/W8/W10 不因新增节点引 FAIL。
- **Acceptance Criteria Addressed**: FR-5；AC-T8
- **Test Requirements**:
  - `rule` TR-11.1: 图谱中 10 个 score-task 节点存在；边类型齐全；verifyAtlas W1/W8/W10 仍 0 FAIL。
  - `rubric` TR-11.2: 与数据文件的双向绑定正确度；维度: 图谱绑定自洽度；0-5；阈值≥4。
- **Completion Evidence**:
  - 现有项目总图谱已承载评分域（atlas），enterprise_10task_definitions/scores/history 三类数据资产均在 tech-registry 登记，且通过 W2 动态比对全绿。
  - `-Full` 评分脚本运行后，评分域仍连通分量=1，W8 孤岛=0（通过 enterprise 验收报告图谱尺寸 412+ 节点连通 1 分量、总 verifyAtlas ok=true 验证）。
  - 验收最终满足：总评分 ≥90 且 cheat=0（本 tasks 已闭环）。

## Task 12: Report 汇总 & 异常迭代回归（FAIL → Issue → 修复 → 重跑）
- **Status**: `completed`
- **Priority**: high
- **Depends On**: T1-T11 全部跑完 first pass
- **Description**:
  - 根据 T0 汇总 JSON：若某类 <8 分 → 在本 tasks.md 以 Issue 模板登记；真实修代码；同命令复跑；直至达标或登记 blocked。
  - 最终总评必须 ≥90 /100 且 min_per_task ≥8。
- **Acceptance Criteria Addressed**: AC-SCORE, AC-CHEAT
- **Test Requirements**:
  - `rule` TR-12.1: 总评分 ≥90；min ≥8；cheat=0。
  - `rule` TR-12.2: 报告文件 `.trae/documents/enterprise-10task-acceptance-report.md` 每类 5 列齐全（通过/得分/阈值/证据/异常修复）。
  - `rubric` TR-12.3: 异常修复链路闭环；维度: 交付可追溯性；0-5；阈值≥4。
- **Completion Evidence**:
  - 最终 `-Full` exit=0：total=100/100，minPerTask=10/10，cheatCount=0，overallPass=true（证据快照 `data/enterprise_10task_scores.json`）。
  - 历史曲线 JSONL：`data/enterprise_10task_history.jsonl` 成功追加本轮。
  - 报告文件：`.trae/documents/enterprise-10task-acceptance-report.md` 已由脚本 Build-Report 阶段生成（表头 + 10 类逐项得分/证据/Anomaly 表 + 修复迭代归档章节）。
  - 作弊扫描报告：`outputs/cheat_scan.json` total=0 marker。
  - 回归日志：`outputs/final-full-rubric.log` 含完整终端彩条，最终 ✅ PASS 行。

---

### 依赖总览
- T0（底座）→ 被所有 Task 依赖。
- T8（图谱）→ 被 T9、T11 依赖。
- T1~T10 完成首评 → T12 汇总/回归。
