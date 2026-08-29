# MOX 核心 Rust 最高性能化 · 任务切片
> 来源规格：`.trae/specs/20260825-mox-all-core-rust-max-algo/spec.md`
> 版本: v1.0 · 切片数: 17（5 里程碑）· 执行次序: 先核心算法 (Rust) → napi/pyo3 绑定 → Node/Python 生产切换 → 基准与度量 → 回归验证
> 依赖约定: 同一里程碑内可并行跨切片; 跨里程碑按 Depends On 串接

每个 Task 字段：
- **ID**：R-xxx (Rust-core) / N-xxx (Node bindings) / P-xxx (Python bindings) / M-xxx (Metrics/verify) / V-xxx (Validate)
- **Status**：`pending | in_progress | completed | blocked | cancelled`
- **Priority**：`high | medium | low`
- **Depends On**：前置任务
- **Maps AC**：对应 spec AC 编号
- **Task-Local Test Requirements (TR)**：仅 `rule` / `rubric`
- **Completion Evidence**：Implement 阶段填写（提交 hash / 脚本输出 / 截图）

---

## 里程碑 M0：统一工作区骨架与共享类型

### Task 1: R-0 工作区骨架 · mox-formulas-core / mox-norm-core / mox-intent-core / xiaobai-dsp 注册
**Status**: pending
**Priority**: high
**Depends On**: 无
**Maps AC**: AC-R1, AC-R8, FR-CORE-1, FR-CORE-5, FR-CORE-6, FR-CORE-8

**Scope**:
- 根 `Cargo.toml` workspace members 新增 4 crates：
  - `platform/services/mox-formulas-core`（稳定图/融合/优化算法，面向 Node 绑定）
  - `platform/services/mox-norm-core`（归一化流水线，面向 Node 绑定）
  - `platform/services/mox-intent-core`（Aho-Corasick 意图分类，面向 Node 绑定）
  - `projects/xiaobai_voice/xiaobai-dsp`（语音 DSP，面向 PyO3）
- `workspace.dependencies` 统一追加：`rayon`, `hashbrown`, `ahash`, `rustc-hash`, `ndarray`, `nalgebra`（可选）, `simd-json`（仅 JSON 边界）, `prometheus`（metrics crate）, `napi` {version="2", features=["napi8"]}, `napi-derive`, `pyo3` {version="0.22", features=["extension-module"]}（分 target 选）。
- 共享 FFI 边界类型：`JsonGraphInput`, `GraphAlgoOpts`, `NormRecord`, `IntentHit`, `DspResult` 等放在 `mox-common-meta`（避免重复）。

**TR**：
- TR (rule): `cargo metadata --format-version=1 | node -e "..."` 解析 4 crates 的 `id` 全部存在并在 workspace 中。
- TR (rule): `cargo build -p mox-formulas-core -p mox-norm-core -p mox-intent-core -p xiaobai-dsp --release` exit=0（空骨架即可）。
- TR (rule): 4 个 crate 的 `README.md` 中明确 "对外稳定 API（v1）"、默认 feature、平台兼容矩阵。

---

## 里程碑 M1：Rust 算法核心 · 最高性能化（R-1 ~ R-6）

### Task 2: R-1 GraphFormulas 12 项权威实现（复用 graph-algorithms 现有 CSR + 补齐缺失 5 项）
**Status**: pending
**Priority**: high
**Depends On**: R-0
**Maps AC**: FR-CORE-1, FR-CORE-2, FR-CORE-4, AC-R1, AC-R2, AC-Rb1, AC-Rb2

