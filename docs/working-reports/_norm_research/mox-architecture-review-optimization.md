# MOX / 璇玑 架构总结·分析·优化（含 RustFS 对标）

> **产出日期**：2026-09-17
> **分析范围**：本仓库全局（12 业务域 / 143 crate / 六层 / 四进程），cloud 域作专项；
> 对标基准 = `docs/working-reports/_norm_research/rustfs-modularity-benchmark.md`（RustFS 模块化归一化分析，同日产出）
> **性质**：L7 过程证据（只读分析，未改动任何源码）
> **事实基准**（2026-09-17 实测/核对，与前文权威文档交叉验证）：
> - workspace **143 个 crate**（根 `Cargo.toml` `[workspace].members` 实测）
> - 业务域 **12 个**（ai/alliance/base/cloud/data/flow/kb/kg/market/platform/project/voice）+ foundation 基座（2 crate）等非业务分组，合计 143
> - 网关 **3080** 唯一 HTTP 入口，**223 条路由**（`actuator.rs` `ROUTES` 生成，`API-REGISTRY.md`）
> - **46 个域描述符全部 ready**（`actuator.rs` `/api/v1/domains` 自述"46 业务域描述符"）
> - 企业默认**四进程**：网关 3080 / operator-server 3001 / alliance-scheduler 3100 / alliance-executor 3200；可选独立角色 3411–3414
> - `platform_config.json` 登记 **8 个服务**（api/frontend/xiaobai_voice/melody2score/primiflow/operator-server/mox-alliance-scheduler/mox-alliance-executor）

---

## 一、总结：架构现状（事实基线）

### 1.1 总规模

| 项 | 数值 | 核对方式 |
|---|---:|---|
| workspace crate | **143** | `Cargo.toml` members 实测 |
| 业务域 | **12**（+foundation 基座 2） | `platform/domains/*` |
| 网关内嵌业务面 | 13 个（actuator/platform/kg/ai/kb/alliance/system/experts/monitor/projects/workspace/notification/misc） | `API-REGISTRY.md` §1 |
| 域描述符 | **46 个全部 ready**（0 stub / 0 beta） | `actuator.rs` + commit `2cd3cc8a` |
| 注册路由 | **223 条**（全部有真实实现） | `API-REGISTRY.md` §1 |
| 企业默认进程 | **4** | `scripts/startup/start-mox-enterprise.ps1` |
| 配置登记服务 | **8** | `platform_config.json` |
| 网关内嵌模块 | 7 个（KG/KB/Cloud/IAM/RBAC/联盟任务域/专家广场） | `NORMALIZED_ARCHITECTURE.md` §4.2 |

### 1.2 分层与命名（MOX 结构性优势）

```
L0 foundation（横切基座：error/audit/observability/paths/api-protocol/framework…）
  ↓
L1 api（各域契约 DTO，零内部业务依赖）
  ↓
L2 proto（gRPC/服务间契约，仅依赖 api）
  ↓
L3 core（纯计算、无 IO，mox-<域>-<能力>-core）
  ↓
L4 svc（服务实现，依赖 api+proto+core）
  ↓
L5 gateway（唯一入口 mox-server :3080，装配路由+鉴权+反代）
```

命名公式 `mox-<域>-<层>-<角色>`（层 ∈ api/proto/core/svc/sdk），**目录即域**（`platform/domains/<域>/<层>/`），"一个主题一个目录、一个事实一个权威源、路径即契约"。

### 1.3 十二域矩阵（摘要）

| 域 | crate 数 | 定位 | 完成度 |
|---|---:|---|---|
| platform | 22 | 平台内核：IAM/DSQL/元数据/编排/企业治理 | 🟢 |
| flow | 18 | 工作流：12 个 unified-* 内核 + WASM/PrimiFlow | 🟢 |
| alliance | 13 | 专家联盟：10 专家 + 6 融合策略 + 调度/执行 | 🟢 |
| cloud | 13 | 云存储：master/volume/s3/filer/rebalance + 纠删码 | 🟢 |
| ai | 12 | AI：意图/专家调度/Flow/Agent 运行时 | 🟢 |
| kg | 12 | 知识图谱：算法/元数据 + 8 svc | 🟢 |
| data | 10 | 数据治理：公式/归一化/标准 + ETL/合规/目录 | 🟢 |
| voice | 8 | 语音：ASR/意图/DSP + Python 绑定 | 🟢 |
| base | 7 | 纯内核抽象：model/store/index/graph/query/perm/lifecycle | 🟢 |
| kb / project / market | 2/2/2 | 知识库 / 项目图 / 模板市场 | 🟢 |

