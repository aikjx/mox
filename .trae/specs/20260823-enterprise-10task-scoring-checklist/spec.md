# 企业级 10 类任务评分验收清单 - 产品需求规格（spec.md）

## Overview
- **Summary**: 为璇玑知识图谱驱动的 10 类企业级任务，建立「先写 RED 评分脚本 → 定位 FAIL → 修代码 → 重跑直至达标」的闭环评分验收体系；覆盖：传媒 CRUD、算法性能、代码生成性能、论文精确度、游戏、网站、数据库、知识图谱、业务流程图、云盘。
- **Purpose**: 把用户提出的「全部功能完完整整企业级测试清单，一个个打勾评分，有异常就一个个修复，不断迭代」落到单一真相源的 spec + tasks + review，并产出「总评分 ≥ 90/100，单项 ≥ 8/10」的企业级交付证据链。
- **Target Users**: 平台交付负责人 / 架构评审 / 质量 QA / 最终用户验收。

## Goals
- G-1: 10 类任务每类 10 分（rule 5pt + rubric 5pt，合计 100 分）。
- G-2: 企业级准入：总评分 ≥ 90；单项 ≥ 8（rule 满分5 → 5，rubric ≥3 → 3）。
- G-3: 每项评分必须来自真实可复现的脚本（scripts/enterprise-10task-rubric.ps1 一键跑），禁止主观打勾。
- G-4: 所有 FAIL 项 → 形成 Issue → 定位根因 → 真实代码修复 → 重跑通过（无伪代码、无 allow 遮罩）。
- G-5: 保留璇玑知识图谱的唯一底层中枢：评分节点挂入 atlas graph，验证结果写回 data/enterprise_10task_scores.json。

## Non-Goals
- 不新增 10 类任务之外的业务线。
- 不做 UI 层面的打分前端（仅脚本+报告）。
- 不引入第三方打分 SaaS。
- 不改变现有 Rust/Node 双端 API 契约（评分脚本作为旁路 consumers，非 API 侵入）。

## Background & Context
- 项目已通过前序：7-gate（Rust build/clippy/test + Node 5 tests）全 exit=0；归一化（图算法/意图/模板单一真源）、W1-W13 全息图谱破窗 0 FAIL。
- 用户原话：「有10个不同任务，一个个的测试……传媒的增删改查啊，然后那些算法的性能啊，包括写代码的那个性能啊，写论文的精那个精确度啊，写那个游戏的，写网站的，写数据库的，写那个知识图谱的，写业务流程图的啊，写那些云盘的，就全部功能，要完完整整的企业级的一个测试的一个清单，一个个个打勾评分，有异常的就一个个的修复，不断的去迭代」。
- 依据 Experience 1281395：Windows 环境统一用 Select-String；避免补丁堆叠；脚本入口归一。
- 依据 Experience 1527742：workflow 输入输出「永不 None」；UI 关键组件「不可降级」；评分透明化、可溯源。

## Functional Requirements
1. **FR-1 评分注册表**: 在 `backend-node/data/enterprise_10task_definitions.json` 固化 10 类任务定义（id/name/ruleEvidence/rubricScale/0-5 anchors/passThreshold/scoreWeight）。
2. **FR-2 RED 评分脚本**: `backend-node/scripts/run-10task-rubric.ps1` 顺序执行 10 类评分；每类先跑 RED → 产出 FAIL 明细；自动定位根因候选（文件/模块/函数）。
3. **FR-3 GREEN 修复流水线**: 每个 FAIL → 在 `tasks.md` 内自动/手动注册 Issue；修复后同一评分命令复跑；保留历史评分曲线（`enterprise_10task_history.jsonl`，一行一次完整 10 类评分）。
4. **FR-4 总评看板与报告**: 输出 Markdown 企业级验收报告 `.trae/documents/enterprise-10task-acceptance-report.md`：每类 5 维（通过/得分/阈值/证据路径/异常修复）。
5. **FR-5 图谱绑定**: 每类评分任务以 `score-task::<id>` 节点挂入全息图谱，边：`scored_by_script` / `persists_to → data:enterprise_10task_scores.json` / `verified_by_R=<N>`。