**Scope**:
- 实现目录：`mox-formulas-core/src/`
  - `graph.rs`：String 顶点 id → `FxHashMap<String, i32>` 压缩 → `CsrAdj`（复用 `graph-algorithms/src/lib.rs` 的 `CsrAdj`，若不可 pub 则复制并加单测）。
  - `centrality.rs`：Brandes 并行介数（rayon 源并行）、度中心性、接近中心性。
  - `pagerank.rs`：CSR + Gauss–Seidel（比 power 迭代省 30~50%）+ 并行 SpMV；`pagerank_with_transpose` 跑正/反向两张 CSR 并计算相关性；Personalized PR 用 `personalized` 向量做 Gauss-Seidel。
  - `community.rs`：CNM 近邻最大堆 ΔQ 凝聚社区（`graph-algorithms` 已有模块作为首选实现，这里做 API 适配 + 模块度计算独立函数）。
  - `rrf.rs`：Reciprocal Rank Fusion（`FxHashMap + par_extend` + top_k 选择）。
  - `shortest.rs`：双向 BFS（无权）+ BinaryHeap Dijkstra（有权），`max_hops` 守卫。
  - `cem.rs`：Cross-Entropy Method 连续/离散参数搜索（参考 JS 版本公式：多精英采样 + 平滑高斯分布）。
  - `lib.rs`：对外 `pub mod graph; pub use graph::*;` 等等。提供 12 个 pub fn，签名对 Node 绑定友好（`#[napi]` 直接可用或极薄的层）。
- 算法最高性能要求：
  - 每个热点循环使用 `Vec::with_capacity`、避免 `clone`；
  - CSR 行指针/列/值数组用 `Vec<i32>/Vec<i32>/Vec<f32>`，浮点用 f32（默认 PageRank/PPR/介数对 f64 并无好处）；
  - 所有 "每个源" 的算法（Brandes/Closeness）使用 rayon 并行；
  - CNM 使用最大堆（`std::collections::BinaryHeap`）与**稀疏相邻社区列表**；

**TR**：
- TR (rule): 运行 `t_r1_oracle_12_methods` 对一组固定 oracle_graph 调用 12 个方法，返回 shape 全部符合约定（度中心性是 flat {id:number} 等）。
- TR (rule): `t_r1_100k_graph_performance` 对 100k 节点 / 500k 边的 Erdos–Renyi 随机图：PageRank(20 iter) ≤ 60s；CNM ≤ 60s；Brandes Betweenness（抽样 5% 源）≤ 90s。不达标则保持 `in_progress`。
- TR (rule): `t_r1_formulas_exact_align` 对 20 组种子输入，与 JS `GraphFormulas` 输出比较：度/模块度/最短路径 位一致；PageRank 误差 <1e-9；社区模块度 ≥ JS 或相差 <1e-6。
- TR (rule): 未使用 feature(blas) 的纯 Rust 实现也必须通过全部 20 组比较（blas 仅加速非必要）。
- TR (rubric 0-2, 阈值 ≥ 1.4): AC-Rb2 算法选择先进性评分。

---

### Task 3: R-2 归一化流水线 NormCore (高性能去重/规则求解/冲突融合)
**Status**: pending
**Priority**: high
**Depends On**: R-0
**Maps AC**: FR-CORE-5, NFR1, NFR4, AC-R4, AC-Rb4

**Scope**:
- `mox-norm-core/src/`：
  - `rules.rs`：规则表达式 AST（`Eq / Contains / Regex / Lowercase / StripWhitespace / Truncate / Fingerprint / MergeIfSame / DedupFingerprint`）；使用 SIMD 友好的 `memchr` / `twoway`；
  - `fingerprint.rs`：基于 `FxHashMap + cityhash3`（指纹大小 64bit）；
  - `dedup.rs`：记录组内去重， rayon 并行按 `(field_keys_combo, fingerprint)` 分组；
  - `conflict.rs`：按规则权重融合；tie-break 确定性（record.id 字典序）；
  - `lib.rs`：`pub fn normalize(input: NormInput) -> NormOutput`。

**TR**：
- TR (rule): `t_r2_100k_dedup` 插入 100,000 条 16 字段记录、其中 10% 为重复 → 结果 `deduped.len == 90,000` 且 `conflicts.len == 10,000`。
- TR (rule): `t_r2_rule_determinism` 同输入运行 10 次 → JSON 序列化结果 100% 一致。
- TR (rule): 与 Node 现有 normalization-pipeline 输出做 10 组业务样本比对，`deduped` 集合与 `conflicts` 集合字段值一致。
- TR (rubric 0-2, 阈值 ≥ 1.4): 性能 ≥ 5× 且 RSS ≤ 1/5 的 AC-Rb4 评分。

