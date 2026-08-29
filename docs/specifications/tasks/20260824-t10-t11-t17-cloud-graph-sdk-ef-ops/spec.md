# 璇玑企业级 SPEC：T10 云盘 M4 · T11 关系图 R4 · T17 官方 SDK ×3 语言 · E/F 批次运维落地

> **SPEC 编号**：DOC-SPEC-20260824-T10T11T17-EF  
> **版本**：v1.0 ENT  
> **最后更新**：2026-08-24  
> **权威链**：本 SPEC ← `18-全域顶层总设计` (L0) ← `22-全文档归一化总控卡` (L1 治理) ← `20-后端开发主控` / `26-前端开发主控` / `27-企业级测试主控`。  
> **硬约束引用**：project_memory（melody2score 会话化死锁、窗口化 stderr、CEM AI 优化、4 端点、CNM、Brandes、harmonic、PageRank 转置、RAW 边双向展开、全精度、人读公式、节点→边构建顺序）。

---

## 0. 问题陈述、用户与目标

### 0.1 Problem Statement
当前璇玑仓库已完成 M0 全域归一化 + 三联盟模式 + 基础 10 task 企业级冒烟（D1-D6 36/36 通过）。但：

1. **云盘 T10**：`mox-cloud-drive-s3/master/volume/filer` 4 crate 仅实现 S3 API 骨架；`SecurityManager` 的 IAM Policy 为 Mock 形态，无 Bucket Policy 10 条真实矩阵验证；STS AssumeRole 的 TTL=900s（15 min）硬约束未在 SDK/后端联合验证；Quota 429 响应（含 `Retry-After` + `X-Quota-*` 头）未接入 API 层；**等保三级 hash_chain 审计**（国密合规基线）在 `mox-standards/dengbao_skeleton` 中仍为 placeholder，与 `mox_expert::audit::AuditContext` 未打通，无法满足 GB/T 22239-2019 三级可追溯要求。
2. **关系图 T11 R4**：`mox-graph-storage::cdc_source` 实现了 Raft → CDC 事件，但 Flink CDC Source Connector 10 万级无丢重语义（at-least-once + idempotent writer = exactly-once 可验证）仅为 trait 契约未验证；**Spark Connector 读写**（GraphX DataFrame → bulkUpsert → GraphX）无实现；**子图 Projection 20 算子**（按 type/community/attr/hop 等条件投影 + 20 种组合）仅 concept 层；**AC-15 故障注入 14**（断链/重复/乱序/字节翻转等 14 故障矩阵）缺少完整测试。
3. **官方 SDK T17**：`mox-sdk-cloud` / `mox-sdk-graph` (Rust) 仅为 1 行 stub；Node.js / Python SDK 目录不存在；**30 示例 × 3 语言 = 90 示例**（cloud×15 + graph×15 每语言）为零；测试覆盖 ≥80 的基线差距巨大。
4. **E/F 运维批次**：T12 (T5 Helm DR) / T13 (T5 信创 + 手册) / T15 (HA + 容量 + TCO rubric) / T18 (8 阶段 trace) / T19 (全量回归 ≥706) / T20 (Helm 一键 + 灰度 1→10→50→100 warmup) 全部缺少可运行脚本与验收矩阵。

### 0.2 Users
- **企业架构师 / 安全合规官**：验收 T10 IAM 10 条 + STS TTL + hash_chain 等保合规、签署 T19 706+ 回归报告
- **数据工程师 / SRE**：使用 T11 Flink CDC / Spark Connector、运行 T12/T15/T18/T20 脚本
- **三方应用开发者**：使用 T17 Rust/Node/Python SDK 30×3=90 示例集成云盘 + 关系图
- **测试联盟（独立质量裁判）**：执行 ≥60(T10) + ≥40(T11) + ≥80(T17) + ≥706(T19) 总计 ≥886 测试并签署 27 §T6 判定

### 0.3 Goals (G1–G12)
| # | 目标 | 对应批次 |
|---|------|---------|
| G1 | T10：云盘冷热分层落地（HOT≤30d → WARM 30-90d → COLD>90d 生命周期引擎，含读回热自动回温），≥60 tests 全绿 | T10 M4 |
| G2 | T10：IAM Policy Engine 10 条完整矩阵（Admin/Owner/Editor/Viewer/Guest + S3 CRUD + PublicRead/DenyIP/DenyUnlessMFA + ResourceTag + VPCSource 共 10 条），逐条 evaluate 验证 | T10 M4 |
| G3 | T10：STS AssumeRole TTL=900s（15 min）硬约束：`expiration - now ∈ [899,901]`（±1s 容差），session_token 使用 HMAC-SHA256 签名可自证，过期凭证自动失效 | T10 M4 |
| G4 | T10：Quota 429 响应标准：HTTP 429 + `Retry-After` + `X-Quota-Used`/`X-Quota-Limit`/`X-Quota-Reset` 头；后端 TokenBucket 与 QuotaProvider 双轨并发写安全 | T10 M4 |
| G5 | T10：等保三级 hash_chain 审计（GB/T 22239-2019 7.4.2.3 可追溯）替换 `dengbao_skeleton`：每条审计 event HMAC(prev_hash+event) 链式绑定；支持独立校验脚本；WORM 写入策略 | T10 M4 |
| G6 | T11：Flink CDC Source 10 万 Vertex/Edge 无丢重：`cdc_source` + FlinkSourceIterator 的 at-least-once + offset checkpoint + idempotent upsert → 实测 exactly-once（丢=0 重=0） | T11 R4 |
| G7 | T11：Spark Connector 读写实现：GraphX VertexRDD/EdgeRDD ↔ `POST /graph/bulk`；读端 `GET /graph` + 分页 → DataFrame；Schema 与 Nebula/nGQL 对齐 | T11 R4 |
| G8 | T11：子图 Projection 20 算子实现（5 过滤 × 2 方向 × 2 hop = 20）：按 {type,community,attr_range,degree_bucket,label_regex} 5 条件 × {in/out} 2 方向 × {1hop/2hop} 2 hop = 20 组合 | T11 R4 |
| G9 | T11：AC-15 故障注入 14：断链(3)/重复(2)/乱序(2)/字节翻转(2)/网络分区(1)/磁盘满(1)/oom(1)/慢请求(2) = 14，按 GB/T 22239-2019 A.4 AC-15 映射 | T11 R4 |
| G10 | T17：3 语言 SDK 全栈实现 + 90 示例（每语言 cloud×15 + graph×15）+ ≥80 tests（Rust≥30 / Node≥30 / Python≥20）；SDK 协议对齐 Gateway 4 端点 + 图谱/云盘 API | T17 |
| G11 | E 批次：T12 (Helm DR 3 副本 1 主 2 从 + 自动 failover 可测) + T13 (信创适配清单 + 部署手册 7 章) + T15 (HA 99.95% + 容量规划 + TCO rubric 4 档) + T18 (8 阶段 trace span 全链路串联) | E |
| G12 | F 批次：T19 全量回归 ≥706 tests 脚本 1 键触发；T20 Helm 一键部署 + 灰度 1→10→50→100 warmup 权重脚本 4 阶段可观测（含 metrics 验证） | F |

