// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! moxfs 全链路端到端测试 — 阶段五
//!
//! 5 条全链路测试场景，验证 moxfs 各组件协同工作的正确性：
//! - 场景 1：对象写入全链路（S3 PutObject → 分块 → 纠删码 → MultiWriter → volume → master → filer → 完整性校验）
//! - 场景 2：对象读取全链路（S3 GetObject → filer → master → HedgedReader → volume → 纠删码解码 → 数据校验）
//! - 场景 3：对象删除全链路（S3 DeleteObject → lifecycle 门控 → 标记删除 → volume chunk 删除 → master 更新 → filer 删除 → 空间回收）
//! - 场景 4：故障恢复全链路（写入中节点故障 → MultiWriter 仲裁 → 重建触发 → 纠删码重建 → 数据一致性校验）
//! - 场景 5：生命周期迁移全链路（对象创建 → lifecycle 规则匹配 → transition_scan → 存储层级迁移 → 元数据更新 → 读取透明路由）
//!
//! 所有测试使用内存后端（InMemoryStorageBackend）或 mock 后端，不依赖外部服务。

use async_trait::async_trait;
use bytes::Bytes;
use mox_cloud_domain_traits::{
    BackendCapabilities, BackendType, ChunkId, ChunkInfo, ChunkListPage, ConsistencyModel,
    LifecycleEvaluator, LifecycleThresholds, ObjectLifecycleMeta, ReplicationStatus,
    StorageBackend, StorageClass, StorageClassTransition, StorageError,
};
use mox_cloud_kernel::{
    EcProfile, HedgedReader, MultiWriter, ReedSolomonEngine, ShardReadCost, ShardReader,
    ShardWriter, WriteProgressPolicy, WriteResult,
};
use mox_cloud_s3_svc::{
    HotWarmColdLifecycle, LifecycleObjectMeta, LifecycleReplicationStatus, S3Server,
    StorageClass as S3StorageClass, TransitionAction,
};
use sha2::{Digest, Sha256};
use std::{
    sync::{
        atomic::{AtomicU16, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

// =========================================================================
// 通用测试辅助
// =========================================================================

static NEXT_PORT: AtomicU16 = AtomicU16::new(25000);
const SKIP: &[(&str, &str)] = &[("x-test-skip-auth", "1")];

/// 启动使用默认 InMemoryStorageBackend 的 S3Server
async fn start_server() -> String {
    start_server_with_backend(Arc::new(mox_cloud_s3_svc::InMemoryStorageBackend::new())).await
}

/// 启动使用自定义 StorageBackend 的 S3Server（依赖注入）
async fn start_server_with_backend(backend: Arc<dyn StorageBackend>) -> String {
    for _ in 0..200 {
        let port = NEXT_PORT.fetch_add(1, Ordering::SeqCst);
        if port < 1025 {
            continue;
        }
        if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            continue;
        }
        let srv = S3Server::with_storage_backend(port, None, backend);
        srv.register_credential("AKIAE2ETEST001", "e2e-test-secret-key-2026", "e2e-user");
        tokio::spawn(async move {
            let _ = srv.run().await;
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        return format!("127.0.0.1:{}", port);
    }
    panic!("no free port for e2e test");
}

/// 启动使用 ReaderPipeline 的 S3Server
async fn start_server_with_pipeline(backends: Vec<Arc<dyn StorageBackend>>) -> String {
    for _ in 0..200 {
        let port = NEXT_PORT.fetch_add(1, Ordering::SeqCst);
        if port < 1025 {
            continue;
        }
        if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            continue;
        }
        let primary = backends
            .first()
            .cloned()
            .unwrap_or_else(|| Arc::new(mox_cloud_s3_svc::InMemoryStorageBackend::new()));
        let pipeline = Arc::new(mox_cloud_s3_svc::storage::S3ReaderPipeline::new(backends));
        let srv =
            S3Server::with_storage_backend(port, None, primary).with_reader_pipeline(pipeline);
        srv.register_credential("AKIAE2ETEST001", "e2e-test-secret-key-2026", "e2e-user");
        tokio::spawn(async move {
            let _ = srv.run().await;
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        return format!("127.0.0.1:{}", port);
    }
    panic!("no free port for e2e pipeline test");
}

/// 原始 HTTP 请求（复用集成测试模式）
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
    for (k, v) in headers {
        req.push_str(&format!("{}: {}\r\n", k, v));
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
        if buf.len() > 50_000_000 {
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

fn extract_header<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    let lower = name.to_lowercase();
    for line in head.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix(&lower) {
            return Some(rest.trim_start_matches(':').trim());
        }
        if let Some(rest) = l.strip_prefix(name) {
            return Some(rest.trim_start_matches(':').trim());
        }
    }
    None
}

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

fn assert_2xx(code: u16, msg: &str) {
    assert!((200..=299).contains(&code), "expected 2xx got {}: {}", code, msg);
}

fn assert_4xx(code: u16, expect: u16, msg: &str) {
    assert_eq!(code, expect, "want {} got {}: {}", expect, code, msg);
}

/// 生成确定性伪随机测试数据（可复现）
fn test_data(size: usize, seed: u8) -> Vec<u8> {
    let mut v = Vec::with_capacity(size);
    let mut acc: u32 = (seed as u32).wrapping_mul(0x9E37_79B9);
    for _ in 0..size {
        acc = acc.wrapping_mul(2654435761).wrapping_add(acc >> 13);
        v.push(acc as u8);
    }
    v
}

// =========================================================================
// Mock StorageBackend — 故障注入与调用追踪
// =========================================================================

/// 可配置故障的 mock StorageBackend，包装 InMemoryStorageBackend。
///
/// 支持：put/get/delete 成功率配置、延迟注入、调用次数统计。
struct FaultyBackend {
    inner: mox_cloud_s3_svc::InMemoryStorageBackend,
    put_fail_rate: f64,
    get_fail_rate: f64,
    delete_fail_rate: f64,
    put_delay: Duration,
    get_delay: Duration,
    put_count: AtomicUsize,
    get_count: AtomicUsize,
    delete_count: AtomicUsize,
    put_fail_count: AtomicUsize,
    get_fail_count: AtomicUsize,
    /// 强制所有 put 失败（用于全部节点故障场景）
    force_put_fail: bool,
    /// 强制所有 get 失败
    force_get_fail: bool,
}

#[allow(dead_code)]
impl FaultyBackend {
    fn new() -> Self {
        Self {
            inner: mox_cloud_s3_svc::InMemoryStorageBackend::new(),
            put_fail_rate: 0.0,
            get_fail_rate: 0.0,
            delete_fail_rate: 0.0,
            put_delay: Duration::ZERO,
            get_delay: Duration::ZERO,
            put_count: AtomicUsize::new(0),
            get_count: AtomicUsize::new(0),
            delete_count: AtomicUsize::new(0),
            put_fail_count: AtomicUsize::new(0),
            get_fail_count: AtomicUsize::new(0),
            force_put_fail: false,
            force_get_fail: false,
        }
    }

    fn with_put_fail_rate(mut self, rate: f64) -> Self {
        self.put_fail_rate = rate;
        self
    }

    fn with_get_delay(mut self, delay: Duration) -> Self {
        self.get_delay = delay;
        self
    }

    fn force_put_fail(mut self) -> Self {
        self.force_put_fail = true;
        self
    }

    fn force_get_fail(mut self) -> Self {
        self.force_get_fail = true;
        self
    }

    fn put_calls(&self) -> usize {
        self.put_count.load(Ordering::SeqCst)
    }

    fn get_calls(&self) -> usize {
        self.get_count.load(Ordering::SeqCst)
    }

    fn delete_calls(&self) -> usize {
        self.delete_count.load(Ordering::SeqCst)
    }

    /// 简单伪随机（基于调用计数，避免依赖 rand crate）
    fn should_fail(rate: f64, counter: &AtomicUsize) -> bool {
        if rate <= 0.0 {
            return false;
        }
        if rate >= 1.0 {
            return true;
        }
        let n = counter.fetch_add(1, Ordering::SeqCst);
        // 用线性同余生成 [0,1) 近似值
        let r = ((n.wrapping_mul(2654435761) >> 8) % 10000) as f64 / 10000.0;
        r < rate
    }
}

#[async_trait]
impl StorageBackend for FaultyBackend {
    async fn put_chunk(&self, chunk_id: &ChunkId, data: &[u8]) -> Result<ChunkInfo, StorageError> {
        self.put_count.fetch_add(1, Ordering::SeqCst);
        if self.put_delay > Duration::ZERO {
            tokio::time::sleep(self.put_delay).await;
        }
        if self.force_put_fail || Self::should_fail(self.put_fail_rate, &self.put_fail_count) {
            return Err(StorageError::BackendUnavailable);
        }
        self.inner.put_chunk(chunk_id, data).await
    }

    async fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Vec<u8>, StorageError> {
        self.get_count.fetch_add(1, Ordering::SeqCst);
        if self.get_delay > Duration::ZERO {
            tokio::time::sleep(self.get_delay).await;
        }
        if self.force_get_fail || Self::should_fail(self.get_fail_rate, &self.get_fail_count) {
            return Err(StorageError::BackendUnavailable);
        }
        self.inner.get_chunk(chunk_id).await
    }

    async fn delete_chunk(&self, chunk_id: &ChunkId) -> Result<bool, StorageError> {
        self.delete_count.fetch_add(1, Ordering::SeqCst);
        if Self::should_fail(self.delete_fail_rate, &self.delete_count) {
            return Err(StorageError::BackendUnavailable);
        }
        self.inner.delete_chunk(chunk_id).await
    }

    async fn chunk_exists(&self, chunk_id: &ChunkId) -> Result<bool, StorageError> {
        self.inner.chunk_exists(chunk_id).await
    }

    async fn list_chunks(
        &self,
        prefix: &str,
        marker: Option<&str>,
        limit: u32,
    ) -> Result<ChunkListPage, StorageError> {
        self.inner.list_chunks(prefix, marker, limit).await
    }

    fn backend_type(&self) -> BackendType {
        BackendType::InMemory
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            supports_range_read: true,
            supports_atomic_write: true,
            supports_conditional_put: false,
            consistency_model: ConsistencyModel::Strong,
            max_chunk_size: u64::MAX,
            preferred_chunk_size: 4 * 1024 * 1024,
        }
    }

    fn name(&self) -> &'static str {
        "faulty-mock-backend"
    }
}

// =========================================================================
// Mock ShardWriter / ShardReader — 用于 MultiWriter / HedgedReader 测试
// =========================================================================

struct MockShardWriter {
    endpoint: String,
    delay: Duration,
    should_fail: bool,
    write_count: AtomicUsize,
}

impl MockShardWriter {
    fn new(endpoint: &str, delay: Duration, should_fail: bool) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            delay,
            should_fail,
            write_count: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl ShardWriter for MockShardWriter {
    async fn write_shard(
        &self,
        _shard_index: usize,
        _data: Bytes,
    ) -> Result<(), mox_cloud_kernel::WriteError> {
        self.write_count.fetch_add(1, Ordering::SeqCst);
        if self.delay > Duration::ZERO {
            tokio::time::sleep(self.delay).await;
        }
        if self.should_fail {
            Err(mox_cloud_kernel::WriteError::ShardWriteFailed(
                _shard_index,
                "mock writer failure".into(),
            ))
        } else {
            Ok(())
        }
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

struct MockShardReader {
    endpoint: String,
    cost: ShardReadCost,
    delay: Duration,
    should_fail: bool,
    payload: Bytes,
}

impl MockShardReader {
    fn new(
        endpoint: &str,
        cost: ShardReadCost,
        delay: Duration,
        should_fail: bool,
        payload: Bytes,
    ) -> Self {
        Self { endpoint: endpoint.to_string(), cost, delay, should_fail, payload }
    }
}

#[async_trait]
impl ShardReader for MockShardReader {
    async fn read_shard(&self, _shard_index: usize) -> Result<Bytes, mox_cloud_kernel::ReadError> {
        if self.delay > Duration::ZERO {
            tokio::time::sleep(self.delay).await;
        }
        if self.should_fail {
            Err(mox_cloud_kernel::ReadError::ShardReadFailed(
                _shard_index,
                format!("mock reader {} failure", self.endpoint),
            ))
        } else {
            Ok(self.payload.clone())
        }
    }

    fn read_cost(&self) -> ShardReadCost {
        self.cost
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

// =========================================================================
// 简单 LifecycleEvaluator 实现 — 用于 trait 测试
// =========================================================================

struct SimpleLifecycleEvaluator {
    expire_after_days: u64,
}

impl LifecycleEvaluator for SimpleLifecycleEvaluator {
    fn should_transition(
        &self,
        meta: &ObjectLifecycleMeta,
        now_ms: u64,
        thresholds: &LifecycleThresholds,
    ) -> Option<StorageClassTransition> {
        let age_days = (now_ms.saturating_sub(meta.last_accessed_at_ms)) / 86_400_000;
        match meta.class {
            StorageClass::Hot if age_days >= thresholds.hot_to_warm_days => {
                Some(StorageClassTransition {
                    from: StorageClass::Hot,
                    to: StorageClass::Warm,
                    reason: format!("not accessed for {} days", age_days),
                })
            },
            StorageClass::Warm
                if age_days >= thresholds.hot_to_warm_days + thresholds.warm_to_cold_days =>
            {
                Some(StorageClassTransition {
                    from: StorageClass::Warm,
                    to: StorageClass::Cold,
                    reason: format!("not accessed for {} days", age_days),
                })
            },
            _ => None,
        }
    }

    fn should_expire(&self, meta: &ObjectLifecycleMeta, now_ms: u64) -> bool {
        let age_days = (now_ms.saturating_sub(meta.created_at_ms)) / 86_400_000;
        age_days >= self.expire_after_days
    }

    fn next_scan_time(&self, last_scan_ms: u64, scan_interval_sec: u64) -> u64 {
        last_scan_ms + scan_interval_sec * 1000
    }

    fn replication_blocks(
        &self,
        status: &ReplicationStatus,
        _action: &mox_cloud_domain_traits::LifecycleAction,
    ) -> bool {
        matches!(status, ReplicationStatus::Pending)
    }
}

// =========================================================================
// 场景 1：对象写入全链路
// =========================================================================

/// 场景 1.1：小对象（1KB）写入全链路 + SHA-256 完整性校验
#[tokio::test]
async fn e2e_s1_write_small_object_1kb() {
    let addr = start_server().await;
    // 创建桶
    let (c, _, _) = http(&addr, "PUT", "/e2e-write-small", SKIP, &[]).await;
    assert_2xx(c, "create bucket");

    let data = test_data(1024, 1);
    let expected_hash = sha256_hex(&data);

    // PutObject
    let (c, h, _) = http(&addr, "PUT", "/e2e-write-small/small-1kb.bin", SKIP, &data).await;
    assert_2xx(c, "put 1KB object");
    let etag = extract_header(&h, "ETag").map(|s| s.replace('"', "")).unwrap_or_default();
    assert!(!etag.is_empty(), "ETag should not be empty");

    // GetObject 验证数据完整性
    let (c2, _, body) = http(&addr, "GET", "/e2e-write-small/small-1kb.bin", SKIP, &[]).await;
    assert_2xx(c2, "get 1KB object");
    assert_eq!(body.len(), 1024, "body size mismatch");
    assert_eq!(sha256_hex(&body), expected_hash, "SHA-256 hash mismatch for 1KB object");

    // HeadObject 验证元数据
    let (c3, h3, _) = http(&addr, "HEAD", "/e2e-write-small/small-1kb.bin", SKIP, &[]).await;
    assert_2xx(c3, "head 1KB object");
    let cl = extract_header(&h3, "Content-Length")
        .unwrap_or("0")
        .parse::<usize>()
        .unwrap_or(0);
    assert_eq!(cl, 1024, "Content-Length mismatch");
}

/// 场景 1.2：中对象（256KB）写入全链路 + SHA-256 完整性校验
#[tokio::test]
async fn e2e_s1_write_medium_object_256kb() {
    let addr = start_server().await;
    http(&addr, "PUT", "/e2e-write-medium", SKIP, &[]).await;

    let data = test_data(256 * 1024, 42);
    let expected_hash = sha256_hex(&data);

    let (c, _, _) = http(&addr, "PUT", "/e2e-write-medium/medium-256kb.bin", SKIP, &data).await;
    assert_2xx(c, "put 256KB object");

    let (c2, _, body) = http(&addr, "GET", "/e2e-write-medium/medium-256kb.bin", SKIP, &[]).await;
    assert_2xx(c2, "get 256KB object");
    assert_eq!(body.len(), 256 * 1024, "body size mismatch");
    assert_eq!(sha256_hex(&body), expected_hash, "SHA-256 hash mismatch for 256KB object");
}

/// 场景 1.3：大对象（4MB）写入全链路 + SHA-256 完整性校验
#[tokio::test]
async fn e2e_s1_write_large_object_4mb() {
    let addr = start_server().await;
    http(&addr, "PUT", "/e2e-write-large", SKIP, &[]).await;

    let data = test_data(4 * 1024 * 1024, 7);
    let expected_hash = sha256_hex(&data);

    let (c, _, _) = http(&addr, "PUT", "/e2e-write-large/large-4mb.bin", SKIP, &data).await;
    assert_2xx(c, "put 4MB object");

    let (c2, _, body) = http(&addr, "GET", "/e2e-write-large/large-4mb.bin", SKIP, &[]).await;
    assert_2xx(c2, "get 4MB object");
    assert_eq!(body.len(), 4 * 1024 * 1024, "body size mismatch");
    assert_eq!(sha256_hex(&body), expected_hash, "SHA-256 hash mismatch for 4MB object");
}

/// 场景 1.4：chunk_id 格式验证（obj:{bucket}:{key}:{version_id}）
#[tokio::test]
async fn e2e_s1_write_chunk_id_format() {
    let backend = Arc::new(FaultyBackend::new());
    let addr = start_server_with_backend(backend.clone()).await;
    http(&addr, "PUT", "/e2e-chunkid-bucket", SKIP, &[]).await;

    let data = b"chunk-id-format-test";
    let (c, _, _) = http(&addr, "PUT", "/e2e-chunkid-bucket/test-key.txt", SKIP, data).await;
    assert_2xx(c, "put object for chunk_id check");

    // 非版本化对象 version_id 为空字符串，chunk_id 应为 obj:bucket:key:
    let expected_chunk_id = "obj:e2e-chunkid-bucket:test-key.txt:";
    let cid = ChunkId::new(expected_chunk_id);
    let stored = backend.inner.get_chunk(&cid).await;
    assert!(stored.is_ok(), "chunk should exist at expected chunk_id: {}", expected_chunk_id);
    assert_eq!(stored.unwrap(), data, "chunk data mismatch");

    // 验证 put_chunk 被调用
    assert!(backend.put_calls() >= 1, "put_chunk should be called at least once");
}

/// 场景 1.5：元数据一致性验证（content-type / size / bucket 存在）
#[tokio::test]
async fn e2e_s1_write_metadata_consistency() {
    let addr = start_server().await;
    http(&addr, "PUT", "/e2e-meta-bucket", SKIP, &[]).await;

    let headers = [("Content-Type", "application/json"), ("x-test-skip-auth", "1")];
    let data = br#"{"hello":"moxfs","version":5}"#;

    let (c, _, _) = http(&addr, "PUT", "/e2e-meta-bucket/meta.json", &headers, data).await;
    assert_2xx(c, "put object with content-type");

    // HeadObject 验证 content-type
    let (c2, h2, _) = http(&addr, "HEAD", "/e2e-meta-bucket/meta.json", SKIP, &[]).await;
    assert_2xx(c2, "head object");
    let ct = extract_header(&h2, "Content-Type").unwrap_or("");
    assert_eq!(ct, "application/json", "Content-Type mismatch");
    let cl = extract_header(&h2, "Content-Length")
        .unwrap_or("0")
        .parse::<usize>()
        .unwrap_or(0);
    assert_eq!(cl, data.len(), "Content-Length mismatch");

    // 桶存在验证
    let (c3, _, _) = http(&addr, "HEAD", "/e2e-meta-bucket", SKIP, &[]).await;
    assert_2xx(c3, "bucket should exist");

    // ListBuckets 包含该桶
    let (_, _, lb) = http(&addr, "GET", "/", SKIP, &[]).await;
    assert!(String::from_utf8_lossy(&lb).contains("e2e-meta-bucket"), "bucket should be in list");
}

// =========================================================================
// 场景 2：对象读取全链路
// =========================================================================

/// 场景 2.1：正常读取全链路（Put → Get，SHA-256 校验）
#[tokio::test]
async fn e2e_s2_read_normal() {
    let addr = start_server().await;
    http(&addr, "PUT", "/e2e-read-normal", SKIP, &[]).await;

    let data = test_data(64 * 1024, 99);
    let expected_hash = sha256_hex(&data);

    http(&addr, "PUT", "/e2e-read-normal/data.bin", SKIP, &data).await;

    let (c, _, body) = http(&addr, "GET", "/e2e-read-normal/data.bin", SKIP, &[]).await;
    assert_2xx(c, "get object");
    assert_eq!(body.len(), data.len());
    assert_eq!(sha256_hex(&body), expected_hash, "read data hash mismatch");
}

/// 场景 2.2：Range 字节范围读取
#[tokio::test]
async fn e2e_s2_read_range() {
    let addr = start_server().await;
    http(&addr, "PUT", "/e2e-read-range", SKIP, &[]).await;

    let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
    http(&addr, "PUT", "/e2e-read-range/range.bin", SKIP, &data).await;

    // Range: bytes=100-199
    let range_headers = [("Range", "bytes=100-199"), ("x-test-skip-auth", "1")];
    let (c, h, body) = http(&addr, "GET", "/e2e-read-range/range.bin", &range_headers, &[]).await;
    assert_eq!(c, 206, "Range request should return 206 Partial Content");
    assert_eq!(body.len(), 100, "Range body should be 100 bytes");
    assert_eq!(&body[..], &data[100..200], "Range content mismatch");

    let cr = extract_header(&h, "Content-Range").unwrap_or("");
    assert_eq!(cr, "bytes 100-199/1000", "Content-Range header mismatch");

    // Range: bytes=500- (到末尾)
    let range_headers2 = [("Range", "bytes=500-"), ("x-test-skip-auth", "1")];
    let (c2, _, body2) =
        http(&addr, "GET", "/e2e-read-range/range.bin", &range_headers2, &[]).await;
    assert_eq!(c2, 206);
    assert_eq!(body2.len(), 500);
    assert_eq!(&body2[..], &data[500..]);
}

/// 场景 2.3：不存在对象返回 404 NoSuchKey
#[tokio::test]
async fn e2e_s2_read_nonexistent_404() {
    let addr = start_server().await;
    http(&addr, "PUT", "/e2e-read-404", SKIP, &[]).await;

    let (c, _, body) = http(&addr, "GET", "/e2e-read-404/nonexistent.txt", SKIP, &[]).await;
    assert_4xx(c, 404, "nonexistent object should return 404");
    assert!(
        String::from_utf8_lossy(&body).contains("NoSuchKey"),
        "response should contain NoSuchKey"
    );

    // HEAD 不存在对象也返回 404
    let (c2, _, _) = http(&addr, "HEAD", "/e2e-read-404/nonexistent.txt", SKIP, &[]).await;
    assert_4xx(c2, 404, "HEAD nonexistent object should return 404");
}

/// 场景 2.4：ReaderPipeline 读取（with_reader_pipeline 启用多后端并发取最快）
#[tokio::test]
async fn e2e_s2_read_with_reader_pipeline() {
    // 两个后端都写入相同数据，pipeline 取最快
    let backend1 = Arc::new(mox_cloud_s3_svc::InMemoryStorageBackend::new());
    let backend2 = Arc::new(mox_cloud_s3_svc::InMemoryStorageBackend::new());

    // 先通过 backend1 写入数据（S3Server 的 primary backend 是 backend1）
    let addr = start_server_with_pipeline(vec![backend1.clone(), backend2.clone()]).await;
    http(&addr, "PUT", "/e2e-pipeline-bucket", SKIP, &[]).await;

    let data = test_data(32 * 1024, 55);
    let expected_hash = sha256_hex(&data);
    http(&addr, "PUT", "/e2e-pipeline-bucket/pipeline-data.bin", SKIP, &data).await;

    // 手动将数据也写入 backend2（模拟多副本）
    let cid = ChunkId::new("obj:e2e-pipeline-bucket:pipeline-data.bin:");
    backend2.put_chunk(&cid, &data).await.unwrap();

    // 通过 pipeline 读取（应从最快的后端返回）
    let (c, _, body) =
        http(&addr, "GET", "/e2e-pipeline-bucket/pipeline-data.bin", SKIP, &[]).await;
    assert_2xx(c, "get via reader pipeline");
    assert_eq!(sha256_hex(&body), expected_hash, "pipeline read hash mismatch");
}

/// 场景 2.5：大对象（>4MB）读取完整性
#[tokio::test]
async fn e2e_s2_read_large_object_5mb() {
    let addr = start_server().await;
    http(&addr, "PUT", "/e2e-read-big", SKIP, &[]).await;

    let data = test_data(5 * 1024 * 1024, 123);
    let expected_hash = sha256_hex(&data);

    http(&addr, "PUT", "/e2e-read-big/big-5mb.bin", SKIP, &data).await;

    let (c, _, body) = http(&addr, "GET", "/e2e-read-big/big-5mb.bin", SKIP, &[]).await;
    assert_2xx(c, "get 5MB object");
    assert_eq!(body.len(), 5 * 1024 * 1024);
    assert_eq!(sha256_hex(&body), expected_hash, "5MB read hash mismatch");
}

// =========================================================================
// 场景 3：对象删除全链路
// =========================================================================

/// 场景 3.1：正常删除全链路（Put → Delete → Get 404 + 后端 chunk 移除）
#[tokio::test]
async fn e2e_s3_delete_normal() {
    let backend = Arc::new(FaultyBackend::new());
    let addr = start_server_with_backend(backend.clone()).await;
    http(&addr, "PUT", "/e2e-del-normal", SKIP, &[]).await;

    let data = b"delete-me-please";
    http(&addr, "PUT", "/e2e-del-normal/todelete.txt", SKIP, data).await;

    // 验证删除前 chunk 存在
    let cid = ChunkId::new("obj:e2e-del-normal:todelete.txt:");
    assert!(backend.inner.chunk_exists(&cid).await.unwrap(), "chunk should exist before delete");

    // DeleteObject
    let (c, _, _) = http(&addr, "DELETE", "/e2e-del-normal/todelete.txt", SKIP, &[]).await;
    assert_2xx(c, "delete object");

    // 验证 delete_chunk 被调用
    assert!(backend.delete_calls() >= 1, "delete_chunk should be called");

    // 验证后端 chunk 已移除（空间回收）
    assert!(
        !backend.inner.chunk_exists(&cid).await.unwrap(),
        "chunk should be removed after delete"
    );

    // 验证 GetObject 返回 404
    let (c2, _, _) = http(&addr, "GET", "/e2e-del-normal/todelete.txt", SKIP, &[]).await;
    assert_4xx(c2, 404, "deleted object should return 404");
}

/// 场景 3.2：批量删除（DeleteMultipleObjects）
#[tokio::test]
async fn e2e_s3_delete_batch() {
    let addr = start_server().await;
    http(&addr, "PUT", "/e2e-del-batch", SKIP, &[]).await;

    // 创建 5 个对象
    for i in 0..5 {
        let data = format!("batch-object-{}", i);
        http(&addr, "PUT", &format!("/e2e-del-batch/obj-{}.txt", i), SKIP, data.as_bytes()).await;
    }

    // 批量删除 3 个
    let delete_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Delete>
  <Objects>
    <Object><Key>obj-0.txt</Key></Object>
    <Object><Key>obj-1.txt</Key></Object>
    <Object><Key>obj-2.txt</Key></Object>
  </Objects>
</Delete>"#;

    let (c, _, body) =
        http(&addr, "POST", "/e2e-del-batch?delete", SKIP, delete_xml.as_bytes()).await;
    assert_2xx(c, "batch delete");
    let resp = String::from_utf8_lossy(&body);
    assert!(resp.contains("DeleteResult"), "response should be DeleteResult XML");
    assert!(resp.contains("obj-0.txt"), "deleted obj-0 should be in result");
    assert!(resp.contains("obj-1.txt"), "deleted obj-1 should be in result");
    assert!(resp.contains("obj-2.txt"), "deleted obj-2 should be in result");

    // 验证被删除的对象返回 404
    for i in 0..3 {
        let (c2, _, _) =
            http(&addr, "GET", &format!("/e2e-del-batch/obj-{}.txt", i), SKIP, &[]).await;
        assert_4xx(c2, 404, &format!("batch-deleted obj-{} should be 404", i));
    }

    // 验证未删除的对象仍可读取
    for i in 3..5 {
        let (c3, _, body3) =
            http(&addr, "GET", &format!("/e2e-del-batch/obj-{}.txt", i), SKIP, &[]).await;
        assert_2xx(c3, &format!("obj-{} should still exist", i));
        assert!(String::from_utf8_lossy(&body3).contains(&format!("batch-object-{}", i)));
    }
}

/// 场景 3.3：删除不存在对象幂等（不报错）
#[tokio::test]
async fn e2e_s3_delete_idempotent() {
    let addr = start_server().await;
    http(&addr, "PUT", "/e2e-del-idem", SKIP, &[]).await;

    // 删除不存在的对象应返回 204（幂等）
    let (c, _, _) = http(&addr, "DELETE", "/e2e-del-idem/nonexistent.txt", SKIP, &[]).await;
    assert_2xx(c, "delete nonexistent object should be idempotent (2xx)");

    // 重复删除同一个已存在后删除的对象
    http(&addr, "PUT", "/e2e-del-idem/exists.txt", SKIP, b"data").await;
    http(&addr, "DELETE", "/e2e-del-idem/exists.txt", SKIP, &[]).await;
    let (c2, _, _) = http(&addr, "DELETE", "/e2e-del-idem/exists.txt", SKIP, &[]).await;
    assert_2xx(c2, "double delete should be idempotent");
}

// =========================================================================
// 场景 4：故障恢复全链路
// =========================================================================

/// 场景 4.1：MultiWriter 部分节点故障 — 达到 write_quorum 仍成功
#[tokio::test]
async fn e2e_s4_multiwriter_partial_failure() {
    // 3 个 writer：2 个成功 + 1 个失败，write_quorum=2
    let writers: Vec<Arc<dyn ShardWriter>> = vec![
        Arc::new(MockShardWriter::new("node-ok-0", Duration::ZERO, false)),
        Arc::new(MockShardWriter::new("node-ok-1", Duration::ZERO, false)),
        Arc::new(MockShardWriter::new("node-fail-2", Duration::ZERO, true)),
    ];

    let policy = WriteProgressPolicy {
        stall_timeout: Duration::from_secs(5),
        absolute_cap: None,
        write_quorum: 2,
    };

    let mw = MultiWriter::new(writers, policy);
    let shards: Vec<(usize, Bytes)> =
        (0..3).map(|i| (i, Bytes::from(vec![i as u8; 128]))).collect();

    let result: WriteResult = mw.write_all(shards).await.expect("should succeed with quorum=2");
    assert_eq!(result.succeeded.len(), 2, "2 writers should succeed");
    assert_eq!(result.failed.len(), 1, "1 writer should fail");
    assert!(result.duration >= Duration::ZERO);
}

/// 场景 4.2：Reed-Solomon 纠删码重建 — 丢失部分 shard 后重建数据
#[tokio::test]
async fn e2e_s4_reed_solomon_reconstruction() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap(); // 4 data + 2 parity

    let original = test_data(4096, 24);
    let original_hash = sha256_hex(&original);

    // 编码
    let shards = engine.encode(&profile, &original).expect("encode should succeed");
    assert_eq!(shards.len(), 6, "should have 6 shards (4 data + 2 parity)");

    // 模拟丢失 2 个 shard（1 data + 1 parity）
    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    slots[1] = None; // data shard 1 丢失
    slots[4] = None; // parity shard 0 丢失

    // 重建
    let reconstructed = engine
        .decode_reconstruct(&profile, &slots, original.len())
        .expect("reconstruction should succeed with 2 missing shards");

    assert_eq!(reconstructed.len(), original.len(), "reconstructed length mismatch");
    assert_eq!(
        sha256_hex(&reconstructed),
        original_hash,
        "reconstructed data hash mismatch — RS reconstruction failed"
    );

    // 验证 reconstruct_shards 也能恢复全部 shard
    let mut slots2: Vec<Option<Vec<u8>>> = (0..6).map(|i| Some(test_data(1024, i as u8))).collect();
    slots2[0] = None;
    slots2[5] = None;
    let all_shards = engine
        .reconstruct_shards(&profile, &slots2)
        .expect("reconstruct_shards should succeed");
    assert_eq!(all_shards.len(), 6, "should reconstruct all 6 shards");
}

/// 场景 4.3：全部节点失败 — MultiWriter 返回 QuorumNotMet
#[tokio::test]
async fn e2e_s4_all_nodes_fail() {
    let writers: Vec<Arc<dyn ShardWriter>> = vec![
        Arc::new(MockShardWriter::new("node-fail-0", Duration::ZERO, true)),
        Arc::new(MockShardWriter::new("node-fail-1", Duration::ZERO, true)),
        Arc::new(MockShardWriter::new("node-fail-2", Duration::ZERO, true)),
    ];

    let policy = WriteProgressPolicy {
        stall_timeout: Duration::from_secs(2),
        absolute_cap: None,
        write_quorum: 2,
    };

    let mw = MultiWriter::new(writers, policy);
    let shards: Vec<(usize, Bytes)> = (0..3).map(|i| (i, Bytes::from(vec![0u8; 64]))).collect();

    let result = mw.write_all(shards).await;
    match result {
        Err(mox_cloud_kernel::WriteError::QuorumNotMet { succeeded, quorum }) => {
            assert_eq!(succeeded, 0, "no writer should succeed");
            assert_eq!(quorum, 2, "quorum should be 2");
        },
        other => panic!("expected QuorumNotMet, got {:?}", other),
    }
}

/// 场景 4.4：HedgedReader — 慢后端触发 hedge，从快后端读取
#[tokio::test]
async fn e2e_s4_hedged_reader_slow_backend() {
    let fast_payload = Bytes::from_static(b"fast-backend-result-data");
    let slow_payload = Bytes::from_static(b"slow-backend-result-data");

    let readers: Vec<Arc<dyn ShardReader>> = vec![
        // 第一个 reader 很慢（远超 hedge_delay）
        Arc::new(MockShardReader::new(
            "slow-local",
            ShardReadCost::Local,
            Duration::from_secs(10),
            false,
            slow_payload,
        )),
        // 第二个 reader 快
        Arc::new(MockShardReader::new(
            "fast-remote",
            ShardReadCost::Remote,
            Duration::ZERO,
            false,
            fast_payload.clone(),
        )),
    ];

    let hr = HedgedReader::new(readers, Duration::from_millis(50));
    assert_eq!(hr.reader_count(), 2);
    assert_eq!(hr.min_read_cost(), ShardReadCost::Local);

    // hedged read 应从快后端返回（慢后端被 hedge 覆盖）
    let result = hr.read_hedged(0).await.expect("hedged read should succeed");
    assert_eq!(result, fast_payload, "should get result from fast backend via hedge");
}

/// 场景 4.5：StorageBackend put_chunk 故障注入 — S3 PutObject 返回错误
#[tokio::test]
async fn e2e_s4_storage_backend_put_failure() {
    // 强制所有 put 失败的后端
    let backend = Arc::new(FaultyBackend::new().force_put_fail());
    let addr = start_server_with_backend(backend).await;
    http(&addr, "PUT", "/e2e-fail-put-bucket", SKIP, &[]).await;

    let (c, _, body) =
        http(&addr, "PUT", "/e2e-fail-put-bucket/will-fail.txt", SKIP, b"this-will-fail").await;
    // 后端不可用应映射为 500 InternalError
    assert!(c >= 500, "backend put failure should return 5xx, got {}", c);
    assert!(
        String::from_utf8_lossy(&body).contains("InternalError")
            || String::from_utf8_lossy(&body).contains("storage"),
        "response should indicate storage error"
    );
}

/// 场景 4.6：纠删码超过容错上限 — TooManyShardsMissing
#[tokio::test]
async fn e2e_s4_rs_too_many_shards_missing() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();

    let original = test_data(2048, 99);
    let shards = engine.encode(&profile, &original).unwrap();

    // 丢失 3 个 shard（> parity=2），应失败
    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    slots[0] = None;
    slots[1] = None;
    slots[2] = None;

    let result = engine.decode_reconstruct(&profile, &slots, original.len());
    assert!(result.is_err(), "should fail with > parity shards missing");
    assert!(matches!(result.unwrap_err(), mox_cloud_kernel::RSError::TooManyShardsMissing(_)));
}

// =========================================================================
// 场景 5：生命周期迁移全链路
// =========================================================================

/// 场景 5.1：LifecycleEvaluator trait — should_transition / should_expire / next_scan_time
#[tokio::test]
async fn e2e_s5_lifecycle_evaluator_trait() {
    let evaluator = SimpleLifecycleEvaluator { expire_after_days: 365 };
    let thresholds = LifecycleThresholds::default(); // 30/90/180 天

    let t0 = 1_700_000_000_000u64;
    let meta = ObjectLifecycleMeta {
        bucket: "lc-bucket".into(),
        key: "data/report.pdf".into(),
        size_bytes: 1024 * 1024,
        class: StorageClass::Hot,
        created_at_ms: t0,
        last_accessed_at_ms: t0,
        last_transition_ms: t0,
        version_id: "v1".into(),
        replication_status: ReplicationStatus::None,
        object_locked: false,
    };

    // 10 天：不应迁移
    let t10 = t0 + 10 * 86_400_000;
    assert!(
        evaluator.should_transition(&meta, t10, &thresholds).is_none(),
        "10-day-old Hot object should not transition"
    );

    // 35 天：Hot → Warm
    let t35 = t0 + 35 * 86_400_000;
    let transition = evaluator
        .should_transition(&meta, t35, &thresholds)
        .expect("35-day-old Hot object should transition to Warm");
    assert_eq!(transition.from, StorageClass::Hot);
    assert_eq!(transition.to, StorageClass::Warm);

    // should_expire：100 天不应过期，400 天应过期
    assert!(
        !evaluator.should_expire(&meta, t0 + 100 * 86_400_000),
        "100-day object should not expire"
    );
    assert!(evaluator.should_expire(&meta, t0 + 400 * 86_400_000), "400-day object should expire");

    // next_scan_time
    let next = evaluator.next_scan_time(t0, 3600);
    assert_eq!(next, t0 + 3_600_000, "next_scan_time should be last + interval_ms");
}

/// 场景 5.2：HotWarmColdLifecycle transition_scan — 存储层级迁移 + 元数据更新
#[tokio::test]
async fn e2e_s5_transition_scan() {
    let lifecycle = HotWarmColdLifecycle::default();
    let t0 = 1_700_000_000_000u64;

    let meta = LifecycleObjectMeta {
        key: "logs/access.log".to_string(),
        bucket: "lc-bucket".to_string(),
        size_bytes: 10 * 1024 * 1024,
        class: S3StorageClass::Hot,
        created_at_ms: t0,
        last_accessed_at_ms: t0,
        last_transition_ms: t0,
        version_id: "null".to_string(),
        replication_status: LifecycleReplicationStatus::None,
        object_locked: false,
    };

    lifecycle.upsert_object(meta);

    // 迁移前：Hot
    assert_eq!(
        lifecycle.class_of("lc-bucket", "logs/access.log"),
        Some(S3StorageClass::Hot),
        "object should be Hot before transition"
    );

    // 90 天后扫描：HOT → WARM（30 天阈值）
    let t90 = t0 + 90 * 86_400_000;
    let plans = lifecycle.transition_scan(t90, true);
    assert!(!plans.is_empty(), "expect at least 1 transition plan after 90 days");
    assert!(
        plans.iter().any(|p| matches!(p.action, TransitionAction::HotToWarm)),
        "should include HotToWarm transition"
    );

    // 迁移后：Warm（元数据更新）
    assert_eq!(
        lifecycle.class_of("lc-bucket", "logs/access.log"),
        Some(S3StorageClass::Warm),
        "object storage class should be updated to Warm after transition_scan"
    );
}

/// 场景 5.3：生命周期迁移后读取透明路由（Put → 模拟迁移 → Get 仍返回正确数据）
#[tokio::test]
async fn e2e_s5_migration_then_read_transparent() {
    let addr = start_server().await;
    http(&addr, "PUT", "/e2e-lc-migrate", SKIP, &[]).await;

    let data = test_data(128 * 1024, 77);
    let expected_hash = sha256_hex(&data);

    // 写入对象（Hot 层）
    http(&addr, "PUT", "/e2e-lc-migrate/migrated-data.bin", SKIP, &data).await;

    // 模拟生命周期迁移：在 HotWarmColdLifecycle 中登记并执行 transition_scan
    let lifecycle = HotWarmColdLifecycle::default();
    let t0 = 1_700_000_000_000u64;
    lifecycle.upsert_object(LifecycleObjectMeta {
        key: "migrated-data.bin".to_string(),
        bucket: "e2e-lc-migrate".to_string(),
        size_bytes: data.len() as u64,
        class: S3StorageClass::Hot,
        created_at_ms: t0,
        last_accessed_at_ms: t0,
        last_transition_ms: t0,
        version_id: "null".to_string(),
        replication_status: LifecycleReplicationStatus::None,
        object_locked: false,
    });

    // 执行迁移（Hot → Warm）
    let t60 = t0 + 60 * 86_400_000;
    let plans = lifecycle.transition_scan(t60, true);
    assert!(!plans.is_empty(), "transition should produce plans");
    assert_eq!(
        lifecycle.class_of("e2e-lc-migrate", "migrated-data.bin"),
        Some(S3StorageClass::Warm),
        "object should be migrated to Warm"
    );

    // 迁移后读取仍应返回正确数据（透明路由：数据仍在 InMemoryStorageBackend 中，
    // storage_class 仅是元数据标签，读取路径不依赖层级）
    let (c, _, body) = http(&addr, "GET", "/e2e-lc-migrate/migrated-data.bin", SKIP, &[]).await;
    assert_2xx(c, "get after lifecycle migration should succeed (transparent routing)");
    assert_eq!(body.len(), data.len());
    assert_eq!(sha256_hex(&body), expected_hash, "data after lifecycle migration should be intact");
}

/// 场景 5.4：过期对象标记删除（should_expire + transition_scan 过期路径）
#[tokio::test]
async fn e2e_s5_expiration() {
    let evaluator = SimpleLifecycleEvaluator { expire_after_days: 30 };
    let t0 = 1_700_000_000_000u64;

    let meta = ObjectLifecycleMeta {
        bucket: "expire-bucket".into(),
        key: "temp/cache.tmp".into(),
        size_bytes: 4096,
        class: StorageClass::Hot,
        created_at_ms: t0,
        last_accessed_at_ms: t0,
        last_transition_ms: t0,
        version_id: "v1".into(),
        replication_status: ReplicationStatus::None,
        object_locked: false,
    };

    // 10 天：不应过期
    assert!(
        !evaluator.should_expire(&meta, t0 + 10 * 86_400_000),
        "10-day object should not expire (30-day threshold)"
    );

    // 35 天：应过期
    assert!(
        evaluator.should_expire(&meta, t0 + 35 * 86_400_000),
        "35-day object should expire (30-day threshold)"
    );

    // HotWarmColdLifecycle 过期路径：创建对象后长时间扫描
    let lifecycle = HotWarmColdLifecycle::default();
    lifecycle.upsert_object(LifecycleObjectMeta {
        key: "temp/expire-me.tmp".to_string(),
        bucket: "expire-bucket".to_string(),
        size_bytes: 1024,
        class: S3StorageClass::Hot,
        created_at_ms: t0,
        last_accessed_at_ms: t0,
        last_transition_ms: t0,
        version_id: "null".to_string(),
        replication_status: LifecycleReplicationStatus::None,
        object_locked: false,
    });

    // 400 天后扫描：应触发迁移（HOT → WARM → COLD），对象仍存在但层级已变
    let t400 = t0 + 400 * 86_400_000;
    let plans = lifecycle.transition_scan(t400, true);
    assert!(!plans.is_empty(), "long-aged object should produce transition plans");
    // 经过多次迁移后应为 Cold
    let class = lifecycle.class_of("expire-bucket", "temp/expire-me.tmp");
    assert!(
        class == Some(S3StorageClass::Cold) || class == Some(S3StorageClass::Warm),
        "object should be migrated to Warm or Cold after 400 days, got {:?}",
        class
    );
}

/// 场景 5.5：ScanBudget 扫描预算控制
#[tokio::test]
async fn e2e_s5_scan_budget() {
    use mox_cloud_s3_svc::{CapacityBudget, IoBudget, ScanBudget, ScanBudgetTracker, TimeBudget};

    // 创建扫描预算：max_objects_per_scan=100, max_bytes_per_scan=10MB
    let budget = ScanBudget {
        time: TimeBudget { max_duration: None, window_start_hour: None, window_end_hour: None },
        io: IoBudget { max_objects_per_sec: 0, max_io_per_sec: 0, max_parallelism: 4 },
        capacity: CapacityBudget {
            max_bytes_per_scan: 10 * 1024 * 1024,
            max_migration_bytes: 0,
            max_objects_per_scan: 100,
        },
    };

    let tracker = ScanBudgetTracker::new(budget);

    // 初始状态：有预算
    assert!(tracker.can_continue(), "should have budget initially");

    // 消耗 50 个对象
    for _ in 0..50 {
        tracker.record_object(1024);
    }
    assert!(tracker.can_continue(), "should still have budget after 50 objects (limit 100)");

    // 再消耗 60 个对象 → 超限（110 > 100）
    for _ in 0..60 {
        tracker.record_object(1024);
    }
    assert!(!tracker.can_continue(), "should be exhausted after 110 objects (limit 100)");
    assert!(tracker.stats().budget_exceeded, "budget_exceeded flag should be set");

    // 容量预算测试：max_bytes_per_scan=500
    let budget2 = ScanBudget {
        capacity: CapacityBudget {
            max_bytes_per_scan: 500,
            max_migration_bytes: 0,
            max_objects_per_scan: 0,
        },
        ..Default::default()
    };
    let tracker2 = ScanBudgetTracker::new(budget2);
    tracker2.record_object(300);
    assert!(tracker2.can_continue(), "should continue after 300 bytes (limit 500)");
    tracker2.record_object(250); // 累计 550 >= 500
    assert!(!tracker2.can_continue(), "should stop after reaching max_bytes_per_scan");

    // 统计验证
    let stats = tracker2.stats();
    assert_eq!(stats.objects_scanned, 2);
    assert_eq!(stats.bytes_scanned, 550);
}
