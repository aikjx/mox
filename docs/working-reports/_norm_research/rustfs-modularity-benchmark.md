# RustFS 架构模块化归一化分析 — 对标基准研究

> **产出日期**：2026-09-17
> **分析范围**：RustFS（对标 MinIO 的开源分布式对象存储）——文章报道（JavaGuide《对标 MinIO！全新一代分布式文件系统正式发布》）+
> 本地参考源码 `ais/RustFS/`（vendored 快照 `bc07cfd`，2026-08-21，`Cargo.toml` 版本 1.0.0-rc.3）
> **性质**：L7 过程证据（只读对标研究，未改动任何 RustFS 文件；结论用于本仓库模块化归一化参考）
> **事实基准**（全部经源码/文档核对）：
> - workspace = **47 成员**（1 主 crate `rustfs` + 46 库 crate，扁平 `crates/` 布局）
> - 源码规模 = **1,584 个 .rs 文件 / ≈98.5 万行**（含 `#[cfg(test)]` 内联测试）
> - 主数据面 = `HTTP → server → app/usecase → storage → ecstore → rio/io-core → disk`；端口 S3/Admin **:9000**、Console **:9001**、节点间 gRPC
> - 文章报道状态：2025-07 开源，2026-07 破 30K Star，截至发文约 **32.5K Star**，GA（1.0.x）已发布

---

## 一、结论速览

1. **RustFS 是当前开源对象存储中模块化治理最体系化的项目之一**：46 个库 crate 全部统一 `rustfs-` 前缀、依赖由 `workspace.dependencies` 集中管理、CI 脚本强制依赖方向与文档路径、PR 强制声明单一类型，并有完整的"契约优先"迁移方法论（backlog#660，Phase 0–7）。
2. **它的模块化是"扁平 crate + 分层主 crate + 契约优先拆分"三件套**：库 crate 按 6 个能力域分组（Foundation / I/O+Storage / Security / Protocols / Operations / Test）；主 crate 内部按 Server→Admin/App→Storage→ecstore→rio→io-core 严格向下分层。
3. **归一化评分 ≈ 4.3/5**（六准则 + 声明诚实度，口径与 `MODULARITY.md` 对齐）：解耦与一致性最强，可演进最弱——`crates/ecstore` 巨型 crate（265 文件 / ≈28.8 万行，单文件最大 2.1 万行）的拆分速度跟不上特性增长速度（`bucket/replication` 半年 +79%）。
4. **最值得借鉴的归一化机制**：①10 类 PR 门禁（一次只做一件事）；②`*_boundary.rs` 边界模块 + `rustfs_ecstore::api` facade 兼容面"单调收缩"；③`RUSTFS_COMPAT_TODO(API-xxx)` 兼容标记 + cleanup register；④`Ready-To-Split` 六项 checklist；⑤数据面/控制面分离与"热路径不漂移"红线。
5. **对本仓库（MOX）的启示集中在 cloud 域**：MOX 的 `mox-cloud-domain-traits::StorageBackend` 抽象已与 RustFS 的 storage-api 契约同构；待对接的 `RustFsEcstoreBackend` 骨架应参照 RustFS "契约先于进程对接"的路径推进（详见 §8）。

---

## 二、背景：文章与项目

- **文章**：JavaGuide 公众号《对标 MinIO！全新一代分布式文件系统正式发布》（2026-09-17），宣布 **RustFS GA**。
- **项目定位**：Rust 编写的开源分布式对象存储，**S3 API 兼容**，Apache-2.0 许可；图片/简历/附件/报表等文件存储场景；可替换 MinIO（保留 AWS SDK 与业务逻辑，只改 Endpoint/凭证/桶配置）。
- **文章强调的 GA 能力**（与模块化直接相关的部分）：
  - **版本控制**：同名覆盖产生新版本 ID，删除产生删除标记，可按版本 ID 找回（误删恢复）。
  - **对象锁定（WORM）**：限制版本删除，适合合规保留。
  - **纠删码**：Reed-Solomon，对象拆为数据块+校验块分布保存（官网图例 4 数据块 + 2 校验块）。
  - **存储池管理**：`RUSTFS_VOLUMES` 追加存储池扩容；已有对象留在原池，需**再均衡**才迁移。
  - **生命周期**：`logs/` 前缀 30 天过期等规则，后台扫描器异步执行。
  - **桶复制 / SSE 服务端加密 / 按需迁移**：回源读取 + 后台回填，存量 MinIO 数据可渐进迁移；`ListObjectsV2` 源端合并。
  - **MinIO 磁盘格式兼容（rio-v2）**：可读/导入 MinIO 未加密对象（标为预览，feature-gated 默认不编译）。
