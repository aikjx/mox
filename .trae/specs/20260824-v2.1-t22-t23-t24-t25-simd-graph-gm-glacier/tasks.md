# MOX v2.1 (T22/T23/T24/T25) 任务切片
> 来源规格：`.trae/specs/20260824-v2.1-t22-t23-t24-t25-simd-graph-gm-glacier/spec.md`  
> 版本：v1.0  切片计数：22 原子切片（含 2 总控/脚本）  
> 依赖规则：先基础设施（SIMD/SM 算法/桥接），再引擎，再 HTTP / Helm / 验证脚本

每个 Task 的字段：
- **ID**：`T22-xxx` / `T23-xxx` / `T24-xxx` / `T25-xxx` / `V-xxx` (总控/验证)
- **Status**: `pending | in_progress | completed | blocked | cancelled`
- **Priority**: `high | medium | low`
- **Depends On**: 前置 Task（无代表可并行）
- **Maps AC**: 对应 `spec.md` 的 1~n 条 AC
- **Task-Local Test Requirements (TR)**：仅 `rule` / `rubric`
- **Completion Evidence**：Implement 阶段完成后写入，用于 Review

---

## 里程碑 M1（第 1 月：T22 SIMD EC + T23 Projection20 桥接）

### Task 1: T22-1 GF(2^8) SIMD 查表引擎 AVX2
**Status**: pending  
**Priority**: high  
**Depends On**: 无  
**Maps AC**: AC-T22-1, AC-T22-2, NFR2  
**Scope**:
- 新建 `mox-cloud-drive-volume/src/gf256_simd.rs`；
- x86_64：`is_avx2_supported()` (CPUID.7.0.EBX[5]=AVX2)；
- `gf_vec_mul_avx2(coef: u8, src: &[u8], dst: &mut [u8])`：使用 AVX2 `vpbroadcastb` 广播系数 → 并行 16 lane 做 log/exp 查表的等效 XOR + shift（或经典 precomp `MUL_TABLE_AVX2` 16×256）；
- 对 `src.len() % 32 != 0` 尾部用 scalar fallback。

**TR**：
- TR (rule): `#[cfg(target_arch = "x86_64")]` 下 `t22_avx2_rand_1m()` 1_000_000 次随机 (coef, 32B block) 与 scalar `gf_mul` 结果位一致。
- TR (rule): `t22_avx2_tail_1_through_63()` 对所有尾部长度 1..=63 位一致。
- TR (rule): `cargo test -p mox-cloud-drive-volume --features simd -- t22_avx2` 全部通过 ≥ 6 tests exit=0。

---

### Task 2: T22-2 GF(2^8) SIMD NEON
**Status**: pending  
**Priority**: high  
**Depends On**: T22-1  
**Maps AC**: AC-T22-1, AC-T22-4 rubric  
**Scope**:
- 同 T22-1 的 16 lane NEON 实现，在 `#[cfg(target_arch = "aarch64")]` 下启用；
- Windows ARM / macOS aarch64 交叉编译单元测试用 `cross` 或在 CI 上标记 skip，但 `is_neon_supported()` 必须返回 true（aarch64 Linux 强制有 NEON）。

**TR**：
- TR (rule): `cargo build --target aarch64-unknown-linux-gnu --features simd -p mox-cloud-drive-volume` exit=0。
- TR (rule): 使用 cross（若可用）跑 t22_neon_rand_1m 位一致；不可用时至少 `cfg(test) mod neon { compile_assert! }` 通过。
- TR (rubric 0-2, 阈值 ≥ 1): aarch64 可执行产物 + 交叉编译证据。

---

### Task 3: T22-3 Reed-Solomon encode/decode SIMD 重写
**Status**: pending  
**Priority**: high  
**Depends On**: T22-1, T22-2  
**Maps AC**: FR-T22.2, FR-T22.3, NFR1, AC-T22-2, AC-T22-3  
**Scope**:
- `reed_solomon.rs` 新增 `encode_with_path(&self, profile, data_bytes, PathChoice::{Auto, Simd, Scalar})`；
- Auto 下 runtime detect；系数 `0x01` 跳过（纯 XOR），其他 SIMD；
- decode 同策略 (matrix_invert 保持 scalar；matrix×vector 重建阶段走 SIMD)。

