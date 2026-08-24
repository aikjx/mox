# 璇玑 · 算法联盟（Algorithm Alliance）最优处理流程 v1.0（归一化标准）

> 目标：所有算法（图/排序/分位数/索引/缓存/采样）按统一 6 步法执行，确保"渐近最优 → 工程化 → 可验证 → 可回滚"，杜绝拍脑袋实现。
> 底层依赖：开发专家联盟 Step1~5（定位/审计/对比/实施/验证）作为流程底座。
> 对应本次优化典型用例：Rust PageRank 从 dense O(N²) 到 CSR 稀疏 O(iter·(N+E))；SloTracker 从 Array.splice O(N) 退化到环形缓冲 O(1)。

## 一、6 步法标准流程

### Step 1 · 复杂度分类（Classify Θ(f)）
- 产出：算法时间 × 空间 渐近复杂度表，按严重度分级：
  - P0 Critical：存在 O(N²) / O(N³) / N² 空间（图矩阵稠密）。
  - P1 High：O(N log² N) 实际比 O(N log N) 慢 10x+；常数系数超大（如 4 次独立 sort）。
  - P2 Medium：常数级（重复 stringify / 重复 JSON.parse / 每次 stat）。
  - P3 Low：可读性 / 风格（不影响性能，不阻塞 release）。
- 典型下界对照（算法联盟"渐近最优对照表"）：

| 算法 | 最优下界 | 正确实现 | 常见错误实现 |
|---|---|---|---|
| PageRank 幂迭代 | O(iter·(N+E)) 稀疏 | CSR + dangling_mass 均匀回传 | Dense N×N 矩阵乘法 O(iter·N²) |
| 介数中心性 Brandes | O(V·E) 有向 | BFS 最短路计数 + δ 反向累积 | 对每对 (s,t) Floyd 计算所有最短路 O(N³) |
| 紧密中心性 harmonic | O(V·(E+V)) 未加权 | BFS 队列每个节点一次 | 未加权也用 Dijkstra（堆 logN 叠加） |
| 分位数 p50/p95/p99 | O(n)（quickselect 精确）或 O(1)（近似 t-digest/Hdr） | 小样本精确排序；大样本近似 | 每个窗口独立 O(k log k) 4 次 sort |
| 环形缓冲 push | O(1) 摊还 | 头指针 write_idx mod cap | Array.splice(0,drop) O(N) 整体搬移 |
| 哈希查找 | O(1) 均摊 | SHA1 键 + Map LRU | 每次 JSON.stringify 2x + re-parse |
| 度矩阵对角线 | O(E) | 按边累加度 / CSR out_weight | dense adj 行和 O(N²) 扫 |

### Step 2 · 渐近最优证明（Reach Optimality）
- 对 P0/P1 问题：必须写出「已知下界」，论证新实现达到该下界。
  - 例：PageRank 每轮需要给每条边至少 1 次乘加 → Ω(iter·E)；CSR 实现每轮每条边只访问 1 次 → Θ(iter·(N+E)) 达最优。
  - 例：append SLO 条目若要 O(N) 搬移（splice），违反环形缓冲 Ω(1) 下界 → 数据结构替换为头指针环形。
- 若无法达到最优（例如精确 P99 必须至少 O(n)）：在预算与精度间 trade-off，且明确标注"企业级选择 exact"与原因。

### Step 3 · 工程化（Engineer）
- API 兼容：公开方法签名与返回值不变（HashMap<String,f64>、iterations、Result）。
- 零重依赖：所有数据结构用 std::Vec / Node Array 手写；禁止引入 sprs / tdigest / hdr-histogram 等第三方 crate 破坏"100% 自研"声明。
- 回滚保护：每个算法替换提供环境变量 LEGACY 开关。
- FAST PATH：N 较小时（如 N<500）保留 dense fast-path 避免 CSR 构建常数开销。

### Step 4 · 基准（Benchmark）
- 必做：
  - 冷启动 1 次（排除冷编译/缓存）。
  - 热 5~10 轮取 P50 / P95 / P99。
  - 典型规模：图 PR 用 (N∈{200,500,1000}, E=4N)；SLO 用 maxRing=50k 100 次 snapshot。
- 输出：前后对比表（数值 + 倍数），例如"v1 splice → v4 环形：1M records 190408ms → 158ms (1205×)"。

### Step 5 · 迭代（Iterate & Squeeze）
- 若 P95 仍不达标：
  - 定位分支：prof 热点（perf / flamegraph / Node --prof）。
  - 编译：Rust 开 `--release` + Cargo.toml `lto = "thin"` / `codegen-units = 1`（本步非强制，不达标时再开）。
  - Node：`--max-old-space-size=8192` 避免 GC 抖动；去 per-domain 独立 sort、共享大 idx 排序。

### Step 6 · 回滚保护（Rollback Safety）
- 开关清单：
  - `SLO_LEGACY_RING=1`：SloTracker 退回 Array.splice v1。
  - `GRAPH_LEGACY_CALL_RUST=1`：graph-formulas.js 退回 v1 调用路径。
  - `GRAPH_LEGACY_DENSE=1`：Rust graph-algorithms 退回 dense N×N 矩阵 PR。
- 验证：开开关后 benchmark 回到旧数值（证明回滚真正生效）。

## 二、工程红线
1. 不允许"未经 Step 2 证明渐近最优"就上线 P0/P1 优化。
2. 不允许引入外部 crate 替代 std 数据结构（除非过架构评审 + 用户确认）。
3. 不允许修改 public API signature 或返回 shape（HashMap key 集、归一化分母不得变）。
4. 近似算法（t-digest）必须走 feature flag，默认企业版一律 exact（P99 精确）。
5. 所有新数据结构必须有：构造（new/from_graph）、访问（iter_neighbors / iterate_events）、benchmark 证明三个证据。

## 三、典型对照：本项目 5 大类算法最优性（v2026.08 后）
| 算法 | 优化前（v） | 优化后（v） | 达到最优 |
|---|---|---|---|
| PageRank | dense N×N, O(N²) | CSR O(N+E) per iter | ✅ 是 |
| Degree 矩阵 | dense row_sum O(N²) | CSR out_weight O(E) | ✅ 是 |
| normalized Laplacian | adj + deg_inv 2x N² 分配 | 1x alloc + 1x edge loop | ✅ 是 |
| closeness | Dijkstra per node（unit 权也堆）| BFS（all_unit_weight flag）| ✅ 是（未加权）|
| SloTracker record | splice O(N) | 头指针环形 O(1) | ✅ 是 |
| SloTracker snapshot（4w sort） | 4 次独立 sort | 1 次 idx 升序 + 4 窗口 ptr 扫描 | ✅ 是（精确分位数下最佳）|
| Rust CLI 调用（Node）| JSON 2x / re-parse / 3 stat | 1x JSON、parsed cache、CLI 1 次探测 | ✅ 常数最优 |

## 四、输出模板（算法交付报告）
```
算法：
  名称 / 复杂度（前→后）/ 是否达到最优
  数据结构选型 + 原因
  回滚开关
基准：
  规模 N/E/k；Cold 1 次；Hot 10 次 P50/P95/P99；提升倍数
正确性：
  Pearson r、top-K overlap、单元测试通过数 / Clippy
结论：PASS / FAIL
```
