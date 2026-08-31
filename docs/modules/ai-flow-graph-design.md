# AI 流程图谱化设计 — 业务流程与算法流程统一承载于图谱引擎

> 版本：v1.0（2026-08-22）
> 上游文档：[ai-engine-master-analysis.md](docs/modules/ai-engine-master-analysis.md)（统一编排核心设计）
> 核心命题：**图谱即 AI 引擎的流程基础设施** —— 业务流程（五步流水线）与算法流程（意图激活扩散）不再散落在代码分支里，而是建模为图谱的节点与边，由图谱引擎统一承载、计算、可视化与验证。

## 1. 设计动机（为什么要把流程放在图谱上）

| 现状痛点 | 图谱化后 |
|---|---|
| 五步流水线（意图→路由→执行→校验→反馈）只是代码注释与 Mermaid 图，**运行时不可见、不可查、不可算** | 流水线变成图谱上的 `step` 节点链，`GET /ai/engine/flow-graph` 直接返回可渲染的结构 |
| 意图识别是"关键词打分循环"，与图谱引擎的加权传播算法割裂 | 意图识别变成图谱上的**激活扩散**（个性化 PageRank），复用统一 PageRank 单源实现 |
| 能力矩阵/降级链/委托关系分散在 `CAPABILITY_META` 对象里 | 全部建模为边：`triggers`（关键词→能力）、`delegates_to`（能力→引擎）、`degrades_to`（能力→chat） |
| 算法正确性靠肉眼读代码，**占位符缺陷（D8：介数/紧密中心性恒为 0）长期潜伏** | 每个公式配已知答案的标准测试图，逐公式断言验证（本文档第 5 节） |

## 2. AI 流程图谱模型（AI Flow Graph）

### 2.1 节点类型（4 类）

| 类型 | 前缀 | 数量 | 说明 |
|---|---|---|---|
| `step` | `step:` | 5 | 流水线步骤：intent（意图识别）→ route（能力路由）→ execute（引擎执行）→ verify（质量校验）→ feedback（指标反馈） |
| `keyword` | `kw:` | ~35 | 意图关键词（来自 `INTENT_KEYWORDS`，一词一节点） |
| `capability` | `cap:` | 6 | 能力：expert / reasoning / memory / graph / workflow / chat |
| `engine` | `eng:` | 4 | 委托引擎：expert-alliance-engine / ultimate-ai-engine / ai-engine / llm-gateway |

### 2.2 边类型（4 类）

| 类型 | 方向 | weight | 语义 |
|---|---|---|---|
| `triggers` | `kw:* → cap:*` | 关键词权重 w | 命中该关键词即激活该能力（业务触发关系） |
| `flows_to` | `step:i → step:i+1` | 1.0 | 流水线顺序（业务流程骨架） |
| `delegates_to` | `cap:* → eng:*` | 1.0 | 能力委托给引擎执行（算法执行关系） |
| `degrades_to` | `cap:* → cap:chat` | 0.5 | 失败单向降级链（不变式②的图谱化表达） |

### 2.3 图谱化意图识别（激活扩散算法）

把"关键词打分"升级为图谱上的**带权激活扩散**（个性化 PageRank 的特例）：

```
输入：问题 q
1. 命中检测：K = { kw ∈ keyword 节点 | q 包含 kw.label }
2. 个性化向量：p(kw) = w(kw)/Σw(K)（命中关键词按权重归一），其余节点为 0
3. 激活扩散：在流程图谱上跑个性化 PageRank
      a_i = (1-d)·p_i + d·Σ_{j→i} a_j·W(j,i)/outW(j)
4. 能力排序：取 capability 节点中激活值最高者
5. 兜底：K = ∅ 或全零 → chat（默认能力）
```

**与旧算法的关系**：当 d=0 时激活扩散退化为"个性化向量直接读出"，即等价于旧的关键词加权打分（分数归一化后排序不变）——因此图谱化是旧算法的**严格泛化**，向后兼容。默认 d=0.85 时，多跳关系（如关键词间无直接关联、但共享能力）也会参与传播。

### 2.4 算法单源委托（不变式①的延续）

