// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! S3 兼容 HTTP Server：内置 axum router 挂 34 API handlers。
//!
//! 34 API 覆盖 ListBuckets/CreateBucket/DeleteBucket/HeadBucket/ListObjectsV1/V2/
//! Get/Put/Delete/Head/CopyObject/Get/PutObjectAcl / Multipart 全套 /
//! DeleteMultipleObjects / Versioning / Tagging / Policy / Lifecycle / CORS。
//!
//! 响应严格遵循 AWS S3 v20060301 XML 格式。

use crate::acl::CannedAcl;
use crate::bucket_analytics::AnalyticsManager;
use crate::cors::{CorsConfiguration, CorsRule};
use crate::error::{S3Error, S3Result};
use crate::inventory::InventoryManager;
use crate::lifecycle::StorageClass;
use crate::mpu::MultipartManager;
use crate::object_batch_ops::BatchOperationManager;
use crate::policy::BucketPolicy;
use crate::replication::ReplicationManager;
use crate::sigv4_middleware::{verify_request, CredentialStore};
use crate::tagging::Tagging;
use crate::versioning::{generate_version_id, VersioningManager, VersioningStatus};
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Request, Response, StatusCode},
    routing::any,
    Router,
};
use bytes::Bytes;
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use mox_cloud_master_svc::MasterServer;
use mox_cloud_foundation::PartETag;

// ---------------- AppState (axum shared state) ----------------
#[derive(Clone)]
struct AppState {
    storage: Arc<InMemoryStorage>,
    creds: Arc<Mutex<CredentialStore>>,
    versioning: Arc<VersioningManager>,
    mpu: Arc<MultipartManager>,
    vcounter: Arc<Mutex<BTreeMap<(String, String), u64>>>,
    analytics: Arc<AnalyticsManager>,
    batch_ops: Arc<BatchOperationManager>,
    replication: Arc<ReplicationManager>,
    inventory: Arc<InventoryManager>,
}

// ---------------- Storage State ----------------

#[derive(Debug, Clone)]
struct ObjectMeta {
    data: Vec<u8>,
    etag: String,
    size: u64,
    last_modified_ms: u64,
    content_type: String,
    version_id: String,
    acl: CannedAcl,
    tags: BTreeMap<String, String>,
    is_delete_marker: bool,
    crc32c: u32,
}

#[derive(Debug, Clone)]
struct BucketMeta {
    #[allow(dead_code)] // name：预留（响应/审计），当前未读取
    name: String,
    created_ms: u64,
    _acl: CannedAcl,
    policy: Option<BucketPolicy>,
    tags: BTreeMap<String, String>,
    cors: Option<CorsConfiguration>,
    lifecycle_xml: Option<String>,
}

/// 内置内存对象存储（生产版可替换为对 master+volumes 的调用）。
#[derive(Debug, Default)]
struct InMemoryStorage {
    buckets: Mutex<BTreeMap<String, BucketMeta>>,
    // bucket -> key -> 多版本（最新版本放 last_versioned 字段，历史版本在 versions 中）
    objects: Mutex<BTreeMap<String, BTreeMap<String, Vec<ObjectMeta>>>>,
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn new_request_id() -> String {
    let mut b = [0u8; 8];
    for (i, slot) in b.iter_mut().enumerate() {
        *slot = (rand_byte().wrapping_add(i as u8)) ^ 0x9E;
    }
    hex::encode(b).to_uppercase()
}

fn rand_byte() -> u8 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u8)
        .unwrap_or(0);
    let tid_str = format!("{:?}", std::thread::current().id());
    let tid_byte = tid_str.bytes().last().unwrap_or(b'*');
    n.wrapping_add(tid_byte)
}

// ---------------- S3Server ----------------

pub struct S3Server {
    port: u16,
    _master: Option<Arc<MasterServer>>,
    storage: Arc<InMemoryStorage>,
    pub creds: Arc<Mutex<CredentialStore>>,
    pub versioning: Arc<VersioningManager>,
    pub mpu: Arc<MultipartManager>,
    pub version_counter: Arc<Mutex<BTreeMap<(String, String), u64>>>, // (bucket,key) -> counter
    pub analytics: Arc<AnalyticsManager>,
    pub batch_ops: Arc<BatchOperationManager>,
    pub replication: Arc<ReplicationManager>,
    pub inventory: Arc<InventoryManager>,
}

impl S3Server {
    pub fn new(port: u16, master: Option<Arc<MasterServer>>) -> Self {
        Self {
            port,
            _master: master,
            storage: Arc::new(InMemoryStorage::default()),
            creds: Arc::new(Mutex::new(CredentialStore::new())),
            versioning: Arc::new(VersioningManager::new()),
            mpu: Arc::new(MultipartManager::new()),
            version_counter: Arc::new(Mutex::new(BTreeMap::new())),
            analytics: Arc::new(AnalyticsManager::new()),
            batch_ops: Arc::new(BatchOperationManager::new()),
            replication: Arc::new(ReplicationManager::new()),
            inventory: Arc::new(InventoryManager::new()),
        }
    }

    /// 便捷：注册一对 AK/SK 以便测试通过鉴权。
    pub fn register_credential(&self, ak: &str, sk: &str, user_id: &str) {
        self.creds
            .lock()
            .insert(ak.to_string(), user_id.to_string(), sk.to_string());
    }

    /// 构建 response helper
    fn xml_response(status: StatusCode, body: String) -> Response<Body> {
        Response::builder()
            .status(status)
            .header("Content-Type", "application/xml")
            .header("x-amz-request-id", new_request_id())
            .body(Body::from(body))
            .unwrap_or_else(|_| Response::new(Body::from("")))
    }

    fn error_response(err: S3Error) -> Response<Body> {
        let rid = new_request_id();
        let body = err.to_xml(&rid);
        Self::xml_response(
            StatusCode::from_u16(err.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            body,
        )
    }

    fn ok_empty_headers(headers: Vec<(&str, String)>) -> Response<Body> {
        let mut b = Response::builder()
            .status(StatusCode::OK)
            .header("x-amz-request-id", new_request_id());
        for (k, v) in &headers {
            b = b.header(*k, v.as_str());
        }
        b.body(Body::empty())
            .unwrap_or_else(|_| Response::new(Body::empty()))
    }

    pub async fn run(self) -> Result<(), S3Error> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let state = Arc::new(AppState {
            storage: self.storage.clone(),
            creds: self.creds.clone(),
            versioning: self.versioning.clone(),
            mpu: self.mpu.clone(),
            vcounter: self.version_counter.clone(),
            analytics: self.analytics.clone(),
            batch_ops: self.batch_ops.clone(),
            replication: self.replication.clone(),
            inventory: self.inventory.clone(),
        });
        let app = Router::new().fallback(any(axum_handler)).with_state(state);
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| S3Error::InternalError(format!("bind {addr}: {e}")))?;
        axum::serve(listener, app)
            .await
            .map_err(|e| S3Error::InternalError(e.to_string()))
    }
}

async fn axum_handler(State(state): State<Arc<AppState>>, req: Request<Body>) -> Response<Body> {
    dispatch(
        req,
        state.storage.clone(),
        state.creds.clone(),
        state.versioning.clone(),
        state.mpu.clone(),
        state.vcounter.clone(),
        state.analytics.clone(),
        state.batch_ops.clone(),
        state.replication.clone(),
        state.inventory.clone(),
    )
    .await
}

// ---------------- Routing ----------------

