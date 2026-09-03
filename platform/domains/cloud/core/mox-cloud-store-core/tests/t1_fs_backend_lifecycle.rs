// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 阶段1 集成测试：FS 后端生命周期（真实落盘、内容寻址、去重、GC、版本、KV）。
//!
//! 全部走 `tempfile`（Windows 兼容，无 mmap/POSIX-only API）。
//!
//! 注：`FsObjectStore` 同时实现 `ObjectStore` 与 `KvStore`（均有 `get`/`delete`），
//! 因此本文件对 FsObjectStore 的调用使用显式 trait 限定以避免方法解析歧义。

use bytes::Bytes;
use mox_base_store_core::{KvStore, ObjectStore, ObjectStreamWriter, StoreError};
use mox_cloud_store_core::{
    create_backend, sha256_hex, BackendKind, ContentDefinedChunker, FsObjectStore,
    GarbageCollector, StoreConfig, VersionManager,
};

fn tmp_store() -> (tempfile::TempDir, FsObjectStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = FsObjectStore::new(dir.path()).unwrap();
    (dir, store)
}

#[tokio::test]
async fn t1_full_lifecycle_roundtrip() {
    let (_d, store) = tmp_store();
    let key = "kb/guide/架构手册.md";
    let content = "# Mox 云盘知识库混合架构\n- 自研为主体\n- 参考 RustFS 优秀算法";
    let data = Bytes::from(content);

    // put → head → get → range → exists → delete
    let obj = ObjectStore::put(&store, key, "text/markdown", data.clone()).await.unwrap();
    assert_eq!(obj.size_bytes, data.len() as u64);
    assert_eq!(obj.sha256.as_deref(), Some(sha256_hex(content.as_bytes()).as_str()));

    let head = ObjectStore::head(&store, key).await.unwrap();
    assert_eq!(head.content_type, "text/markdown");

    let got = ObjectStore::get(&store, key).await.unwrap();
    assert_eq!(got, data);

    let range = ObjectStore::get_range(&store, key, 2, 8).await.unwrap();
    assert_eq!(&range[..], &content.as_bytes()[2..10]);

    assert!(ObjectStore::exists(&store, key).await.unwrap());
    ObjectStore::delete(&store, key).await.unwrap();
    assert!(!ObjectStore::exists(&store, key).await.unwrap());
    assert!(matches!(ObjectStore::get(&store, key).await, Err(StoreError::NotFound { .. })));
}

#[tokio::test]
async fn t1_content_addressing_dedup_and_refcount() {
    let (_d, store) = tmp_store();
    let payload = Bytes::from_static(b"identical bytes");
    ObjectStore::put(&store, "a.txt", "text/plain", payload.clone()).await.unwrap();
    ObjectStore::put(&store, "b.txt", "text/plain", payload.clone()).await.unwrap();

    // 同一内容只存一份 chunk，引用计数 = 2
    let sha = sha256_hex(b"identical bytes");
    let cp = store.chunk_path(&sha);
    assert!(tokio::fs::try_exists(&cp).await.unwrap());
    let rc = store.refcount(&sha).await.unwrap();
    assert_eq!(rc, 2);

    // 删除一份后引用降为 1，数据仍可读
    ObjectStore::delete(&store, "a.txt").await.unwrap();
    assert_eq!(store.refcount(&sha).await.unwrap(), 1);
    assert_eq!(&ObjectStore::get(&store, "b.txt").await.unwrap()[..], b"identical bytes");
}