## Non-Functional Requirements
1. **NFR-1 可复现**: 同一 commit 上同平台同命令两次评分差 ≤ 1 分。
2. **NFR-2 全绿耗时（不含 Rust 测试）**: ≤ 25 分钟（Node 侧 + 静态扫描 + 轻量性能样本）；包含 Rust 测试 ≤ 90 分钟。
3. **NFR-3 零伪代码**: 任何用于凑分的 stub/todo/allow 遮罩，计 0 分并标注 cheat=1。
4. **NFR-4 审计性**: 每次评分写入 4 元组不可变证据：timestamp/commit/runner/scoreSnapshot SHA256。
5. **NFR-5 中文**: 所有评分规则、报告、错误提示用中文输出。

## Constraints
- **技术**: Windows PowerShell 7+；Node ≥ 22；Rust 1.80+（workspace）；不得新增 npm/cargo 依赖（复用现有 fs/path/child_process/json-store）。
- **业务**: 单项 ≤7 分视为失败；总评 <90 时，自动回到 Implement 阶段，不进入 Review。
- **依赖**: 需 data/、src/project-atlas、src/graph、src/expert-alliance、Rust workspace 15 crate 可用。

## Assumptions
- A-1: Rust 侧算法性能样本以 release 模式运行 30 次取 P50/P95/P99，统计量足够（避免 warmup 偏差）。
- A-2: 「写代码/论文/游戏/网站/数据库/知识图谱/业务流程图/云盘」均有对应 Node/Rust 工作流或业务域作为评分对象，无需新建无中生有的产品线。
- A-3: 传媒 CRUD 指「内容媒介（kb_documents.json + graph nodes/edges + workflows）」的增删改查；如域不对齐，以现有路由 + 存储接口为真相。

## Open Questions
- [ ] Q1: 用户是否要求 10 类得分全部 ≥9/10 还是仅总评 ≥90？默认规则：总评 ≥90 且单项 ≥8（如有必要，后续可按用户要求升级为单项 ≥9）。
- [ ] Q2: 「写代码性能」的评测基准题集：以 10 个真实算子/路由/算法实现为输入？默认：用 Rust workspace 里 10 个已有模块（operator-core/flow-ai/…）做「时间 + AIS 分层合规 + 测试数 + clippy pass」四维综合。

---

## Acceptance Criteria（10 类 × rule 5pt + rubric 5pt = 100 分）

### AC-T1: 传媒（内容媒介）增删改查 功能正确
- **Type**: `rule`
- **Given**: 后端服务数据目录完整；kb/graph/projects 三张表（JSON 文件或 SQLite）初始化；评分脚本对 4 类实体（kb document、graph node、graph edge、project）各 2 组样本数据。
- **When**: 执行 `Invoke-EnterpriseRubric -Task t1_crud`（脚本内部：构造 create → read → update → delete → read 四步往返 + 4 类实体）。
- **Then**: 8 条往返（4 类 × C/R/U/D）全部通过；无数据损坏；幂等写入无重复键；删除后 read 返回空/404。
- **Pass Condition**: 8/8 CRUD 全部成功；并且「空库读」与「写后读」ID/字段一致性 100%。
- **Evidence**: `outputs/t1_crud.log` + `data/enterprise_10task_scores.json::t1.rule.pass=true`。
- **分值贡献 (满分 5)**: 全部 PASS → 5 分；任何 1 条 FAIL → 0 分（无半分）。

