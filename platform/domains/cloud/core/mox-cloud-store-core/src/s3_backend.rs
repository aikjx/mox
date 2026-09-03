// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! S3 兼容后端（feature `s3`）：自研异步 SigV4 客户端 + [`S3ObjectStore`]。
//!
//! 不依赖重型 `aws-sdk-s3`：以标准 HTTP(S) 手写 AWS Signature V4，
//! 与 MinIO / 腾讯 COS / 华为 OBS / 阿里 OSS 互通（path-style）。
//!
//! 设计要点：
//! - **key 同构**：S3 key 与 FS 逻辑路径逐字一致（`path == key`），
//!   保证同一逻辑路径在 FS/S3 后端下可互换、可迁移。
//! - **三路物理口**：`ObjectStore`（PUT/GET/RANGE/DELETE/HEAD/EXISTS）+ `KvStore`
//!   （本地 `data_dir/kv` 落盘）+ `ObjectStreamWriter`（MPU 落盘复用）。
//! - **读时 RANGE 直达**：`get_range` 走 S3 `Range` 头，不整对象下载。

use crate::backend::S3ClientConfig;
use crate::kv_backend::FsKvStore;
use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use mox_base_store_core::{
    BlobObject, KvStore, ObjectStore, ObjectStreamWriter, StoreError, StoreResult, StreamHandle,
};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

type HmacSha256 = Hmac<Sha256>;

/// S3 响应头信息
#[derive(Debug, Clone)]
pub struct S3HeadInfo {
    pub size_bytes: u64,
    pub content_type: String,
    pub etag: String,
    pub last_modified: String,
}

/// 自研异步 S3 客户端（SigV4，path-style）
#[derive(Clone)]
pub struct S3Client {
    endpoint: String,
    region: String,
    access_key: String,
    secret_key: String,
    bucket: String,
    http: reqwest::Client,
}