- **本仓库关联**：RustFS 源码完整 vendored 于 `ais/RustFS/`（与 minio/seaweedfs/ceph/juicefs 等并列作为第三方参考）；MOX cloud 域 `mox-cloud-s3-svc` 已定义 `RustFsEcstoreBackend` 接入点骨架（feature `rustfs_ecstore_backend`，当前全部返回 `Unsupported`，标注"实际 FFI/进程对接待后续阶段"）。

---

## 三、总体架构快照

### 3.1 部署形态与端口

| 面 | 端口 | 说明 |
|---|---|---|
| S3 API（数据面） | **9000** | 对象 CRUD 主路径；Admin API 同端口 `/minio/` 前缀 |
| Console（控制面 UI） | **9001** | Web 控制台，走 Admin API |
| 节点间 RPC | gRPC/tonic | 分布式模式集群通信 |

### 3.2 数据面请求管线（架构中心）

```mermaid
flowchart LR
    REQ["HTTP 请求"] --> SRV["server 层<br/>TLS/鉴权/路由/压缩"]
    SRV --> APP["app 层<br/>object/bucket/multipart usecase<br/>策略/生命周期校验"]
    APP --> STG["storage 层<br/>S3 语义翻译 / EC 文件系统"]
    STG --> ECS["ecstore<br/>存储池选择 / 数据分布"]
    ECS --> RIO["rio<br/>加密→压缩→哈希→写入管线"]
    RIO --> IOC["io-core<br/>缓冲池 / 存储画像 / 准入控制"]
    IOC --> DISK["本地盘 / RPC 远端盘"]
```

### 3.3 数据面 / 控制面分离（`storage-control-data-plane.md`）

- **StorageCore**：对象热路径（放、取、删、列、多段），行为红线：对象-集合哈希、写 quorum、读解密/etag/版本/删除标记**不得漂移**。
- **ECStore**：纠删码 + 位腐检测 + 远程盘传输与恢复的运行时宿主。
- **ClusterControlPlane**：只读快照起步（拓扑/成员/锁注册表/对端健康/池状态），显式禁止"未授权健康探测/RPC 健康检查"，读模型 crate-private（`rustfs_ecstore::api::cluster` 出口）。
- **BackgroundControllers**：scanner/heal/lifecycle/replication/config reload/metrics 等后台控制器，须在生命周期契约稳定后逐个收敛到显式控制器边界。

---

## 四、模块化现状盘点（六维）

### 4.1 Crate 层：双级结构

| 级 | 形态 | 规模 | 说明 |
|---|---|---|---|
| **库 crate 级** | 扁平 `crates/<name>/`，46 个库 crate | 46 | 每个 crate 一个能力；`Cargo.toml [workspace].members` 为权威清单 |
| **主 crate 级** | `rustfs/` 单一二进制+库 | 顶层约 60 个模块 | 按职责分层组织（见 4.4），通过 lib.rs 显式声明模块树 |

主 crate 顶层子域规模（源码核对）：

