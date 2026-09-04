# MOX v2.0 mox 模块化系统架构维度融合架构规格（参考 NVIDIA AIS + 政企最佳实践）

## 1. Problem / Users / Goals / Non-Goals

### 1.1 Problem
当前 T10/T11/T17/E12-E20 产品虽然单体能力完备，但面向政企级大规模落地仍存关键差距：
- **数据保护**：仅有对象版本 + WORM，缺少 AIS 级的 **EC(n+k) 纠删码 + 自修复 + 磁盘健康 FSHC**；
- **传输层**：只支持 HTTP/1.1，缺少 **HTTP/3 QUIC + 三网分离**，跨地域高并发场景握手延迟大；
- **云盘×知识图谱**：双系统物理独立，PutObject 后标签/分类/版本变更**不会自动沉淀为知识图谱顶点与边**，无法形成"数据入湖→自动认知"闭环；
- **部署形态**：多组件（Rust backend / Node / SDK services / graph-stream / Spark / Helm）依赖 docker-compose/K8s，**缺少单二进制 all-in-one 独立部署**于信创裸机/边缘/内网离线环境；
- **合规能力**：缺少 **Legal Hold 法规保留锁 + 密级 4 级标签** 等政企核心场景；
- **观测与取证**：指标散点多，但缺 **P99/P999 全链路热力图 + 审计链取证仪表**；
- **性能量化**：未定义 1KB/1MB/1GB 三档政企基准；
- **SDK ETL near-data**：缺少 AIS Inline-ETL 风格的 I/O 密集型预处理器外挂框架。

### 1.2 Users
| 角色 | 典型场景 |
|---|---|
| 政企数据中心运维师 | 信创裸机单机一键部署、等保审计、密级标签策略 |
| 研发架构师 | SDK 上传/下载、标签出图、图查询反查对象 |
| 安全合规官 | LegalHold + 审计链取证 + 密级访问裁决 |
| AI/ML 工程师 | 大规模对象数据集 → ETL 预处理 → 图关联特征 |
| CIO/CTO | TCO（已有 5.94M 3 年基线）+ SLA/容灾/可独立部署 |

### 1.3 Goals（对标 NVIDIA AIS + Neo4j/Weaviate 最佳理念）
1. **EC(n+k) 纠删码引擎**：支持 4+2 / 8+4 / 12+4，对象 > 64MB 自动分片，丢失 ≤ k 片可自修复；
2. **传输层升级**：HTTP/3(QUIC) + S3 分片断点续传 UploadId（三语言 SDK 统一）+ 三网分离 listener（Public/IntraCtrl/IntraData）；
3. **云盘×知识图谱零延迟融合**：PutObject → 标签/元数据 → CDC（CdcEvent ObjectTagged）→ 自动建 `tag://...` / `obj://...` 顶点 + `HAS_TAG` 边，反向 GraphQL 查询 obj.uri 可直接回源 S3 HEAD；
4. **单二进制 mox-server**：Rust workspace member，包含 proxy+target+graph+trace+metric+WAL，`mox server --single-node` 零依赖启动；
5. **合规升级**：Legal Hold 法规锁（HoldUntil 不可删除覆盖 PUT）+ 密级 4 级（绝密/机密/秘密/内部）+ 访问裁决（Bell-LaPadula 无向上读、无向下写）；
6. **P99 观测 + 取证仪表**：Prometheus 指标扩展（obj_put_p99 / ec_encode_us / fshc_disk_fail_count / tag2graph_lag_ms）+ 审计链 hash_chain 取证页；
7. **三档性能基准**：1KB（小文件）≥ 60K ops/s · 1MB（对象）≥ 1.2 GB/s · 1GB（大文件）≥ 8 GB/s（单节点基准，含 JWT + 审计链 + 标签出图）；
8. **ETL near-data 插件框架**：参考 AIS ETL，对象 GET/PUT 路径可注入 `WASM plugin` 完成压缩/指纹/特征提取/脱敏；
9. **独立部署验证**：`mox server --single-node --data-dir ./x --graph-dir ./g --port 8080` + `helm install mox ./deploy/helm/mox --set standalone=true` 双路径；
10. **磁盘健康 FSHC + 挂载路径热插拔**：Mountpath enable/disable/attach/detach；坏盘检测自动触发 EC 自修复；
11. **Read-after-Write 一致性 + 端到端 CRC64**：任何网关读返回 CRC 一致性校验；Client 提供 checksum 时进行端到端验证；
12. **E2E 回归 ≥ 1000 tests**：T10(118)+T11(126)+T17(154)+T19(719) 既有 1117 + 新增 ≥ 120 = 实际 ≥ 1237 tests。

