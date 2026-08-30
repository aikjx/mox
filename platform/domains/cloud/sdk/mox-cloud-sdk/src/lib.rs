// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Mox SDK Cloud — in-memory fake facade for S3-compatible storage,
//! STS, IAM, Quota, WORM/S3Lock, Lifecycle and DengBao HashChain.
//!
//! No network I/O is performed. All state lives inside `CloudClient`.

// ============================================================================
// 模块声明
// ============================================================================

pub mod prelude;

mod client;
mod error;
mod hashchain;
mod lifecycle;
mod security;
mod storage;
mod types;
mod utils;

// ============================================================================
// 公开 API 重导出（保持向后兼容）
// ============================================================================

pub use client::{Client, CloudClient, CLOUD_EXAMPLE_IDS};
pub use error::{CloudError, Result};
pub use types::{
    BucketInfo, HashBlock, IamPolicy, LifecycleRule, LifecycleStats, MultipartUpload,
    MultipartUploadInfo, ObjectInfo, PartEtag, QuotaConfig, StsToken, WormRetention,
};
pub use utils::crc64_ecma;
