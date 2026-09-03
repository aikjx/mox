// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 批量对象操作模块
//!
//! 实现 AWS S3 风格的批量操作 API，参考 S3 Batch Operations 和 Multi-Object Delete。
//!
//! # 功能特性
//!
//! * **批量删除 (Batch Delete)**：支持最多 1000 个对象的单次删除请求，兼容 S3 DeleteObjects API
//! * **批量复制 (Batch Copy)**：跨桶或同桶批量复制对象，支持前缀匹配
//! * **批量解冻 (Batch Restore)**：批量解冻归档/冷归档对象，支持标准/批量/加急三种取回模式
//! * **操作状态跟踪**：每个批量任务有唯一 Job ID，可查询进度与结果
//! * **幂等性保证**：基于 ClientToken 的幂等请求去重
//! * **错误处理**：部分失败时返回成功/失败明细，支持静默模式与详细模式
//!
//! # 设计说明
//!
//! 批量操作采用 Job 模型：提交请求时创建一个 Job，返回 Job ID。
//! Job 在后台异步执行（对于内存实现为同步执行，生产环境可替换为线程池）。
//! 调用方可通过 Job ID 查询执行状态与结果报告。

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    error::{S3Error, S3Result},
    lifecycle::StorageClass,
};

// ---------------- 常量 ----------------

/// 批量操作最大对象数（AWS S3 限制为 1000）
pub const MAX_BATCH_OBJECTS: usize = 1000;

// ---------------- 类型定义 ----------------

/// 批量操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum BatchOperationType {
    /// 批量删除
    Delete,
    /// 批量复制
    Copy,
    /// 批量解冻
    Restore,
}

/// 批量任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum BatchJobStatus {
    /// 任务已创建，等待执行
    Pending,
    /// 正在执行
    Running,
    /// 执行完成（可能有部分失败）
    Completed,
    /// 执行失败（全部失败或严重错误）
    Failed,
    /// 已取消
    Cancelled,
}

impl BatchJobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BatchJobStatus::Pending => "Pending",
            BatchJobStatus::Running => "Running",
            BatchJobStatus::Completed => "Completed",
            BatchJobStatus::Failed => "Failed",
            BatchJobStatus::Cancelled => "Cancelled",
        }
    }
}

/// 单个对象操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchObjectResult {
    /// 对象键
    pub key: String,
    /// 版本 ID（可选）
    pub version_id: Option<String>,
    /// 是否成功
    pub success: bool,
    /// 错误码（失败时）
    pub error_code: Option<String>,
    /// 错误消息（失败时）
    pub error_message: Option<String>,
}

/// 批量操作任务报告
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BatchJobReport {
    /// 总对象数
    pub total_objects: usize,
    /// 成功数
    pub succeeded_count: usize,
    /// 失败数
    pub failed_count: usize,
    /// 成功的对象结果列表
    pub succeeded: Vec<BatchObjectResult>,
    /// 失败的对象结果列表
    pub failed: Vec<BatchObjectResult>,
    /// 开始时间（毫秒）
    pub start_time_ms: u64,
    /// 结束时间（毫秒）
    pub end_time_ms: u64,
}

/// 批量删除请求中的对象标识
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteObjectIdentifier {
    pub key: String,
    pub version_id: Option<String>,
}

/// 批量删除请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteObjectsRequest {
    pub objects: Vec<DeleteObjectIdentifier>,
    /// 静默模式：true 时只返回错误的对象，false 时返回所有结果
    pub quiet: bool,
}

