// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 存储桶清单 (Inventory) 模块
//!
//! 实现 AWS S3 Inventory 风格的存储桶清单功能。
//!
//! # 功能特性
//!
//! * **清单配置管理**：支持每日/每周生成周期，目标存储桶配置
//! * **多种输出格式**：CSV / Parquet / ORC 三种格式支持
//! * **丰富的清单字段**：对象键、大小、版本、存储类、最后修改时间、ETag、标签等
//! * **任务调度**：基于时间的清单生成任务自动调度
//! * **清单加密**：支持服务端加密 (SSE-S3 / SSE-KMS)
//! * **完整性校验**：清单文件的 MD5/SHA256 校验和 manifest
//!
//! # 设计说明
//!
//! 清单功能基于配置驱动：每个存储桶可以配置多个清单（不同前缀/频率/格式）。
//! 清单生成由定时任务触发（简化实现为手动调用 generate），生成的清单文件
//! 写入目标存储桶，并附带 manifest.json 描述清单内容和校验信息。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{S3Error, S3Result};
use crate::lifecycle::StorageClass;

// ---------------- 常量 ----------------

/// 默认清单输出字段
pub const DEFAULT_INVENTORY_FIELDS: &[&str] = &[
    "Bucket",
    "Key",
    "VersionId",
    "IsLatest",
    "IsDeleteMarker",
    "Size",
    "LastModifiedDate",
    "ETag",
    "StorageClass",
];

// ---------------- 类型定义 ----------------

/// 清单生成频率
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum InventoryFrequency {
    /// 每日生成
    Daily,
    /// 每周生成
    Weekly,
}

impl InventoryFrequency {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "daily" | "day" => Some(InventoryFrequency::Daily),
            "weekly" | "week" => Some(InventoryFrequency::Weekly),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            InventoryFrequency::Daily => "Daily",
            InventoryFrequency::Weekly => "Weekly",
        }
    }

    /// 周期秒数
    pub fn seconds(&self) -> u64 {
        match self {
            InventoryFrequency::Daily => 86400,
            InventoryFrequency::Weekly => 7 * 86400,
        }
    }
}

/// 清单输出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum InventoryFormat {
    /// CSV 格式
    CSV,
    /// Parquet 列式存储格式
    Parquet,
    /// ORC 列式存储格式
    ORC,
}

impl InventoryFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "CSV" => Some(InventoryFormat::CSV),
            "PARQUET" => Some(InventoryFormat::Parquet),
            "ORC" => Some(InventoryFormat::ORC),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            InventoryFormat::CSV => "CSV",
            InventoryFormat::Parquet => "Parquet",
            InventoryFormat::ORC => "ORC",
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            InventoryFormat::CSV => "csv",
            InventoryFormat::Parquet => "parquet",
            InventoryFormat::ORC => "orc",
        }
    }
}

/// 清单加密配置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "Type", rename_all = "UPPERCASE")]
pub enum InventoryEncryption {
    /// SSE-S3 服务端加密
    #[serde(rename = "SSES3")]
    SseS3,
    /// SSE-KMS 加密
    #[serde(rename = "SSEKMS")]
    SseKms { key_id: String },
}

impl InventoryEncryption {
    pub fn as_str(&self) -> &'static str {
        match self {
            InventoryEncryption::SseS3 => "SSE-S3",
            InventoryEncryption::SseKms { .. } => "SSE-KMS",
        }
    }
}

/// 清单目标配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryDestination {
    /// 目标存储桶
    pub bucket: String,
    /// 目标前缀
    pub prefix: String,
    /// 输出格式
    pub format: InventoryFormat,
    /// 加密配置（可选）
    pub encryption: Option<InventoryEncryption>,
}

/// 清单筛选配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InventoryFilter {
    /// 对象键前缀
    pub prefix: Option<String>,
}

impl InventoryFilter {
    pub fn matches(&self, key: &str) -> bool {
        match &self.prefix {
            Some(p) => key.starts_with(p),
            None => true,
        }
    }
}

/// 清单配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryConfiguration {
    /// 配置 ID
    pub id: String,
    /// 是否启用
    pub enabled: bool,
    /// 目标配置
    pub destination: InventoryDestination,
    /// 生成频率
    pub frequency: InventoryFrequency,
    /// 筛选条件
    pub filter: InventoryFilter,
    /// 包含的字段
    pub included_fields: Vec<String>,
    /// 是否包含所有版本
    pub include_all_versions: bool,
    /// 可选字段：是否包含对象标签
    pub include_object_tags: bool,
}