### AC-T1-Quality: 传媒 CRUD 质量与韧性（rubric 5pt）
- **Type**: `rubric`
- **Dimension**: CRUD 韧性 · 错误隔离 · 异常分支正确性
- **Scale**: 0-5
- **Anchors**: 1 = 仅 happy path，任何异常直接抛；3 = 主要异常（字段缺失 / 非法 ID / 删不存在）被捕获并返回结构化错误，测试有 1 条异常用例；5 = 每类实体都有 ≥2 条异常用例，并发写 50 次无脏写，读写一致性经 100 次。
- **Pass Threshold**: ≥4（企业准入）
- **Evidence**: `test-storage-postgres.js` 或同等 CRUD 测试中 异常用例数 + 并发测试结果。
- **分值贡献 (满分 5)**: score×1（5 →5；4→4；3→3 不及格直接单项不达标）。

### AC-T2: 算法性能（7 核心图算法 + 守恒律）P95 预算通过
- **Type**: `rule`
- **Given**: release 模式；`graph-algorithms/compare_with_node` 及 `t9_deep_chain_p99 / graph algorithm 基准`。
- **When**: 运行 `cargo test -p graph-algorithms --release benchmark 样本` + Node 侧 `mocha_graph_algorithms.js`。
- **Then**: 每类算法在各自基准规模（节点≥500/边≥4000）下 P95 不得超过各自预设预算：度中心性 ≤30ms；介数 Brandes ≤1500ms；Harmonic 紧密 ≤1000ms；CNM 模块度 ≤800ms；Pagerank/PPR ≤400ms；RAW_EXPAND ≤200ms；守恒律 ≤60ms。
- **Pass Condition**: 全部 7 项 P95 不超预算，且相对 Rust 基线 Node wrapper 的数值误差 ≤1e-4（co_impl 允许）。
- **Evidence**: `outputs/t2_algo_perf.json`；score=（预算满足项数/7）×5。
- **分值贡献 (满分 5)**: 全部满足 →5；任何 1 项超预算 →减 1 分，最低 0。

### AC-T2-Stability: 算法稳定性与数值一致性（rubric 5pt）
- **Type**: `rubric`
- **Dimension**: 算法 数值一致性 / 无波动 / 鲁棒性
- **Scale**: 0-5
- **Anchors**: 1 = 不同 seed 下跑 10 次数值有明显抖动；3 = 数值稳定但空图/孤立节点有边界报错；5 = 空图/自环/重边/大规模（1w 节点）均稳定，10 次同输入误差绝对值 Σ ≤1e-6。
- **Pass Threshold**: ≥4
- **Evidence**: `test-algo-rust-node-diff.js` 同输入 10 次稳定报告。
- **分值贡献**: score×1。

### AC-T3: 代码生成性能（AIS 分层合规 + 产出速度 + 代码质量）
- **Type**: `rule`
- **Given**: 10 个企业级代码模块算子/路由/算法/工具作为需求题（与 Rust/Node 现有模块一一对应）。
- **When**: 走 `ai-agent/requirement_compiler` 生成代码；再走 clippy/Rust build/Node lint 三级门。
- **Then**: 10 个中 ≥9 个通过 build + clippy（-D warnings）+ 至少 1 个自己的单元测试 绿。
- **Pass Condition**: build/clippy 通过率 ≥90%；测试绿率 ≥80%。
- **Evidence**: `outputs/t3_codegen.log` 与评分快照。
- **分值贡献**: 通过 →5；否则按比例 ×5。

### AC-T3-Quality: 代码质量（可读性 + AIS 标签 + 注释密度）（rubric 5pt）
- **Type**: `rubric`
- **Dimension**: 代码可维护性 · AIS 分层合规 · 注释覆盖率
- **Scale**: 0-5
- **Anchors**: 1 = 无 AIS 分层 tag、注释 <5%；3 = 有分层 tag、注释 10%；5 = 分层全量、注释 ≥15%、公共函数 100% doc comment。
- **Pass Threshold**: ≥4
- **Evidence**: 静态扫描脚本（Select-String + clippy `missing_docs` if any）输出。
- **分值贡献**: score×1。