| 子域 | 文件数 | 行数 | 职责 |
|---|---:|---:|---|
| `admin/` | 88 | ≈100,334 | Admin API 处理器 + 控制台后端 |
| `app/` | 31 | ≈39,361 | usecase 层（object/bucket/multipart） |
| `storage/` | 40 | ≈39,351 | S3 语义翻译、EC 文件系统、RPC、并发 |
| `table_catalog/` | 20 | ≈37,551 | S3 Tables / Iceberg Catalog |
| `server/` | 17 | ≈14,336 | HTTP 监听、TLS、CORS、中间件、优雅停机 |
| `config/` | 8 | ≈3,388 | CLI 参数、配置解析、负载画像 |
| `startup_*` 家族 | 30+ 文件 | — | 启动顺序/预检/IAM/存储/停机等细粒度拆分 |

### 4.2 命名归一（已高度统一）

- **crate 命名**：全部 `rustfs-<名>`（`rustfs-ecstore`、`rustfs-storage-api`、`rustfs-rio`…）；目录名与包名一致（`crates/ecstore` → `rustfs-ecstore`）；主 crate 即 `rustfs`。
- **workspace 依赖**：`[workspace.dependencies]` 集中登记全部 ~250 项内部+外部依赖，成员 crate 只引用不重复定义版本。
- **版本策略**：全 workspace 单版本号（`1.0.0-rc.3`），`[workspace.package]` 统一 edition 2024 / rust-version 1.97.1 / Apache-2.0。

### 4.3 域切分：6 能力域分组（`ARCHITECTURE.md` Crate Reference）

| 域 | crate 数 | 成员 | 职责 |
|---|---:|---|---|
| **Foundation** | 5 | checksums / common / config / data-usage / utils | 共享配置、数据用量模型、工具、校验和 |
| **I/O and storage** | 15 | concurrency / ecstore / filemeta / heal / io-core / io-metrics / lifecycle / lock / object-capacity / object-data-cache / replication / rio / rio-v2 / scanner / storage-api | 纠删码对象存储、元数据、恢复、生命周期、复制、锁、缓存、I/O 管线 |
| **Security and identity** | 10 | credentials / crypto / iam / keystone / kms / policy / security-governance / signer / tls-runtime / trusted-proxies | 凭证、认证、授权、加密、密钥管理、TLS、安全契约 |
| **Protocols and contracts** | 8 | extension-schema / madmin / protos / protocols / s3-ops / s3-types / s3select-api / s3select-query | Admin/节点间/S3/S3 Select 契约 |
| **Operations and integration** | 5 | audit / notify / obs / targets / zip | 审计、可观测、事件投递、通知目标、归档 |
| **Test support** | 2 | e2e_test / test-utils（+log-analyzer 诊断） | 端到端验证、共享测试引导 |

> 注意：域分组是**文档级分类**，不反映目录结构——目录仍为扁平 `crates/`。这与 MOX "路径即契约"（目录即域）的约定不同（详见 §7 建议）。

### 4.4 主 crate 分层：严格向下

```
Server → Admin/App → Storage → ecstore → rio/io-core → disk
```

架构不变量（`ARCHITECTURE.md` Invariants，违反项 ⚠️ 显式标注可跟踪）：
1. **分层只向下**：禁止底层反向依赖顶层。
2. **叶子 crate 零内部依赖**：`config`/`credentials`/`crypto`/`io-metrics`/`madmin` 只依赖外部 crate（历史 `utils→config`、`common→filemeta/madmin` 违规已解决 ✅）。
3. **热路径不漂移**：迁移期间不得改变放置、quorum、读语义、队列与修复行为。

### 4.5 接口面：facade + 契约优先

- **facade 模式**：`rustfs_ecstore::api` 是 ECStore 对外兼容面（layout/storage owner/admin/metrics/notification/capacity/cluster/runtime 等组），**只减不增**（Facade Shrink Plan 单调收缩）；外部 crate 不得直接 import ECStore 根模块。
- **边界模块**：`*_boundary.rs` 集中 ECStore 内外依赖转换（replication_storage_boundary / lifecycle_metadata_boundary / replication_error_boundary…），外部访问必须经过边界。
- **契约独立命名**：`*Independence` 契约显式声明 crate 独立性（`ReplicationCrateFileMetaIndependence` / `ReplicationCrateStorageApiIndependence` / `LifecycleCrateCoreIndependence`…），禁止再依赖被解耦的 crate。
- **契约清单**：`storage-api` 公开 re-export 白名单（对象/桶/多段/拓扑/能力/可观测快照…）由文档锚定，防止漂移。

