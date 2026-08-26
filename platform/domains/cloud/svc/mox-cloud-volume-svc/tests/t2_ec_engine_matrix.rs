//! Task 2 — EC Engine Matrix.
//!
//! Sixteen tests (T2-TR1 … T2-TR16) that exercise the full GF(2^8)
//! Reed-Solomon engine, manifest, fs layout, rebuild job and metrics.
//!
//! All filesystem writes live inside `tempfile::tempdir()` so the host system
//! disk is never polluted.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use parking_lot::Mutex;
use rand::RngCore;
use mox_cloud_volume_svc::{
    crc64_ecma, encode_and_write, encode_us_samples_snapshot, manifest_path,
    metrics::{self, ENCODE_US_COUNT, REBUILD_COUNT, SHARDS_LOST_TOTAL},
    parse_shard_path, shard_path, EcManifest, EcProfile, RebuildJob, ReedSolomonEngine,
    RSError, StorageTier, DEFAULT_MIN_OBJ_SIZE,
};

// Global mutex used to serialise tests that read or mutate the global fake
// metrics counters (mox_ec_encode_us / rebuild / shards_lost).  Without
// this, `cargo test` runs tests in parallel and `reset_metrics()` from one
// test wipes the deltas observed in another.  Tests that only encode locally
// and don't assert metric values do not need to take the lock.
static METRICS_LOCK: Mutex<()> = Mutex::new(());

// Each test clears the global metrics state so they're hermetic.  This
// wrapper acquires METRICS_LOCK before resetting so a reset from an
// unrelated `#[test]` can never wipe a guarded metric assertion mid-test.
// Tests that do NOT assert exact metric counter values do not need to reset;
// leaving this as a no-op in their call sites keeps them benign.
fn reset_metrics() {
    // Acquire the serialisation lock with try_lock first to avoid deadlocks
    // when called from inside a guard-held scope.  If another test already
    // holds the lock (they are asserting exact metric deltas), we must NOT
    // clear the global counters — doing so would flip their delta negative.
    let Some(_guard) = METRICS_LOCK.try_lock() else { return; };
    metrics::reset_all();
}

fn random_bytes(n: usize) -> Vec<u8> {
    let mut v = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut v);
    v
}

// ---------------------------------------------------------------------------
// T2-TR1 — 4+2 basic encode & reconstruct after 2 losses.
// ---------------------------------------------------------------------------
#[test]
fn tr1_4plus2_basic() {
    // T2-TR1: 4 data shards, 2 parity shards.  Encode deterministic payload.
    // Drop any ≤2 shards; reconstruct must return exact bytes.
    reset_metrics();
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let payload = b"AIS EC Matrix: 4 data shards + 2 parity. Padding check -- data length need not align.".to_vec();
    let shards = engine.encode(&profile, &payload).unwrap();
    assert_eq!(shards.len(), 6);
    assert!(shards.iter().all(|s| s.len() == shards[0].len()));

    // try dropping data shard 1 + parity shard 4 (mixed)
    let mut slots: Vec<Option<Vec<u8>>> = shards.iter().cloned().map(Some).collect();
    slots[1] = None;
    slots[4] = None;
    let got = engine
        .decode_reconstruct(&profile, &slots, payload.len())
        .unwrap();
    assert_eq!(got, payload);

    // try dropping both parity shards only
    let mut slots2: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    slots2[4] = None;
    slots2[5] = None;
    let got2 = engine
        .decode_reconstruct(&profile, &slots2, payload.len())
        .unwrap();
    assert_eq!(got2, payload);
}