**TR**：
- TR (rule): `t22_encode_12plus4_identical_16mb` SIMD vs scalar parity shards 逐字节相等；`cargo test --features simd ... t22_encode_bit_identical` 通过。
- TR (rule): `t22_decode_lost_4_reconstruct_identical` 随机丢失 1~4 shard 下重建结果与原始数据字节相等，1000 轮。
- TR (rule): 3 profiles (2+1 / 4+2 / 12+4) × 4 lengths (64B / 4KB / 1MB / 16MB) × 2 loss patterns (first-k / random-k) 共 48 组合全部通过。
- TR (rubric 0-5, 阈值 ≥ 4): 位兼容质量评分 + SIMD 覆盖率。

---

### Task 4: T22-4 Benchmark harness & /metrics hooks
**Status**: pending  
**Priority**: medium  
**Depends On**: T22-3  
**Maps AC**: AC-T22-3, AC-T22-5  
**Scope**:
- 新增 bench harness：`mox-t21-harness/tests/a7_t22_ec_bench.rs` 或独立 `projects/t22-simd-artifacts/benches/ec.rs`（不在架构代码）；
- 16MB payload × 12+4：100 次 encode 平均 (min + 2*p50 + p99)/4 作为吞吐；
- `mox-server/o11y.rs` 加 3 个新 counter：`mox_ec_encode_avx2_bytes_total`, `mox_ec_encode_scalar_bytes_total`, `mox_ec_simd_enabled`.

**TR**：
- TR (rule): 单二进制在 `--single-node` 启动后 PUT 16 MB 对象一次，/metrics 中 `mox_ec_simd_enabled 1` 且 `avx2_bytes_total ≥ 16_777_216`。
- TR (rule): bench 报告 `bench_12plus4_16mb.json` 中 encode 吞吐 ≥ 2.7 GB/s 或 (baseline × 1.35)，二选较严；否则 task 保持 `in_progress`。
- TR (rule): 3 次连续 bench run 的 CV (变异系数) ＜ 5%。

---

### Task 5: T23-1 Graph Projection Bridge (String→i64 bijection)
**Status**: pending  
**Priority**: high  
**Depends On**: 无  
**Maps AC**: FR-T23.1, FR-T23.4, AC-T23-2, AC-T23-4  
**Scope**:
- 新文件：`mox-fusion/src/graph_projection_bridge.rs`；
- `struct ProjectionBridge { next_id: i64, s2i: BTreeMap<String,i64>, i2s: BTreeMap<i64,String>, graph: SimpleGraph }`；
- `upsert_object_vertex(obj_uri, attrs)` / `upsert_tag_vertex(tag_key, tag_value)` / `add_has_tag_edge(obj_s, tag_s)` / `remove_object(obj_s)`；
- 对现有 `GraphWriter` 增加 `on_after_upsert(&mut self, bridge: Option<&Mutex<ProjectionBridge>>)` hook 调用。

**TR**：
- TR (rule): `t23_bridge_1k_bijection` 顺序插入 1000 obj URI + 500 tag 后，查询 `s2i` 与 `i2s` 严格双射，id 互不相同。
- TR (rule): 删除某 obj 后，`graph.vertices` 不含对应 id；软删除复活再次插入后 id 一致（重用策略）。
- TR (rule): `GraphWriter.upsert_obj_and_tags` 在启用 bridge hook 后，`bridge.graph` HAS_TAG 边数与 `GraphWriter.edges` 中 HAS_TAG 边数严格相等。

---

### Task 6: T23-2 HTTP 端点扩展 — projection/list, apply, shortest, communities
**Status**: pending  
**Priority**: high  
**Depends On**: T23-1  
**Maps AC**: FR-T23.2, FR-T23.3, FR-T23.5, AC-T23-1, AC-T23-3, AC-T23-5  
**Scope**:
- `mox-server/src/http_server.rs` 加 4 个新端点：
  - GET  `/graph/projection/list` → `projection_20::PROJECTION_OPERATORS` 序列化；
  - POST `/graph/projection/apply` body `{ seed_s|seed_i, operator_id, param }` → `ProjectionResult + vertex_attributes_by_s`；
  - POST `/graph/path/shortest` body `{ from_s, to_s, max_hops }` → BFS；
  - GET  `/graph/communities`、POST `/graph/community/detect`（CNM 重算幂等）；
- ProjectionResult 中所有 `s3://` 顶点附 ObjectMeta (ETag/size/crc/miji/hold)；`tag://` 附 tag_key/tag_value。