### 4.6 治理面：PR 门禁 + CI 脚本 + 专家评审（最体系化部分）

- **PR 10 类型**：`docs-only / test-only / contract / api-extraction / pure-move / consumer-migration / dependency-migration / security-change / behavior-change / ci-gate`——一次 PR 只允许一类，禁止目录移动+安全收紧+行为变更混提。
- **依赖方向禁止边**：`storage-api→ecstore`、`security-governance→rustfs`、`extension-schema→rustfs/ecstore`。
- **CI 强制脚本**：`check_layer_dependencies.sh`（分层依赖）、`check_architecture_migration_rules.sh`（迁移规则+必选文档锚点）、`check_doc_paths.sh`（文档引用路径存在性，pre-commit 门禁）。
- **必选架构文档**：6 份文档（overview / runtime-lifecycle / storage-control-data-plane / crate-boundaries / readiness-matrix / global-state-crate-split-plan）锚定必需章节，缺则门禁失败。
- **三专家评审**（推送前强制）：架构/质量、迁移保持、测试/验证；任一 `blocker` 禁止推送。
- **临时兼容代码**：`// RUSTFS_COMPAT_TODO(API-xxx): 保留原因 + 移除条件` 标记 + `compat-cleanup-register.md` 登记 + 单独 cleanup PR 删除。
- **拆分就绪六项 checklist**（Ready-To-Split）：无环依赖 / 契约 trait 不 import 实现模块 / 旧 facade 有兼容测试 / 聚焦测试先行 / 回滚保持 IO+quorum+队列+审计 / 文档-代码同步。

### 4.7 状态面：AppContext-first + 全局状态收敛

- **AppContext 优先**：启动期构造 AppContext，业务逻辑查 `runtime_sources` 边界而非直接摸全局。
- **GLOB-007 收敛**：`rustfs_ecstore::api::global` 直连白名单收窄到 1 个文件（`rustfs/src/storage/storage_api.rs`），只保留 bootstrap 写与生命周期控制；只读 getter 走 `api::runtime`。
- **embedded 单进程约束**：底层 process-global singletons，一个进程只能起一个内嵌 server（集成测试便利 vs 多实例限制，文档如实声明）。

### 4.8 文档治理

- `docs/architecture/` 28 份：CI 强制核心 6 份 + 契约/不变量（erasure-coding 冻结契约、unified-object-generation、placement-repair-invariants）+ 支持矩阵（s3-compatibility / minio-file-format-compat / s3-tables）+ 存量清单（global-state / ecstore-api-facade / compat-cleanup…）。
- 两条铁律：①**只存持久参考**，一次性实施计划/任务跟踪不进仓库；②**不复制事实源**（crate 清单看 Cargo.toml、CI 看 workflow、代码结构看代码），`check_doc_paths.sh` 防断链。

---

## 五、归一化评估（六准则 + 声明诚实度，1–5 分）

