//! # mox-graph-meta（璇玑关系图 R1 Meta Service）
//!
//! **AIS L4 自研模块**：提供关系图元数据服务（Schema 管理、权限鉴权、分区路由），
//! 底层使用 `async-raft 0.11` 作为 Raft 协议 driver，状态机逻辑完全自研。
//!
//! ## 架构分层
//!
//! - **协议 driver（外部）**：`async-raft = 0.11`（Apache-2.0）
//! - **状态机（自研）**：`SchemaStore + AuthStore + PartitionStore`
//! - **持久化（外部）**：`rocksdb`（Apache-2.0）
//! - **对外 API**：`MetaServer` — 兼容 L5 `GraphMetaProvider` trait
//!
//! ## 模块
//!
//! - `meta_server`   — 对外 API + 3 节点 Raft 集群编排（MetaCluster）
//! - `raft_state_machine` — async-raft `RaftStorage` 实现 + `RaftLog` 枚举 + `MetaStateMachine`
//! - `schema_store`  — Space/Tag/EdgeType 定义与校验
//! - `auth_store`    — 用户/角色/Policy + 加盐 SHA-256 密码 + `authorize()` 授权
//! - `partition_store` — VID 哈希分片 + shard ↔ storage host 映射
//! - `error`         — 统一错误枚举 `MetaError`
//!

pub mod auth_store;
pub mod error;
pub mod meta_server;
pub mod partition_store;
pub mod raft_state_machine;
pub mod schema_store;

pub use auth_store::{AuthStore, Policy, Resource, Role, UserDef, UserId};
pub use error::{MetaError, MetaResult};
pub use meta_server::{
    build_raft_config, CreateEdgeTypeArgs, HostView, MetaCluster, MetaNodeConfig, MetaNodeRuntime,
    MetaServer, NodeRole,
};
pub use partition_store::{vid_hash_partition, PartitionStore, StorageHost};
pub use raft_state_machine::{MetaSnapshot, MetaStateMachine, RaftLog};
pub use schema_store::{EdgeDef, FieldDef, FieldType, IndexKind, SchemaStore, SpaceDef, TagDef};