### 1.4 进程拓扑

- **网关 3080 进程内内嵌**：KG/KB/Cloud/IAM/RBAC/联盟任务域/专家广场（7 模块统一 merge + 统一鉴权 HS256）。
- **HTTP 后连**：未命中的 `/api/*` catch-all → operator-server :3001（注入 `OUS_API_TOKEN`）；`/api/projects/*` → PrimiFlow :8000；联盟调度 → 3100 → 3200（HTTP 桥接，可用环境变量切本地/远程）。
- **可选独立扩展**：同一二进制 + `MOX_HOST_ROLE` → kg:3411 / cloud:3412 / iam:3413 / kb:3414（默认 fused 不另起）。

### 1.5 治理体系现状

- **生成器闭环（声明即实现）**：`actuator.rs ROUTES` → `gen-api-registry.py` → `API-REGISTRY.md`；`PORT-REGISTRY.md` + `verify-ports.py`；`check-doc-links.py`；`AGENTS.md`。
- **Clippy 门禁**：`[workspace.lints.clippy]`（correctness/suspicious deny 组，`-D warnings` 执行）。
- **域归一化**：46 域全部 ready（commit `2cd3cc8a`），9 能力组（platform/knowledge/ai/orchestration/storage/data/media/commerce/streaming）。
- **模块化自评**（`MODULARITY.md`，2026-09-13）：**总分 ≈ 4.3/5**，六准则中解耦/一致性/声明诚实度最高（4.5），可观测 3.5→4.0。

---

## 二、分析：问题与差距（对标 RustFS）

### 2.1 六准则评分：MOX vs RustFS

| 准则 | MOX | RustFS | 差距解读 |
|---|---|---:|---:|---|
| 内聚（域边界） | 4.5 | 4.5 | 同档：MOX 目录即域更强；RustFS 契约 crate 独立更细 |
| 解耦（依赖单向） | 4.5 | 4.0 | MOX 六层单向 + 命名公式优于 RustFS 扁平；RustFS 巨型 ecstore 内部耦合 |
| 可演进（渐进拆分） | 4.5 | 3.5 | MOX 进程面已分层（kb/scheduler/executor 示范）；RustFS 拆分滞后于特性增长 |
| 可替换（后端抽象） | 4.0 | 4.0 | 同档：MOX StorageBackend/任务仓储可插拔；RustFS facade+feature 门控 |
| 可观测（可诊断） | 4.0 | 4.0 | 同档 |
| 一致性（单一事实源） | 4.0 | 4.5 | **RustFS 胜出**：PR 类型门禁 + CI 强制文档锚点 + check_doc_paths；MOX 生成器未入 CI，漂移仍在发生 |
| 声明诚实度（不虚标） | 4.5 | 4.5 | 同档：MOX 修过"联盟任务仓储"过时描述；RustFS 不变量 ⚠️ 显式标注 |

**总体**：MOX 结构与演进占优，**治理闭环（一致性）是相对短板**——与 RustFS 的最大差距不在代码结构，而在"迁移过程治理"。

### 2.2 已核实的问题清单（含证据与优先级）

