# MOX v2.0 任务切片 — AC: rule(12) + rubric(8)

## 依赖图（串行 → 并行组）
1. 骨架 → Task 1 (workspace skeleton + metric registry + WASM ABI stubs)
2. 并行组 A（数据）：Task 2 (EC Engine) / Task 3 (Multipart UploadId)
3. 并行组 B（融合 + 合规）：Task 4 (Tag-CDC-Graph) / Task 5 (LegalHold + Miji)
4. 并行组 C（运维 + 部署）：Task 6 (FSHC + Mountpath) / Task 7 (ETL WASM)
5. Task 8 (Single Binary mox-server CLI)
6. Task 9 (P99 Observability + CRC Read-after-Write)
7. Task 10 (Bench 三档 + Deploy 双路径)
8. Task 11 (T21 E2E ≥ 1237 tests 汇总 + Harness 700 参数化)
9. Task 12 (Rubric 汇总 + Grade S 验收)

---

## Task 1: Workspace 骨架 + 指标注册中心 + WASM ABI stubs

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | 所有 FR 前置（类型定义共享） |
| Dependencies | — |
| Blocked By | — |
| Unblock Condition | — |

### 产物
- `platform/services/mox-ec-engine/Cargo.toml` + stub lib.rs
- `platform/services/mox-data-plane/Cargo.toml`（HTTP3 listener + mountpath/fshc）
- `platform/services/mox-fusion/Cargo.toml`（Tag-CDC-Graph）
- `platform/services/mox-compliance/Cargo.toml`（LegalHold + Miji）
- `platform/services/mox-etl-wasm/Cargo.toml`（WASM plugin 注册中心）
- `platform/services/mox-server/Cargo.toml`（bin=`mox`；workspace workspace-hack 可选）
- 修改 workspace `Cargo.toml`：`members += ["platform/services/mox-ec-engine", "..."]` 6 个新成员
- `platform/services/mox-observability/src/registry_ext.rs`：注册 9 项新指标（histogram + counter）

### Task-local Test Requirements (TR)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T1-TR1 | rule | 6 个新 crate `cargo check` 通过 0 error | 6/6 | cargo check 输出 |
| T1-TR2 | rule | Metric Registry 注册后 `prometheus.gather()` 包含 `mox_obj_put_latency_us` 等 9 指标名 | metric name count = 9 | registry unit test |
| T1-TR3 | rule | WASM ABI stub `EtContext::new()` 创建 + 空 transform 返回 Ok(Vec::new) | pass | etl abi unit test |
| T1-TR4 | rubric | 依赖图解耦与目录布局工程质量（0-100） | ≥ 90 S | code review 结构评审 |

### Completion Evidence
_（待实施时填写）_

---

## Task 2: EC Engine - Reed-Solomon(n+k) + 分片 + 自修复 xaction

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | AC-R1, AC-U1 |
| Dependencies | Task 1 (mox-ec-engine skeleton) |
| Blocked By | — |
| Unblock Condition | — |

### 产物
- `mox-ec-engine/src/profile.rs`：EcProfile { data, parity, min_obj_size }
- `mox-ec-engine/src/encode.rs`：RSEncode::encode(data) -> Vec<Vec<u8>>
- `mox-ec-engine/src/decode.rs`：RSDecode::reconstruct(shards, missing_idx) -> Result<Vec<u8>>
- `mox-ec-engine/src/manifest.rs`：EcManifest { oid, bid, crc64, shards[], created_at, profile }
- `mox-ec-engine/src/fs_layout.rs`：`<mountpath>/<bid>/<oid_prefix2>/<oid>/ec/shard_{i}.slice` + manifest.json
- `mox-ec-engine/src/rebuild.rs`：RebuildJob（后台 async task，失败重试 + 指标更新）