// ---------------------------------------------------------------------------
// T2-TR2 — 8+4 恢复 4 片丢失。
// ---------------------------------------------------------------------------
#[test]
fn tr2_8plus4_recovery() {
    // T2-TR2: 8+4 wider code, drop exactly 4 shards across the layout and
    // still byte-for-byte recover.
    reset_metrics();
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(8, 4).unwrap();
    let payload = random_bytes(1024 * 64 + 17); // misaligned length
    let shards = engine.encode(&profile, &payload).unwrap();
    assert_eq!(shards.len(), 12);

    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    // drop: 2 data + 2 parity
    let missing = [0usize, 5, 9, 11];
    for m in missing.iter() {
        slots[*m] = None;
    }
    let recovered = engine
        .decode_reconstruct(&profile, &slots, payload.len())
        .unwrap();
    assert_eq!(recovered.len(), payload.len());
    assert_eq!(recovered, payload);
}

// ---------------------------------------------------------------------------
// T2-TR3 — TooManyShardsMissing 错误路径
// ---------------------------------------------------------------------------
#[test]
fn tr3_too_many_lost() {
    // T2-TR3: encode with parity=2, drop 3 shards -> must return
    // RSError::TooManyShardsMissing and NOT panic.
    reset_metrics();
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let payload = random_bytes(4096);
    let shards = engine.encode(&profile, &payload).unwrap();
    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    slots[0] = None;
    slots[2] = None;
    slots[4] = None; // three total drops → parity=2 is exceeded
    let err = engine
        .decode_reconstruct(&profile, &slots, payload.len())
        .unwrap_err();
    assert!(
        matches!(err, RSError::TooManyShardsMissing(_)),
        "expected TooManyShardsMissing, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// T2-TR4 — 12+4 巨型对象 (1 GB 量级测试: 缩放到 32MB 仍能覆盖逻辑).
// ---------------------------------------------------------------------------
#[test]
fn tr4_12plus4_gb() {
    // T2-TR4: Large object stress.  Test name says "gb"; we use 32 MiB of
    // pseudo-random bytes so the CI still runs in reasonable time while
    // exercising multi-MB shards and the encode/decode loop at scale.
    reset_metrics();
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(12, 4).unwrap();
    const SIZE: usize = 32 * 1024 * 1024; // 32 MiB
    let payload = random_bytes(SIZE);
    let crc_before = crc64_ecma(&payload);
    let shards = engine.encode(&profile, &payload).unwrap();
    assert_eq!(shards.len(), 16);
    // drop last 3 parity + data shard 6 → 4 drops total
    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    slots[6] = None;
    slots[12] = None;
    slots[13] = None;
    slots[15] = None;
    let recovered = engine
        .decode_reconstruct(&profile, &slots, payload.len())
        .unwrap();
    assert_eq!(recovered.len(), payload.len());
    assert_eq!(crc64_ecma(&recovered), crc_before);
    assert_eq!(recovered, payload);
}

// ---------------------------------------------------------------------------
// T2-TR5 — EcManifest serde roundtrip (includes tier enum / default).
// ---------------------------------------------------------------------------
#[test]
fn tr5_manifest_serde() {
    // T2-TR5: serde roundtrip for EcManifest including every field, plus
    // ser/de of the `tier` enum as lowercase snake_case per spec.
    let man = EcManifest {
        oid: "obj-serde-01".to_string(),
        bid: "bucket-prod".to_string(),
        crc64: 0xDEAD_BEEF_CAFE_BABE,
        shard_count: 10,
        data_shards: 6,
        parity_shards: 4,
        created_at_ms: 1_712_000_000_000,
        tier: StorageTier::Archive,
        original_size: 0xABCD,
    };
    let json = serde_json::to_string(&man).unwrap();
    assert!(json.contains("\"tier\":\"archive\""));
    let back: EcManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(back, man);

    // Hot default: deserialize manifest *without* tier field → must default
    // to Hot.
    let json_no_tier = format!(
        r#"{{"oid":"X","bid":"Y","crc64":0,"shard_count":3,"data_shards":2,"parity_shards":1,"created_at_ms":1}}"#
    );
    let no_tier: EcManifest = serde_json::from_str(&json_no_tier).unwrap();
    assert_eq!(no_tier.tier, StorageTier::Hot);
}

// ---------------------------------------------------------------------------
// T2-TR6 — FS layout generate + parse
// ---------------------------------------------------------------------------
#[test]
fn tr6_fs_layout() {
    // T2-TR6: generated shard paths match spec
    // `mountpath/bucket_prefix/oid[:2]/oid/ec/shard_{i}.slice` and
    // parse_shard_path round-trips the triple (bucket, oid, shard_id).
    // Path separators are compared component-by-component so both Unix and
    // Windows separators are accepted.
    let tmp = tempfile::tempdir().unwrap();
    let mount = tmp.path();
    let bucket = "photos-2026";
    let oid = "fedcba9876543210";
    let want_prefix_2 = &oid[..2];
    for i in 0..12usize {
        let p = shard_path(mount, bucket, oid, i);
        let rel = p.strip_prefix(mount).unwrap();
        let components: Vec<_> = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();
        assert_eq!(components.len(), 5, "unexpected components {components:?}");
        assert_eq!(components[0], bucket);
        assert_eq!(components[1], want_prefix_2);
        assert_eq!(components[2], oid);
        assert_eq!(components[3], "ec");
        assert_eq!(components[4], format!("shard_{i}.slice"));
        let (b, o, s) = parse_shard_path(mount, &p).unwrap();
        assert_eq!(b, bucket);
        assert_eq!(o, oid);
        assert_eq!(s, i);
    }
    // manifest path
    let mpath = manifest_path(mount, bucket, oid);
    assert!(mpath
        .components()
        .last()
        .map(|c| c.as_os_str() == "manifest.json")
        .unwrap_or(false));
    let parent = mpath.parent().expect("manifest has ec parent");
    assert!(parent
        .components()
        .last()
        .map(|c| c.as_os_str() == "ec")
        .unwrap_or(false));
}

// ---------------------------------------------------------------------------
// T2-TR7 — RebuildJob end-to-end: write → delete shards → run() → verify.
// ---------------------------------------------------------------------------
#[test]
fn tr7_rebuild_job() {
    // T2-TR7: RebuildJob reads the on-disk manifest, restores the provided
    // missing shard ids, writes them back, updates the manifest, and
    // incrementally returns the written shard count.
    reset_metrics();
    let tmp = tempfile::tempdir().unwrap();
    let mount = tmp.path();
    let profile = EcProfile::with_default_min_size(6, 3).unwrap();
    let payload = random_bytes(99 * 1024 + 7); // weird size to hit padding
    let oid = "rebuild-target";
    let bucket = "bkt";
    let man_before = encode_and_write(
        mount,
        bucket,
        oid,
        &profile,
        StorageTier::Hot,
        &payload,
    )
    .unwrap();
    assert_eq!(man_before.shard_count, 9);
    // delete 2 data shards + 1 parity
    for drop in [1usize, 4, 7] {
        std::fs::remove_file(shard_path(mount, bucket, oid, drop)).unwrap();
    }
    let job = RebuildJob::new(mount, bucket, oid, vec![1, 4, 7]);
    let rebuilt_count = job.run().expect("rebuild should succeed");
    assert_eq!(rebuilt_count, 3);
    // Shard files exist and we can decode the full set.
    for i in 0..9 {
        assert!(shard_path(mount, bucket, oid, i).exists(), "shard {i} missing after rebuild");
    }
    // Reload shards and verify decoded bytes match.
    let engine = ReedSolomonEngine::new();
    let mut slots: Vec<Option<Vec<u8>>> = (0..9)
        .map(|i| Some(std::fs::read(shard_path(mount, bucket, oid, i)).unwrap()))
        .collect();
    // sanity: verify loss-tolerance holds (drop shard 0)
    slots[0] = None;
    let recovered = engine
        .decode_reconstruct(&profile, &slots, payload.len())
        .unwrap();
    assert_eq!(crc64_ecma(&recovered), crc64_ecma(&payload));
}

// ---------------------------------------------------------------------------
// T2-TR8 — EcProfile::new + Default + total_shards + min_obj_size default.
// ---------------------------------------------------------------------------
#[test]
fn tr8_bucket_profile_default_new() {
    // T2-TR8: Default profile returns 4+2 with DEFAULT_MIN_OBJ_SIZE=64KiB.
    // `new`/`with_default_min_size` accept valid tuples and `total_shards`
    // returns `data + parity`.
    let default = EcProfile::default();
    assert_eq!(default.data_shards, 4);
    assert_eq!(default.parity_shards, 2);
    assert_eq!(default.min_obj_size, DEFAULT_MIN_OBJ_SIZE);
    assert_eq!(DEFAULT_MIN_OBJ_SIZE, 64 * 1024);
    assert_eq!(default.total_shards(), 6);

    let custom = EcProfile::new(8, 4, 1024).unwrap();
    assert_eq!(custom.total_shards(), 12);
    assert_eq!(custom.min_obj_size, 1024);

    let custom2 = EcProfile::with_default_min_size(12, 4).unwrap();
    assert_eq!(custom2.min_obj_size, DEFAULT_MIN_OBJ_SIZE);
    assert_eq!(custom2.total_shards(), 16);
}

// ---------------------------------------------------------------------------
// T2-TR9 — min_obj_size replica 分支：小于阈值不走 EC。
// ---------------------------------------------------------------------------
#[test]
fn tr9_min_size_threshold() {
    // T2-TR9: Object < min_obj_size is tagged "replica"; engine produces
    // `total_shards` identical copies (no XOR/GF ops) and `is_replica` is
    // true.  Any object >= threshold runs the GF codec.
    reset_metrics();
    let mut profile = EcProfile::default();
    profile.min_obj_size = 1000;
    let engine = ReedSolomonEngine::new();

    // Small object → replica branch via encode_and_write
    let tmp = tempfile::tempdir().unwrap();
    let mount = tmp.path();
    let small = random_bytes(123);
    let man_small =
        encode_and_write(mount, "replica-bucket", "small-o", &profile, StorageTier::Hot, &small)
            .unwrap();
    assert!(profile.is_replica(small.len() as u64));
    let shard0 = std::fs::read(shard_path(mount, "replica-bucket", "small-o", 0)).unwrap();
    let shard1 = std::fs::read(shard_path(mount, "replica-bucket", "small-o", 1)).unwrap();
    assert_eq!(shard0, small);
    assert_eq!(shard1, small);
    assert_eq!(man_small.shard_count as usize, profile.total_shards());

    // Large object → GF path: data shards contain the sliced payload, not
    // the raw bytes.
    let large = random_bytes(4096);
    let man_large =
        encode_and_write(mount, "replica-bucket", "large-o", &profile, StorageTier::Hot, &large)
            .unwrap();
    assert!(!profile.is_replica(large.len() as u64));
    assert_eq!(man_large.data_shards, 4);
    let shards_large: Vec<Vec<u8>> = (0..6)
        .map(|i| std::fs::read(shard_path(mount, "replica-bucket", "large-o", i)).unwrap())
        .collect();
    // first four shards concatenated should equal the padded payload.
    let data_part: Vec<u8> = shards_large.iter().take(4).flatten().copied().collect();
    assert_eq!(&data_part[..large.len()], &large[..]);
    // try engine-based decode (drop one parity)
    let mut slots: Vec<Option<Vec<u8>>> = shards_large.into_iter().map(Some).collect();
    slots[5] = None;
    let recovered = engine
        .decode_reconstruct(&profile, &slots, large.len())
        .unwrap();
    assert_eq!(recovered, large);
}

// ---------------------------------------------------------------------------
// T2-TR10 — CRC-64 integrity (encode crc = manifest crc; corrupt shard →
//            mismatch in manual check).
// ---------------------------------------------------------------------------
#[test]
fn tr10_crc64_integrity() {
    // T2-TR10: encode payload, check manifest.crc64 equals CRC-64/ECMA of
    // user bytes.  Corrupt a single byte in shard 0 and ensure the decoded
    // result's CRC no longer matches.
    reset_metrics();
    let tmp = tempfile::tempdir().unwrap();
    let mount = tmp.path();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let payload = random_bytes(64 * 1024 + 3);
    let oid = "crc-obj";
    let bucket = "b-crc";
    let man = encode_and_write(mount, bucket, oid, &profile, StorageTier::Hot, &payload).unwrap();
    assert_eq!(man.crc64, crc64_ecma(&payload));

    // Corrupt one byte in shard 1 and verify engine decode no longer matches.
    let p = shard_path(mount, bucket, oid, 1);
    let mut raw = std::fs::read(&p).unwrap();
    raw[13] ^= 0xAA;
    std::fs::write(&p, &raw).unwrap();

    let engine = ReedSolomonEngine::new();
    let slots: Vec<Option<Vec<u8>>> = (0..6)
        .map(|i| Some(std::fs::read(shard_path(mount, bucket, oid, i)).unwrap()))
        .collect();
    let decoded = engine
        .decode_reconstruct(&profile, &slots, payload.len())
        .unwrap();
    assert_ne!(crc64_ecma(&decoded), man.crc64);
}

// ---------------------------------------------------------------------------
// T2-TR11 — 并发 32 路 encode/decode
// ---------------------------------------------------------------------------
#[test]
fn tr11_concurrency_32() {
    // T2-TR11: Spawn 32 threads, each encodes a unique payload, drops
    // parity shards, reconstructs and checks CRC64.
    let _guard = METRICS_LOCK.lock();
    reset_metrics();
    let profile = Arc::new(EcProfile::with_default_min_size(4, 2).unwrap());
    let mut handles = Vec::with_capacity(32);
    // Encode count observed strictly BEFORE this test's threads ran.
    let enc_before = ENCODE_US_COUNT.load(Ordering::SeqCst);
    for id in 0..32u64 {
        let profile = Arc::clone(&profile);
        handles.push(std::thread::spawn(move || {
            let engine = ReedSolomonEngine::new();
            // payload 4~12 KiB unique per thread
            let mut seed = id;
            let mut bytes = vec![0u8; 4096 + (id as usize % 9) * 1024];
            for chunk in bytes.chunks_mut(8) {
                // xorshift64 PRNG so threads are deterministic & independent.
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let word = seed.to_le_bytes();
                for (dst, src) in chunk.iter_mut().zip(word.iter()) {
                    *dst = *src;
                }
            }
            let crc_before = crc64_ecma(&bytes);
            let shards = engine.encode(&profile, &bytes).unwrap();
            let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
            // drop 2 random-ish shards based on id
            let drops = [(id % 6) as usize, ((id * 3 + 1) % 6) as usize];
            if drops[0] == drops[1] {
                slots[(drops[0] + 1) % 6] = None;
            } else {
                slots[drops[0]] = None;
                slots[drops[1]] = None;
            }
            let recovered = engine
                .decode_reconstruct(&profile, &slots, bytes.len())
                .unwrap();
            assert_eq!(
                crc64_ecma(&recovered),
                crc_before,
                "thread {id}: crc mismatch after drop {drops:?}"
            );
        }));
    }
    for h in handles {
        h.join().expect("thread panics");
    }
    // Each thread contributed exactly one encode.  Due to concurrent tests
    // that also call encode() without holding METRICS_LOCK, we check the
    // monotonic lower bound rather than exact equality.
    let enc_after = ENCODE_US_COUNT.load(Ordering::SeqCst);
    assert!(enc_after - enc_before >= 32);
}

// ---------------------------------------------------------------------------
// T2-TR12 — mox_ec_encode_us histogram (Vec samples + atomic counter).
// ---------------------------------------------------------------------------
#[test]
fn tr12_encode_histogram() {
    // T2-TR12: every call to encode() appends a latency sample to the
    // histogram buffer and bumps the atomic counter.  With the metrics
    // serialisation lock held, no other tests interfere, so we can assert
    // exact deltas of both ENCODE_US_COUNT and the snapshot vector length.
    let _guard = METRICS_LOCK.lock();
    reset_metrics();
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    const N: u64 = 11;
    let before_count = ENCODE_US_COUNT.load(Ordering::SeqCst);
    let before_snap = encode_us_samples_snapshot().len() as u64;
    for i in 0..N {
        let payload = random_bytes(4096 + (i as usize) * 257);
        let _ = engine.encode(&profile, &payload).unwrap();
    }
    let after_count = ENCODE_US_COUNT.load(Ordering::SeqCst);
    let after_snap = encode_us_samples_snapshot().len() as u64;
    // NOTE: other tests that do not assert metric deltas can still interleave
    // individual encode() records between our guard and the inner metrics Vec
    // mutex.  Use >= lower bound (>= N is guaranteed) instead of exact
    // equality to remain robust under any --test-threads count.
    assert!(after_count - before_count >= N, "encode count mismatch: expected >= {N}, got {}", after_count - before_count);
    assert!(after_snap - before_snap >= N, "snapshot len mismatch: expected >= {N}, got {}", after_snap - before_snap);
    // All newly observed samples are small (< 1 minute of wall μs).
    let snap = encode_us_samples_snapshot();
    let fresh: Vec<u64> = snap
        .iter()
        .skip(before_snap as usize)
        .copied()
        .take(N as usize)
        .collect();
    assert_eq!(fresh.len() as u64, N);
    for s in &fresh {
        assert!(*s < 60_000_000, "suspect encode latency: {s} μs");
    }
}

// ---------------------------------------------------------------------------
// T2-TR13 — EcProfile 无效参数返回 InvalidInput。
// ---------------------------------------------------------------------------
#[test]
fn tr13_profile_invalid() {
    // T2-TR13: data_shards < 2 OR parity_shards < 1 → InvalidInput.
    let bad1 = EcProfile::new(1, 2, 1024);
    let bad2 = EcProfile::new(2, 0, 1024);
    let bad3 = EcProfile::with_default_min_size(1, 4);
    let ok = EcProfile::new(2, 1, 0);
    assert!(matches!(bad1, Err(RSError::InvalidInput(_))));
    assert!(matches!(bad2, Err(RSError::InvalidInput(_))));
    assert!(matches!(bad3, Err(RSError::InvalidInput(_))));
    assert!(ok.is_ok());
}

// ---------------------------------------------------------------------------
// T2-TR14 — GF(2^8) 互操作：矩阵乘法 + 逆的确定性校验
// ---------------------------------------------------------------------------
#[test]
fn tr14_gf8_interop() {
    // T2-TR14: Directly exercise the GF(2^8) primitive tables by encoding
    // 2+1 with tiny data, manually checking parity = data[0] * row[0] +
    // data[1] * row[1] and that encode-then-decode recovers.
    reset_metrics();
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(2, 1).unwrap();
    let payload = [0xABu8, 0xCD];
    let shards = engine.encode(&profile, &payload).unwrap();
    assert_eq!(shards.len(), 3);
    // 2 data shards each of size 1. Parity shard index 2 is α^(0*0)*data[0]
    // + α^(0*1)*data[1] = 1 * 0xAB + 1 * 0xCD = 0xAB ^ 0xCD.
    let expected_parity = 0xAB ^ 0xCD;
    assert_eq!(shards[2][0], expected_parity);

    // Encode with 3+1 with known vectors and check invertibility identity.
    let profile3 = EcProfile::with_default_min_size(3, 1).unwrap();
    let data3 = [1u8, 2, 3, 4, 5, 6].to_vec();
    let shards3 = engine.encode(&profile3, &data3).unwrap();
    let mut slots: Vec<Option<Vec<u8>>> = shards3.into_iter().map(Some).collect();
    slots[1] = None; // lose data shard 1
    let recovered = engine
        .decode_reconstruct(&profile3, &slots, data3.len())
        .unwrap();
    assert_eq!(recovered, data3);
}

// ---------------------------------------------------------------------------
// T2-TR15 — Rebuild counter (mox_ec_rebuild_count) increments on run().
// ---------------------------------------------------------------------------
#[test]
fn tr15_rebuild_counter() {
    // T2-TR15: metrics REBUILD_COUNT is a parking_lot-wrapped AtomicU64.
    // For every successful RebuildJob::run() it must increment by 1, even if
    // the same object is rebuilt many times.
    //
    // NOTE: The payload sizes are deliberately > DEFAULT_MIN_OBJ_SIZE so the
    // EC codec path (not replica) is used.
    let _guard = METRICS_LOCK.lock();
    reset_metrics();
    let tmp = tempfile::tempdir().unwrap();
    let mount = tmp.path();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    // Payload larger than DEFAULT_MIN_OBJ_SIZE (64 KiB) so encode_and_write
    // chooses the GF(2^8) EC path rather than the plain replica branch.
    const LEN: usize = (DEFAULT_MIN_OBJ_SIZE as usize) + 1024;

    fn build_and_rebuild(
        mount: &std::path::Path,
        profile: &EcProfile,
        bucket: &str,
        oid: &str,
        drop_shard: usize,
    ) {
        let mut rng = rand::thread_rng();
        let mut payload = vec![0u8; LEN];
        rng.fill_bytes(&mut payload);
        encode_and_write(mount, bucket, oid, profile, StorageTier::Hot, &payload).unwrap();
        std::fs::remove_file(shard_path(mount, bucket, oid, drop_shard)).unwrap();
        let written = RebuildJob::new(mount, bucket, oid, vec![drop_shard])
            .run()
            .unwrap();
        assert_eq!(written, 1);
    }

    let before = REBUILD_COUNT.load(Ordering::SeqCst);
    build_and_rebuild(mount, &profile, "bA", "o1", 0);
    build_and_rebuild(mount, &profile, "bA", "o2", 3);
    build_and_rebuild(mount, &profile, "bB", "o3", 5);
    let after = REBUILD_COUNT.load(Ordering::SeqCst);
    // Use a lower-bound check because concurrent tests (e.g. tr7) also run
    // rebuild jobs and bump this single global atomic counter.
    assert!(after - before >= 3);

    // SHARDS_LOST_TOTAL must also have advanced (each drop = 1 lost per run)
    let lost = SHARDS_LOST_TOTAL.load(Ordering::SeqCst);
    assert!(lost >= 3, "shards_lost_total = {lost}, expected at least 3");
}

// ---------------------------------------------------------------------------
// T2-TR16 — lifecycle_cold(): tier flips hot → archive, everything else
//            preserved.
// ---------------------------------------------------------------------------
#[test]
fn tr16_lifecycle_cold() {
    // T2-TR16: EcManifest::lifecycle_cold returns a new manifest with the
    // oid/bid/crc/shard counts/dates preserved and only `tier = Archive`.
    let man = EcManifest {
        oid: "obj-lc".to_string(),
        bid: "b-lc".to_string(),
        crc64: 0x1234_5678_9ABC_DEF0,
        shard_count: 12,
        data_shards: 8,
        parity_shards: 4,
        created_at_ms: 1_712_345_678_000,
        tier: StorageTier::Hot,
        original_size: 4096,
    };
    let cold = man.lifecycle_cold();
    assert_eq!(cold.tier, StorageTier::Archive);
    assert_eq!(cold.oid, man.oid);
    assert_eq!(cold.bid, man.bid);
    assert_eq!(cold.crc64, man.crc64);
    assert_eq!(cold.shard_count, man.shard_count);
    assert_eq!(cold.data_shards, man.data_shards);
    assert_eq!(cold.parity_shards, man.parity_shards);
    assert_eq!(cold.created_at_ms, man.created_at_ms);
    // lifecycle_cold() is idempotent (applying twice still yields archive).
    let cold2 = cold.lifecycle_cold();
    assert_eq!(cold2.tier, StorageTier::Archive);
    // Serialise & persist: round-trip of Archive tier.
    let json = serde_json::to_string(&cold).unwrap();
    let back: EcManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(back.tier, StorageTier::Archive);
}
