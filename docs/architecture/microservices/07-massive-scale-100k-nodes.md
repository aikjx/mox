# 十万级分布式节点：知识图谱 · 云盘知识库 · OSS 部署与数据管理方案

> **层定位**：L2 架构层（微服务卷）。本文是「海量规模（≥10 万节点）部署拓扑与数据管理」这一事实的唯一权威源。
> 相关：[04-部署](./04-deployment.md) · [DOMAIN-DEPLOYMENT](./DOMAIN-DEPLOYMENT.md) · [HA 容量与 TCO](../../../deploy/docs/ha-capacity-tco.md) · [传输加密一键开关](../../api/API-CRYPTO-TRANSPORT.md) · [联盟现状架构](../../expert-alliance/CURRENT-ARCHITECTURE.md)
> 容量数字可复算：`python tools/scale-model/capacity_model.py`（证据 JSON 落 `reports/data/`）

---

## 〇、结论速览

1. **10 万节点不能是"一个集群"**：必须归一化为 **Cell（爆炸半径单元）分层拓扑** —— 本文给出 98 Cell × 1024 节点的标准形态与全部分层参数。
2. **数据管理归一化为一条管线**：OSS（冷/对象）→ 云盘知识库（温/文件+FTS）→ KG 分片（热/图），统一 namespace、生命周期、多租户、纠删码策略，一个事实一个权威源。
3. **本次实测改进**（证据见 §六）：KG 在线分裂**丢边正确性 bug 已修复**（`shard_split_edge_integrity`，先红后绿）；顶点点查从整分片扫描 O(n) 改为 **vid 二级索引 O(log n)**，10 万顶点级点查延迟 **244µs→2.26µs（108×）**，扩展比 48.6×→2.2×；VID 长度前缀 1B→2B（长 URI 型 ID 可写入）；`mox-kg-storage-svc` 全套测试 release 单线程 **281 项全绿**（90+45+27+19+33+44+23）。
4. **专家联盟**从"任务协作系统"升级为"海量数据管线编排器"：分片感知调度、分级心跳、批量导入 DAG（§五）。
5. 诚实边界：仓库当前 Raft 为单进程模拟、部署二进制持久化已转正（P0，RocksDB 模式以 CI `kg-scale` 首绿为准）、KB 无向量检索——方案给出 P0→P3 收敛路线与每阶段验收命令，不假装已存在。

## 一、现状盘点（证据基线）

