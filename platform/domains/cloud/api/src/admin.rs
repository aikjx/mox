// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! StorageAdmin —— 存储管理面契约（status / switch / migrate / verify / gc / stats）。
//!
//! - **契约部分**（trait + DTO，零额外依赖）恒可用；
//! - 具体实现移至 `mox-cloud-admin-sdk`，契约包不依赖存储后端。

use super::CloudApiResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ---------------- 管理面 DTO ----------------

/// 存储健康状态（status 输出）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStatus {
    /// 当前后端类型：fs | s3 | minio | oss
    pub backend: String,
    /// healthy | degraded | unhealthy
    pub health: String,
    /// 本地数据目录
    pub data_dir: String,
    /// 对象元数据数
    pub object_count: u64,
    /// 内容寻址唯一块数
    pub chunk_count: u64,
    /// 版本元数据数
    pub version_count: u64,
    /// KV 条目数
    pub kv_count: u64,
    /// 数据块物理占用（去重后字节）
    pub data_bytes: u64,
    /// 逻辑体积（去重前字节）
    pub logical_bytes: u64,
    /// 去重率（逻辑 / 物理，≥1）
    pub dedup_ratio: f64,
    /// 最近一次 GC 时间（epoch ms，0 = 未执行）
    pub last_gc_ms: u64,
    /// 累计错误数
    pub error_count: u64,
    /// 最近一次错误
    pub last_error: Option<String>,
}

/// 存储规模统计（stats 输出）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageStats {
    pub object_count: u64,
    pub chunk_count: u64,
    pub version_count: u64,
    pub kv_count: u64,
    pub physical_bytes: u64,
    pub logical_bytes: u64,
    pub dedup_ratio: f64,
    pub ref_total: u64,
}

/// 跨后端迁移报告（migrate 输出）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrateReport {
    pub source: String,
    pub target: String,
    pub objects_total: u64,
    pub objects_ok: u64,
    pub objects_failed: u64,
    pub bytes_migrated: u64,
    pub duration_ms: u64,
    pub errors: Vec<String>,
}

/// 数据完整性校验报告（verify 输出）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerifyReport {
    pub objects_checked: u64,
    pub objects_ok: u64,
    pub corrupted: u64,
    pub missing: u64,
    pub duration_ms: u64,
    pub errors: Vec<String>,
}

/// GC 报告（gc 输出；dry_run=true 仅预览）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GcReport {
    pub dry_run: bool,
    pub chunks_scanned: u64,
    pub soft_purged: u64,
    pub hard_deleted: u64,
    pub bytes_freed: u64,
    pub duration_ms: u64,
    pub warnings: Vec<String>,
}

/// 存储管理面 trait：运维对存储后端的全生命周期管控。
#[async_trait]
pub trait StorageAdmin: Send + Sync {
    /// 后端健康状态 + 规模快照
    async fn status(&self) -> CloudApiResult<StorageStatus>;
    /// 规模统计（含去重率）
    async fn stats(&self) -> CloudApiResult<StorageStats>;
    /// 数据完整性校验（对象 → chunk 存在性 + 尺寸）
    async fn verify(&self) -> CloudApiResult<VerifyReport>;
    /// 引用计数 GC（dry_run=true 仅预览不动数据）
    async fn gc(&self, dry_run: bool) -> CloudApiResult<GcReport>;
    /// 热切换后端（fs|s3|minio|oss），返回切换后状态
    async fn switch_backend(&self, target: &str) -> CloudApiResult<StorageStatus>;
    /// 跨后端迁移（source→target，描述符 `kind[@data_dir]`），返回迁移报告
    async fn migrate(&self, source: &str, target: &str) -> CloudApiResult<MigrateReport>;
}
