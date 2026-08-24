# 规格：璇玑 RelGraph V5 · 云盘×关系图 全部自研（云平台 API 兼容 · 标准规范 · 全链路 TDD 闭环）

> 规格语言：中文
> 治理中枢：璇玑归一化知识图谱（6 层双向绑定铁律）
> 定位：**L2~L6 全自研（璇玑自有版权 + 全部代码自己实现）；L7 仅允许 Apache 2.0 / MIT 协议的单库级组件（RocksDB / async-raft / sha2 / crc 等），禁止套入 SeaweedFS / NebulaGraph 等成品开源系统**
> 兼容标准：**AWS S3 API v20060301 (30 最常用 100% / 80 API ≥ 90% 兼容) + POSIX IEEE 1003.1 + nGQL 子集 + openCypher MATCH/CREATE 常用子集 + ISO GQL 基础兼容 + AIS 七层层抽象 DIP + 精度护栏 / 路由护栏 / 等保三级审计 hash_chain**
> 前置基线（Spec Mode 实跑 GREEN）：SPEC-1 双写回源 / SPEC-2 FS-S3 Chunk + MPU / SPEC-3 Nebula L1 CDC / SPEC-4 CNM+RAW+精度 / SPEC-5 PR转置 / SPEC-6 Rust Gateway / SPEC-7 rerank+CEM / SPEC-8 协议兼容 / SPEC-10 三流程 Trace / SPEC-13 SLO / SPEC-14 HA / SPEC-15 129 GREEN / SPEC-V4 S 级（review2）

---

## 1. 问题、目标用户、目标、非目标

### 1.1 问题陈述（用户最高决策：全部自研，兼容云平台 API + 规范标准 + 全链路）

| 类别 | 编号 | 客观问题（必须本规格内全部解决）|
|---|---|---|
| 自研范围不清晰 | S-01 | 上一版分析给出「从零全自研」和「半自研」两档，用户**明确拍板：全部自研**；必须定义边界：L2/L3/L4/L5/L6 璇玑自有代码 100%；L7 = 单库 Apache2.0/MIT 组件允许（RocksDB/async-raft/crc 等），但不得套入 SeaweedFS/Nebula 成品系统（白盒集成即套入=违规）|
| 云平台 API 兼容缺失 | S-02 | 兼容 S3 AWS API 标准需量化（不是口头兼容）；30 个最常用 S3 API 必须 100% 协议兼容（s3cmd / mc / boto3 三客户端全验证绿）|
| 图查询语言标准缺失 | S-03 | nGQL 60 语句 + openCypher 常用 MATCH/CREATE 20 语句必须标准兼容；Nebula Studio / neo4j-browser 原生客户端至少连通并返回标准结果集 |
| 规范标准缺失 | S-04 | POSIX IEEE 1003.1 fio + pjd-fstest ≥ 95%；S3 signature v4 v2 兼容；CRC32C/ETag 算法严格等于 AWS；nGQL 返回列完全等于标准；审计 hash_chain 等保三级不可篡改；License 合规（全部自研代码 MIT/Apache2 双开）|
| 全链路治理缺失 | S-05 | 分析 → 设计 → 开发 → 测试 → 修复 → 优化 → 验收 → 运维 8 阶段 每阶段必须：① 写回 6 层图谱；② /atlas/verify 绿；③ TDD RED→GREEN 闭环；④ 独立 Review 隔离上下文 |
| 协议感染红线 | S-06 | 坚决拒绝 AGPL / GPL v3 协议感染进入依赖树（白名单 Apache2.0/MIT/BSD-2/ISC，CI license-scanner 强制阻断）|

### 1.2 目标用户（企业级 6 类，与 SPEC-V4 相同但加 2 类新标准角色）

