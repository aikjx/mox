// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! S3 兼容后端（feature `s3`）：自研异步 SigV4 客户端 + [`S3ObjectStore`]。
//!
//! 不依赖重型 `aws-sdk-s3`：以标准 HTTP(S) 手写 AWS Signature V4，
//! 与 MinIO / 腾讯 COS / 华为 OBS / 阿里 OSS 适配；OSS 仅支持 virtual-hosted 地址，云端互通须单独验收。
//!
//! 设计要点：
//! - **key 同构**：S3 key 与 FS 逻辑路径逐字一致（`path == key`），
//!   保证同一逻辑路径在 FS/S3 后端下可互换、可迁移。
//! - **三路物理口**：`ObjectStore`（PUT/GET/RANGE/DELETE/HEAD/EXISTS）+ `KvStore`
//!   （本地 `data_dir/kv` 落盘）+ `ObjectStreamWriter`（MPU 落盘复用）。
//! - **读时 RANGE 直达**：`get_range` 走 S3 `Range` 头，不整对象下载。

use crate::{backend::S3ClientConfig, kv_backend::FsKvStore};
use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use mox_base_store_core::{
    BlobObject, KvStore, ObjectStore, ObjectStreamWriter, StoreError, StoreResult, StreamHandle,
};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Arc, time::Duration};

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
    force_path_style: bool,
    http: reqwest::Client,
}