### 0.4 Non-Goals (NG)
- NG-1：不生产部署真实物理集群（MinIO/Flink/Spark/Neo4j 集群）；仅本地 standalone / embedded / 容器 compose 可验证形态通过验收。
- NG-2：不重写既有 Rust crate 的对外 trait 契约（`IamProvider` / `QuotaProvider` / `CdcSource` / `AuditSink`）；内部实现替换必须保持 trait 签名零变。
- NG-3：不新增 frontend-ui 视图；仅 SDK 示例可引用现有 API（26 §前端越权保护）。
- NG-4：不接入真实云服务商；全部 S3 / STS / CDC 可用本地 MinIO + Mock。
- NG-5：不修改 00-INDEX / 18-TOP-MASTER / 22-NORMALIZATION-CARD 的权威字段；如有冲突一律改本 SPEC 下游实现。

---

## 1. 约束、依赖、假设、开放问题

### 1.1 Constraints (硬约束，不可谈判)
| # | 约束 | 来源 |
|---|------|------|
| C-1 | CNM 社区检测（禁 LPA）；Brandes 介数；harmonic 紧密；激活扩散 = PPR(d=0.85, 30)；PageRank 必须转置图 | project_memory + 27 |
| C-2 | 公式库全精度（禁 toFixed）；中心性指标附人读公式；密度附解读文案 | project_memory |
| C-3 | 流程图谱：节点创建 → 边添加顺序；RAW 无向边库内双向展开 | project_memory |
| C-4 | melody2score `_PlaySession` 会话化 + `_ensure_windowed_streams` + jianpu-ly stderr 单独捕获 | project_memory |
| C-5 | AI 统一 4 端点：`/ai/engine/{process,analyze,capabilities,metrics}` | project_memory |
| C-6 | CEM AI 优化；评分 0.55Q + 0.20S + 0.10T + 0.15St；σ̄<0.06 或 3 轮无改进停止 | project_memory |
| C-7 | T10 STS AssumeRole TTL **必须** 900 秒 ±1s；硬编码不允许改 | 用户需求原文 |
| C-8 | T10 IAM Policy ≥10 条；hash_chain 审计 WORM | 用户需求原文 |
| C-9 | T11 Flink CDC ≥10 万无丢重；Spark 读写；子图 20；AC-15 ≥14 | 用户需求原文 |
| C-10 | T17 SDK 示例 = 30×3=90；测试 ≥80 | 用户需求原文 |
| C-11 | T19 全量回归 ≥706 tests；T20 灰度 = 1→10→50→100 四阶段 | 用户需求原文 |
| C-12 | 代码/项目数据分离，projects 目录独立；不得污染架构代码 | user_profile + 上次 snake.py 迁移决策 |
| C-13 | 所有新增模块必须先通过 27 §T0 18 烟测 + 不降低棘轮（22 表 8） | 27 主控 |

### 1.2 Dependencies (内部依赖)
- DEP-1：`mox-standards` (sigv4 / fips_hmac / rfc5424) → T10 STS 签名、hash_chain
- DEP-2：`mox-domain-abstractions` (IamProvider, QuotaProvider, CdcPublisher, GraphQueryProvider) → T10/T11 实现所依赖的 trait 契约（**不允许改签名**）
- DEP-3：`mox-expert::audit` (AuditContext / MultiSink / S3Sink) → T10 hash_chain 的 WORM Sink 承载
- DEP-4：`mox-graph-storage::cdc_source` → T11 Flink / Spark Connector 的事实源
- DEP-5：`platform/backend-node/src/api-server.js` + `routes/*.js` → T10 Quota 429 头注入点
- DEP-6：`platform/backend-node/test/test-enterprise-10task-t10-cloud.js` → T10 既有基线，新增测试不能使其回归
- DEP-7：Cargo workspace 30 crate 零孤儿约束（test-d5-build-workspace.js 验证）

### 1.3 Assumptions (可验证假设)
- A-1：开发环境 `cargo` / `rustc` ≥ 1.75 + `node` ≥ 18 + `python` ≥ 3.10 已可用（T17 Python SDK）
- A-2：`mocha` / `cargo test` 可运行 ≥200 并发用例不内存溢出
- A-3：T11 10 万 CDC 事件总字节 ≤ 256MB（单机 mock 可承载）；若超则自动缩至 10 万但每个 payload 缩小
- A-4：T20 Helm 脚本即使无真实 K8s，也必须 `helm template --dry-run` 通过 + 4 阶段权重变更脚本幂等可测

### 1.4 Open Questions (用户需澄清，默认值如下)
| # | 问题 | 默认值 |
|---|------|--------|
| OQ-1 | T10 冷热分层阈值：HOT / WARM / COLD 天数 | 30d / 90d / 365d（可配置） |
| OQ-2 | T17 Python SDK 打包格式：`setuptools` + `pyproject.toml` vs `poetry` | setuptools（最广兼容） |
| OQ-3 | T13 信创适配矩阵：OS/CPU/数据库 组合数量优先 6 组还是 12 组 | 6 组（麒麟/UOS × 飞腾/鲲鹏 × openGauss/达梦） |
| OQ-4 | T19 全量回归是否含 `--release` 模式 Rust 二进制 | 默认含 debug，可选 `RELEASE=1` 强制 release |

---

## 2. 功能需求（FR 1–28）

> 编号原则：`FR-{批次}-{序号}`。批次= T10/T11/T17/E/F。

### T10 云盘 M4 功能需求（FR-T10-1 ~ FR-T10-14）