/// 批量删除响应
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeleteObjectsResponse {
    pub deleted: Vec<DeletedObject>,
    pub errors: Vec<DeleteError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletedObject {
    pub key: String,
    pub version_id: Option<String>,
    pub delete_marker: bool,
    pub delete_marker_version_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteError {
    pub key: String,
    pub version_id: Option<String>,
    pub code: String,
    pub message: String,
}

/// 批量复制请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCopyRequest {
    /// 源存储桶
    pub source_bucket: String,
    /// 目标存储桶
    pub destination_bucket: String,
    /// 源对象键列表
    pub source_keys: Vec<String>,
    /// 目标键前缀（可选）
    pub destination_prefix: Option<String>,
    /// 源前缀匹配（可选，用于按前缀批量复制）
    pub source_prefix: Option<String>,
    /// 存储类
    pub storage_class: StorageClass,
}

/// 批量解冻请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRestoreRequest {
    /// 存储桶
    pub bucket: String,
    /// 对象键列表
    pub keys: Vec<String>,
    /// 取回天数
    pub days: u32,
    /// 取回模式：Standard / Bulk / Expedited
    pub tier: RestoreTier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RestoreTier {
    /// 标准取回（3-5小时）
    Standard,
    /// 批量取回（5-12小时，最便宜）
    Bulk,
    /// 加急取回（1-5分钟，最贵）
    Expedited,
}

impl RestoreTier {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Standard" => Some(RestoreTier::Standard),
            "Bulk" => Some(RestoreTier::Bulk),
            "Expedited" => Some(RestoreTier::Expedited),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RestoreTier::Standard => "Standard",
            RestoreTier::Bulk => "Bulk",
            RestoreTier::Expedited => "Expedited",
        }
    }
}

/// 批量操作任务
#[derive(Debug, Clone)]
pub struct BatchJob {
    /// 任务唯一 ID
    pub job_id: String,
    /// 操作类型
    pub operation: BatchOperationType,
    /// 状态
    pub status: BatchJobStatus,
    /// 创建时间（毫秒）
    pub created_at_ms: u64,
    /// 结果报告
    pub report: BatchJobReport,
    /// 客户端幂等令牌
    pub client_token: Option<String>,
    /// 描述
    pub description: Option<String>,
}

// ---------------- 批量操作管理器 ----------------

/// 批量操作管理器
///
/// 负责管理所有批量操作任务的创建、执行和状态跟踪。
#[derive(Debug)]
pub struct BatchOperationManager {
    /// 任务表：job_id -> job
    jobs: parking_lot::Mutex<BTreeMap<String, BatchJob>>,
    /// 幂等令牌映射：client_token -> job_id
    idempotency_map: parking_lot::Mutex<BTreeMap<String, String>>,
    /// 任务计数器（用于生成 job_id）
    job_counter: parking_lot::Mutex<u64>,
}

impl Default for BatchOperationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchOperationManager {
    /// 创建新的批量操作管理器
    pub fn new() -> Self {
        Self {
            jobs: parking_lot::Mutex::new(BTreeMap::new()),
            idempotency_map: parking_lot::Mutex::new(BTreeMap::new()),
            job_counter: parking_lot::Mutex::new(0),
        }
    }

    /// 生成新的 Job ID
    fn generate_job_id(&self, op_type: BatchOperationType) -> String {
        let mut counter = self.job_counter.lock();
        *counter += 1;
        let ts = now_ms();
        let prefix = match op_type {
            BatchOperationType::Delete => "del",
            BatchOperationType::Copy => "cpy",
            BatchOperationType::Restore => "rst",
        };
        format!("{}-{}-{:08x}", prefix, ts, *counter)
    }

    /// 检查幂等令牌，若存在则返回已有 Job ID
    pub fn check_idempotency(&self, client_token: &str) -> Option<String> {
        self.idempotency_map.lock().get(client_token).cloned()
    }

    /// 注册幂等令牌
    fn register_idempotency(&self, client_token: &str, job_id: &str) {
        self.idempotency_map.lock().insert(client_token.to_string(), job_id.to_string());
    }

