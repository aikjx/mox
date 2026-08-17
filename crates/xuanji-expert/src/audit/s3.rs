//! S3 Sink — 写入 AWS S3（WORM 合规存储，不可篡改）
//! 
//! 适用：SOC2 Type II / HIPAA / GDPR 数据处理活动记录
//! 路径格式：s3://{bucket}/{tenant}/audit/{year}/{month}/{day}/{hour}/{event_id}.ndjson
//! 
//! 生产依赖：cargo add aws-sdk-s3 --features client-native-tls
//! 开发占位：无 reqwest 时返回明确的 "需要配置" 错误

use super::{AuditError, AuditSink};
use super::event::ExtAuditEvent;
use chrono::{Datelike, Timelike, Utc};
use std::sync::Mutex;

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
    IamRole,
    AccessKey { key_id: String, secret: String },
}

impl S3Sink {
    pub fn new(bucket: &str, region: &str) -> Self {
        let now = Utc::now();
        let hour_key = format!("{:04}-{:02}-{:02}-{:02}", now.year(), now.month(), now.day(), now.hour());
        Self {
            bucket: bucket.into(),
            region: region.into(),
            credentials: S3Credentials::IamRole,
            prefix: String::new(),
            hour_buffer: Mutex::new(Vec::new()),
            current_hour: Mutex::new(hour_key),
            endpoint: None,
            object_lock: false,
        }
    }

    pub fn with_credentials(mut self, cred: S3Credentials) -> Self { self.credentials = cred; self }

    pub fn with_prefix(mut self, prefix: &str) -> Self { self.prefix = prefix.into(); self }

    /// 开启 S3 Object Lock（WORM，bucket 须在创建时启用）
    pub fn with_object_lock(mut self) -> Self { self.object_lock = true; self }

    /// 支持 MinIO / 腾讯 COS / 华为 OBS 等 S3 兼容存储
    pub fn with_endpoint(mut self, endpoint: &str) -> Self { self.endpoint = Some(endpoint.into()); self }

    // 生产上传时由 append_sync 调用，计算对象存储路径；占位实现下暂未触发
    #[allow(dead_code)]
    fn key_for(&self, event: &ExtAuditEvent) -> String {
        let ts = event.timestamp;
        format!(
            "{}{}/audit/{:04}/{:02}/{:02}/{:02}/{}.ndjson",
            self.prefix, event.tenant_id,
            ts.year(), ts.month(), ts.day(), ts.hour(),
            event.event_id,
        )
    }

    fn hour_key(ts: &chrono::DateTime<Utc>) -> String {
        format!("{:04}-{:02}-{:02}-{:02}", ts.year(), ts.month(), ts.day(), ts.hour())
    }

    /// 上传内容到 S3（生产实现需 aws-sdk-s3）
    fn upload(&self, key: &str, body: &[u8]) -> Result<(), AuditError> {
        // 占位实现：生产版用 key/body 写入对象存储（见下方注释实现）
        let _ = (key, body);
        // === 生产实现（取消注释并配置依赖）===
        // use aws_sdk_s3 as s3;
        // let config = aws_config::default_loader().load().await;
        // let client = s3::Client::new(&config);
        // let put = PutObjectInput {
        //     bucket: self.bucket.clone(),
        //     key: key.into(),
        //     body: body.into(),
        //     content_type: Some("application/x-ndjson".into()),
        //     ..Default::default()
        // };
        // if self.object_lock {
        //     put.object_lock_mode = Some(s3::types::ObjectLockMode::Compliance);
        //     put.object_lock_retain_until_date = Some(
        //         (Utc::now() + chrono::Duration::days(365 * 7)).into()
        //     );
        // }
        // client.put_object(put).await.map_err(|e| AuditError::WriteFailed(e.to_string()))?;

        // === 当前占位：检测到缺少 AWS SDK 时给出明确错误 ===
        Err(AuditError::WriteFailed(format!(
            "S3 upload requires 'aws-sdk-s3' + 'aws-config'. \
             Configure: bucket={}, region={}, endpoint={:?}. \
             Run: cargo add aws-sdk-s3 --features client-native-tls && cargo add aws-config",
            self.bucket, self.region, self.endpoint,
        )))
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
        if self.bucket.is_empty() { return Err(AuditError::Disabled); }
        Ok(())
    }
}