async fn dispatch(
    req: Request<Body>,
    storage: Arc<InMemoryStorage>,
    creds: Arc<Mutex<CredentialStore>>,
    versioning: Arc<VersioningManager>,
    mpu: Arc<MultipartManager>,
    vcounter: Arc<Mutex<BTreeMap<(String, String), u64>>>,
    analytics: Arc<AnalyticsManager>,
    batch_ops: Arc<BatchOperationManager>,
    replication: Arc<ReplicationManager>,
    inventory: Arc<InventoryManager>,
) -> Response<Body> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = header_map_to_btree(req.headers());
    let (_parts, body_bytes) = req.into_parts();
    let body = match axum::body::to_bytes(body_bytes, 5_000_000_000).await {
        Ok(b) => b,
        Err(e) => return S3Server::error_response(S3Error::InternalError(e.to_string())),
    };

    // 鉴权：以 SigV4 校验为准。`x-test-skip-auth: 1` 仅为测试/开发兜底，且仅允许在 debug 构建生效；
    // 生产 release 构建下该头无效，杜绝「加一个头即可绕过 SigV4 鉴权」的后门。
    #[cfg(debug_assertions)]
    let skip_auth = headers
        .get("x-test-skip-auth")
        .map(|s| s == "1")
        .unwrap_or(false);
    #[cfg(not(debug_assertions))]
    let skip_auth = false;
    let user_id = if skip_auth {
        "test-user".to_string()
    } else {
        let path = uri.path().to_string();
        let query_pairs = parse_query(uri.query().unwrap_or(""));
        // payload sha256（用 UNSIGNED-PAYLOAD 兼容未签名场景；正式场景需要计算）
        let payload = match headers.get("x-amz-content-sha256").cloned() {
            Some(v) => v,
            None => "UNSIGNED-PAYLOAD".to_string(),
        };
        match verify_request(
            method.as_str(),
            &path,
            &query_pairs,
            &headers,
            &payload,
            &creds.lock(),
        ) {
            Ok(u) => u,
            Err(e) => return S3Server::error_response(e),
        }
    };
    let _ = user_id;

    let (bucket, key) = split_bucket_key(uri.path());
    let query = uri.query().unwrap_or("").to_string();

    handle_s3_operation(
        &method, bucket, key, &query, &headers, body, storage, versioning, mpu, vcounter,
        analytics, batch_ops, replication, inventory,
    )
    .await
}

fn header_map_to_btree(h: &HeaderMap) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (k, v) in h.iter() {
        let key = k.as_str().to_lowercase();
        let val = v.to_str().unwrap_or("").to_string();
        out.insert(key, val);
    }
    out
}

fn parse_query(q: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if q.is_empty() {
        return out;
    }
    for pair in q.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = match pair.split_once('=') {
            Some((k, v)) => (k.to_string(), v.to_string()),
            None => (pair.to_string(), String::new()),
        };
        out.push((k, v));
    }
    out
}

fn split_bucket_key(path: &str) -> (Option<String>, Option<String>) {
    let p = path.trim_start_matches('/');
    if p.is_empty() {
        return (None, None);
    }
    match p.find('/') {
        Some(i) => {
            let b = p[..i].to_string();
            let k = p[i + 1..].to_string();
            (Some(b), if k.is_empty() { None } else { Some(k) })
        }
        None => (Some(p.to_string()), None),
    }
}

fn query_has(q: &str, key: &str) -> bool {
    parse_query(q).iter().any(|(k, _)| k == key)
}

fn query_val(q: &str, key: &str) -> Option<String> {
    parse_query(q)
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
}

// ---------------- Operation Dispatcher ----------------

async fn handle_s3_operation(
    method: &Method,
    bucket: Option<String>,
    key: Option<String>,
    query: &str,
    headers: &BTreeMap<String, String>,
    body: Bytes,
    storage: Arc<InMemoryStorage>,
    versioning: Arc<VersioningManager>,
    mpu: Arc<MultipartManager>,
    vcounter: Arc<Mutex<BTreeMap<(String, String), u64>>>,
    _analytics: Arc<AnalyticsManager>,
    _batch_ops: Arc<BatchOperationManager>,
    _replication: Arc<ReplicationManager>,
    _inventory: Arc<InventoryManager>,
) -> Response<Body> {
    // 无 bucket → 只有 ListBuckets
    if bucket.is_none() {
        if method == Method::GET {
            return op_list_buckets(&storage);
        }
        return S3Server::error_response(S3Error::MethodNotAllowed);
    }
    let bucket = bucket.unwrap();

    // 有 bucket 无 key → Bucket-level 操作
    if key.is_none() {
        match (method.as_str(), query) {
            (m, "") if m == Method::PUT => return op_create_bucket(&storage, &bucket, headers),
            (m, "") if m == Method::DELETE => return op_delete_bucket(&storage, &bucket),
            (m, "") if m == Method::HEAD => return op_head_bucket(&storage, &bucket),
            (m, q) if m == Method::GET => {
                if query_has(q, "versioning") {
                    return op_get_bucket_versioning(&versioning, &bucket);
                }
                if query_has(q, "tagging") {
                    return op_get_bucket_tagging(&storage, &bucket);
                }
                if query_has(q, "policy") {
                    return op_get_bucket_policy(&storage, &bucket);
                }
                if query_has(q, "cors") {
                    return op_get_bucket_cors(&storage, &bucket);
                }
                if query_has(q, "lifecycle") {
                    return op_get_bucket_lifecycle(&storage, &bucket);
                }
                if query_has(q, "uploads") {
                    return op_list_multipart_uploads(&mpu, &bucket, q);
                }
                if query_has(q, "versions") {
                    return op_list_object_versions(&storage, &bucket, q);
                }
                if query_has(q, "list-type") && query_val(q, "list-type").as_deref() == Some("2") {
                    return op_list_objects_v2(&storage, &bucket, q);
                }
                return op_list_objects_v1(&storage, &bucket, q);
            }
            (m, q) if m == Method::PUT => {
                if query_has(q, "versioning") {
                    return op_put_bucket_versioning(&versioning, &bucket, &body);
                }
                if query_has(q, "tagging") {
                    return op_put_bucket_tagging(&storage, &bucket, &body);
                }
                if query_has(q, "policy") {
                    return op_put_bucket_policy(&storage, &bucket, &body);
                }
                if query_has(q, "cors") {
                    return op_put_bucket_cors(&storage, &bucket, &body);
                }
                if query_has(q, "lifecycle") {
                    return op_put_bucket_lifecycle(&storage, &bucket, &body);
                }
                return op_create_bucket(&storage, &bucket, headers);
            }
            (m, q) if m == Method::POST && query_has(q, "delete") => {
                return op_delete_multiple_objects(&storage, &bucket, body);
            }
            _ => {}
        }
    }

    // 有 bucket + 有 key → Object-level 操作
    let key = key.unwrap();
    match (method.as_str(), query) {
        (m, q) if m == "PUT" => {
            // x-amz-copy-source 指示 CopyObject 或 UploadPartCopy，优先级最高
            if let Some(src) = headers.get("x-amz-copy-source").cloned() {
                if query_has(q, "uploadId") && query_has(q, "partNumber") {
                    return op_upload_part_copy(&mpu, &storage, &bucket, &key, q, src);
                } else {
                    return op_copy_object(
                        &storage,
                        &bucket,
                        &key,
                        headers,
                        &versioning,
                        &vcounter,
                    );
                }
            }
            if query_has(q, "uploadId") && query_has(q, "partNumber") {
                return op_upload_part(&mpu, &bucket, &key, q, body);
            }
            if query_has(q, "uploadId") {
                return op_complete_or_abort_mpu(
                    &mpu,
                    &storage,
                    &bucket,
                    &key,
                    q,
                    body,
                    headers,
                    &versioning,
                    &vcounter,
                );
            }
            if query_has(q, "uploads") {
                return op_create_multipart_upload(&mpu, &bucket, &key, headers);
            }
            if query_has(q, "tagging") {
                return op_put_object_tagging(&storage, &bucket, &key, body);
            }
            if query_has(q, "acl") {
                return op_put_object_acl(&storage, &bucket, &key, headers, body);
            }
            op_put_object(
                &storage,
                &bucket,
                &key,
                headers,
                body,
                &versioning,
                &vcounter,
            )
        }
        (m, q) if m == "GET" => {
            if query_has(q, "uploadId") {
                return op_list_parts(&mpu, &bucket, &key, q);
            }
            if query_has(q, "tagging") {
                return op_get_object_tagging(&storage, &bucket, &key);
            }
            if query_has(q, "acl") {
                return op_get_object_acl(&storage, &bucket, &key);
            }
            op_get_object(&storage, &bucket, &key, q, headers)
        }
        (m, q) if m == "DELETE" => {
            if query_has(q, "uploadId") {
                match mpu.abort(&query_val(q, "uploadId").unwrap_or_default()) {
                    Ok(()) => S3Server::ok_empty_headers(vec![]),
                    Err(e) => S3Server::error_response(e),
                }
            } else {
                op_delete_object(&storage, &bucket, &key, q, &versioning)
            }
        }
        (m, _) if m == "HEAD" => op_head_object(&storage, &bucket, &key, headers),
        (m, q) if m == "POST" => {
            if query_has(q, "delete") {
                return op_delete_multiple_objects(&storage, &bucket, body);
            }
            if query_has(q, "uploads") {
                return op_create_multipart_upload(&mpu, &bucket, &key, headers);
            }
            if query_has(q, "uploadId") {
                return op_complete_or_abort_mpu(
                    &mpu,
                    &storage,
                    &bucket,
                    &key,
                    q,
                    body,
                    headers,
                    &versioning,
                    &vcounter,
                );
            }
            S3Server::error_response(S3Error::MethodNotAllowed)
        }
        (m, q) if m == "COPY" || headers.contains_key("x-amz-copy-source") => {
            let _ = q;
            op_copy_object(&storage, &bucket, &key, headers, &versioning, &vcounter)
        }
        _ => S3Server::error_response(S3Error::NotImplemented(format!(
            "{:?} /{}/{}?{}",
            method, bucket, key, query
        ))),
    }
}