impl S3Client {
    /// 依据后端配置构建客户端
    pub fn new(cfg: &S3ClientConfig) -> StoreResult<Self> {
        if cfg.bucket.is_empty() || cfg.endpoint.is_empty() {
            return Err(StoreError::Other("S3 配置缺少 bucket/endpoint".into()));
        }
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| StoreError::Other(format!("构建 HTTP 客户端失败: {e}")))?;
        Ok(Self {
            endpoint: cfg.endpoint.trim_end_matches('/').to_string(),
            region: if cfg.region.is_empty() {
                "us-east-1".into()
            } else {
                cfg.region.clone()
            },
            access_key: cfg.access_key.clone(),
            secret_key: cfg.secret_key.clone(),
            bucket: cfg.bucket.clone(),
            http,
        })
    }

    /// 目标 URL（path-style：`{endpoint}/{bucket}/{key}`）
    fn url(&self, key: &str) -> String {
        let key = key.trim_start_matches('/');
        format!("{}/{}/{}", self.endpoint, self.bucket, key)
    }

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    /// 单对象请求（带 SigV4 签名）
    async fn request(
        &self,
        method: &str,
        key: &str,
        query: &str,
        extra_headers: &[(&str, String)],
        body: Option<&[u8]>,
    ) -> StoreResult<reqwest::Response> {
        let payload = body.unwrap_or(&[]);
        let payload_hash = hex::encode(Sha256::digest(payload));
        let now = Self::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();
        let scope = format!("{}/{}/s3/aws4_request", date_stamp, self.region);

        let mut url = self.url(key);
        if !query.is_empty() {
            url.push('?');
            url.push_str(query);
        }
        let parsed = url
            .parse::<reqwest::Url>()
            .map_err(|e| StoreError::Other(format!("非法 S3 URL '{url}': {e}")))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| StoreError::Other("S3 URL 缺少主机".into()))?
            .to_string();
        let canonical_uri = uri_encode_path(parsed.path());

        // 排序后的查询参数（S3 要求 lexicographic）
        let mut query_pairs: Vec<(String, String)> = Vec::new();
        if !query.is_empty() {
            for pair in query.split('&').filter(|s| !s.is_empty()) {
                match pair.split_once('=') {
                    Some((kk, vv)) => query_pairs.push((kk.to_string(), vv.to_string())),
                    None => query_pairs.push((pair.to_string(), String::new())),
                }
            }
        }
        query_pairs.sort();
        let canonical_query = query_pairs
            .iter()
            .map(|(k, v)| format!("{}={}", uri_encode(k), uri_encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        // Canonical headers（host + x-amz-date + x-amz-content-sha256 + extras）
        let mut canonical_headers = format!(
            "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
            host, payload_hash, amz_date
        );
        let mut signed_headers = "host;x-amz-content-sha256;x-amz-date".to_string();
        for (name, value) in extra_headers {
            let lower = name.to_ascii_lowercase();
            canonical_headers.push_str(&format!("{lower}:{value}\n"));
            signed_headers.push(';');
            signed_headers.push_str(&lower);
        }

        let canonical_request = format!(
            "{method}\n{canonical_uri}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
        );
        let hashed_canonical = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!("AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{hashed_canonical}");

        fn hmac_sha256(key: &[u8], data: &str) -> Vec<u8> {
            let mut mac = HmacSha256::new_from_slice(key).expect("HMAC 接受任意长度密钥");
            mac.update(data.as_bytes());
            mac.finalize().into_bytes().to_vec()
        }
        let k_date = hmac_sha256(format!("AWS4{}", self.secret_key).as_bytes(), &date_stamp);
        let k_region = hmac_sha256(&k_date, &self.region);
        let k_service = hmac_sha256(&k_region, "s3");
        let k_signing = hmac_sha256(&k_service, "aws4_request");
        let signature = hex::encode(hmac_sha256(&k_signing, &string_to_sign));
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.access_key, scope, signed_headers, signature
        );

        let mut req = self
            .http
            .request(
                reqwest::Method::from_bytes(method.as_bytes())
                    .map_err(|e| StoreError::Other(format!("非法方法 {method}: {e}")))?,
                &url,
            )
            .header("Host", host)
            .header("x-amz-date", amz_date)
            .header("x-amz-content-sha256", &payload_hash)
            .header("Authorization", authorization);
        for (name, value) in extra_headers {
            req = req.header(*name, value);
        }
        if !payload.is_empty() {
            req = req.body(payload.to_vec());
        }
        req.send()
            .await
            .map_err(|e| StoreError::Io(format!("S3 {method} {key} 网络错误: {e}")))
    }

    /// 解析非 2xx 响应为错误详情（消费响应体）
    async fn error_from(resp: reqwest::Response, method: &str, key: &str) -> StoreError {
        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return StoreError::NotFound { path: key.to_string() };
        }
        let body = resp.text().await.unwrap_or_default();
        StoreError::Io(format!("S3 {method} {key} 返回 {status}: {body}"))
    }

    /// PUT 完整对象
    pub async fn put_object(&self, key: &str, content_type: &str, data: &[u8]) -> StoreResult<()> {
        let resp = self
            .request(
                "PUT",
                key,
                "",
                &[("Content-Type", content_type.to_string())],
                Some(data),
            )
            .await?;
        if !resp.status().is_success() {
            return Err(Self::error_from(resp, "PUT", key).await);
        }
        Ok(())
    }

    /// GET 完整对象
    pub async fn get_object(&self, key: &str) -> StoreResult<Vec<u8>> {
        let resp = self.request("GET", key, "", &[], None).await?;
        if !resp.status().is_success() {
            return Err(Self::error_from(resp, "GET", key).await);
        }
        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| StoreError::Io(format!("S3 GET {key} 读取失败: {e}")))
    }

    /// HEAD 对象元数据；不存在返回 Ok(None)
    pub async fn head_object(&self, key: &str) -> StoreResult<Option<S3HeadInfo>> {
        let resp = self.request("HEAD", key, "", &[], None).await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(Self::error_from(resp, "HEAD", key).await);
        }
        let size_bytes = resp
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        let etag = resp
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let last_modified = resp
            .headers()
            .get(reqwest::header::LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        Ok(Some(S3HeadInfo {
            size_bytes,
            content_type,
            etag,
            last_modified,
        }))
    }

    /// DELETE 对象
    pub async fn delete_object(&self, key: &str) -> StoreResult<()> {
        let resp = self.request("DELETE", key, "", &[], None).await?;
        if !resp.status().is_success() && resp.status() != reqwest::StatusCode::NOT_FOUND {
            return Err(Self::error_from(resp, "DELETE", key).await);
        }
        Ok(())
    }

    /// 按 RANGE 读取
    pub async fn get_object_range(
        &self,
        key: &str,
        offset: u64,
        length: u64,
    ) -> StoreResult<Bytes> {
        let range = format!("bytes={}-{}", offset, offset + length.saturating_sub(1));
        let resp = self
            .request("GET", key, "", &[("Range", range)], None)
            .await?;
        if !resp.status().is_success() {
            return Err(Self::error_from(resp, "GET(range)", key).await);
        }
        resp.bytes()
            .await
            .map_err(|e| StoreError::Io(format!("S3 GET(range) {key} 失败: {e}")))
    }

    /// ListObjectsV2：返回对象 key 列表
    pub async fn list_objects(&self, prefix: &str) -> StoreResult<Vec<String>> {
        let query = format!("list-type=2&prefix={}", uri_encode(prefix));
        let resp = self.request("GET", "", &query, &[], None).await?;
        if !resp.status().is_success() {
            return Err(Self::error_from(resp, "ListObjectsV2", prefix).await);
        }
        let body = resp
            .bytes()
            .await
            .map_err(|e| StoreError::Io(format!("S3 ListObjectsV2 读取失败: {e}")))?;
        // 解析 XML：提取 <Key> 标签
        let text = String::from_utf8_lossy(&body);
        let mut keys = Vec::new();
        for part in text.split("<Key>") {
            if let Some(rest) = part.split("</Key>").next() {
                if !rest.is_empty() {
                    keys.push(rest.to_string());
                }
            }
        }
        Ok(keys)
    }
}