- **FR-T10-1 冷热分层引擎**：`HotWarmColdLifecycle` 模块实现；对象属性 `storage_class ∈ {HOT,WARM,COLD,ARCHIVE}`；定时器每日 02:00 UTC 扫描并迁移；WARM 读触发自动回温为 HOT（写 `last_accessed_at`）；COLD → HOT 需 `restore_request`（等待 `restore_expiry_days`）
- **FR-T10-2 迁移指标可查**：`GET /cloud/lifecycle/stats` 返回 `{ per_class_counts, migrated_today_bytes, pending_restores }`
- **FR-T10-3 IAM Policy 10 条完整落地**：10 条 policy 存于 `iam::STANDARD_POLICIES` 常量，每条有 SID 文档注释；覆盖：P1 AdminFullAccess、P2 BucketOwnerFull、P3 EditorWrite、P4 ViewerReadOnly、P5 GuestListOnly、P6 PublicReadGetObject、P7 DenyAllNonMFA、P8 DenySourceIPRange、P9 ResourceTagConditional、P10 VpcEndpointRestrict
- **FR-T10-4 Policy evaluate Deny 优先语义**：显式 Deny 立即短路，即使有 Allow 覆盖；implicit deny 默认；单元测试覆盖每 policy × {匹配/不匹配/边缘前缀} 3 情况 = ≥30
- **FR-T10-5 STS AssumeRole 硬 TTL=900s**：`sts_assume_role(role_id, session_name, duration_secs=900)` 非 900 直接拒绝；`expiration = issued_at + 900`；单元测试断言 `|exp-now-900| ≤ 1`
- **FR-T10-6 session_token 可自证签名**：`session_token = HMAC-SHA256(secret, role_id + session_name + expiration)` + base64；SDK 端 `StsCredentials.verify(secret_key) → bool` 可校验
- **FR-T10-7 过期 STS 拒绝操作**：任何 IamProvider::authorize_policy 调用时若凭据含 STS 且过期 → Err("STS token expired")
- **FR-T10-8 QuotaProvider 并发安全**：`check_put_allowed + add_used` 封装为原子操作（parking_lot Mutex 或 SQLite tx）；禁止 TOCTOU 竞态
- **FR-T10-9 HTTP 429 标准响应**：Quota 超限 → `res.statusCode=429`、`Retry-After: N`、`X-Quota-Used/Limit/Reset` 头、body `{code:"QuotaExceeded",retry_after_ms:N}`；测试用并发 100 请求验证不丢 Quota 计数
- **FR-T10-10 429 语义与 TokenBucket 限流区分**：Quota = 容量（字节/对象数）超限 → 429；Rate = QPS 超限 → 429 但头为 `X-RateLimit-*`；头命名区分明确
- **FR-T10-11 等保三级 hash_chain 替换 dengbao_skeleton**：`DengbaoHashChain { genesis, blocks, verify() }`；每块 `{idx, ts_ms, actor, action, resource, outcome, payload_hash, prev_hash, block_hash, hmac_signature}`；链式 `block_hash = SHA256(prev_hash + all_fields_except_hash)`
- **FR-T10-12 WORM 写入策略**：append-only；hash_chain 写 S3Sink（object-lock=COMPLIANCE，保留期 180d）与本地 SQLite（只读表触发器，禁止 UPDATE/DELETE）
- **FR-T10-13 独立验证器**：`cargo run -p mox-standards --example verify-hash-chain <path>` 输出 `{blocks:N, integrity:true/false, broken_at:idx?}`；不破坏数据只读
- **FR-T10-14 与现有 audit 集成**：`mox_expert::audit::AuditContext` 新增 `.with_dengbao_chain(chain)` 钩子；每条 ExtAuditEvent 同步 append 到 hash_chain，写失败整体返回 error（不可旁路）

### T11 关系图 R4 功能需求（FR-T11-1 ~ FR-T11-12）

- **FR-T11-1 Flink CDC Source Rust 实现**：`FlinkCdcSource { cdc_ref }` 实现 `Iterator<Item=CdcEvent>`；支持从 `offset` 恢复；内部异步 prefetch buffer=1024；commit offset ack = subscriber 调 commit 后 CdcSource 更新 `committed`
- **FR-T11-2 10 万事件无丢重测试引擎**：生成 10 万 Vertex+Edge（比例 7:3），cdc_source.emit → FlinkSourceIterator → idempotent_writer（按 raft_index upsert）→ 统计 `{total_in, total_out, duplicates_in_upsert, lost}` → 断言 lost=0 ∧ duplicates=0 ∧ total_in=total_out=100000
- **FR-T11-3 Spark Connector Reader**：`GraphSparkReader` 读取 `GET /graph/nodes?page=&size=` 分页 → `VertexRDD[(Long, NodeProps)]` / `EdgeRDD[EdgeProps]`；分页 size=5000；分页一致性用 snapshot `graph_version`
- **FR-T11-4 Spark Connector Writer**：`GraphSparkWriter` 将 GraphX RDD → `POST /graph/bulk`（body = `{nodes,edges}`），每批 2000 项；幂等键=`source_target`
- **FR-T11-5 Spark Connector Round-trip 测试**：写入 N 节点+M 边 → 读回 → 哈希集合比对 100%（排除 UUID 漂移字段）
- **FR-T11-6 子图 Projection 算子 20 矩阵**：定义 `ProjectionFilter { type_in?, community_in?, attr_range?, degree_bucket?, label_regex? }` × `Direction {In,Out,Both}` × `Hop(u8)`；固定 5 类过滤 × 2 方向 × 2 hop = **20 具体操作符**（每个有独立函数 `proj_{filter_id}_{dir}_{hop}`）
- **FR-T11-7 Projection 算子正确性**：对生成的 200 节点测试图（10 type × 5 community × attr 连续值），每个算子手工 Oracle 期望值 vs 实际结果哈希一致
- **FR-T11-8 算子 20 组合式 API**：统一入口 `project_graph(graph, filters: Vec<ProjectionFilter>, dir, hop)`；链式可组合（filter1.and(filter2)）
- **FR-T11-9 AC-15 故障注入框架**：`FaultInjector` 支持 14 故障类，每类有 id + weight；注入点覆盖 CDC emit、Connector 读/写、Projection 遍历
- **FR-T11-10 AC-15 14 故障类枚举**：F1 network_break_peer1、F2 network_break_peer2、F3 network_partition_3way、F4 duplicate_event(2x)、F5 duplicate_event(10x)、F6 reorder_by_time_skew、F7 reorder_random_shuffle、F8 bit_flip_single_byte、F9 bit_flip_burst_16bytes、F10 disk_full_emulation、F11 oom_drop_message、F12 slow_request_5x、F13 slow_request_50x、F14 leader_kill_and_recover
- **FR-T11-11 14 故障类独立单测 + 降级质量门**：每个故障注入后 → 系统要么 100% 恢复（lost=0），要么触发熔断器且报警事件审计记录 ∈ hash_chain
- **FR-T11-12 Graph Service CDC 对外端口**：`GET /graph/cdc/stream?since_offset=&topic=` → `text/event-stream` NDJSON，暴露给 Flink/Spark/JVM 侧 connector

### T17 官方 SDK 功能需求（FR-T17-1 ~ FR-T17-14）