### AC-T4: 写论文/报告精确度（专家联盟六阶段 + 辩论综合）
- **Type**: `rule`
- **Given**: 3 道企业级咨询题（架构优化/算法选型/治理方案），每道需输出含 ≥8 节、≥1500 字 的结构化报告，并附可溯源引用。
- **When**: 跑 `expert-alliance` 六阶段管线（EAF-STD-001），最终报告经「证据引用存在性校验 + 前后一致性校验 + 幻觉反推检查（3 轮 self-check）」。
- **Then**: 3 份报告全部通过「引用可定位」校验；内部自相矛盾断言数 = 0；幻觉（无引用断言） ≤2%。
- **Pass Condition**: 3/3 报告通过；幻觉率 ≤2%。
- **Evidence**: `outputs/t4_thesis_accuracy.json`。
- **分值贡献 (满分 5)**: 3/3 + 幻觉≤2% →5；2/3 →3；其余 0。

### AC-T4-Evidence: 证据链完备度（rubric 5pt）
- **Type**: `rubric`
- **Dimension**: 报告 引用真实率 · 溯源深度 · 领域术语正确性
- **Scale**: 0-5
- **Anchors**: 1 = 大部分断言无引用；3 = 关键断言有引用，引用域/文档路径真实；5 = 每个「数字/结论/架构」断言对应 ≥1 条本地图谱节点或文档引用；术语错用率 ≤1‰。
- **Pass Threshold**: ≥4
- **Evidence**: test-expert-alliance-enterprise.js 的 「debate correctness」 指标输出。
- **分值贡献**: score×1。

### AC-T5: 写游戏（互动游戏策划 + 前端互动脚本生成 + 可运行性）
- **Type**: `rule`
- **Given**: 3 类小游戏（打砖块/数独/猜词）策划案 + 可运行 HTML/CSS/JS 产物规格。
- **When**: 走 `workflow-engine.js 三阶段` 生成可运行单页 HTML；再经静态可运行检查（入口函数存在、无 SyntaxError、canvas/dom API 合规）。
- **Then**: 3/3 产物通过「静态可运行检查」+ 至少 1 个游戏循环存在 + 基本胜负判定逻辑存在（AST 判定）。
- **Pass Condition**: 3/3 通过。
- **Evidence**: `outputs/t5_game_artifacts/` 下 3 个 HTML + 生成 log。
- **分值贡献 (满分 5)**: 3/3 →5；2/3 →3；其余 0。

### AC-T5-Playability: 游戏可玩性与代码安全（rubric 5pt）
- **Type**: `rubric`
- **Dimension**: 可玩性 · 代码无危险 eval/无限循环 · 渲染帧率合理
- **Scale**: 0-5
- **Anchors**: 1 = 存在 eval/innerHTML 无净化 或 无限循环；3 = 无危险 API，有胜负判定但无计分/关卡；5 = 计分+关卡+异常输入安全，FPS 在浏览器预期范围内（可通过代码结构推断）。
- **Pass Threshold**: ≥4
- **Evidence**: ESLint 等价危险 API 扫描 + AST 可达性分析结果。
- **分值贡献**: score×1。

### AC-T6: 写网站（企业官网/门户/登录仪表盘）
- **Type**: `rule`
- **Given**: 3 类网站产物（官网 / 登录仪表盘 / API 文档落地页）。
- **When**: 走 `ai-platform` 工作流生成单页 HTML + 配套 CSS/JS；再跑 HTML5 合规性检查 + 响应式断点检查（375/768/1440）。
- **Then**: 3/3 网站 HTML 合规；至少含 ≥3 个响应式断点类；登录仪表盘必须包含：导航 / 数据卡片 ≥6 / API 状态微件 / footer。
- **Pass Condition**: 3/3 通过 且 仪表盘 4 子组件齐全。
- **Evidence**: `outputs/t6_site_artifacts/` 截图/结构快照。
- **分值贡献**: 3/3 + 组件齐 →5；3/3 但缺组件 →3；其余 0。