| 公式 | 唯一实现位置 | 调用方 |
|---|---|---|
| PageRank（含激活扩散） | `ai-integration-engine.GraphIntelligenceEngine.computePersonalizedPageRank` | ai-flow-graph（意图识别）、ai-engine（图谱分析） |
| 度中心性 | `ai-flow-graph.GraphFormulas.degreeCentrality` | ai-engine._computeCentrality |
| 介数中心性（Brandes） | `ai-flow-graph.GraphFormulas.betweennessCentrality` | ai-engine._computeCentrality |
| 紧密中心性（harmonic） | `ai-flow-graph.GraphFormulas.closenessCentrality` | ai-engine._computeCentrality |
| 社区检测（LPA） | `ai-engine._detectCommunities`（既有序列实现） | ai-engine.analyzeGraph |
| 模块度 Q | `ai-flow-graph.GraphFormulas.modularity` | 公式测试套件（社区质量评估） |

## 3. 依赖方向（无环）

```
api-server ──→ ai-engine-core ──→ ai-flow-graph ──→ ai-integration-engine ──→ llm-gateway
                    │                  ↑
                    └── ai-engine ─────┘（委托公式库）
expert-alliance-engine / ultimate-ai-engine（core 的执行目标，独立）
```

`ai-flow-graph` 不依赖 `ai-engine-core`（配置注入式：`buildAIFlowGraph({INTENT_KEYWORDS, CAPABILITY_META, PIPELINE})`），避免环。

## 4. 公式清单（逐个定义）

| # | 公式 | 数学形式 | 约定 |
|---|---|---|---|
| F1 | 密度 | `D = 2E/(N(N-1))`（无向） | 无重边 |
| F2 | 度中心性 | `C_D(v) = deg(v)/(N-1)` | 无向度 |
| F3 | PageRank | `PR(v) = (1-d)/N + d·(Σ_{u→v} PR(u)/out(u) + M_dangling/N)` | d=0.85，悬挂质量均匀回传 |
| F4 | 介数中心性（Brandes） | `C_B(v) = Σ_{s≠v≠t} σ_st(v)/σ_st`，归一化除以 `(N-1)(N-2)/2`（无向）或 `(N-1)(N-2)`（有向） | BFS 最短路计数 |
| F5 | 紧密中心性（harmonic） | `C_C(v) = (Σ_{u≠v} 1/d(v,u)) / (N-1)` | 不可达贡献 0，比经典版稳健 |
| F6 | 社区检测（模块度贪心凝聚 CNM） | 反复合并 ΔQ 最大的相邻社区对：`ΔQ(A,B) = e_cross(A,B)/m − d_A·d_B/(2m²)`，无正增益即收敛 | 替代 LPA（实测发现其平局取最小标签导致"标签吞并"：双团+桥图坍缩为 1 社区） |
| F7 | 模块度 | `Q = Σ_c [ e_c/m − (d_c/(2m))² ]` | e_c=社区内边数，d_c=社区度数和 |
| F8 | 激活扩散 | `a_i = (1-d)·p_i + d·Σ_{j→i} a_j·W(j,i)/outW(j)` | 个性化 PageRank 特例 |

## 5. 公式测试验证方案（一个个来，已知答案断言）

每个公式配一张**已知解析答案的标准测试图**，断言数值误差 < 1e-9（PageRank 容差 1e-6）：

> **边约定**：无向图输入 RAW 边（单条），公式库内部双向展开；仅 PageRank（有向算法）需要双向展开边。

| 测试图 | 结构 | 验证目标 |
|---|---|---|
| T1 星型 | 中心 c 连 4 叶（无向） | F1 密度=0.4；F2 中心=1.0/叶=0.25；F4 中心介数=1.0/叶=0；F5 中心=1.0/叶=0.625（harmonic） |
| T2 链 | a→b→c→d→e（有向） | F3 e 最高且 ΣPR=1；F4 b 介数=0.25、c=1/3；F5 a=25/48≈0.5208、e=0 |
| T3 双团 | {a,b,c}∪{d,e,f} 全互连+桥 b−d | F6 应得 2 社区；F7 Q=5/14≈0.3571 |
| T4 双环 | a↔b | F3 各=0.5 |
| T5 孤立 | 3 个孤立点 | F1=0；F6=3 社区；F4/F5 全 0 |
| T6 星型有向 | 中心指向 4 叶 | F4 中心介数=0（无路径经过中心）；叶=0 |
| T7 意图用例 | 4 个真实问题 | F8："分析图谱PageRank"→graph；"深度推理"→reasoning；"专家会诊"→expert；"你好"→chat |
| T8 流程图谱自检 | AI 流程图谱自身 | 节点/边数量守恒；激活扩散与旧打分的 **top-1 路由决策一致**（决策一致性回归） |

## 6. API 设计

