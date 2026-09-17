# cloud 域模块化拆分评估（Ready-To-Split）

> **产出日期**：2026-09-17
> **评估范围**：`platform/domains/cloud/` 全部 13 crate（api1 / core4 / svc6 / sdk2），源码规模逐 crate 实测
> **性质**：L7 过程证据（只读盘点 + 拆分候选排序，未改动任何代码）
> **方法**：对标 RustFS `ecstore-module-split-plan.md` 的"契约优先 + Ready-To-Split checklist"（见 `docs/working-reports/_norm_research/rustfs-modularity-benchmark.md` §7.3）
> **结论先行**：cloud 域**当前不建议拆 crate**——kernel（6,937 行）与 rebalance（2,345 行）内联单测充分（250/87 测试全绿，见 §2），主要缺口是**公共 API 集成测试缺失**（已补，见 §4.1）与边界治理；先完成边界收敛，再按 §4 候选顺序评估拆分。

---

## 1. crate 规模盘点（2026-09-17 实测）

| crate | 层 | src 文件 | src 行数 | tests 文件 | tests 行数 | 定位 |
|---|---:|---:|---:|---:|---:|---|
| `mox-cloud-s3-svc` | svc | 28 | **11,019** | 3 | 2,212 | S3 语义/签名/版本/生命周期/复制/MPU/策略 |
| `mox-cloud-kernel` | core | 11 | 6,937 | 2¹ | 550 | Reed-Solomon/GF256-SIMD/多写者/对冲读/缓冲池 |
| `mox-cloud-filer-svc` | svc | 14 | 6,936 | 2 | 1,431 | POSIX 文件服务/元数据多后端/配额/快照 |
| `mox-cloud-store-core` | core | 16 | 4,099 | 3 | 833 | 多后端存储/去重/位腐/GC/heal/版本 |
| `mox-cloud-volume-svc` | svc | 10 | 3,658 | 3 | 2,439 | 卷服务/EC 扩展/存储分层/重建 |
| `mox-cloud-master-svc` | svc | 8 | 3,311 | 3 | 2,663 | 控制面：raft/调度/卷分配/副本 |
| `mox-cloud-rebalance-svc` | svc | 4 | 2,345 | 3¹ | 516 | 再均衡：迁移任务/放置策略/控制器 |
| `mox-cloud-kb-core` | core | 7 | 1,431 | 2 | 553 | 知识库向量化/索引/重排（非存储内核） |
| `mox-cloud-domain-traits` | core | 7 | 1,160 | 0 | 0 | 存储契约 trait（StorageBackend 等） |
| `mox-cloud-sdk` | sdk | 10 | 801 | 1 | 319 | 客户端 SDK |
| `mox-cloud-admin-sdk` | sdk | 1 | 361 | 0 | 0 | 管理 SDK |
| `mox-cloud-api` | api | 2 | 204 | 0 | 0 | 契约 DTO |
| `mox-cloud-server` | svc | 1 | 187 | 0 | 0 | 独立进程入口（3412） |

**合计**：src ≈ 42.5K 行 / tests（tests/ 目录）≈ 11.5K 行。
¹ 2026-09-17 补齐的公共 API 黑盒集成测试（C1/C2 落地）：kernel 2 文件 28 用例 / 550 行、rebalance 3 文件 25 用例 / 516 行。

## 2. 测试现状与风险发现（评估副产品）

> **口径修正（2026-09-17）**：初版将"无 tests/ 目录"误判为"零测试"。实际 `mox-cloud-kernel` 与 `mox-cloud-rebalance-svc` 均内联了大量 `#[cfg(test)]` 单测（kernel 222 用例、rebalance 62 用例，`cargo test` 全绿）。真实的缺口是**公共 API 黑盒集成测试缺失**（tests/ 目录），已于当日补齐。

| # | 风险 | 证据 | 影响 | 优先级 |
|---|---|---|---|---|
| R1 | `mox-cloud-kernel` 公共 API 无黑盒集成测试（内联单测 222 用例已绿） | 初版无 tests/ 目录 | 纠删码是数据可靠性核心，公共契约（encode/decode/verify/multi-writer/hedged-read）回归无保护 | 🔴 P1（已补，见 §4.1） |
| R2 | `mox-cloud-rebalance-svc` 公共 API 无黑盒集成测试（内联单测 62 用例已绿） | 初版无 tests/ 目录 | 再均衡动数据，放置策略/迁移状态机/控制器公共契约无保护 | 🔴 P1（已补，见 §4.1） |
| R3 | `mox-cloud-s3-svc` 单 crate 11K 行 / 28 文件 | 与 RustFS `bucket/replication` 膨胀同型风险 | 边界纪律难维持 | 🟡 P2 |
| R4 | `mox-cloud-domain-traits`（1,160 行契约）与实现耦合方向未机器校验 | 无依赖方向脚本（对标 RustFS `check_layer_dependencies.sh`） | 契约被实现反向依赖 | 🟡 P2 |