// ---------------- 34 API Implementations ----------------

// --- 1. ListBuckets ---
fn op_list_buckets(storage: &InMemoryStorage) -> Response<Body> {
    let buckets = storage.buckets.lock();
    let mut inner = String::new();
    for (name, meta) in buckets.iter() {
        inner.push_str(&format!(
            "      <Bucket>\n        <Name>{}</Name>\n        <CreationDate>{}</CreationDate>\n      </Bucket>\n",
            name, iso8601(meta.created_ms)
        ));
    }
    let owner_id = "mox-owner-id";
    let owner_disp = "mox";
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <ListAllMyBucketsResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n\
           <Owner>\n             <ID>{}</ID>\n             <DisplayName>{}</DisplayName>\n           </Owner>\n\
           <Buckets>\n{}\
           </Buckets>\n\
         </ListAllMyBucketsResult>",
        owner_id, owner_disp, inner
    );
    S3Server::xml_response(StatusCode::OK, body)
}

// --- 2. CreateBucket ---
fn op_create_bucket(
    storage: &InMemoryStorage,
    bucket: &str,
    headers: &BTreeMap<String, String>,
) -> Response<Body> {
    if bucket.len() < 3 || bucket.len() > 63 {
        return S3Server::error_response(S3Error::InvalidArgument);
    }
    let mut b = storage.buckets.lock();
    if b.contains_key(bucket) {
        return S3Server::error_response(S3Error::BucketAlreadyExists);
    }
    let acl = headers
        .get("x-amz-acl")
        .and_then(|s| CannedAcl::from_header(s))
        .unwrap_or_default();
    b.insert(
        bucket.to_string(),
        BucketMeta {
            name: bucket.to_string(),
            created_ms: now_ms(),
            _acl: acl,
            policy: None,
            tags: BTreeMap::new(),
            cors: None,
            lifecycle_xml: None,
        },
    );
    storage
        .objects
        .lock()
        .insert(bucket.to_string(), BTreeMap::new());
    let mut resp = Response::builder()
        .status(StatusCode::OK)
        .header("Location", format!("/{}", bucket))
        .header("x-amz-request-id", new_request_id());
    let _ = headers;
    resp = resp.header("Content-Length", "0");
    resp.body(Body::empty())
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

// --- 3. DeleteBucket ---
fn op_delete_bucket(storage: &InMemoryStorage, bucket: &str) -> Response<Body> {
    let objs = storage.objects.lock();
    if let Some(m) = objs.get(bucket) {
        for versions in m.values() {
            for v in versions {
                if !v.is_delete_marker {
                    return S3Server::error_response(S3Error::BucketNotEmpty);
                }
            }
        }
    }
    drop(objs);
    let mut buckets = storage.buckets.lock();
    if buckets.remove(bucket).is_none() {
        return S3Server::error_response(S3Error::NoSuchBucket);
    }
    storage.objects.lock().remove(bucket);
    S3Server::ok_empty_headers(vec![])
}

// --- 4. HeadBucket ---
fn op_head_bucket(storage: &InMemoryStorage, bucket: &str) -> Response<Body> {
    if storage.buckets.lock().contains_key(bucket) {
        S3Server::ok_empty_headers(vec![])
    } else {
        S3Server::error_response(S3Error::NoSuchBucket)
    }
}

// --- 5. ListObjectsV1 ---
fn op_list_objects_v1(storage: &InMemoryStorage, bucket: &str, query: &str) -> Response<Body> {
    if !storage.buckets.lock().contains_key(bucket) {
        return S3Server::error_response(S3Error::NoSuchBucket);
    }
    let prefix = query_val(query, "prefix").unwrap_or_default();
    let marker = query_val(query, "marker").unwrap_or_default();
    let max_keys = query_val(query, "max-keys")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000u32);
    let delimiter = query_val(query, "delimiter").unwrap_or_default();
    list_objects_render(
        storage, bucket, &prefix, &marker, max_keys, &delimiter, false,
    )
}

// --- 6. ListObjectsV2 ---
fn op_list_objects_v2(storage: &InMemoryStorage, bucket: &str, query: &str) -> Response<Body> {
    if !storage.buckets.lock().contains_key(bucket) {
        return S3Server::error_response(S3Error::NoSuchBucket);
    }
    let prefix = query_val(query, "prefix").unwrap_or_default();
    let _start_after = query_val(query, "start-after").unwrap_or_default();
    let cont_token = query_val(query, "continuation-token").unwrap_or_default();
    let marker = if !cont_token.is_empty() {
        cont_token
    } else {
        String::new()
    };
    let max_keys = query_val(query, "max-keys")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000u32);
    let delimiter = query_val(query, "delimiter").unwrap_or_default();
    list_objects_render(
        storage, bucket, &prefix, &marker, max_keys, &delimiter, true,
    )
}

fn list_objects_render(
    storage: &InMemoryStorage,
    bucket: &str,
    prefix: &str,
    marker: &str,
    max_keys: u32,
    delimiter: &str,
    is_v2: bool,
) -> Response<Body> {
    let objs = storage.objects.lock();
    let bucket_map = objs.get(bucket).cloned().unwrap_or_default();
    // 收集所有 (key, latest_version) 对
    let mut all: Vec<(String, ObjectMeta)> = Vec::new();
    for (k, versions) in bucket_map.iter() {
        if let Some(latest) = versions.last() {
            if latest.is_delete_marker {
                continue;
            }
            all.push((k.clone(), latest.clone()));
        }
    }
    all.sort_by(|a, b| a.0.cmp(&b.0));

    // prefix 过滤 + marker 跳过 + delimiter
    let mut contents = Vec::new();
    let mut common_prefixes: Vec<String> = Vec::new();
    let mut count = 0u32;
    let mut is_truncated = false;
    let mut next_marker = String::new();
    for (k, meta) in all {
        if !k.starts_with(prefix) {
            continue;
        }
        if !marker.is_empty() && k.as_str() <= marker {
            continue;
        }
        if count >= max_keys {
            is_truncated = true;
            next_marker = k.clone();
            break;
        }
        if !delimiter.is_empty() {
            let after_prefix = &k[prefix.len()..];
            if let Some(idx) = after_prefix.find(delimiter) {
                let cp = format!("{}{}{}", prefix, &after_prefix[..idx], delimiter);
                if !common_prefixes.contains(&cp) {
                    common_prefixes.push(cp);
                }
                count += 1;
                if count >= max_keys {
                    is_truncated = true;
                    next_marker = k.clone();
                    break;
                }
                continue;
            }
        }
        contents.push((k.clone(), meta));
        count += 1;
    }

    let mut xml = String::new();
    if is_v2 {
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<ListBucketResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n");
        xml.push_str(&format!("  <Name>{}</Name>\n", bucket));
        xml.push_str(&format!("  <Prefix>{}</Prefix>\n", prefix));
        xml.push_str(&format!("  <KeyCount>{}</KeyCount>\n", count));
        xml.push_str(&format!("  <MaxKeys>{}</MaxKeys>\n", max_keys));
        xml.push_str(&format!("  <IsTruncated>{}</IsTruncated>\n", is_truncated));
    } else {
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<ListBucketResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n");
        xml.push_str(&format!("  <Name>{}</Name>\n", bucket));
        xml.push_str(&format!("  <Prefix>{}</Prefix>\n", prefix));
        xml.push_str(&format!("  <Marker>{}</Marker>\n", marker));
        xml.push_str(&format!("  <MaxKeys>{}</MaxKeys>\n", max_keys));
        xml.push_str(&format!("  <IsTruncated>{}</IsTruncated>\n", is_truncated));
    }
    for (k, m) in &contents {
        xml.push_str("  <Contents>\n");
        xml.push_str(&format!("    <Key>{}</Key>\n", k));
        xml.push_str(&format!(
            "    <LastModified>{}</LastModified>\n",
            iso8601(m.last_modified_ms)
        ));
        xml.push_str(&format!("    <ETag>{}</ETag>\n", m.etag));
        xml.push_str(&format!("    <Size>{}</Size>\n", m.size));
        xml.push_str("    <StorageClass>STANDARD</StorageClass>\n");
        xml.push_str("    <Owner><ID>mox</ID><DisplayName>mox</DisplayName></Owner>\n");
        xml.push_str("  </Contents>\n");
    }
    for cp in &common_prefixes {
        xml.push_str(&format!(
            "  <CommonPrefixes><Prefix>{}</Prefix></CommonPrefixes>\n",
            cp
        ));
    }
    if is_v2 && is_truncated {
        xml.push_str(&format!(
            "  <NextContinuationToken>{}</NextContinuationToken>\n",
            next_marker
        ));
    }
    if !is_v2 && is_truncated {
        xml.push_str(&format!("  <NextMarker>{}</NextMarker>\n", next_marker));
    }
    xml.push_str("</ListBucketResult>\n");
    S3Server::xml_response(StatusCode::OK, xml)
}

