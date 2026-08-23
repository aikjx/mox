# backend-node · 璇玑 JS 子系统（Node.js 后端 + AI 引擎 + 路由）

backend-node 是璇玑平台的 Node 侧综合后端：承担 AI 引擎路由（ai-engine / ai-engine-core / ai-integration-engine / ai-ultimate-engine / ultimate-ai-engine）、专家联盟（expert-alliance）、图谱与知识库（graph / kb / kg-hub 接入）、算子/插件（engine-kernel / engine-universe / mcp / modules / plugins）、项目图谱（project-atlas）、业务服务（orchestration / browser-market / storage / kb / tasks / …）、以及统一 API 服务（server / api-server / routes/*）。同时提供数据层 JSON 文件持久化（`data/*.json`）、脚本（`scripts/*`，包括企业级 C3 单一真源看门狗 `validate_no_duplicate_functions.js`）、以及 E2E/回归测试（`test/*`、`tests/*`）。

> 本目录虽非 Rust workspace crate（Rust 侧 16 大 crates 见 `platform/Cargo.toml` members），但作为 JS 侧「C3 函数族单一真源」承载目录，必须同样具备企业级归一化 SSoT 声明，与 Rust 侧 `ai-agent/flow_engine.rs` + `graph-algorithms/lib.rs` 形成双语言真源对。

---

## 企业级归一化 C3 单一真源声明 SINGLE SOURCE OF TRUTH (SSoT)

### 1. 本目录在 C3 函数族归一化体系中扮演的角色（5 条 JS 族清单）

JS 侧 C3 归一化清单共 5 个函数族；每族在此目录下的 SSoT 文件 / wrapper 文件对（与 watchdog REGISTRY 保持一致）如下：

| # | C3 族 | 角色分类 | 真源文件（独立算法实现唯一允许处） | Wrapper 文件（仅转发，体 ≤ 4 行） | 真源算法内容摘要 |
|---|-------|----------|------------------------------------|-----------------------------------|------------------|
| 1 | `degreeCentrality`（度中心性） | 【单一真源】 + 【thin wrapper】×2 | `src/graph/graph-formulas.js` → `GraphFormulas.degreeCentrality(nodes, edges, {expandRaw, legacyShape})`（第 40 行起，`(inDeg+outDeg)/(N-1)` 归一；支持 flat `{id:number}` 与 legacy `{id:{degree,...}}` 双 shape） | ① `src/lib/graph-algos.js` → `function degreeCentrality(nodes, edges)`（只转发 `GF.degreeCentrality(..., {expandRaw:true, legacyShape:true})`）；② `src/ai-flow-graph.js` → `AIFlowGraph.degreeCentrality(nodes, edges)`（只转发 `SrcGF.degreeCentrality(..., {expandRaw:true})`） | 真实 forEach 节点/边计数 + 除 (N-1) 归一 |
| 2 | `betweennessCentrality`（介数中心性 Brandes） | 【单一真源】 + 【thin wrapper】×2 | `src/graph/graph-formulas.js` → `GraphFormulas.betweennessCentrality(nodes, edges, {directed})`（第 85 行起，Brandes 2001：σ/P/δ 三组数组 + 栈逆序回传 Σ） | ① `src/lib/graph-algos.js` → `function betweennessCentrality(nodes, edges, opts)`（只转发 `GF.betweennessCentrality(..., opts||{directed:false})`）；② `src/ai-flow-graph.js` → `AIFlowGraph.betweennessCentrality(nodes, edges, {directed})`（只转发 `SrcGF.betweennessCentrality(...)`） | 真实 Brandes 双 for 循环 + 数组栈，指纹 `sigma[w] / preds[w] / dist[w]` |
| 3 | `pagerank`（PageRank 迭代 + 悬挂节点处理） | 【单一真源】 + 【thin wrapper】×2 | `src/graph/graph-formulas.js` → `GraphFormulas.pagerank(nodes, edges, {dampingFactor, maxIterations})`（第 589 行起）以及 `GraphFormulas.pagerankWithTranspose(...)`（第 234 行起，带转置图对照 + 悬挂质量均匀回传；与 Rust `graph-algorithms/src/lib.rs::pagerank_personalized` 做 T12 双语言对账） | ① `src/lib/graph-algos.js` → `function pagerank(nodes, edges, damping, maxIter)`（只转发 `GF.pagerank(..., {dampingFactor, maxIterations})`）；② `src/ai-flow-graph.js` → `AIFlowGraph.pagerank(nodes, edges, opts)`（只转发 `SrcGF.pagerank(nodes, edges, opts||{})`） | 真实 for 迭代 + dangling mass 回传，Σ≈1 归一 |
| 4 | `detectIntent`（意图分类打分 / 关键词匹配） | 【单一真源】 + 【thin wrapper】×2 | `src/expert-alliance/domain/intent-classifier.js` → `function detectIntent(question)`（第 41 行起；关键词先验表 `intent-patterns.js`；中英文双语 + 多词短语加权 + 置信度归一；导出 `{detectIntent, keywordMatches}`） | ① `src/ai-engine-core.js` → `AIEngineCore.detectIntent(question)`（只 `const r = _domainDetectIntent(question)`，再拼 shape `{intent, score, scores, matched_keywords, method}`）；② `src/ai-integration-engine.js` → `_detectIntention(question)`（方法名别名，只转发 domain 层 `detectIntent`） | 真实关键词遍历 + 归一化打分；`allScores / matchedKeywords / primary` 三元输出 |
| 5 | `apply_template`（变量模板替换） | 【JS 侧不声明独立 SSoT · 对应 Rust SSoT】 | 单一真源只在 Rust 侧：`../services/ai-agent/src/flow_engine.rs::pub fn apply_template`（第 565 行；`{{k}}` 占位符替换 + 缺失保留） | Wrapper 只在 Rust 侧：`../services/ai-agent/src/workflow_engine.rs::fn apply_template`（先 `${k}` → `{{k}}` 桥接，再转发 flow_engine） | JS 侧若后续引入模板替换族，必须在 watchdog REGISTRY 登记并指向上述 Rust SSoT；禁止在 Node 端独立实现 `{{k}}`/`${k}` 替换算法本体 |

#### 特别说明：已知的历史实现位置（属于企业 C3 治理观察对象）
- `src/nebulagraph-adapter.js::NebulaGraphAdapter.pagerank(dampingFactor, maxIterations, tolerance)`（第 537 行起）：该类为「NebulaGraph 远程图适配器本地回退模拟器」，含一份独立 PageRank 循环实现。它不是 C3 真源；已登记为「适配器级本地模拟器」，调用场景受 `remote?.connected` 条件约束。后续若图 4 算法家族扩展 REGISTRY 覆盖到 Nebula 适配器，应转为 wrapper 并转发 `GraphFormulas.pagerank`，禁止继续独立维护。
- `src/ai-engine.js::_computePageRank` 与 `_computeCentrality`：已按 C3 归一化路径转发到 `AIIntegrationEngine.graphEngine.computePersonalizedPageRank` / `GraphFormulas.*` 真源（体 ≤ 4 行有效代码），属于合规 shape 层。
- `src/expert-alliance-engine.js::classifyIntent` / `src/orchestration-engine.js::_detectIntent`：已声明在 watchdog 扫描路径；未命中 detectIntent 函数头白名单时不得出现独立关键词打分循环，否则同样被 TR-7.3「非注册重复开发」拦截。

### 2. 重复开发违规定义
重写/重实现单一真源中已存在的同名函数（包括但不限于：在 `routes/*.js`、`services` 文件、`*.engine.js` 里再写一份 PageRank / Brandes 介数 / degree / apply_template / detectIntent 的独立 for/forEach 循环 + if-else if 链 + Map/Set 构造，或体 ≥15 行有效代码），视为 C3 违规，会被 `scripts/validate_no_duplicate_functions.js` 看门狗拦截（`exit 1`）。看门狗的判定算法是 `isLikelyIndependentImplementation()`：只要函数体（去注释去空行后）出现 `for/while/forEach` 循环、`else if` 链、≥4 次方法调用、`Map/Set/Math.*` 构造、或总有效行数 ≥15，即视为独立实现违规。Wrapper 的硬约束上限是 **MAX_WRAPPER_LINES=4 行有效代码**。

### 3. 合规更新流程
如需修改 `apply_template` / 图 4 算法 / 意图识别 detectIntent，请**只改 SSoT 文件**，wrapper 文件保持 4 行以内：

- 修改 `degreeCentrality / betweennessCentrality / pagerank` → 只改 `src/graph/graph-formulas.js` 的 `GraphFormulas.*` 方法体；Rust 侧同步改 `../services/graph-algorithms/src/lib.rs` 对应方法，再跑 `test/test-t12-algorithm-reconcile.js` 做双语言数值对账（误差 < 1e-6）。
- 修改 `detectIntent` → 只改 `src/expert-alliance/domain/intent-classifier.js`（与 `intent-patterns.js` 关键词先验表）；所有 wrapper（`ai-engine-core.detectIntent` / `ai-integration-engine._detectIntention`）保持只做 shape 转换 + 转发。
- 修改 `apply_template` → 只改 Rust 侧 SSoT `../services/ai-agent/src/flow_engine.rs::apply_template`；wrapper `../services/ai-agent/src/workflow_engine.rs::apply_template` 保持 ≤4 行（语法桥接 `${k}` → `{{k}}` + 转发）。

**新增 C3 族公共函数流程**：先在 `scripts/validate_no_duplicate_functions.js::REGISTRY` 增加一项 `{family, sigHeads, truthFile, wrapperFiles[], lang}`，再改真源实现 + 登记 wrapper；禁止"先写实现、后补声明"。

### 4. 看门狗脚本与手动跑法
- 脚本位置：`platform/backend-node/scripts/validate_no_duplicate_functions.js`
- 手动跑法：在 `platform/backend-node/` 目录下执行 `node scripts/validate_no_duplicate_functions.js`
- 出口码：`0` 即通过（无 C3 重复开发违规）。
- 扫描范围：`src/**/*.js`（JS 侧）+ `../services/**/*.rs`（Rust 侧）。覆盖 5 个函数族：degreeCentrality / betweennessCentrality / pagerank / detectIntent / apply_template。