### Task-local TR（≥ 16）
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T2-TR1 | rule | 4+2: 编码 1KB / 64KB / 1MB / 16MB 4 档 size 全通过；decode 100% 一致 | 4/4 | ec_basic |
| T2-TR2 | rule | 8+4: 128MB 对象编码/解码通过；12 片任意丢 4 片可恢复 | 恢复成功 10 场景 | ec_8_4_recovery |
| T2-TR3 | rule | 4+2: 丢 3 片 (超过 parity=2) → 返回 Err(TooManyLost) | Err 场景 100% | ec_too_many_lost |
| T2-TR4 | rule | 12+4: 1GB 对象模拟分块；16 片丢 4 片 recover 对比原始 sha256 = | sha256 match | ec_12_4_gb |
| T2-TR5 | rule | Manifest serde roundtrip：JSON 序列化/反序列化字段 24 项一致 | 24/24 | manifest_serde |
| T2-TR6 | rule | FS layout：写入 1 对象 → 检查 n+k shard files 存在 + manifest 存在 + 大小一致 | 目录检查 pass | ec_fs_layout |
| T2-TR7 | rule | RebuildJob：标记 shard_1,shard_3 faulty → start rebuild → 2 shards 重新出现且 crc64 与原始匹配 | crc match 2/2 | ec_rebuild_job |
| T2-TR8 | rule | 桶级 profile 默认 4+2；设置 8+4 后新对象进入 12 shards；旧对象仍以 4+2 可读（向后兼容） | 新旧对象都可读 | ec_bucket_profile |
| T2-TR9 | rule | min_obj_size：对象 < 64KB → 不进入 EC（3 副本）；≥64KB 进入 EC（2 场景） | 路径正确 2/2 | ec_threshold |
| T2-TR10 | rule | CRC64 写时校验：manifest crc 错误 → decode 返回 IntegrityError | Err 正确 | ec_crc64_integrity |
| T2-TR11 | rule | 并发 32 对象 4+2 编码 + 随机 2 片丢失 + 并行 recover + 最终 sha256 全匹配 | 32/32 match | ec_concurrency |
| T2-TR12 | rule | mox_ec_encode_us 指标 encode 1000 次 histogram bucket 命中 ≥ 95% | histogram fill ≥ 95% | ec_metrics |
| T2-TR13 | rule | profile.data=1 非法（RS 要求 data≥2） → new_profile 返回 Err | InvalidParams | ec_profile_valid |
| T2-TR14 | rule | 支持 custom_galois_field=8（默认），兼容既有 AIS Reed-Solomon GF(2^8) | 互操作向量 8 对 8 | ec_interop |
| T2-TR15 | rule | RebuildJob 指标 mox_ec_rebuild_count 增加 1 当且仅当成功 | counter delta=1 | ec_rebuild_counter |
| T2-TR16 | rule | 冷热分层迁移：Lifecycle.move_to_cold 触发对象 EC manifest 带 tier=archive 标签；GET 返回 tier 正确 | tier 标签一致 | ec_lifecycle_cold |

### Completion Evidence
_（待实施时填写）_

---

## Task 3: Multipart UploadId + 可选 HTTP3(QUIC) + 三语言 SDK

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | AC-R2, AC-U2 |
| Dependencies | Task 1 |

### 产物
- `mox-data-plane/src/multipart.rs`：MultipartStore { upload_id, parts[]: Part, created_at, expires } + Create/Abort/Complete
- `mox-data-plane/src/listeners.rs`：TripleListener { public, intra_ctrl, intra_data }；`--http3` 时 public 走 quinn endpoint
- Rust SDK：`Client::create_multipart_upload()` → `Uploader::upload_part(part_num, bytes)` → `complete()` → PartAggregate{crc64, etag, n}
- Node.js SDK：`uploadMultipart(bucket, key, stream, {partSize: 8*1024*1024})` 统一 Promise API
- Python SDK：`multipart_upload(bucket, key, iterable, part_size=8*1024*1024)` context manager

### Task-local TR（≥ 21）
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T3-TR1..7 | rule ×7 | Rust SDK：空 64KB / 6×8MB+2MB=50MB / 101×1MB=101MB / abort / complete_crc64_match / server_restart_uploadid_persistent / concurrent_parts_order_independent | 7/7 | rust multipart |
| T3-TR8..14 | rule ×7 | Node.js SDK：对应 7 场景 | 7/7 | node multipart |
| T3-TR15..21 | rule ×7 | Python SDK：对应 7 场景 | 7/7 | python multipart |
| T3-TR22 | rule | TripleListener：三端口绑定；public/intra_ctrl/intra_data health endpoint 各返回 ok | 3/3 200 OK | listeners_smoke |
| T3-TR23 | rule | HTTP3 `--http3` 启动；curl/quiche 客户端能 GET（若环境缺实现，则至少 `listener.into_async()` 返回 Poll::Ready） | pass | http3_smoke |
| T3-TR24 | rubric | 三语言 API 一致性：签名、错误码、UploadId 格式（uuid v4 + HMAC suffix）、CRC 算法一致 | ≥ 90 S | api_consistency |