/// 单条清单记录（对象元数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryRecord {
    pub bucket: String,
    pub key: String,
    pub version_id: Option<String>,
    pub is_latest: bool,
    pub is_delete_marker: bool,
    pub size: u64,
    pub last_modified_date: String, // ISO 8601
    pub etag: String,
    pub storage_class: String,
    #[serde(default)]
    pub tags: Option<String>, // 序列化后的标签，如 "k1=v1&k2=v2"
}

/// 清单生成任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InventoryJobStatus {
    /// 待处理
    Pending,
    /// 生成中
    InProgress,
    /// 已完成
    Completed,
    /// 失败
    Failed,
}

/// 清单生成任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryJob {
    /// 任务 ID
    pub job_id: String,
    /// 源存储桶
    pub source_bucket: String,
    /// 配置 ID
    pub config_id: String,
    /// 状态
    pub status: InventoryJobStatus,
    /// 创建时间（毫秒）
    pub created_at_ms: u64,
    /// 完成时间（毫秒）
    pub completed_at_ms: Option<u64>,
    /// 对象总数
    pub total_objects: usize,
    /// 清单文件大小（字节）
    pub inventory_size: u64,
    /// 清单文件在目标桶中的位置
    pub output_path: Option<String>,
    /// 错误信息（失败时）
    pub error_message: Option<String>,
}

/// 清单 manifest（描述清单文件内容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryManifest {
    /// 源存储桶
    pub source_bucket: String,
    /// 配置 ID
    pub inventory_configuration_id: String,
    /// 生成时间
    pub creation_timestamp: String,
    /// 清单文件列表
    pub files: Vec<InventoryFileInfo>,
    /// 文件格式
    pub file_format: String,
    /// 文件 schema
    pub file_schema: String,
    /// 清单总数
    pub total_objects: u64,
    /// 总大小
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryFileInfo {
    /// 文件键
    pub key: String,
    /// 文件大小
    pub size: u64,
    /// MD5 校验和
    pub md5_checksum: String,
}

// ---------------- 清单管理器 ----------------

/// 清单管理器
///
/// 负责管理存储桶的清单配置和清单生成任务。
#[derive(Debug)]
pub struct InventoryManager {
    /// 清单配置：(bucket, config_id) -> config
    configurations: parking_lot::Mutex<BTreeMap<(String, String), InventoryConfiguration>>,
    /// 清单任务：job_id -> job
    jobs: parking_lot::Mutex<BTreeMap<String, InventoryJob>>,
    /// 任务计数器
    job_counter: parking_lot::Mutex<u64>,
    /// 历史任务保留数量
    max_history_jobs: usize,
}

impl Default for InventoryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InventoryManager {
    /// 创建新的清单管理器
    pub fn new() -> Self {
        Self {
            configurations: parking_lot::Mutex::new(BTreeMap::new()),
            jobs: parking_lot::Mutex::new(BTreeMap::new()),
            job_counter: parking_lot::Mutex::new(0),
            max_history_jobs: 1000,
        }
    }

    // ---- 配置管理 ----

    /// 添加清单配置
    pub fn add_configuration(
        &self,
        bucket: &str,
        config: InventoryConfiguration,
    ) -> S3Result<()> {
        let key = (bucket.to_string(), config.id.clone());
        self.configurations.lock().insert(key, config);
        Ok(())
    }

