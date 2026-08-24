//! # 璇玑关系图 R2 Storage Service（xuanji-graph-storage）
//!
//! ## 自研边界（白名单依赖）
//! - `rocksdb 0.25` (bundled) — Apache-2.0
//! - `async-raft 0.6` — Apache-2.0
//! - 其余：tokio / parking_lot / serde / sha2 / thiserror 等基础库
//!
//! **零引用**第三方商业或开源成品图数据库实现。
//!
//! ## 架构分层
//! - `storage_server`：对外 7 API（add/update/remove vertex/edge, neighbors, scan）
//! - `storage_api`：参数校验 + hot vertex LRU cache (100k)
//! - `cdc_source`：CDC 事件总线；订阅 / commit_offset / lag_ms 指标
//! - `partition_raft`：分片 Raft Group 外观 + RaftLog enum + rebalance
//! - `kv_engine`：RocksDB 5 列族封装（vid_meta / out_edges / in_edges / vertex_props / edge_props）
//! - `graph_codec`：byte 级 key/value encode-decode；VID hash shard
//! - `error`：统一 StorageError（ShardNotFound / VidNotFound / EdgeNotFound / ...）

pub mod cdc_source;
pub mod error;
pub mod graph_codec;
pub mod kv_engine;
pub mod partition_raft;
pub mod storage_api;
pub mod storage_server;

pub use cdc_source::{CdcEvent, CdcEventType, CdcSource};
pub use error::{StorageError, StorageResult};
pub use graph_codec::PropValue;
pub use kv_engine::KvEngine;
pub use partition_raft::{rebalance_16_to_32, NodeRole, RaftGroup, RaftLog, ShardRaft};
pub use storage_api::{Direction, Edge, EdgeAck, HotNeighborCache, LruCache, Neighbor, VertexAck};
pub use storage_server::{R2StorageServer, StorageAddr, StorageServer};