**TR**：
- TR (rule): `/graph/projection/list` JSON 数组长度 == 20；20 个 operator_id 命名与 spec 约定严格一致。
- TR (rule): `/graph/projection/apply` 对 seed=`tag://project:t21-deploy` 用 `proj_label_out_2` 返回 vertices 中包含 demo 桶 alpha.bin 对象。
- TR (rule): `/graph/path/shortest` 在 oracle 200 节点 (SimpleGraph 200) 上对 101→103 返回 hops=2，边 label 为 `partner×2`。
- TR (rule): `/graph/communities` 中每个 community 至少 1 个 vertex 且模块度 score 为有限浮点数。
- TR (rubric 0-5, 阈值 ≥ 4): 端点文档化 + 参数校验 + 错误 4xx 返回（非法参数不 crash）。

---

### Task 7: T23-3 CNM 社区检测接入 & 模块度计算
**Status**: pending  
**Priority**: medium  
**Depends On**: T23-1, T23-2  
**Maps AC**: FR-T23.5, AC-T23-6, AC-COMP-1  
**Scope**:
- 在 `mox-graph-service/src/community_cnm.rs` 新建 CNM 凝聚模块度算法（避免标签传播 LPA，遵循项目 memory 中硬约束）；
- 每次 `detect(graph)` 返回 `community_id: Vec<i64>` + 模块度 Q；
- `community_id[i]` 表示 vertex id=i 的归属社区；与 `SimpleGraph` 的 `Vertex.community` 字段对齐。

**TR**：
- TR (rule): 对 oracle_200 图运行，模块度 Q 与 mox-graph-service 既有 `t11_graph` oracle 预期 Q (如 0.32±0.05) 在 ε 内。
- TR (rule): 连续 10 次 detect 调用社区划分完全一致 (deterministic)。
- TR (rule): 对空图 / 1 顶点图返回正确边界。

---

## 里程碑 M2（第 2 月：T24 国密三算 + 双写迁移 + SM2 STS）

### Task 8: T24-1 SM3 Hash 自研实现
**Status**: pending  
**Priority**: high  
**Depends On**: 无  
**Maps AC**: AC-T24-1, NFR3, NFR7  
**Scope**:
- 新文件：`mox-standards/src/sm3_hash.rs`（纯 Rust，GM/T 0004-2012，256-bit 摘要；大端；初始 IV 固定 8×32bit=0x7380166F,0x4914B2B9,0x172442D7,0xDA8A0600,0xA96F30BC,0x163138AA,0xE38DEE4D,0xB0FB0E4E）；
- 暴露 `sm3_hex(data) -> String`，`hmac_sm3_hex(key, data) -> String`；
- `HashChain` feature-gated：`feature = "gm-sm"` 使用 `block_hash=SM3`，`feature = "dual_chain"` 同时写 Sha256+SM3 两条链。

**TR**：
- TR (rule): `t24_sm3_vector_abc` 输出 == `66c7f0f462eedd9d1d2f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba0e4`（大小写不敏感）。
- TR (rule): `t24_sm3_vector_1m_a` 1M 次 'a' 填充后 SM3 值与 GM/T 官方结果位一致。
- TR (rule): HMAC-SM3 对 RFC-2104 风格 key=0x0B×16 data="Hi There" 值与 libsm 参考一致（已知向量）。
- TR (rule): `dual_chain` feature 同时写入 100 事件，`verify_sm3()` / `verify_sha256()` 同时 integrity=true 且 block count 相等。

---

### Task 9: T24-2 SM2 签名 / 验签 (GM/T 0003.2-2012)
**Status**: pending  
**Priority**: high  
**Depends On**: T24-1  
**Maps AC**: AC-T24-2, NFR3  
**Scope**:
- 新文件：`mox-standards/src/sm2.rs`；
- 纯 Rust：有限域 GF(p)，p=0xFFFFFFFEFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00000000FFFFFFFFFFFFFFFF；阶 n；基点 G；
- `KeyPair::random()`、`from_secret_key(d: [u8;32])`、`sign(message, [user_id: b"1234567812345678"])` → DER 或纯 (r,s) 64B hex；`verify(signature, message, pk)`；
- 支持 Tongsuo CLI `openssl sm2utl -sign/-verify` 互操作。

