# 璇玑 · 开发专家联盟（Expert Alliance）处理模式 v1.0（归一化标准）

> 💡 **术语说明**：璇玑/Mox，指同一系统，代码中统一使用 mox- 前缀。

> 目标：所有研发动作（需求→架构→代码→测试→运维）按统一 5 步法执行，杜绝拍脑袋、重复造轮子、不可验证。
> 适用范围：Rust 后端 21 crate + Node 企业服务层 + 前端/脚本/运维等全链路。
> 底层中枢：璇玑全域知识图谱（需求↔业务↔架构↔代码↔测试 双向绑定）。
> 参考经验：#1307001（性能问题必须先定位后改代码，禁止先改再找根因）、#1498698（缓存/改动必须有命中证据与 TTL 区分，缺证据的"已生效"不接受）。

## 一、5 步法标准流程（每步必须留 commit/doc 证据）

### Step 1 · 定位（Locate）
- 输入：用户需求 / bug 报告 / 性能告警 / 回归失败。
- 输出：`问题 ID`、`代码定位（文件 + 行号范围 + 函数）`、`实际调用链（从入口到根因，按文件顺序列出）`、`数据规模与形态（若涉及性能，如 N/E/QPS）`、`严重度分级（P0/P1/P2/P3）`。
- 必做检查：
  - (a) 使用 Grep/Read 真实读代码，不能"我猜"。
  - (b) 若用户质疑性能变慢：找到具体函数、循环、查询次数、调用栈，给出数量级估算（经验 #1498698）。
  - (c) 任何"DB CPU / 性能 100%"类问题：先定位 DB 侧证据（锁、长时间 running、聚合、全表扫），再改代码（经验 #1307001）。

### Step 2 · 审计（Audit）
- 输入：Step 1 定位结论。
- 输出：`现状 vs 理论最优差距表（复杂度、内存、调用次数）`、`基线 benchmark（真实跑出的 baseline 数值）`、`业界对标（NetworkX/petgraph/igraph 对应算法实现方式）`。
- 原则：
  - 没有 baseline 的优化 = 打补丁（经验 #1307001 反模式）。
  - 所有优化点必须定量："从 O(N²)→O(E)" 不是文字，必须附 N=5000/E=20000 时的 FLOPs 估算。

### Step 3 · 对比（Compare & Decide）
- 任务：对照 3+ 种业界/开源实现方式，明确选择"最优渐近 + 零重依赖 + 与本项目 AIS 架构兼容"的实现。
- 决策记录：`为什么选 CSR 不选邻接哈希表？`、`为什么保留 sorted 精确分位数不采用 t-digest 近似？`（附 trade-off：对企业 P95 需要精确 ±1 桶误差 → 选择 exact）。
- 自研红线：新增 runtime crate 依赖需过 AIS L5 架构评审；默认用 std 手写。

### Step 4 · 实施（Implement）
- TDD 铁律：先写性能/正确性测试（Red）→ 再实现（Green）。
- 顺序：底层数据结构 → 上层算法 → 业务绑定 → 文档。
- 归一化铁律：
  - 单源真相：同一个算法（PageRank/介数/PRF 意图/日志）只能有 1 个真实现（Node→Rust 委托时，Rust 为真源，Node 仅留薄壳）。
  - DIP：高层（mox-system / orchestrator）只依赖 trait，不依赖具体 services crate。
  - 回滚保护：所有行为变更保留 `LEGACY_*` env flag（例：SLO_LEGACY_RING、GRAPH_LEGACY_DENSE、GRAPH_LEGACY_CALL_RUST）。
  - TTL 分级缓存：对"不再变化的数据"长 TTL，对"正在写入的数据"与粒度一致 TTL（经验 #1498698）。

### Step 5 · 验证（Verify & Report）
- 必须包含：
  - (a) 单元/集成测试通过数量与退出码。
  - (b) Clippy `-D warnings` exit 0。
  - (c) 性能优化前后对比表（数值 + 提升倍数 + 样本规模）。
  - (d) D1-D6 36 TR 回归全绿。
  - (e) 回滚开关验证：开 LEGACY 后路径确实走旧代码（用 benchmark 或日志）。
- 没有验证 = 没完成（TRAE-verification 铁律）。

## 二、AIS 分层归一化约束（6 层 DIP 倒置）
| 层 | 代码 | 依赖规则 |
|---|---|---|
| L1 基础算子 operator-core | crates/operator-core | 无业务依赖 |
| L2 公共元 mox-common-meta | crates/mox-common-meta | 仅依赖 L1 |
| L3 算法内核 graph-algorithms/primiflow-core | platform/services/graph-algorithms | 仅依赖 L1/L2，不依赖 L5 |
| L4 服务层 mox-expert/runtime/... | platform/services/* | 依赖 trait 抽象（L3），不依赖实现 |
| L5 编排层 mox-system orchestrator | mox-system | 通过 `dyn Trait` 依赖 L4 抽象，禁止 `use crate::services::*` |
| L6 应用层 Node backend / frontend | platform/backend-node, ui/* | 通过 HTTP/CLI 与 L5 解耦 |

## 三、反模式清单（一票否决）
1. 未定位根因先改代码（经验 #1307001 反模式）。
2. 声称"已生效/已验证"但无命令退出码或日志片段（经验 #1498698 反模式）。
3. 同一算法存在 2 份互不等价实现（违反单源归一化）。
4. 引入重框架/脚手架破坏纯自研声明。
5. 无 LEGACY 回滚开关的行为性重构。
6. 手工时区双重转换（Date UTC+8 再次 -8h 偏移）。
7. 重复声明索引 / 重复调用 `appendLog` 导致双写 race。
8. 对测试写 `todo!() / unimplemented!() / stub 占位` 合并到 production 分支。

## 四、验收文档模板
```
交付物：
  - 代码改动：文件 + 行号 + 摘要
  - 基线 benchmark：优化前 数值、样本、退出码
  - 优化 benchmark：优化后 数值、提升倍数
  - 回归：cargo test / cargo clippy -D warnings / D1~D6 36 TR
  - 回滚：LEGACY_* 开/关 2 套均能正常通过
  - 结论：PASS / FAIL（附证据链接）
```