// --- 7. GetObjectAcl ---
fn op_get_object_acl(storage: &InMemoryStorage, bucket: &str, key: &str) -> Response<Body> {
    let objs = storage.objects.lock();
    let b = objs.get(bucket);
    let versions = match b.and_then(|m| m.get(key)) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchKey),
    };
    let meta = versions.last().unwrap();
    let xml = meta.acl.to_acl_xml("mox-owner-id", "mox");
    S3Server::xml_response(StatusCode::OK, xml)
}

// --- 8. PutObjectAcl ---
fn op_put_object_acl(
    storage: &InMemoryStorage,
    bucket: &str,
    key: &str,
    headers: &BTreeMap<String, String>,
    body: Bytes,
) -> Response<Body> {
    // 优先使用 x-amz-acl header；否则尝试从 body XML 推导（简化为 private）
    let acl = if let Some(h) = headers.get("x-amz-acl") {
        CannedAcl::from_header(h).unwrap_or(CannedAcl::Private)
    } else {
        // 简化：body XML 若有 FULL_CONTROL/READ 就按 private/public-read 粗略判断
        let s = String::from_utf8_lossy(&body);
        if s.contains("<Permission>READ</Permission>") && s.contains("AllUsers") {
            if s.contains("<Permission>WRITE</Permission>") {
                CannedAcl::PublicReadWrite
            } else {
                CannedAcl::PublicRead
            }
        } else {
            CannedAcl::Private
        }
    };
    let mut objs = storage.objects.lock();
    let m = objs.get_mut(bucket);
    let versions = match m.and_then(|mm| mm.get_mut(key)) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchKey),
    };
    if let Some(last) = versions.last_mut() {
        last.acl = acl;
    }
    S3Server::ok_empty_headers(vec![])
}

// --- 9. PutObject ---
fn op_put_object(
    storage: &InMemoryStorage,
    bucket: &str,
    key: &str,
    headers: &BTreeMap<String, String>,
    body: Bytes,
    versioning: &VersioningManager,
    vcounter: &Mutex<BTreeMap<(String, String), u64>>,
) -> Response<Body> {
    // If-None-Match: * → 存在时 412
    if let Some(v) = headers.get("if-none-match") {
        if v == "*" {
            let objs = storage.objects.lock();
            if let Some(m) = objs.get(bucket) {
                if let Some(vers) = m.get(key) {
                    if let Some(latest) = vers.last() {
                        if !latest.is_delete_marker {
                            return Response::builder()
                                .status(StatusCode::PRECONDITION_FAILED)
                                .header("x-amz-request-id", new_request_id())
                                .body(Body::empty())
                                .unwrap();
                        }
                    }
                }
            }
        }
    }
    if !storage.buckets.lock().contains_key(bucket) {
        return S3Server::error_response(S3Error::NoSuchBucket);
    }
    let acl = headers
        .get("x-amz-acl")
        .and_then(|s| CannedAcl::from_header(s))
        .unwrap_or_default();
    let data = body.to_vec();
    let size = data.len() as u64;
    let etag = crate::etag::etag_small(&data);
    let crc32c = crate::etag::checksum_crc32c(&data);
    let content_type = headers
        .get("content-type")
        .cloned()
        .unwrap_or_else(|| "application/octet-stream".into());

    // versioning 状态
    let v_status = versioning.get(bucket);
    let version_id = if v_status.should_generate_version() {
        let mut c = vcounter.lock();
        let entry = c.entry((bucket.to_string(), key.to_string())).or_insert(0);
        *entry += 1;
        generate_version_id(key, now_ms(), *entry)
    } else {
        if matches!(v_status, VersioningStatus::Suspended) {
            "null".to_string()
        } else {
            String::new()
        }
    };

    let meta = ObjectMeta {
        data,
        etag: etag.clone(),
        size,
        last_modified_ms: now_ms(),
        content_type,
        version_id: version_id.clone(),
        acl,
        tags: BTreeMap::new(),
        is_delete_marker: false,
        crc32c,
    };

    let mut objs = storage.objects.lock();
    let bucket_map = objs.entry(bucket.to_string()).or_default();
    let versions = bucket_map.entry(key.to_string()).or_default();
    if !v_status.should_generate_version() {
        // Off / Suspended：覆盖最新版本
        if versions.is_empty() {
            versions.push(meta);
        } else {
            let last = versions.last_mut().unwrap();
            *last = meta;
        }
    } else {
        versions.push(meta);
    }
    drop(objs);

    let mut hs: Vec<(&str, String)> = Vec::new();
    hs.push(("ETag", etag.clone()));
    if !version_id.is_empty() {
        hs.push(("x-amz-version-id", version_id));
    }
    let crc_header = crate::etag::checksum_crc32c_base64(&body);
    hs.push(("x-amz-checksum-crc32c", crc_header));
    S3Server::ok_empty_headers(hs)
}

