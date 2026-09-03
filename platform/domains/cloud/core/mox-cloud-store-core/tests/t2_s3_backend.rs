// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 阶段2 集成测试：S3 后端对**内联 MockS3Server** 的真实 HTTP 往返。
//!
//! 不依赖外部 MinIO：mock 以 `std::net` 阻塞线程实现，支持
//! PUT/GET/HEAD/DELETE/RANGE/ListObjectsV2（canned XML），
//! 覆盖 [`S3ObjectStore`] 三路物理口与 key 同构断言。
//!
//! 仅在 `--features s3` 下编译运行（`#![cfg(feature = "s3")]`）。

#![cfg(feature = "s3")]

use bytes::Bytes;
use mox_base_store_core::{KvStore, ObjectStore, ObjectStreamWriter, StoreError};
use mox_cloud_store_core::{
    create_backend, BackendKind, S3ClientConfig, S3ObjectStore, StoreConfig,
};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

/// 内联 MockS3Server：记录对象 + 收到的 key（供 key 同构断言）
struct MockS3Server {
    addr: SocketAddr,
    objects: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    seen_keys: Arc<Mutex<Vec<String>>>,
    _handle: Option<thread::JoinHandle<()>>,
}

impl MockS3Server {
    fn start(bucket: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("绑定 mock 端口失败");
        let addr = listener.local_addr().unwrap();
        let objects: Arc<Mutex<HashMap<String, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
        let seen_keys: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let objects2 = objects.clone();
        let seen2 = seen_keys.clone();
        let bucket = bucket.to_string();
        let handle = thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(s) => {
                        let objects = objects2.clone();
                        let seen = seen2.clone();
                        let bucket = bucket.clone();
                        thread::spawn(move || handle_conn(s, &objects, &seen, &bucket));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            addr,
            objects,
            seen_keys,
            _handle: Some(handle),
        }
    }

    fn endpoint(&self) -> String {
        format!("http://{}", self.addr)
    }

    fn seen(&self) -> Vec<String> {
        self.seen_keys.lock().unwrap().clone()
    }
}

/// 逐连接处理：读请求头 + 体 → 分发
fn handle_conn(
    mut stream: TcpStream,
    objects: &Mutex<HashMap<String, Vec<u8>>>,
    seen: &Mutex<Vec<String>>,
    bucket: &str,
) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let text = String::from_utf8_lossy(&buf);
    let mut lines = text.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");
    let mut content_length = 0usize;
    let mut range: Option<(u64, u64)> = None;
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
        if let Some(v) = lower.strip_prefix("range:") {
            if let Some((s, e)) = parse_range(v.trim()) {
                range = Some((s, e));
            }
        }
    }
    let body_start = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)
        .unwrap_or(buf.len());
    let body: Vec<u8> = if body_start + content_length <= buf.len() {
        buf[body_start..body_start + content_length].to_vec()
    } else {
        buf[body_start..].to_vec()
    };

    let prefix = format!("/{bucket}/");
    let key = if let Some(rest) = path.strip_prefix(&prefix) {
        percent_decode(rest.split('?').next().unwrap_or(""))
    } else {
        String::new()
    };
    let has_list_query = path.contains("list-type=2");

    let mut obj_map = objects.lock().unwrap();
    let mut seen_map = seen.lock().unwrap();

    if method == "PUT" && !key.is_empty() {
        seen_map.push(key.clone());
        obj_map.insert(key.clone(), body);
        respond(
            &mut stream,
            200,
            "OK",
            &[("ETag", "\"mock-etag\""), ("Content-Length", "0")],
            b"",
        );
        return;
    }
    if method == "DELETE" && !key.is_empty() {
        seen_map.push(key.clone());
        obj_map.remove(&key);
        respond(&mut stream, 204, "No Content", &[], b"");
        return;
    }
    if method == "HEAD" && !key.is_empty() {
        match obj_map.get(&key) {
            Some(data) => {
                respond(
                    &mut stream,
                    200,
                    "OK",
                    &[
                        ("Content-Length", &data.len().to_string()),
                        ("Content-Type", "application/octet-stream"),
                    ],
                    b"",
                );
            }
            None => respond(&mut stream, 404, "Not Found", &[], &not_found_xml(&key)),
        }
        return;
    }
    if method == "GET" && has_list_query {
        let prefix_filter = path
            .split("prefix=")
            .nth(1)
            .map(|s| s.split('&').next().unwrap_or(""))
            .map(percent_decode)
            .unwrap_or_default();
        let keys: Vec<String> = obj_map
            .keys()
            .filter(|k| k.starts_with(&prefix_filter))
            .cloned()
            .collect();
        let mut xml = String::from("<ListBucketResult>");
        for k in keys {
            xml.push_str(&format!("<Contents><Key>{}</Key></Contents>", k));
        }
        xml.push_str("</ListBucketResult>");
        respond(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/xml")],
            xml.as_bytes(),
        );
        return;
    }
    if method == "GET" && !key.is_empty() {
        match obj_map.get(&key) {
            Some(data) => match range {
                Some((s, e)) => {
                    let start = s as usize;
                    let end = ((e as usize).saturating_add(1)).min(data.len());
                    let part = &data[start.min(data.len())..end];
                    respond(
                        &mut stream,
                        206,
                        "Partial Content",
                        &[("Content-Length", &part.len().to_string())],
                        part,
                    );
                }
                None => respond(
                    &mut stream,
                    200,
                    "OK",
                    &[
                        ("Content-Length", &data.len().to_string()),
                        ("Content-Type", "application/octet-stream"),
                    ],
                    data,
                ),
            },
            None => respond(&mut stream, 404, "Not Found", &[], &not_found_xml(&key)),
        }
        return;
    }
    respond(&mut stream, 400, "Bad Request", &[], b"");
}