---

### Task 4: R-3 意图分类 · Aho-Corasick + 多类别加权
**Status**: pending
**Priority**: high
**Depends On**: R-0
**Maps AC**: FR-CORE-6, NFR1, AC-R5

**Scope**:
- `mox-intent-core/src/`：
  - `ac.rs`：goto table 用 `Vec<[u32;256]>` 紧凑化；双数组 trie 备选；fail 使用 BFS；output 为 `Vec<(pattern_id, label, score)>`；
  - `classify.rs`：命中后按 `pattern_count × weight × position_decay` 累计分数；支持 top-k；
  - `lib.rs`：`Automaton::from_patterns(pats: Vec<Pattern>) -> Self`、`classify(&self, text: &str, top_k: usize) -> Vec<IntentHit>`。

**TR**：
- TR (rule): `t_r3_500_pats_10k_sents` 500 模式 × 10,000 句子，吞吐 ≥ 10× JS 正则链（同机基准）。
- TR (rule): `t_r3_ground_truth` 对 300 条人工标注样本，Rust 分类 top-1 F1 ≥ JS 版本或提升 ≥ 1%。
- TR (rule): 超长文本 (100KB) 单次 classify 无 OOM 且 ≤ 5ms。

---

### Task 5: R-4 联盟辩论打分聚合 · SIMD + rayon
**Status**: pending
**Priority**: medium
**Depends On**: R-0
**Maps AC**: FR-CORE-7, NFR1, NFR6

**Scope**:
- 在 `mox-expert/src/alliance/`（与现有 `debate.rs / team.rs / gate.rs` 同层）新增 `scoring_rs.rs`：
  - `score_ballots_par(ballots: &[Ballot], weights: &[f32]) -> Aggregate`：每个候选 × 专家的矩阵使用 rayon，并在每一行用 `std::simd`（nightly 稳定化后）或 f32×8 手工 unroll 做加权点积。
  - `rank_aggregate(ballots_matrix) -> RRF`：直接调用 `mox-formulas-core::reciprocal_rank_fusion`（避免重复实现）。
- Node `alliance-orchestrator.js` 增加可选 `rustScoring: true`，通过 napi-rs 调用 scoring 函数。

**TR**：
- TR (rule): `t_r4_24_expert_ballots` 生成 24×1000 投票矩阵，Rust 与 JS 计算聚合结果排序一致（Kendall τ ≥ 0.99）。
- TR (rule): 延迟 ≤ JS 1/3。
- TR (rule): 空 ballots / 单候选 / 零权重三种边界均返回合法结构。

---

### Task 6: R-5 Voice DSP · resample / SOLA / loudness-lim / wav-encode（SIMD + 预分配）
**Status**: pending
**Priority**: high
**Depends On**: R-0
**Maps AC**: FR-CORE-8, NFR1, AC-R6

**Scope**:
- `xiaobai-dsp/src/`：
  - `resample.rs`：线性插值 + `f32x8 SIMD`（`core::simd::Simd`，feature = "std-simd"；fallback 标量），预分配输出容量。
  - `sola.rs`：SOLA overlap-add，搜索窗口内 normalized cross-correlation 用 SIMD 点积；
  - `loudness.rs`：ITU-R BS.1770-4 K-加权滤波器（近似实现）+ soft limiter `tanh`；
  - `wav.rs`：`Vec<u8>` 写 16-bit PCM wav 头 + 数据（与现有 Python 头位一致）。
  - `lib.rs`：`#[pyclass]`/`#[pyfunction]` 暴露。

**TR**：
- TR (rule): `t_r6_resample_linear` 44.1k→22.05k 对正弦波输出 SNR ≥ 90dB。
- TR (rule): `t_r6_sola_snr` 对 5 秒语音做 1.03× 拉伸，与 Python SOLA 原结果 SNR ≥ 40dB（对齐后）。
- TR (rule): `t_r6_loudness` 输入 RMS -24dBFS，目标 LUFS=-18，输出测量在 ±0.5 LUFS。
- TR (rule): 5 秒 22050 Hz 端到端 DSP（resample+sola+loudness+encode）延迟 ≤ 200 ms。