### Completion Evidence
_（待实施时填写）_

---

## Task 4: PutObject → Tag → CDC → 知识图谱 融合

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | AC-R3, AC-U3 |
| Dependencies | Task 1 + T11 Graph service 既有 projection |

### 产物
- `mox-fusion/src/tag_parser.rs`：TagSet.from_s3_headers(headers) + 默认 contentType/size_bucket/mimeCategory 标签
- `mox-fusion/src/cdc_stage.rs`：`tag_cdc_graph_stage(obj, tags) -> [CdcEvent::ObjectTagged; 1]`
- `mox-fusion/src/graph_writer.rs`：GraphWriter::upsert_obj_and_tags(obj, tags) → 落既有 mox-graph-service（mock+real）
- GraphQL schema 新增 `type Obj { uri: String!, bucket: String!, size: Long!, tags: [Tag!]! }` + query `objectsByTag(k:"project", v:"p1"): [Obj!]!`

### Task-local TR（≥ 14）
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T4-TR1 | rule | PUT 1 对象 {3 自定义 + 2 默认} → ObjectTagged event + UPSERT obj + 5 tag + 5 HAS_TAG 边 | count 验证 1+5+5=11 | tag_cdc_1 |
| T4-TR2 | rule | PUT 10 对象共享 2 公共标签 → obj=10, tag=2, HAS_TAG=20, dedup 标签顶点正确 | count 匹配 | tag_shared_dedup |
| T4-TR3 | rule | PUT 覆盖原对象 tags 变化 → 旧 HAS_TAG 删除 + 新边创建 | diff=correct | tag_update |
| T4-TR4 | rule | Tag 顶点 `tag:contentType/application%2Fpdf` URL 编码 roundtrip 一致 | 一致 20 场景 | tag_url_encode |
| T4-TR5 | rule | Graph Writer 失败（超时/熔断）→ CDC event 重试 3 次后写入 dead-letter-queue（队列可观测） | DLQ count=1 | graph_write_retry_dlq |
| T4-TR6 | rule | 反向查询 GraphQL `objectsByTag(k:"project",v:"finance") LIMIT 20` → 返回 obj.uri 全部 S3 HEAD 200 | HEAD 20/20 200 | tag_reverse_s3_head |
| T4-TR7 | rule | mox_tag2graph_lag_ms 指标：1000 次 PUT → P99 lag ≤ 500 ms（mock graph writer） | p99 ≤ 500 ms | tag2graph_latency |
| T4-TR8 | rule | DELETE 对象 → obj 顶点 soft-deleted=1，HAS_TAG 边 archived_at=now | 2 属性正确 | obj_delete_archive |
| T4-TR9 | rule | Tag key 大小写规范化（"Content-Type" → "content_type"）+ 非法字符过滤 | 规范化 15 场景 | tag_norm |
| T4-TR10 | rule | 批量 PUT 1000 对象 + tags 总 3450 → Graph 写入 batch_size=64 幂等无重复 | edge count exact | tag_batch_put |
| T4-TR11 | rule | 融合审计链：每次 ObjectTagged 都作为 AuditChain::TagApplied record 链式追加 | chain_len 增长正确 | fusion_audit |
| T4-TR12 | rule | miji_level 标签自动同步：obj.miji_level=3 → obj 顶点带属性 + tag `level:3` 自动 | 属性同步 | miji_propagate |
| T4-TR13 | rule | 用户自定义标签超过 50 → 截断告警 + 取前 50；审计链 record TagTruncated | truncation + audit | tag_limit |
| T4-TR14 | rule | 默认标签开关 `--no-default-tags` 启动：size_bucket 等默认不注入 | 仅自定义生效 | default_tags_switch |

### Completion Evidence
_（待实施时填写）_

---

## Task 5: LegalHold 法规保留锁 + 密级 4 级 Bell-LaPadula 裁决

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | AC-R5, AC-U5 |
| Dependencies | Task 1 + 既有 hash_chain |

