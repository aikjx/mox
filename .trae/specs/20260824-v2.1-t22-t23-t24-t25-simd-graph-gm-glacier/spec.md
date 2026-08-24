# XUANJI v2.1 规格 — T22 SIMD EC / T23 Graph Projection20 / T24 国密 PKI SMx / T25 Glacier 冷层
> 规格版本: v2.1-SPEC-20260824  
> 对应自然需求: "以最规范标准完成 v2.1 规划建议 (T22~T25 三个月切片)"

---

## 1. 背景与劣势基线（现状 = 必须超越的 baseline）

在 v2.0 验收通过 (40/40 单二进制部署验证, Grade S) 基础上，识别出 4 项已被量化的性能/能力/合规/生态缺口，构成 v2.1 立项基线：

| # | 劣势 | 量化影响 | 根因 | 现实现状（代码锚点） |
|---|---|---|---|---|
| G1 | SIMD/ASM EC 加速缺失 | NVMe+万兆 Intel 平台吞吐损失 **15~25%**（NVMe 顺序写≥3.5GB/s 时瓶颈落在 GF(2^8) 表查乘法，L1 带宽 1 B/cycle） | `gf_mul()` 基于 [reed_solomon.rs:L71-L79](file:///d:/a10/aikjx/gitcode/infotopograph/platform/services/xuanji-cloud-drive-volume/src/reed_solomon.rs#L71-L79) 的 log/exp 表查 + 串行 encode/decode | 当前 12+4 profile 在 TigerLake i7-1165G7 上 encode 实测 ≈ 2.0 GB/s，离目标 2.7 GB/s 差 +35% |
| G2 | GraphQL 反向查询只支持 by-tag 单条件 | 政企关联检索、社区画像、6 度人脉、证据链溯源等 20 类常见业务不可用 | `http_server.rs` 只暴露 2 个图端点：`/graph/stats` 和 `/graph/query_by_tag`；未对接 `projection_20.rs` 20 子图算子 | 现存 20 算子见 [projection_20.rs:L187-L230](file:///d:/a10/aikjx/gitcode/infotopograph/platform/services/xuanji-graph-service/src/projection_20.rs#L187-L230)（proj_{type,community,attr,degree,label}_{in,out}_{1,2}），但 GraphWriter 的 `tag://` 图顶点 id 是 `String` 型 (`s3://bucket/key`)，而 `projection_20` 顶点 id 是 `i64`，类型桥接与 HTTP 端点未做 |
| G3 | 真 K8s 3 主 3 从 + 国密 PKI 未落地 | 政企业务准入不通过（等保三级/密评强制要求国密 SM2/SM3/SM4；真集群高可用未验证） | `HashChain` 使用 HMAC-SHA256 + Sha256（[dengbao_hash_chain.rs:L10-L14](file:///d:/a10/aikjx/gitcode/infotopograph/platform/services/xuanji-standards/src/dengbao_hash_chain.rs#L10-L14)）；STS 会话令牌签名基于 HMAC-SHA256（[sts_ttl900.rs:L10-L14](file:///d:/a10/aikjx/gitcode/infotopograph/platform/services/xuanji-domain-abstractions/src/sts_ttl900.rs#L10-L14)）；Helm Chart 只写未在真 K8s Kind/K3s 集群跑过 | 目前仅本地 `cargo test` + Mocha 验证过 STS / hash_chain / lifecycle 单元；未验证：3 主 3 从 raft 选主 + 脑裂恢复、国密互操作测试向量、SM4-GCM 对象级落盘加密、TLS 握手 SM2 证书链 |
| G4 | AWS S3 Glacier Deep Archive（>1 年归档）未接入 | 合规 7~30 年长期归档场景需外部对接 3rd 归档系统；元数据/对象生命周期 4 态 HOT/WARM/COLD→GLACIER 断档 | `StorageClass` 枚举仅 3 态 (HOT/WARM/COLD) 见 [lifecycle.rs:L19-L25](file:///d:/a10/aikjx/gitcode/infotopograph/platform/services/xuanji-cloud-drive-s3/src/lifecycle.rs#L19-L25)；未提供 Glacier S3 兼容后端 Adapter、Restore 作业、取回延迟（标准/批量/加急）建模 | Lifecycle 现有 `transition_scan` 只支持 H→W→C，`ColdRestoreToHot` 未建模归档取出小时级延迟与任务状态机 |

> **反模式禁令**：本规格严格继承 v2.0 架构约束——代码与项目数据路径严格分离、projects/ 存放全部产物与 artifacts；任何模块**不得引入开源通用脚手架**替代自研核心（保留 Rust crates.io 纯库依赖但不得改 AIS 级架构）。

---

## 2. 目标、用户、非目标

### 2.1 目标 (Objectives)
1. **T22 SIMD EC 加速**：在 x86_64 AVX2 / aarch64 NEON 上对 GF(2^8) 乘法进行 16/32 lane SIMD 化，Reed-Solomon 12+4 encode 吞吐相对表查 baseline 提升 ≥**+35%**，并保留 100% 位兼容（bit-identical parity shards，与 scalar 版本逐字节比对一致）。
2. **T23 Graph×Projection20 打通**：`fusion` 查询层把 `GraphWriter` String-id 顶点/边图映射到 `projection_20.rs` i64-id `SimpleGraph`，提供统一 HTTP/GraphQL 端点，**全部 20 算子 + 最短路径 (BFS unweighted) + 社区归属（CNM 模块度归属复用 GraphWriter.community）在 HTTP 层 1 次 roundtrip 可调用**，并 2-hop 查询返回对象元数据（ETag/CRC/size）可用于取证。
3. **T24 真集群 3 主 3 从 PKI + 国密 SMx**：
   - 审计链哈希由 SHA-256 **切换为 SM3**（GM/T 0004-2012），HMAC 算法同步替换为 HMAC-SM3（可选与 SHA-256 链并存通过 feature flag 做双写迁移）；
   - STS AssumeRole 会话令牌签名：**HMAC-SM3** 对称签发 + **SM2（GM/T 0003-2012）非对称自证签名**（角色持有 SM2 key pair，签发时用角色 sk 签名，验证时用 pk 校验）；
   - 对象落盘加密：**SM4-GCM（GM/T 0002-2012）** 对象级 envelope DEK 由 SM2-KMS 密钥包装；
   - 真集群 3 主 3 从：在 Kind/K3s 本地真集群通过 Helm Chart（deploy/helm/xuanji）部署 3 主 + 3 从 Pod，验证 leader 选举、读从写主、滚动升级 PodDisruptionBudget、30s 内脑裂自愈（杀死主节点 → 新 leader 当选 ＜ 30s）。
4. **T25 冷层 Glacier 对接**：StorageClass 枚举扩展 1 档 `Glacier`（>1 年归档），与 `HotWarmColdLifecycle` 引擎对接 COLD→GLACIER 迁移，提供 Glacier S3 兼容后端 HttpAdapter（签名 v4 + RESTORE/HEAD/GET 封装），Restore 任务支持 `Expedited/Standard/Bulk` 三档取回时间 SLA 并支持异步任务状态机。

### 2.2 用户 & 业务场景
- **政企/金融合规**（T24/T25 强相关）：等保三级、密评 GM/T 要求、审计日志 7 年不可篡改、归档 30 年、冷层取回符合司法调证窗口。
- **大数据/高性能对象存储**（T22 强相关）：NVMe 10 盘 + 100GbE RoCEv2 RDMA 场景下 EC 不成为 IO 瓶颈。
- **公安/情报关联分析**（T23 强相关）：标签→人→单位→案件的 2 跳取证、社区聚类、关键人物度数筛选。
- **混合云归档**（T25 强相关）：本地 HOT/WARM/COLD + 公有云 Glacier Deep Archive 作为 >1 年归档末端，通过 S3 兼容 API 透明切换。

### 2.3 非目标 (Non-Goals)
- 不实现 RDMA verbs 层、NVMe-oF 分布式盘阵（T22 仅 SIMD 化 encode/decode hot path，不引入新协议）。
- 不替换单二进制 `xuanji-server` 的 Tokio TCP listener 为 Hyper/Axum（保持自研 HTTP）。
- 不在此版本引入多租户配额控制（v2.0 已交付 Quota 429 sliding-window middleware）。
- 不重写 GraphWriter 核心持久化（只加 projection 桥接层）。
- 不在 v2.1 接入国产物理加密机（仅软件 SMx，HSM 接口保留位定义但不落地）。

---

## 3. 功能需求 (Functional Requirements, FR)
> 每条 FR 编号 `FR-T22.x / FR-T23.x / FR-T24.x / FR-T25.x`，随后用 `rule` / `rubric` 验收。

### 3.1 T22 SIMD EC 加速
- **FR-T22.1 SIMD 架构切换**：新增 `xuanji-cloud-drive-volume/src/gf256_simd.rs` 模块，暴露：
  - `gf_vec_mul_avx2(coef: u8, src: &[u8], dst: &mut [u8])` — x86_64 AVX2 32 lane 并行 GF(2^8) 乘法，`#[target_feature(enable = "avx2")]`，runtime CPUID 回退到 NEON/scalar；
  - `gf_vec_mul_neon(coef: u8, src: &[u8], dst: &mut [u8])` — aarch64 NEON 16 lane 并行；
  - `encode_avx2(matrix: &Matrix, data_shards: &[&[u8]], parity: &mut [Vec<u8>])` — 32 byte unroll，系数非 0 时走 SIMD；
  - `decode_avx2` — 同策略；
  - `is_avx2_supported() / is_neon_supported()` — runtime feature detect。
- **FR-T22.2 位兼容验证**：任意 (n,k) profile、任意数据字节序列在 SIMD 与 scalar 路径产出的 parity shards / reconstructed data shards **逐字节位相同**（bit-identical）。
- **FR-T22.3 自动调度**：`ReedSolomonEngine::encode/decode` 在检测到 AVX2/NEON 时自动走 SIMD 路径，对上层 API 0 调用变更；`cargo test` 默认跑 scalar 版（CI 无需特殊 CPU）；`--features simd` 打开 SIMD 目标编译并强制走 SIMD 分支测试。
- **FR-T22.4 吞吐度量 hooks**：新增 `/metrics` 指标 `xuanji_ec_encode_avx2_bytes_total / xuanji_ec_encode_scalar_bytes_total / xuanji_ec_simd_enabled`，供 Prometheus 量化 SIMD 覆盖率。

### 3.2 T23 Graph×Projection20 打通
- **FR-T23.1 String→i64 顶点映射桥**：在 `xuanji-fusion/src/graph_projection_bridge.rs`（新文件）建立双向 bijective `StringId ↔ i64` 映射表（自增 i64 分配 + BTreeMap 双向查），当 `GraphWriter` 插入新顶点/边时投影同步到 `SimpleGraph`（通过 `GraphWriter` 新增 hook `fn on_after_upsert(...)`）。
- **FR-T23.2 端点扩展**（全部挂入 `xuanji-server` HTTP public port）：
  - `GET  /graph/projection/list` → 返回 20 算子清单（id/filter/dir/hops）；
  - `POST /graph/projection/apply` body `{ seed_s: "s3://bucket/key" | "tag://project" | id_i64, operator_id, param }` → 返回 `ProjectionResult + 顶点属性映射（ObjectMeta/TagMeta）`；
  - `POST /graph/path/shortest` body `{ from_s, to_s, max_hops: u8 }` → 返回 BFS unweighted 最短路径边序列 + 跳数；
  - `GET  /graph/communities` → 返回（community_id → [vertex_string_ids]）+ 模块度 score。
- **FR-T23.3 2-hop 取证完整元数据**：所有 projection 返回中的对象顶点 (s3://) 附 **ETag + size_bytes + crc64_ecma + miji_level + hold_until_ms**，标签顶点 (tag://) 附 **tag_key + tag_value**，确保反向取证不需要二次回源（"1 次 roundtrip 完整取证"）。
- **FR-T23.4 属性桥**：GraphWriter 顶点 `type/label/attr` 同步到 `SimpleGraph`：对象顶点 type=`object`、label=URI 后 32 byte、attr={bucket,key,size,etag,crc,miji,hold}；标签顶点 type=`tag`、label=tag_key、attr={tag_key,tag_value,usage_count}。
- **FR-T23.5 社区归属**：对 GraphWriter 对象/标签图运行一次 CNM 凝聚社区检测（从 projection_20 的 `vertex.community` 字段暴露），新增 `POST /graph/community/detect` 触发重算（幂等）。

### 3.3 T24 真集群 3 主 3 从 PKI + 国密 SM2/SM3/SM4
- **FR-T24.1 SM3 哈希链切换 (feature flag)**：新增 `xuanji-standards/src/sm3_hash.rs` + `sm3_hex(bytes) -> String`（GM/T 0004-2012，256 bit）；`HashChain` 在 feature `gm-sm` 下使用 `SM3` 作为 block_hash 算法、使用 `HMAC-SM3` 作为 `hmac_signature`；默认 feature 保留 SHA-256 以兼容 v2.0 链；新增 `dual_chain` feature 同时写两条链（迁移窗口）。
- **FR-T24.2 SM2 非对称 STS 自证签名**：`StsAssumeRoleResult` 新增字段 `sm2_signature_hex: String`（使用角色私钥对 `session_token_hex + expiration_ms_LE8` 进行 SM2 签名，GM/T 0003.2-2012）；`StsCredentials::verify` 增加 feature-gated SM2 pubkey 验证路径；`StsService` 接受可选 `Arc<Sm2RoleKeystore>`（角色 id → (pk, sk)），缺省时回退到纯 HMAC-SM3。
- **FR-T24.3 SM4-GCM 对象级 envelope 加密**：`xuanji-server http_server` PUT/GET 路径在 feature `gm-sm` 下：PUT 生成随机 128-bit SM4-DEK，SM4-CTR 包装 DEK 得到 WDEK 存 `object_meta.encrypted_dek_hex`，对象 body 使用 SM4-GCM(nonce=12B, tag=16B) 加密写入；GET 时使用 SM2 unwrap DEK → SM4-GCM 解密；暴露 `x-xuanji-gm-sm4-kid` header 指定 KMS key id。
- **FR-T24.4 真集群 3 主 3 从验收**：
  - `deploy/helm/xuanji/values.yaml` 增加 `replicaCount: 6`, `roleAffinity: {masters: 3, followers: 3}`；
  - Kind 真集群部署：`kind create cluster --name xuanji` + `helm install xuanji deploy/helm/xuanji -f ...`；
  - 选举验证：3 主中 1 个 leader，follower 2 个；`kubectl delete pod xuanji-master-0` 后 30s 内新 leader 当选；
  - 读写验证：从 follower `GET /health` 通过；向 leader 写入对象 10,000 × 1 KB 后 followers `GET /cloud/stats` 计数一致（最终一致窗口 ≤ 5 s 由 xuanji 内部 raft 保证）；
  - PodDisruptionBudget：`minAvailable: 2 masters + 2 followers`，`kubectl drain node` 时业务 0 中断。

### 3.4 T25 Glacier 冷层对接
- **FR-T25.1 StorageClass 扩展 4 档**：`StorageClass::{Hot,Warm,Cold,Glacier}`；`HotWarmColdLifecycle` 重命名为 `LifecycleEngine`（向后兼容 alias），新增阈值 `cold_to_glacier_ms: u64`（默认 365 天）；`TransitionAction` 新增 `ColdToGlacier / GlacierRestoreToCold`。
- **FR-T25.2 Glacier S3 兼容后端 Adapter**：新增 `xuanji-cloud-drive-s3/src/glacier_adapter.rs`（Rust async HTTP client，仅依赖 `reqwest+rustls`），暴露 `GlacierAdapter { endpoint, region, ak, sk }`，方法：`put_object(bucket, key, bytes)` / `initiate_restore(bucket, key, tier)` / `head_object(bucket, key) -> (StorageClass, restore_state)` / `get_object(bucket, key) -> bytes`，支持 S3 v4 signature（复用 `xuanji-cloud-drive-s3/src/s3_sigv4.rs` 如有，否则实现最小签名集）。
- **FR-T25.3 Restore 异步任务状态机**：`RestoreTask { id, bucket, key, tier: Expedited|Standard|Bulk, queued_at_ms, eta_ms, state: Queued|InProgress|Available|Expired|Failed }`，LifecycleEngine 内部 task queue 定时检查 Adapter HEAD，若 Available 则拉取对象回本地 COLD 类 (restore)；`/cloud/glacier/restore/tasks` 列任务列表，`/cloud/glacier/restore/:id/status` 任务状态。
- **FR-T25.4 单元冷层回温**：读 GLACIER 对象时 → 自动提交 Restore(Standard tier) + 返回 `445 Restore In Progress`（自定义 HTTP 状态，附带 Retry-After = eta 秒 header）；读 Available → 返回 body 并将对象 class 保持为 COLD（符合 AWS 语义：restore 后仍在 Glacier，只是产生一份临时副本），`touch_and_restore_to_hot` 触发时先完成 restore 再到 HOT。

---

## 4. 非功能需求 (Non-Functional Requirements, NFR)
- **NFR1 性能 (T22)**：x86_64 (AVX2) 平台上，`12+4` profile × 16 MB payload，encode 吞吐 `≥ (baseline + 35%)` 或下限 `≥ 2.7 GB/s` 二者取较严格；aarch64 NEON `≥ (baseline + 35%)` 或下限 `≥ 1.5 GB/s`；decode 同要求。
- **NFR2 正确性 (T22)**：100 万次随机系数 × 随机 64B 块 gf_mul SIMD 结果 = scalar 结果；1000 次随机 encode/decode with ≤k lost shards 重建位一致。
- **NFR3 国密互操作 (T24)**：SM3 `abc` 消息 64-byte padding 向量输出与 GM/T 标准向量 `66C7F0F462EEEDD9D1F2D46BDC10E4E24167C4875CF2F7A2297DA02B8F4BA0E4` 字节相同；SM2 2P pair (固定 d) 的签名可由 openssl-gm / Tongsuo 工具独立验证；SM4-GCM 向量 128-bit key + 12B nonce + `16 × 0x00` AAD + `0x00*len` 明文解密还原（GM/T 0002 附录 D）。
- **NFR4 Helm 真集群 (T24)**：Kind 真集群部署时间 `helm install` → 6 Pod Running ≤ 120 s；`helm uninstall` → 资源清理 ≤ 60 s；Rolling update（`helm upgrade --set image.tag=new`）零 HTTP 5xx。
- **NFR5 Glacier 成本可追踪 (T25)**：Lifecycle 指标新增 `objects_glacier / bytes_glacier / glacier_restore_running / glacier_restore_wait_sla_ms`；每次 PUT GLACIER 记录 adapter 往返耗时 (latency_ms)。
- **NFR6 向后兼容**：默认 feature `default = []` 下所有 v2.0 API (S3/Graph/Audit/Metrics/Lifecycle 40 项) 行为位一致；所有新增能力均为 opt-in feature flag 或新端点。
- **NFR7 测试数量门**：
  - T22 ≥ 32 unit tests（SIMD vs scalar correctness，含 3 种 profile × 2 种平台 × 4 种 length × 2 种 loss）；
  - T23 ≥ 40 integration tests（20 算子 × 2 种子（obj/tag）+ 最短路径 × 3 对 + 社区检测 × 1）；
  - T24 ≥ 52 tests（SM3 向量 × 12 + SM2 sign/verify × 10 + dual chain × 10 + SM4-GCM roundtrip × 10 + Kind 真集群 6 项）；
  - T25 ≥ 28 tests（TransitionPlan × 8 + Restore state machine × 12 + Adapter Mock (record-replay) × 8）；
  - 合计 ≥ **152** tests。
- **NFR8 打包产物**：xuanji-server release 单二进制在启用 `simd,gm-sm,glacier` 三 feature 后，**体积相较 baseline 增幅 ≤ +35%**（控制依赖膨胀）。

---

## 5. 约束 / 依赖 / 假设 / 开放问题
### 5.1 约束 (Constraints)
- C1：所有新增 Rust crate 必须加入 workspace `[workspace.dependencies]` 统一 version；禁止每个子 crate 独立 pin version。
- C2：任何新增 HTTP 端点必须通过单二进制 `server --single-node`（public port）可访问；不得要求额外启动参数才能暴露。
- C3：不得引入任何商用加密库（SMx 必须自研或使用 `libsm` 风格开源纯 Rust 实现，优先自研）。
- C4：projects/ 目录外不允许生成任何 runtime artifacts（报告、测试输出、截图等一律进 projects/t22-simd-artifacts、projects/t23-projection-artifacts、projects/t24-gm-artifacts、projects/t25-glacier-artifacts）。

### 5.2 依赖 (Dependencies)
- D1：`projection_20.rs` 已存在 20 算子（T23）。
- D2：`HotWarmColdLifecycle` 已有 `transition_scan` / `touch_and_restore_to_hot`（T25）。
- D3：`HashChain` / `StsService` API 已稳定（T24）。
- D4：CI/CD 已有 PowerShell one-liner 总控脚本基础结构（可复用 `scripts/Run-T11-AllTests.ps1` 模板）。

### 5.3 假设 (Assumptions)
- A1：v2.1 验证环境具备 `cargo 1.80+`、`rustup target add x86_64-pc-windows-msvc`（主机），可选 `aarch64-apple-darwin` 交叉；Kind/K3s 本地真集群安装可用（T24 手动验收）。
- A2：Glacier Adapter 默认对接真实 S3 Glacier 需 AWS 凭证；测试走 record-replay mock，真实调用 optional（由环境变量 `RUN_GLACIER_E2E=1` 打开）。
- A3：SM3/SM2/SM4 算法互操作测试向量可通过 GM/T 官方样例复现，不依赖商用硬件。

### 5.4 开放问题 (Open Questions)
- OQ1：Kind 集群验证无 CI runner 时，验收通过是否允许"本机执行成功证据视频 + kubectl 输出截图"？（**暂按允许处理**；review 阶段复核）
- OQ2：SM4-GCM 对象加密是否可选定"仅 Glacier 冷层对象加密"模式以降低性能损失？（**默认对所有类都加；后续通过 bucket policy 细化作为 v2.2 扩展**）
- OQ3：T22 SIMD 是否要同时引入 x86_64 `gf_complete` (James Plank) 风格 Carry-less Multiply (PCLMULQDQ)？（**v2.1 仅做查表 AVX2/NEON 无进位乘法，PCLMULQDQ 作为 T22.2 扩展留 v2.2**）

---

## 6. 验收准则 (Acceptance Criteria)
> 所有 AC 仅使用 `rule` / `rubric` 两类。类型不可混用。

### 6.1 T22 SIMD EC 验收
| ID | 类型 | 验收准则 | 证据来源 |
|---|---|---|---|
| AC-T22-1 | rule | `cargo build --release --features simd -p xuanji-cloud-drive-volume` 0 errors exit=0，且 `objdump` (linux) 或 `dumpbin /disasm` (win) 在生成二进制中出现 `vpxor / vpbroadcastb / vpmovzxbw` 等 AVX2 指令（或 NEON `mov / mul / eor`），证明 SIMD hot path 被编译 | `projects/t22-simd-artifacts/runs/latest/simd_compile_evidence.log` |
| AC-T22-2 | rule | `cargo test --features simd -p xuanji-cloud-drive-volume t22_` 通过 ≥ 32 tests exit=0；其中 `t22_1m_random_gf256_mul_parity` (1M 次 scalar==simd) 通过 | `projects/t22-simd-artifacts/runs/latest/t22_ut_report.json` |
| AC-T22-3 | rule | `t22_encode_16mb_12plus4_avx2` benchmark：吞吐 `≥ 2.7 GB/s`；`t22_decode_lost_4shards_12plus4_avx2` 重建吞吐 `≥ 2.3 GB/s`；两者与 scalar 比值 `≥ 1.35`（即 +35%） | `projects/t22-simd-artifacts/runs/latest/bench_12plus4_16mb.json` |
| AC-T22-4 | rubric | SIMD 架构质量 (0-5)：5=runtime detect+feature flag+NEON 实现+文档齐；4=缺任一项；3=只实现 x86；2=强制编译无 fallback；1=不可运行。阈值 ≥ 4 | `gf256_simd.rs` + `ec_encode_us` 度量结果 |
| AC-T22-5 | rule | `/metrics` 中 `xuanji_ec_simd_enabled=1` 且 `xuanji_ec_encode_avx2_bytes_total > 0`（在 PUT 16 MB 对象后） | validator 抓取单二进制 /metrics |

### 6.2 T23 Graph×Projection20 打通 验收
| ID | 类型 | 验收准则 | 证据来源 |
|---|---|---|---|
| AC-T23-1 | rule | HTTP 端点新增清单与 20 算子一一对应：`GET /graph/projection/list` 返回 `len(list)==20`；20 个 id 为 `proj_{type,community,attr,degree,label}_{in,out}_{1,2}` | `projects/t23-projection-artifacts/runs/latest/operator_list.json` |
| AC-T23-2 | rule | `cargo test -p xuanji-fusion t23_` 通过 ≥ 40 tests；其中 `t23_bridge_bijection_*` 20 项 20 算子在 obj/tag 双种子下 projection 返回 vertices 非空（或 oracle 期望数） | `projects/t23-projection-artifacts/runs/latest/t23_ut_report.json` |
| AC-T23-3 | rule | `POST /graph/path/shortest` 针对 oracle 链 101→partner→102→...→200→101 的 101→103 最短路径 hops == 2，边序列 labels 含 partner×2；`to_s == from_s` 返回 hops == 0 | validator 脚本 |
| AC-T23-4 | rule | `GET /graph/communities` 返回的每个 vertex id 与 GraphWriter 映射一致：任一对象顶点的 community id 在 `SimpleGraph` 与桥接层查询相同；模块度 score 为有限数 | validator + 单二进制抓取 |
| AC-T23-5 | rule | projection 返回对象顶点包含 `size_bytes, etag, crc64_ecma, miji_level, hold_until_ms` 五个字段均非空（对 PUT 带标签 + 设 miji_level=Internal + 设 LegalHold=1y 的对象执行 projection 查询） | validator JSON 断言 |
| AC-T23-6 | rubric | 业务融合覆盖度 (0-5)：5=支持 2-hop/最短路径/社区归属/CNM 重算/by-tag 全量；4=缺一项；3=缺两项。阈值 ≥ 4 | HTTP 端点清单 + 测试断言 |

### 6.3 T24 国密 PKI / 真集群 验收
| ID | 类型 | 验收准则 | 证据来源 |
|---|---|---|---|
| AC-T24-1 | rule | SM3：`sm3_hex(b"abc")` == `66C7F0F462EEEDD9D1F2D46BDC10E4E24167C4875CF2F7A2297DA02B8F4BA0E4`（大写或小写均可），SM3 1M 重复 `0x61` 向量结果与 GM/T 标准一致 | `projects/t24-gm-artifacts/runs/latest/sm3_vectors.log` |
| AC-T24-2 | rule | SM2：使用固定私钥 `d=...` (GM/T 附录 A) 对固定消息签名后，**独立 Tongsuo/OpenSSL-GM 命令行**成功 `openssl sm2utl -verify`；v2.1 代码内 verify 同样通过 | 脚本 + 日志截图 |
| AC-T24-3 | rule | SM4-GCM roundtrip：SM4-GCM(key=16B, nonce=12B, aad=任意, pt=任意字节) → encrypt → decrypt 得回 pt + GCM tag 校验 1 mismatch case 返回 Err；100 次 fuzz roundtrip 全部一致 | t24_sm4 tests |
| AC-T24-4 | rule | HashChain feature `gm-sm` 下的 chain.verify() 与 feature=`default` 的 chain verify 在 dual_chain feature 下 **同一事件序列 integrity=true 且 broken_at=None**（双写一致性） | `cargo test -p xuanji-standards --features dual_chain` 报告 |
| AC-T24-5 | rule | Kind 真集群：`kubectl get pods -o wide` 显示 3 masters + 3 followers (6 Running)；`kubectl delete pod <leader-master>` 后 30s 内 `kubectl get lease xuanji-leader -o yaml` 中 holderIdentity 切换为新主；期间向集群 POST 10K 对象无 5xx；PDB `minAvailable masters=2` | `projects/t24-gm-artifacts/runs/latest/kind_cluster_report.md` + JSON |
| AC-T24-6 | rubric | 国密合规度 (0-5)：5=SM3/SM2/SM4-GCM 均通过 GM/T 向量 + dual chain 迁移 feature + STS SM2 双签名 + 对象 SM4-GCM + Kind/PDB 验证；4=缺一项；3=缺两项；阈值 ≥ 4 | 上述 rule 证据的覆盖率 |

### 6.4 T25 Glacier 冷层对接 验收
| ID | 类型 | 验收准则 | 证据来源 |
|---|---|---|---|
| AC-T25-1 | rule | `StorageClass::Glacier` 存在；`transition_scan(now_ms = created_at + 366days, apply=true)` 中 COLD 超过 cold_to_glacier_ms 阈值对象迁移到 Glacier，生成 `TransitionPlan.action = ColdToGlacier` | `cargo test -p xuanji-cloud-drive-s3 t25_lifecycle_*` |
| AC-T25-2 | rule | RestoreTask 状态机：Queued → InProgress → Available/Expired 四态可切换；3 档 tier (Expedited 1-5min / Standard 3-5h / Bulk 5-12h) 的 eta_ms 正确落在相应区间 ± 10%（由 mock time provider 驱动可测）；`GET /cloud/glacier/restore/:id/status` 返回正确 state | t25_restore tests |
| AC-T25-3 | rule | Adapter 最小签名：mock HTTP server 接受 PUT/RESTORE/HEAD/GET，reqwest 侧生成的 `Authorization: AWS4-HMAC-SHA256 ...` 头可被 mock server 原样校验签名为合法 v4；mock glacier response XML (含 `RestoreRequest`/`RestoreOutput` 子元素) 被 Adapter 正确解析 | record-replay tests 日志 |
| AC-T25-4 | rule | GLACIER 对象 GET 返回 `445 Restore In Progress` HTTP 状态码 + `Retry-After: <eta_ms/1000>` header；触发 `touch_and_restore_to_hot` 后对象先从 GLACIER 自动发起 restore，Available 后再升级到 COLD → 下一次读从 COLD 升级到 HOT | validator 单二进制调用脚本 |
| AC-T25-5 | rubric | Glacier 生态完备度 (0-5)：5=StorageClass 四态+迁移+restore 状态机三档+Adapter v4 签名+metrics 上报+validator 脚本；4=缺一项；3=缺两项。阈值 ≥ 4 | 代码实现清单 + 测试报告 |

### 6.5 综合 Rubric
| ID | 类型 | 验收准则 | 证据来源 |
|---|---|---|---|
| AC-COMP-1 | rubric | **整体性能提升率 (0-5)**：5=T22 +35% 且 T23 projection 2-hop latency ≤ 150ms；4=任一项略低于阈值；3=T22 仅 +25%。阈值 ≥ 4 | T22 吞吐报告 + T23 单二进制 validator |
| AC-COMP-2 | rubric | **企业合规达标度 (0-5)**：5=国密三算法互操作通过+真集群 3+3 PDB 通过+审计链 SM3 双写迁移 feature；4=缺一项；阈值 ≥ 4 | T24 rule 覆盖率 |
| AC-COMP-3 | rubric | **测试数量与质量 (0-5)**：5=实际 ≥ 152 tests 且无 flaky (3 次连续全部通过)；4=140-151；3=130-139；阈值 ≥ 4 | `projects/*/runs/latest/..._ut_report.json` 数量字段 |

---

## 7. 里程碑时间切片（3 个月节奏 = 用户声明基线）
- **第 1 月**：T22 SIMD EC + T23 Projection20 桥接层 & HTTP 端点（AC-T22 ×5 + AC-T23 ×6 = 11 条 AC 完成）；测试 ≥ 72。
- **第 2 月**：T24 国密 SM3/SM2/SM4 + HashChain dual + STS SM2 签名 + SM4-GCM envelope（AC-T24 ×6 完成）；测试 ≥ 124。
- **第 3 月**：T25 Glacier Adapter + Restore 状态机 + Lifecycle 四态 + 真集群 Kind 3+3 PDB（全部 AC 完成）；测试 ≥ 152。

---

## 8. 验收交付物最小集
- **Artifacts**（projects/ 目录，不得污染 platform）：
  - `projects/t22-simd-artifacts/runs/latest/simd_compile_evidence.log`, `t22_ut_report.json`, `bench_12plus4_16mb.json`
  - `projects/t23-projection-artifacts/runs/latest/operator_list.json`, `t23_ut_report.json`
  - `projects/t24-gm-artifacts/runs/latest/sm3_vectors.log`, `sm2_tongsuo_verify.log`, `kind_cluster_report.{md,json}`
  - `projects/t25-glacier-artifacts/runs/latest/glacier_mock_trace.log`, `t25_ut_report.json`
- **总控脚本**：`scripts/Run-V21-AllTests.ps1`（T22→T25 全量一键跑，输出汇总 JSON/MD）
- **Helm 新 values**：`deploy/helm/xuanji/values-3m3s.yaml`（3 主 3 从）与 `deploy/helm/xuanji/kind-hook.sh` (setup/teardown Kind)
- **Validator 新脚本**：`scripts/validate-v21-features.js`（40 项 v2.0 回归 + v2.1 新端点 AC 映射项 = 合计 ≥ 80 子项的 1 个脚本 exit 0 表示 PASS）