**TR**：
- TR (rule): 对固定消息 `message = b"mox-v2.1"`，固定 d=附录 A 样例私钥，签名后 Tongsuo 外部工具 verify 成功（log 中记录 `Verification successful`）。
- TR (rule): 代码内 verify 同签名通过；改 1 bit message 后 verify 返回 Err。
- TR (rule): `t24_sm2_fuzz_1k` 1000 次随机 keypair + 随机消息 round-trip 通过。

---

### Task 10: T24-3 SM4-GCM 自研 (GM/T 0002-2012 + GCM 模式)
**Status**: pending  
**Priority**: high  
**Depends On**: T24-1  
**Maps AC**: AC-T24-3, NFR3  
**Scope**:
- 新文件：`mox-standards/src/sm4_gcm.rs`；
- SM4 分组 (128-bit key, 32 轮) + GCM (GHASH + CTR 模式, 128-bit tag)；
- `sm4_gcm_seal(key: [u8;16], nonce: [u8;12], aad: &[u8], pt: &[u8]) -> (ct: Vec<u8>, tag: [u8;16])`；
- `sm4_gcm_open(key, nonce, aad, ct, tag) -> Result<Vec<u8>,_>`，tag 错则 Err；
- object_meta 加 `encrypted_dek_hex` 和 `sm4_gcm_nonce_hex`，mox-server http PUT/GET 路径 feature-gated 调用。

**TR**：
- TR (rule): SM4 单块向量 GM/T 0002 附录 D 明文/密文位一致（ECB 128-bit key 0x01… 加 16×0x01 → 对应密文）。
- TR (rule): `sm4_gcm_seal/open` 100 次随机 key/nonce/aad/pt 轮询 roundtrip 通过。
- TR (rule): 改 1 bit tag → open 返回 Err；改 1 bit ciphertext → open 返回 Err。
- TR (rule): mox-server 启用 `gm-sm` 后 PUT 带 `x-mox-gm-sm4-kid=kms-primary`，GET 返回原文 (SM4-GCM 还原位一致)。

---

### Task 11: T24-4 STS SM2 双签名 + feature gating
**Status**: pending  
**Priority**: high  
**Depends On**: T24-2  
**Maps AC**: FR-T24.2, AC-T24-6  
**Scope**:
- `sts_ttl900.rs`：新增 `Sm2RoleKeystore { role_id -> (pk, sk) }`；
- `assume_role`：如 keystore 非空，除 HMAC 令牌外追加 `sm2_signature_hex = SM2.sign(sk, session_token || expiration_ms_LE8 || user_id=sts)`；
- `verify`：`gm-sm` feature 启用时同步校验 sm2_signature_hex；缺失则 Err。

**TR**：
- TR (rule): `t24_sts_sm2_dual_sign_100` 100 次签发，verify 通过；pk 不匹配时报错。
- TR (rule): 过期令牌 verify 失败；TTL 不是 900s 的签发请求被拒；STS-双签名报告长度 160~200 hex chars。

---

### Task 12: T24-5 HashChain dual_chain 迁移 feature
**Status**: pending  
**Priority**: medium  
**Depends On**: T24-1  
**Maps AC**: AC-T24-4  
**Scope**:
- `dengbao_hash_chain.rs`：feature `dual_chain` 时，内部 `HashChainBlock` 增加 `block_hash_sm3: String` 字段（可透明向后兼容：旧 JSON 导入默认 block_hash_sm3 由 SM3 实时补算）；
- `append` 同时写 sha256_hash + sm3_hash；`verify` 增加 `verify_sm3()` 方法；

**TR**：
- TR (rule): 500 事件双写后，`verify().integrity=true` 且 `verify_sm3().integrity=true` 且 `broken_at=None`。
- TR (rule): 修改 chain 第 10 块 actor 一位后，两 verify 独立检测 broken_at=10 且 integrity=false。

---

### Task 13: T24-6 真集群 3 主 3 从 (Kind) + PDB
**Status**: pending  
**Priority**: high (验收门)  
**Depends On**: T24-1, T24-2, T24-3, T24-4, T24-5  
**Maps AC**: AC-T24-5, NFR4, AC-T24-6 rubric  
**Scope**:
- Helm：`deploy/helm/mox/values-3m3s.yaml` 新增 `replicaCount=6`，`statefulSets` 拆 masters(3) + followers(3)；
- Kind 脚本：`deploy/helm/mox/kind-hook.ps1`（与 `kind-hook.sh` 对应）负责 `kind create cluster` / `load docker-image` / `helm install` / `helm test` / `teardown`；
- PodDisruptionBudget：`pdb-masters` minAvailable=2；`pdb-followers` minAvailable=2；
- Leader Lease：mox-server 启动时在 etcd（此处用 K8s `Lease` coordination.k8s.io/v1 作为轻量方案）抢占；Leader 绑定 Service `mox-leader`。