#[tokio::test]
async fn t1_stream_writer_via_trait_and_fs_writer() {
    let (_d, store) = tmp_store();
    // ObjectStreamWriter trait 通道
    let h = store.open_writer("big.bin", "application/octet-stream").await.unwrap();
    for part in [b"alpha-".as_slice(), b"beta-", b"gamma"] {
        store.write(&h, Bytes::copy_from_slice(part)).await.unwrap();
    }
    let obj = store.close(h).await.unwrap();
    assert_eq!(obj.size_bytes, 16);
    assert_eq!(&ObjectStore::get(&store, "big.bin").await.unwrap()[..], b"alpha-beta-gamma");

    // FsStreamWriter 增量哈希一致（复用 MPU 落盘路径）
    let mut w = mox_cloud_store_core::FsStreamWriter::open(_d.path()).await.unwrap();
    w.write(Bytes::from_static(b"hello ")).await.unwrap();
    w.write(Bytes::from_static(b"world")).await.unwrap();
    let res = w.finish().await.unwrap();
    assert_eq!(res.sha256, sha256_hex(b"hello world"));
    tokio::fs::remove_file(&res.tmp_path).await.unwrap();
}

#[tokio::test]
async fn t1_cdc_chunking_roundtrip() {
    // 确定性数据：内容定义分块可重组为原文
    let mut x = 12345u64;
    let data: Vec<u8> = (0..50_000)
        .map(|_| {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (x >> 33) as u8
        })
        .collect();
    let mut chunker = ContentDefinedChunker::new(4096, 1024, 32_768);
    let mut chunks = Vec::new();
    for block in data.chunks(3333) {
        chunker.feed(block, &mut chunks);
    }
    let tail = chunker.finish();
    if !tail.is_empty() {
        chunks.push(tail);
    }
    assert!(chunks.len() >= 2, "大载荷应产出多个分块");
    let mut reassembled = Vec::new();
    for c in &chunks {
        reassembled.extend_from_slice(c);
    }
    assert_eq!(reassembled, data);
}

#[tokio::test]
async fn t1_gc_dry_run_and_real_collect() {
    let (_d, store) = tmp_store();
    ObjectStore::put(&store, "keep.txt", "text/plain", Bytes::from_static(b"keep me")).await.unwrap();
    ObjectStore::put(&store, "gone.txt", "text/plain", Bytes::from_static(b"delete me")).await.unwrap();
    ObjectStore::delete(&store, "gone.txt").await.unwrap();

    let gc = GarbageCollector::with_grace(_d.path(), 0);

    // dry-run：识别候选但不删除
    let report = gc.collect(true).await.unwrap();
    assert_eq!(report.hard_deleted, 1, "dry-run 应识别 1 个候选");
    assert_eq!(report.soft_purged, 0);
    assert!(report.chunks_scanned >= 2, "应扫描全部 chunk，实际 {}", report.chunks_scanned);
    assert!(report.bytes_freed >= 9, "dry-run 也应报告可释放字节");
    let gone_sha = sha256_hex(b"delete me");
    assert!(tokio::fs::try_exists(store.chunk_path(&gone_sha)).await.unwrap());

    // 实跑：物理删除，保留对象不受影响
    let report = gc.collect(false).await.unwrap();
    assert_eq!(report.hard_deleted, 1);
    assert_eq!(report.soft_purged, 0);
    assert!(!tokio::fs::try_exists(store.chunk_path(&gone_sha)).await.unwrap());
    assert!(tokio::fs::try_exists(store.chunk_path(&sha256_hex(b"keep me"))).await.unwrap(), "保留对象 chunk 不应被删除");
    assert_eq!(&ObjectStore::get(&store, "keep.txt").await.unwrap()[..], b"keep me");

    // 幂等：二次 GC 无可回收
    let report = gc.collect(false).await.unwrap();
    assert_eq!(report.hard_deleted, 0);
}