| 用户层 | 关注价值 |
|---|---|
| 企业决策者（CTO/CIO/采购/法务）| **100% 自有版权** 证明；信创兼容认证；等保三级报告；License 白名单无 AGPL；TCO 比商业版低 |
| 架构/工程委员会 | 全部自研符合 AIS 七层 DIP；L2~L6 代码全自有；L7 单组件可替换；标准接口可插拔 |
| 开发专家联盟 | 12 里程碑 × 每里程碑 10+ GREEN TDD；规范统一；6 层图谱写回闭环；精度护栏锁死 |
| 算法联盟 | 7 核心算法 Rust 单源不变；兼容标准 nGQL；扩展图算法库 20+ 条新算法自研 |
| SRE / 运维联盟 | S3/nGQL 标准接口 → 标准监控工具直接接（Prometheus exporter / Grafana 社区模板可用）；故障注入 HA 14 类 GREEN |
| 开源用户社区 | MIT/Apache2 双开源；标准兼容 S3 SDK / Cypher SDK 直接用，不用学新 API |
| **信创适配工程师**（新） | 鲲鹏 / 飞腾 / 海光 / 兆芯 CPU × 统信 / 麒麟 OS 物理机回归；100% 源码可控可编译 |
| **等保三级审计师**（新） | 审计 hash_chain 不可篡改链 180 天；日志标准 RFC 5424；签名算法 HMAC-SHA256-FIPS |

### 1.3 目标（Must / Should / Nice-to-have）

#### Must（验收 Gate，任何一条 fail = Review fail）

1. **全部自研边界清晰**：L2 (Rust Gateway) / L3 (Node EAF) / L4 (Services crates) / L5 (Domain traits) / L6 (Kernel 纯 std) 所有代码 100% 璇玑实现；L7 仅 = RocksDB(Apache2.0) + async-raft(Apache2.0) + libc/POSIX(系统接口) + sha2/hmac/crc 单算法 crate；**绝不引入/嵌入/套 SeaweedFS / JuiceFS / MinIO / Neo4j / NebulaGraph 等「成品开源系统」作为内部依赖（直接二进制调用 = 违规）**
2. **S3 兼容**：30 个最常用 S3 API 100%（ListBuckets / Put/Get/Delete/Head/Copy / MultipartUpload 4 + DeleteMultipleObjects / Get/Put ObjectAcl / BucketVersioning / Get/Put BucketPolicy / Get/Put Lifecycle / ListMultipartUploads / ListParts / ListObjectVersions / Get/Put ObjectTagging / Get/Put BucketTagging / Get/Put BucketCors）；使用 mc / s5cmd / boto3 三客户端 **s3 冒烟套件 100/100 GREEN**
3. **POSIX 兼容**：IEEE 1003.1 pjd-fstest ≥ 95%（900 case ≥ 855 pass）；fio 顺序/随机读写 4 场景全绿；POSIX 语义 = 璇玑客户端 libfuse（Linux）/ dokan（Windows）全部自研实现
4. **图查询标准兼容**：① nGQL 60 条常用语句（INSERT VERTEX/EDGE / LOOKUP / GO / FETCH / MATCH PATH / CREATE SPACE / CREATE TAG / CREATE EDGE / ALTER / DROP / SHOW SPACES / SHOW HOSTS / SUBMIT JOB 等）100%；② openCypher MATCH / CREATE / MERGE / DELETE / RETURN 20 语句 ≥ 95%；③ Nebula Studio 与 neo4j-browser 至少连通并返回标准列
5. **规范标准清单 10 项全过**（§3 标准表）；License 白名单扫描 = 0 违规
6. **全链路 8 阶段闭环**：每阶段对应 6 层图谱节点 + 边 + /atlas/verify 8 项绿 + 独立 Review（Reviewer 上下文隔离）
7. **继承所有前置基线护栏**：PPR d=0.85 maxIter=30 / CNM Newman / Brandes / Harmonic / RAW 双向 / Density 无 toFixed / LPA 禁用公域 / Router AC-10 / SLO p99≥99.9 / RPO=0 / RTO<60s / 三流程 Trace E2E GREEN 等 129 GREEN 基线永不退步
8. **全量回归 GREEN ≥ SPEC-V4 基线 706 / 28 条 rule AC 全通过**

#### Should（质量提升）

9. S3 兼容 80 个 API ≥ 90%
10. nGQL 扩展至 100 条语句 + GQL ISO 子集 20 条
11. 中文运维文档 1,000 页（对标 NebulaGraph 手册 830 页体量）
12. 信创 5 套物理机回归 100%（鲲鹏/飞腾/海光/兆芯 × 统信 × 麒麟）

#### Nice-to-have