fn parse_range(v: &str) -> Option<(u64, u64)> {
    let r = v.strip_prefix("bytes=")?;
    let (s, e) = r.split_once('-')?;
    Some((s.trim().parse().ok()?, e.trim().parse().ok()?))
}

/// URL 百分号解码（mock 侧还原语义 key，模拟真实 S3 服务器）
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn not_found_xml(key: &str) -> Vec<u8> {
    format!(
        "<Error><Code>NoSuchKey</Code><Message>The specified key does not exist.</Message><Key>{key}</Key></Error>"
    )
    .into_bytes()
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) {
    let mut resp = format!("HTTP/1.1 {status} {reason}\r\n");
    for (k, v) in headers {
        resp.push_str(&format!("{k}: {v}\r\n"));
    }
    resp.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn s3_cfg(server: &MockS3Server) -> S3ClientConfig {
    S3ClientConfig {
        endpoint: server.endpoint(),
        region: "us-east-1".into(),
        access_key: "minioadmin".into(),
        secret_key: "minioadmin".into(),
        bucket: "kb".into(),
        force_path_style: true,
    }
}

#[tokio::test]
async fn t2_s3_object_store_http_roundtrip() {
    let server = MockS3Server::start("kb");
    let dir = tempfile::tempdir().unwrap();
    let store = S3ObjectStore::new(dir.path(), &s3_cfg(&server)).unwrap();

    let data = Bytes::from_static(b"hello s3 backend");
    let obj = ObjectStore::put(&store, "kb/doc/a.md", "text/markdown", data.clone())
        .await
        .unwrap();
    assert_eq!(obj.size_bytes, 16);

    let head = ObjectStore::head(&store, "kb/doc/a.md").await.unwrap();
    assert_eq!(head.size_bytes, 16);
    assert_eq!(head.content_type, "application/octet-stream");

    let got = ObjectStore::get(&store, "kb/doc/a.md").await.unwrap();
    assert_eq!(&got[..], b"hello s3 backend");

    let range = ObjectStore::get_range(&store, "kb/doc/a.md", 6, 3).await.unwrap();
    assert_eq!(&range[..], b"s3 ");

    assert!(ObjectStore::exists(&store, "kb/doc/a.md").await.unwrap());

    ObjectStore::delete(&store, "kb/doc/a.md").await.unwrap();
    assert!(!ObjectStore::exists(&store, "kb/doc/a.md").await.unwrap());
    assert!(matches!(
        ObjectStore::get(&store, "kb/doc/a.md").await,
        Err(StoreError::NotFound { .. })
    ));
}

#[tokio::test]
async fn t2_s3_key_isomorphism_with_fs() {
    // key 同构：FS 逻辑路径 == S3 key（逐字一致），保证 FS/S3 可互换迁移
    let server = MockS3Server::start("kb");
    let dir = tempfile::tempdir().unwrap();
    let store = S3ObjectStore::new(dir.path(), &s3_cfg(&server)).unwrap();
    let path = "kb/guide/架构手册.md"; // 含中文与斜杠
    ObjectStore::put(&store, path, "text/markdown", Bytes::from_static(b"content"))
        .await
        .unwrap();
    let seen = server.seen();
    assert_eq!(seen, vec![path], "S3 key 必须与 FS 逻辑路径逐字一致");
}

#[tokio::test]
async fn t2_s3_list_objects_and_kv_and_stream() {
    let server = MockS3Server::start("kb");
    let dir = tempfile::tempdir().unwrap();
    let store = S3ObjectStore::new(dir.path(), &s3_cfg(&server)).unwrap();

    ObjectStore::put(&store, "kb/a.txt", "text/plain", Bytes::from_static(b"a"))
        .await
        .unwrap();
    ObjectStore::put(&store, "kb/b.txt", "text/plain", Bytes::from_static(b"b"))
        .await
        .unwrap();
    let keys = store.client().list_objects("kb/").await.unwrap();
    assert!(keys.contains(&"kb/a.txt".to_string()));
    assert!(keys.contains(&"kb/b.txt".to_string()));

    // KvStore：本地 data_dir/kv 落盘
    KvStore::put(&store, "bucket:meta", Bytes::from_static(b"{}"))
        .await
        .unwrap();
    assert_eq!(&KvStore::get(&store, "bucket:meta").await.unwrap().unwrap()[..], b"{}");

    // ObjectStreamWriter：流式 → 单次 PUT
    let h = store.open_writer("kb/big.bin", "application/octet-stream").await.unwrap();
    for part in [b"12345".as_slice(), b"67890"] {
        store.write(&h, Bytes::copy_from_slice(part)).await.unwrap();
    }
    let obj = store.close(h).await.unwrap();
    assert_eq!(obj.size_bytes, 10);
    assert_eq!(&ObjectStore::get(&store, "kb/big.bin").await.unwrap()[..], b"1234567890");
}

#[tokio::test]
async fn t2_backend_factory_s3_with_mock() {
    let server = MockS3Server::start("kb");
    let dir = tempfile::tempdir().unwrap();
    let cfg = StoreConfig {
        kind: BackendKind::Minio,
        data_dir: dir.path().to_path_buf(),
        s3: Some(s3_cfg(&server)),
        ..Default::default()
    };
    let be = create_backend(&cfg).unwrap();
    assert_eq!(be.kind, BackendKind::Minio);
    be.object
        .put("k", "text/plain", Bytes::from_static(b"v"))
        .await
        .unwrap();
    assert_eq!(&be.object.get("k").await.unwrap()[..], b"v");
    assert!(server.seen().contains(&"k".to_string()));
}