---

### Task 7: R-6 统一 Metrics 与 共享 Registry
**Status**: pending
**Priority**: medium
**Depends On**: R-1, R-2, R-3, R-4, R-5
**Maps AC**: FR-CORE-10, AC-R7

**Scope**:
- 新建 `mox-common-meta/src/perf_registry.rs`：线程安全 `PerfRegistry { name: Counter, Histogram }`；
- mox-server `/metrics` 暴露；Node 后端 napi 模块通过 `prom-client` 注册同源指标（`MOX_RUST_CORE_*`）。
- 启动日志：`[mox-core] impl=rust formulas/intent/norm/alliance OK; voice-dsp OK/NOT_LOADED; fallback_mode=auto/force/off`。

**TR**：
- TR (rule): curl 拉 `/metrics` 后 grep 得到 `mox_formulas_call_total{name="pagerank", impl="rust"}`。
- TR (rule): `MOX_RUST_CORE=off` 启动后 fallback 指标每次 GraphFormulas 调用都 +1。

---

## 里程碑 M2：Node napi 绑定 · 生产切换 (N-1 ~ N-3)

### Task 8: N-1 mox-formulas-core → napi 绑定（包名 `@infotopograph/mox-formulas-native`）
**Status**: pending
**Priority**: high
**Depends On**: R-1
**Maps AC**: FR-CORE-3, FR-CORE-9, AC-R1, AC-R2, AC-R3, AC-Rb3

**Scope**:
- `platform/services/mox-formulas-core/napi/` 或单独 workspace crate `mox-formulas-napi`，使用 `#[napi(object)]` 输入 `{nodes:[{id,type?,weight?}], edges:[{source,target,weight?}]}`。
- 所有 12 个方法 `#[napi] pub async fn pagerank(...)` 同步版本也暴露（因为 CPU 密集，实际调用通过 `napi::tokio` 线程池）。
- 对 JS 保持 `GraphFormulas` 输出 shape：对 legacy `graph-algos.js` 结构做 wrapper。

**TR**：
- TR (rule): `MOX_RUST_CORE=force node platform/backend-node/test/test-algo-rust-node-diff.js` exit=0。
- TR (rule): `MOX_RUST_CORE=force node platform/backend-node/test/mocha_graph_algorithms.js` exit=0。
- TR (rule): 相同输入 10k×50k 图下 `cargo bench` 与 Node 测的比值 ≥ 5× (PR) / ≥10× (Brandes)。
- TR (rule): Native binding 加载失败时，`auto` 模式启动不抛异常、调用走 fallback + 指标 + 日志 warning。
- TR (rubric 0-2, 阈值 ≥ 1.4): AC-Rb3 工程集成完备性评分。

---

### Task 9: N-2 mox-norm-core + mox-intent-core → 单包 `@infotopograph/mox-norm-intent-native`
**Status**: pending
**Priority**: high
**Depends On**: R-2, R-3
**Maps AC**: FR-CORE-5, FR-CORE-6, AC-R4, AC-R5

**Scope**:
- 一个包导出两个对象：`NormCore`、`IntentCore`；
- 对 Node 的 `project-atlas/application/normalization-pipeline.js` 与 `expert-alliance/domain/intent-classifier.js` 做薄切换：
  - `if (NormCore.nativeAvailable) { return NormCore.run(records, rules); } else { fallback(); }`
  - Intent 同理。

**TR**：
- TR (rule): `MOX_RUST_CORE=force node platform/backend-node/test/test-normalization-pipeline.js 100k` 通过（或同系列断言文件）。
- TR (rule): `MOX_RUST_CORE=force node platform/backend-node/test/test-intent-single-source.js` 通过。
- TR (rule): fallback 模式下，相同输入输出不变。

---

### Task 10: N-3 AllianceScore · 通过 napi 调用 mox-expert scoring
**Status**: pending
**Priority**: medium
**Depends On**: R-4
**Maps AC**: FR-CORE-7, NFR6

