// Copyright (c) 2026 璇玑 RelGraph · 统一存储引擎 (Unified Storage Engine)
// Licensed under the MIT License.

//! 统一存储引擎核心
//!
//! 融合知识图谱存储与云盘对象存储，提供统一的存储抽象：
//! - 图数据（节点/边）
//! - 对象数据（文件/Blob）
//! - 键值数据（通用 KV）
//!
//! 底层可以接入不同的存储后端（RocksDB、对象存储、内存等）。

pub mod error;
pub mod types;
pub mod storage_trait;
pub mod memory_backend;
pub mod graph_store;
pub mod object_store;
pub mod kv_store;
pub mod cache;
pub mod unified_engine;

pub use error::{StorageError, StorageResult};
pub use types::{
    DataModel, GraphEdge, GraphNode, ObjectMeta, StorageBackend, StorageStats, Value,
};
pub use unified_engine::UnifiedStorageEngine;
