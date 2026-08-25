# MOX 核心高性能规格 — 全部 mox* 算法核心统一用 Rust + 最高性能算法
> 规格版本: MOX-RUST-MAX-SPEC-20260825
> 对应用户自然需求: "是所有mox里面的核心内容用rust最高性能与算法。"

---

## 1. 背景与基线

璇玑系统当前的 MOX 体系（`mox-server / mox-expert / mox-graph-* / mox-cloud-drive-* / mox-standards / mox-domain-abstractions / graph-algorithms / operator-core / optimizer ...`）已有大量 Rust 实现，但仍存在 **计算密集核心散落在 Node.js / Python / JS 胶水层** 的真实瓶颈，构成项目「算法核心未 100% Rust 化」的基线缺口：

| # | 定位 | 现状 (性能/质量瓶颈) | 代码锚点 |
|---|---|---|---|
| G1 | 图算法权威公式 (GraphFormulas 12 项) | Node.js 单线程运行：10k 节点 PageRank/Brandes 介数/CNM 社区检测 → 实测比 Rust 同算法慢 **5~25×**；无法利用多核心 /  SIMD / CSR 稀疏。 | [graph-formulas.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/graph/graph-formulas.js) / 调方：[ai-engine.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/ai-engine.js#L373-L375)、[routes/internal.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/routes/internal.js#L89-L104)、[routes/graph.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/routes/graph.js#L324-L330) |
| G2 | 归一化流水线 (normalization-pipeline) | 归一化/校验/去重/冲突合并 用 JS `reduce/map` + 多轮 JSON.stringify 对比，10 万条实体占满 1.5GB 堆；Rust rayon+Ahash 期望 **5× 吞吐 / 1/10 内存**。 | [project-atlas/application/normalization-pipeline.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/project-atlas/application/normalization-pipeline.js) |
| G3 | 意图分类器 / 联盟意图路由 | 正则链 + 字符串 contains 扫描，缺少 Aho-Corasick 自动机 + 等级评分；多模式匹配 **QPS / 延迟退化**。 | [expert-alliance/domain/intent-classifier.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/expert-alliance/domain/intent-classifier.js) |
| G4 | CEM 优化 (GraphFormulas.cemOptimize) & RRF 融合 | 启发式参数搜索在 JS 串行 + 数组 splice 高频分配；高并发接口 P99 尾延迟 ≥ 300ms。 | [graph-formulas.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/graph/graph-formulas.js) `cemOptimize` / `reciprocalRankFusion` |
| G5 | 专家联盟编排/规则判定 (alliance-orchestrator) | 多阶段分派/投票/门控 纯 JS 循环，缺少 SIMD 批量打分；企业级 24 专家联盟 → T5/TR-8 延迟预算偶发超时。 | [alliance-orchestrator.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/expert-alliance/application/alliance-orchestrator.js) |
| G6 | xiaobai_voice 音频 DSP（Python） | 上一轮 TTS 优化后 DSP 在 Python 侧执行（重采样 / SOLA / 响度归一化）；大长文本内存峰值与单线程推理时延仍有 2~4× 改进空间（Rust `fundsp`/`dasp`/`cpal` + SIMD 流水线）。 | [cosyvoice2.py](file:///d:/a10/aikjx/gitcode/infotopograph/projects/xiaobai_voice/xiaobai_voice/tts/cosyvoice2.py) |
| G7 | JS↔Rust 调用未统一 | 当前 Mocha 有 `rust_crate_bindings_e2e.js`，但 **Node 运行时真正路径不通过 Rust**，HTTP 端点仍走 graph-formulas.js 自身实现 —— 等于 "有 Rust crate 却没接通生产链路"。 | [rust_crate_bindings_e2e.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/test/rust_crate_bindings_e2e.js) 对比 [routes/graph.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/routes/graph.js#L324-L330) |

---

## 2. 目标 / 用户 / 非目标

### 2.1 目标
1. **Rust 核心单一来源**：把 G1~G7 中所有「计算密集、业务核心」内容，**全部迁移/重写为 Rust 实现**，并确保原 Node/Python/浏览器生产链路真实调用该 Rust 核心（测试/HTTP/实时数据一致）。
2. **算法选择必须"最高性能"**：每个算法选择当前学术界/工业界公认最高效的实现策略（CSR+Gauss-Seidel PageRank、Brandes 并行介数、CNM 模块度凝聚、Aho-Corasick 多模匹配、Rayon 并行迭代、SIMD 向量内积等），不得用 JS 直译 Rust 版本。
3. **接口零破坏**：Node 层保持 `GraphFormulas.{degreeCentrality | betweennessCentrality | closenessCentrality | pagerank | pagerankWithTranspose | personalizedPageRank | communityDetectionCNM | modularity | density | shortestPath | reciprocalRankFusion | cemOptimize}` 原有 shape，Python 音频 DSP 保持函数签名。
4. **度量可观测**：所有迁移核心在 Prometheus `/metrics` 暴露耗时 / 调用数 / 命中率 / 回退率，提供 Node↔Rust 对比基线。

### 2.2 用户 & 业务场景
- **企业级图谱分析**：PageRank/介数/社区/个性化 PR 支撑 6 度关联 / 证据链溯源 / 画像构建（百万边）。
- **专家联盟（Mox-Expert）**：24 专家并行辩论 / 归一 / 投票，延迟可控。
- **AI 对话/意图**：多标签意图识别、联盟融合 RRF、CEM 优化。
- **语音交互（xiaobai_voice）**：长文本 TTS DSP 流水线（重采样 / SOLA / 响度归一 / 回声抑制）。

### 2.3 非目标
- 不重写 GUI（Vue 页面）/ PBAC / 登录态 / 静态 HTTP 资源分发等非核心。
- 不引入新的外部微服务基础设施，仅允许：(a) napi-rs Node 原生模块；(b) UDS gRPC 微服务（与 mox-server 同进程优先）；(c) Python PyO3 原生扩展。
- 不改变数据落盘格式（graph_nodes.json / edges.json / KB 数据等严格 0 变更）。
- 不重写深度学习推理本身（CosyVoice2/Fish 的模型 forward 仍走框架；只加速 DSP 前后处理）。

---

## 3. 功能需求 (Functional Requirements)

### 3.1 GraphFormulas 12 项 → Rust（唯一权威）
- **FR-CORE-1 全覆盖**：新建 Rust crate `mox-formulas-core`（或纳入 `graph-algorithms` 并 pub 稳定 API），**精确覆盖** GraphFormulas 12 项：
  1. `degree_centrality(nodes, edges, opts)` → flat `{id: score}`；`expandRaw/legacyShape` 兼容
  2. `betweenness_centrality(nodes, edges, opts)`（支持 `directed/undirected`、Brandes）
  3. `closeness_centrality(nodes, edges, opts)`
  4. `pagerank(nodes, edges, opts)`（dampingFactor / maxIterations）
  5. `pagerank_with_transpose(nodes, edges, opts)` → 返回 `{forward, transposed, correlation}`
  6. `personalized_pagerank(nodes, edges, seedMap, opts)`（d=0.85 / maxIter=30 项目记忆锁死）
  7. `community_detection_cnm(nodes, edges, opts)` → `{communities: [[ids]], modularity}`
  8. `modularity(graph, communities)` → f64
  9. `density(nodes, edges)` → f64
  10. `shortest_path(nodes, edges, from, to)` → `{path:[ids], hops, edges}`；无权 BFS，有权 Dijkstra-BinaryHeap；`{max_hops}` 守卫
  11. `reciprocal_rank_fusion(lists: [[id]], k=60)` → `{id: score}`
  12. `cem_optimize(cfg, scores)` → 结构化调参结果（与当前 JS 输出 shape 对齐）

- **FR-CORE-2 算法最高性能**（每一个算法必须满足 ≥1 项优化要求）：
  - PageRank/PPR：**CSR 稀疏 + Gauss–Seidel（比 power iter 省 30~50% 迭代）**，并行 SpMV（Rayon）；
  - Brandes betweenness：每个源 **BFS / Δ(σ,δ)**，并在无自环稀疏下按 `rayon::par_iter().map(|s| brandes_one(s))` 并行汇总；
  - Closeness：无权重走 BFS；有权重用 BinaryHeap Dijkstra；
  - CNM 社区：**近邻最大堆 H(ΔQ)** 存相邻社区 ΔQ 最大值，**稀疏化 e_ij 仅维护相邻**；
  - RRF：`HashMap<u32>` + 并行 `par_extend` + 一轮排序；
  - ShortestPath：双向 BFS（bidir）对 long-path 最坏优化 2×；
  - 所有向量/矩阵运算使用 `ndarray`+`blas`(可选)，核心循环避免堆分配（prealloc + `Vec::with_capacity`）。

- **FR-CORE-3 生产链路切换**：在 Node 后端 `src/graph/graph-formulas.js` 中，`GraphFormulas.xxx` 全部变成 **仅通过 napi-rs 原生模块调用 mox-formulas-core**（本地无原生模块时保留 JS 实现作为 fallback，并在启动日志告警 + metrics `mox_formulas_fallback_total`）。

- **FR-CORE-4 结果精确对齐**（到项目记忆容忍阈值）：
  - 度中心性、密度、最短路径、模块度：必须 **逐字段位一致**（浮点符号/整数 1e-12 容差）；
  - PageRank / CNM：**允许数值稳定误差 1e-9**，但排名（argsort）100% 一致；
  - 社区划分（CNM）的模块度 **≥ JS 版本或相差 < 1e-6**。

### 3.2 归一化流水线 Rust 化
- **FR-CORE-5 NormCore**：新建 `mox-norm-core`，输入 `{records:[{id,fields}], rules:[rule]}`，输出 `{deduped, conflicts, normalized, stats}`；实现：
  - 规则引擎：SIMD-友好的 `FxBt/BitVec` 字段级打标；
  - 去重：`HashMap + (cityhash3/fxhash)` 指纹 + `group by` rayon 并行；
  - 冲突求解：按规则权重 + 确定性 tie-break（id 排序）保证可复现。

### 3.3 意图分类 Aho-Corasick
- **FR-CORE-6 IntentAC**：`mox-intent-core` 实现 Aho–Corasick 自动机（goto/fail/output），支持 500+ 模式 × 多类别 × 得分衰减；分类接口 `classify(text, top_k)` 返回 `[{label,score,hit_positions}]`。

### 3.4 联盟编排打分 Rust 化
- **FR-CORE-7 AllianceScoreCore**：`mox-expert` 已 Rust 化的基础上，把 Node orchestrator 的 "阶段分派/门控/投票聚合" 中所有 **批量打分** 下沉为 `mox-expert` 的 pub fn：`debate_score(ballots) -> aggregated`（SIMD 点积 + rayon 并行）；Node 只负责 JSON 编排与回调。

### 3.5 xiaobai_voice DSP Rust 化
- **FR-CORE-8 VoiceDSP**：新建 Rust crate `xiaobai_dsp`（PyO3 原生扩展），对外纯函数：
  1. `resample_linear(audio: [f32], sr_in, sr_out) -> [f32]`（线性插值 + SIMD）
  2. `time_stretch_sola(audio: [f32], sr, speed, frame_ms=20, overlap_hop_ms=10) -> [f32]`
  3. `apply_limiter_and_loudness(audio, target_lufs=-18, limiter_ceiling_db=-0.5) -> [f32]`
  4. `wav_encode(audio:[f32], sr, bits=16) -> bytes`
- Python 端 [cosyvoice2.py](file:///d:/a10/aikjx/gitcode/infotopograph/projects/xiaobai_voice/xiaobai_voice/tts/cosyvoice2.py) 统一改走 PyO3 扩展；若扩展未安装，Python 现有实现为 fallback。

### 3.6 统一接入 & 特性开关
- **FR-CORE-9 Fallback/灰度**：通过环境变量 `MOX_RUST_CORE=force|auto|off`（默认 auto）控制：
  - `force`：禁止 JS/Python fallback，缺失时启动失败；
  - `auto`（生产推荐）：Rust 成功加载就走 Rust，否则走 fallback + metric 递增；
  - `off`：完全走旧 JS/Python 实现。
- **FR-CORE-10 可观测**：`mox-formulas-core` 暴露 `mox_formulas_call_total{name,impl="rust|fallback"}`、`mox_formulas_duration_seconds{name}`、`mox_formulas_bytes_processed`；在 Node 与 mox-server 各自 `metrics` 端点都可抓。

---

## 4. 非功能需求 (Non-Functional Requirements)
- **NFR1 性能（vs JS baseline，同等输入、单节点）**：
  - 图算法：10k 节点 / 50k 边 → PageRank **≥ 5×**、Brandes Betweenness **≥ 10×**、CNM 社区检测 **≥ 8×**、Personalized PR **≥ 6×**；
  - 100k 节点 / 500k 边：Rust 端必须在 60s 内完成 PageRank（20 iter）、CNM；
  - 归一化流水线：10 万条 16 字段记录 → **吞吐 ≥ 5×** / RSS **≤ 1/5**；
  - 意图分类：500 模式 × 1 万句输入 → **≥ 10×** 吞吐；
  - 语音 DSP：5 秒 22050 Hz 音频端到端（resample + SOLA 1.03× + loudness-limiter）**延迟 ≤ 200 ms**。
- **NFR2 正确性**：
  - 与现存 GraphFormulas 的 **T5/TR-5/TR-8 业务断言 100% 通过**（精度按 FR-CORE-4）；
  - 所有 Rust 路径 **随机 fuzz ≥ 10 万轮** 不崩溃（proptest / quickcheck 风格）。
- **NFR3 兼容**：
  - 默认 `MOX_RUST_CORE=auto` 下，现有 `node tests/`、`test_*.js`、`mocha_*.js`（含 `test-algo-rust-node-diff.js`、`mocha_graph_algorithms.js`）**全部 0 修改可运行通过**（除非测试本身断言 JS 内部细节）。
  - 对生产路由响应体字段、类型、形状（`{id:number}` vs `{id:{degree:number}}`）完全一致。
- **NFR4 构建**：
  - Rust workspace `cargo build --release` 全通过；
  - napi-rs 产出 `@infotopograph/mox-formulas-native` + `@infotopograph/mox-norm-native`，Windows x64、Linux x64、macOS aarch64 三平台 prebuilt（通过 napi-rs npm 发布）。
  - PyO3 `xiaobai-dsp` wheel 在 CPython 3.10 / 3.11 / 3.12 三版本可用（至少本机 CPython3.12 x64）。
- **NFR5 依赖膨胀**：
  - 新增 crates.io 依赖总量 ≤ 15 个；
  - mox-server release 二进制体积增幅 ≤ +15%。
- **NFR6 零数据 / 零落盘变更**：
  - `data/graph_nodes.json / graph_edges.json / workflows.json / ...` 读写格式 100% 兼容（字节级）。

---

## 5. 约束 / 依赖 / 假设 / 开放问题

### 5.1 约束
- C1：Rust 工作区必须复用已定义 `[workspace.dependencies]`（`Cargo.toml` root），禁止 crate 独立锁版本。
- C2：本规格不接受「伪 Rust 化」——仅测试可用但生产路径未切换 **视为不通过**（验收时用 `NODE_DEBUG=mox-core:*` 确认每条图算法请求走 Rust）。
- C3：Rust 与 JS 之间的参数边界必须做「最小拷贝」—— 大数组优先使用 `SharedArrayBuffer` / `Buffer` 转移所有权，不得序列化 JSON 再反序列化。
- C4：单二进制 / 本地脚本都能运行（`cargo test`、`npm test`、`node test/test_algo_rust_node_diff.js` 三项可在同一台机器执行）。

### 5.2 依赖
- D1：已存在 Rust crate `graph-algorithms` 提供 PageRank / Betweenness / Closeness / CNM / PPR / shortest path / activation spread，**必须复用而非重复造轮子**（扩展不足部分在其 src 内补）。
- D2：已存在 `mox-expert` / `mox-server` / `mox-domain-abstractions`，用于 AllianceScoreCore、metrics 端点 及 Rust HTTP 接入。
- D3：Node 后端已有 napi 绑定样例（`rust_crate_bindings_e2e.js`），必须基于相同机制。
- D4：xiaobai_voice 的 cosyvoice2.py 已在 Python 实现 DSP，用于 fallback 正确性基准。

### 5.3 假设
- A1：图数据顶点 id 以字符串为主（Node 层 JSON），Rust 侧使用 `FxHashMap<String, i32>` 做 i32 压缩后算 CSR，结果再映射回 String，该步骤在大 N 下不应 >10% 总耗时。
- A2：用户本机能安装 `cargo / npx napi / maturin`，缺失时 fallback 仍可用。
- A3：语音 DSP 仅处理 `f32` PCM，不涉及模型格式转换 / 训练。

### 5.4 开放问题（Specify 阶段留档，Approve 前未解决则视为假设默认接受）
- OQ1：是否需要在 Rust 侧同时提供 HTTP 微服务端点（除 napi 外），以便非 Node 调用方复用？**默认：仅 napi + pyo3；HTTP 通过 mox-server `/formulas/*` 暴露可选。**
- OQ2：联盟编排全量迁移到 mox-expert Rust，是否允许删除 Node orchestrator 的分支？**默认：保留 Node orchestrator 逻辑，但打分计算下沉，不做行为改变。**
- OQ3：是否启用 BLAS（openblas-static / intel-mkl-src）？**默认：feature flag，默认纯 Rust CSR + Gauss–Seidel，可选 blas 以获 PageRank 再 +20~30%。**

---

## 6. 验收标准 (Acceptance Criteria)

### Rule 类 (客观可验证，必须 100% 通过)
1. **AC-R1 权威覆盖**：在 Node 运行时设置 `MOX_RUST_CORE=force` 后，GraphFormulas 12 项方法对 100 组随机输入均返回非 fallback（`mox_formulas_call_total{impl="rust"}>0` 且 fallback=0）。
2. **AC-R2 结果对齐**：`node platform/backend-node/test/test-algo-rust-node-diff.js` 0 失败（或同等断言：逐字段误差 ≤ FR-CORE-4 阈值）。
3. **AC-R3 图接口未变**：`node platform/backend-node/test/mocha_graph_algorithms.js`、`mocha_atlas_registry.js`、`mocha_alliance_and_flows_v2.js` 全部通过（0 新增失败）。
4. **AC-R4 归一化**：10 万条记录归一化的最终 `deduped.length + conflicts.length == 输入记录数`，并且输出与 JS 版本的结果等价（record id 集合、字段归一值一致）。
5. **AC-R5 意图分类 Aho**：Aho-Corasick 的命中位置与 JS 正则链版本一致（覆盖率 ≥ 99%；差异样本经人审确认 Rust 侧更严格更正确）。
6. **AC-R6 VoiceDSP PyO3**：在 CPython 3.12 本机 `import xiaobai_dsp` 成功；4 个 DSP 函数对固定种子音频输出与 Python fallback 的误差容差：`resample max|Δ|<1e-4`、`sola SNR>40dB`、`loudness ±0.5 LUFS`。
7. **AC-R7 可观测**：Node `/metrics` 与 mox-server `/metrics` 均可见 `mox_formulas_call_total`、`mox_formulas_duration_seconds`，Prometheus 解析无错。
8. **AC-R8 构建通过**：`cargo build --release --workspace`、`cargo test --workspace` 全绿；napi-rs prebuilt 至少 win x64 + linux x64，`maturin develop` 生成 wheel 成功。

### Rubric 类 (评估性打分 0-2，通过阈值 ≥ 1.4 平均)
1. **AC-Rb1 性能加速率（0-2）**：
   - 0：≤2× 或无基准；1：5~10× 平均；2：≥10× 平均 & 大 100k 节点仍满足 NFR1。
   - 证据来源：`_perf_rust_vs_node.js` 报告截图/JSON。
2. **AC-Rb2 算法选择先进性（0-2）**：
   - 0：直接把 JS 逐字翻译成 Rust；1：至少 5/12 公式采用最高性能算法说明并落地（CSR、Brandes 并行、双向 BFS、Aho-Corasick、近邻堆 CNM 等）；2：全部 12 项均符合工业最高性能惯例 + 代码注释附算法论文/标准链接。
3. **AC-Rb3 工程集成完备性（0-2）**：
   - 0：仅单元测试通过但生产路由仍走 JS；1：GraphFormulas 已统一切换 + `auto/force/off` 三级 + metrics；2：`force` 模式启动时 JS fallback 代码路径被静态 ESLint 规则禁止（确保不再被悄悄写入），并附带 CI gate 阻止回退。
4. **AC-Rb4 内存占用（0-2）**：
   - 0：与 Node 相当或更差；1：RSS/峰值内存 ≤ 1/3 Node；2：≤ 1/5 且 100k 图 PageRank 内存 ≤ 300MB。
5. **AC-Rb5 可维护性与文档（0-2）**：
   - 0：无文档；1：每个 Rust 模块有 doc comments 并附算法原理；2：docs/standards/mox-formulas-rust.md 完整说明 "算法-性能-边界" + 与 Node 回退策略 + 故障诊断。