**Scope**:
- 新增 `@infotopograph/mox-expert-native`（或复用 N-1），导出 `AllianceCore.debateScore(ballots, weights)`。
- `alliance-orchestrator.js` 在需要大批量打分时调用。

**TR**：
- TR (rule): `MOX_RUST_CORE=force node platform/backend-node/test/test-expert-alliance-e2e.js` 通过。
- TR (rule): 与未切换前的聚合排序一致。

---

## 里程碑 M3：Python PyO3 · xiaobai_voice DSP 切换

### Task 11: P-1 xiaobai-dsp PyO3 扩展 + maturin build
**Status**: pending
**Priority**: high
**Depends On**: R-5
**Maps AC**: FR-CORE-8, AC-R6, NFR4

**Scope**:
- `xiaobai-dsp/Cargo.toml` 配置 `[lib] name="xiaobai_dsp" crate-type=["cdylib"]`；
- `pyproject.toml`：maturin backend；
- `xiaobai_voice/tts/cosyvoice2.py`：在模块顶部尝试 `import xiaobai_dsp`，成功则使用 Rust 实现替换 `_resample_linear / _time_stretch_sola / _apply_limiter_and_loudness / _encode_wav` 四个工具函数；否则 fallback。

**TR**：
- TR (rule): `maturin develop --release` 后 `py -3 -c "import xiaobai_dsp; import numpy as np; a=np.random.randn(48000).astype(np.float32); b=xiaobai_dsp.resample_linear(a,22050,24000); print(b.shape, b[:3])"` 不报错且输出 shape==(52245)。
- TR (rule): 按 AC-R6 要求 4 个 DSP 误差容差通过。
- TR (rule): 未安装 xiaobai_dsp 时 `cosyvoice2.py` 功能完全 fallback，启动无错误。
- TR (rubric 0-2, 阈值 ≥ 1.0): 构建易用性 + 文档清晰度。

---

## 里程碑 M4：前端/路由/管理 零破坏回归 (V-1~V-3)

### Task 12: V-1 Graph 路由/内部路由 / Atlas / ai-engine 端到端回归
**Status**: pending
**Priority**: high
**Depends On**: N-1
**Maps AC**: AC-R1, AC-R2, AC-R3, NFR3

**Scope**:
- 保持 Node 路由 (`routes/graph.js`, `routes/internal.js`, `ai-engine.js`, `ai-integration-engine.js`, `nebulagraph-adapter.js`) 代码尽量少改——只在 `require('./graph/graph-formulas')` 时把 `GraphFormulas` 对象替换为 napi 实现 + fallback。
- `NODE_DEBUG=mox-core:*` 下，每条图请求都会打印 `impl=rust`（force 模式）或 `impl=fallback`。

**TR**：
- TR (rule): `MOX_RUST_CORE=force ./node_modules/.bin/mocha platform/backend-node/test/{test-pagerank-transpose-activation,test-graph-formulas-single-source,mocha_graph_algorithms,mocha_atlas_registry,test-graph-search-rerank,test-graph-cnm-raw-precision,test-atlas-flows,test-enterprise-10task-t2-algorithm,test-algo-rust-node-diff}.js` 全部通过 (exit=0)。
- TR (rule): HTTP API：GET `http://localhost:3010/graph/pagerank` 等 10 个常用 graph 端点响应与 baseline server 的 JSON diff 中仅浮点字段有 <1e-9 误差，其他字段无差异。
- TR (rule): 运行 1000 次请求，P99 延迟 ≤ 基线 1/3。

---

### Task 13: V-2 专家联盟回归 · V-3 意图/归一化回归
**Status**: pending
**Priority**: high
**Depends On**: N-2, N-3
**Maps AC**: AC-R4, AC-R5, NFR3

**Scope**:
- 运行企业联盟全流程：`test-expert-alliance-e2e.js`、`test-expert-alliance-enterprise.js`、`test-intent-single-source.js`、`test-normalization-pipeline.js`、`test-project-atlas.js` 等。

**TR**：
- TR (rule): 上列 5 文件 exit=0。
- TR (rule): 至少一个 alliance e2e 请求 `mox_formulas_call_total{impl="rust"}` 真实 ≥1。