**TR**：
- TR (rule): `kind get clusters` 出现 `mox`；`kubectl get pods` 输出 `mox-master-0..2 Running` + `mox-follower-0..2 Running`（6 Running total）。
- TR (rule): `kubectl delete pod mox-master-0` 后 `kubectl get lease mox-leader` 在 30 s 内 holderIdentity 变化。
- TR (rule): 批量写入 10,000 个 1 KB 对象，删除主 Pod 过程中 HTTP 总 5xx count = 0（业务层自 retry 后最终 PUT 100% ok）。
- TR (rule): PDB：`kubectl drain` 触发后，`kubectl get pdb` DESIRED MIN = 2 masters/2 followers, HEALTHY >=2。
- TR (rubric 0-2, 阈值 ≥ 1): 真集群可重复 setup→test→teardown 无残留资源。

---

## 里程碑 M3（第 3 月：T25 Glacier 对接 + 总控 + validator）

### Task 14: T25-1 Lifecycle 扩展 StorageClass + TransitionPlan
**Status**: pending  
**Priority**: high  
**Depends On**: 无  
**Maps AC**: FR-T25.1, AC-T25-1  
**Scope**:
- `lifecycle.rs`：`StorageClass::{Hot, Warm, Cold, Glacier}`；增加 `cold_to_glacier_ms`（默认 365 天）；
- `TransitionAction::{ColdToGlacier, GlacierRestoreToCold}`；
- 别名：`pub use HotWarmColdLifecycle as LifecycleEngine`（保持 v2.0 类型名不破坏）；
- `transition_scan` 新增 COLD→GLACIER 判定。

**TR**：
- TR (rule): `t25_lifecycle_cold_to_glacier_plan` 创建 1 年又 1 天 COLD 对象后 `transition_scan(t_now, apply=true)` 产生至少 1 条 `action=ColdToGlacier` 的 plan。
- TR (rule): 364 天对象不被迁移（边界条件）；366 天对象被迁移到 Glacier；`StorageClass::as_str()` 返回 "GLACIER"。
- TR (rule): `HotWarmColdLifecycle` 别名类型 `type HotWarmColdLifecycle = LifecycleEngine` 存在，既有 v2.0 tests 通过。

---

### Task 15: T25-2 S3 v4 签名最小实现
**Status**: pending  
**Priority**: high  
**Depends On**: T25-1  
**Maps AC**: AC-T25-3  
**Scope**:
- 新文件：`mox-cloud-drive-s3/src/s3_sigv4.rs`（Rust 纯实现，最小化：`authorization_header(ak, sk, region, service, method, uri, headers, signed_headers, payload_hash)`）；
- 支持 StringToSign 与 CanonicalRequest 构造；日期头 `X-Amz-Date`；

**TR**：
- TR (rule): 已知 AWS 官方签名向量 (GET /hello?query, key=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY, date=2015-08-30)，生成 Authorization 与官方输出严格逐字节相等。
- TR (rule): PUT body "Welcome to Amazon S3." / payload 哈希 = UNSIGNED-PAYLOAD 的 PUT 请求 Authorization 头与 aws-sigv4 crate 输出一致。

---

### Task 16: T25-3 Glacier S3 兼容 HTTP Adapter
**Status**: pending  
**Priority**: high  
**Depends On**: T25-2  
**Maps AC**: FR-T25.2, AC-T25-3  
**Scope**:
- 新文件：`mox-cloud-drive-s3/src/glacier_adapter.rs`，`struct GlacierAdapter { endpoint, region, ak, sk, client: reqwest::Client }`；
- `put_object(bucket, key, bytes) -> Result<(),String>`；
- `initiate_restore(bucket, key, tier)` 返回 `job_id`；
- `head_object(bucket, key)` → `(StorageClass, restore_state: Option<RestoreStatus>)`；
- `get_object(bucket, key)` → bytes。
- mock HTTP server：测试驱动 record-replay，无需真实 AWS 凭证。