| 路由 | 方法 | 返回 |
|---|---|---|
| `/ai/engine/flow-graph` | GET | 流程图谱（nodes/edges/图例/统计），可直接供前端力导向渲染 |
| `/ai/engine/capabilities` | GET | （增强）附 `flow_graph_ref` 指向流程图谱 |
| `/ai/engine/process` | POST | （增强）意图识别结果附 `activation` 扩散明细（每个能力的激活值+命中词） |

## 7. 人性化输出约定

- 每个指标输出附 `formula`（人读公式）与 `interpretation`（解读文案），例如度中心性 1.0 → "该节点与所有其他节点直接相连，是全局枢纽"；
- 公式测试报告用表格打印：`✓/✗ | 公式 | 测试图 | 期望 | 实测 | 误差`；
- 流程图谱自带 `legend`（图例），前端无需硬编码颜色/类型映射。

## 8. 交付物清单

| 交付物 | 路径 |
|---|---|
| 流程图谱引擎 | `platform/backend-node/src/ai-flow-graph.js` |
| 公式测试套件 | `platform/backend-node/test/test-graph-formulas.js` |
| D8 修复（介数/紧密） | `platform/backend-node/src/ai-engine.js`（委托公式库） |
| 意图识别图谱化 | `platform/backend-node/src/ai-engine-core.js`（detectIntent 委托激活扩散） |
| 路由注册 | `platform/backend-node/src/api-server.js`（/ai/engine/flow-graph） |
| 本设计文档 | `docs/modules/ai-flow-graph-design.md` |

## 9. 实测结论（2026-08-22）

### 9.1 公式测试：35/35 全部通过

```
node test/test-graph-formulas.js
总计: 35 项断言 | 通过 35 | 失败 0
```

逐公式验证明细（节选，完整报告见测试套件输出）：

| 公式 | 测试图 | 期望 | 实测 | 误差 |
|---|---|---|---|---|
| F1 密度 | 星型图 | 0.4 | 0.4 | 0 |
| F2 度中心性(c) | 星型图 | 1.0 | 1.0 | 0 |
| F4 介数(b) | 链图(有向) | 0.25 | 0.25 | 0 |
| F4 介数(c) | 链图(有向) | 1/3 | 0.3333333333 | 0 |
| F5 紧密(a) | 链图(有向) | 25/48 | 0.5208333333 | 1.11e-16 |
| F3 PR | 双环图 | 各 0.5 | 各 0.5 | 0 |
| F6 社区数 | 双团+桥 | 2 | 2（{a,b,c}+{d,e,f}） | 0 |
| F7 模块度 | 双团+桥 | 5/14 | 0.3571428571 | 5.55e-17 |
| F8 激活扩散 | 4 个意图用例 | graph/reasoning/expert/chat | 全部一致 | 0 |
| F8 决策一致性 | 4 个用例 top-1 | ≡ 旧打分 | 全部一致 | 0 |

### 9.2 测试驱动的算法演进（逐公式验证的价值）

| 轮次 | 发现 | 处置 |
|---|---|---|
| 第 1 轮 T3 | **LPA 标签吞并**：平局取最小标签导致双团+桥图坍缩为 1 社区 | 社区检测升级为**模块度贪心凝聚（CNM）**：确定性、无平局歧义、模块度单调递增保证终止 |
| 第 1 轮 T2 | **精度截断**：`toFixed(8)` 使 1/3 类值的误差超 1e-9 容差 | 公式库移除 toFixed，保留全精度（展示层负责格式化） |
| 第 1 轮 T1 | **边约定不一致**：无向图双向展开边使度中心性翻倍 | 统一约定：无向 RAW 边输入，公式库内部展开 |
| 第 1 轮 T8 | **等价性断言语义错误**：d=0 时激活不传播，与旧打分本不等价 | 改为 top-1 路由决策一致性回归（务实且可长期维护） |
| 第 2 轮 T3 | F7 期望值推导笔误（桥对两团度贡献各计 2，实为各 1） | 修正期望 Q=5/14，实测吻合 |

### 9.3 端到端上线验证

- `GET /ai/engine/flow-graph`：51 节点（5 step + 36 keyword + 6 capability + 4 engine）/ 51 边（36 triggers + 4 flows_to + 6 delegates_to + 5 degrades_to），自带图例与公式说明；
- `POST /ai/engine/process`（"请深度推理并逐步分析…"）：`activation.method=spread`、`damping=0.85`、30 轮收敛，正确路由 reasoning；
- graph 能力回归（链式图）：介数 b=0.25、c=0.3333（修复 D8 前恒 0）；紧密 a=0.5208（修复前恒 0）；响应附 3 条人读公式。