### 产物
- `mox-compliance/src/legal_hold.rs`：LegalHold { placed_by, placed_at, hold_until } + 校验
- `mox-compliance/src/miji.rs`：MijiLevel(u8) + Clearance(u8) + BellLaPadula judge_read / judge_write
- Object 元数据扩展（通过 mox-domain-abstractions ObjectMeta trait）
- 审计链 3 新 record 类型：LegalHoldDenied / MijiAccessDenied / LegalHoldPlaced
- CLI：`mox legal-hold put|release` + `mox miji set|inspect`

### Task-local TR（≥ 18）
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T5-TR1 | rule | 对象放置 LegalHold(hold_until=+30day) → DELETE 返回 412 + AuditChain::LegalHoldDenied | 412 | lh_delete_denied |
| T5-TR2 | rule | LegalHold 期间 PUT overwirte 返回 412；同 key 不能覆盖 | 412 | lh_put_denied |
| T5-TR3 | rule | hold_until = 今日；立即 DELETE → 成功（刚好到期） | 204 OK | lh_expired_release |
| T5-TR4 | rule | release legal-hold → 删除+覆盖均成功 | 成功 | lh_release_clear |
| T5-TR5 | rule | 用户 clearance=2(秘密) 读 miji=3(机密) → 403 + AuditChain::MijiAccessDenied(simple_security_upward_read) | 403 | miji_read_up_denied |
| T5-TR6 | rule | clearance=3 → read miji=2(秘密) → 200（下读允许） | 200 | miji_read_down_ok |
| T5-TR7 | rule | clearance=3 → write miji=2 → 403（*-Property: 禁止高写低） | 403 | miji_write_star_down_denied |
| T5-TR8 | rule | clearance=2 → write miji=2 → 200（同级写允许） | 200 | miji_write_same_ok |
| T5-TR9 | rule | clearance=1(内部) → write miji=3 → 200（低写高，*-Property 允许上写） | 200 | miji_write_up_ok |
| T5-TR10 | rule | `enforce_miji=false` 关闭裁决：所有访问允许 + 审计链 MijiEnforceDisabled 记录 | allow+audit | miji_off |
| T5-TR11 | rule | 批量 100 对象 × 4 miji 档 × 4 clearance 档 = 1600 裁决正确 | 1600/1600 | miji_matrix |
| T5-TR12 | rule | LegalHold + Miji 联合：LH 对象即使 clearance 最高也不能删（LH 先于 miji 判定） | 412 优先级高 | lh_miji_union |
| T5-TR13 | rule | 新 3 类 record 写入 hash_chain → 链式完整性校验通过（100 连续块） | integrity=ok 100 | compliance_audit_chain |
| T5-TR14 | rule | 指标 mox_miji_denied_total / mox_legal_hold_blocked_total 每类拒绝 +1 | counters 正确 | compliance_metrics |
| T5-TR15 | rule | CLI `legal-hold put --hold-until` 参数非法日期 → UsageError；合法 → placed=true 200 | cli 2 场景 | lh_cli |
| T5-TR16 | rule | CLI `miji set --level 0` 非法（要求 1..4） → UsageError；set/get roundtrip 1..4 正确 | cli 4 场景 | miji_cli |
| T5-TR17 | rule | 密级裁决拒绝时响应头 `X-Mox-Deny-Reason` 语义正确（不泄露对象是否存在） | reason 语义 | deny_reason_semantic |
| T5-TR18 | rule | STS AssumeRole 传递 clearance 到临时凭证：session_token 解码后 clearance ≤ 用户原始（防提权） | no_privilege_escalation | sts_clearance_flow |

### Completion Evidence
_（待实施时填写）_

---

## Task 6: FSHC 磁盘健康检测 + Mountpath 热插拔 + Rebalance

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | medium |
| AC 覆盖 | AC-R9 |
| Dependencies | Task 1 + Task 2 (EC rebuild) |

### TR（≥ 10）
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T6-TR1..10 | rule ×10 | 健康扫描 OK → Healthy；连续 3 读失败 → Faulty；attach/detach 列表变化；enable/disable 生效；faulty→rebalance→EC rebuild count++；mountpath 目录嵌套检测拒绝；磁盘 100% 满时写入触发 Quota 与 FSHC 降级；disable 后 PUT 被拒绝 400；mountpath metrics exporter 9 字段；双路径（Linux / Windows 盘符）路径分隔符兼容 | 10/10 | fshc suite |