13. s3api 全 104 API ≥ 80% 兼容；Cypher GDS 算法库 50 条
14. K8s Operator（自研 Xuanji Operator）一键扩缩容

### 1.4 非目标（严禁进入本 Spec，违规 = Cancelled）

- ❌ **绝不**：引入 SeaweedFS / JuiceFS / MinIO / Ceph 任何一款「成品分布式存储系统」做内部组件（代码引用 / 二进制调用 / sidecar 部署均违规）
- ❌ **绝不**：引入 NebulaGraph / Neo4j / JanusGraph 任何一款「成品分布式图数据库」做内部组件；图存储/计算/元数据 100% 璇玑代码
- ❌ **绝不**：引入 AGPL / GPL v3 / SSPL 协议依赖（含 `cargo deny` + license-scanner CI 阻断）
- ❌ **不要**：改算法护栏（PPR / CNM / Brandes / Harmonic / RAW / Density / LPA）
- ❌ **不要**：改 Router AC-10 语义
- ❌ **不要**：新建前端 UI 组件；只交付端点 / SDK / CLI / Helm / 文档 / 测试

---

## 2. 功能需求（Functional Requirements）

| 编号 | 需求（全自研 + 标准兼容）| 类型 |
|---|---|---|
| FR-01 | L5 xuanji-domain-abstractions 新增 10 大 trait（ObjectStorage / MetaStorage / ChunkManager / Quota / Iam × 5；GraphQuery / GraphMeta / GraphAlgoSingle / PartitionRouter / CdcPublisher × 5），TDD mock 先 RED→GREEN 各 ≥ 5 case | rule |
| FR-02 | 云盘 L4 xuanji-cloud-drive Master/Volume 拓扑层（卷分配/心跳/N×副本/快照恢复）— 100% 璇玑代码实现 | rule |
| FR-03 | S3 Service 30 API 全自研 100%：签名（SigV4 / SigV2）、分块、ETag、MPU、Versioning、ACL、Tagging、Policy、Lifecycle、Cors | rule |
| FR-04 | POSIX Filer 自研：mkdir/rename/symlink/chmod/xattr/… + 3 Meta 后端（SQLite dev / Postgres+Citus prod / Redis cluster）— 100% 璇玑代码 | rule |
| FR-05 | FUSE 客户端自研（Linux libfuse / Windows dokan-rs wrapper）；支持 POSIX 标准读写；pjd-fstest ≥ 95% | rule |
| FR-06 | 云盘冷热分层 TieringService（JuiceFS 架构思想自研实现）+ IAM Policy 引擎 + STS AssumeRole + Quota 用户级/目录级 — 璇玑自研 | rule |
| FR-07 | 关系图 Meta Service 三节点 Raft（async-raft Apache2.0 仅作协议库，Meta 逻辑 100% 璇玑）：Schema Tag/EdgeType/索引、权限、分区路由、心跳、快照备份恢复 | rule |
| FR-08 | 关系图 Storage Service（RocksDB KV 仅作底层 K-V 库，分片/Raft/Storage 5 API/getNeighbors/CDC 全部璇玑自研逻辑 100%）| rule |
| FR-09 | 关系图 Graph Service 无状态：nGQL 60 条 Parser + Optimizer + 7 Rust 算法接入 + openCypher 转换；全部璇玑自研 | rule |
| FR-10 | Flink CDC 连接器（璇玑 Source/Sink，Java/Scala 社区可选外包 SDK）+ Spark Connector Connector 自研 + Graph Projection 子图分析 | rule |
| FR-11 | 标准接口验证：mc (MinIO 客户端)100 case GREEN + s5cmd 100 case GREEN + boto3 Python SDK 100 case GREEN = **300 条 S3 兼容互验 GREEN** | rule |
| FR-12 | 标准图客户端验证：Nebula Studio 连 Graph Service 返回标准列 10 条查询 GREEN + neo4j-browser MATCH/CREATE 10 条 GREEN | rule |
| FR-13 | 规范标准清单 10 项：POSIX IEEE 1003.1 / AWS S3 SigV4 / CRC32C / RFC 5424 日志 / FIPS 140-2 HMAC / nGQL / openCypher / ISO GQL 子集 / AIS 七层抽象 / 等保三级审计 — 对照测试全 GREEN | rule |
| FR-14 | License 合规：cargo deny + license-scanner 双工具 CI = 0 AGPL/GPL/SSPL 违规；白名单 Apache2.0/MIT/BSD-2/ISC | rule |
| FR-15 | 全链路 8 阶段闭环：分析 → 设计 → 开发 → 测试 → 修复 → 优化 → 验收 → 运维 每阶段 6 层写回 + /atlas/verify GREEN + 独立 trace ID | rubric |
| FR-16 | 信创物理机兼容：鲲鹏 920 + 统信 UOS × 麒麟 V10 双 OS `cargo test -p xuanji-cloud-drive -p xuanji-graph-storage --release` 全 GREEN | rubric |
| FR-17 | S3 兼容扩展：80 API ≥ 90%；POSIX pjd-fstest ≥ 98%；nGQL 扩展 100 条 95% | rubric |
| FR-18 | 中文文档 1,000 页齐备（云盘 500 页 + 关系图 500 页，对标 NebulaGraph 830 页手册密度）| rubric |
| FR-19 | 6 层图谱边密度（新增 5+ crate + 8 阶段节点）≥ 0.15（SPEC-V4 AC-23 = 2/2）| rubric |