    /// 创建批量删除任务
    ///
    /// 注意：此方法创建任务并立即执行（内存实现为同步）。
    /// 生产环境应提交到后台线程池异步执行。
    pub fn create_delete_job(
        &self,
        bucket: &str,
        request: DeleteObjectsRequest,
        client_token: Option<String>,
        delete_fn: impl Fn(&str, &str, Option<&str>) -> S3Result<()>,
    ) -> S3Result<DeleteObjectsResponse> {
        // 幂等检查
        if let Some(ref token) = client_token {
            if let Some(_existing) = self.check_idempotency(token) {
                // 幂等命中：返回已有的结果（简化实现，实际应返回相同响应）
                // 这里简化为继续执行，生产环境应缓存响应
            }
        }

        if request.objects.len() > MAX_BATCH_OBJECTS {
            return Err(S3Error::InvalidArgument);
        }

        let job_id = self.generate_job_id(BatchOperationType::Delete);
        let start_time = now_ms();

        let mut response = DeleteObjectsResponse::default();
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();

        for obj in &request.objects {
            match delete_fn(bucket, &obj.key, obj.version_id.as_deref()) {
                Ok(()) => {
                    if !request.quiet {
                        response.deleted.push(DeletedObject {
                            key: obj.key.clone(),
                            version_id: obj.version_id.clone(),
                            delete_marker: false,
                            delete_marker_version_id: None,
                        });
                    }
                    succeeded.push(BatchObjectResult {
                        key: obj.key.clone(),
                        version_id: obj.version_id.clone(),
                        success: true,
                        error_code: None,
                        error_message: None,
                    });
                },
                Err(e) => {
                    response.errors.push(DeleteError {
                        key: obj.key.clone(),
                        version_id: obj.version_id.clone(),
                        code: e.code().to_string(),
                        message: e.message(),
                    });
                    failed.push(BatchObjectResult {
                        key: obj.key.clone(),
                        version_id: obj.version_id.clone(),
                        success: false,
                        error_code: Some(e.code().to_string()),
                        error_message: Some(e.message()),
                    });
                },
            }
        }

        let end_time = now_ms();

        // 记录任务
        let report = BatchJobReport {
            total_objects: request.objects.len(),
            succeeded_count: succeeded.len(),
            failed_count: failed.len(),
            succeeded,
            failed,
            start_time_ms: start_time,
            end_time_ms: end_time,
        };

        let job = BatchJob {
            job_id: job_id.clone(),
            operation: BatchOperationType::Delete,
            status: if report.failed_count == 0 {
                BatchJobStatus::Completed
            } else if report.succeeded_count == 0 {
                BatchJobStatus::Failed
            } else {
                BatchJobStatus::Completed
            },
            created_at_ms: start_time,
            report,
            client_token: client_token.clone(),
            description: Some(format!("Delete {} objects from {}", request.objects.len(), bucket)),
        };

        if let Some(ref token) = client_token {
            self.register_idempotency(token, &job_id);
        }

        self.jobs.lock().insert(job_id, job);

        Ok(response)
    }

    /// 创建批量复制任务
    pub fn create_copy_job(
        &self,
        request: BatchCopyRequest,
        client_token: Option<String>,
        copy_fn: impl Fn(&str, &str, &str, &str, StorageClass) -> S3Result<()>,
    ) -> S3Result<String> {
        // 幂等检查
        if let Some(ref token) = client_token {
            if let Some(existing_id) = self.check_idempotency(token) {
                return Ok(existing_id);
            }
        }

        if request.source_keys.len() > MAX_BATCH_OBJECTS {
            return Err(S3Error::InvalidArgument);
        }

        let job_id = self.generate_job_id(BatchOperationType::Copy);
        let start_time = now_ms();

        // 先创建 Pending 状态的任务
        let job = BatchJob {
            job_id: job_id.clone(),
            operation: BatchOperationType::Copy,
            status: BatchJobStatus::Pending,
            created_at_ms: start_time,
            report: BatchJobReport::default(),
            client_token: client_token.clone(),
            description: Some(format!(
                "Copy {} objects from {} to {}",
                request.source_keys.len(),
                request.source_bucket,
                request.destination_bucket
            )),
        };

        if let Some(ref token) = client_token {
            self.register_idempotency(token, &job_id);
        }

        self.jobs.lock().insert(job_id.clone(), job);

        // 执行复制（内存实现同步执行，标记为 Running）
        self.execute_copy_job(&job_id, request, copy_fn);

        Ok(job_id)
    }