| # | 问题 | 证据（2026-09-17 实测） | 影响 | 优先级 |
|---|---|---|---|---|
| P1-1 | **生成器未入 CI，文档漂移仍在发生** | `API-REGISTRY.md` §1 仍写"域描述符 43 个：ready 7 · beta 1 · stub 35"；`actuator.rs` 已是"46 业务域描述符"全部 ready——两份权威打架 | 权威源失效，新人按文档得到过时事实 | 🔴 P1 |
| P1-2 | **配置登记口径漂移** | `platform_config.json` 实测 **8 服务**；`MODULARITY.md` 记"已登记 9 服务" | 事实口径不一致 | 🔴 P1 |
| P1-3 | **多版本架构稿并行** | `doc-landscape.md` 盘点 8 篇并行/过期权威（architecture.md / SYSTEM-OVERVIEW / OPTIMAL / ARCHITECTURE_DESIGN_v3.1 / BASELINE / ARCHITECTURE-ENTERPRISE / rust-enterprise…） | 同主题多权威，检索成本高 | 🔴 P1 |
| P2-1 | **跨域直连 24 处收敛中** | `NORMALIZED_ARCHITECTURE.md` §7 结论 | 层间依赖纪律靠人工 | 🟡 P2 |
| P2-2 | **无 PR 类型门禁** | RustFS 有 10 类"一次一 PR"；MOX 无对应规范 | 混提 PR 难审查、难回滚 | 🟡 P2 |
| P2-3 | **无 boundary/兼容标记机制** | RustFS 有 `*_boundary.rs` + `RUSTFS_COMPAT_TODO` + cleanup register；MOX 历史遗留（旧 Python 栈、多版本稿）仅"头部标注" | 兼容代码静默常驻 | 🟡 P2 |
| P2-4 | **cloud 数据面/控制面无显式声明** | `mox-cloud-master-svc`（raft/scheduler/volume_allocator）与 `volume/s3`（数据面）边界未文档化，无"热路径不漂移"红线 | 热路径风险 | 🟡 P2 |
| P2-5 | **proto 契约无版本化承诺** | gRPC 契约内部使用，未语义化版本（`MODULARITY.md` 建议项） | 演进困难 | 🟡 P2 |
| P3-1 | **RustFsEcstoreBackend 骨架长期悬空** | `mox-cloud-s3-svc/src/storage/rustfs_ecstore.rs` 全部方法返回 `Unsupported`（feature 门控），无契约先行文档 | 接入点承诺过期 | 🟢 P3 |
| P3-2 | **workspace lints 可补强** | MOX：clippy correctness/suspicious 门禁；RustFS 另有 `unsafe_code=deny` + 统一 `[workspace.package]` 版本 | 安全基线 | 🟢 P3 |

### 2.3 对照小结

- **RustFS 强在"迁移治理"**：契约优先提取（lifecycle/replication 契约已独立）、facade 兼容面单调收缩、Ready-To-Split checklist、PR 门禁、三专家评审、文档路径 CI 门禁——**这些都是 MOX 可直接迁移的机制**。
- **MOX 强在"结构声明"**：目录即域、命名公式、六层单向、46 域全 ready、core 纯计算约束——**RustFS 的扁平布局与文档级分组反而弱于此**。
- 结论：**MOX 不缺结构，缺的是"让声明保持最新"的机器门禁**。P1-1/P1-2/P1-3 三个"口径漂移"问题本质同源：权威清单生成后无人重跑、无 diff 门禁。

---

## 三、优化：模块化归一化优化方案（落地清单）

> 优先级：🔴 P1 立即（本周）/ 🟡 P2 近期（本迭代）/ 🟢 P3 中期。每项给出验收标准。

### 3.1 治理层（补机器门禁，对标 RustFS）