---

## 3. 标准规范兼容矩阵（10 大标准 × 对应验收方法）

| # | 标准 | 版本/引用 | 自研实现要求（璇玑必须 100% 实现核心条款）| 验收方法 |
|---|---|---|---|---|
| 1 | POSIX 文件系统接口 | IEEE Std 1003.1-2017 | FUSE 自研客户端支持 open/close/read/write/seek/fsync/mkdir/rmdir/unlink/symlink/readlink/rename/chmod/chown/statfs/getxattr/setxattr/listxattr/removexattr 共 20 系统调用 | pjd-fstest 900 套件 ≥ 95% + fio 4 场景 100% |
| 2 | AWS S3 REST API | v2006-03-01 / SigV4 | 30 最常用 API 100%；ETag=MD5(parts concat)；Content-MD5；ListObjects v1/v2；Bucket/Object 版本化 | mc (MinIO 客户端) 100 + s5cmd 100 + boto3 100 = 300 GREEN |
| 3 | CRC / 哈希校验 | RFC 3720 iSCSI CRC32C / FIPS 180-4 SHA-256 | 对象写入 CRC32C；元数据 SHA-256；版本 manifest HMAC-SHA256（FIPS 模式）| unit 10 case + bit-flip 故障注入 14 GREEN |
| 4 | 审计日志格式 | RFC 5424 (The Syslog Protocol) | 所有审计事件（上传/下载/权限/删除/版本）遵循 RFC 5424 Structured Data 格式；syslog 远程导出功能 | test-syslog-rfc5424.js 50 GREEN |
| 5 | 安全 HMAC 签名 | FIPS Pub 140-2 HMAC-SHA-256 | S3 SigV4 签名严格遵循 AWS Signature V4 算法；审计 hash_chain = HMAC-SHA256；不可篡改链验证 | test-sigv4.js 30 GREEN + hash_chain 验证 10 GREEN |
| 6 | nGQL 图查询语言 | NebulaGraph 3.6.0 nGQL 标准 | 60 语句 100%：SPACE/TAG/EDGE DDL + INSERT/LOOKUP/GO/FETCH/MATCH/SHOW/SUBMIT JOB；返回列名、类型、null 语义与标准一致 | ngql_conformance 60 GREEN |
| 7 | openCypher 查询 | openCypher 9 / GQL 草稿 ISO/IEC 39075 | MATCH / CREATE / MERGE / DELETE / RETURN / WHERE / WITH / ORDER BY / LIMIT 20 常用语句 ≥ 95% 兼容 | cypher_conformance 20 GREEN |
| 8 | ISO GQL（Graph Query Language）基础 | ISO/IEC 39075:2024（Nebula v5 对齐）| 标准 GRAPH / NODE / EDGE / PATH 类型映射；SELECT FROM GRAPH 20 条子集（Nice-to-have ≥15）| gql_iso 20 GREEN（Nice-to-have）|
| 9 | AIS 七层抽象 DIP | 璇玑内部标准（SPEC-V4 T7 19 GREEN）| L2 Gateway / L3 EAF / L4 Services / L5 Traits / L6 Kernel(std-only) / L7 Infra 边界；跨 crate 依赖仅经 L5；Kernel 0 extern；Package 继承 workspace=true | cargo audit + t7 测试 + /atlas/verify AIS check |
| 10 | 等保三级审计留痕 | GB/T 22239-2019 三级 / DJCP01-2024 | ① 审计日志 180 天；② 不可篡改 hash_chain；③ 操作留痕（创建/读取/更新/删除/权限/导出）；④ 日志导出 CSV/JSON-LD；⑤ FIPS HMAC 签名算法 | test-audit-grade3.js 50 GREEN |