### AC-T6-SecUX: 网站 UX & 安全基线（rubric 5pt）
- **Type**: `rubric`
- **Dimension**: UX 可访问性 · 安全基线（CSP/无 inline 危险脚本/XSS 入口）
- **Scale**: 0-5
- **Anchors**: 1 = 无 alt 属性、表单无 label、inline script 达 ≥10 处；3 = a11y 基础具备、少量 inline；5 = Lighthouse a11y ≥90 分等价条件、无危险 inline，登录表单带 CSRF token 字段/结构。
- **Pass Threshold**: ≥4
- **Evidence**: Select-String 扫描 inline/a11y 属性输出。
- **分值贡献**: score×1。

### AC-T7: 写数据库（Schema 设计 + 迁移 + CRUD 往返）
- **Type**: `rule`
- **Given**: Rust sqlite 与 Node storage 两套实现（`mox-system/tests/persistence_provider_crud.rs`、`primiflow-core/tests/mock_persistence.rs`、`backend-node/test-storage-postgres.js`）。
- **When**: 统一走 5 模型（Member/Document/Resource/Task/Notification）×4 CRUD = 20 条，分别跑 SQLite / Mock（模拟多后端）。
- **Then**: 20/20 CRUD 通过；schema 版本 ≥ 1；迁移 ID 不重复。
- **Pass Condition**: 20/20 通过（至少 SQLite 与 Mock；有 Postgres 环境时额外 bonus 不扣）。
- **Evidence**: t5_2 persistence test 输出。
- **分值贡献**: 20/20 →5；18-19 →3；<18→0。

### AC-T7-Index: 数据库索引/事务/一致性质量（rubric 5pt）
- **Type**: `rubric`
- **Dimension**: 索引覆盖 · 事务原子性 · 并发写一致性
- **Scale**: 0-5
- **Anchors**: 1 = 无索引设计、直接全表扫/无事务；3 = 关键查询主键索引，单线程无脏写；5 = 外键/索引齐全，并发 50 写事务全成功，死锁 0，rollback 语义正确。
- **Pass Threshold**: ≥4
- **Evidence**: storage tests 并发用例 + 事务 rollback 用例输出。
- **分值贡献**: score×1。

### AC-T8: 写知识图谱（全息图谱正确性 + 连通 + 归一化 + 自洽）
- **Type**: `rule`
- **Given**: Node 全息图谱（domain/module/engine/algorithm/data/doc/project 七类节点 + 11 种边）。
- **When**: 跑 `verifyAtlas()` 与 connectedComponents。
- **Then**: W1-W13 全规则通过 = 0 FAIL；七类节点齐全；连通分量 = 1；边数 ≥ 500；节点数 ≥ 300。
- **Pass Condition**: W1-W13 0 FAIL + 连通 1 分量。
- **Evidence**: verifyAtlas 输出。
- **分值贡献**: 全满足 →5；任何 1 条 W fail → 3（≤2 fails），否则 0。

### AC-T8-Governance: 图谱治理（无破窗 + 自同步 + 自闭环度量）（rubric 5pt）
- **Type**: `rubric`
- **Dimension**: 图谱治理成熟度 · 自同步自愈 · 资产覆盖率
- **Scale**: 0-5
- **Anchors**: 1 = 域/引擎引用有大量悬空；3 = 引用基本无悬空，自同步能发现 50% 新增域；5 = 引用 100% 真实；self-sync 发现新资产 100% 并回填；域/资产/文档三方一致率 ≥99%。
- **Pass Threshold**: ≥4
- **Evidence**: test-atlas-self-sync.js 与归一化校验输出。
- **分值贡献**: score×1。