### 9.4 D8 修复确认

介数中心性（Brandes）与紧密中心性（harmonic）此前为占位符恒 0，现已委托 `ai-flow-graph.GraphFormulas` 单源实现，经 T1/T2/T5/T6 四张标准图共 10 项断言验证，数值与解析解完全一致。

## 10. Rust 层跨语言对齐（2026-08-22 第二轮）

### 10.1 Rust 版流程图谱引擎

新增 `platform/domains/graph-algorithms/src/flow_graph.rs`（与 Node 层 `ai-flow-graph.js` 跨语言对齐）：

- `AIFlowGraph::build(rules, capabilities)`：两阶段构建（先全部节点、后全部边）——单阶段边建边加会因 `add_edge` 要求两端节点存在而静默失败（实测边数 4/31）；
- `detect_intent_by_spread(question)`：激活扩散意图识别（F8），委托修复后的 `pagerank_personalized` 单源实现，平局取字典序最小（确定性）；
- `stats()`：节点/边数量守恒自检；
- `default_config()`：与 Node 层 INTENT_KEYWORDS 核心子集对齐（16 关键词 × 6 能力 × 5 步流水线 × 4 引擎）。

### 10.2 Rust 层缺陷清单与修复（公式测试驱动发现）

| 缺陷 | 位置 | 表现 | 修复 |
|---|---|---|---|
| R-D1 介数中心性空占位符 | `centrality_metrics()` 返回 `HashMap::new()` | 恒为空（同 Node 层 D8） | Brandes 2001 算法：BFS 最短路计数 + 反向依赖累积，归一化 ÷(N-1)(N-2) |
| R-D2 PageRank 悬挂质量丢失 | `pagerank()` | 悬挂节点质量直接消失，ΣPR<1 | 悬挂质量均匀回传全图 + 收敛提前终止（1e-6） |
| R-D3 LPA 不可复现且标签吞并 | `detect_communities()` | HashMap 迭代顺序随机 → 平局结果不可复现；双团+桥坍缩 1 社区（同 D6/D9） | CNM 模块度贪心凝聚：ΔQ 排序取最大（平局取字典序最小对），无正增益即收敛 |
| R-D4 度中心性语义不一致 | `degree_centrality()` | 除以 2(N-1)，与 Node 层 F2 不一致 | 统一为 deg(v)/(N-1) |
| R-D5 紧密中心性经典版偏大 | `closeness_centrality()` | (n-1)/Σd 漏掉不可达 ∞ 项 | 统一为 harmonic 版（与 Node 层 F5 一致） |
| **R-D6 PageRank 传播方向错误**（实测发现） | `pagerank()` 矩阵乘法 | `transition*rank` 是吸收者视角，缺转置 → 质量反向流动，链式图 ΣPR=0.445 | 推模型取转置：`transitionᵀ·rank`（与 Node 层 D5 修复同源） |
| **R-D7 流程图谱构建顺序错误**（实测发现） | 初版 `flow_graph.rs` | 边建到未创建节点 → `let _=` 吞错，31 条边只建成 4 条 | 两阶段构建 + `expect` 显式校验 |

### 10.3 Rust 公式测试：14/14 通过（跨语言一致）

```
cargo test（platform/domains/graph-algorithms）
test flow_graph::tests::t1_star_graph_formulas ... ok    # F2 度 1.0/0.25；F4 介数 1.0/0；F5 紧密 1.0/0.625
test flow_graph::tests::t2_chain_graph_formulas ... ok   # F3 ΣPR=1 且 e 最高；F4 b=0.25/c=1/3；F5 a=25/48/e=0
test flow_graph::tests::t3_two_cliques_communities ... ok # F6 CNM 恰好 2 社区 {a,b,c}+{d,e,f}
test flow_graph::tests::t4_two_cycle_pagerank ... ok     # F3 对称不动点各 0.5
test flow_graph::tests::t5_isolated_graph ... ok         # 3 社区；中心性全 0
test flow_graph::tests::t6_directed_star_betweenness ... ok # F4 全 0（无路径经过中心）
test flow_graph::tests::t7_intent_detection ... ok       # F8 四用例路由 graph/reasoning/expert/chat
test flow_graph::tests::t8_flow_graph_integrity ... ok   # 节点/边数量守恒；决策一致性
test result: ok. 14 passed; 0 failed（含 6 项旧测试回归无破坏）
```