---

## 4. 非功能需求（NFR）

| 维度 | 指标 | 验收方法 |
|---|---|---|
| **性能** | 云盘：写 1 GB/s / 读 2 GB/s（4 节点集群）；10k 小文件写后读 P95 ≤ 5 ms。关系图：2 跳 100k 子图 ≤ 200 ms / 3 跳 ≤ 5 s / 插入 100k/s 单节点。 | 集群压测 + T15 专项 |
| **稳定性 / HA** | SPEC-14 14 故障注入（MinIO 等效 kill-2 EC 重建 / Nebula 等效 kill-1 Raft 节点 / Sidecar 挂 / Gateway 崩溃）GREEN；RPO=0；RTO<60 s；SLO p99 ≥ 99.9% | T15 HA + SLO |
| **兼容性** | 标准 10 项矩阵全 GREEN；S3 三客户端 300 GREEN；图客户端 Nebula Studio/neo4j-browser 各 10 GREEN | §3 验收 |
| **可扩展** | 云盘：3→30 节点扩容不停机；关系图：Storage 16→32 分片扩容不停机 / Graph 无状态横向扩容。 | T15 扩容测试套件 |
| **可部署** | Helm `helm install xuanji-standard-cluster`：OSS 版（3 节点）≤ 20 min；Enterprise 版（9 节点：Meta×3 / Storage×3 / Graph×3）≤ 45 min；Helm K8s Operator 后续 Nice。 | Helm smoke 实跑 |
| **可二次开发** | L5 trait 文档齐全；L4 crate README 8 节齐；SDK：Rust（cloud / graph 2 个 SDK crate）/ Node / Python 3 个官方 SDK + 30 用法示例 | SDK 各 10 GREEN + 二次开发指引评审 |
| **可观测** | OTel 标准 Metrics/Traces/Logs 三信号；Prometheus exporter；Grafana dashboard JSON（对标 MinIO/Nebula 社区模板）| /metrics 端点全指标 ≥ 200 条 |
| **合规** | License 白名单 0 违规；等保三级 50 GREEN；信创 5 套物理机回归；Rust 二进制 SBOM 生成（cyclonedx）| license-scanner + 等保测试 + 信创回归 |
| **安全** | OWASP Top10 全覆盖；S3 SigV4 签名严格 + ACL RBAC；桶级 IAM；图查询级资源隔离（CPU 内存限流）| 安全扫描 `cargo audit` + `npm audit` + OWASP Top10 套件 |

---

## 5. 约束、依赖、假设、开放问题

### 5.1 强约束（违反 = 直接 fail）

- **自研边界红线（最高）**：L2/L3/L4/L5/L6 = 100% 璇玑代码；L7 = 仅限 RocksDB / async-raft / sha2 / hmac / crc / libc / POSIX 单库接口；**禁止引入/嵌入/白盒集成任何分布式成品系统（SeaweedFS / JuiceFS / MinIO / Ceph / Nebula / Neo4j / JanusGraph 等）**——集成即违规。
- **协议红线**：依赖树禁止 AGPL v3 / GPL v3 / SSPL / BSL（非 OSI 标准协议）；CI 自动跑 `cargo deny` + `license-scanner`，任何一条违规 = build 失败。
- **算法 / 路由 / 审计护栏红线**：PPR d=0.85 / maxIter=30；CNM Newman 公式；Brandes / Harmonic；RAW 双向；Density 无 toFixed；LPA 公开禁用；Router AC-10 语义；审计 hash_chain sha256 HMAC 180 天 TTI。
- **AIS 分层红线**：L6 Kernel 仅 std/alloc/core，L5 仅 trait，L4 业务逻辑，L7 单库 RocksDB 等；跨层调用必须经 L5 trait（DIP）。
- **License 输出红线**：璇玑自研代码输出 MIT + Apache 2.0 双 License（方便商业友好）。