- **FR-T17-1 Rust SDK Cloud 实现**：`mox-sdk-cloud` 实现 `CloudClient { config, list_buckets, create_bucket, put_object, get_object, delete_object, upload_part, complete_mpu, sts_assume_role, iam_evaluate }`；SigV4 自签名（复用 mox-standards::sigv4）
- **FR-T17-2 Rust SDK Graph 实现**：`mox-sdk-graph` 实现 `GraphClient { get_graph, stats, centrality, communities, pagerank, bulk_upsert, project, cdc_stream, search }`；与 `/graph/*` 端点 1:1 对齐
- **FR-T17-3 Node.js SDK Cloud**：`platform/sdk/nodejs/mox-sdk-cloud/`（index.js + package.json），API 形状 = `class CloudClient { constructor({endpoint,accessKey,secretKey,sessionToken?}); async listBuckets(); async putObject(key,buf,opts?); ... }`；零外部 aws-sdk 依赖，使用内置 https + SigV4
- **FR-T17-4 Node.js SDK Graph**：`platform/sdk/nodejs/mox-sdk-graph/` 同构
- **FR-T17-5 Python SDK Cloud**：`platform/sdk/python/mox_sdk_cloud/`（`__init__.py` + `client.py` + `pyproject.toml`），基于标准库 `urllib` + `hmac` + `hashlib`；零第三方依赖
- **FR-T17-6 Python SDK Graph**：`platform/sdk/python/mox_sdk_graph/` 同构
- **FR-T17-7 3 语言协议一致性**：统一 `examples/*.{rs,js,py}` 用同一 8 个核心场景脚本化生成，避免 drift
- **FR-T17-8 Rust Cloud SDK 15 示例**：C1 bucket CRUD、C2 put/get small、C3 put 10MB + etag 验证、C4 并发 100 put + quorum、C5 sts_assume_role + verify sig、C6 使用 STS 凭据调用 put_object、C7 policy evaluate 10 条 × 1 case、C8 生命周期 transition 触发、C9 软删 + 恢复、C10 MPU 3 part 完成、C11 AbortMPU 资源清理、C12 Quota 429 头读取、C13 hash_chain auditor 审计列表、C14 跨桶复制（mock）、C15 签名失败 → AccessDenied 错误
- **FR-T17-9 Rust Graph SDK 15 示例**：G1 取整图、G2 取 stats、G3 3 种中心性 + 公式验证、G4 CNM 社区 + 模块度、G5 PageRank d=0.85、G6 单节点 upsert、G7 单边 upsert、G8 bulk 1000 节点 + 2000 边、G9 Projection 算子 5 条（覆盖 5 过滤维度各 1）、G10 子图 1hop/2hop 遍历、G11 CDC stream 订阅 100 event、G12 graph.search 关键词、G13 export json、G14 import json roundtrip、G15 删除节点 + 级联边检查
- **FR-T17-10 Node.js Cloud 15 + Graph 15 示例**：Rust C1–C15 / G1–G15 的 JS 同构版（文件命名完全一致：`ex_c1_bucket_crud.js` vs `ex_c1_bucket_crud.rs`）
- **FR-T17-11 Python Cloud 15 + Graph 15 示例**：同 Rust/Node 同构版（`ex_c1_bucket_crud.py` 等）
- **FR-T17-12 Rust SDK 测试 ≥30**：cloud unit ≥15 + graph unit ≥15；含 mock_server（使用 `axum::Router` 本地 0 端口启动模拟后端）
- **FR-T17-13 Node.js SDK 测试 ≥30**：cloud unit ≥15 + graph unit ≥15；使用 nock + 本地 Node http mock
- **FR-T17-14 Python SDK 测试 ≥20**：cloud unit ≥10 + graph unit ≥10；使用 `unittest` + `http.server` mock
- **FR-T17-15（硬）总计 90 示例 + 测试 ≥80**：脚本 `scripts/count_sdk_artifacts.js` 输出 `{ examples_rs: N, examples_js: N, examples_py: N, tests_rs: N, tests_js: N, tests_py: N, total_examples: N, total_tests: N }`；断言 total_examples≥90 ∧ total_tests≥80

### E 批次运维（FR-E-1 ~ FR-E-9）

- **FR-E-1 T12 Helm DR Chart**：`deploy/helm/mox-dr/` 含 Chart.yaml + values.yaml + templates/{master,volume,s3,graph-gateway}; replicaCount=3; antiAffinity=hard; 自动 failover = liveness probe 失败 → k8s 重建主
- **FR-E-2 T12 DR 恢复脚本**：`deploy/helm/dr-failover.sh {promote,demote,status}`；纯 bash 幂等；无 K8s 时返回 "dry-run: ok"
- **FR-E-3 T13 信创适配矩阵**：`docs/enterprise/28-信创适配清单-V1.0.md` 6 组合表格（OS × CPU × DB × 适配状态：已验证/待补测/N/A）
- **FR-E-4 T13 部署手册 7 章**：`docs/enterprise/29-企业级部署手册-V1.0.md`：①架构视图 ②环境准备 ③单机版 ④集群版 ⑤K8s Helm 版 ⑥运维与监控 ⑦故障排查；每章 ≥3 小节
- **FR-E-5 T15 HA rubric 量化**：SLA 目标 99.95% = 每月停机 ≤21.6 min；容量基线=百万节点/千万边/10TB 文件；测试脚本 `test-enterprise-slo-capacity-tco.js` 已存在，需增强为含 `assert sla_9995_ok`
- **FR-E-6 T15 容量 Rubric 4 档**：Tiny(0-1万/512M) / Small(1-10万/4G) / Medium(10万-1M/32G) / Large(1M-10M/256G)；每档有推荐配置表
- **FR-E-7 T15 TCO Rubric 4 档**：按 Tiny/Large 对比本地磁盘 vs MinIO vs COS，1/3/5 年 TCO；`scripts/tco-rubric.js` 生成表
- **FR-E-8 T18 8 阶段 trace**：定义 8 阶段 = P1 Accept → P2 IAM Auth → P3 Quota Check → P4 Graph Query Plan → P5 Storage IO → P6 Algorithm → P7 Audit Log → P8 Response；`trace_e2e` 中间件串联每个 span，trace_id 透传全链路；`test-three-flows-trace-e2e.js` 存在，需断言 8 span 全部存在且嵌套层级正确
- **FR-E-9 T18 8 阶段 span 可视化**：`GET /system/traces/{id}` 返回 `{ spans: [{name,dur_ms,start_ts,parent}], trace_id }`；支持 zipkin compatible

### F 批次运维（FR-F-1 ~ FR-F-7）

- **FR-F-1 T19 全量回归触发脚本**：`scripts/run-full-regression.ps1` + `scripts/run-full-regression.sh`；执行所有 `cargo test --workspace` + 所有 `mocha platform/backend-node/test/*.js` + 所有 `frontend-ui` vitest + playwright；总测试数计数器 `totalPassed` 记录到 `data/regression_last.json`
- **FR-F-2 T19 ≥706 断言**：`scripts/count_test_specs.js` 已有（`test-enterprise-10task-t2-algorithm.js` 引用），增强后输出 `{ rust_unit: N, mocha: N, vitest: N, playwright: N, total: N }`；断言 total≥706
- **FR-F-3 T20 Helm 一键 Chart**：`deploy/helm/mox/` 含完整一键部署（云盘+关图+网关+前端+minio-ha）；`helm install mox ./mox` 即 0 配置可起
- **FR-F-4 T20 Helm 可模板化测试**：`helm template --dry-run mox ./mox > /tmp/rendered.yaml` 必须成功；模板输出包含 ≥10 Deployment / ≥1 Service / ≥1 ConfigMap
- **FR-F-5 T20 灰度 4 阶段脚本**：`deploy/helm/canary-{init,step1-1pct,step2-10pct,step3-50pct,step4-100pct}.sh`（ps1 对应版本）；权重路由文件 `values-canary-{1,10,50,100}.yaml`；每阶段 sleep + `GET /healthz` 验证 ready
- **FR-F-6 T20 Warmup 验证**：每阶段 10s 预热探针 = 100 次 `/healthz` + 10 次 `POST /ai/engine/metrics`，成功率 ≥95% 才自动进入下一阶段
- **FR-F-7 T20 灰度 metrics 快照**：每个阶段结束写 `data/canary/{phase}.metrics.json`，含 p50/p95/p99 latency + error_rate + throughput；最终断言 phase4 error_rate<0.1%