### Completion Evidence
_（待实施时填写）_

---

## Task 7: ETL WASM 插件框架（inline-get / inline-put / offline xaction）

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | medium |
| AC 覆盖 | AC-R8 |
| Dependencies | Task 1 (registry) |

### TR（≥ 10）
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T7-TR1..10 | rule ×10 | md5.wasm 注册+inline-get；uppercase text wasm；offline xaction 桶 10 对象处理完成；未注册 etl=xxx GET 400；wasm OOM 超时 1s 熔断；ctx.bucket/ctx.uri 读取正确；inline-put 预处理压缩；离线 bucket 源→目的 SHA 一致；feature="no-wasm" 禁用 wasm 所有调用报错提示友好；WASM ABI string 截断保护 | 10/10 | etl wasm suite |

### Completion Evidence
_（待实施时填写）_

---

## Task 8: 单二进制 mox-server all-in-one CLI（clap）

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | AC-R4, AC-U4 |
| Dependencies | Task 1,2,4,5,6,7 基本可用 |

### TR（≥ 10）
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T8-TR1 | rule | `mox --help` 打印子命令列表 server/ec/mount/legal-hold/miji/bench/etl | 7 子命令存在 | cli help |
| T8-TR2 | rule | `server --single-node --data-dir ./x --port 18080` 启动；GET /health 200；GET /metrics 含 9 新指标 | 200 OK + metrics | single-node smoke |
| T8-TR3 | rule | `--single-node` 冷启动（到 health ready）< 2.5 s（CI 放宽到 4 s） | < 4 s | startup_latency |
| T8-TR4 | rule | `ec profile add --bucket b1 --data 4 --parity 2` → list 列出；非法 data=1 → error | 增 + 校验 | ec CLI |
| T8-TR5 | rule | `mount attach /mnt/x1` + list + detach → 列表正确 | attach/list/detach | mount CLI |
| T8-TR6 | rule | `legal-hold put --hold-until 2040-01-01` → inspect 返回 hold=true | 双向一致 | lh CLI |
| T8-TR7 | rule | `miji set --level 3` → get 返回 3；非法 level 报错 | 双向 + 校验 | miji CLI |
| T8-TR8 | rule | `bench 1KB --n 100 --concurrency 4` 跑通输出 JSON ops/s 字段 | exit 0 + json 字段 | bench smoke |
| T8-TR9 | rule | mox.toml 配置文件读入（[server] port=18080，[storage] mountpaths=["/mnt/a"]）→ 启动生效 | 生效 | config_file |
| T8-TR10 | rule | SIGTERM/SIGINT 优雅关闭：WAL flush + metric persist + 指标 shutdown_time_us | graceful | graceful_shutdown |

### Completion Evidence
_（待实施时填写）_

---

## Task 9: P99 观测仪表 + CRC Read-after-Write 一致性

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | medium |
| AC 覆盖 | AC-R6, AC-R10 |
| Dependencies | Task 1, 5 |

### TR（≥ 20 组合 → 取 ≥ 10 rule）
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T9-TR1..10 | rule ×10 | `GET /metrics` 9 指标名存在 + `mox_obj_put_latency_us` 100 次 PUT histogram >0；审计链取证端点 200；100 个 block 导出 CSV integrity=1；Read-after-Write 100 对象 10 线程 GET etag=；Client 端 crc64 错 → 400；服务端 crc64 返回头 header 存在；乱序 part multipart 完成后 CRC 聚合正确；deny reason header 不泄漏；`GET /ops/audit/chain?format=html` 页脚带 hash_chain signature | 10/10 | obs + crc suite |

### Completion Evidence
_（待实施时填写）_

---

## Task 10: Bench 三档 (1KB/1MB/1GB) + Deploy 双路径

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | medium |
| AC 覆盖 | AC-R7, AC-R11 |
| Dependencies | Task 8 (CLI), Task 2 (EC) |

