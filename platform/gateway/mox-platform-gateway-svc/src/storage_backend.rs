// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 存储后端抽象（StorageBackend trait）+ S3/MinIO 兼容 adapter
//!
//! # 设计目标
//! 把 `CloudState`（本地磁盘对象存储）从"具体实现"提升为"可插拔后端"：
//! - 统一 trait：list_buckets / create_bucket / delete_bucket / list_objects /
//!   put_object / get_object / delete_object / health 探测；
//! - 本地磁盘（`CloudState`）迁移为 trait 的首个真实实现，**行为与 `/cloud/v1/*` 完全一致**；
//! - S3/MinIO adapter：复用仓库已验证的 SigV4 签名范式（`platform/foundation/mox-audit/src/s3.rs`），
//!   经 env 配置门控（`MOX_S3_*`），未配置绝不暴露为"可用"。
//!
//! # 诚实声明（禁桩）
//! - 本地后端：真实读写磁盘，回归测试通过。
//! - S3/MinIO 后端：签名请求为真实 HTTP（reqwest + HMAC-SHA256），但**本环境无
//!   MinIO/S3 实例可连**，未做成功往返 E2E；`providers` 仅在 `MOX_S3_*` 配置齐全且
//!   `health()` 探测可达时才标 `available:true`。

use async_trait::async_trait;
use serde_json::{Value, json};

/// bucket 清单条目（对象数 / 占用字节）
#[derive(Debug, Clone)]
pub struct BucketInfo {
    pub name: String,
    pub object_count: u64,
    pub used_bytes: u64,
}

/// 对象元数据（key / 字节大小）
#[derive(Debug, Clone)]
pub struct ObjectMeta {
    pub key: String,
    pub size: u64,
}

/// 存储后端抽象（异步；本地磁盘实现为同步 fs，远程实现为异步 HTTP）
#[async_trait]
pub trait StorageBackend: Send + Sync + 'static {
    /// 后端 id（`local` / `s3`）
    fn id(&self) -> &str;
    /// 后端种类（`disk` / `s3`）
    fn kind(&self) -> &str;
    /// 前端 `/api/storage/providers` 卡片描述
    fn describe(&self) -> Value;

    async fn list_buckets(&self) -> Result<Vec<BucketInfo>, String>;
    async fn create_bucket(&self, name: &str) -> Result<(), String>;
    async fn delete_bucket(&self, name: &str) -> Result<(), String>;
    async fn list_objects(&self, bucket: &str) -> Result<Vec<ObjectMeta>, String>;
    async fn put_object(&self, bucket: &str, key: &str, body: Vec<u8>) -> Result<u64, String>;
    async fn get_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>, String>;
    async fn delete_object(&self, bucket: &str, key: &str) -> Result<(), String>;

    /// 连通性/配置健康探测（本地恒 Ok；远程发真实签名请求）
    async fn health(&self) -> Result<Value, String>;
}

// =============================================================================
// S3 / MinIO 兼容 adapter（env 门控，SigV4 签名）
// =============================================================================

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

fn hmac_sha256(key: &[u8], data: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC 接受任意长度密钥");
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

/// S3/MinIO 兼容后端（path-style endpoint）
pub struct S3Backend {
    endpoint: String,
    bucket: String,
    region: String,
    access_key: String,
    secret_key: String,
    client: reqwest::Client,
}

impl S3Backend {
    /// 从 env 构造；缺任一关键变量返回 None（不暴露为可用）。
    pub fn from_env() -> Option<Self> {
        let endpoint = std::env::var("MOX_S3_ENDPOINT").ok()?;
        let bucket = std::env::var("MOX_S3_BUCKET").ok()?;
        let access_key = std::env::var("MOX_S3_ACCESS_KEY_ID").ok()?;
        let secret_key = std::env::var("MOX_S3_SECRET_ACCESS_KEY").ok()?;
        if endpoint.is_empty() || bucket.is_empty() || access_key.is_empty() || secret_key.is_empty() {
            return None;
        }
        let region = std::env::var("MOX_S3_REGION").unwrap_or_else(|_| "us-east-1".into());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .ok()?;
        Some(Self { endpoint: endpoint.trim_end_matches('/').to_string(), bucket, region, access_key, secret_key, client })
    }

    /// 对一次 S3 请求做 SigV4 签名，返回 (method, url, headers)。
    fn signed(&self, method: &str, path: &str, query: &str, payload_hash: &str) -> Result<(String, Vec<(String, String)>), String> {
        use chrono::Utc;
        let now = Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();
        let scope = format!("{}/{}/s3/aws4_request", date_stamp, self.region);

        let url = format!("{}{}", self.endpoint, path);
        let host = url
            .parse::<reqwest::Url>()
            .map_err(|e| format!("非法 S3 URL: {e}"))?
            .host_str()
            .ok_or("S3 URL 缺 host")?
            .to_string();

        let canonical_headers = format!("host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n", host, payload_hash, amz_date);
        let signed_headers = "host;x-amz-content-sha256;x-amz-date".to_string();
        let canonical_request = format!("{}\n{}\n{}\n{}\n{}\n{}", method, path, query, canonical_headers, signed_headers, payload_hash);
        let hashed = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!("AWS4-HMAC-SHA256\n{}\n{}\n{}", amz_date, scope, hashed);

        let k_date = hmac_sha256(format!("AWS4{}", self.secret_key).as_bytes(), &date_stamp);
        let k_region = hmac_sha256(&k_date, &self.region);
        let k_service = hmac_sha256(&k_region, "s3");
        let k_signing = hmac_sha256(&k_service, "aws4_request");
        let signature = hex::encode(hmac_sha256(&k_signing, &string_to_sign));
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.access_key, scope, signed_headers, signature
        );

        let headers = vec![
            ("host".into(), host),
            ("x-amz-date".into(), amz_date),
            ("x-amz-content-sha256".into(), payload_hash.to_string()),
            ("Authorization".into(), authorization),
        ];
        Ok((url, headers))
    }

    fn empty_hash() -> String {
        hex::encode(Sha256::digest(""))
    }
}