---

## 3. 非功能需求（NFR 1–18）

- **NFR-1 性能**：T11 10 万 CDC 事件在 16GB 内存机器 ≤60s 完成端到端（emit→write→verify）
- **NFR-2 性能**：T17 SDK 每个示例运行 ≤5s（mock backend 场景）；90 示例总运行 ≤450s
- **NFR-3 安全**：所有密钥、AK/SK 默认从 env 读取，禁止硬编码；测试用固定假 key `AKIAIOSFODNN7EXAMPLE` / `wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY`（AWS 文档公开占位，非真 key）
- **NFR-4 安全**：hash_chain HMAC 密钥环境变量 `DENGBAO_HMAC_SECRET`；若未设置启动报错而非静默使用默认
- **NFR-5 可靠性**：T10 Quota 写入失败（db error）不得泄漏业务数据；必须返回 500 且审计记录 error
- **NFR-6 可靠性**：T11 故障注入 14 类每类至少 1 个自动重启恢复用例，无数据丢失才 PASS
- **NFR-7 可测试性**：所有新增模块至少提供 `{unit, integration, contract}` 3 层；contract=JSON Schema SoT
- **NFR-8 兼容性**：T10 file-store 向后兼容：旧 `writeFile/readFile` 接口零改动；测试 `test-enterprise-10task-t10-cloud.js` 原测试仍全绿
- **NFR-9 兼容性**：T10 IamProvider / QuotaProvider trait 签名零变更；测试 `t1_t2_t3_red_green.rs` 仍绿
- **NFR-10 可观测性**：T10/T11/T17 每模块 `tracing::info!` 埋点 ≥5；含 `{module, action, dur_ms, ok}` 标准字段
- **NFR-11 文档化**：T13 信创 + 部署手册必须真实引用所有新增模块路径；相对路径使用 `./` 指向 repo 根
- **NFR-12 国际化**：T17 SDK 错误消息中英双栏（`{message_en, message_zh}`），避免纯中文
- **NFR-13 代码质量**：Rust `cargo clippy --workspace --all-targets -- -D warnings` 0 warning；Node `eslint` 0 error；Python `ruff check` 0 error（Python 新增强制）
- **NFR-14 构建时间**：Rust 首次 clean build ≤ 20 分钟（不联网 crate 拉取失败自动跳过网络测试标记 `#[ignore]`）
- **NFR-15 幂等**：所有 E/F 脚本（Helm / 灰度 / failover / 回归）支持多次重复运行，输出不变或有 dry-run 标志
- **NFR-16 资源路径分离**：所有 T10/T11/T17 示例输出目录 → `projects/t10-cloud-artifacts/`、`projects/t11-graph-artifacts/`、`projects/t17-sdk-examples/`；架构代码 `platform/` 不写业务数据（C-12 / user_profile 硬约束）
- **NFR-17 测试棘轮**：所有新增测试必须在 27 §T0 18 烟测通过后才能被认为有效；不得低于 2026-08-24 的基线（`meta_latest.json`）
- **NFR-18 CLI 响应式**：长任务（T19 706+测试 / T11 10 万 CDC）必须有实时进度条（Node `progress` / Rust `indicatif`）；不允许黑屏无输出

---

## 4. 验收标准（Acceptance Criteria = AC）

> 严格使用 **rule** 或 **rubric** 二分类。

### 4.1 T10 云盘 M4 AC（AC-T10-1 ~ AC-T10-24，其中 rule ≥ 20，rubric ≥ 4）

- **[rule] AC-T10-1**：运行 `cargo test -p mox-cloud-drive-s3 --test t6_m2_s3_service lifecycle_hot_warm_cold` → exit 0；断言 ≥3 子用例（HOT→WARM 自动迁移、WARM 读回温、COLD restore）全部通过
- **[rule] AC-T10-2**：运行 `mocha platform/backend-node/test/test-t10-m4-lifecycle.js`（新增）→ 12/12 绿；覆盖写入、迁移、回温、restore 路径
- **[rule] AC-T10-3**：`IamProvider::authorize_policy` 对 10 条标准 policy × 3 场景 = 30 子断言，执行 `cargo test -p mox-domain-abstractions --test t1_t2_t3_red_green iam_10_policies` → 30/30 通过
- **[rule] AC-T10-4**：Deny 优先语义验证：policy（Allow * + Deny s3:DeleteBucket）+ 操作 s3:DeleteBucket → result=false；脚本化断言 10 条 DenyOverride 均正确
- **[rule] AC-T10-5**：`sts_assume_role` duration != 900 → 返回 Err；duration=900 → credentials.expiration - now_ms ∈ [899_000, 901_000] ms
- **[rule] AC-T10-6**：`StsCredentials::verify(&secret) == true` 对刚签发 token；篡改 `session_token` 最后 1 字节 → verify == false
- **[rule] AC-T10-7**：`sts_assume_role` 后 thread::sleep(1s) 模拟 15min+ 通过篡改 expiration，authorize_policy → Err("STS token expired")
- **[rule] AC-T10-8**：并发 200 任务同时 `check_put_allowed + commit_used_bytes`，最终字节计数无偏差（Δ=0）；用 `loom` 或 Tokio 并发测试
- **[rule] AC-T10-9**：写 API 超限 → `res.statusCode === 429`；头含 `retry-after`、`x-quota-used`、`x-quota-limit`、`x-quota-reset`
- **[rule] AC-T10-10**：Quota 超限 vs Rate 超限头区分正确（X-Quota-* vs X-RateLimit-*），各自 body code 不同
- **[rule] AC-T10-11**：`mox-standards::dengbao` 不再是 skeleton，存在 `HashChain::append` / `verify` / `iter` 三个 pub fn
- **[rule] AC-T10-12**：生成 N=1000 hash chain 块 → verify 返回 true；随机篡改第 500 块 1 字节 → verify 返回 false，且 `broken_at` 字段 = 500
- **[rule] AC-T10-13**：`cargo run -p mox-standards --example verify-hash-chain <chain.json>` 输出 JSON 含 `integrity: true/false`；exit code 0 仅当 integrity=true
- **[rule] AC-T10-14**：SQLite 审计表 `audit_chain` 有触发器禁止 UPDATE/DELETE；尝试 `DELETE FROM audit_chain` → SQL error；写入只能 INSERT
- **[rule] AC-T10-15**：AuditContext 开启 dengbao_chain 后，append 100 审计项 → dengbao_chain.len() == 100；AuditContext 写入失败（如 S3 mock err）→ 整体返回 Err，无旁路写
- **[rule] AC-T10-16**：既有 `test-enterprise-10task-t10-cloud.js` 运行 → 原 baseline 用例仍 全绿（回归 0 退化，NFR-8）
- **[rule] AC-T10-17**：既有 `t1_t2_t3_red_green.rs` → 0 red（NFR-9）
- **[rule] AC-T10-18**：T10 相关 `projects/t10-cloud-artifacts/` 目录存在，含 ≥5 子产物 JSON；架构代码 `platform/` 不写业务数据（NFR-16）
- **[rule] AC-T10-19**：`cargo clippy -p mox-standards -p mox-cloud-drive-s3 -p mox-domain-abstractions -p mox-expert -- -D warnings` → 0 warning（NFR-13）
- **[rule] AC-T10-20**：`GET /cloud/hash_chain/stats` 返回 `{blocks:N, last_block_ts, integrity:true}`；与内部链字段一致
- **[rubric, scale 0-4, pass≥3] AC-T10-21 IAM Policy 覆盖率质量**：0=仅存 2 条且文档缺失，1=6 条但无 Condition，2=9 条含 Condition 但无单元注释，3=10 条齐全每条有 SID + 中文业务注释 + DENY 可审计，4=10 条 + 额外 ≥2 自定义 policy 样例 + 覆盖率矩阵文档。**threshold ≥3**
- **[rubric, scale 0-4, pass≥3] AC-T10-22 等保合规证据完整性**：0=无 hash_chain，1=仅内存实现不持久，2=持久化但非 WORM，3=WORM+SQLite 触发器+独立验证器通过 N=1000，4=3+S3 Object Lock 合规 + GB/T 22239 对标说明 ≥1 页。**threshold ≥3**
- **[rubric, scale 0-5, pass≥4] AC-T10-23 Quota/Rate 响应标准化度**：5 分维度各 1（HTTP code、Retry-After、X-Quota-* 头、body code 规范、区分头 vs RateLimit），缺 1 扣 1 分。**threshold ≥4**
- **[rubric, scale 0-2, pass≥2] AC-T10-24 测试数量≥60**：计数脚本输出 T10 相关 `rust + mocha` 用例数 N；score 映射：N<40→0, 40≤N<60→1, N≥60→2。**必须=2**

