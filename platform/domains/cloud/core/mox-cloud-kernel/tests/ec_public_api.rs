// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! C1: mox-cloud-kernel 公共 API 黑盒集成测试 —— Reed-Solomon 编解码面。
//!
//! 与 crate 内联 `#[cfg(test)]` 单测互补：这里只通过 crate 对外 re-export 的
//! 公共 API 驱动（`mox_cloud_kernel::*`），验证真实消费者视角的契约，
//! 防止内部重构导致公共签名/行为漂移。

use mox_cloud_kernel::{
    reed_solomon::PathChoice, shard_size_for, EcProfile, ReedSolomon2Plus1, ReedSolomonEngine,
    RSError,
};

/// 构造确定性的伪随机载荷（避开对 rand 的依赖，保证测试可复现）。
fn payload(len: usize, seed: u8) -> Vec<u8> {
    (0..len).map(|i| seed.wrapping_mul(31).wrapping_add((i as u8).wrapping_mul(7))).collect()
}

#[test]
fn encode_decode_roundtrip_4plus2_lose_two() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let data = payload(4096 + 17, 0xA5);
    let shards = engine.encode(&profile, &data).unwrap();
    assert_eq!(shards.len(), 6, "4+2 must produce 6 shards");

    // 丢失 1 个数据分片 + 1 个校验分片，仍可完整重建。
    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    slots[1] = None;
    slots[5] = None;
    let recovered = engine.decode_reconstruct(&profile, &slots, data.len()).unwrap();
    assert_eq!(recovered, data, "4+2 lose-two roundtrip must reproduce original bytes");
}

#[test]
fn encode_decode_roundtrip_12plus4_large_payload() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(12, 4).unwrap();
    // 1 MiB + 3 字节：覆盖非对齐尾部（div_ceil 补齐路径）。
    let data = payload(1024 * 1024 + 3, 0x3C);
    let shards = engine.encode(&profile, &data).unwrap();
    assert_eq!(shards.len(), 16);

    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    for i in [2usize, 7, 11, 15] {
        slots[i] = None; // 丢失 3 数据 + 1 校验
    }
    let recovered = engine.decode_reconstruct(&profile, &slots, data.len()).unwrap();
    assert_eq!(recovered, data);
}

#[test]
fn decode_with_verification_detects_corruption_fail_closed() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let data = payload(2048, 0x77);
    let shards = engine.encode(&profile, &data).unwrap();
    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();

    // 丢 1 数据分片，但篡改 1 个剩余校验分片：必须 fail-closed 拒绝返回数据。
    slots[0] = None;
    if let Some(p) = slots[4].as_mut() {
        p[0] ^= 0xFF;
    }
    let err = engine.decode_with_verification(&profile, &slots, data.len()).unwrap_err();
    assert!(
        matches!(err, RSError::ReconstructionVerificationFailed(_)),
        "corrupted surplus shard must fail verification, got {err:?}"
    );
}

#[test]
fn decode_with_verification_passes_intact() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let data = payload(1024, 0x11);
    let shards = engine.encode(&profile, &data).unwrap();
    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    slots[2] = None; // 5 present > 4 data → 触发校验路径

    let recovered = engine.decode_with_verification(&profile, &slots, data.len()).unwrap();
    assert_eq!(recovered, data);
}

#[test]
fn reconstruct_shards_restores_missing_shards() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let data = payload(2048, 0x22);
    let original = engine.encode(&profile, &data).unwrap();
    let mut slots: Vec<Option<Vec<u8>>> = original.iter().cloned().map(Some).collect();
    slots[3] = None;
    slots[5] = None;

    let rebuilt = engine.reconstruct_shards(&profile, &slots).unwrap();
    assert_eq!(rebuilt.len(), 6);
    // 重建出的数据分片必须与原始编码一致。
    assert_eq!(rebuilt[0], original[0]);
    assert_eq!(rebuilt[3], original[3]);
    // 重建出的校验分片同样一致（校验分片由数据分片确定性导出）。
    assert_eq!(rebuilt[5], original[5]);
}