### 1.4 Non-Goals（明确不做）
- 不实现 RDMA/RoCE 硬件驱动；（保持 NIC-agnostic）
- 不替换既有 WORM/SQLite 存储为专用 OSD；（保持兼容）
- 不实现 K8s Operator 自定义控制器；（保留 Helm 一键）
- 不重做已有 STS/JWT/Quota 模块；（增量引入而非重写）

---

## 2. Functional Requirements (FR)

### FR-1：Erasure Coding 引擎（EC-Engine）
- 定义 `EcProfile { data_shards: u16, parity_shards: u16, min_obj_size: u64 }`，默认桶级 `4+2`。
- 对象 PUT ≥ min_obj_size → Reed-Solomon(n,k) 切分 → 落盘为 `[mountpath]/<bid>/<oid>/ec/shard_<i>.slice` + `manifest.json (CRC64, size, created_at)`。
- GET：按顺序读 shard，若丢失 ≤ k 片则 on-the-fly 重建；缺失 shard 后台回写（xaction 风格异步修复）。
- `Mountpath` 标记为 Faulty 时，触发 RebuildJob；完成后标记 Healthy。

### FR-2：HTTP/3(QUIC) + 三网分离 + Multipart UploadId
- `mox server` 启动三 listener：
  - `:8080` public（SDK/CLI/JWT 入口）
  - `:9080` intra_ctrl（membership/health/metasync/xaction）
  - `:9081` intra_data（target→target 数据面）
- Public listener 可通过 `--http3` 启用 QUIC（fallback TCP）。
- Multipart：`POST /s3/<bucket>?uploads` 返回 UploadId；`PUT ?partNumber=N&uploadId=X`；`POST ?uploadId=X` CompleteMultipart 返回 CRC64 聚合；Abort 清理分片。
- 三语言 SDK：`upload_multipart(bucket,key,reader,part_size=8MB) -> PartAggregate { crc64, etag, n_parts }`。

### FR-3：PutObject → Tag → CDC → 知识图谱 自动融合
- 写入管道新增阶段：AuditChain 之后 → `tag_cdc_graph_stage`。
- 对 PUT 携带的 `x-mox-tag-{k}={v}`（或默认 contentType / size_bucket / mimeTypeCategory）→ 解析为 TagSet → 产生 `CdcEvent::ObjectTagged(obj_uri, tags[])`。
- Graph CDC 消费者收到后：UPSERT `obj:<sha256(uri)>`（props: uri, size, etag, bucket），UPSERT 每个 `tag:<k|v>` 顶点，UPSERT `(obj)-[:HAS_TAG]->(tag)` 边。
- 反向：Graph 查询 `MATCH (o:obj)-[:HAS_TAG]->(t:tag {k:"project",v:"secretariat"}) RETURN o.uri LIMIT 100` → 返回后客户端可用 SDK `head_object(uri)` 回源元数据。

### FR-4：单二进制 mox-server All-in-One
- 新增 workspace member：`platform/services/mox-server/Cargo.toml`（bin name=`mox`）。
- 子命令：
  - `mox server --single-node [--data-dir D] [--graph-dir G] [--http3] [--port P] [--intra-ctrl-port CP] [--intra-data-port DP]`
  - `mox ec profile add --bucket b1 --data 4 --parity 2`
  - `mox mount attach /dev/sdb1  /mnt/x1`
  - `mox legal-hold put --bucket b1 --key k1 --hold-until 2040-01-01`
  - `mox bench 1KB|1MB|1GB --n 10000`
- `--single-node`：内嵌 in-memory 成员管理 + 本地 mountpath；对外保持 S3/Graph/Trace/Metric 接口。
- 启动后 JWT 默认开；配置文件 `mox.toml`。