| 准则 | 得分 | 依据 |
|---|---|---|
| 内聚（域边界） | 4.5 | 46 crate 按 6 能力域分组；契约 crate 独立（lifecycle/replication/storage-api 已提取）；facade 组与 crate 一一对应 |
| 解耦（依赖单向） | 4.0 | 分层严格向下 + 禁止边 + boundary 汇聚，体系一流；但 `ecstore` 巨型 crate 内部耦合深（单文件 2.1 万行），边界靠文档纪律维持 |
| 可演进（渐进拆分） | 3.5 | 阶段迁移（Phase 0–7）路径清晰、checklist 完备；但 crate 拆分速度落后于特性增长（`bucket/replication` 8,730→15,619 行，+79%），存量债务积累 |
| 可替换（后端抽象） | 4.0 | storage-api 契约 + facade 兼容面；rio-v2（MinIO 磁盘格式）feature-gated 保留替换路径；StorageBackend 抽象支持多后端 |
| 可观测（可诊断） | 4.0 | obs / io-metrics / opentelemetry / pyroscope / tracing 全套；I/O 画像与准入控制进 io-core |
| 一致性（单一事实源） | 4.5 | Cargo.toml=清单权威、ROUTES/脚本=门禁、文档锚点强制；check_doc_paths 防漂移 |
| 声明诚实度（不虚标） | 4.5 | 不变量违规 ⚠️ 显式标注；rio-v2 "ships in no default build"、embedded 单进程限制如实声明；兼容面标 preview |

**总分 ≈ 4.2/5**。横向对照：RustFS 的**治理完备度**（PR 门禁/专家评审/契约优先）在开源存储领域显著高于 MinIO/SeaweedFS 等同类；**归一化短板**集中在巨型 crate 的物理拆分滞后与文档级域分组（非目录级）。

---

## 六、归一化发现的问题与风险

| # | 问题 | 证据 | 风险 |
|---|---|---|---|
| 1 | **巨型 crate 拆分滞后** | `crates/ecstore` 265 文件 / ≈28.8 万行（约一半是 `#[cfg(test)]`）；`disk/local.rs` 21,063 行、`bucket_lifecycle_ops.rs` 11,961 行、`set_disk/mod.rs` 11,151 行 | 编译时长、认知负载、合并冲突、边界纪律失效 |
| 2 | **契约提取速度 < 特性增速** | `bucket/replication` 半年 +79%（8,730→15,619 行） | 债务越滚越大，拆分永远"再等等" |
| 3 | **全局状态收敛未闭环** | GLOB-007 尚在白名单阶段，`api::global` 直连仍存在（1 个文件） | 多实例/可测试性受限 |
| 4 | **facade 兼容面宽** | `rustfs_ecstore::api` 覆盖十余个组，收缩是单调过程 | 外部依赖面大，收缩周期长 |
| 5 | **文档级域分组 ≠ 目录结构** | 6 域分组只在 ARCHITECTURE.md，目录仍是扁平 `crates/` | 新人按文档找代码需映射；与"路径即契约"实践有差距 |
| 6 | **embedded 单进程限制** | process-global singletons 约束 | 一个进程一个 server，多租户测试需起多进程 |

---

## 七、模块化归一化优化建议（整理优化输出）

### 7.1 布局归一（择一执行，勿并行）

- **方案 A（推荐，低扰动）**：保持扁平 `crates/`，引入**命名前缀**分域（`rustfs-storage-ecstore` 类命名演进不可行——改名即 API 破坏），改为在 `ARCHITECTURE.md` 的 Crate Reference 增加"目录前缀约定"：新 crate 一律 `crates/<域>-<能力>/`（如 `crates/storage-replication`、`crates/security-iam`）。
- **方案 B（高成本高收益）**：目录按域分组 `crates/<domain>/<crate>/`，一次性迁移 + `deny.toml`/CI 校验；与 MOX `platform/domains/<域>/<层>/` 同构。**建议等到 Phase 7 全局状态收敛后再做**，避免与拆分 PR 混动（违反"一次 PR 一类"）。

### 7.2 命名归一（已基本完成，补两项）

- 新 crate 强制 `rustfs-<域>-<能力>` 公式并加入 `workspace.dependencies` 统一登记（现有已符合）。
- 为契约独立新增统一的 `*Independence` 命名规范文档条目，使"谁不得依赖谁"成为机器可查清单（现散见于各 split plan）。

### 7.3 巨型 crate 拆分（按 Ready-To-Split checklist 排序）

建议候选顺序（结合 ecstore-module-split-plan 现状）：

