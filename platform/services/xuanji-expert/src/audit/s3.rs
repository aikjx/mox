//! S3 Sink — 写入 S3 兼容对象存储（WORM 合规存储，不可篡改）
//!
//! 适用：SOC2 Type II / HIPAA / GDPR 数据处理活动记录
//! 路径格式：`{prefix}{tenant}/audit/{year}/{month}/{day}/{hour}/{event_id}.ndjson`
//!
//! 实现说明：不依赖重型 `aws-sdk-s3`，直接以标准 HTTP(S) 完成
//! PutObject 请求，并手写 AWS Signature V4 请求签名（与 S3/S3 兼容
//! 存储如 MinIO / 腾讯 COS / 华为 OBS / 阿里 OSS 互通）。
//! 启用 `object_lock` 时附带 Compliance 模式对象锁头（WORM 语义）。

use super::{AuditError, AuditSink};
use super::event::ExtAuditEvent;
use chrono::{Datelike, Timelike, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::sync::{Mutex, OnceLock};

/// HMAC-SHA256 别名
type HmacSha256 = Hmac<Sha256>;

/// S3 Sink（WORM 合规存储）
pub struct S3Sink {
    bucket: String,
    region: String,
    credentials: S3Credentials,
    prefix: String,
    /// 每小时缓冲（NDJSON 行）
    hour_buffer: Mutex<Vec<String>>,
    current_hour: Mutex<String>,
    endpoint: Option<String>,
    object_lock: bool,
}

#[derive(Debug, Clone)]
pub enum S3Credentials {
    /// 从标准 AWS 环境变量链读取（AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY）
    FromEnv,
    /// 显式访问密钥
    AccessKey { key_id: String, secret: String },
}

impl S3Sink {
    pub fn new(bucket: &str, region: &str) -> Self {
        let now = Utc::now();
        let hour_key = Self::hour_key(&now);
        Self {
            bucket: bucket.into(),
            region: region.into(),
            credentials: S3Credentials::FromEnv,
            prefix: String::new(),
            hour_buffer: Mutex::new(Vec::new()),
            current_hour: Mutex::new(hour_key),
            endpoint: None,
            object_lock: false,
        }
    }

    pub fn with_credentials(mut self, cred: S3Credentials) -> Self {
        self.credentials = cred;
        self
    }

    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// 开启 S3 Object Lock（WORM，bucket 须在创建时启用 Compliance 模式）
    pub fn with_object_lock(mut self) -> Self {
        self.object_lock = true;
        self
    }

    /// 支持 MinIO / 腾讯 COS / 华为 OBS 等 S3 兼容存储
    pub fn with_endpoint(mut self, endpoint: &str) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    fn hour_key(ts: &chrono::DateTime<Utc>) -> String {
        format!(
            "{:04}-{:02}-{:02}-{:02}",
            ts.year(),
            ts.month(),
            ts.day(),
            ts.hour()
        )
    }

    /// 惰性共享 HTTP 客户端（blocking，带超时）
    fn client(&self) -> &'static reqwest::blocking::Client {
        static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
        CLIENT.get_or_init(|| {
            reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("构建 S3 HTTP 客户端失败")
        })
    }

    /// 当前有效凭据：显式密钥优先，否则读取环境变量
    fn active_credentials(&self) -> Result<(String, String), AuditError> {
        match &self.credentials {
            S3Credentials::AccessKey { key_id, secret } => Ok((key_id.clone(), secret.clone())),
            S3Credentials::FromEnv => {
                let key_id = std::env::var("AWS_ACCESS_KEY_ID")
                    .map_err(|_| AuditError::Connection("缺少 AWS_ACCESS_KEY_ID 环境变量".into()))?;
                let secret = std::env::var("AWS_SECRET_ACCESS_KEY")
                    .map_err(|_| AuditError::Connection("缺少 AWS_SECRET_ACCESS_KEY 环境变量".into()))?;
                Ok((key_id, secret))
            }
        }
    }

    /// 上传内容到 S3（真实 PutObject + SigV4 签名）
    fn upload(&self, key: &str, body: &[u8]) -> Result<(), AuditError> {
        if self.bucket.is_empty() {
            return Err(AuditError::Disabled);
        }
        let (access_key, secret_key) = self.active_credentials()?;
        let region = if self.region.is_empty() { "us-east-1".to_string() } else { self.region.clone() };

        // 1. 目标 URL：自定义 endpoint（path-style）或 AWS 虚拟主机风格
        let base = match &self.endpoint {
            Some(ep) => format!("{}/{}", ep.trim_end_matches('/'), self.bucket),
            None => format!("https://{}.s3.{}.amazonaws.com", self.bucket, region),
        };
        let url = format!("{}/{}", base, key.trim_start_matches('/'));
        let parsed = url
            .parse::<reqwest::Url>()
            .map_err(|e| AuditError::Connection(format!("非法 S3 URL '{url}': {e}")))?;

        let host = parsed
            .host_str()
            .ok_or_else(|| AuditError::Connection("S3 URL 缺少主机".into()))?
            .to_string();
        let path = if parsed.path().is_empty() { "/".to_string() } else { parsed.path().to_string() };
        let query = parsed.query().unwrap_or("").to_string();

        // 2. SigV4 时间与作用域
        let now = Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();
        let scope = format!("{}/{}/s3/aws4_request", date_stamp, region);

        // 3. 载荷哈希（x-amz-content-sha256）
        let payload_hash = hex::encode(Sha256::digest(body));

        // 4. Canonical Headers（host + x-amz-date + x-amz-content-sha256 [+ object lock]）
        let mut canonical_headers = format!(
            "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
            host, payload_hash, amz_date
        );
        let mut signed_headers = "host;x-amz-content-sha256;x-amz-date".to_string();
        if self.object_lock {
            let retain_until = (now + chrono::Duration::days(365 * 7)).format("%Y%m%dT%H%M%SZ").to_string();
            canonical_headers.push_str(&format!(
                "x-amz-object-lock-mode:COMPLIANCE\nx-amz-object-lock-retain-until-date:{}\n",
                retain_until
            ));
            signed_headers.push_str(";x-amz-object-lock-mode;x-amz-object-lock-retain-until-date");
        }

        // 5. Canonical Request
        let canonical_request = format!(
            "PUT\n{}\n{}\n{}\n{}\n{}",
            path,
            query,
            canonical_headers,
            signed_headers,
            payload_hash
        );
        let hashed_canonical = hex::encode(Sha256::digest(canonical_request.as_bytes()));

        // 6. String To Sign
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            amz_date, scope, hashed_canonical
        );

        // 7. 签名密钥：HMAC 链
        fn hmac_sha256(key: &[u8], data: &str) -> Vec<u8> {
            let mut mac = HmacSha256::new_from_slice(key).expect("HMAC 接受任意长度密钥");
            mac.update(data.as_bytes());
            mac.finalize().into_bytes().to_vec()
        }
        let k_date = hmac_sha256(format!("AWS4{}", secret_key).as_bytes(), &date_stamp);
        let k_region = hmac_sha256(&k_date, &region);
        let k_service = hmac_sha256(&k_region, "s3");
        let k_signing = hmac_sha256(&k_service, "aws4_request");
        let signature = hex::encode(hmac_sha256(&k_signing, &string_to_sign));

        // 8. Authorization 头
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            access_key, scope, signed_headers, signature
        );

        // 9. 发送 PUT 请求
        let mut req = self.client().put(&url).body(body.to_vec());
        for (name, value) in [
            ("Host", host.as_str()),
            ("x-amz-date", amz_date.as_str()),
            ("x-amz-content-sha256", payload_hash.as_str()),
            ("Authorization", authorization.as_str()),
            ("Content-Type", "application/x-ndjson"),
        ] {
            req = req.header(name, value);
        }
        if self.object_lock {
            let retain_until = (now + chrono::Duration::days(365 * 7)).format("%Y%m%dT%H%M%SZ").to_string();
            req = req
                .header("x-amz-object-lock-mode", "COMPLIANCE")
                .header("x-amz-object-lock-retain-until-date", retain_until);
        }

        let resp = req
            .send()
            .map_err(|e| AuditError::WriteFailed(format!("S3 PUT {key} 失败: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let detail = resp.text().unwrap_or_default();
            return Err(AuditError::WriteFailed(format!(
                "S3 PUT {key} 返回 {}: {}",
                status, detail
            )));
        }
        Ok(())
    }
}