### FR-5：Legal Hold + 密级 4 级访问裁决
- Object 元数据新增 `legal_hold: Option<LegalHold { placed_by, placed_at, hold_until }>`；
- DELETE/PUT-overwrite：若 hold_until > now → 拒绝（412 Precondition Failed + 审计链 Record::LegalHoldDenied）。
- 密级：UserClaims 带 `clearance: u8`；Object 带 `miji_level: u8`（绝密=4 机密=3 秘密=2 内部=1）。
- **Bell-LaPadula**：`read allowed iff user.clearance >= obj.miji_level`（Simple Security）；`write allowed iff user.clearance <= obj.miji_level`（*-Property，禁止高密写入低密对象覆盖）。
- CLI：`mox miji set --bucket b --key k --level 3`；审计链记录 `MijiAccessDenied` 原因码。

### FR-6：P99 观测 + 审计链取证仪表
- 新增指标（全局 Registry 共享）：
  - `mox_obj_put_latency_us` Histogram(buckets 10..10M) + `_p99 / _p999` 聚合
  - `mox_ec_encode_us` / `mox_ec_rebuild_count` / `mox_ec_shards_lost_total`
  - `mox_fshc_disk_fail_count` / `mox_fshc_mountpath_disabled`
  - `mox_tag2graph_lag_ms`（PUT→边落图延迟）
  - `mox_miji_denied_total`（密级裁决拒绝） / `mox_legal_hold_blocked_total`
- 审计链取证 UI 端点：`GET /ops/audit/chain?block_from=1000&block_to=2000&format=html`，返回 JSON 摘要 + 可导出 CSV，提供每个 block `(prev_hash, payload, signature, integrity)` 列。

### FR-7：三档性能基准 Bench（可脚本化）
- CLI 子命令 `mox bench`：
  - `1KB --n 100000 --concurrency 128`：小文件并发 PUT + GET
  - `1MB --n 1000 --concurrency 64`：常规对象
  - `1GB --n 30 --concurrency 8`：大对象 + Multipart
- 每个档输出 `ops/s, throughput(MB/s), p50/p95/p99(ms)`，并对 EC 4+2 再跑一次对比（EC 开销 < 15%）。

### FR-8：Near-data WASM ETL 插件框架
- 插件目录 `./etl-plugins/*.wasm`；
- 注册：`mox etl register --name md5-sum --wasm ./md5.wasm --kind inline-get`；
- 类型：`inline-get`（读时触发）/ `inline-put`（写时触发）/ `offline`（后台异步 xaction）。
- 插件 ABI：`fn transform(input: &[u8], ctx: &EtContext) -> Result<Vec<u8>>`；Context 提供 obj_uri/bucket/miji_level。
- 支持 `GET /s3/b/k?etl=md5-sum` 直接返回处理结果。

### FR-9：磁盘健康 FSHC + Mountpath 热插拔
- 每 60s 后台 `fshc_scan(mountpaths[])`：读 1MB 随机块 + write-then-verify 128KB；失败计数 ≥3 次 → 标记 Mountpath Faulty。
- CLI：`mox mount list / attach / detach / enable / disable`，热插拔后自动 Rebalance → 触发 EC 重建。
- 指标 `mox_fshc_mountpath_disabled{path="/mnt/x"} 1`。

### FR-10：Read-after-Write + E2E CRC64
- PUT 请求支持 `x-amz-checksum-crc64`（或 x-mox-crc64）；服务端计算 vs 客户端送值 mismatch → 拒绝写入（400 Bad Checksum）。
- GET 返回 `x-mox-crc64`；SDK 默认校验开启；审计链 `ChecksumMismatch` 记录。
- 任何 gateway 读同一对象返回同一 etag（强一致）。

### FR-11：独立部署双路径
- **裸机/信创离线**：`./mox server --single-node --data-dir /data/x --port 8080 --no-k8s` → 默认 SQLite backend + in-memory 成员 + 单 mountpath；
- **K8s/Helm**：`helm install mox ./deploy/helm/mox --set standalone=false --set replicaCount=6 --set ec.profile=8+4`；