### 4.2 T11 关系图 R4 AC（AC-T11-1 ~ AC-T11-18）

- **[rule] AC-T11-1**：`FlinkCdcSource` 实现存在（Rust 新模块 `mox-graph-streams::flink_source.rs`），含 `new(Arc<CdcSource>) → Self; fn resume(offset) → Result<(),_>; fn next_blocking() -> Option<CdcEvent>`
- **[rule] AC-T11-2**：10 万事件测试：N_VERT=70_000, N_EDGE=30_000 → emit → source.iter → idemp_writer → final_count == 100_000 ∧ lost==0 ∧ duplicate_in_upsert==0
- **[rule] AC-T11-3**：`GraphSparkReader` / `GraphSparkWriter` 模块存在，含 DataFrame schema（NodeFrame / EdgeFrame）；schema 字段有 `#[pyclass]`/`#[derive(ArrowField)]` 等价元数据
- **[rule] AC-T11-4**：Spark roundtrip：write(2000 nodes, 3000 edges) → read back → symmetric_difference(node id set) 为空；edge 的 `(source,target,label)` tuple 集相同
- **[rule] AC-T11-5**：20 Projection 算子注册表存在 `PROJECTION_OPERATORS: [(&str, ProjectionFn); 20]`；每个算子函数指针命名形如 `proj_type_in_in_1hop`
- **[rule] AC-T11-6**：对 200 节点测试图，20 算子 × oracle 结果集哈希对比；20/20 完全匹配
- **[rule] AC-T11-7**：`project_graph` 组合链式：`f1.and(f2).and(f3)(graph, Both, 2hop)` 节点数 = 三交集大小正确
- **[rule] AC-T11-8**：`FaultInjector::all_14()` 返回 [Fault;14]，ID 与 FR-T11-10 一 一对应，无缺无重复
- **[rule] AC-T11-9**：运行故障注入 14 类 × 3 次 run = 42 场景；每类最终 lost==0 或 circuit_breaker_trip==true 且 audit_event ∈ hash_chain；42/42 PASS
- **[rule] AC-T11-10**：`GET /graph/cdc/stream?since_offset=0` 响应 Content-Type=text/event-stream；NDJSON 每行可 parse；10 秒内至少推送 1 event（如存在）
- **[rule] AC-T11-11**：`cargo test -p mox-graph-storage -p mox-graph-service --test t7_r2_storage t9_r3_graph_service` → 0 失败
- **[rule] AC-T11-12**：`projects/t11-graph-artifacts/` 目录存在，含 cdc_100k_report.json、projection_20_matrix.json、fault_14_report.json
- **[rule] AC-T11-13**：既有 graph 基线（routes/graph.js 的公式/中心性/社区/PageRank）`mocha test/test-graph-formulas.js test/test-enterprise-10task-t2-algorithm.js` → 全绿（0 回归）
- **[rubric, scale 0-2, pass≥2] AC-T11-14 测试≥40**：T11 相关测试 rust+mocha 计数 N；N<25→0, 25≤N<40→1, N≥40→2。**必须=2**
- **[rubric, scale 0-5, pass≥4] AC-T11-15 10 万无丢重可靠度**：丢=0+重=0=5；丢<10 或重<5=3；丢>100=0
- **[rubric, scale 0-4, pass≥3] AC-T11-16 Projection 20 算子完备度**：每个算子存在=0.2 分（20×0.2=4 满）；少于 18 个 = <3.6 → 不达标；需 ≥18 算子存在且各自单测通过 → score≥3.6≥3
- **[rubric, scale 0-4, pass≥3] AC-T11-17 Spark 接口完备度**：0=无实现；1=Reader 无 Writer；2=Reader+Writer 无分页；3=Reader+Writer+分页+幂等键；4=3+roundtrip 全绿 + Schema 文档。**threshold≥3**
- **[rubric, scale 0-3, pass≥3] AC-T11-18 故障注入覆盖度**：14 类各 1 分满 14，14≥13 → score=3；10-12=2；7-9=1；<7=0。**必须=3**

### 4.3 T17 官方 SDK AC（AC-T17-1 ~ AC-T17-18）