### TR（≥ 12）
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T10-TR1 | rule | 1KB 档：100000 ops，输出 ops/s> 字段；（CI 无 NVMe 时放松为 >0，基准线不绑定） | pass with fields | bench_1k |
| T10-TR2 | rule | 1MB 档：n=1000 输出 throughput_MB/s >0 | pass | bench_1m |
| T10-TR3 | rule | 1GB 档：n=20，p99(ms) 字段存在 | pass | bench_1g |
| T10-TR4 | rule | 1MB 档 EC off vs EC 4+2 吞吐比：EC 吞吐 / EC_off 吞吐 ≥ 0.85（即开销 ≤15%） | ratio ≥ 0.85 | ec_overhead |
| T10-TR5 | rule | benchmark JSON 报告写入 ./bench-report.json 字段齐全（9 字段） | 9/9 | bench_report_schema |
| T10-TR6 | rule | deploy standalone：`mox server --single-node --no-k8s` + `/health` 200 | 200 | deploy_smoke_standalone |
| T10-TR7 | rule | helm lint mox/（如无 helm CLI 则跳过当 pass）+ helm template --set standalone=false 至少 12 K8s 资源（Deployment/StatefulSet/Svc/Pdb/Hpa/Cm/Secret/SA/RBAC） | ≥ 12 kinds | helm_template |
| T10-TR8 | rule | 单机部署 + PUT/GET/Delete lifecycle 往返（对象 10 + 标签出图 + 审计链增长） | end-to-end pass | standalone_e2e |
| T10-TR9 | rule | TLS enable：`--tls-cert ./c1.pem --tls-key ./k1.pem`（自签）启动 https → curl -k 200 | 200 | tls_smoke |
| T10-TR10 | rule | 配置文件 + CLI 覆盖优先级：CLI > ENV > TOML file > 默认 | 3 级优先级正确 | config_priority |
| T10-TR11 | rule | bench warmup 选项 `--warmup 5s`：warmup 不计入报告指标 | report≠warmup_count | bench_warmup |
| T10-TR12 | rule | 文档 README-mox-server.md（嵌入 deploy docs）：含 "60 秒单机跑通" 步骤 + "helm 一键" 步骤；步骤能可重复 | smoke manual pass | deploy_docs |

### Completion Evidence
_（待实施时填写）_

---

## Task 11: T21 E2E ≥ 1237 tests 汇总 + 700 参数化 Harness

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | AC-R12 |
| Dependencies | Task 2-10 所有 tests 生成 harness + 既有 T10/T11/T17/T19 报告 |

### TR
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T11-TR1 | rule | T10(118)+T11(126)+T17(154)+T19(719) + 新增 Tasks 2-10 tests(16+24+14+18+10+10+10+20+12 = 134) → Total = 1117+134 = 1251 ≥ 1237，fail ≤ 2 | ≥ 1237 ∧ fail ≤ 2 | t21/report.json |
| T11-TR2 | rule | 参数化 Ops Harness 新增：HA(2) × Net(2 Public-Only / Tri) × EC(2 off / 4+2) × ETL(2) × Miji(4) = 2×2×2×2×4 = 64 × Stages(11 ops) ≈ 704 ≥ 700 | ≥ 700 | harness_count |

### Completion Evidence
_（待实施时填写）_

---

## Task 12: Rubric 汇总 + Grade S 最终验收

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | AC-U1..U8 (rubric) → Grade S ≥ 90 |
| Dependencies | Tasks 1-11 completion evidence |

### Rubric 加权表
| Rubric 项目 | 权重 | 指标来源 |
|---|---|---|
| U1 EC + Self-Heal 工程质量 | 15% | EC manifest + rebuild review + T2 TRs |
| U2 HTTP3 + Multipart + 三网 | 15% | T3 TR22-24 + SDK 对齐 |
| U3 云盘×图融合体验 | 20% | T4-TR1..TR14 + 反向回查 HEAD |
| U4 单二进制部署体验 | 10% | T8 启动时间 + 命令 UX 走查 |
| U5 政企合规体验 | 20% | T5 Matrix 1600 + 审计链 9×9 |
| U6 观测 + 取证仪表 | 8% | T9 + Grafana dashboard JSON |
| U7 ETL + Bench + FSHC + 部署 | 12% | T6/T7/T10 综合流程走查 |
| **合计** | **100%** | Grade ≥ 90 = S |

### TR
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T12-TR1 | rubric | U1..U8 单项分别 ≥ (92/90/92/90/94/88/86)，加权合计 ≥ 90 Grade S | Weighted ≥ 90 | rubric-t21-all.json |

### Completion Evidence
_（待实施时填写）_