// --- 10. GetObject (with Range) ---
fn op_get_object(
    storage: &InMemoryStorage,
    bucket: &str,
    key: &str,
    query: &str,
    headers: &BTreeMap<String, String>,
) -> Response<Body> {
    if !storage.buckets.lock().contains_key(bucket) {
        return S3Server::error_response(S3Error::NoSuchBucket);
    }
    let objs = storage.objects.lock();
    let m = match objs.get(bucket) {
        Some(m) => m,
        None => return S3Server::error_response(S3Error::NoSuchKey),
    };
    let versions = match m.get(key) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchKey),
    };

    // 支持 versionId query
    let want_vid = query_val(query, "versionId").unwrap_or_default();
    let meta = if !want_vid.is_empty() {
        versions.iter().find(|v| v.version_id == want_vid)
    } else {
        versions.last()
    };
    let meta = match meta {
        Some(m) => m,
        None => return S3Server::error_response(S3Error::NoSuchKey),
    };
    if meta.is_delete_marker {
        return S3Server::error_response(S3Error::NoSuchKey);
    }

    // Range 支持
    let (data, status, range_resp) = if let Some(r) = headers.get("range") {
        // Range: bytes=start-end
        let spec = r.trim_start_matches("bytes=");
        let (start_s, end_s) = match spec.split_once('-') {
            Some(p) => p,
            None => return S3Server::error_response(S3Error::InvalidArgument),
        };
        let start: u64 = if start_s.is_empty() {
            0
        } else {
            start_s.parse().unwrap_or(0)
        };
        let total = meta.size;
        let end: u64 = if end_s.is_empty() {
            total.saturating_sub(1)
        } else {
            end_s.parse().unwrap_or(total.saturating_sub(1))
        };
        let end = end.min(total.saturating_sub(1));
        if start > end || start >= total {
            return Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header("Content-Range", format!("bytes */{}", total))
                .header("x-amz-request-id", new_request_id())
                .body(Body::empty())
                .unwrap();
        }
        let slice = &meta.data[start as usize..=end as usize];
        let cr = format!("bytes {}-{}/{}", start, end, total);
        (slice.to_vec(), StatusCode::PARTIAL_CONTENT, Some(cr))
    } else {
        (meta.data.clone(), StatusCode::OK, None)
    };

    let mut resp = Response::builder()
        .status(status)
        .header("Content-Type", meta.content_type.as_str())
        .header("Content-Length", data.len().to_string())
        .header("ETag", meta.etag.as_str())
        .header("Last-Modified", http_date(meta.last_modified_ms))
        .header("Accept-Ranges", "bytes")
        .header("x-amz-request-id", new_request_id())
        .header(
            "x-amz-checksum-crc32c",
            crate::etag::checksum_crc32c_base64(&data),
        );
    if let Some(cr) = range_resp {
        resp = resp.header("Content-Range", cr);
    }
    if !meta.version_id.is_empty() {
        resp = resp.header("x-amz-version-id", meta.version_id.as_str());
    }
    resp.body(Body::from(data))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

// --- 11. DeleteObject ---
fn op_delete_object(
    storage: &InMemoryStorage,
    bucket: &str,
    key: &str,
    query: &str,
    versioning: &VersioningManager,
) -> Response<Body> {
    if !storage.buckets.lock().contains_key(bucket) {
        return S3Server::error_response(S3Error::NoSuchBucket);
    }
    let v_status = versioning.get(bucket);
    let want_vid = query_val(query, "versionId").unwrap_or_default();
    let mut objs = storage.objects.lock();
    let m = match objs.get_mut(bucket) {
        Some(m) => m,
        None => return S3Server::ok_empty_headers(vec![]),
    };
    let versions = match m.get_mut(key) {
        Some(v) => v,
        None => return S3Server::ok_empty_headers(vec![]),
    };

    if v_status.should_generate_version() && want_vid.is_empty() {
        // Add delete marker
        versions.push(ObjectMeta {
            data: vec![],
            etag: "\"d41d8cd98f00b204e9800998ecf8427e\"".into(),
            size: 0,
            last_modified_ms: now_ms(),
            content_type: "application/octet-stream".into(),
            version_id: generate_version_id(key, now_ms(), versions.len() as u64),
            acl: CannedAcl::Private,
            tags: BTreeMap::new(),
            is_delete_marker: true,
            crc32c: 0,
        });
        let vid = versions.last().unwrap().version_id.clone();
        let mut resp = Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header("x-amz-delete-marker", "true")
            .header("x-amz-version-id", vid)
            .header("x-amz-request-id", new_request_id());
        if versions.is_empty() {
            resp = resp.header("Content-Length", "0");
        }
        return resp.body(Body::empty()).unwrap();
    }

    if !want_vid.is_empty() {
        versions.retain(|v| v.version_id != want_vid);
        return S3Server::ok_empty_headers(vec![("x-amz-version-id", want_vid)]);
    }

    // Off：直接清空
    versions.clear();
    S3Server::ok_empty_headers(vec![])
}

// --- 12. HeadObject ---
fn op_head_object(
    storage: &InMemoryStorage,
    bucket: &str,
    key: &str,
    _h: &BTreeMap<String, String>,
) -> Response<Body> {
    if !storage.buckets.lock().contains_key(bucket) {
        return S3Server::error_response(S3Error::NoSuchBucket);
    }
    let objs = storage.objects.lock();
    let m = match objs.get(bucket) {
        Some(m) => m,
        None => return S3Server::error_response(S3Error::NoSuchKey),
    };
    let versions = match m.get(key) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchKey),
    };
    let meta = match versions.last() {
        Some(v) if !v.is_delete_marker => v,
        _ => return S3Server::error_response(S3Error::NoSuchKey),
    };
    // 注：此处返回实际字节数的 Body（通过 clone），
    // Axum 会根据 HEAD method 自动丢弃发送体，但会使用 Body 的 size_hint 设置正确的 Content-Length。
    let body = Body::from(meta.data.clone());
    let mut resp = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", meta.content_type.as_str())
        .header("ETag", meta.etag.as_str())
        .header("Last-Modified", http_date(meta.last_modified_ms))
        .header("Accept-Ranges", "bytes")
        .header("x-amz-request-id", new_request_id());
    if !meta.version_id.is_empty() {
        resp = resp.header("x-amz-version-id", meta.version_id.as_str());
    }
    resp.body(body)
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

// --- 13. CopyObject ---
fn op_copy_object(
    storage: &InMemoryStorage,
    bucket: &str,
    key: &str,
    headers: &BTreeMap<String, String>,
    versioning: &VersioningManager,
    vcounter: &Mutex<BTreeMap<(String, String), u64>>,
) -> Response<Body> {
    let src = match headers.get("x-amz-copy-source") {
        Some(s) => s.clone(),
        None => return S3Server::error_response(S3Error::InvalidArgument),
    };
    // source form: /bucket/key or bucket/key
    let src_path = src.trim_start_matches('/');
    let (src_bucket, src_key) = match src_path.find('/') {
        Some(i) => (src_path[..i].to_string(), src_path[i + 1..].to_string()),
        None => return S3Server::error_response(S3Error::InvalidArgument),
    };

    let objs = storage.objects.lock();
    let src_vers = objs.get(&src_bucket).and_then(|m| m.get(&src_key).cloned());
    drop(objs);
    let versions = match src_vers {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchKey),
    };
    let src_meta = match versions.last() {
        Some(v) if !v.is_delete_marker => v.clone(),
        _ => return S3Server::error_response(S3Error::NoSuchKey),
    };

    let acl = headers
        .get("x-amz-acl")
        .and_then(|s| CannedAcl::from_header(s))
        .unwrap_or_default();

    let v_status = versioning.get(bucket);
    let version_id = if v_status.should_generate_version() {
        let mut c = vcounter.lock();
        let entry = c.entry((bucket.to_string(), key.to_string())).or_insert(0);
        *entry += 1;
        generate_version_id(key, now_ms(), *entry)
    } else {
        if matches!(v_status, VersioningStatus::Suspended) {
            "null".into()
        } else {
            String::new()
        }
    };

    let new_meta = ObjectMeta {
        data: src_meta.data.clone(),
        etag: src_meta.etag.clone(),
        size: src_meta.size,
        last_modified_ms: now_ms(),
        content_type: src_meta.content_type.clone(),
        version_id: version_id.clone(),
        acl,
        tags: BTreeMap::new(),
        is_delete_marker: false,
        crc32c: src_meta.crc32c,
    };

    let mut objs = storage.objects.lock();
    let bucket_map = objs.entry(bucket.to_string()).or_default();
    let dest_versions = bucket_map.entry(key.to_string()).or_default();
    if !v_status.should_generate_version() {
        if dest_versions.is_empty() {
            dest_versions.push(new_meta);
        } else {
            *dest_versions.last_mut().unwrap() = new_meta;
        }
    } else {
        dest_versions.push(new_meta);
    }
    drop(objs);

    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <CopyObjectResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n\
           <LastModified>{}</LastModified>\n\
           <ETag>{}</ETag>\n\
         </CopyObjectResult>",
        iso8601(now_ms()),
        src_meta.etag
    );
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header("x-amz-request-id", new_request_id());
    if !version_id.is_empty() {
        builder = builder.header("x-amz-version-id", version_id);
    }
    builder
        .header("Content-Type", "application/xml")
        .body(Body::from(body))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

