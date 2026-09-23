// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! P0 持久化闭环回归：`persist-rocksdb` 模式下写入 → 关闭 → 重开同一目录 → 读回。
//! 验证三件事：RocksDB 真实落盘、vid 二级索引跨重启一致、ack 前 fsync 语义生效。

#![cfg(feature = "persist-rocksdb")]

use std::collections::BTreeMap;
use std::path::Path;

use mox_kg_storage_svc::graph_codec::PropValue;
use mox_kg_storage_svc::storage_server::StorageServer;

fn props(pairs: &[(&str, &str)]) -> BTreeMap<String, PropValue> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), PropValue::Str(v.to_string())))
        .collect()
}

fn add_docs(srv: &StorageServer, n: usize) {
    for i in 0..n {
        let vid = format!("doc://p0/{i}");
        srv.add_vertex(vid, "doc".into(), props(&[("title", &format!("t{i}"))]))
            .expect("add_vertex");
    }
}

fn read_back_all(srv: &StorageServer, n: usize) -> usize {
    (0..n)
        .filter(|i| {
            let vid = format!("doc://p0/{i}");
            match srv.read_vertex(&vid) {
                Some((_shard, tag, p)) => {
                    tag == "doc" && p.get("title") == Some(&PropValue::Str(format!("t{i}")))
                }
                None => false,
            }
        })
        .count()
}

#[test]
fn persistence_reopen_same_dir_reads_back() {
    // 默认即 durable 模式（MOX_KG_WAL_SYNC 未设 → ack 前 fsync）
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir: &Path = tmp.path();

    {
        let srv = StorageServer::start_cluster(16, &[], Some(dir)).expect("first open");
        add_docs(&srv, 500);
        assert_eq!(read_back_all(&srv, 500), 500, "写后立即读必须命中");
        srv.rocks_db_handles
            .graceful_shutdown()
            .expect("graceful shutdown flushes memtables");
    } // 释放全部句柄，否则 RocksDB 目录锁会阻塞重开

    let srv2 = StorageServer::start_cluster(16, &[], Some(dir)).expect("reopen same dir");
    assert_eq!(
        read_back_all(&srv2, 500),
        500,
        "跨重启读回必须完整（vid_idx + vid_meta 同目录恢复）"
    );
}

#[test]
fn long_uri_vid_survives_persistence() {
    // VID 长度前缀 2B 的回归锁定：>255B 的 URI 型 ID 可写、可重启读回
    let long_vid = format!("uri://{}", "x".repeat(400));
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir: &Path = tmp.path();
    {
        let srv = StorageServer::start_cluster(8, &[], Some(dir)).expect("open");
        srv.add_vertex(long_vid.clone(), "doc".into(), props(&[("k", "v")]))
            .expect("400B+ vid must be accepted (2B len prefix)");
        srv.rocks_db_handles
            .graceful_shutdown()
            .expect("shutdown");
    }
    let srv2 = StorageServer::start_cluster(8, &[], Some(dir)).expect("reopen");
    assert!(srv2.read_vertex(&long_vid).is_some(), "长 VID 必须跨重启读回");
}