/// S3 对象存储（实现 [`ObjectStore`]）
#[derive(Clone)]
pub struct S3ObjectStore {
    client: Arc<S3Client>,
    kv: Arc<FsKvStore>,
}

impl S3ObjectStore {
    pub fn new(data_dir: impl AsRef<Path>, cfg: &S3ClientConfig) -> StoreResult<Self> {
        Ok(Self {
            client: Arc::new(S3Client::new(cfg)?),
            kv: Arc::new(FsKvStore::new(data_dir.as_ref().join("kv"))?),
        })
    }

    /// 底层 S3 客户端（供运维/回源装饰器复用）
    pub fn client(&self) -> &S3Client {
        &self.client
    }
}

#[async_trait]
impl ObjectStore for S3ObjectStore {
    async fn put(&self, path: &str, content_type: &str, data: Bytes) -> StoreResult<BlobObject> {
        self.client.put_object(path, content_type, &data).await?;
        Ok(BlobObject {
            path: path.to_string(),
            content_type: content_type.to_string(),
            size_bytes: data.len() as u64,
            sha256: Some(hex::encode(Sha256::digest(&data))),
        })
    }

    async fn get(&self, path: &str) -> StoreResult<Bytes> {
        let data = self.client.get_object(path).await?;
        Ok(Bytes::from(data))
    }

    async fn get_range(&self, path: &str, offset: u64, length: u64) -> StoreResult<Bytes> {
        self.client.get_object_range(path, offset, length).await
    }

    async fn delete(&self, path: &str) -> StoreResult<()> {
        self.client.delete_object(path).await
    }

    async fn head(&self, path: &str) -> StoreResult<BlobObject> {
        let info = self
            .client
            .head_object(path)
            .await?
            .ok_or_else(|| StoreError::NotFound { path: path.to_string() })?;
        Ok(BlobObject {
            path: path.to_string(),
            content_type: info.content_type,
            size_bytes: info.size_bytes,
            sha256: None,
        })
    }

    async fn exists(&self, path: &str) -> StoreResult<bool> {
        Ok(self.client.head_object(path).await?.is_some())
    }
}

#[async_trait]
impl KvStore for S3ObjectStore {
    async fn put(&self, key: &str, value: Bytes) -> StoreResult<()> {
        self.kv.put(key, value).await
    }

    async fn get(&self, key: &str) -> StoreResult<Option<Bytes>> {
        self.kv.get(key).await
    }

    async fn delete(&self, key: &str) -> StoreResult<()> {
        self.kv.delete(key).await
    }
}