- **[rule] AC-T17-1**：Rust SDK 两 crate 的 `lib.rs` 不再是 1 行 stub；存在 `CloudClient::new(Config) -> Self` 和 `GraphClient::new(Config) -> Self`，每 crate pub fn ≥8
- **[rule] AC-T17-2**：Node.js SDK 两目录存在：`platform/sdk/nodejs/mox-sdk-cloud/package.json` + `mox-sdk-graph/package.json`；`exports.main` 指向 `index.js`
- **[rule] AC-T17-3**：Python SDK 两目录存在：`platform/sdk/python/mox_sdk_cloud/pyproject.toml`（setuptools）+ `mox_sdk_graph/pyproject.toml`；`python -c "import mox_sdk_cloud as c; print(c.__version__)"` 成功
- **[rule] AC-T17-4**：Rust Cloud SDK 15 示例文件存在（`examples/ex_c1.rs` 到 `examples/ex_c15.rs` + `examples/ex_g1.rs` 到 `examples/ex_g15.rs`）；`cargo test -p mox-sdk-cloud --examples` 编译通过
- **[rule] AC-T17-5**：Node.js SDK 30 示例存在：`platform/sdk/nodejs/examples/{cloud,graph}/ex_c{1-15}.js` 与 `ex_g{1-15}.js`
- **[rule] AC-T17-6**：Python SDK 30 示例存在：`platform/sdk/python/examples/{cloud,graph}/ex_c{1-15}.py` 与 `ex_g{1-15}.py`
- **[rule] AC-T17-7**：`scripts/count_sdk_artifacts.js` → `total_examples >= 90`
- **[rule] AC-T17-8**：Rust SDK 测试 `cargo test -p mox-sdk-cloud -p mox-sdk-graph` → passed ≥ 30
- **[rule] AC-T17-9**：Node.js SDK 测试 `mocha platform/sdk/nodejs/test/**/*.js` → passed ≥ 30
- **[rule] AC-T17-10**：Python SDK 测试 `python -m unittest discover -s platform/sdk/python/test` → passed ≥ 20
- **[rule] AC-T17-11**：3 语言协议一致性：3×3 语言 × 示例（C1,C5,G1,G8,G15 各一条）输出 JSON schema 字段一致（忽略 lang 专属字段）
- **[rule] AC-T17-12**：Rust SDK mock_server 使用本地 axum 随机端口；测试用例不访问外网（`CARGO_NET_OFFLINE=1` 下通过 ≥ 25 个）
- **[rule] AC-T17-13**：Node SDK mock 使用 `nock` 离线；测试不触网（`node test-offline-all.js` 全部 PASS）
- **[rule] AC-T17-14**：Python SDK mock 走 `unittest.mock` / 本地 http.server；不触网
- **[rule] AC-T17-15**：项目数据目录 `projects/t17-sdk-examples/` 存在；含三语言运行输出 JSON（每语言 ≥5 份）；`platform/sdk/` 内不生成业务运行数据
- **[rubric, scale 0-2, pass≥2] AC-T17-16 总测试数 ≥80**：rust + node + py 实测通过数 N；N<60→0, 60≤N<80→1, N≥80→2。**必须=2**
- **[rubric, scale 0-4, pass≥3] AC-T17-17 三语言同构一致性**：0=缺 1 语言；1=3 语言但 API 形状差距大；2=3 语言示例命名一致；3=3 语言字段级 schema 90% 匹配；4=3+共享 example spec JSON（script 生成）避免 drift
- **[rubric, scale 0-4, pass≥3] AC-T17-18 SDK 文档健全度**：README.md 6 要素（安装/快速开始/配置/示例链接/错误码/FAQ）每要素 0.6 分满 4；缺 2 要素以上（<2.4）FAIL

### 4.4 E 批次运维 AC（AC-E-1 ~ AC-E-10）

- **[rule] AC-E-1**：T12 Helm DR `deploy/helm/mox-dr/Chart.yaml` + `values.yaml` + 至少 4 个模板存在；`helm lint ./deploy/helm/mox-dr`（无 helm 则模板 yaml 解析通过）→ exit 0
- **[rule] AC-E-2**：`dr-failover.sh status` bash 脚本语法正确（`bash -n`）；ps1 对应 `powershell -NoProfile -Command "& { Test-Path deploy/helm/dr-failover.ps1; $ast = [System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path deploy/helm/dr-failover.ps1),[ref]$null,[ref]$null); $ast.Errors.Count -eq 0 }"` → True
- **[rule] AC-E-3**：`docs/enterprise/28-信创适配清单-V1.0.md` 存在；6 适配组合表格完整（≥6 行 × 4 列）；含已验证/待补测/N/A 标注
- **[rule] AC-E-4**：`docs/enterprise/29-企业级部署手册-V1.0.md` 存在；7 章齐全，每章 ≥3 小节；≥2 张 ASCII / Mermaid 架构图
- **[rule] AC-E-5**：T15 HA：`mocha test-enterprise-slo-capacity-tco.js`（已存在）含 `sla_9995_ok` 断言 → true
- **[rule] AC-E-6**：容量 Rubric 4 档表存在；`scripts/tco-rubric.js gen` → 输出 4 × 3 表矩阵 JSON
- **[rule] AC-E-7**：T18 8 阶段 trace：`mocha test-three-flows-trace-e2e.js`（已存在）断言 8 span 全部存在：Accept / IAM Auth / Quota Check / Query Plan / Storage IO / Algorithm / Audit Log / Response；嵌套层级 parent_id 链路正确
- **[rule] AC-E-8**：`GET /system/traces/{id}` 返回 JSON `spans` 数组长度 ≥8；字段齐全
- **[rule] AC-E-9**：8 阶段每阶段有 ≥1 条 `tracing` 埋点在代码里（grep 证明）
- **[rubric, scale 0-3, pass≥2] AC-E-10 文档与脚本质量**：0=缺；1=脚本语法但不幂等；2=幂等+dry-run；3=幂等+dry-run+CI hooks。**threshold≥2**

### 4.5 F 批次运维 AC（AC-F-1 ~ AC-F-10）

- **[rule] AC-F-1**：T19 全量回归脚本存在 `scripts/run-full-regression.{ps1,sh}`；bash -n / AST parse 通过
- **[rule] AC-F-2**：`scripts/count_test_specs.js` 运行并返回 `{total: N}`；N ≥ 706（棘轮不得低于 2026-08-24 的基线 649+，且本次新增数百）
- **[rule] AC-F-3**：T20 Helm 一键 Chart 存在 `deploy/helm/mox/Chart.yaml`；模板渲染输出 YAML ≥ 10 Deployment / ≥ 1 Service / ≥ 1 ConfigMap
- **[rule] AC-F-4**：T20 灰度 4 阶段脚本存在：`canary-step1-1pct` / `step2-10pct` / `step3-50pct` / `step4-100pct`；对应 values 文件 4 个存在；权重数值正确（1/10/50/100）
- **[rule] AC-F-5**：每阶段 warmup 逻辑存在：10 秒内 ≥100 /healthz + ≥10 /ai/engine/metrics 请求；success 率 ≥ 95% 自动进入下一阶段
- **[rule] AC-F-6**：每阶段 metrics 快照写入 `projects/t20-canary-metrics/{phase}.json`（projects 目录！=架构代码，NFR-16）；字段含 p50/p95/p99/error_rate/throughput
- **[rule] AC-F-7**：最终断言 phase4 error_rate < 0.001（0.1%）；脚本显式断言此阈值
- **[rule] AC-F-8**：`helm template --dry-run mox ./deploy/helm/mox > /tmp/mox-render.yaml` exit 0（有 helm 环境时）；无 helm 时 `yq eval-all` 模板语法检查通过
- **[rule] AC-F-9**：T19 全量回归输出 `projects/t19-regression-report/last_summary.json` 含 `{ rust_passed, mocha_passed, frontend_passed, total_passed, total_failed, ran_at }`
- **[rubric, scale 0-4, pass≥3] AC-F-10 灰度与全量回归可观察性**：0=无进度条/无 metrics；1=有计数器；2=有进度条+JSON 报告；3=2 + 每阶段 console summary；4=3 + 输出 HTML 报告。**threshold≥3**