### 5.2 依赖（External Dependencies，白名单且版本 workspace 统一）

- Rust 生态白名单（均 Apache 2.0 / MIT 协议）：
  - 存储：rocksdb 0.22 / sled（备选 Apache）
  - Raft：async-raft（Apache2.0）
  - 哈希 / 校验：sha2 / hmac / crc32c / md5（ETag MD5 兼容 AWS S3 标准要求）
  - FUSE：fuser（Linux libfuse 安全封装）/ dokan（Windows 可选）
  - 序列化：serde / serde_json（与 kernel_ext 双结构一致）
  - Web：axum（Gateway）/ hyper（S3 Service）
  - 测试：tokio / criterion / mockall / proptest
- Node.js 生态（MIT 协议白名单）：workspace 已有依赖；新增仅：ajv JSON Schema v8 / s3-verify（签名验证自研）
- 系统接口（POSIX / syslog / socket）属系统标准接口，不算"引入开源系统"。

### 5.3 假设（不成立时必须升级为 Issue 追踪）

1. 团队 8 人配置可到位（Rust 4 / Node 2 / 架构 SRE 1 / 项目文档 1）
2. SPEC-1~SPEC-V4 基线 706 GREEN 永不退步；不会出现"改旧代码破坏基线"的回归
3. 目标用户可以接受"分 12 里程碑上线"（过渡期用：Cloud = SPEC-2 FS/S3 ChunkBackend；Graph = Nebula Adapter Mock）——**过渡期不违规，因为只是"上线前业务临时用"，自研完成后平滑切换**

### 5.4 开放问题（Spec Mode 批准前必须 3 选 1 决定）

1. **Q1 过渡期策略**：今天用户拍板，同意业务方等 7 个月？还是过渡期上线"兼容 S3/nGQL 的薄壳"（SPEC-2/Nebula Mock），自研完后无停机切换？→ **推荐同意过渡期薄壳（100% 符合用户"兼容云平台 API"要求，过渡期就是兼容 S3 接口）**
2. **Q2 标准开源协议输出**：MIT 单协议？还是 MIT + Apache2.0 双协议？→ **推荐双协议（商业用户可选 MIT 宽松；社区用户 Apache 2.0 强专利保护）**
3. **Q3 信创物理机预算**：是否已有鲲鹏/飞腾物理机？还是需要云厂商 ARM 仿真？→ **推荐先 ARM 仿真跑通，再采购 2 台物理机做信创验收**

---

## 6. 验收标准（Acceptance Criteria）——仅 rule 或 rubric（Spec Mode 要求）

### Rule（22 条，可观察二值）