#[tokio::test]
async fn t1_versioning_zero_copy_restore() {
    let (_d, store) = tmp_store();
    let vm = VersionManager::new(_d.path());
    let v1 = vm
        .save_version(&store, "doc/x", "text/plain", Bytes::from_static(b"v1 content"), serde_json::json!({"note": "初始"}))
        .await
        .unwrap();
    let v2 = vm
        .save_version(&store, "doc/x", "text/plain", Bytes::from_static(b"v2 content"), serde_json::json!({"note": "更新"}))
        .await
        .unwrap();

    let versions = vm.list_versions("doc/x").await.unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version, 1);
    assert_eq!(versions[1].version, 2);

    // 版本元数据校验
    assert_eq!(versions[0].size_bytes, b"v1 content".len() as u64);
    assert_eq!(versions[0].content_type, "text/plain");
    assert!(versions[0].created_ms > 0, "应记录创建时间");
    assert_eq!(versions[0].meta["note"], "初始");
    assert_ne!(v1.sha256, v2.sha256, "不同内容 → 不同内容寻址哈希");

    // 零拷贝恢复：仅新增引用，不复制数据
    let restored = vm.restore(&store, "doc/x", 1, "kb/restored.txt").await.unwrap();
    assert_eq!(&ObjectStore::get(&store, "kb/restored.txt").await.unwrap()[..], b"v1 content");
    assert_eq!(restored.sha256.as_deref(), Some(v1.sha256.as_str()));
    assert_eq!(store.refcount(&v1.sha256).await.unwrap(), 2, "内部版本引用 + 恢复对象引用 = 2");
    assert_eq!(store.refcount(&v2.sha256).await.unwrap(), 1, "v2 仅内部引用");

    // 恢复内容与源版本数据字节级一致
    assert_eq!(&ObjectStore::get(&store, "kb/restored.txt").await.unwrap()[..], b"v1 content");
}

#[tokio::test]
async fn t1_kv_persistence_across_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let store = FsObjectStore::new(dir.path()).unwrap();
        KvStore::put(&store, "bucket:meta", Bytes::from_static(b"{\"v\":1}")).await.unwrap();
    }
    let store = FsObjectStore::new(dir.path()).unwrap();
    assert_eq!(&KvStore::get(&store, "bucket:meta").await.unwrap().unwrap()[..], b"{\"v\":1}");
}

#[tokio::test]
async fn t1_backend_factory_fs() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = StoreConfig {
        kind: BackendKind::Fs,
        data_dir: dir.path().to_path_buf(),
        ..Default::default()
    };
    let be = create_backend(&cfg).unwrap();
    assert_eq!(be.kind, BackendKind::Fs);
    be.object.put("k", "text/plain", Bytes::from_static(b"v")).await.unwrap();
    assert_eq!(&be.object.get("k").await.unwrap()[..], b"v");
    // 未启用 s3 feature 时，S3 后端应返回清晰错误
    let r = create_backend(&StoreConfig {
        kind: BackendKind::S3,
        data_dir: dir.path().to_path_buf(),
        ..Default::default()
    });
    assert!(r.is_err());
}

#[tokio::test]
async fn t1_atomic_write_no_partial_files() {
    let dir = tempfile::tempdir().unwrap();
    let store = FsObjectStore::new(dir.path()).unwrap();
    for i in 0..50 {
        ObjectStore::put(
            &store,
            &format!("batch/{i}.json"),
            "application/json",
            Bytes::from(format!("{{\"i\":{i}}}")),
        )
        .await
        .unwrap();
    }
    // 无 .tmp 残留
    let mut stack = vec![dir.path().join("objects")];
    let mut tmp_count = 0u32;
    while let Some(d) = stack.pop() {
        let mut rd = tokio::fs::read_dir(&d).await.unwrap();
        while let Ok(Some(ent)) = rd.next_entry().await {
            let is_dir = match ent.file_type().await {
                Ok(ft) => ft.is_dir(),
                Err(_) => std::fs::metadata(ent.path()).map(|m| m.is_dir()).unwrap_or(false),
            };
            if is_dir {
                stack.push(ent.path());
            } else if ent.file_name().to_string_lossy().starts_with(".tmp-") {
                tmp_count += 1;
            }
        }
    }
    assert_eq!(tmp_count, 0, "原子写不应残留临时文件");
    for i in 0..50 {
        let got = ObjectStore::get(&store, &format!("batch/{i}.json")).await.unwrap();
        assert_eq!(&got[..], format!("{{\"i\":{i}}}").as_bytes());
    }
}