impl S3Client {
    /// 依据后端配置构建客户端
    pub fn new(cfg: &S3ClientConfig) -> StoreResult<Self> {
        if cfg.bucket.is_empty()
            || cfg.endpoint.is_empty()
            || cfg.access_key.is_empty()
            || cfg.secret_key.is_empty()
        {
            return Err(StoreError::Other("S3 配置缺少 bucket/endpoint".into()));
        }
        let endpoint = reqwest::Url::parse(&cfg.endpoint)
            .map_err(|_| StoreError::Other("非法 S3 endpoint".into()))?;
        if !matches!(endpoint.scheme(), "http" | "https")
            || endpoint.host_str().is_none()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(StoreError::Other(
                "S3 endpoint 必须为不含凭据/查询/片段的 HTTP(S) 地址".into(),
            ));
        }
        if !cfg
            .bucket
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.'))
        {
            return Err(StoreError::Other("非法 S3 bucket".into()));
        }
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| StoreError::Other(format!("构建 HTTP 客户端失败: {e}")))?;
        Ok(Self {
            endpoint: cfg.endpoint.trim_end_matches('/').to_string(),
            region: if cfg.region.is_empty() { "us-east-1".into() } else { cfg.region.clone() },
            access_key: cfg.access_key.clone(),
            secret_key: cfg.secret_key.clone(),
            bucket: cfg.bucket.clone(),
            force_path_style: cfg.force_path_style,
            http,
        })
    }

    /// 目标地址遵循显式寻址配置；对象名先编码，避免 ?/#/% 改变请求含义。
    fn url(&self, key: &str) -> StoreResult<String> {
        let key = key.trim_start_matches('/');
        if key.split('/').any(|part| matches!(part, "." | "..")) {
            return Err(StoreError::Other("S3 key 不允许点路径段".into()));
        }
        let mut endpoint = reqwest::Url::parse(&self.endpoint)
            .map_err(|_| StoreError::Other("非法 S3 endpoint".into()))?;
        let prefix = endpoint.path().trim_end_matches('/').to_string();
        let path = if self.force_path_style {
            format!("{prefix}/{}/{}", self.bucket, uri_encode_path(key))
        } else {
            let host = endpoint
                .host_str()
                .ok_or_else(|| StoreError::Other("S3 endpoint 缺少 host".into()))?;
            let host = format!("{}.{}", self.bucket, host);
            endpoint
                .set_host(Some(&host))
                .map_err(|_| StoreError::Other("virtual-hosted endpoint 无效".into()))?;
            format!("{prefix}/{}", uri_encode_path(key))
        };
        endpoint.set_path(&path);
        Ok(endpoint.to_string())
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

        let mut url = self.url(key)?;
        if !query.is_empty() {
            url.push('?');
            url.push_str(query);
        }
        let parsed = url
            .parse::<reqwest::Url>()
            .map_err(|e| StoreError::Other(format!("非法 S3 URL '{url}': {e}")))?;
        let host = signing_host(&parsed)?;
        // URL 中路径已按原始 key 编码一次，签名不能再次编码百分号。
        let canonical_uri = parsed.path().to_owned();

        let canonical_query = canonical_query(&parsed);

        let (canonical_headers, signed_headers) =
            canonical_headers(&host, &amz_date, &payload_hash, extra_headers);

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
            .request("PUT", key, "", &[("Content-Type", content_type.to_string())], Some(data))
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
        Ok(Some(S3HeadInfo { size_bytes, content_type, etag, last_modified }))
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
        if length == 0 {
            return Ok(Bytes::new());
        }
        let end = offset
            .checked_add(length - 1)
            .ok_or_else(|| StoreError::Other("S3 range overflow".into()))?;
        let range = format!("bytes={offset}-{end}");
        let resp = self.request("GET", key, "", &[("Range", range)], None).await?;
        if !resp.status().is_success() {
            return Err(Self::error_from(resp, "GET(range)", key).await);
        }
        resp.bytes()
            .await
            .map_err(|e| StoreError::Io(format!("S3 GET(range) {key} 失败: {e}")))
    }

    /// ListObjectsV2：返回对象 key 列表
    pub async fn list_objects(&self, prefix: &str) -> StoreResult<Vec<String>> {
        let mut keys = Vec::new();
        let mut token: Option<String> = None;
        let mut seen = std::collections::HashSet::new();
        loop {
            let mut query = format!("list-type=2&prefix={}", uri_encode(prefix));
            if let Some(token) = &token {
                query.push_str(&format!("&continuation-token={}", uri_encode(token)));
            }
            let resp = self.request("GET", "", &query, &[], None).await?;
            if !resp.status().is_success() {
                return Err(Self::error_from(resp, "ListObjectsV2", prefix).await);
            }
            let body = resp
                .text()
                .await
                .map_err(|error| StoreError::Io(format!("S3 list response failed: {error}")))?;
            let page = parse_list_page(&body)?;
            keys.extend(page.contents.into_iter().map(|object| object.key));
            if keys.len() > 100_000 {
                return Err(StoreError::Other("S3 listing exceeds bounded result limit".into()));
            }
            if !page.is_truncated {
                return Ok(keys);
            }
            let next = page.next_token.filter(|token| !token.is_empty()).ok_or_else(|| {
                StoreError::Other("S3 truncated listing missing continuation token".into())
            })?;
            if !seen.insert(next.clone()) {
                return Err(StoreError::Other("S3 repeated continuation token".into()));
            }
            token = Some(next);
        }
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
    async fn list_keys(&self, prefix: &str) -> StoreResult<Vec<String>> {
        self.client.list_objects(prefix).await
    }

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
        f.flush().await.map_err(|e| StoreError::Io(format!("flush 失败: {e}")))?;
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
    if kind == crate::backend::BackendKind::Oss && cfg.force_path_style {
        return Err(StoreError::Other(
            "OSS 必须使用 virtual-hosted 寻址，请设置 force_path_style=false".into(),
        ));
    }
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
            },
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[derive(serde::Deserialize)]
#[serde(rename = "ListBucketResult")]
struct ListPage {
    #[serde(rename = "Contents", default)]
    contents: Vec<ListObject>,
    #[serde(rename = "IsTruncated")]
    is_truncated: bool,
    #[serde(rename = "NextContinuationToken", default)]
    next_token: Option<String>,
}
#[derive(serde::Deserialize)]
struct ListObject {
    #[serde(rename = "Key")]
    key: String,
}
fn parse_list_page(body: &str) -> StoreResult<ListPage> {
    let mut reader = quick_xml::Reader::from_str(body);
    loop {
        match reader
            .read_event()
            .map_err(|_| StoreError::Other("invalid S3 listing XML".into()))?
        {
            quick_xml::events::Event::Start(root)
                if root.local_name().as_ref() == b"ListBucketResult" =>
            {
                break
            },
            quick_xml::events::Event::Decl(_) | quick_xml::events::Event::Comment(_) => {},
            quick_xml::events::Event::Text(text)
                if text.as_ref().iter().all(u8::is_ascii_whitespace) => {},
            _ => return Err(StoreError::Other("unexpected S3 listing root".into())),
        }
    }
    quick_xml::de::from_str(body)
        .map_err(|_| StoreError::Other("invalid S3 ListObjectsV2 XML response".into()))
}
fn canonical_query(url: &reqwest::Url) -> String {
    let mut pairs: Vec<_> = url
        .query_pairs()
        .map(|(key, value)| (uri_encode(&key), uri_encode(&value)))
        .collect();
    pairs.sort();
    pairs
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn signing_host(url: &reqwest::Url) -> StoreResult<String> {
    let host = url.host_str().ok_or_else(|| StoreError::Other("S3 URL 缺少主机".into()))?;
    Ok(match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    })
}
fn canonical_headers(
    host: &str,
    date: &str,
    hash: &str,
    extras: &[(&str, String)],
) -> (String, String) {
    let mut headers = std::collections::BTreeMap::from([
        ("host".to_owned(), host.to_owned()),
        ("x-amz-date".to_owned(), date.to_owned()),
        ("x-amz-content-sha256".to_owned(), hash.to_owned()),
    ]);
    for (name, value) in extras {
        headers.insert(
            name.to_ascii_lowercase(),
            value.split_whitespace().collect::<Vec<_>>().join(" "),
        );
    }
    let canonical = headers.iter().map(|(name, value)| format!("{name}:{value}\n")).collect();
    let signed = headers.keys().cloned().collect::<Vec<_>>().join(";");
    (canonical, signed)
}