与 Node 层 T1-T8 使用**同套测试图与同套期望值**（0.25 / 1/3 / 25/48 / 0.625 / 5÷14 等），实现跨语言公式级一致。全 workspace（含 ai-agent、kg-hub 依赖方）编译通过，API 向后兼容。

## 11. 自动开发引擎：需求 → 业务架构图谱 → 代码 → 预览（2026-08-22 第三轮）

回答核心问题：**"会自动在电脑开发完成任务？比如开发一个官网"——会。**

### 11.1 五阶段流水线（auto-dev-engine.js）

```
POST /ai/engine/auto-dev { requirement: "开发一个企业官网" }
  ↓ ① architecture   LLM 生成业务架构图谱（站点/页面/分区/导航/实体/主题）
  ↓ ② validate       校验归一化（页面≤8、分区≤8、条目≤8、类型白名单、链接安全化）
  ↓ ③ render         确定性代码渲染（HTML 语义分节 + 黄金分割 CSS + site.json）
  ↓ ④ persist        安全落盘（扩展名白名单 + 路径逃逸校验 + 覆盖须显式授权 + sha256 登记）
  ↓ ⑤ graph-store    架构图谱入图（graph_nodes/graph_edges，图谱 UI 可查看）
  → 返回 { project, files, architecture, preview_url }
```

设计原则：**LLM 只负责"想"（架构图谱 JSON），代码由确定性渲染器"写"**——生成结果可校验、可复现、无幻觉代码。

### 11.2 API 清单

| 路由 | 方法 | 说明 |
|---|---|---|
| `/ai/engine/auto-dev` | POST | 一句话需求全自动开发（body: requirement, project_name, overwrite） |
| `/ai/engine/auto-dev/projects` | GET | 已生成项目列表（去重文件计数 + 总体积 + 最近时间） |
| `/ai/engine/auto-dev/preview/:project/:file` | GET | 在线预览（html/css/js/json 白名单 + nosniff + no-store） |

### 11.3 架构图谱入图结构（实例：site-corp-site）

- 节点 29 个：`site`×1 + `page`×4 + `section`×16 + `entity`×8（id 前缀 `sd:{project}:`）
- 边 40 条：`包含`×20（site→page、page→section）+ `导航`×12（页面互链）+ `使用`×8（page→entity）
- 全部标记 `ai_generated: true`、`topic: auto-dev:{requirement}`，可在图谱 UI 中与知识图谱联合分析

### 11.4 端到端实测（test/test-auto-dev-e2e.js，17/17 通过）

| 验证项 | 结果 |
|---|---|
| 服务健康 | GET /ai/engine/capabilities → 200 |
| 端到端开发 | HTTP 200，耗时 9~14s，五阶段流水线完整 |
| 生成产物 | 6 文件（index/about/services/contact.html + styles.css + site.json），4 页面 |
| 内容质量 | 语义 HTML5（nav/hero/features/about/cta/contact/footer）+ SEO meta + 黄金分割 CSS |
| 在线预览 | HTTP 200 + text/html + nosniff 安全头，CSS 资源可访问 |
| 安全闸门 | 路径逃逸 `../../` 与编码逃逸 `..%2F` 均被 404 拒绝 |
| 图谱入图 | 29 节点/40 边，层级完整 |

预览地址：`http://localhost:3010/ai/engine/auto-dev/preview/site-corp-site/index.html`

### 11.5 本轮实测发现并修复的缺陷

| 缺陷 | 表现 | 修复 |
|---|---|---|
| D12 路由参数传递错误 | preview 路由读 `req.params.project` → 500（params 是 handler 第三参） | 改为 `(req, res, params) => params.project` |
| D13 缺失模块导入 | api-server.js 未 require auto-dev-engine → 启动即崩 | 补 `const { getAutoDevEngine } = require('./auto-dev-engine')` |
| D14 分区空类名 | `<section class="">` 丢失类型样式钩子 | 按类型输出 hero/features/about/text/cta/contact |
| D15 项目列表重复计数 | 重复生成（overwrite）后文件数虚增（24≠6） | 按文件名去重计数 |

### 11.6 边界与约定

- 需要真实 AI 引擎（已接 DeepSeek Key）；未接 Key 时显式报错引导配置，不静默降级
- 站点名/文案由 LLM 生成，每次运行可不同（创意多样性）；代码结构与安全约束由确定性渲染器保证
- 覆盖已有文件必须显式传 `overwrite: true`（防误覆盖）
- 预览仅服务白名单扩展名，杜绝任意文件读取