## 3. Ready-To-Split checklist 现状（六项全过才可拆 crate）

| 项 | 现状 | 判定 |
|---|---|---|
| 1) 依赖图无环（与 trait/运行时/owner 边界） | 未做机器校验（无 cargo-machete/check 脚本） | ⚠️ 未验证 |
| 2) 契约 trait 编译不 import 实现模块 | `mox-cloud-domain-traits` 疑似纯净（1,160 行），未机器验证 | ⚠️ 待验证 |
| 3) 旧 facade 名有兼容测试/显式弃用 | 无 facade 层（当前直接引用），无 compat 标记机制 | ❌ 未满足 |
| 4) 聚焦测试覆盖被改 owner 路径 | kernel/rebalance 内联单测充分 + 公共 API 集成测试已补（2026-09-17） | ✅ 已满足 |
| 5) 回滚保持 IO/放置/quorum/队列/审计兼容 | 无显式"热路径不漂移"红线文档 | ❌ 未满足 |
| 6) 文档-代码同步 | 无 cloud 边界文档（本报告与 S2 声明补齐中） | ❌ 未满足 |

**结论：六项未全过，cloud 域现阶段不满足拆 crate 条件。**

## 4. 候选动作排序（先测试 → 再边界 → 后评估拆分）

### 4.1 立即（P1，不拆 crate）

| # | 动作 | 验收 |
|---|---|---|
| C1 | `mox-cloud-kernel` 公共 API 黑盒集成测试：reed_solomon 编解码往返/验证 fail-closed/重建/SIMD-Scalar 一致性 + multi_writer/hedged_reader/buffer_pool/backpressure/scanner 公共契约 | `cargo test -p mox-cloud-kernel` 绿（222 内联 + 28 集成）；clippy --all-targets 无测试 warning ✅ 2026-09-17 完成 |
| C2 | `mox-cloud-rebalance-svc` 公共 API 黑盒集成测试：placement 选择/排名/约束/副本分散/均衡度 + migration 生命周期/优先级/并发上限/断点 + controller 计划/tick/状态机 | `cargo test -p mox-cloud-rebalance-svc` 绿（62 内联 + 25 集成）；clippy --all-targets 无测试 warning ✅ 2026-09-17 完成 |

### 4.2 近期（P2，边界收敛）

| # | 动作 | 验收 |
|---|---|---|
| C3 | 新增依赖方向校验：core crate 不得依赖 svc；`mox-cloud-domain-traits` 不得被 svc 反向污染（脚本或 CI 步骤） | CI 出现 cloud 依赖方向检查 |
| C4 | `mox-cloud-s3-svc` 内部边界收敛：s3_server/persist/glacier_http 等运行时模块与 lifecycle/replication/versioning 策略模块之间访问走本地边界文件（对齐 RustFS `*_boundary.rs`） | 边界文件建立，直接跨模块 import 归零 |
| C5 | 热路径红线声明：写入放置/EC 配置/quorum 不漂移（见 `NORMALIZED_ARCHITECTURE.md` §4.4） | §4.4 已并入权威稿 |

### 4.3 中期（P3，拆分评估，六项全过后）

候选顺序（每个候选先过 §3 checklist 再动）：

1. **`mox-cloud-s3-svc` 契约提取**：把 `mox-cloud-s3-ops`（S3 操作/策略/生命周期规则纯契约）从 svc 提出——对齐 RustFS `s3-ops`/`storage-api` 思路；
2. **`mox-cloud-store-core` 后端边界**：fs_backend/s3_backend/kv_backend 已是模块级多后端，评估是否按 trait 再收敛（不急拆 crate）；
3. **`mox-cloud-kernel` 保持独立**：它已是正确的独立内核 crate，测试已充分（内联 222 + 集成 28），无需因拆分而改动。

## 5. 与 RustFS 对标小结

- MOX cloud 域的**结构起点优于 RustFS 迁移前**：13 crate 按 api/proto/core/svc/sdk 分层，`mox-cloud-kernel`（纠删码）已独立成 crate——RustFS 的 ecstore 是 265 文件/28.8 万行的巨型 crate，MOX 没有这个历史包袱。
- 差距在**治理**：RustFS 有 `*_boundary.rs` + compat 标记 + 依赖方向脚本 + 热路径红线文档；MOX cloud 域缺这些（C3/C4/C5 补齐）。
- **最大教训**：RustFS `bucket/replication` 因"契约提取后运行时继续膨胀"（+79%），MOX 应避免在 s3-svc 上重演——新 S3 功能代码若不需要 s3-svc 运行时状态，优先落 `mox-cloud-s3-ops`（如建立）而非塞进 svc。

---

*L7 过程证据 · 2026-09-17 · 盘点为只读；2026-09-17 依 C1/C2 落地补测（kernel 2 文件 / rebalance 3 文件，仅新增 tests/ 与 dev-deps，未动生产逻辑）。*