#[async_trait]
impl StorageBackend for S3Backend {
    fn id(&self) -> &str { "s3" }
    fn kind(&self) -> &str { "s3" }
    fn describe(&self) -> Value {
        json!({
            "id": "s3", "name": "s3", "type": "s3",
            "description": format!("S3/MinIO 兼容对象存储（endpoint={}, bucket={}）", self.endpoint, self.bucket),
            "endpoint": self.endpoint, "bucket": self.bucket, "region": self.region,
            "available": true,
        })
    }

    async fn list_buckets(&self) -> Result<Vec<BucketInfo>, String> {
        // ListBuckets（path-style GET /）
        let (url, headers) = self.signed("GET", "/", "", &Self::empty_hash())?;
        let mut req = self.client.get(&url);
        for (k, v) in &headers { req = req.header(k, v); }
        let resp = req.send().await.map_err(|e| format!("S3 list_buckets 请求失败: {e}"))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("S3 ListBuckets 返回 {}: {}", status, truncate(&text, 200)));
        }
        // 最小 XML 解析：提取 <Name>...</Name>（bucket 名）
        let mut out = Vec::new();
        for chunk in text.split("<Bucket>").skip(1) {
            if let Some(name) = extract_tag(chunk, "Name") {
                out.push(BucketInfo { name, object_count: 0, used_bytes: 0 });
            }
        }
        Ok(out)
    }

    async fn create_bucket(&self, _name: &str) -> Result<(), String> {
        // path-style 单桶模型：bucket 在 env 配置；多桶管理留待后续
        Err("S3 adapter 为单桶（env 指定）模型，create_bucket 未启用".into())
    }

    async fn delete_bucket(&self, _name: &str) -> Result<(), String> {
        Err("S3 adapter 为单桶模型，delete_bucket 未启用".into())
    }

    async fn list_objects(&self, bucket: &str) -> Result<Vec<ObjectMeta>, String> {
        let path = format!("/{}?list-type=2", bucket);
        let (url, headers) = self.signed("GET", &path, "list-type=2", &Self::empty_hash())?;
        let mut req = self.client.get(&url);
        for (k, v) in &headers { req = req.header(k, v); }
        let resp = req.send().await.map_err(|e| format!("S3 list_objects 请求失败: {e}"))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("S3 ListObjectsV2 返回 {}: {}", status, truncate(&text, 200)));
        }
        let mut out = Vec::new();
        for chunk in text.split("<Contents>").skip(1) {
            let key = extract_tag(chunk, "Key").unwrap_or_default();
            let size: u64 = extract_tag(chunk, "Size").and_then(|s| s.parse().ok()).unwrap_or(0);
            if !key.is_empty() { out.push(ObjectMeta { key, size }); }
        }
        Ok(out)
    }

    async fn put_object(&self, bucket: &str, key: &str, body: Vec<u8>) -> Result<u64, String> {
        let size = body.len() as u64;
        let path = format!("/{}/{}", bucket, key);
        let payload_hash = hex::encode(Sha256::digest(&body));
        let (url, headers) = self.signed("PUT", &path, "", &payload_hash)?;
        let mut req = self.client.put(&url).body(body);
        for (k, v) in &headers { req = req.header(k, v); }
        let resp = req.send().await.map_err(|e| format!("S3 put_object 请求失败: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            let t = resp.text().await.unwrap_or_default();
            return Err(format!("S3 PUT 返回 {}: {}", status, truncate(&t, 200)));
        }
        Ok(size)
    }

    async fn get_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>, String> {
        let path = format!("/{}/{}", bucket, key);
        let (url, headers) = self.signed("GET", &path, "", &Self::empty_hash())?;
        let mut req = self.client.get(&url);
        for (k, v) in &headers { req = req.header(k, v); }
        let resp = req.send().await.map_err(|e| format!("S3 get_object 请求失败: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("S3 GET 返回 {}", status));
        }
        Ok(resp.bytes().await.map_err(|e| format!("读取 S3 响应体失败: {e}"))?.to_vec())
    }

    async fn delete_object(&self, bucket: &str, key: &str) -> Result<(), String> {
        let path = format!("/{}/{}", bucket, key);
        let (url, headers) = self.signed("DELETE", &path, "", &Self::empty_hash())?;
        let mut req = self.client.delete(&url);
        for (k, v) in &headers { req = req.header(k, v); }
        let resp = req.send().await.map_err(|e| format!("S3 delete_object 请求失败: {e}"))?;
        let status = resp.status();
        if !status.is_success() && status.as_u16() != 404 {
            return Err(format!("S3 DELETE 返回 {}", status));
        }
        Ok(())
    }

    async fn health(&self) -> Result<Value, String> {
        // 真实探测：对已配置 bucket 发一次 ListObjectsV2（列出 0 个）
        match self.list_objects(&self.bucket).await {
            Ok(_) => Ok(json!({"reachable": true, "endpoint": self.endpoint, "bucket": self.bucket})),
            Err(e) => Err(e),
        }
    }
}

/// 从 XML 片段提取首个 `<tag>...</tag>` 内容
fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let s = xml.find(&open)? + open.len();
    let e = xml[s..].find(&close)?;
    Some(xml[s..s + e].to_string())
}

fn truncate(s: &str, n: usize) -> &str {
    if s.len() > n { &s[..n] } else { s }
}