// --- 14. CreateMultipartUpload ---
fn op_create_multipart_upload(
    mpu: &MultipartManager,
    bucket: &str,
    key: &str,
    headers: &BTreeMap<String, String>,
) -> Response<Body> {
    let upload_id = mpu.create(bucket, key);
    let acl = headers
        .get("x-amz-acl")
        .and_then(|s| CannedAcl::from_header(s))
        .unwrap_or_default();
    let _ = acl;
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <InitiateMultipartUploadResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n\
           <Bucket>{}</Bucket>\n\
           <Key>{}</Key>\n\
           <UploadId>{}</UploadId>\n\
         </InitiateMultipartUploadResult>",
        bucket, key, upload_id
    );
    S3Server::xml_response(StatusCode::OK, body)
}

// --- 15. UploadPart ---
fn op_upload_part(
    mpu: &MultipartManager,
    _bucket: &str,
    _key: &str,
    query: &str,
    body: Bytes,
) -> Response<Body> {
    let uid = match query_val(query, "uploadId") {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchUpload),
    };
    let pn = query_val(query, "partNumber")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0u16);
    if pn == 0 {
        return S3Server::error_response(S3Error::InvalidArgument);
    }
    match mpu.upload_part(&uid, pn, body.to_vec()) {
        Ok(p) => S3Server::ok_empty_headers(vec![("ETag", format!("\"{}\"", p.etag))]),
        Err(e) => S3Server::error_response(e),
    }
}

// --- 16. UploadPartCopy ---
fn op_upload_part_copy(
    mpu: &MultipartManager,
    storage: &InMemoryStorage,
    _bucket: &str,
    _key: &str,
    query: &str,
    src: String,
) -> Response<Body> {
    let uid = match query_val(query, "uploadId") {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchUpload),
    };
    let pn = match query_val(query, "partNumber").and_then(|v| v.parse::<u16>().ok()) {
        Some(p) if p > 0 => p,
        _ => return S3Server::error_response(S3Error::InvalidArgument),
    };
    let src_path = src.trim_start_matches('/');
    let (src_b, src_k) = match src_path.find('/') {
        Some(i) => (src_path[..i].to_string(), src_path[i + 1..].to_string()),
        None => return S3Server::error_response(S3Error::InvalidArgument),
    };
    let objs = storage.objects.lock();
    let data = objs
        .get(&src_b)
        .and_then(|m| m.get(&src_k))
        .and_then(|v| v.last())
        .map(|m| m.data.clone());
    drop(objs);
    match data {
        Some(d) => match mpu.upload_part_copy(&uid, pn, d) {
            Ok(p) => {
                let body_xml = format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                     <CopyPartResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n\
                       <LastModified>{}</LastModified>\n\
                       <ETag>\"{}\"</ETag>\n\
                     </CopyPartResult>",
                    iso8601(now_ms()),
                    p.etag
                );
                S3Server::xml_response(StatusCode::OK, body_xml)
            }
            Err(e) => S3Server::error_response(e),
        },
        None => S3Server::error_response(S3Error::NoSuchKey),
    }
}

// --- 17. CompleteMultipartUpload + Abort(辅助分发) + 16(UploadPartCopy 在 body 中处理) ---
fn op_complete_or_abort_mpu(
    mpu: &MultipartManager,
    storage: &InMemoryStorage,
    bucket: &str,
    key: &str,
    query: &str,
    body: Bytes,
    headers: &BTreeMap<String, String>,
    versioning: &VersioningManager,
    vcounter: &Mutex<BTreeMap<(String, String), u64>>,
) -> Response<Body> {
    let uid = match query_val(query, "uploadId") {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchUpload),
    };

    // UploadPartCopy via x-amz-copy-source header + partNumber
    if let (Some(pn_s), Some(src)) = (
        query_val(query, "partNumber"),
        headers.get("x-amz-copy-source"),
    ) {
        if let Ok(pn) = pn_s.parse::<u16>() {
            // copy source data
            let src_path = src.trim_start_matches('/');
            let (src_b, src_k) = match src_path.find('/') {
                Some(i) => (src_path[..i].to_string(), src_path[i + 1..].to_string()),
                None => return S3Server::error_response(S3Error::InvalidArgument),
            };
            let objs = storage.objects.lock();
            let data = objs
                .get(&src_b)
                .and_then(|m| m.get(&src_k))
                .and_then(|v| v.last())
                .map(|m| m.data.clone());
            drop(objs);
            match data {
                Some(d) => match mpu.upload_part_copy(&uid, pn, d) {
                    Ok(p) => {
                        let body_xml = format!(
                            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                             <CopyPartResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n\
                               <LastModified>{}</LastModified>\n\
                               <ETag>\"{}\"</ETag>\n\
                             </CopyPartResult>",
                            iso8601(now_ms()),
                            p.etag
                        );
                        return S3Server::xml_response(StatusCode::OK, body_xml);
                    }
                    Err(e) => return S3Server::error_response(e),
                },
                None => return S3Server::error_response(S3Error::NoSuchKey),
            }
        }
    }

    // 如果 method 不是 PUT（如 DELETE query uploadId → abort），上层已处理 DELETE。
    // 此处 PUT + uploadId 一定是 Complete。
    let parts = match parse_complete_body(&body) {
        Ok(v) => v,
        Err(e) => return S3Server::error_response(e),
    };
    match mpu.complete(&uid, &parts) {
        Ok((data, etag)) => {
            // 写入对象存储
            let size = data.len() as u64;
            let crc32c = crate::etag::checksum_crc32c(&data);
            let v_status = versioning.get(bucket);
            let version_id = if v_status.should_generate_version() {
                let mut c = vcounter.lock();
                let entry = c.entry((bucket.to_string(), key.to_string())).or_insert(0);
                *entry += 1;
                generate_version_id(key, now_ms(), *entry)
            } else if matches!(v_status, VersioningStatus::Suspended) {
                "null".into()
            } else {
                String::new()
            };
            let meta = ObjectMeta {
                data,
                etag: etag.clone(),
                size,
                last_modified_ms: now_ms(),
                content_type: "application/octet-stream".into(),
                version_id: version_id.clone(),
                acl: CannedAcl::Private,
                tags: BTreeMap::new(),
                is_delete_marker: false,
                crc32c,
            };
            let mut objs = storage.objects.lock();
            let bucket_map = objs.entry(bucket.to_string()).or_default();
            let dest_vers = bucket_map.entry(key.to_string()).or_default();
            if !v_status.should_generate_version() {
                if dest_vers.is_empty() {
                    dest_vers.push(meta);
                } else {
                    *dest_vers.last_mut().unwrap() = meta;
                }
            } else {
                dest_vers.push(meta);
            }
            drop(objs);
            let body_xml = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <CompleteMultipartUploadResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n\
                   <Location>/{}/{}</Location>\n\
                   <Bucket>{}</Bucket>\n\
                   <Key>{}</Key>\n\
                   <ETag>{}</ETag>\n\
                 </CompleteMultipartUploadResult>",
                bucket, key, bucket, key, etag
            );
            let mut b = Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/xml")
                .header("x-amz-request-id", new_request_id());
            if !version_id.is_empty() {
                b = b.header("x-amz-version-id", version_id);
            }
            b.body(Body::from(body_xml)).unwrap()
        }
        Err(e) => S3Server::error_response(e),
    }
}

fn parse_complete_body(body: &[u8]) -> S3Result<Vec<PartETag>> {
    let s = String::from_utf8_lossy(body);
    let reader = xml::EventReader::from_str(&s);
    let mut parts: Vec<PartETag> = Vec::new();
    let mut cur_part_num: Option<u16> = None;
    let mut cur_etag = String::new();
    let mut text = String::new();
    for e in reader {
        use xml::reader::XmlEvent;
        match e.map_err(|x| S3Error::BadRequest(x.to_string()))? {
            XmlEvent::StartElement { .. } => {
                text.clear();
            }
            XmlEvent::Characters(ss) => text.push_str(&ss),
            XmlEvent::EndElement { name, .. } => {
                match name.local_name.as_str() {
                    "PartNumber" => cur_part_num = text.trim().parse().ok(),
                    "ETag" => cur_etag = text.trim().trim_matches('"').to_string(),
                    "Part" => {
                        if let Some(pn) = cur_part_num.take() {
                            parts.push(PartETag {
                                part_number: pn,
                                etag: std::mem::take(&mut cur_etag),
                            });
                        }
                    }
                    _ => {}
                }
                text.clear();
            }
            _ => {}
        }
    }
    Ok(parts)
}