1. **`disk/local.rs`（21K 行）**：先做文件级模块拆分（按 read/write/list/scandir/disk-state 分组），不动运行时行为——`pure-move` PR。
2. **`bucket/replication` runtime**：契约（约 20 个）已提取完毕，按 `ReplicationObjectIO → ReplicationStorage → ReplicationMetadataStore` 顺序逐个迁移运行时依赖，每个契约一个 `contract`/`consumer-migration` PR。
3. **`set_disk/`**：保留共享状态载体，先提取纯契约（shard source / disk error / bitrot IO / namespace lock / metrics labels / filemeta access）。
4. 拆分红线：**禁止**在移动运行时状态或改启动行为的同一 PR 里拆 crate。

### 7.4 兼容面治理（维持单调收缩）

- 新增代码一律禁止直连 `rustfs_ecstore` 内部；外部访问只能经 `rustfs_ecstore::api::*` 或 owner-local `*_boundary.rs`。
- 删除 facade 组前必须：清单盘点 → 编译覆盖 → 迁移消费者 → 兼容测试通过，**一组一 PR**。
- 临时兼容代码必须带 `RUSTFS_COMPAT_TODO(API-xxx)` + cleanup register 登记，禁止"静默常驻"。

### 7.5 构建与门禁归一

- **未用依赖检测**：workspace 已配 `cargo-shear`（ignored: hotpath/rustfs），建议纳入 CI 门禁（现为本地工具）。
- **lints 统一**：`unsafe_code=deny` + `clippy all=warn` 已是基线；建议补 `#![deny(missing_docs)]`（库 crate）与 rustfmt 门禁。
- **profile 标准化**：dev（line-tables-only）/ release（thin-LTO+cgu1+strip）/ production / profiling 已是良好模板，建议文档化选型规则（默认 dev 调试体验与 CI 速度的平衡）。

### 7.6 文档治理补强

- 建立 **crate 清单 ↔ Cargo.toml 自动同步校验**（现状依赖 `check_doc_paths.sh` 防路径失效，但清单内容仍是手写；可借鉴 MOX `gen-api-registry` 的生成器思路）。
- `docs/architecture` 只存持久参考铁律保持；拆分/迁移进度类文档进 issue tracker（现状已执行，注意别把 §7.3 的候选清单长期留在仓库内）。

---

## 八、对本仓库（MOX / 璇玑）的启示

### 8.1 能力映射：RustFS ↔ MOX cloud 域

| RustFS（46 crate） | MOX cloud 域（13 crate） | 对应状态 |
|---|---|---|
| `ecstore`（纠删码存储引擎） | `mox-cloud-kernel`（reed_solomon / multi_writer / hedged_reader / buffer_pool）+ `mox-cloud-store-core`（erasure / dedup / heal / gc） | MOX 已有自研内核 |
| `storage-api`（契约面） | `mox-cloud-domain-traits`（StorageBackend / ChunkId / BackendCapabilities） | **同构**，trait 抽象一致 |
| `rio` / `io-core`（I/O 管线） | `mox-cloud-store-core`（stream_writer / cache / bitrot / fs_backend / s3_backend / kv_backend） | MOX 已具备多后端 |
| `scanner` / `heal` / `lifecycle` / `replication` | `mox-cloud-rebalance-svc` / s3-svc（lifecycle / replication / versioning / restore_tasks） | MOX 已实现 |
| `madmin` / `obs` / `audit` | `mox-cloud-admin-sdk` / platform 可观测/审计基座 | MOX 走 platform/shared |
| **ecstore 接入点（RustFsEcstoreBackend）** | `mox-cloud-s3-svc/src/storage/rustfs_ecstore.rs`（骨架，全部 Unsupported） | **待对接** |

### 8.2 可直接借鉴的归一化机制（按优先级）