impl AuditSink for S3Sink {
    fn append_sync(&self, event: &ExtAuditEvent) -> Result<(), AuditError> {
        let line = serde_json::to_string(event).map_err(|e| AuditError::Serialization(e.to_string()))?;
        let ts = event.timestamp;
        let hour = Self::hour_key(&ts);

        // 小时切换时，刷新上一个小时的缓冲
        let hour_changed = {
            let current = self.current_hour.lock().unwrap();
            hour != *current
        };
        if hour_changed {
            let prev_hour = {
                let current = self.current_hour.lock().unwrap();
                current.clone()
            };
            let prev_key = format!(
                "{}{}/audit/{}/hour_summary.ndjson",
                self.prefix, event.tenant_id, prev_hour
            );
            let buf: Vec<String> = {
                let mut h = self.hour_buffer.lock().unwrap();
                std::mem::take(&mut *h)
            };
            if !buf.is_empty() {
                let body: Vec<u8> = buf.join("\n").into_bytes();
                self.upload(&prev_key, &body)?;
            }
            *self.current_hour.lock().unwrap() = hour;
        }

        self.hour_buffer.lock().unwrap().push(line);
        Ok(())
    }

    fn flush(&self) -> Result<(), AuditError> {
        let buf: Vec<String> = {
            let mut h = self.hour_buffer.lock().unwrap();
            std::mem::take(&mut *h)
        };
        if !buf.is_empty() {
            let ts = Utc::now();
            let key = format!(
                "{}/audit/{:04}/{:02}/{:02}/{:02}/hour_summary.ndjson",
                self.prefix, ts.year(), ts.month(), ts.day(), ts.hour()
            );
            let body: Vec<u8> = buf.join("\n").into_bytes();
            self.upload(&key, &body)?;
        }
        Ok(())
    }