#[test]
fn scalar_and_auto_paths_bit_identical() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let data = payload(8192, 0x5A);
    let scalar = engine.encode_with_path(&profile, &data, PathChoice::Scalar).unwrap();
    let auto = engine.encode_with_path(&profile, &data, PathChoice::Auto).unwrap();
    assert_eq!(scalar, auto, "Scalar 与 Auto 编码输出必须逐字节一致");

    // 解码路径同样一致。
    let mut slots: Vec<Option<Vec<u8>>> = scalar.clone().into_iter().map(Some).collect();
    slots[0] = None;
    let r_scalar = engine
        .decode_with_path(&profile, &slots, data.len(), PathChoice::Scalar)
        .unwrap();
    let r_auto = engine
        .decode_with_path(&profile, &slots, data.len(), PathChoice::Auto)
        .unwrap();
    assert_eq!(r_scalar, r_auto);
    assert_eq!(r_scalar, data);
}

#[test]
fn invalid_profiles_rejected() {
    assert!(EcProfile::new(0, 1, 100).is_err(), "data_shards=0 must be rejected");
    assert!(EcProfile::new(1, 1, 100).is_err(), "data_shards=1 must be rejected");
    assert!(EcProfile::new(4, 0, 100).is_err(), "parity_shards=0 must be rejected");
    // min_obj_size=0 合法（仅约束 data/parity 下限），确认可正常构造。
    assert!(EcProfile::new(4, 2, 0).is_ok());
}

#[test]
fn too_many_missing_shards_is_rejected() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let data = payload(512, 0x99);
    let shards = engine.encode(&profile, &data).unwrap();
    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    for slot in slots.iter_mut().take(3) {
        *slot = None; // 丢失 3 > parity 2
    }
    let err = engine.decode_reconstruct(&profile, &slots, data.len()).unwrap_err();
    assert!(matches!(err, RSError::TooManyShardsMissing(_)));
}

#[test]
fn shard_size_for_ceil_division() {
    assert_eq!(shard_size_for(4, 1000), 250);
    assert_eq!(shard_size_for(4, 1001), 251);
    assert_eq!(shard_size_for(4, 0), 0);
    assert_eq!(shard_size_for(0, 100), 0);
}

#[test]
fn ec_profile_helpers() {
    let p = EcProfile::with_default_min_size(4, 2).unwrap();
    assert_eq!(p.total_shards(), 6);
    assert_eq!(p.data_shards, 4);
    assert_eq!(p.parity_shards, 2);
    assert!(p.is_replica(mox_cloud_kernel::DEFAULT_MIN_OBJ_SIZE - 1));
    assert!(!p.is_replica(mox_cloud_kernel::DEFAULT_MIN_OBJ_SIZE));
}

#[test]
fn legacy_2plus1_xor_engine_roundtrip() {
    use bytes::Bytes;
    let rs = ReedSolomon2Plus1;
    let d0 = Bytes::from(vec![0x01u8, 0x02, 0x03, 0x04]);
    let d1 = Bytes::from(vec![0x05u8, 0x06, 0x07, 0x08]);

    let encoded = rs.encode_2_1(&[d0.clone(), d1.clone()]).unwrap();
    assert_eq!(encoded[2], Bytes::from(vec![0x04u8, 0x04, 0x04, 0x0C]), "XOR parity");

    let decoded = rs
        .decode_2_1([Some(encoded[0].clone()), None, Some(encoded[2].clone())])
        .unwrap();
    assert_eq!(decoded[0], d0);
    assert_eq!(decoded[1], d1);

    // 两个分片同时缺失 → 拒绝。
    let err = rs.decode_2_1([None, None, Some(encoded[2].clone())]).unwrap_err();
    assert!(matches!(err, RSError::TooManyShardsMissing(_)));
}