### FR-12：E2E 回归 ≥ 1237 tests（既有 1117 + 新增 ≥ 120）
- 新增模块单元测试：EC ≥ 16、HTTP3+Multipart ≥ 20、Tag-CDC-Graph ≥ 14、Single-Binary CLI ≥ 10、LegalHold+Miji ≥ 18、Bench ≥ 12、ETL-WASM ≥ 10、FSHC ≥ 10、Observability+CRC ≥ 10。
- 合成 Harness：新增 700 cases（HA 配置 × 网分离 × EC 4+2/8+4 × ETL 开关 × 密级 4 档）。

---

## 3. Non-Functional Requirements (NFR)

| # | 维度 | 目标 |
|---|---|---|
| NFR-1 | 性能（单节点，EC 关） | 1KB ≥ 60K ops/s · 1MB ≥ 1.2 GB/s · 1GB ≥ 8 GB/s |
| NFR-2 | EC 编码开销 | 4+2 ≤ 15% 吞吐下降；8+4 ≤ 25% |
| NFR-3 | Read-after-Write 一致率 | 100%（集群多网关） |
| NFR-4 | Tag→Graph 延迟 P99 | ≤ 500 ms |
| NFR-5 | 单二进制启动冷时间 | ≤ 2.5 s（到健康探针 ready） |
| NFR-6 | 密级/LegalHold 零绕过 | 任何非法访问均 403 + 审计链记录（覆盖率 ≥ 99.99%） |
| NFR-7 | FSHC 坏盘检测 | 3 次连续失败 → ≤ 3 分钟内标记 Faulty |
| NFR-8 | HTTP/3 首字节 TTFB | 对比 HTTPS/1.1 降低 ≥ 30%（高丢包网络） |
| NFR-9 | Helm 一键部署 | `helm install mox --set ...` 后 ≤ 3 min 所有 Pod Ready |
| NFR-10 | 等保三级兼容 | 新增审计链 `ChecksumMismatch / MijiDenied / LegalHoldBlocked / MountpathFault` 4 类 record |

---

## 4. Constraints / Dependencies / Assumptions / Open Questions

### Constraints
- **零第三方商业库依赖**：EC 用 Reed-Solomon 纯 Rust（如 `reed-solomon-erasure` 或自研 SIMD+no_std crate 以兼容信创 LoongArch）。
- **向后兼容**：既有 T17 SDK API 保持不变；仅新增 `multipart_upload` 与 `etl_transform` 方法。
- **审计链 hash_chain 不可破坏**：所有新类型 record 必须遵循既有 chain_append→signature 协议。

### Dependencies
- `reed-solomon-erasure`（或自研）、`quinn`（Rust QUIC/HTTP3）、`wasmtime`（WASM 运行时，可选 feature）、`crc64fast`、`clap`(CLI)。
- 既存 crate：`mox-domain-abstractions`、`mox-graph-service`、`mox-standards`（hash_chain）。

### Assumptions
- 单二进制 "单机模式" 下 mountpath = `<data-dir>/mount-0`，membership 1-node。
- 政企密级裁决默认开启；可通过 `mox.toml [security] enforce_miji=false` 关闭（需审计链记录）。

### Open Questions
- 用户是否需要 **Rustls + 国密 SM2/SM4**（信创 TLS）？当前先按 Rustls 基础 TLS + 可选 feature="gm-sm" 预留。
- ETL WASM ABI 是否优先支持 Python SDK 侧注册？暂定先 Rust CLI + Rust SDK register。

---

## 5. Acceptance Criteria (AC)

> 类型：**rule** = 二值可验证；**rubric** = 0-100 分质量评价。