| 维度 | 现状事实 | 证据 | 与 10 万节点差距 |
|------|----------|------|------------------|
| 分片路由 | VID→SHA256 低位→shard，分片数须 2^k，在线分裂 16→32 | `kg/svc/mox-kg-storage-svc/src/graph_codec.rs vid_hash_shard`、`partition_raft.rs` | 无一致性哈希/vnode，跨机再平衡未实现 |
| 共识/复制 | "最小实现"：单进程内保留 shard 状态，Leader 静态取模，无选举/网络复制 | `src/shard_raft.rs` 头注释、`:234` | 无跨机副本 → 无故障转移 |
| 持久化 | 库默认 `RwLock<BTreeMap>` 内存引擎；部署二进制 `mox-kg-server` 已默认开启 `persist-rocksdb`（构建需 libclang），RocksDB 写路径 ack 前 fsync（`MOX_KG_WAL_SYNC=0` 显式降级性能档） | `Cargo.toml [features]`、`kv_engine.rs wal_sync_enabled`、`tests/t_persistence_rocksdb.rs` | 跨机复制仍未实现，单机 fsync 是当前唯一掉电防线 |
| 实测规模 | 单测最大 10 万顶点路由/χ² 均匀、16→32 分裂 10 万顶点不丢、写基线 100k ops/s、千亿级靠**外推** | `tests/t_distributed_sharding.rs`、`t_perf_bench.rs`、`t_billion_scale_simulation.rs` | 100 节点仅为测试断言假设 |
| CDC | 进程内内存队列（消费者组/offset/背压），10 万级 harness 因依赖未实现类型被禁用 | `src/cdc_publisher.rs`、`mox-kg-streams-svc/src/bin/cdc_100k_harness.rs.disabled` | 无 WAL/Kafka 持久化，10 万级 CDC 从未实测 |
| 已承诺上限 | 6 节点 / 1 亿顶点 / 5 亿边 / 读 60k 写 6k QPS；缩放公式 `基准×(边/500M)^0.9` | `deploy/docs/ha-capacity-tco.md` §二 | 与 10 万节点差 ~4 个数量级 |
| 云盘/OSS | 双后端 FS/S3（自研 SigV4，与 MinIO/COS/OBS/阿里 OSS path-style 互通）；filer 3 种元数据后端（SQLite / Postgres+Citus / Redis mock）；EC 纠删码+bitrot 自愈；mpu/lifecycle/replication/glacier | `cloud/core/mox-cloud-store-core/src/s3_backend.rs`、`cloud/svc/mox-cloud-{s3,filer,master,volume,rebalance}-svc` | 未真机对接公有云 OSS 验证；bucket 级多租户隔离缺失 |
| 知识库 | SQLite+FTS5/BM25，embedding 字段预留 NULL，无向量检索 | `kb/core/mox-kb-core/src/sqlite_store.rs` | 无 pgvector/HNSW，KB 与 KG/OSS 未成管线 |
| 联盟 | DAG 引擎真实并行调度；queue=1000/并发=100 可 env 覆盖；registry 单点、执行面向"专家"非"分片" | `alliance/core/mox-alliance-executor-core/src/dag_engine.rs`、`boot-config/src/lib.rs:115,512` | 无 10 万级注册/心跳验证，与 KG 分片体系未打通 |
| 编排 | docker-compose 4 套（最大 9 服务）；Helm kind-3m3s + 双集群 DR 模板 | 根 `docker-compose*.yml`、`deploy/helm/` | 无 Cell/多集群 Operator 编排 |

## 二、容量模型（10 万节点，可复算）

`tools/scale-model/capacity_model.py` 输出（假设全部显式、锚定上表权威源）：

| 指标 | 值 | 推导 |
|------|-----|------|
| Cell 数 | **98**（每 Cell 1024 节点，2^10） | 爆炸半径上限 = 单 Cell 控制面 Raft 成员/再平衡风暴半径 |
| 总容量 | **1T 顶点 / 5T 边** | 基线密度 83.3M 边/节点 × 60% 水位 × 10^5 节点 |
| naive 公式交叉校验 | 5T 边按 `(边/500M)^0.9` 需 23,886 节点 < 10^5 | 余量来自副本/EC/多租户与管理面开销 |
| 内存 / CPU | 3.2 PB / 1.6M cores（按基线线性） | `ha-capacity-tco` 每节点密度 |
| 读 / 写 QPS @60% | 600M / 60M | 基线 10k/2k 每节点 |
| 热数据（KG RF3） | 4.5 PB（原始，NVMe） | 5T 边 × 300B × 3 |
| 冷数据（OSS EC 8+3） | 2.06 PB | 5T 边 × 300B × 11/8 |
| 全量导入 | **0.48 天**（Raft 写 2k/节点为限）；内存态上限参考 0.23h（100k ops/s/节点） | 瓶颈在共识提交而非磁盘 |
| 全局心跳 | 聚合后仅 **1k pps**（节点→机架 10:1→Cell 10:1） | 分级聚合是 10 万节点控制面可行性的前提 |
| 每节点 shard 副本 | 12（4096 分片 × RF3 ÷ 1024 节点） | 分片数/Cell=2^12，倍增在线分裂 |

## 三、总体架构：三平面 + Cell 分层（归一化拓扑）