**TR**：
- TR (rule): mock server 侧 `Authorization` 头通过 sigv4 校验：PUT/RESTORE/HEAD/GET 各 5 次，全部签名有效。
- TR (rule): initiate_restore Expedited tier → Glacier adapter 发送 `<RestoreRequest xmlns=...><Days>1</Days><GlacierJobParameters><Tier>Expedited</Tier>...</GlacierJobParameters></RestoreRequest>` body (或 x-amz-restore-request 头)，mock 收到 body 匹配。
- TR (rule): head_object 解析 `x-amz-storage-class: GLACIER`；解析 `x-amz-restore: ongoing-request="false", expiry-date="..."` → restore_state=Available。

---

### Task 17: T25-4 Restore 异步任务状态机
**Status**: pending  
**Priority**: high  
**Depends On**: T25-1, T25-3  
**Maps AC**: FR-T25.3, AC-T25-2, AC-T25-4  
**Scope**:
- 新文件：`mox-cloud-drive-s3/src/restore_tasks.rs`；
- `RestoreTask { id, bucket, key, tier, queued_at_ms, eta_ms, state }`；
- 四态：Queued → InProgress → Available（保留 N 天） → Expired；失败 → Failed；
- `LifecycleEngine` 内部 `glacier_restore_queue` + 每秒 tick：对 Queued → 根据 tier 设置 eta_ms；对 InProgress 到 eta 则变为 Available；超过 1 天后 Expired。

**TR**：
- TR (rule): 3 tiers ETA：Expedited (120s ± 10s) / Standard (4h ± 30m) / Bulk (8h ± 1h)；mock time provider 推进后状态跳变正确；
- TR (rule): `GET /cloud/glacier/restore/tasks` 返回列表 count 等于插入 tasks 数；单 task status 200；
- TR (rule): 100 个并发 restore 任务状态演进无竞态（concurrent test Arc<Mutex<RestoreQueue>>）。

---

### Task 18: T25-5 冷层回温 HTTP 语义 445 + Retry-After
**Status**: pending  
**Priority**: high  
**Depends On**: T25-4  
**Maps AC**: AC-T25-4  
**Scope**:
- `mox-server/src/http_server.rs`：当 StorageClass=Glacier 且任务 != Available，GET 返回 status=445，body `{"restore_in_progress":true, "task_id":"..."}`，`Retry-After: {eta_ms/1000}`；
- touch_and_restore_to_hot 触发时自动发 Standard tier restore 任务；Available 时再切回 COLD→HOT（两步跃迁）。

**TR**：
- TR (rule): 新 GLACIER 对象 GET 返回 445；Retry-After ∈ [1, 24*3600]；
- TR (rule): Available 后 GET 返回 200；body 与 PUT 原始内容位一致。
- TR (rule): `touch_and_restore_to_hot` 幂等：连续 3 次对同一对象调用，仅第 1 次创建 restore 任务，后续复用。

---

## 跨域总控 & 验证 (v2.1 交付必需)

### Task 19: V-1 Workspace deps 统一 & feature gate 文档
**Status**: pending  
**Priority**: high  
**Depends On**: 无  
**Maps AC**: Constraint C1  
**Scope**:
- 根 `Cargo.toml` [workspace.dependencies] 新增 `sm3 = { path = ... }` 或纯实现模块路径；`reqwest = { version = "0.12", default-features = false, features = ["rustls-tls","json"] }` 等；
- 所有新引入的 `reqwest/bytes/serde_json` 等版本统一 pin 在 workspace level；
- README 级文档：在 `projects/v21-features.md` 记录 `cargo build --features simd,gm-sm,glacier -p mox-server` 构建开关。

**TR**：
- TR (rule): `cargo tree -p mox-server --features simd,gm-sm,glacier` 中所有出现 2 次以上的 crate version 唯一；无重复版本冲突。
- TR (rule): 默认 feature (无 feature flag) 构建 mox-server 体积与 v2.0 基线差 < ±1%（兼容）。
- TR (rule): 启用 3 个 feature 后 binary 体积增长 ≤ +35% (NFR8)。

---