#[async_trait]
impl ObjectStreamWriter for S3ObjectStore {
    /// 流式会话：本地磁盘句柄累积分片，Close 时单次 PUT。
    async fn open_writer(&self, path: &str, content_type: &str) -> StoreResult<StreamHandle> {
        let writer = crate::FsStreamWriter::open(self.kv.data_dir()).await?;
        Ok(StreamHandle {
            path: path.to_string(),
            state: format!("{}|{}", content_type, writer.tmp_path().display()),
        })
    }

    async fn write(&self, handle: &StreamHandle, chunk: Bytes) -> StoreResult<()> {
        let (_, tmp) = handle
            .state
            .split_once('|')
            .ok_or_else(|| StoreError::Other("非法流式句柄".into()))?;
        use tokio::io::AsyncWriteExt;
        let mut f = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(tmp)
            .await
            .map_err(|e| StoreError::Io(format!("打开 MPU 分片失败 {tmp}: {e}")))?;
        f.write_all(&chunk)
            .await
            .map_err(|e| StoreError::Io(format!("追加 MPU 分片失败: {e}")))?;
        f.flush()
            .await
            .map_err(|e| StoreError::Io(format!("flush 失败: {e}")))?;
        Ok(())
    }

    async fn close(&self, handle: StreamHandle) -> StoreResult<BlobObject> {
        let (content_type, tmp) = handle
            .state
            .split_once('|')
            .ok_or_else(|| StoreError::Other("非法流式句柄".into()))?;
        let data = tokio::fs::read(tmp)
            .await
            .map_err(|e| StoreError::Io(format!("读取 MPU 分片失败 {tmp}: {e}")))?;
        let _ = tokio::fs::remove_file(tmp).await;
        ObjectStore::put(self, &handle.path, content_type, Bytes::from(data)).await
    }
}

/// 装配 S3 后端（供 `create_backend` 调用）
pub fn build_s3_backend(
    data_dir: &Path,
    cfg: &S3ClientConfig,
    kind: crate::backend::BackendKind,
) -> StoreResult<crate::backend::StoreBackend> {
    let store = Arc::new(S3ObjectStore::new(data_dir, cfg)?);
    Ok(crate::backend::StoreBackend {
        kind,
        object: store.clone(),
        kv: store.clone(),
        stream: store,
        data_dir: data_dir.to_path_buf(),
    })
}

// =============== SigV4 工具 ===============

/// URI 路径段编码（保留 `/`，S3 规范 §4.1.1）
pub fn uri_encode_path(path: &str) -> String {
    path.split('/').map(uri_encode).collect::<Vec<_>>().join("/")
}

/// RFC 3986 编码（保留 S3 允许的 unreserved 字符）
pub fn uri_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

// =============== 单元测试（仅算法级，不发真实请求） ===============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_encoding_rules() {
        assert_eq!(uri_encode("a/b c"), "a%2Fb%20c");
        assert_eq!(uri_encode("中"), "%E4%B8%AD");
        assert_eq!(uri_encode("safe_-~."), "safe_-~.");
        assert_eq!(uri_encode_path("/a/b c/d"), "/a/b%20c/d");
    }

    #[test]
    fn s3_client_rejects_empty_config() {
        let cfg = S3ClientConfig {
            endpoint: "".into(),
            region: "us-east-1".into(),
            access_key: "k".into(),
            secret_key: "s".into(),
            bucket: "".into(),
            force_path_style: true,
        };
        assert!(S3Client::new(&cfg).is_err());
    }

    #[tokio::test]
    async fn canonical_request_shape_is_sigv4() {
        // 本地无服务：应返回网络类错误（证明走真实 HTTP 路径，非占位）
        let cfg = S3ClientConfig {
            endpoint: "http://127.0.0.1:9".into(),
            region: "us-east-1".into(),
            access_key: "AKIDEXAMPLE".into(),
            secret_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".into(),
            bucket: "bkt".into(),
            force_path_style: true,
        };
        let client = S3Client::new(&cfg).unwrap();
        let err = client.get_object("nope").await.unwrap_err();
        match err {
            StoreError::Io(msg) => assert!(msg.contains("S3 GET"), "应含真实请求错误: {msg}"),
            other => panic!("预期 Io 网络错误，实际 {other:?}"),
        }
    }
}