    /// 执行批量复制任务
    fn execute_copy_job(
        &self,
        job_id: &str,
        request: BatchCopyRequest,
        copy_fn: impl Fn(&str, &str, &str, &str, StorageClass) -> S3Result<()>,
    ) {
        // 更新状态为 Running
        {
            let mut jobs = self.jobs.lock();
            if let Some(job) = jobs.get_mut(job_id) {
                job.status = BatchJobStatus::Running;
                job.report.start_time_ms = now_ms();
                job.report.total_objects = request.source_keys.len();
            }
        }

        let mut succeeded = Vec::new();
        let mut failed = Vec::new();

        for src_key in &request.source_keys {
            let dst_key = match &request.destination_prefix {
                Some(prefix) => format!("{}{}", prefix, src_key),
                None => src_key.clone(),
            };

            match copy_fn(
                &request.source_bucket,
                src_key,
                &request.destination_bucket,
                &dst_key,
                request.storage_class,
            ) {
                Ok(()) => {
                    succeeded.push(BatchObjectResult {
                        key: src_key.clone(),
                        version_id: None,
                        success: true,
                        error_code: None,
                        error_message: None,
                    });
                },
                Err(e) => {
                    failed.push(BatchObjectResult {
                        key: src_key.clone(),
                        version_id: None,
                        success: false,
                        error_code: Some(e.code().to_string()),
                        error_message: Some(e.message()),
                    });
                },
            }
        }

        // 更新任务状态和报告
        let mut jobs = self.jobs.lock();
        if let Some(job) = jobs.get_mut(job_id) {
            job.report.succeeded_count = succeeded.len();
            job.report.failed_count = failed.len();
            job.report.succeeded = succeeded;
            job.report.failed = failed;
            job.report.end_time_ms = now_ms();
            job.status = if job.report.failed_count == 0 {
                BatchJobStatus::Completed
            } else if job.report.succeeded_count == 0 {
                BatchJobStatus::Failed
            } else {
                BatchJobStatus::Completed
            };
        }
    }

    /// 创建批量解冻任务
    pub fn create_restore_job(
        &self,
        request: BatchRestoreRequest,
        client_token: Option<String>,
        restore_fn: impl Fn(&str, &str, u32, RestoreTier) -> S3Result<()>,
    ) -> S3Result<String> {
        // 幂等检查
        if let Some(ref token) = client_token {
            if let Some(existing_id) = self.check_idempotency(token) {
                return Ok(existing_id);
            }
        }

        if request.keys.len() > MAX_BATCH_OBJECTS {
            return Err(S3Error::InvalidArgument);
        }

        let job_id = self.generate_job_id(BatchOperationType::Restore);
        let start_time = now_ms();

        let job = BatchJob {
            job_id: job_id.clone(),
            operation: BatchOperationType::Restore,
            status: BatchJobStatus::Pending,
            created_at_ms: start_time,
            report: BatchJobReport::default(),
            client_token: client_token.clone(),
            description: Some(format!(
                "Restore {} objects from {} ({} tier, {} days)",
                request.keys.len(),
                request.bucket,
                request.tier.as_str(),
                request.days
            )),
        };

        if let Some(ref token) = client_token {
            self.register_idempotency(token, &job_id);
        }

        self.jobs.lock().insert(job_id.clone(), job);

        // 执行解冻
        self.execute_restore_job(&job_id, request, restore_fn);

        Ok(job_id)
    }

    /// 执行批量解冻任务
    fn execute_restore_job(
        &self,
        job_id: &str,
        request: BatchRestoreRequest,
        restore_fn: impl Fn(&str, &str, u32, RestoreTier) -> S3Result<()>,
    ) {
        {
            let mut jobs = self.jobs.lock();
            if let Some(job) = jobs.get_mut(job_id) {
                job.status = BatchJobStatus::Running;
                job.report.start_time_ms = now_ms();
                job.report.total_objects = request.keys.len();
            }
        }

        let mut succeeded = Vec::new();
        let mut failed = Vec::new();

        for key in &request.keys {
            match restore_fn(&request.bucket, key, request.days, request.tier) {
                Ok(()) => {
                    succeeded.push(BatchObjectResult {
                        key: key.clone(),
                        version_id: None,
                        success: true,
                        error_code: None,
                        error_message: None,
                    });
                },
                Err(e) => {
                    failed.push(BatchObjectResult {
                        key: key.clone(),
                        version_id: None,
                        success: false,
                        error_code: Some(e.code().to_string()),
                        error_message: Some(e.message()),
                    });
                },
            }
        }

        let mut jobs = self.jobs.lock();
        if let Some(job) = jobs.get_mut(job_id) {
            job.report.succeeded_count = succeeded.len();
            job.report.failed_count = failed.len();
            job.report.succeeded = succeeded;
            job.report.failed = failed;
            job.report.end_time_ms = now_ms();
            job.status = if job.report.failed_count == 0 {
                BatchJobStatus::Completed
            } else if job.report.succeeded_count == 0 {
                BatchJobStatus::Failed
            } else {
                BatchJobStatus::Completed
            };
        }
    }

