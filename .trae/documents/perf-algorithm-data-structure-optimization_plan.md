# 性能 x 算法 x 数据结构 最优性 全维归一化 Plan（精简版）

## 一、基线问题 P0（按严重度）

| ID | 模块 | 当前复杂度 | 最优复杂度 | 严重度 |
|---|---|---|---|---|
| A1 | graph-algorithms/lib.rs pagerank（dense NxN matrix） | O(iter*N^2) 时空 | CSR 稀疏 O(iter*(N+E)) | P0 Critical |
| A2 | degree_matrix/laplacian（dense row_sum loop） | O(N^2) | O(E) 直接度数累加 | P0 Critical |
| B1 | slo-tracker.js record（splice(0,drop) + snapshot 4x sort） | O(N) append + 4*K log K | 环形缓冲 O(1) append + 1 趟窗口分流+排序复用 | P1 High |
| B2 | graph-formulas.js call_rust_algo（双 stringify、缓存未 parsed、每次 stat） | 2x serialize/parse + 3x stat/call | 1x stringify、cache parsed object、1x init CLI path | P1 High |
| B3 | closeness_centrality（对每个节点 Dijkstra），centrality_metrics 4次造 adj | O(N*(E+N log N)) + 4*O(N^2) | 未加权 BFS + 1次 CSR 复用 | P2 Medium |

参考经验 #1307001 (DB CPU 100% 定位法)：先基线再优化；#1498698 (缓存有效性)：按数据稳定性分 TTL。

## 二、改动文件
- platform/services/graph-algorithms/src/lib.rs：新增 CSR 邻接视图 + pagerank_csr；degree_matrix 改为 O(E) 度数
- platform/backend-node/src/slo-tracker.js：头指针环形缓冲 + 单趟分流 + 排序复用
- platform/backend-node/src/graph/graph-formulas.js：1x stringify、缓存 parsed、CLI 初始化一次
- 新 test-perf-algorithms.js + test-slo-performance.js（TDD 基线）
- 新 文档：专家联盟 5 步法 + 算法联盟 6 步法（流程归一化）

## 三、步骤（依赖有序）
0. TDD 前置：写性能基准测试（Red 失败再实施）
1. Rust CSR PageRank + O(E) 度（Pearson>0.9999）
2. SloTracker 环形缓冲替换 splice
3. graph-formulas 缓存/序列化常数优化
4. 专家/算法联盟流程文档化
5. D1~D6 36 TR + cargo test/clippy 全绿验证

## 四、验证
- V1：N=1000 E=4000 CSR PageRank >=5x 快且结果一致
- V2：100k SloTracker records <=500ms；100 snapshot <=2s
- V3：Rust 热命中 call 100x <= 20ms（比基线 2x 快）
- V4：36 TR 全绿 + clippy 0 ERROR

## 五、风险
- CSR 与 Dense 浮点误差：Pearson 双门槛
- 小图 N<500 dense 保留 fast-path
- 所有优化保留 LEGACY 环境变量回滚开关