| 编号 | 验收内容（可观察 pass 条件 + 证据源）|
|---|---|
| **AC-01** | cargo deny + license-scanner 报告 0 AGPL/GPL/SSPL/BSL：`cargo deny check licenses` exit 0 + license-scanner 输出 `0 violations` |
| **AC-02** | L6 Kernel grep 7 条 forbidden crate（serde/nalgebra/ndarray/thiserror/anyhow/tracing/uuid）= 0 匹配：t7_kernel_zero_external_deps test GREEN |
| **AC-03** | 6 层图谱 AIS verify ais_layers_compliant ok=true：`GET /atlas/verify` check ais_layers ok=true |
| **AC-04** | S3 30 API mc 客户端 100 case GREEN：`mc-smoke` 套件 100/100 |
| **AC-05** | S3 30 API s5cmd 客户端 100 case GREEN：`s5cmd-smoke` 100/100 |
| **AC-06** | S3 30 API boto3 客户端 100 case GREEN：`boto3-smoke` 100/100 |
| **AC-07** | POSIX pjd-fstest ≥ 95%：`pjd-fstest` 900 case ≥ 855 pass |
| **AC-08** | nGQL 60 语句套件 60/60 GREEN：`ngql-conformance` 60/60 |
| **AC-09** | openCypher 20 语句套件 ≥ 95%：`cypher-conformance` ≥ 19/20 |
| **AC-10** | Nebula Studio 连通 + 10 条标准查询 GREEN：`nebula-studio-smoke` 10/10 |
| **AC-11** | neo4j-browser 连通 + 10 条标准 MATCH GREEN：`neo4j-browser-smoke` 10/10 |
| **AC-12** | 等保三级 hash_chain 不可篡改 + RFC 5424 日志：`test-audit-grade3.js` 50/50 |
| **AC-13** | S3 SigV4 签名严格等于 AWS 算法：`test-sigv4.js` 30/30（签名向量对照 AWS 官方向量）|
| **AC-14** | CRC32C / ETag 算法等于 AWS 标准：`test-etag-crc32c.js` 20/20 |
| **AC-15** | 云盘 3 AZ 14 故障注入（SPEC-14 基线等效）全 GREEN：14/14；RPO=0；RTO<60 s |
| **AC-16** | 关系图 Meta 3 节点 kill-1 Raft ≤ 5 s 自恢复：3 轮测试 kill-1 leader 均 ≤ 5 s |
| **AC-17** | 关系图 Storage 分片自动平衡 16→32：分片差 ≤ 10% 5 min 内 |
| **AC-18** | 全自研代码边界：grep 仓库代码 `seaweed|juicefs|minio|ceph|nebula-graph|neo4j|janusgraph`（排除 README/文档/注释）= 0 条（证明未引入成品开源系统）|
| **AC-19** | 8 阶段 trace：每阶段 traceId 存在 + 6 层图谱节点存在 + /atlas/verify ok=true：3×8=24 check 全绿 |
| **AC-20** | 全量回归 Node GREEN ≥ 706（SPEC-V4 基线不退步）+ Rust workspace test / build / clippy 全 0 fail |
| **AC-21** | Router AC-10 语义：`router_semantics.rs` 全绿（精度/路由护栏不退步基线）|
| **AC-22** | Helm 一键部署：OSS ≤ 20 min；Enterprise ≤ 45 min：smoke 实跑计时 |

### Rubric（7 项，量化打分 0/1/2 + 阈值）

| 编号 | 维度 | 刻度（0=低锚 / 1=中 / 2=高锚）+ 阈值 | 证据源 |
|---|---|---|---|
| **AC-23** | 全链路 8 阶段闭环完整度（FR-15）| 0：<4 阶段闭环；1：4~7 个；2：全部 8 阶段。阈值 ≥ 2。| /atlas/verify trace_lifecycle check 报告 + 6 层图谱 nodes 计数 per 阶段 |
| **AC-24** | 信创物理机兼容度（FR-16）| 0：0 套全绿；1：2~3 套全绿；2：鲲鹏/飞腾/海光/兆芯/统信/麒麟 5 套全绿。阈值 ≥ 1。| 物理机 cargo test --release 日志（或 QEMU ARM 仿真）|
| **AC-25** | 扩展兼容度（FR-17）| 0：S3<60 API / POSIX<95 / nGQL<60；1：S3≥80 90% + POSIX≥96% + nGQL 80 90%；2：S3 104 API ≥ 80% + POSIX≥98% + nGQL 100 95% + GQL 20 ≥15。阈值 ≥ 1。| 扩展套件结果 |
| **AC-26** | 中文文档完备度（FR-18，对标 NebulaGraph 830 页密度）| 0：<500 页；1：500~999 页齐备；2：≥ 1000 页章节结构完整（目录/部署/API/SQL/运维/案例/附录）。阈值 ≥ 1。| docs 目录 8 章节齐全 + wc -l 页数估算（每 250 行 ≈ 1 页）|
| **AC-27** | 图谱边密度（FR-19）| 0：<0.10；1：0.10~0.15；2：>0.15。阈值 ≥ 2。| /atlas/verify six_layer_density detail |
| **AC-28** | TCO 节约度（vs 商业版 7 年）| 0：<40% 节约；1：40%~70%；2：>70%。阈值 ≥ 1。| TCO 计算表（半自研36 vs 商业 1050 × 自有 8 人团队）|

---

## 开放问题 3 条（批准前用户拍板即可）：Q1 过渡期薄壳策略 / Q2 双开源协议 / Q3 信创物理机预算
