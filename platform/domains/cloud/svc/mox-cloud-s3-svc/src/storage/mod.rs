// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! S3 服务存储后端适配层。
//!
//! 通过 [`mox_cloud_domain_traits::StorageBackend`] trait 解耦 S3 协议层与底层
//! 数据面实现。默认使用 [`InMemoryStorageBackend`]（测试/内存模式），
//! 可通过 `with_storage_backend` 注入任意 trait 实现（如 RustFS ecstore）。
//!
//! ## 后端清单
//! - [`InMemoryStorageBackend`]：纯内存 HashMap，默认后端，测试零依赖
//! - [`RustFsEcstoreBackend`]：RustFS ecstore 接入点骨架（feature `rustfs_ecstore_backend`），
//!   实际 FFI/进程对接待后续阶段

pub mod in_memory;
pub mod reader_pipeline;

#[cfg(feature = "rustfs_ecstore_backend")]
pub mod rustfs_ecstore;

pub use in_memory::InMemoryStorageBackend;
pub use reader_pipeline::{S3ReaderPipeline, StorageBackendReader};

#[cfg(feature = "rustfs_ecstore_backend")]
pub use rustfs_ecstore::RustFsEcstoreBackend;