```
┌─ 全局层（唯一，跨 Region）────────────────────────────┐
│ global-router：Cell 路由表(98 条) + namespace/租户权威    │
│ 协议：gRPC :50051（v3 目标态）+ 一键传输加密信封           │
├─ Cell 层（×98，每 Cell 1024 节点，2 机架组×512）─────────┤
│ cell-master：本 Cell 分片 Raft 组(5) + 心跳聚合 + 再平衡   │
│ KG 4096 分片(2^12, RF3) │ 云盘 volume 池 │ KB 索引分片     │
├─ 节点层 ────────────────────────────────────────────┤
│ 16vCPU/64GB/NVMe4TB：kg-storage-node(12 shard 副本)      │
│ + cloud-volume-agent + kb-agent，sidecar 统一观测/鉴权     │
└─ 数据平面归一化（一个对象一条生命周期）──────────────────┘
OSS 冷(对象, EC 8+3, glacier 归档) ← 云盘知识库 温(文件+FTS5/BM25→向量) ← KG 热(图分片, RF3)
```

**为什么这是最优**（对比备选，ADR 级论证）：
- 平面 10 万节点单集群：Raft 成员 O(N)、一次全量再平衡即全站风暴、控制面心跳 O(N) —— **不可行**。
- 完全独立多集群无分层：租户/全局查询/跨 Cell 边失去归属 —— **数据管理碎裂**。
- **Cell 分层**：控制面规模只随 Cell 数增长（98），数据面线性水平扩展，故障域=1 Cell（≤1%），倍增分裂（2^k）与 vid 低位路由让跨 Cell 迁移仅翻倍/减半——与仓库既有 `rebalance_16_to_32` 语义同构，是唯一"代码已具雏形、语义可平移"的方案。

**部署编排归一化**：K8s + 两级 Operator。`cell-operator`（每 Cell 一个：kg-storage StatefulSet×1024 + volume-agent + 分片拓扑 CR）与 `cluster-operator`（全局：Cell 编排、扩 Cell、金丝雀按 Cell 灰度）。基座沿用 `deploy/helm/kind-3m3s-deployment.yaml` 的 3 主 3 从形态并参数化到 Cell 模板；跨 AZ 反亲和 = 每机架组一 AZ。

## 四、数据管理归一化（一个事实一个权威源）

1. **统一 namespace**：`/{tenant}/{space}/{dataset}` 三段式贯穿 OSS bucket → 云盘路径 → KG space_id；`global-router` 为唯一归属权威；KG 分片路由 `vid_hash_shard(vid, shards)` 是数据位置的唯一函数（幂等、可重放）。
2. **生命周期一条线**：入湖（OSS multipart+幂等键，`mox-cloud-s3-svc` lifecycle/glacier）→ 解析入知识库（filer 元数据 Postgres+Citus 多租户；FTS5→向量索引升级见 P2）→ 抽边入 KG 热层（bulk upsert + `graph_spark_writer` 幂等键）→ 冷热回沉（KG 归档边快照写回 OSS EC）。每段交接以 CDC 事件为契约（P1 起 CDC 持久化到 WAL/Kafka 后成为权威事件流）。
3. **多租户与密级**：bucket 级配额+隔离（缺失，P1 补）；租户密钥隔离与字段密级复用一键传输加密体系（`MOX_API_CRYPTO=sm4`，见 API-CRYPTO-TRANSPORT），密钥经 KMS 注入 `MOX_API_CRYPTO_KEY`。
4. **可靠性语义闭环**：热层 RF3（Raft 多数派提交后才 ACK）+ 冷层 EC 8+3 + 每日快照异 Region 复制 + bitrot 周期巡检自愈（`mox-cloud-kernel` reed_solomon/hedged_reader 已有）。修复既有矛盾：启用 `persist-rocksdb` 为生产默认，WAL sync 与"Raft 保证持久性"承诺对齐（P0）。
5. **配置/端口/命名归一**：一切规模参数走 `MOX_ALLIANCE_*`/`SCHEDULER_*` 环境变量单通道（boot-config 已落地 env>yml>默认三级）；端口以 `docs/api/PORT-REGISTRY.md` 为唯一权威并经 `verify-ports.py` 门禁；Cell/分片命名沿用 `cf_name_*(shard)` 编码规范，禁止旁路直写 KV（vid 索引不变量依赖 `apply_*` 单入口）。

## 五、专家联盟：海量数据分析与优化

联盟在 10 万节点体系中的角色 = **数据管线的编排大脑**（图谱构建/批量导入/跨 Cell 治理任务都表达为联盟任务）：

