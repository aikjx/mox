// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 阶段2d：S3 写 chokepoint → store-core 持久化镜像集成测试。
//!
//! 通过 `S3Server::with_persist` 挂载 `StoreCorePersist`，走 HTTP 全链路：
//! - PutObject / DeleteObject / CopyObject 三类写 chokepoint
//! - 内存主路径保持绿（S3 GET 立即可读）
//! - `flush()` 后从 store-core FS 后端读回，验证真实落盘与删除

use mox_cloud_s3_svc::{PersistSink, S3Server, StoreCorePersist};
use mox_cloud_store_core::{create_backend, BackendKind, StoreConfig, StoreError};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc,
    },
    time::Duration,
};

const TEST_AK: &str = "AKIAMOXPERSIST0001";
const TEST_SK: &str = "mox-persist-secret-v1-2026";
static NEXT_PORT: AtomicU16 = AtomicU16::new(23100);

async fn start_server() -> (String, Arc<StoreCorePersist>, tempfile::TempDir) {
    for _ in 0..200 {
        let port = NEXT_PORT.fetch_add(1, Ordering::SeqCst);
        if port < 1025 {
            continue;
        }
        if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            continue;
        }
        let dir = tempfile::tempdir().unwrap();
        let backend = create_backend(&StoreConfig {
            kind: BackendKind::Fs,
            data_dir: dir.path().to_path_buf(),
            ..Default::default()
        })
        .unwrap();
        let persist = Arc::new(StoreCorePersist::new(backend));
        let sink: Arc<dyn PersistSink> = persist.clone();
        let srv = S3Server::with_persist(port, None, sink);
        srv.register_credential(TEST_AK, TEST_SK, "mox-user");
        tokio::spawn(async move {
            let _ = srv.run().await;
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        return (format!("127.0.0.1:{port}"), persist, dir);
    }
    panic!("no free port");
}

/// 直接从 store-core FS 后端读回（验证真实落盘）。
async fn read_store(dir: &Path, path: &str) -> Result<Vec<u8>, StoreError> {
    let backend = create_backend(&StoreConfig {
        kind: BackendKind::Fs,
        data_dir: dir.to_path_buf(),
        ..Default::default()
    })
    .unwrap();
    backend.object.get(path).await.map(|b| b.to_vec())
}

async fn http(
    addr: &str,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> (u16, String, Vec<u8>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let Ok(mut s) = tokio::net::TcpStream::connect(addr).await else {
        return (0, String::new(), vec![]);
    };
    let cl = body.len();
    let mut req = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        method, path, addr, cl
    );
    req.push_str("x-test-skip-auth: 1\r\n");
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("\r\n");
    s.write_all(req.as_bytes()).await.ok();
    if cl > 0 {
        s.write_all(body).await.ok();
    }
    s.flush().await.ok();
    let mut buf = Vec::with_capacity(4096);
    let mut tmp = [0u8; 2048];
    loop {
        let n = match s.read(&mut tmp).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() > 10_000_000 {
            break;
        }
    }
    let sp = buf.windows(4).position(|w| w == b"\r\n\r\n").unwrap_or(buf.len());
    let head = String::from_utf8_lossy(&buf[..sp]).to_string();
    let code: u16 = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        .unwrap_or(0);
    let bo = if sp + 4 < buf.len() { buf[sp + 4..].to_vec() } else { vec![] };
    (code, head, bo)
}

#[tokio::test]
async fn s3_write_chokepoints_persist_to_store_core() {
    let (addr, persist, dir) = start_server().await;

    // 0. 创建桶（S3 语义：bucket 不存在时 PutObject → 404）
    let (c, _, _) = http(&addr, "PUT", "/persist", &[], &[]).await;
    assert!((200..=299).contains(&c), "create bucket got {c}");

    // 1. PutObject x3（小/中/大）
    let (c, _, _) = http(&addr, "PUT", "/persist/docs/a.md", &[], "hello from s3".as_bytes()).await;
    assert!((200..=299).contains(&c), "put a.md got {c}");
    let (c, _, _) = http(&addr, "PUT", "/persist/docs/b.md", &[], b"content-b").await;
    assert!((200..=299).contains(&c), "put b.md got {c}");
    let big: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
    let (c, _, _) = http(&addr, "PUT", "/persist/media/c.bin", &[], &big).await;
    assert!((200..=299).contains(&c), "put c.bin got {c}");

    // 内存主路径立即可读（一致性）
    let (c, _, body) = http(&addr, "GET", "/persist/docs/a.md", &[], &[]).await;
    assert!((200..=299).contains(&c), "get a.md got {c}");
    assert_eq!(body, "hello from s3".as_bytes().to_vec());

    // 2. flush → store-core 真实落盘
    persist.flush();
    assert_eq!(
        read_store(dir.path(), "persist/docs/a.md").await.unwrap(),
        "hello from s3".as_bytes().to_vec()
    );
    assert_eq!(read_store(dir.path(), "persist/docs/b.md").await.unwrap(), b"content-b".to_vec());
    assert_eq!(read_store(dir.path(), "persist/media/c.bin").await.unwrap(), big);

    // 3. DeleteObject → 镜像删除
    let (c, _, _) = http(&addr, "DELETE", "/persist/docs/b.md", &[], &[]).await;
    assert!((200..=299).contains(&c), "delete b.md got {c}");
    persist.flush();
    assert!(matches!(
        read_store(dir.path(), "persist/docs/b.md").await,
        Err(StoreError::NotFound { .. })
    ));
    // 未删除对象仍可读
    assert_eq!(
        read_store(dir.path(), "persist/docs/a.md").await.unwrap(),
        "hello from s3".as_bytes().to_vec()
    );

    // 4. CopyObject → 目标 key 镜像落盘
    let (c, _, _) = http(
        &addr,
        "PUT",
        "/persist/docs/copy.md",
        &[("x-amz-copy-source", "/persist/docs/a.md")],
        &[],
    )
    .await;
    assert!((200..=299).contains(&c), "copy got {c}");
    persist.flush();
    assert_eq!(
        read_store(dir.path(), "persist/docs/copy.md").await.unwrap(),
        "hello from s3".as_bytes().to_vec()
    );

    // 5. 覆盖写（PutObject 覆盖最新）→ store-core 最新内容
    let (c, _, _) = http(&addr, "PUT", "/persist/docs/a.md", &[], "updated-v2".as_bytes()).await;
    assert!((200..=299).contains(&c), "overwrite got {c}");
    persist.flush();
    assert_eq!(
        read_store(dir.path(), "persist/docs/a.md").await.unwrap(),
        "updated-v2".as_bytes().to_vec()
    );
}

#[tokio::test]
async fn s3_server_without_persist_stays_pure_memory() {
    // 无钩子 → 默认 new 仍纯内存（回归）
    for _ in 0..200 {
        let port = NEXT_PORT.fetch_add(1, Ordering::SeqCst);
        if port < 1025 {
            continue;
        }
        if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            continue;
        }
        let srv = S3Server::new(port, None);
        srv.register_credential(TEST_AK, TEST_SK, "mox-user");
        tokio::spawn(async move {
            let _ = srv.run().await;
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        let addr = format!("127.0.0.1:{port}");
        let (c, _, _) = http(&addr, "PUT", "/mem", &[], &[]).await;
        assert!((200..=299).contains(&c), "create bucket got {c}");
        let (c, _, _) = http(&addr, "PUT", "/mem/docs/x.md", &[], b"in-memory").await;
        assert!((200..=299).contains(&c), "put got {c}");
        let (c, _, body) = http(&addr, "GET", "/mem/docs/x.md", &[], &[]).await;
        assert!((200..=299).contains(&c), "get got {c}");
        assert_eq!(body, b"in-memory".to_vec());
        return;
    }
    panic!("no free port");
}
