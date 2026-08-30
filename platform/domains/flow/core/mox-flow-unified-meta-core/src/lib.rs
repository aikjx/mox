// Copyright (c) 2026 璇玑 RelGraph · 统一元数据层 (Unified Metadata Layer)
// Licensed under the MIT License.

//! 统一元数据层核心
//!
//! 统一知识图谱与云盘的元数据模型，提供：
//! - 统一实体/资源模型
//! - Schema 管理
//! - 元数据版本控制
//! - 分布式元数据共识（Raft）
//! - 元数据索引与查询

pub mod error;
pub mod types;
pub mod entity;
pub mod schema;
pub mod metadata_store;
pub mod raft_meta;
pub mod index;

pub use error::{MetaError, MetaResult};
pub use types::{EntityKind, EntityRef, MetadataEntry, MetadataMap, ResourceStatus, VersionInfo};
pub use entity::{Entity, EntityBuilder};
pub use schema::{Schema, SchemaBuilder, SchemaField, SchemaFieldType};
pub use metadata_store::MetadataStore;
pub use raft_meta::{RaftMetadataNode, RaftNodeRole};