### AC-T9: 写业务流程图（Flow Registry 结构完整 + 连通 + 核心域覆盖 + 标准锚点）
- **Type**: `rule`
- **Given**: FLOWS 注册（项目全息图谱 W9 业务流程族），≥ 3 条完整流程，至少覆盖 专家联盟 / 自动开发 / 内容治理 三个核心域。
- **When**: 跑 flow-validator 与 W9 验证（结构/引用完整/连通/核心域覆盖/标准锚点）。
- **Then**: 流程结构合法（≥3 steps / 首末 step）、边连通 1 分量 per 流程、核心域覆盖 3/3、EAF-STD-001 等 标准锚点 ≥2。
- **Pass Condition**: W9 0 FAIL；覆盖率达标。
- **Evidence**: W9 check + flow-validator。
- **分值贡献**: 全满足 →5；仅 1 子项 FAIL →3；否则 0。

### AC-T9-Executable: 业务流程可执行 & 降级链完整（rubric 5pt）
- **Type**: `rubric`
- **Dimension**: 流程 可执行性 · degrades_to 降级链完整性 · 步骤委托引擎真实
- **Scale**: 0-5
- **Anchors**: 1 = 流程仅静态声明，无委托/降级；3 = 主链路有 delegates_to 引擎引用真实；degrades_to 至少 1 条；5 = 每步至少 delegates_to 1 真实引擎；每关键步都有 degrades_to 回退 + reads/writes 数据清单可解析。
- **Pass Threshold**: ≥4
- **Evidence**: flow-validator delegates/degrades 统计。
- **分值贡献**: score×1。

### AC-T10: 写云盘（文件上传下载版本权限）
- **Type**: `rule`
- **Given**: file-store.js 本地存储 + 版本号 + 权限读；ai-agent resource_manager（Rust 侧也有资源管理）。
- **When**: 执行 上传 → 下载 → 内容比对（SHA1 同）；版本递增（v1 → v2）；权限控制（无权限→403，有权限→OK）；共 6 条：上传/下载/一致性/版本/权限无读/权限有读。
- **Then**: 6/6 全通过；无权限不得泄露文件内容哈希或正文。
- **Pass Condition**: 6/6。
- **Evidence**: file_store 6 条测试输出（test-filestore-red.js / s3 or local）。
- **分值贡献**: 6/6 →5；4-5 →3；否则 0。

### AC-T10-Reliability: 云盘可靠性 & 资源配额（rubric 5pt）
- **Type**: `rubric`
- **Dimension**: 可靠性 · 配额/限速 · 大文件分片/断点续传结构
- **Scale**: 0-5
- **Anchors**: 1 = 无限速/无配额，写 10GB 直接写崩；3 = 有基础配额 + 小文件可靠；5 = 配额超限严格拒绝，连续写 1000 个文件不丢，≥10MB 文件分片/断点结构存在（或接口声明）。
- **Pass Threshold**: ≥4
- **Evidence**: file_store 配额用例与资源管理器审计日志。
- **分值贡献**: score×1。

---

### AC-汇总 & 总评 阈值
### AC-SCORE: 10 类加权总分 ≥ 90/100（rule）
- **Type**: `rule`
- **Given**: 10 类各 10 分（rule 5 + rubric 5）共 100 分；企业级准入规则：单项 ≥ 8 分。
- **When**: 运行 `run-10task-rubric.ps1 -Full`。
- **Then**: 总评分 ≥90 并且 每类得分 ≥8。
- **Pass Condition**: total ≥90 AND min_per_task ≥8。
- **Evidence**: enterprise-10task-acceptance-report.md。

### AC-CHEAT: 零伪代码零 allow 遮罩作弊（rule）
- **Type**: `rule`
- **Given**: 代码库扫描 cheat 特征：stub/伪代码字符串/空函数/无意义的 allow(clippy::*) 全量使用。
- **When**: 扫描脚本：`Select-String -Pattern '\[stub\]|todo!\(\)|unimplemented!\(\)' platform -Recurse -Include *.rs,*.js`；同时对 allow(clippy) 统计：单个 crate >5 个非必要 allow → 计 1 cheat。
- **Then**: cheat count = 0。
- **Pass Condition**: 0 cheat marker。
- **Evidence**: cheat_scan.log。