| # | 动作 | 验收标准 | 优先级 |
|---|---|---|---|
| G1 | **生成器纳入 CI + diff 门禁**：`gen-api-registry.py` 重跑后 `git diff --exit-code`，漂移即 CI 失败 | `API-REGISTRY.md` 与 `actuator.rs` 永不同步失败；46 域/223 路由口径全库一致 | 🔴 P1 |
| G2 | **配置口径收敛**：`platform_config.json`（8 服务）设为唯一运行登记权威；`MODULARITY.md` 更正"9 服务"，新增服务必须先登记再启动 | 全库搜索"9 服务"归零 | 🔴 P1 |
| G3 | **多版本架构稿收敛**：按 `doc-landscape.md` 清单，8 篇并行稿头部标注"已被 `NORMALIZED_ARCHITECTURE.md` v2.0 取代"，不动内容（保留 git 历史） | `doc-landscape.md` 漂移项全部闭环 | 🔴 P1 |
| G4 | **PR 类型门禁规范**：新增 `docs/standards/PR-TYPES.md`，10 类（docs-only/test-only/contract/api-extraction/pure-move/consumer-migration/dependency-migration/security-change/behavior-change/ci-gate），一次一 PR | 架构迁移类 PR 全部声明类型 | 🟡 P2 |
| G5 | **兼容标记三件套**：`MOX_COMPAT_TODO(ARC-xxx)` 注释 + `docs/working-reports/_norm_research/compat-cleanup-register.md` 登记 + 独立 cleanup PR | 历史遗留（旧 Python 栈、多版本稿）全部登记 | 🟡 P2 |
| G6 | **架构迁移评审**：对跨域/跨层 PR 引入"结构/迁移保持/测试"三视角评审（可轻量化为 checklist，不必三专家） | 评审 checklist 进入 `AGENTS.md` | 🟢 P3 |

### 3.2 结构层

| # | 动作 | 验收标准 | 优先级 |
|---|---|---|---|
| S1 | **cloud 域 Ready-To-Split 评估**：参照 RustFS 六项 checklist 审视 `mox-cloud-s3-svc`（32 文件 / 13,295 行）与 `mox-cloud-filer-svc`，输出拆分候选排序（先契约后运行时） | 输出 `cloud-split-assessment.md`（L7），候选与理由明确 | 🟡 P2 |
| S2 | **数据面/控制面显式声明**：cloud 域新增边界文档：master=控制面（raft/scheduler/allocator 只读快照起步），volume/s3=数据面（热路径行为红线：放置/EC 配置/quorum 不漂移） | `NORMALIZED_ARCHITECTURE.md` cloud 节更新 | 🟡 P2 |
| S3 | **跨域直连收敛**：业务域间一律走 SDK/事件（`mox-event-core`）/平台编排；新增依赖必须过 `deny.toml` 或 review check | 24 处收敛过半 | 🟡 P2 |
| S4 | **core 层纯计算门禁**：新增 CI 检查 core crate 不得依赖 `tokio`/IO 类外部 crate | 100% 维持 | 🟢 P3 |

### 3.3 契约层

| # | 动作 | 验收标准 | 优先级 |
|---|---|---|---|
| C1 | **proto 契约语义化版本**：gRPC 服务版本后缀（v1），内部首版固化，后续改版走兼容共存 | `proto` 层出现版本化首版 | 🟡 P2 |
| C2 | **RustFS 对接先立契约**：在 `mox-cloud-domain-traits` 补齐 `RustFsEcstore` 契约（chunk 读写/元数据/EC profile/健康探测）与边界文件，再谈进程/FFI（详见 §4.2） | 契约 trait 编译通过，骨架方法不再裸 `Unsupported` | 🟢 P3 |

### 3.4 演进路线（继承 MODULARITY.md 三步 + 本次增量）

1. **近期（P1）**：G1–G3 三个口径漂移闭环 + cloud 数据面/控制面声明（S2）。
2. **中期（P2）**：PR 类型门禁（G4）+ cloud 拆分评估（S1）+ proto 版本化（C1）+ 跨域收敛（S3）。
3. **远期（P3）**：兼容标记全量登记与清理（G5/G6）+ RustFS 契约对接（C2）+ 按 `MOX_HOST_ROLE` 渐进拆分（3411–3414 上线）。

> 原则不变：**拆分是演进而非目标**；任何拆分由真实负载/团队边界驱动。

---

## 四、cloud 域专项（RustFS 对标落地）

### 4.1 能力映射

| RustFS | MOX cloud 域 | 状态 |
|---|---|---|
| `ecstore`（纠删码引擎） | `mox-cloud-kernel`（reed_solomon/multi_writer/hedged_reader）+ `mox-cloud-store-core`（erasure/dedup/heal/gc） | MOX 自研已就绪 |
| `storage-api`（契约面） | `mox-cloud-domain-traits`（StorageBackend/ChunkId/BackendCapabilities） | **同构** |
| `rio`/`io-core`（I/O 管线） | `mox-cloud-store-core`（stream_writer/cache/bitrot/多后端） | 已具备 |
| `scanner`/`heal`/`lifecycle`/`replication` | `mox-cloud-rebalance-svc` + s3-svc（lifecycle/replication/versioning/restore） | 已实现 |
| **ecstore 接入点** | `rustfs_ecstore.rs` 骨架（全部 `Unsupported`） | **待对接** |
| 控制面（ClusterControlPlane） | `mox-cloud-master-svc`（raft/scheduler/volume_allocator） | 已实现，边界未声明 |