| ID | 类型 | 验收要求 | 通过阈值 | 证据来源 |
|---|---|---|---|---|
| AC-R1 | rule | EC-Engine：4+2 编码 / 丢失 2 片可恢复 / 丢失 3 片报错；16 场景 tests pass | cargo test -p mox-ec-engine `16/16` | EC crate 测试报告 |
| AC-R2 | rule | Multipart UploadId：Rust/Node/Python SDK 各 7 场景（创建+5 上传+完成），合计 ≥ 21 tests pass | 21 / 21 + CRC 聚合验证 | 三语言 SDK 矩阵 |
| AC-R3 | rule | Tag-CDC-Graph：PUT 5 标签对象 → 5 obj 顶点 + N tag 顶点 + N HAS_TAG 边；反向 obj→tags 查询匹配；14 tests pass | 14 / 14 | tag_cdc_graph tests |
| AC-R4 | rule | 单二进制 CLI：`server --single-node` 启动 + 健康探针 + 5 子命令 ec/mount/legal-hold/miji/bench 存在；≥ 10 tests pass | 10 / 10 0 fail | mox-server integration tests |
| AC-R5 | rule | LegalHold：持有期内 DELETE/PUT-overwrite 均返回 412；密级裁决：高密读低密 OK / 低密读高密 403 / 高密写低密 403；≥ 18 tests pass | 18 / 18 + 审计链 record 类型正确 | miji_legal_hold tests |
| AC-R6 | rule | P99 指标暴露：GET /metrics 包含上述 9 个新指标；审计链取证端点返回 200 + block 完整性校验通过；≥ 10 tests pass | 10 / 10 | observability tests |
| AC-R7 | rule | Bench 三档：1KB/1MB/1GB 各跑 10000/1000/20，输出 ops/s & p99；EC 4+2 对比 EC off 吞吐下降 ≤ 15%；≥ 12 tests pass | 12 / 12 且开销 ≤15% | bench harness |
| AC-R8 | rule | ETL WASM：注册 md5 inline-get → GET 带 etl=md5 返回 md5；注册 off-line → xaction 后台跑新桶 10 对象完成；≥ 10 tests pass | 10 / 10 | etl_wasm tests |
| AC-R9 | rule | FSHC：3 次失败 mountpath 标记 faulty；attach → Rebalance → EC 恢复计数+1；≥ 10 tests pass | 10 / 10 | fshc_mountpath tests |
| AC-R10 | rule | Read-after-Write：并发 PUT 100 对象 → 10 线程 10 轮 GET → etag 100% 一致；端到端 CRC mismatch 拒绝；≥ 10 tests pass | 10 / 10 + 0 不一致 | raw_consistency tests |
| AC-R11 | rule | 双部署：① single-node 启动后 GET `/health` 200；② helm lint mox/ + helm template --set standalone=false 12 资源渲染成功 | ①② 都成立 | deploy smoke |
| AC-R12 | rule | E2E 回归 ≥ 1237 tests（Base 1117 + 新 120 = 1237，fail ≤ 2） | tests_total ≥ 1237 ∧ fail ≤ 2 | t21-e2e report.json |
| AC-U1 | rubric | EC + 磁盘自修复整体工程质量（0-100）：S ≥ 92 / A ≥ 80 / B ≥ 70 | ≥ 92（S） | EC manifest + rebuild code review |
| AC-U2 | rubric | HTTP3 + Multipart + 三网分离工程质量（0-100） | ≥ 90（S） | listener + SDK multipart 一致性 |
| AC-U3 | rubric | 云盘×知识图谱融合体验（0-100）：5 标签秒出图 + 反向回查 S3 HEAD 成功率；易用性 | ≥ 92（S） | Fusion UX walkthrough |
| AC-U4 | rubric | 单二进制部署体验（0-100）：零依赖启动时间 / 命令一致性 / 日志完备度 | ≥ 90（S） | single-node smoke + UX 评审 |
| AC-U5 | rubric | 政企合规体验（0-100）：密级 4 档 Bell-LaPadula + LegalHold 覆盖度 + 审计链取证导出；等保就绪度 | ≥ 94（S） | compliance matrix 9×9 |
| AC-U6 | rubric | P99 观测 + 取证仪表可用性（0-100）：指标命名、Grafana dashboard JSON、取证页 CSV 导出 | ≥ 88（A） | dashboard + evidence |
| AC-U7 | rubric | ETL + Bench + FSHC + 独立部署组合综合体验（0-100） | ≥ 86（A） | ops workflow 走查 |
| AC-U8 | rubric | 综合 Grade（加权：U1 15% U2 15% U3 20% U4 10% U5 20% U6 8% U7 12%）；目标 Grade S | ≥ 90（S） | rubric-t21-all.json |

---

_参考：NVIDIA AIStore In-depth overview (docs.nvidia.com/aistore/overview), Networking Model v5.0 (docs.nvidia.com/aistore/networking), ETL overview (docs.nvidia.com/aistore/etl)._