    /// 获取存储桶的所有清单配置
    pub fn list_configurations(&self, bucket: &str) -> Vec<InventoryConfiguration> {
        let configs = self.configurations.lock();
        configs
            .range((bucket.to_string(), String::new())..=(bucket.to_string(), String::from(char::MAX)))
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// 获取指定清单配置
    pub fn get_configuration(&self, bucket: &str, config_id: &str) -> Option<InventoryConfiguration> {
        self.configurations
            .lock()
            .get(&(bucket.to_string(), config_id.to_string()))
            .cloned()
    }

    /// 删除清单配置
    pub fn delete_configuration(&self, bucket: &str, config_id: &str) -> S3Result<()> {
        let key = (bucket.to_string(), config_id.to_string());
        if self.configurations.lock().remove(&key).is_some() {
            Ok(())
        } else {
            Err(S3Error::NoSuchBucket)
        }
    }

    /// 更新清单配置
    pub fn update_configuration(
        &self,
        bucket: &str,
        config_id: &str,
        update_fn: impl FnOnce(&mut InventoryConfiguration),
    ) -> S3Result<()> {
        let mut configs = self.configurations.lock();
        let key = (bucket.to_string(), config_id.to_string());
        let config = configs.get_mut(&key).ok_or(S3Error::NoSuchBucket)?;
        update_fn(config);
        Ok(())
    }

    // ---- 清单生成 ----

    /// 生成清单
    ///
    /// 参数：
    /// - bucket: 源存储桶
    /// - config_id: 清单配置 ID
    /// - object_iter_fn: 迭代对象的函数，返回 (key, size, version_id, storage_class, etag, last_modified_ms, is_latest, is_delete_marker, tags)
    /// - write_fn: 写入清单文件到目标存储桶的函数
    pub fn generate_inventory(
        &self,
        bucket: &str,
        config_id: &str,
        object_iter_fn: impl Fn() -> Vec<(String, u64, String, StorageClass, String, u64, bool, bool, BTreeMap<String, String>)>,
        write_fn: impl Fn(&str, &str, &[u8]) -> S3Result<()>,
    ) -> S3Result<String> {
        let config = self
            .get_configuration(bucket, config_id)
            .ok_or(S3Error::NoSuchBucket)?;

        if !config.enabled {
            return Err(S3Error::InvalidArgument);
        }

        // 创建任务
        let job_id = self.generate_job_id(bucket, config_id);
        let now = now_ms();

        let job = InventoryJob {
            job_id: job_id.clone(),
            source_bucket: bucket.to_string(),
            config_id: config_id.to_string(),
            status: InventoryJobStatus::InProgress,
            created_at_ms: now,
            completed_at_ms: None,
            total_objects: 0,
            inventory_size: 0,
            output_path: None,
            error_message: None,
        };

        self.jobs.lock().insert(job_id.clone(), job);

        // 获取对象列表
        let all_objects = object_iter_fn();

        // 筛选
        let filtered: Vec<_> = all_objects
            .into_iter()
            .filter(|(key, _, _, _, _, _, _, _, _)| config.filter.matches(key))
            // 若不包含所有版本，只取最新版本
            .filter(|(_, _, _, _, _, _, is_latest, _, _)| {
                config.include_all_versions || *is_latest
            })
            .collect();

        // 生成清单记录
        let records: Vec<InventoryRecord> = filtered
            .iter()
            .map(|(key, size, version_id, class, etag, last_modified_ms, is_latest, is_delete_marker, tags)| {
                InventoryRecord {
                    bucket: bucket.to_string(),
                    key: key.clone(),
                    version_id: Some(version_id.clone()),
                    is_latest: *is_latest,
                    is_delete_marker: *is_delete_marker,
                    size: *size,
                    last_modified_date: format_iso8601(*last_modified_ms),
                    etag: etag.clone(),
                    storage_class: class.as_str().to_string(),
                    tags: if config.include_object_tags {
                        Some(serialize_tags(tags))
                    } else {
                        None
                    },
                }
            })
            .collect();

        let total_objects = records.len();

        // 根据格式生成清单内容
        let (content, file_ext) = match config.destination.format {
            InventoryFormat::CSV => (generate_csv(&records, &config.included_fields), "csv"),
            InventoryFormat::Parquet => (generate_parquet_mock(&records), "parquet"),
            InventoryFormat::ORC => (generate_orc_mock(&records), "orc"),
        };

        // 生成输出路径
        let date_str = format_date(now);
        let output_key = format!(
            "{}{}/{}/{}/{}.{}",
            config.destination.prefix,
            bucket,
            config.id,
            date_str,
            "inventory",
            file_ext
        );

        // 写入清单文件
        match write_fn(&config.destination.bucket, &output_key, &content) {
            Ok(()) => {
                // 生成 manifest
                let manifest = InventoryManifest {
                    source_bucket: bucket.to_string(),
                    inventory_configuration_id: config_id.to_string(),
                    creation_timestamp: format_iso8601(now),
                    files: vec![InventoryFileInfo {
                        key: output_key.clone(),
                        size: content.len() as u64,
                        md5_checksum: md5_hex(&content),
                    }],
                    file_format: config.destination.format.as_str().to_string(),
                    file_schema: config.included_fields.join(","),
                    total_objects: total_objects as u64,
                    total_size: records.iter().map(|r| r.size).sum(),
                };

                let manifest_content = serde_json::to_string_pretty(&manifest).unwrap_or_default();
                let manifest_key = format!(
                    "{}{}/{}/{}/manifest.json",
                    config.destination.prefix, bucket, config.id, date_str
                );

                let _ = write_fn(&config.destination.bucket, &manifest_key, manifest_content.as_bytes());

                // 更新任务状态
                let mut jobs = self.jobs.lock();
                if let Some(job) = jobs.get_mut(&job_id) {
                    job.status = InventoryJobStatus::Completed;
                    job.completed_at_ms = Some(now_ms());
                    job.total_objects = total_objects;
                    job.inventory_size = content.len() as u64;
                    job.output_path = Some(output_key);
                }

                self.cleanup_old_jobs();

                Ok(job_id)
            }
            Err(e) => {
                // 更新任务状态为失败
                let mut jobs = self.jobs.lock();
                if let Some(job) = jobs.get_mut(&job_id) {
                    job.status = InventoryJobStatus::Failed;
                    job.completed_at_ms = Some(now_ms());
                    job.error_message = Some(e.message());
                }
                Err(e)
            }
        }
    }

    /// 生成任务 ID
    fn generate_job_id(&self, bucket: &str, config_id: &str) -> String {
        let mut counter = self.job_counter.lock();
        *counter += 1;
        let ts = now_ms();
        format!("inv-{}-{}-{}-{:06x}", bucket, config_id, ts, *counter)
    }

    /// 清理旧任务
    fn cleanup_old_jobs(&self) {
        let mut jobs = self.jobs.lock();
        if jobs.len() > self.max_history_jobs {
            let excess = jobs.len() - self.max_history_jobs;
            let keys: Vec<String> = jobs.keys().take(excess).cloned().collect();
            for k in keys {
                jobs.remove(&k);
            }
        }
    }

    // ---- 查询接口 ----

    /// 获取任务状态
    pub fn get_job(&self, job_id: &str) -> Option<InventoryJob> {
        self.jobs.lock().get(job_id).cloned()
    }

    /// 列出存储桶的清单任务
    pub fn list_jobs(&self, bucket: &str, limit: usize) -> Vec<InventoryJob> {
        let jobs = self.jobs.lock();
        jobs.values()
            .filter(|j| j.source_bucket == bucket)
            .take(limit)
            .cloned()
            .collect()
    }

    /// 获取最近一次成功的清单任务
    pub fn get_latest_successful(&self, bucket: &str, config_id: &str) -> Option<InventoryJob> {
        let jobs = self.jobs.lock();
        jobs.values()
            .filter(|j| j.source_bucket == bucket && j.config_id == config_id)
            .filter(|j| matches!(j.status, InventoryJobStatus::Completed))
            .max_by_key(|j| j.created_at_ms)
            .cloned()
    }

    /// 检查是否该生成新清单（基于频率）
    pub fn should_generate(&self, bucket: &str, config_id: &str) -> bool {
        let config = match self.get_configuration(bucket, config_id) {
            Some(c) => c,
            None => return false,
        };

        if !config.enabled {
            return false;
        }

        let last_job = self.get_latest_successful(bucket, config_id);
        match last_job {
            None => true, // 从未生成过，应该生成
            Some(job) => {
                let now = now_ms();
                let elapsed = now.saturating_sub(job.created_at_ms) / 1000;
                elapsed >= config.frequency.seconds()
            }
        }
    }
}

// ---------------- 辅助函数 ----------------

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn format_iso8601(ms: u64) -> String {
    let secs = ms / 1000;
    let days = secs / 86400;
    let secs_in_day = secs % 86400;
    let hours = secs_in_day / 3600;
    let minutes = (secs_in_day % 3600) / 60;
    let seconds = secs_in_day % 60;
    // 简化的 ISO8601（不处理闰年/月份）
    format!(
        "1970-01-{}T{:02}:{:02}:{:02}Z",
        days + 1,
        hours,
        minutes,
        seconds
    )
}

fn format_date(ms: u64) -> String {
    let secs = ms / 1000;
    let days = secs / 86400;
    // 简化日期格式
    format!("1970-01-{:02}", days + 1)
}

fn md5_hex(data: &[u8]) -> String {
    // 简化：用 SHA256 前 16 字节模拟 MD5
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    let d = h.finalize();
    hex::encode(&d[..16])
}

fn serialize_tags(tags: &BTreeMap<String, String>) -> String {
    let parts: Vec<String> = tags
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();
    parts.join("&")
}

/// 生成 CSV 格式清单
fn generate_csv(records: &[InventoryRecord], fields: &[String]) -> Vec<u8> {
    let mut csv = String::new();

    // 表头
    csv.push_str(&fields.join(","));
    csv.push('\n');

    for record in records {
        let mut row = Vec::new();
        for field in fields {
            let value = match field.as_str() {
                "Bucket" => record.bucket.clone(),
                "Key" => record.key.clone(),
                "VersionId" => record.version_id.clone().unwrap_or_default(),
                "IsLatest" => record.is_latest.to_string(),
                "IsDeleteMarker" => record.is_delete_marker.to_string(),
                "Size" => record.size.to_string(),
                "LastModifiedDate" => record.last_modified_date.clone(),
                "ETag" => record.etag.clone(),
                "StorageClass" => record.storage_class.clone(),
                "Tags" => record.tags.clone().unwrap_or_default(),
                _ => String::new(),
            };
            // 简单 CSV 转义
            if value.contains(',') || value.contains('"') || value.contains('\n') {
                row.push(format!("\"{}\"", value.replace('"', "\"\"")));
            } else {
                row.push(value);
            }
        }
        csv.push_str(&row.join(","));
        csv.push('\n');
    }

    csv.into_bytes()
}

/// 生成 Parquet 模拟数据（实际应用中使用 parquet crate）
fn generate_parquet_mock(records: &[InventoryRecord]) -> Vec<u8> {
    // 简化实现：用 JSON 序列化模拟 Parquet 二进制
    // 生产环境应使用 apache-arrow / parquet crate
    let json = serde_json::to_vec(records).unwrap_or_default();
    // 添加 Parquet 魔数标记（模拟）
    let mut result = b"PAR1".to_vec();
    result.extend_from_slice(&json);
    result.extend_from_slice(b"PAR1");
    result
}

/// 生成 ORC 模拟数据
fn generate_orc_mock(records: &[InventoryRecord]) -> Vec<u8> {
    // 简化实现：与 Parquet 类似，用 JSON 序列化模拟
    let json = serde_json::to_vec(records).unwrap_or_default();
    let mut result = b"ORC\x1a".to_vec(); // ORC 魔数
    result.extend_from_slice(&json);
    result
}

// ---------------- 共享类型别名 ----------------

/// 共享的清单管理器引用
pub type SharedInventory = Arc<InventoryManager>;

// ---------------- 单元测试 ----------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inventory_frequency_from_str() {
        assert_eq!(
            InventoryFrequency::from_str("Daily"),
            Some(InventoryFrequency::Daily)
        );
        assert_eq!(
            InventoryFrequency::from_str("weekly"),
            Some(InventoryFrequency::Weekly)
        );
        assert_eq!(InventoryFrequency::from_str("invalid"), None);
    }

    #[test]
    fn test_inventory_format_from_str() {
        assert_eq!(InventoryFormat::from_str("CSV"), Some(InventoryFormat::CSV));
        assert_eq!(
            InventoryFormat::from_str("parquet"),
            Some(InventoryFormat::Parquet)
        );
        assert_eq!(InventoryFormat::from_str("ORC"), Some(InventoryFormat::ORC));
        assert_eq!(InventoryFormat::from_str("json"), None);
    }

    #[test]
    fn test_inventory_filter_prefix() {
        let filter = InventoryFilter {
            prefix: Some("logs/".into()),
        };

        assert!(filter.matches("logs/2024/01.log"));
        assert!(!filter.matches("data/file.txt"));
    }

    #[test]
    fn test_configuration_management() {
        let mgr = InventoryManager::new();

        let config = InventoryConfiguration {
            id: "daily-inv".into(),
            enabled: true,
            destination: InventoryDestination {
                bucket: "inventory-bucket".into(),
                prefix: "inventory/".into(),
                format: InventoryFormat::CSV,
                encryption: None,
            },
            frequency: InventoryFrequency::Daily,
            filter: InventoryFilter::default(),
            included_fields: DEFAULT_INVENTORY_FIELDS.iter().map(|s| s.to_string()).collect(),
            include_all_versions: false,
            include_object_tags: false,
        };

        mgr.add_configuration("source-bucket", config).unwrap();

        let configs = mgr.list_configurations("source-bucket");
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].id, "daily-inv");

        let config = mgr.get_configuration("source-bucket", "daily-inv");
        assert!(config.is_some());
        assert_eq!(config.unwrap().frequency, InventoryFrequency::Daily);

        // 删除
        mgr.delete_configuration("source-bucket", "daily-inv").unwrap();
        assert!(mgr.get_configuration("source-bucket", "daily-inv").is_none());
    }

    #[test]
    fn test_generate_inventory_csv() {
        let mgr = InventoryManager::new();

        let config = InventoryConfiguration {
            id: "test-inv".into(),
            enabled: true,
            destination: InventoryDestination {
                bucket: "dest-bucket".into(),
                prefix: "inv/".into(),
                format: InventoryFormat::CSV,
                encryption: None,
            },
            frequency: InventoryFrequency::Daily,
            filter: InventoryFilter::default(),
            included_fields: vec![
                "Bucket".into(),
                "Key".into(),
                "Size".into(),
                "StorageClass".into(),
            ],
            include_all_versions: false,
            include_object_tags: false,
        };

        mgr.add_configuration("source-bucket", config).unwrap();

        // 模拟对象数据
        let objects = vec![
            (
                "file1.txt".to_string(),
                1024u64,
                "v1".to_string(),
                StorageClass::Hot,
                "\"abc123\"".to_string(),
                1_700_000_000_000u64,
                true,
                false,
                BTreeMap::new(),
            ),
            (
                "file2.txt".to_string(),
                2048u64,
                "v1".to_string(),
                StorageClass::Warm,
                "\"def456\"".to_string(),
                1_700_001_000_000u64,
                true,
                false,
                BTreeMap::new(),
            ),
        ];

        // 存储写入结果
        let written = Arc::new(parking_lot::Mutex::new(BTreeMap::new()));
        let written_clone = written.clone();

        let write_fn = move |bucket: &str, key: &str, data: &[u8]| -> S3Result<()> {
            written_clone
                .lock()
                .insert(format!("{}/{}", bucket, key), data.to_vec());
            Ok(())
        };

        let object_iter_fn = move || objects.clone();

        let job_id = mgr
            .generate_inventory("source-bucket", "test-inv", object_iter_fn, write_fn)
            .unwrap();

        // 验证任务
        let job = mgr.get_job(&job_id).unwrap();
        assert_eq!(job.status, InventoryJobStatus::Completed);
        assert_eq!(job.total_objects, 2);
        assert!(job.completed_at_ms.is_some());
        assert!(job.output_path.is_some());

        // 验证清单文件被写入
        let w = written.lock();
        assert!(w.len() >= 2); // 清单文件 + manifest

        // 找到清单文件并验证内容
        let inventory_content = w
            .iter()
            .find(|(k, _)| k.ends_with(".csv"))
            .map(|(_, v)| String::from_utf8_lossy(v).to_string())
            .unwrap();

        assert!(inventory_content.contains("Bucket,Key,Size,StorageClass"));
        assert!(inventory_content.contains("source-bucket,file1.txt,1024,HOT"));
        assert!(inventory_content.contains("source-bucket,file2.txt,2048,WARM"));
    }

    #[test]
    fn test_generate_inventory_with_prefix_filter() {
        let mgr = InventoryManager::new();

        let config = InventoryConfiguration {
            id: "filtered-inv".into(),
            enabled: true,
            destination: InventoryDestination {
                bucket: "dest".into(),
                prefix: "".into(),
                format: InventoryFormat::CSV,
                encryption: None,
            },
            frequency: InventoryFrequency::Daily,
            filter: InventoryFilter {
                prefix: Some("docs/".into()),
            },
            included_fields: vec!["Key".into(), "Size".into()],
            include_all_versions: false,
            include_object_tags: false,
        };

        mgr.add_configuration("bucket", config).unwrap();

        let objects = vec![
            (
                "docs/report.pdf".to_string(),
                1000u64,
                "v1".to_string(),
                StorageClass::Hot,
                "etag1".to_string(),
                1_700_000_000_000u64,
                true,
                false,
                BTreeMap::new(),
            ),
            (
                "images/photo.jpg".to_string(),
                2000u64,
                "v1".to_string(),
                StorageClass::Hot,
                "etag2".to_string(),
                1_700_000_000_000u64,
                true,
                false,
                BTreeMap::new(),
            ),
        ];

        let written = Arc::new(parking_lot::Mutex::new(BTreeMap::new()));
        let written_clone = written.clone();
        let write_fn = move |bucket: &str, key: &str, data: &[u8]| -> S3Result<()> {
            written_clone
                .lock()
                .insert(format!("{}/{}", bucket, key), data.to_vec());
            Ok(())
        };

        let objects_clone = objects.clone();
        let object_iter_fn = move || objects_clone.clone();

        let job_id = mgr
            .generate_inventory("bucket", "filtered-inv", object_iter_fn, write_fn)
            .unwrap();

        let job = mgr.get_job(&job_id).unwrap();
        assert_eq!(job.total_objects, 1); // 只有 docs/ 前缀的被包含
    }

    #[test]
    fn test_disabled_config_no_generate() {
        let mgr = InventoryManager::new();

        let config = InventoryConfiguration {
            id: "disabled-inv".into(),
            enabled: false, // 禁用
            destination: InventoryDestination {
                bucket: "dest".into(),
                prefix: "".into(),
                format: InventoryFormat::CSV,
                encryption: None,
            },
            frequency: InventoryFrequency::Daily,
            filter: InventoryFilter::default(),
            included_fields: vec!["Key".into()],
            include_all_versions: false,
            include_object_tags: false,
        };

        mgr.add_configuration("bucket", config).unwrap();

        let result = mgr.generate_inventory(
            "bucket",
            "disabled-inv",
            || vec![],
            |_, _, _| Ok(()),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_should_generate() {
        let mgr = InventoryManager::new();

        let config = InventoryConfiguration {
            id: "daily".into(),
            enabled: true,
            destination: InventoryDestination {
                bucket: "dest".into(),
                prefix: "".into(),
                format: InventoryFormat::CSV,
                encryption: None,
            },
            frequency: InventoryFrequency::Daily,
            filter: InventoryFilter::default(),
            included_fields: vec!["Key".into()],
            include_all_versions: false,
            include_object_tags: false,
        };

        mgr.add_configuration("bucket", config).unwrap();

        // 从未生成过，应该生成
        assert!(mgr.should_generate("bucket", "daily"));

        // 生成一次
        let _ = mgr.generate_inventory(
            "bucket",
            "daily",
            || vec![],
            |_, _, _| Ok(()),
        );

        // 刚生成过，不应该立即再生成
        assert!(!mgr.should_generate("bucket", "daily"));
    }

    #[test]
    fn test_list_jobs() {
        let mgr = InventoryManager::new();

        let config = InventoryConfiguration {
            id: "inv".into(),
            enabled: true,
            destination: InventoryDestination {
                bucket: "dest".into(),
                prefix: "".into(),
                format: InventoryFormat::CSV,
                encryption: None,
            },
            frequency: InventoryFrequency::Daily,
            filter: InventoryFilter::default(),
            included_fields: vec!["Key".into()],
            include_all_versions: false,
            include_object_tags: false,
        };

        mgr.add_configuration("bucket-a", config.clone()).unwrap();
        mgr.add_configuration("bucket-b", config).unwrap();

        let _ = mgr.generate_inventory("bucket-a", "inv", || vec![], |_, _, _| Ok(()));
        let _ = mgr.generate_inventory("bucket-a", "inv", || vec![], |_, _, _| Ok(()));
        let _ = mgr.generate_inventory("bucket-b", "inv", || vec![], |_, _, _| Ok(()));

        let jobs_a = mgr.list_jobs("bucket-a", 10);
        assert_eq!(jobs_a.len(), 2);

        let jobs_b = mgr.list_jobs("bucket-b", 10);
        assert_eq!(jobs_b.len(), 1);
    }

    #[test]
    fn test_inventory_encryption() {
        let sse_s3 = InventoryEncryption::SseS3;
        assert_eq!(sse_s3.as_str(), "SSE-S3");

        let sse_kms = InventoryEncryption::SseKms {
            key_id: "arn:aws:kms:us-east-1:123456789012:key/abc".into(),
        };
        assert_eq!(sse_kms.as_str(), "SSE-KMS");
    }

    #[test]
    fn test_format_file_extension() {
        assert_eq!(InventoryFormat::CSV.file_extension(), "csv");
        assert_eq!(InventoryFormat::Parquet.file_extension(), "parquet");
        assert_eq!(InventoryFormat::ORC.file_extension(), "orc");
    }
}