// =============== 单元测试（仅算法级，不发真实请求） ===============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addressing_encoding_and_signature_headers_are_consistent() {
        let mut cfg = S3ClientConfig {
            endpoint: "http://storage.example:9000".into(),
            region: "region".into(),
            access_key: "test".into(),
            secret_key: "test".into(),
            bucket: "bucket".into(),
            force_path_style: true,
        };
        let client = S3Client::new(&cfg).unwrap();
        assert_eq!(
            client.url("文档/a b?x#y%.txt").unwrap(),
            "http://storage.example:9000/bucket/%E6%96%87%E6%A1%A3/a%20b%3Fx%23y%25.txt"
        );
        let parsed = reqwest::Url::parse(&client.url("文档/a b").unwrap()).unwrap();
        assert_eq!(signing_host(&parsed).unwrap(), "storage.example:9000");
        assert!(client.url("a/../b").is_err());
        cfg.force_path_style = false;
        cfg.endpoint = "https://oss-cn-hangzhou.aliyuncs.com".into();
        let client = S3Client::new(&cfg).unwrap();
        assert_eq!(client.url("a/b").unwrap(), "https://bucket.oss-cn-hangzhou.aliyuncs.com/a/b");
        let (headers, signed) = canonical_headers(
            "host",
            "date",
            "hash",
            &[("Range", " bytes=0-1  ".into()), ("Content-Type", "text/plain".into())],
        );
        assert_eq!(signed, "content-type;host;range;x-amz-content-sha256;x-amz-date");
        assert!(headers.starts_with("content-type:text/plain\nhost:host\nrange:bytes=0-1\n"));
    }

    #[test]
    fn list_xml_and_query_encoding_preserve_real_keys() {
        let page = parse_list_page("<ListBucketResult><IsTruncated>true</IsTruncated><Contents><Key>文档/a&amp;b</Key></Contents><NextContinuationToken>a+/=</NextContinuationToken></ListBucketResult>").unwrap();
        assert_eq!(page.contents.len(), 1);
        assert_eq!(page.contents[0].key, "文档/a&b");
        assert_eq!(page.next_token.as_deref(), Some("a+/="));
        assert!(page.is_truncated);
        assert!(parse_list_page("<ListBucketResult><Contents>").is_err());
        let url = reqwest::Url::parse(
            "https://host/?prefix=a%2Fb%20c&continuation-token=a%2B%2F%3D&list-type=2",
        )
        .unwrap();
        assert_eq!(
            canonical_query(&url),
            "continuation-token=a%2B%2F%3D&list-type=2&prefix=a%2Fb%20c"
        );
    }

    #[tokio::test]
    async fn list_objects_follows_tokens_and_keeps_encoded_query_and_host_port() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for page in ["<ListBucketResult><IsTruncated>true</IsTruncated><Contents><Key>kb/a&amp;b</Key></Contents><NextContinuationToken>a+/=</NextContinuationToken></ListBucketResult>",
                "<ListBucketResult><IsTruncated>false</IsTruncated><Contents><Key>kb/second</Key></Contents></ListBucketResult>"] {
                let (mut stream,_) = listener.accept().unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut bytes = Vec::new();
                loop { let mut byte=[0]; stream.read_exact(&mut byte).unwrap(); bytes.push(byte[0]); if bytes.ends_with(b"\r\n\r\n") { break; } assert!(bytes.len()<16*1024); }
                requests.push(String::from_utf8(bytes).unwrap());
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",page.len()).unwrap();
            }
            requests
        });
        let client = S3Client::new(&S3ClientConfig {
            endpoint: format!("http://{address}"),
            region: "region".into(),
            access_key: "test".into(),
            secret_key: "test".into(),
            bucket: "bucket".into(),
            force_path_style: true,
        })
        .unwrap();
        assert_eq!(client.list_objects("kb/a&").await.unwrap(), vec!["kb/a&b", "kb/second"]);
        let requests = server.join().unwrap();
        assert!(requests[0].contains("prefix=kb%2Fa%26"));
        assert!(requests[1].contains("continuation-token=a%2B%2F%3D"));
        assert!(requests[0].to_ascii_lowercase().contains(&format!("host: {address}")));
        assert!(parse_list_page("<Error><IsTruncated>false</IsTruncated></Error>").is_err());
    }

    #[tokio::test]
    async fn range_bounds_are_checked_before_network_access() {
        let client = S3Client::new(&S3ClientConfig {
            endpoint: "http://127.0.0.1:9".into(),
            region: "region".into(),
            access_key: "test".into(),
            secret_key: "test".into(),
            bucket: "bucket".into(),
            force_path_style: true,
        })
        .unwrap();
        assert!(client.get_object_range("key", 0, 0).await.unwrap().is_empty());
        let error = client.get_object_range("key", u64::MAX, 2).await.unwrap_err();
        assert!(matches!(error,StoreError::Other(message) if message.contains("overflow")));
    }

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