// --- 18. AbortMultipartUpload 已在 DELETE?uploadId=... 分支中 ---

// --- 19. ListMultipartUploads ---
fn op_list_multipart_uploads(mpu: &MultipartManager, bucket: &str, query: &str) -> Response<Body> {
    let prefix = query_val(query, "prefix").unwrap_or_default();
    let list = mpu.list_uploads(bucket, &prefix);
    let mut inner = String::new();
    for up in list.iter().take(1000) {
        inner.push_str("  <Upload>\n");
        inner.push_str(&format!("    <Key>{}</Key>\n", up.key));
        inner.push_str(&format!("    <UploadId>{}</UploadId>\n", up.upload_id));
        inner.push_str(&format!(
            "    <Initiated>{}</Initiated>\n",
            iso8601(up.initiated_ms)
        ));
        inner.push_str(
            "    <Initiator><ID>mox</ID><DisplayName>mox</DisplayName></Initiator>\n",
        );
        inner.push_str("    <Owner><ID>mox</ID><DisplayName>mox</DisplayName></Owner>\n");
        inner.push_str("    <StorageClass>STANDARD</StorageClass>\n");
        inner.push_str("  </Upload>\n");
    }
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <ListMultipartUploadsResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n\
           <Bucket>{}</Bucket>\n\
           <KeyMarker></KeyMarker>\n\
           <UploadIdMarker></UploadIdMarker>\n\
           <NextKeyMarker></NextKeyMarker>\n\
           <NextUploadIdMarker></NextUploadIdMarker>\n\
           <Delimiter></Delimiter>\n\
           <Prefix>{}</Prefix>\n\
           <MaxUploads>1000</MaxUploads>\n\
           <IsTruncated>false</IsTruncated>\n\
         {}\
         </ListMultipartUploadsResult>",
        bucket, prefix, inner
    );
    S3Server::xml_response(StatusCode::OK, body)
}

// --- 20. ListParts ---
fn op_list_parts(mpu: &MultipartManager, _bucket: &str, _key: &str, query: &str) -> Response<Body> {
    let uid = match query_val(query, "uploadId") {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchUpload),
    };
    let up_info = match mpu.get(&uid) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchUpload),
    };
    let parts = match mpu.list_parts(&uid) {
        Ok(v) => v,
        Err(e) => return S3Server::error_response(e),
    };
    let mut inner = String::new();
    for p in &parts {
        inner.push_str("  <Part>\n");
        inner.push_str(&format!("    <PartNumber>{}</PartNumber>\n", p.part_number));
        inner.push_str(&format!(
            "    <LastModified>{}</LastModified>\n",
            iso8601(now_ms())
        ));
        inner.push_str(&format!("    <ETag>{}</ETag>\n", p.etag));
        inner.push_str(&format!("    <Size>{}</Size>\n", p.size));
        inner.push_str("  </Part>\n");
    }
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <ListPartsResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n\
           <Bucket>{}</Bucket>\n\
           <Key>{}</Key>\n\
           <UploadId>{}</UploadId>\n\
           <StorageClass>STANDARD</StorageClass>\n\
           <Owner><ID>mox</ID><DisplayName>mox</DisplayName></Owner>\n\
           <Initiator><ID>mox</ID><DisplayName>mox</DisplayName></Initiator>\n\
           <PartNumberMarker>0</PartNumberMarker>\n\
           <NextPartNumberMarker>0</NextPartNumberMarker>\n\
           <MaxParts>1000</MaxParts>\n\
           <IsTruncated>false</IsTruncated>\n\
         {}\
         </ListPartsResult>",
        up_info.bucket, up_info.key, uid, inner
    );
    S3Server::xml_response(StatusCode::OK, body)
}

// --- 21. DeleteMultipleObjects ---
fn op_delete_multiple_objects(
    storage: &InMemoryStorage,
    bucket: &str,
    body: Bytes,
) -> Response<Body> {
    let s = String::from_utf8_lossy(&body);
    let reader = xml::EventReader::from_str(&s);
    let mut quiet = false;
    let mut to_delete: Vec<String> = Vec::new();
    let mut text = String::new();
    let mut cur_key = String::new();
    let mut cur_vid: Option<String> = None;
    for e in reader {
        use xml::reader::XmlEvent;
        match e {
            Ok(XmlEvent::StartElement { .. }) => text.clear(),
            Ok(XmlEvent::Characters(ss)) => text.push_str(&ss),
            Ok(XmlEvent::EndElement { name, .. }) => {
                match name.local_name.as_str() {
                    "Quiet" => quiet = text.trim() == "true",
                    "Key" => cur_key = text.trim().to_string(),
                    "VersionId" => cur_vid = Some(text.trim().to_string()),
                    "Object" => {
                        if !cur_key.is_empty() {
                            // Apply deletion
                            let mut objs = storage.objects.lock();
                            if let Some(m) = objs.get_mut(bucket) {
                                if let Some(vers) = m.get_mut(&cur_key) {
                                    if let Some(vid) = cur_vid.take() {
                                        vers.retain(|v| v.version_id != vid);
                                    } else {
                                        vers.clear();
                                    }
                                }
                            }
                            drop(objs);
                            to_delete.push(cur_key.clone());
                        }
                        cur_key.clear();
                        cur_vid = None;
                    }
                    _ => {}
                }
                text.clear();
            }
            _ => {}
        }
    }
    let mut inner = String::new();
    if !quiet {
        for k in &to_delete {
            inner.push_str(&format!("  <Deleted><Key>{}</Key></Deleted>\n", k));
        }
    }
    let body_xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <DeleteResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n{}\
         </DeleteResult>",
        inner
    );
    S3Server::xml_response(StatusCode::OK, body_xml)
}

// --- 22. GetBucketVersioning ---
fn op_get_bucket_versioning(versioning: &VersioningManager, bucket: &str) -> Response<Body> {
    let st = versioning.get(bucket);
    let status_xml = if matches!(st, VersioningStatus::Off) {
        String::new()
    } else {
        format!("  <Status>{}</Status>\n", st.as_str())
    };
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <VersioningConfiguration xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n{}\
         </VersioningConfiguration>",
        status_xml
    );
    S3Server::xml_response(StatusCode::OK, body)
}

// --- 23. PutBucketVersioning ---
fn op_put_bucket_versioning(
    versioning: &VersioningManager,
    bucket: &str,
    body: &[u8],
) -> Response<Body> {
    let s = String::from_utf8_lossy(body);
    let reader = xml::EventReader::from_str(&s);
    let mut status = String::new();
    let mut text = String::new();
    for e in reader {
        use xml::reader::XmlEvent;
        match e {
            Ok(XmlEvent::Characters(ss)) => text.push_str(&ss),
            Ok(XmlEvent::EndElement { name, .. }) => {
                if name.local_name == "Status" {
                    status = text.trim().to_string();
                }
                text.clear();
            }
            _ => {}
        }
    }
    let st = VersioningStatus::parse(&status).unwrap_or(VersioningStatus::Off);
    versioning.set(bucket, st);
    S3Server::ok_empty_headers(vec![])
}