| 优化项 | 现状 | 方案 | 优先级 |
|--------|------|------|:------:|
| 调度器水平扩展 | 单实例，queue=1000/并发=100 已可 env 覆盖 | 多活 scheduler + Postgres 任务表为权威状态（已具备 sqlite/pg 双仓库），无状态化后 LB 前置；分片键=task space | P1 |
| 节点/执行体注册分级 | registry-svc 单点，面向"专家" | 三级注册：节点→cell-master 本地租约（10s 抖动防惊群）→registry 仅持 98 Cell 摘要；心跳聚合后全局 1k pps（§二） | P1 |
| DAG 分片感知 | executor DAG 并行调度真实存在但面向任务 | 新增 `ShardFanout` 节点类型：一个逻辑任务按 vid_hash 区间展开 N×分片子任务，executor 池按 Cell 反亲和放置；批量导入即一个 DAG（10 万级实测 harness 解禁重建） | P2 |
| LLM 编排成本 | 10 专家独立配置+降级链（已归一） | 海量数据预处理走自研 `expert-code-engine`（mox-selfhosted 零 token），LLM 仅裁决层；降级链校验已修复（provider_options 完整性） | 已落地 |
| 融合/内存服务拆分 | 网关内联 | 沿用 v3 目标态 fusion-svc/memory-svc，按 Cell 部署本地实例避免跨 Cell 流量 | P2 |

## 六、本次落地的改进与实测证据

**正确性（在线分裂丢边 bug）**：`apply_split_shard` 迁移边时，对端顶点若留在原子片，旧键与新键是同一物理键，批量写入顺序"先插后删"自删新数据 → 分裂后 `get_neighbors` 为空。修复=新归属统一按 `hash(vid, 2×current)` 计算 + 同键保护（`partition_raft.rs`）。复现→修复：`shard_split_edge_integrity` 先红（left:0 right:1）后绿，`t_distributed_sharding` **27/27**。

**性能（点查 O(n)→O(log n)）**：`read_vertex`/`apply_del_vertex` 原为整分片前缀扫描（`seek_prefix` 克隆全分片）；新增 `vid_idx_{shard}` 二级索引 CF（`[shard][vid]→[(tag_hash, vertex_value)]`），写/删/分裂三路径维护不变量，测试 benchmark 改走生产读路径：

```
修复前(release): 100v=5.0µs  1k=13.6µs  10k=244µs   100x 数据 → 48.6x 延迟  FAILED
修复后(release): 100v=1.03µs 1k=982ns   10k=2.26µs  100x 数据 → 2.2x  延迟  ok
```

**测试口径**：`cargo test -p mox-kg-storage-svc --release -- --test-threads=1` 全绿（90+45+27+19+33+44+23，共 281 项）；并行跑时两个计时断言受线程争抢影响（traversal 单独复跑 3 次均 16–19x 线性区间），规模化计时类断言今后应固定单线程或加 warmup。

**双向 BFS 集成测试**：断言（≤4 步）与算法（tie 时永远只扩 forward，退化为单向 7 步）不符——已改为每步双侧各扩一层，33/33 通过。

**VID 长度编码（海量数据可行性）**：vertex/edge key 的 VID 长度前缀原为 1B（≤255 字节），URI 型/长 ID 直接写入失败（`bytes too long`），且与自带边界测试（1000 字符 VID 期望成功）矛盾——HEAD 上 `codec_boundary_values` 即为红灯（该 crate 不在默认门禁内故长期未暴露）。已统一改为 2B 前缀（≤64KB，`graph_codec.rs` 编码/解码/扫描前缀三处同步，与 `vid_idx` key 口径一致）；etype 保持 1B（模式级字符串，非用户数据）。

**PageRank 基准测试（HEAD 上即随机失败）**：`t_perf_bench` 内联实现的 PageRank 对悬挂节点（出度 0）不做质量重分配，总和随迭代单调流失（HEAD 实测 sum≈0.94–0.96 抖动，断言 <0.01 必红）。已按标准 PageRank 补悬挂项均摊，23/23 连续两轮通过。