1. **PR 类型门禁**（10 类一次一类）：MOX 目前无此约束，跨域/跨层混改是文档漂移主因之一——建议先落地为 `docs/standards/` 规范 + 评审 check。
2. **`*_boundary.rs` 边界模块**：MOX cloud 域跨 crate 依赖可用同款模式收敛（如 s3-svc → store-core 的访问集中到边界文件）。
3. **`RUSTFS_COMPAT_TODO` 兼容标记 + cleanup register**：MOX 历史遗留（旧 Python 栈、多版本架构稿）可引入"标记 + 登记 + 单独清理"三件套，替代"头部标注 LEGACY 了事"。
4. **Ready-To-Split 六项 checklist**：cloud 域若把 `mox-cloud-s3-svc` 继续拆细（现约 40 文件、s3_server/versioning/lifecycle/replication 同 crate），先过 checklist 再动。
5. **数据面/控制面分离**：MOX `mox-cloud-master-svc`（raft/scheduler/volume_allocator）即控制面，`mox-cloud-volume-svc` / s3-svc 即数据面——建议在 cloud 域文档中显式声明边界与"热路径不漂移"红线（现无此声明）。
6. **RustFsEcstoreBackend 对接路径**：参照 RustFS"契约先于进程"——先定义 `rustfs_ecstore` 侧契约（chunk 读写/元数据/EC profile）并写进 `mox-cloud-domain-traits`，再实现 Unix Socket/gRPC 或 FFI，最后接 feature flag；避免骨架长期悬空。

### 8.3 MOX 相对更强的点（不必照搬）

- **命名公式**：`mox-<域>-<层>-<角色>`（api/proto/core/svc/sdk）比 RustFS 扁平命名更具结构信息；**目录即域**（`platform/domains/<域>/`）优于 RustFS 的文档级分组。
- **域分组粒度**：12 业务域 vs RustFS 6 能力域——MOX 已按业务能力切分到域，RustFS 的 6 组更像分类标签。
- **六层单向依赖**（foundation→api→proto→core→svc→gateway）与 RustFS 五层主 crate 分层同构，且 MOX 的 `core` 层"纯计算无 IO"约束更严格。

---

## 九、证据与引用

### 9.1 外部来源

- JavaGuide《对标 MinIO！全新一代分布式文件系统正式发布》（2026-09-17）：https://mp.weixin.qq.com/s/ePJgcVAYYscAcKIG70C5yA
- RustFS 官方仓库：https://github.com/rustfs/rustfs

### 9.2 本地源码（`ais/RustFS/`，只读引用）

| 路径 | 用途 |
|---|---|
| `Cargo.toml` | workspace 47 成员、命名、依赖集中、lints、profiles（证据：§4.1/4.2/7.5） |
| `ARCHITECTURE.md` | 总览、数据流、Code Map、6 域分组、架构不变量（§3.2/4.3/4.4） |
| `rustfs/src/lib.rs` | 主 crate 模块树、embedded 单进程声明（§4.1/4.7） |
| `docs/architecture/README.md` | 文档治理铁律、28 份清单（§4.8） |
| `docs/architecture/overview.md` | 迁移基线、核心原则、Phase 0–7 顺序（§4.6） |
| `docs/architecture/crate-boundaries.md` | PR 10 类型、禁止边、三专家评审、COMPAT_TODO（§4.6/7.4） |
| `docs/architecture/ecstore-module-split-plan.md` | ECStore 拆分现状、契约清单、Ready-To-Split（§6/7.3） |
| `docs/architecture/storage-control-data-plane.md` | 数据面/控制面边界（§3.3） |
| `docs/architecture/global-state-crate-split-plan.md` | AppContext-first、GLOB-007（§4.7） |

### 9.3 本仓库对照

- `docs/architecture/MODULARITY.md`（MOX 模块化评估框架，评分口径对齐）
- `docs/architecture/NORMALIZED_ARCHITECTURE.md`（143 crate / 12 域 / 六层 / 四进程事实基准）
- `platform/domains/cloud/`（cloud 域 13 crate；`mox-cloud-s3-svc/src/storage/rustfs_ecstore.rs` 待对接骨架）

---

*L7 过程证据 · 2026-09-17 · 只读对标研究，未改动 `ais/RustFS/` 任何文件。*