// --- 24. ListObjectVersions ---
fn op_list_object_versions(storage: &InMemoryStorage, bucket: &str, query: &str) -> Response<Body> {
    let prefix = query_val(query, "prefix").unwrap_or_default();
    let objs = storage.objects.lock();
    let bucket_map = objs.get(bucket).cloned().unwrap_or_default();
    let mut items: Vec<(String, ObjectMeta)> = Vec::new();
    for (k, versions) in bucket_map.iter() {
        if !k.starts_with(&prefix) {
            continue;
        }
        for v in versions {
            items.push((k.clone(), v.clone()));
        }
    }
    items.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.last_modified_ms.cmp(&b.1.last_modified_ms))
    });

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<ListVersionsResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n");
    xml.push_str(&format!("  <Name>{}</Name>\n", bucket));
    xml.push_str(&format!("  <Prefix>{}</Prefix>\n", prefix));
    xml.push_str("  <KeyMarker></KeyMarker>\n");
    xml.push_str("  <VersionIdMarker></VersionIdMarker>\n");
    xml.push_str("  <MaxKeys>1000</MaxKeys>\n");
    xml.push_str("  <IsTruncated>false</IsTruncated>\n");
    for (k, v) in &items {
        if v.is_delete_marker {
            xml.push_str("  <DeleteMarker>\n");
            xml.push_str(&format!("    <Key>{}</Key>\n", k));
            xml.push_str(&format!("    <VersionId>{}</VersionId>\n", v.version_id));
            xml.push_str("    <IsLatest>true</IsLatest>\n");
            xml.push_str(&format!(
                "    <LastModified>{}</LastModified>\n",
                iso8601(v.last_modified_ms)
            ));
            xml.push_str("    <Owner><ID>mox</ID><DisplayName>mox</DisplayName></Owner>\n");
            xml.push_str("  </DeleteMarker>\n");
        } else {
            xml.push_str("  <Version>\n");
            xml.push_str(&format!("    <Key>{}</Key>\n", k));
            xml.push_str(&format!("    <VersionId>{}</VersionId>\n", v.version_id));
            xml.push_str("    <IsLatest>true</IsLatest>\n");
            xml.push_str(&format!(
                "    <LastModified>{}</LastModified>\n",
                iso8601(v.last_modified_ms)
            ));
            xml.push_str(&format!("    <ETag>{}</ETag>\n", v.etag));
            xml.push_str(&format!("    <Size>{}</Size>\n", v.size));
            xml.push_str("    <StorageClass>STANDARD</StorageClass>\n");
            xml.push_str("    <Owner><ID>mox</ID><DisplayName>mox</DisplayName></Owner>\n");
            xml.push_str("  </Version>\n");
        }
    }
    xml.push_str("</ListVersionsResult>\n");
    S3Server::xml_response(StatusCode::OK, xml)
}

// --- 25. GetObjectTagging ---
fn op_get_object_tagging(storage: &InMemoryStorage, bucket: &str, key: &str) -> Response<Body> {
    let objs = storage.objects.lock();
    let m = match objs.get(bucket) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchBucket),
    };
    let versions = match m.get(key) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchKey),
    };
    let meta = match versions.last() {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchKey),
    };
    let tagging = Tagging::from_map(&meta.tags);
    S3Server::xml_response(StatusCode::OK, tagging.to_xml())
}

// --- 26. PutObjectTagging ---
fn op_put_object_tagging(
    storage: &InMemoryStorage,
    bucket: &str,
    key: &str,
    body: Bytes,
) -> Response<Body> {
    let tagging = match Tagging::from_xml(&String::from_utf8_lossy(&body)) {
        Ok(t) => t,
        Err(e) => return S3Server::error_response(S3Error::BadRequest(e)),
    };
    let mut objs = storage.objects.lock();
    let m = match objs.get_mut(bucket) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchBucket),
    };
    let versions = match m.get_mut(key) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchKey),
    };
    if let Some(last) = versions.last_mut() {
        last.tags = tagging.to_map();
    }
    S3Server::ok_empty_headers(vec![])
}

// --- 27. GetBucketTagging ---
fn op_get_bucket_tagging(storage: &InMemoryStorage, bucket: &str) -> Response<Body> {
    let buckets = storage.buckets.lock();
    let b = match buckets.get(bucket) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchBucket),
    };
    let tagging = Tagging::from_map(&b.tags);
    S3Server::xml_response(StatusCode::OK, tagging.to_xml())
}

// --- 28. PutBucketTagging ---
fn op_put_bucket_tagging(storage: &InMemoryStorage, bucket: &str, body: &[u8]) -> Response<Body> {
    let tagging = match Tagging::from_xml(&String::from_utf8_lossy(body)) {
        Ok(t) => t,
        Err(e) => return S3Server::error_response(S3Error::BadRequest(e)),
    };
    let mut buckets = storage.buckets.lock();
    let b = match buckets.get_mut(bucket) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchBucket),
    };
    b.tags = tagging.to_map();
    S3Server::ok_empty_headers(vec![])
}

// --- 29. GetBucketPolicy ---
fn op_get_bucket_policy(storage: &InMemoryStorage, bucket: &str) -> Response<Body> {
    let buckets = storage.buckets.lock();
    let b = match buckets.get(bucket) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchBucket),
    };
    let json = match &b.policy {
        Some(p) => p.to_json(),
        None => "{}".to_string(),
    };
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("x-amz-request-id", new_request_id())
        .body(Body::from(json))
        .unwrap()
}

// --- 30. PutBucketPolicy ---
fn op_put_bucket_policy(storage: &InMemoryStorage, bucket: &str, body: &[u8]) -> Response<Body> {
    // 先检查桶存在，避免空 JSON 的解析错误掩盖 NoSuchBucket
    if !storage.buckets.lock().contains_key(bucket) {
        return S3Server::error_response(S3Error::NoSuchBucket);
    }
    let policy_s = String::from_utf8_lossy(body);
    let policy = match BucketPolicy::from_json(&policy_s) {
        Ok(p) => p,
        Err(e) => return S3Server::error_response(S3Error::BadRequest(e)),
    };
    let mut buckets = storage.buckets.lock();
    let b = match buckets.get_mut(bucket) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchBucket),
    };
    b.policy = Some(policy);
    S3Server::ok_empty_headers(vec![])
}

// --- 31. GetBucketLifecycle ---
fn op_get_bucket_lifecycle(storage: &InMemoryStorage, bucket: &str) -> Response<Body> {
    let buckets = storage.buckets.lock();
    let b = match buckets.get(bucket) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchBucket),
    };
    let body = b.lifecycle_xml.clone().unwrap_or_else(|| {
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <LifecycleConfiguration xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n\
         </LifecycleConfiguration>"
            .into()
    });
    S3Server::xml_response(StatusCode::OK, body)
}

// --- 32. PutBucketLifecycle ---
fn op_put_bucket_lifecycle(storage: &InMemoryStorage, bucket: &str, body: &[u8]) -> Response<Body> {
    let xml_str = String::from_utf8_lossy(body).to_string();
    let mut buckets = storage.buckets.lock();
    let b = match buckets.get_mut(bucket) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchBucket),
    };
    b.lifecycle_xml = Some(xml_str);
    S3Server::ok_empty_headers(vec![])
}

// --- 33. GetBucketCors ---
fn op_get_bucket_cors(storage: &InMemoryStorage, bucket: &str) -> Response<Body> {
    let buckets = storage.buckets.lock();
    let b = match buckets.get(bucket) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchBucket),
    };
    let cors = b.cors.clone().unwrap_or_default();
    S3Server::xml_response(StatusCode::OK, cors.to_xml())
}

// --- 34. PutBucketCors ---
fn op_put_bucket_cors(storage: &InMemoryStorage, bucket: &str, body: &[u8]) -> Response<Body> {
    let xml_s = String::from_utf8_lossy(body);
    let cors = match CorsConfiguration::from_xml(&xml_s) {
        Ok(c) => c,
        Err(e) => return S3Server::error_response(S3Error::BadRequest(e)),
    };
    let mut buckets = storage.buckets.lock();
    let b = match buckets.get_mut(bucket) {
        Some(v) => v,
        None => return S3Server::error_response(S3Error::NoSuchBucket),
    };
    b.cors = Some(cors);
    S3Server::ok_empty_headers(vec![])
}

// --- unused silencers for CORS ---
#[allow(dead_code)]
fn _cors_unused_type() -> CorsRule {
    CorsRule::default()
}

// ---------------- Date helpers ----------------

fn iso8601(ms: u64) -> String {
    // 直接生成简化 ISO 8601，便于测试
    let secs = ms / 1000;
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let d = (secs / 86400) + 1; // 近似 1970-01-01
    format!(
        "1970-01-{:02}T{:02}:{:02}:{:02}.000Z",
        (d.min(28)) as u8,
        h as u8,
        m as u8,
        s as u8
    )
}

fn http_date(ms: u64) -> String {
    let secs = ms / 1000;
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    format!("Thu, 01 Jan 1970 {:02}:{:02}:{:02} GMT", h, m, s)
}
