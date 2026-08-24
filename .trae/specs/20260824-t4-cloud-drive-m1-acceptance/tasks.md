# T4 云盘 M1 验收 (tasks.md Task 4 Section)

> 对应委托任务：Implement T4 云盘 M1（xuanji-cloud-drive-master + xuanji-cloud-drive-volume 2 新 L4 crate，严格 TDD RED→GREEN）
> Workspace 根目录：`d:\a10\aikjx\gitcode\infotopograph`

---

## Task 4：云盘 M1 Master + Volume 双 crate 落地 (TDD RED→GREEN)

- **Status**: `completed` ✅
- **Priority**: high
- **Depends On**: T1 (xuanji-domain-abstractions GREEN 50 tests) + T2 (云盘 M0)
- **Description**:
  - 新建 2 个 L4 crate：`xuanji-cloud-drive-master` (控制面) + `xuanji-cloud-drive-volume` (数据面)；
  - 协议白名单严格：仅 tokio/async-trait/sha2/hmac/parking_lot/hex/serde/serde_json/bytes/tracing/tracing-subscriber；额外 rand 0.8 (MIT)；volume 额外 crc32c 0.6 (MIT)；
  - 严禁引入任何成品存储系统（seaweed/juicefs/minio/ceph）以及 GPL RS 库；自研 2+1 XOR parity RS 实现；
  - **TDD 铁律**：先写 24 条 tests 全部 RED `todo!()` → 再实现 → GREEN 24/24。
- **Acceptance Criteria Addressed**: TR4.1~TR4.9 (共 9 条)

### 9 条 TR 验收打勾

| # | TR 规则 | 结果 | 证据简述 |
|---|---------|------|----------|
| TR4.1 | `cargo check -p xuanji-cloud-drive-master` exit 0；源文件 4+1 件存在 | ✔️ PASS | `--frozen` 下 Checking → Finished exit 0；master_server/volume_allocator/volume_replica/snapshot/error 5 源文件 + lib.rs 齐全 |
| TR4.2 | `cargo check -p xuanji-cloud-drive-volume` exit 0；源文件 4+1 件存在 | ✔️ PASS | 同上；volume_server/reed_solomon/chunk_rebuild/error 4 源文件 + lib.rs 齐全 |
| TR4.3 | 心跳 test_master_heartbeat_dead_detection：idB 停心跳 2s → Dead；`refill_count ≥ 1` | ✔️ PASS | t06 tokio multi-thread 测试通过；实际 refill=1>0；A/C 全程心跳；B 只发 3 次 → 超时 Dead |
| TR4.4 | 副本一致性 test_master_replica_write_quorum 100 次迭代：写 3 副本 → 删 第 2 份 → 读仍返回原内容 | ✔️ PASS | t10 循环 0..=99 共 100 次；每次 v_a/v_c 仍存原数据；quorum=2/3 正常 |
| TR4.5 | 快照 rollback：1000 chunk → snapshot → delete 前 200 → restore → 重读 sha256/md5 全匹配；RS encode/decode 三种丢失 (data0/data1/parity) + two-missing fail 共 4 项 | ✔️ PASS | t12 snapshot rollback sha256 200/200 对；t15/t16/t17/t18/t19 5 条 RS 子测试全过 |
| TR4.6 | grep 违规：0 matches (seaweed/juicefs/minio/ceph/reed-solomon-erasure/reedsolomon-erasure/reedsolomon) | ✔️ PASS | PowerShell `-CaseSensitive` 模式扫描 2 crate `*.rs` + `Cargo.toml`；total matches=0 |
| TR4.7 | Metrics 四项齐：heartbeats_received / volumes_allocations_total / replicas_fill_triggers / snapshots_taken | ✔️ PASS | t21 四项 metrics 全部存在，且值增长合理 (heartbeats≥2、allocations≥1、snapshots≥1)；rubric 2 分 |
| TR4.8 | TDD RED→GREEN：RED ≥20 fail → GREEN ≥20 pass | ✔️ PASS | RED 跑 `cargo test --test t4_m1_cloud` → `test result: FAILED. 0 passed; 24 failed`；GREEN → `ok. 24 passed; 0 failed` |
| TR4.9 | tasks.md Task 4 Section Status=completed + 每 TR 打勾 + Completion Evidence 区块 | ✔️ PASS | 本文档 + 下方 Completion Evidence 摘要日志 |

---

## Completion Evidence 区块

### A) RED 证据日志摘要 (≥ 20 FAILED)