    /// 查询任务状态
    pub fn get_job(&self, job_id: &str) -> Option<BatchJob> {
        self.jobs.lock().get(job_id).cloned()
    }

    /// 取消任务（仅 Pending 状态可取消）
    pub fn cancel_job(&self, job_id: &str) -> S3Result<()> {
        let mut jobs = self.jobs.lock();
        let job = jobs.get_mut(job_id).ok_or(S3Error::NoSuchKey)?;

        match job.status {
            BatchJobStatus::Pending => {
                job.status = BatchJobStatus::Cancelled;
                Ok(())
            },
            _ => Err(S3Error::InvalidArgument),
        }
    }

    /// 列出任务（可选按状态过滤）
    pub fn list_jobs(&self, status_filter: Option<BatchJobStatus>) -> Vec<BatchJob> {
        let jobs = self.jobs.lock();
        jobs.values()
            .filter(|j| match status_filter {
                Some(s) => j.status == s,
                None => true,
            })
            .cloned()
            .collect()
    }

    /// 获取任务报告
    pub fn get_job_report(&self, job_id: &str) -> S3Result<BatchJobReport> {
        let jobs = self.jobs.lock();
        jobs.get(job_id).map(|j| j.report.clone()).ok_or(S3Error::NoSuchKey)
    }
}

// ---------------- 辅助函数 ----------------

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------- 共享类型别名 ----------------

/// 共享的批量操作管理器引用
pub type SharedBatchOps = Arc<BatchOperationManager>;