**P0 基座闭环（本轮追加）**：
- 纠偏假承诺：原注释「Raft 层已保证持久性」不成立（Raft 为单进程实现、日志不落盘、无副本多数派），而两处 `write_opts` 均 `set_sync(false)` → ack 不等于持久。现 RocksDB 写路径**默认 ack 前 fsync**（`kv_engine.rs::wal_sync_enabled`，env `MOX_KG_WAL_SYNC` 显式降级，含单元测试锁定解析语义）。
- 部署形态持久化转正：`mox-kg-server`（端口 3411）默认 feature `persist-rocksdb`，经 `mox-kg-service-svc` 透传；`deploy/docker/Dockerfile.rust-service` 补 `clang libclang-dev`。库 crate 默认仍为内存引擎，保持本地/CI 轻量测试面。
- 回归锁定入 CI（`ci.yml` job `kg-scale`）：内存全套单线程 281 项 → RocksDB 模式 `--all-targets` 编译门禁 → `t_persistence_rocksdb`（写 500 点+长 URI VID，关闭重开同目录读回，验证 vid_idx 跨重启一致）→ 正确性子集（`t_distributed_sharding`/`t_integration_storage`/`t_integration_query` 跑在 RocksDB+异步 WAL 档）→ 容量模型复算 + `reports/data/` 证据上传。
- 诚实边界：本机（Windows）无 libclang，RocksDB 模式运行时以 CI `kg-scale` 首绿为准；且全仓尚无生产入口实例化 `StorageServer`（仅测试构造），feature 转正是为存储宿主接线后的默认 durable，接线属 P1 范围。

## 七、演进路线与验收（P0→P3）

| 阶段 | 交付 | 验收命令（必须可执行） |
|------|------|------------------------|
| **P0 基座闭环**（上生产前置） | ✅ 已落地：`mox-kg-server` 默认 `persist-rocksdb` + ack 前 fsync WAL（`MOX_KG_WAL_SYNC=0` 可降性能档）；vid 索引/分裂/长 VID 回归入 CI 专用 job `kg-scale`；容量模型脚本 CI 复算并上传产物 | `cargo test -p mox-kg-storage-svc --release -- --test-threads=1`；`python tools/scale-model/capacity_model.py`；RocksDB 模式见 `.github/workflows/ci.yml` job `kg-scale`（本机无 libclang 时的权威执行环境） |
| **P1 真分布式** | 跨机 Raft 复制（复用 `async-raft`，meta-core 已有依赖）、cell-master 心跳聚合、CDC 持久化（WAL/Kafka）+ 10 万级 harness 解禁、bucket 多租户 | 新建 `t_cell_split_crosshost`（3 VM×3 分片组杀 1 副本无数据丢失）；CDC 100k 幂等报告落 `reports/data/` |
| **P2 管线打通** | KB 向量检索（embedding 落 pgvector 或自研 HNSW on CloudKernel）、ShardFanout DAG、fusion/memory-svc 按 Cell 部署、冷回沉自动化 | `alliance_demo.py` 增加 bulk-import 模式端到端；`check-doc-links.py` 门禁 |
| **P3 全局形态** | 多 Region Cell、global-router gRPC+mTLS、跨 Cell 分布式遍历（当前为分片局部+应用层聚合）、混沌工程（Cell 级故障注入） | kind-3m3s → `helm --set cell.size=…` 参数化演练 1K 节点（1 Cell）沙箱 |

**10 万节点的现实路径**：6（现状）→ 1K（1 Cell，P1 完成即实测）→ 10K（10 Cell，P2）→ 100K（98 Cell，P3）。每级都是同一 Cell 模板的水平复制——这就是"企业级归一化"的含义：**扩规模不扩架构**。

---

*本文数字全部可复算：容量模型 `tools/scale-model/capacity_model.py`（JSON 证据落 `reports/data/`）；性能/正确性证据命令见 §六。实现真源：`platform/domains/kg/svc/mox-kg-storage-svc/`、`platform/domains/cloud/`、`platform/domains/kb/`、`platform/domains/alliance/`。*