### 4.2 RustFS ecstore 接入路径（契约先于进程）

1. **立契约**：在 `mox-cloud-domain-traits` 定义 `RustFsEcstoreContract`（put/get/delete/list chunk、EC profile、一致性模型、健康探测），替换骨架中的裸 `Unsupported` 为"契约已定义、传输未接"（保留显式状态）。
2. **立边界**：新增 `storage/rustfs_ecstore_boundary.rs`，s3-svc 其余代码不得直接触碰接入细节（对齐 RustFS `*_boundary.rs` 模式）。
3. **选传输**：按部署形态定 Unix Socket / gRPC / FFI（当前文档建议 socket/gRPC），写进契约注释与 `AGENTS.md`。
4. **接 feature flag**：`rustfs_ecstore_backend` 默认关闭、CI 覆盖编译路径（对齐 RustFS rio-v2 "ships in no default build" 的诚实声明）。
5. **验收**：一条真实 put/get 链路（写入 → EC 分片 → 读取重建）跑通后，才允许 `BackendType::RustFsEcstore` 出现在生产配置。

### 4.3 s3-svc 拆分建议（参照 RustFS ecstore 方法论）

现 `mox-cloud-s3-svc` 32 文件 / 13,295 行，同 crate 内已有 versioning/lifecycle/replication/policy/glacier/inventory 等 20+ 模块。参照 RustFS 经验：

- 先**盘点契约**：区分纯契约模块（s3-ops/s3-types 类）与运行时（s3_server/persist/glacier_http）；
- 不急于拆 crate：先做**边界模块收敛**（跨模块访问走本地 boundary）与**大文件拆分**（若存在 >5K 行单文件）；
- 满足 Ready-To-Split 六项 checklist 后再评估 `mox-cloud-s3-svc` 是否拆出 `mox-cloud-s3-ops`（契约）与 `mox-cloud-s3-svc`（运行时）。

---

## 五、证据与引用

### 5.1 实测证据（2026-09-17）

- `Cargo.toml`：143 members（PowerShell 实测计数）；`[workspace.lints.clippy]` 存在
- `platform/gateway/mox-platform-gateway-svc/src/actuator.rs`：`platform.domains` 路由描述"46 业务域描述符"；`r()` 状态列 ready
- `docs/API-REGISTRY.md`：§1 总览（223 条/13 内嵌域/**43 描述符 ready 7·stub 35——漂移项**）
- `platform_config.json`：version 3.0，services 8 个
- `docs/api/PORT-REGISTRY.md`：3080/3001/3100/3200/30010/3020/8000/8012/3999/3411–3414
- `platform/domains/cloud/svc/mox-cloud-s3-svc/`：32 .rs / 13,295 行；`storage/rustfs_ecstore.rs` 骨架（全部 `Unsupported`）

### 5.2 既有权威文档

- `docs/architecture/NORMALIZED_ARCHITECTURE.md` v2.0（143/12/六层/四进程/46 域）
- `docs/architecture/MODULARITY.md`（六维盘点与 4.3/5 评分、三步演进）
- `docs/working-reports/_norm_research/doc-landscape.md`（66 文档权威分级与 49 处漂移清单）
- `docs/working-reports/_norm_research/rustfs-modularity-benchmark.md`（RustFS 对标，同日产出）

### 5.3 外部来源

- JavaGuide《对标 MinIO！全新一代分布式文件系统正式发布》：https://mp.weixin.qq.com/s/ePJgcVAYYscAcKIG70C5yA

---

*L7 过程证据 · 2026-09-17 · 只读分析；建议按 §3 的 G1–G3 先行闭环口径漂移。*