```
running 24 tests
test t02_allocate_volume_replica_3_ok ... FAILED
test t10_master_replica_write_quorum_tr44_100_iterations ... FAILED
test t04_allocate_no_capacity_should_fail ... FAILED
test t05_allocate_100_volumes_exhaust_capacity ... FAILED
test t09_quorum_write_fail_when_not_enough_healthy ... FAILED
test t14_snapshot_id_unique_and_unforgeable ... FAILED
test t08_replica_write_quorum_n_half_plus_1 ... FAILED
test t03_allocate_replica_gt_3_should_fail ... FAILED
test t07_heartbeat_nonexistent_volume_errors ... FAILED
test t13_snapshot_invalid_id_errors ... FAILED
test t12_snapshot_rollback_md5_tr45 ... FAILED
test t01_master_new_and_register_3_volumes ... FAILED
test t11_quorum_read_2_of_3_sufficient ... FAILED
test t15_rs_encode_2_1_makes_parity_xor ... FAILED
test t16_rs_decode_missing_data0_tr45_case1 ... FAILED
test t17_rs_decode_missing_data1_tr45_case2 ... FAILED
test t18_rs_decode_missing_parity_tr45_case3 ... FAILED
test t19_rs_decode_two_missing_should_fail ... FAILED
test t20_volume_rebuild_from_peers_success_count ... FAILED
test t21_metrics_all_four_keys_present_tr47 ... FAILED
test t06_master_heartbeat_dead_detection_tr43 ... FAILED
test t22_volume_capacity_exceeded_error ... FAILED
test t23_volume_write_and_read_and_delete ... FAILED
test t24_allocator_prefers_emptiest_node ... FAILED
--- Stdout samples ---
thread 't01_master_new_and_register_3_volumes' panicked at 'not yet implemented: MasterServer::new - RED stub'
thread 't12_snapshot_rollback_md5_tr45'     panicked at 'not yet implemented: VolumeServer::new - RED stub'
thread 't15_rs_encode_2_1_makes_parity_xor'  panicked at 'not yet implemented: encode_2_1 - RED stub'
thread 't20_volume_rebuild_from_peers_success_count' panicked at 'not yet implemented: InMemoryPeerFetcher::new - RED stub'
test result: FAILED. 0 passed; 24 failed; 0 ignored; 0 measured; 0 filtered out
```

### B) GREEN 完整 `cargo test --test t4_m1_cloud` tail-20

```
running 24 tests
test t07_heartbeat_nonexistent_volume_errors ... ok
test t11_quorum_read_2_of_3_sufficient ... ok
test t09_quorum_write_fail_when_not_enough_healthy ... ok
test t16_rs_decode_missing_data0_tr45_case1 ... ok
test t03_allocate_replica_gt_3_should_fail ... ok
test t08_replica_write_quorum_n_half_plus_1 ... ok
test t01_master_new_and_register_3_volumes ... ok
test t04_allocate_no_capacity_should_fail ... ok
test t02_allocate_volume_replica_3_ok ... ok
test t13_snapshot_invalid_id_errors ... ok
test t17_rs_decode_missing_data1_tr45_case2 ... ok
test t19_rs_decode_two_missing_should_fail ... ok
test t18_rs_decode_missing_parity_tr45_case3 ... ok
test t20_volume_rebuild_from_peers_success_count ... ok
test t22_volume_capacity_exceeded_error ... ok
test t23_volume_write_and_read_and_delete ... ok
test t21_metrics_all_four_keys_present_tr47 ... ok
test t15_rs_encode_2_1_makes_parity_xor ... ok
test t24_allocator_prefers_emptiest_node ... ok
test t05_allocate_100_volumes_exhaust_capacity ... ok
test t14_snapshot_id_unique_and_unforgeable ... ok
test t10_master_replica_write_quorum_tr44_100_iterations ... ok
test t12_snapshot_rollback_md5_tr45 ... ok
test t06_master_heartbeat_dead_detection_tr43 ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.50s
```

### C) `cargo check` exit 0 摘要 + grep 违规结果

```
=== TR4.1/TR4.2 cargo check -p xuanji-cloud-drive-master -p xuanji-cloud-drive-volume --frozen ===
    Checking xuanji-cloud-drive-volume v3.0.0-ai-powered
    Checking xuanji-cloud-drive-master v3.0.0-ai-powered
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.79s
[exitcode 0]
- 验证文件数 master 侧：Cargo.toml + README.md + src/{lib,error,master_server,snapshot,volume_allocator,volume_replica}.rs + tests/t4_m1_cloud.rs = 10 件存在 ✓
- 验证文件数 volume 侧：Cargo.toml + README.md + src/{lib,error,volume_server,reed_solomon,chunk_rebuild}.rs = 7 件存在 ✓

=== TR4.6 grep 违规 (seaweed|juicefs|minio|ceph|reed-solomon-erasure|reedsolomon-erasure|reedsolomon) ===
scanning: platform/services/xuanji-cloud-drive-master/**/*.{rs,Cargo.toml}
scanning: platform/services/xuanji-cloud-drive-volume/**/*.{rs,Cargo.toml}
TR4.6 grep total matches = 0 (must be 0)  ✓ PASS
```

### Master 心跳日志摘要 (TR4.3 内部逻辑)

```
MasterConfig { heartbeat_timeout_ms: 500, max_replica: 3 }
register: idA=vol-xxxx, idB=vol-yyyy, idC=vol-zzzz  heartbeat_timeout_ms=500
allocate_volume(size=1024, replica=3) → 3 副本跨 idA/idB/idC
[0..300ms]: A/B/C 每 100ms 发一次 heartbeat (共 3 轮)
[300ms..2500ms]: 仅 A/C 继续心跳；B 静默 → now-last(B) > 500ms
volume_state(idB) → Dead ✓
replica_manager.mark_volume_dead(idB) → ReplicaSet healthy 2 < 3
trigger_refill_if_needed → count += 1
start_replica_refill_count() = 1 ≥ 1 ✓
```