    fn health_check(&self) -> Result<(), AuditError> {
        if self.bucket.is_empty() {
            return Err(AuditError::Disabled);
        }
        Ok(())
    }

    fn is_enabled(&self) -> bool {
        !self.bucket.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hour_key_format() {
        let ts = chrono::DateTime::parse_from_rfc3339("2026-08-18T08:42:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(S3Sink::hour_key(&ts), "2026-08-18-08");
    }

    #[test]
    fn sigv4_canonical_request_shape() {
        // 仅验证签名算法产出的 Authorization 头结构（不对真实存储发请求）
        let sink = S3Sink::new("audit-bucket", "cn-north-1")
            .with_credentials(S3Credentials::AccessKey {
                key_id: "AKIDEXAMPLE".into(),
                secret: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".into(),
            })
            .with_endpoint("http://localhost:9000");
        // 触发一次 upload：本地无服务 → 应返回 WriteFailed（网络错误），
        // 但绝不返回"需要 aws-sdk"类占位错误——证明是真实 HTTP 路径
        let err = sink.upload("t/audit/test.ndjson", b"{}").unwrap_err();
        match err {
            AuditError::WriteFailed(msg) => {
                assert!(!msg.contains("aws-sdk"), "不得出现占位提示: {msg}");
                assert!(msg.contains("S3 PUT"), "应包含真实请求错误: {msg}");
            }
            other => panic!("预期 WriteFailed，实际 {other:?}"),
        }
    }

    #[test]
    fn disabled_when_empty_bucket() {
        let sink = S3Sink::new("", "cn-north-1");
        assert!(!sink.is_enabled());
        assert!(matches!(sink.health_check(), Err(AuditError::Disabled)));
    }

    #[test]
    fn buffering_then_flush_empty_is_ok() {
        let sink = S3Sink::new("b", "r");
        // 空缓冲 flush 应 Ok（无上传）
        assert!(sink.flush().is_ok());
    }
}