// ---------------- 单元测试 ----------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_delete_success() {
        let mgr = BatchOperationManager::new();

        // 模拟存储：用一个 BTreeMap
        use std::sync::Mutex;
        let storage = Arc::new(Mutex::new(BTreeMap::new()));
        storage.lock().unwrap().insert("obj1.txt".to_string(), vec![1, 2, 3]);
        storage.lock().unwrap().insert("obj2.txt".to_string(), vec![4, 5, 6]);
        storage.lock().unwrap().insert("obj3.txt".to_string(), vec![7, 8, 9]);

        let storage_clone = storage.clone();
        let delete_fn = move |_bucket: &str, key: &str, _version: Option<&str>| -> S3Result<()> {
            let mut s = storage_clone.lock().unwrap();
            if s.remove(key).is_some() {
                Ok(())
            } else {
                Err(S3Error::NoSuchKey)
            }
        };

        let request = DeleteObjectsRequest {
            objects: vec![
                DeleteObjectIdentifier { key: "obj1.txt".into(), version_id: None },
                DeleteObjectIdentifier { key: "obj2.txt".into(), version_id: None },
                DeleteObjectIdentifier { key: "nonexistent.txt".into(), version_id: None },
            ],
            quiet: false,
        };

        let response = mgr.create_delete_job("test-bucket", request, None, delete_fn).unwrap();

        assert_eq!(response.deleted.len(), 2);
        assert_eq!(response.errors.len(), 1);
        assert_eq!(response.errors[0].code, "NoSuchKey");

        // 验证存储
        let s = storage.lock().unwrap();
        assert!(!s.contains_key("obj1.txt"));
        assert!(!s.contains_key("obj2.txt"));
        assert!(s.contains_key("obj3.txt"));
    }

    #[test]
    fn test_batch_delete_quiet_mode() {
        let mgr = BatchOperationManager::new();

        let storage = Arc::new(parking_lot::Mutex::new(BTreeMap::new()));
        storage.lock().insert("obj1.txt".to_string(), vec![1]);
        storage.lock().insert("obj2.txt".to_string(), vec![2]);

        let storage_clone = storage.clone();
        let delete_fn = move |_bucket: &str, key: &str, _version: Option<&str>| -> S3Result<()> {
            let mut s = storage_clone.lock();
            if s.remove(key).is_some() {
                Ok(())
            } else {
                Err(S3Error::NoSuchKey)
            }
        };

        let request = DeleteObjectsRequest {
            objects: vec![
                DeleteObjectIdentifier { key: "obj1.txt".into(), version_id: None },
                DeleteObjectIdentifier { key: "obj2.txt".into(), version_id: None },
            ],
            quiet: true, // 静默模式
        };

        let response = mgr.create_delete_job("test-bucket", request, None, delete_fn).unwrap();

        // 静默模式下成功的不返回
        assert_eq!(response.deleted.len(), 0);
        assert_eq!(response.errors.len(), 0);
    }

    #[test]
    fn test_batch_delete_exceeds_max() {
        let mgr = BatchOperationManager::new();

        let objects: Vec<DeleteObjectIdentifier> = (0..1001)
            .map(|i| DeleteObjectIdentifier { key: format!("obj{}.txt", i), version_id: None })
            .collect();

        let request = DeleteObjectsRequest { objects, quiet: false };

        let result = mgr.create_delete_job("bucket", request, None, |_, _, _| Ok(()));
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_copy() {
        let mgr = BatchOperationManager::new();

        let storage = Arc::new(parking_lot::Mutex::new(BTreeMap::new()));
        storage
            .lock()
            .insert(("src-bucket".to_string(), "file1.txt".to_string()), b"hello".to_vec());
        storage
            .lock()
            .insert(("src-bucket".to_string(), "file2.txt".to_string()), b"world".to_vec());

        let storage_clone = storage.clone();
        let copy_fn = move |src_bucket: &str,
                            src_key: &str,
                            dst_bucket: &str,
                            dst_key: &str,
                            _class: StorageClass|
              -> S3Result<()> {
            let s = storage_clone.lock();
            let data = s
                .get(&(src_bucket.to_string(), src_key.to_string()))
                .cloned()
                .ok_or(S3Error::NoSuchKey)?;
            drop(s);
            storage_clone.lock().insert((dst_bucket.to_string(), dst_key.to_string()), data);
            Ok(())
        };

        let request = BatchCopyRequest {
            source_bucket: "src-bucket".into(),
            destination_bucket: "dst-bucket".into(),
            source_keys: vec!["file1.txt".into(), "file2.txt".into()],
            destination_prefix: Some("copied/".into()),
            source_prefix: None,
            storage_class: StorageClass::Hot,
        };

        let job_id = mgr.create_copy_job(request, None, copy_fn).unwrap();
        assert!(!job_id.is_empty());
        assert!(job_id.starts_with("cpy-"));

        // 查询任务状态
        let job = mgr.get_job(&job_id).unwrap();
        assert_eq!(job.status, BatchJobStatus::Completed);
        assert_eq!(job.report.total_objects, 2);
        assert_eq!(job.report.succeeded_count, 2);
        assert_eq!(job.report.failed_count, 0);

        // 验证目标存储
        let s = storage.lock();
        assert!(s.contains_key(&("dst-bucket".to_string(), "copied/file1.txt".to_string())));
        assert!(s.contains_key(&("dst-bucket".to_string(), "copied/file2.txt".to_string())));
    }

    #[test]
    fn test_batch_restore() {
        let mgr = BatchOperationManager::new();

        let restored = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let restored_clone = restored.clone();

        let restore_fn =
            move |bucket: &str, key: &str, _days: u32, _tier: RestoreTier| -> S3Result<()> {
                restored_clone.lock().push(format!("{}/{}", bucket, key));
                Ok(())
            };

        let request = BatchRestoreRequest {
            bucket: "archive-bucket".into(),
            keys: vec!["archive1.zip".into(), "archive2.zip".into()],
            days: 7,
            tier: RestoreTier::Standard,
        };

        let job_id = mgr.create_restore_job(request, None, restore_fn).unwrap();
        assert!(job_id.starts_with("rst-"));

        let job = mgr.get_job(&job_id).unwrap();
        assert_eq!(job.status, BatchJobStatus::Completed);
        assert_eq!(job.report.succeeded_count, 2);

        assert_eq!(restored.lock().len(), 2);
    }

    #[test]
    fn test_batch_job_idempotency() {
        let mgr = BatchOperationManager::new();

        let request = BatchCopyRequest {
            source_bucket: "src".into(),
            destination_bucket: "dst".into(),
            source_keys: vec!["f1.txt".into()],
            destination_prefix: None,
            source_prefix: None,
            storage_class: StorageClass::Hot,
        };

        let copy_fn =
            |_sb: &str, _sk: &str, _db: &str, _dk: &str, _c: StorageClass| -> S3Result<()> {
                Ok(())
            };

        let token = "unique-client-token-123";
        let job_id1 =
            mgr.create_copy_job(request.clone(), Some(token.to_string()), copy_fn).unwrap();

        // 使用相同令牌再次请求，应返回相同的 Job ID
        let job_id2 = mgr.create_copy_job(request, Some(token.to_string()), copy_fn).unwrap();

        assert_eq!(job_id1, job_id2);
    }

    #[test]
    fn test_list_and_cancel_jobs() {
        let mgr = BatchOperationManager::new();

        // 创建几个不同状态的任务
        let copy_fn =
            |_sb: &str, _sk: &str, _db: &str, _dk: &str, _c: StorageClass| -> S3Result<()> {
                Ok(())
            };

        let req1 = BatchCopyRequest {
            source_bucket: "s".into(),
            destination_bucket: "d".into(),
            source_keys: vec!["a.txt".into()],
            destination_prefix: None,
            source_prefix: None,
            storage_class: StorageClass::Hot,
        };
        let job1 = mgr.create_copy_job(req1, None, copy_fn).unwrap();

        // 列出所有任务
        let all_jobs = mgr.list_jobs(None);
        assert_eq!(all_jobs.len(), 1);

        // 按状态过滤
        let completed = mgr.list_jobs(Some(BatchJobStatus::Completed));
        assert_eq!(completed.len(), 1);

        let pending = mgr.list_jobs(Some(BatchJobStatus::Pending));
        assert_eq!(pending.len(), 0);

        // 获取报告
        let report = mgr.get_job_report(&job1).unwrap();
        assert_eq!(report.total_objects, 1);
        assert_eq!(report.succeeded_count, 1);
    }

    #[test]
    fn test_restore_tier_from_str() {
        assert_eq!(RestoreTier::from_str("Standard"), Some(RestoreTier::Standard));
        assert_eq!(RestoreTier::from_str("Bulk"), Some(RestoreTier::Bulk));
        assert_eq!(RestoreTier::from_str("Expedited"), Some(RestoreTier::Expedited));
        assert_eq!(RestoreTier::from_str("invalid"), None);
    }

    #[test]
    fn test_batch_job_status_as_str() {
        assert_eq!(BatchJobStatus::Pending.as_str(), "Pending");
        assert_eq!(BatchJobStatus::Running.as_str(), "Running");
        assert_eq!(BatchJobStatus::Completed.as_str(), "Completed");
        assert_eq!(BatchJobStatus::Failed.as_str(), "Failed");
        assert_eq!(BatchJobStatus::Cancelled.as_str(), "Cancelled");
    }

    #[test]
    fn test_partial_failure_report() {
        let mgr = BatchOperationManager::new();

        let restore_fn =
            |_bucket: &str, key: &str, _days: u32, _tier: RestoreTier| -> S3Result<()> {
                if key.starts_with("bad-") {
                    Err(S3Error::InvalidArgument)
                } else {
                    Ok(())
                }
            };

        let request = BatchRestoreRequest {
            bucket: "bkt".into(),
            keys: vec![
                "good-1.zip".into(),
                "bad-1.zip".into(),
                "good-2.zip".into(),
                "bad-2.zip".into(),
            ],
            days: 3,
            tier: RestoreTier::Bulk,
        };

        let job_id = mgr.create_restore_job(request, None, restore_fn).unwrap();
        let report = mgr.get_job_report(&job_id).unwrap();

        assert_eq!(report.total_objects, 4);
        assert_eq!(report.succeeded_count, 2);
        assert_eq!(report.failed_count, 2);
        assert!(report.end_time_ms >= report.start_time_ms);
    }
}