---

## 5. 交付物 Manifest（交付物清单）

| 交付物 ID | 路径 | 说明 |
|-----------|------|------|
| D-T10-LC | `platform/services/mox-cloud-drive-s3/src/lifecycle.rs`（新） | 冷热分层引擎 |
| D-T10-IAM10 | `platform/services/mox-domain-abstractions/src/iam_standard_policies.rs`（新） | IAM 10 条标准 policy 常量 + evaluate |
| D-T10-STS | `platform/services/mox-domain-abstractions/src/sts_ttl900.rs`（新） | STS TTL=900 硬约束 + verify |
| D-T10-QUO | `platform/backend-node/src/middleware/quota429.js`（新） | HTTP Quota 429 头中间件 |
| D-T10-HC | `platform/services/mox-standards/src/dengbao_hash_chain.rs`（替换 dengbao_skeleton） | 等保三级 hash chain + WORM |
| D-T10-TEST | `platform/backend-node/test/test-t10-m4-{lifecycle,iam10,sts,quota429,dengbao}.js` ×5 新 | T10 ≥60 测试主体 |
| D-T11-CDC100K | `platform/services/mox-graph-streams/`（新 crate，workspace Cargo.toml 注册） | Flink Source + 10 万 harness |
| D-T11-SPARK | `platform/services/mox-graph-spark/`（新 crate） | Spark Connector Reader/Writer |
| D-T11-PROJ | `platform/services/mox-graph-service/src/projection_20.rs`（新） | 子图 Projection 20 算子 |
| D-T11-AC15 | `platform/services/mox-graph-service/src/ac15_faults.rs`（新） | AC-15 14 故障注入 |
| D-T11-TEST | `platform/backend-node/test/test-t11-r4-{cdc100k,spark,proj20,ac15}.js` ×4 新 | T11 ≥40 测试主体 |
| D-T17-RUST | `platform/sdk/rust/mox-sdk-cloud/src/lib.rs` + `mox-sdk-graph/src/lib.rs`（重写）+ `examples/ex_{c,g}{1-15}.rs` ×30 | Rust SDK + 30 示例 |
| D-T17-NODE | `platform/sdk/nodejs/{mox-sdk-cloud,mox-sdk-graph,test,examples}`（新目录） | Node SDK + 30 示例 |
| D-T17-PY | `platform/sdk/python/{mox_sdk_cloud,mox_sdk_graph,test,examples}`（新目录） | Python SDK + 30 示例 |
| D-T17-COUNT | `platform/backend-node/scripts/count_sdk_artifacts.js`（新） | 90 示例 / 80 测试计数器 |
| D-E-T12 | `deploy/helm/mox-dr/{Chart.yaml,values.yaml,templates/*,scripts/dr-failover.*}` | T12 Helm DR |
| D-E-T13 | `docs/enterprise/28-信创适配清单-V1.0.md` + `29-企业级部署手册-V1.0.md` | T13 信创 + 手册 |
| D-E-T15 | `scripts/tco-rubric.js` + test 增强 | T15 HA/容量/TCO |
| D-E-T18 | `platform/backend-node/src/middleware/trace_8phase.js`（新） | T18 8 阶段 trace 中间件 |
| D-F-T19 | `scripts/run-full-regression.{ps1,sh}` + `scripts/count_test_specs.js` 增强 | T19 全量回归 ≥706 |
| D-F-T20 | `deploy/helm/mox/` + `deploy/helm/canary-{step1-4}.*` + `values-canary-*.yaml` | Helm 一键 + 4 阶段灰度 |
| D-DATA | `projects/t10-cloud-artifacts/`, `t11-graph-artifacts/`, `t17-sdk-examples/`, `t19-regression-report/`, `t20-canary-metrics/`（新） | 项目数据目录，不污染架构 |

---

## 6. 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| T11 10 万 CDC 内存 OOM | 中 | 高 | payload_json 控制在 ≤1KB；启用 `pending.flush()` 批大小上限；测试若单机不足则 `#[cfg_attr(skip_100k_ci, ignore)]` 但保留脚本手动触发 |
| T17 Python/Node SDK 与 Rust 协议 drift | 中 | 高 | 单一源 `platform/sdk/specs/example_specs.json`（8 场景 1 JSON SoT）；脚本自动 tri-generate；CI 一致性 AC-T17-11 失败阻断合并 |
| T19 706 基线不足 | 低 | 中 | 既有 649+ baseline + T10(≥60)+T11(≥40)+T17(≥80) 合计 649+180=829>706；脚本 `count_test_specs.js` 增强统计更全 |
| 无 helm 环境导致 T12/T20 无法真实验证 | 高 | 中 | 双轨：有 helm → `helm lint/template`；无 helm → YAML AST parse + 模板变量模拟渲染；都写脚本兼容 |
| cargo clippy warning 超 0 | 中 | 高 | 所有新增 Rust 代码启用 `#![warn(clippy::all, clippy::pedantic)]` 但 `#![allow(clippy::too_many_lines, clippy::large_enum_variant)]` 仅对合理项；CI 阶段 `clippy -D warnings` 必过 |

---

## 7. Schedule & Milestone（建议 5 个 Slicing Batch，对齐用户批次 A/B/C/E/F）

| Batch | 内容 | AC 验收门 | 里程碑 |
|-------|------|-----------|--------|
| A (T10) | 云盘 M4：冷热分层+IAM10+STS 900s+Quota429+hash_chain | AC-T10-1..24 全通过；tests≥60 | M1 |
| B (T11) | 关系图 R4：Flink CDC 10 万 + Spark R/W + Projection 20 + AC-15 14 | AC-T11-1..18 全通过；tests≥40 | M2 |
| C (T17) | 3 语言 SDK × 90 示例 + ≥80 tests | AC-T17-1..18 全通过；tests≥80；examples=90 | M3 |
| D (Review 1) | 独立审查 A/B/C 批次 artifacts + evidence | Review pass（修复需重测） | M3.5 |
| E (E 批次) | T12/T13 → T15 → T18 全维串联 | AC-E-1..10 全通过 | M4 |
| F (F 批次) | T19(≥706) + T20(灰度 1→10→50→100) 闭环 | AC-F-1..10 + Review Final pass | M5 RELEASE |

> **最终放行条件**（27 §T6 映射）：T0=18 烟测全过；7×8 算法对账 全绿；棘轮不退化；四闸门 G1/G2/G3/G4 全过；RELEASE_L2_PASS 签字 5 位齐全。若任一未满足 → REJECT 并开 remediation 队列。