---

## 里程碑 M5：基准报告 / 文档 / 特性闸

### Task 14: M-1 基准测试 Harness + 性能报告
**Status**: pending
**Priority**: medium
**Depends On**: V-1
**Maps AC**: NFR1, AC-Rb1, AC-Rb4

**Scope**:
- 新建 `platform/backend-node/test/_perf_rust_vs_node.js` 的强化版（或 `projects/t25-mox-rust-core-bench/` 目录，含 1k/10k/100k 三档图）。
- 产出 `performance_report.md`：每档 × 12 公式 × 2 impl (Rust/JS) 的 mean/p50/p95/throughput/Memory RSS；加速比；失败项。

**TR**：
- TR (rule): 10k/50k 节点全部 12 项都达到 NFR1 下限（PageRank ≥5×、Brandes ≥10×、CNM ≥8×、PPR ≥6× 等）。
- TR (rule): 100k 节点 PageRank(20 iter) ≤ 60 s。
- TR (rubric 0-2, 阈值 ≥ 1.4): AC-Rb1 性能加速评分。

---

### Task 15: M-2 `MOX_RUST_CORE` 三级切换 + ESLint 禁写 fallback guard
**Status**: pending
**Priority**: medium
**Depends On**: N-1, N-2, N-3
**Maps AC**: FR-CORE-9, AC-Rb3

**Scope**:
- `MOX_RUST_CORE=force|auto|off` 三档；
- 在 `graph-formulas.js` 中仅保留 **真正的 fallback 实现**（不可再写独立算法变种）；并引入 ESLint 自定义规则 `rules/no-independent-formula-implementation.js`：
  - 如果 `pagerank/betweenness/...` 函数中出现循环/矩阵计算特征（非单纯转调 napi + falllback）→ lint 失败。

**TR**：
- TR (rule): `MOX_RUST_CORE=force` 启动，调用 fallback 函数会抛 `RustCoreRequired`。
- TR (rule): `./node_modules/.bin/eslint platform/backend-node/src/graph/graph-formulas.js platform/backend-node/src/lib/graph-algos.js` exit=0（若存在违规独立实现则保持 in_progress 直到移除）。

---

### Task 16: M-3 文档 · `docs/standards/mox-formulas-rust.md`
**Status**: pending
**Priority**: low
**Depends On**: V-1, M-2
**Maps AC**: AC-Rb5, NFR6

**Scope**:
- 内容：
  1. 12 个公式 & 其它 Rust 化模块的算法选择（附经典参考链接）；
  2. CSR 结构 & 并行度说明；
  3. 回退策略 (`auto/force/off`) 与故障排查；
  4. 如何构建 napi-rs prebuilt / maturin wheel；
  5. metrics 列表与告警建议。

**TR**：
- TR (rule): `docs/standards/mox-formulas-rust.md` 存在；含 5 章。
- TR (rubric 0-2, 阈值 ≥ 1.4): AC-Rb5 文档与可维护评分。

---

### Task 17: V-ALL 综合验收脚本（一键 "全绿"）
**Status**: pending
**Priority**: high
**Depends On**: V-1, V-2, V-3, M-1, M-2
**Maps AC**: 所有 AC 汇总 (Rule 1-8 + Rubrics 1-5)

**Scope**:
- `scripts/verify_mox_rust_core.ps1` 和 `scripts/verify_mox_rust_core.sh`：
  - 设置 `MOX_RUST_CORE=force`；
  - 运行 `cargo test -p mox-formulas-core -p mox-norm-core -p mox-intent-core -p xiaobai-dsp`；
  - 运行关键 mocha 套件；
  - 运行性能 harness 并校验关键阈值；
  - 打印 `== MOX RUST CORE ACCEPTANCE: PASS | FAIL ==`。

**TR**：
- TR (rule): 脚本在一台干净机器（安装过 napi-rs + maturin）上一键跑完，最终输出 `PASS` 且 exit code = 0。
- TR (rule): `review.md` 所引用的全部证据文件路径存在且可读。