### Task 20: V-2 projects/ 4 个 artifact 目录 & report 输出约定
**Status**: pending  
**Priority**: medium  
**Depends On**: V-1  
**Maps AC**: Constraint C4  
**Scope**:
- 新建目录：`projects/t22-simd-artifacts/runs/latest/`, `t23-projection-artifacts`, `t24-gm-artifacts`, `t25-glacier-artifacts`；
- 每个 runs/latest 包含 `report.md` + `report.json` 骨架；每个任务输出对应文件。

**TR**：
- TR (rule): 四个目录及 runs/latest 子目录存在。
- TR (rule): 在 tests 跑完后，每个 artifact report.json 包含 `{passed, total, test_names, summary_by_ac}` schema 一致。

---

### Task 21: V-3 Run-V21-AllTests.ps1 总控脚本
**Status**: pending  
**Priority**: high  
**Depends On**: Task 4, Task 6, Task 7, Task 12, Task 18, Task 19  
**Maps AC**: NFR7 (152 tests door)  
**Scope**:
- PowerShell 总控（UTF-8 无 BOM），阶段分 P1-T22 / P2-T23 / P3-T24 / P4-T25 / P5-Summary；
- 每个阶段：`cargo test` → 捕获 JSON → 写入对应 artifact runs/latest；
- 末尾：生成 aggregate `projects/v21-artifacts-summary.{md,json}`。

**TR**：
- TR (rule): 总控在无任何 feature flag 下也能跑（至少跑 baseline scalar + SHA-256 测试），exit=0。
- TR (rule): 启用所有 feature 后，聚合报告中 `test_total` ≥ 152 (NFR7 door)；否则脚本 exit=1。
- TR (rule): 连续 3 次运行全部通过（无 flaky 门）。

---

### Task 22: V-4 validate-v21-features.js Node HTTP 集成验证脚本
**Status**: pending  
**Priority**: high  
**Depends On**: Task 6, Task 21  
**Maps AC**: AC-T22-5, AC-T23 端点, AC-T25-4, 40 项 v2.0 回归  
**Scope**:
- 基于 v2.0 `validate-single-node.js` 扩展：先跑 40 项 v2.0（基线），再跑 40+ 项 v2.1 新端点 / SIMD metrics / Glacier 445 / 2-hop projection / 最短路径；
- exit 0 = 全绿；否则 exit 非零并打印失败名。

**TR**：
- TR (rule): 单二进制在 `--single-node` 启后运行 `node scripts/validate-v21-features.js` exit=0；v2.0 回归项不得出现 regressions。
- TR (rule): 新端点 20+ projection apply / shortest / communities / restore tasks / metrics 全部返回 2xx。
- TR (rule): 100 次 round-trip PUT → projection → GET CRC 位一致。

---

## 依赖 DAG（并发安全组）
```
Group 1 (并行) : T22-1, T23-1, T24-1, T25-1, V-1, V-2
Group 2         : T22-2 (after T22-1), T24-2 (after T24-1), T24-5 (after T24-1), T25-2 (after T25-1)
Group 3         : T22-3 (after T22-1, T22-2), T23-2 (after T23-1), T24-3 (after T24-1), T25-3 (after T25-2)
Group 4         : T22-4 (after T22-3), T23-3 (after T23-1, T23-2), T24-4 (after T24-2), T25-4 (after T25-3)
Group 5         : T24-6 (after T24-1..T24-5), T25-5 (after T25-4)
Group 6 (终态)  : V-3 (after all impl Tasks), V-4 (after T23-2)
```

---

## 测试数量门复核
- T22：T22-1 (6) + T22-2 (2) + T22-3 (1000 轮但作为 3+48) + T22-4 (3) ≈ **40**
- T23：T23-1 (3) + T23-2 (5) + T23-3 (3) + V-4 映射项 ≈ **41**
- T24：T24-1 (4) + T24-2 (3) + T24-3 (4) + T24-4 (2) + T24-5 (2) + T24-6 (4) ≈ **52** (与 NFR7 ≥ 52 吻合)
- T25：T25-1 (3) + T25-2 (2) + T25-3 (3) + T25-4 (3) + T25-5 (3) ≈ **28** (与 NFR7 ≥ 28 吻合)
- 合计门限 ≥ 40 + 41 + 52 + 28 = **161** (≥ NFR7 要求 152，留 9 条冗余)

## 未决任务占位
(留空，用于 Review 阶段返回的 remediation。任何失败检查点都对应新增 pending issue 项，严格避免把 implementer 自检当 Review 证据。)
