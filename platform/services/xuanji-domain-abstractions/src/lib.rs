//! Xuanji V5 L5 Domain Abstractions.
//!
//! **AIS Layer**: L5 — Pure Trait Definitions.
//!
//! This crate defines the 10 core domain provider traits (5 Cloud Drive + 5 Graph)
//! that the Xuanji V5 platform relies on. All traits use `#[async_trait]` to allow
//! both synchronous and asynchronous implementations across L4 adapters.
//!
//! # Traits
//!
//! ## Cloud Drive (5)
//! - [`ObjectStorageProvider`] — S3-compatible object storage (put/get/delete/list/multipart)
//! - [`MetaStorageProvider`] — POSIX-compatible metadata (mkdir/symlink/xattr/statfs)
//! - [`ChunkManagerProvider`] — Data chunk lifecycle (allocate/read/write/gc)
//! - [`IamProvider`] — Identity & Access (user/role/policy/STS)
//! - [`QuotaProvider`] — Quota enforcement (user & directory byte/object limits)
//!
//! ## Graph (5)
//! - [`GraphQueryProvider`] — Graph query (vertex/edge/nGQL/Cypher)
//! - [`GraphMetaProvider`] — Graph meta DDL (space/tag/edge type)
//! - [`GraphAlgoSingleProvider`] — 7-algo guardrail (PPR/CNM/betweenness/harmonic/degree/density/raw-BDE)
//! - [`PartitionRouterProvider`] — Partition routing (shard→storage / rebalance)
//! - [`CdcPublisherProvider`] — CDC publisher (vertex/edge event stream + subscriptions)
//!
//! Each module ships with a `MockXxxProvider` that uses `parking_lot::Mutex<BTreeMap>`
//! for in-memory state — zero I/O, fully deterministic, suitable for unit testing.

pub mod cdc_publisher;
pub mod chunk_manager;
pub mod graph_algo_single;
pub mod graph_meta;
pub mod graph_query;
pub mod iam;
pub mod meta_storage;
pub mod object_storage;
pub mod partition_router;
pub mod quota;

pub use cdc_publisher::{
    CdcEvent, CdcPublisherProvider, CdcSubscription, ConsumerLag, MockCdcPublisherProvider,
};
pub use chunk_manager::{ChunkManagerProvider, ChunkStats, MockChunkManagerProvider};
pub use graph_algo_single::{AlgoOutput, GraphAlgoSingleProvider, MockGraphAlgoSingleProvider};
pub use graph_meta::{
    EdgeTypeDef, GraphMetaProvider, HostInfo, MockGraphMetaProvider, SpaceInfo, TagDef,
};
pub use graph_query::{
    AlgoSingleResult, Edge, GraphQueryError, GraphQueryProvider, MockGraphQueryProvider,
    QueryResultSet, Subgraph, Vertex,
};
pub use iam::{IamProvider, MockIamProvider, PolicyStatement, RoleInfo, StsCredentials, UserInfo};
pub use meta_storage::{FileStat, MetaStorageProvider, MockMetaStorageProvider, StatFs};
pub use object_storage::{
    ListResult, MockObjectStorageProvider, ObjectHead, ObjectStorageProvider, PartETag,
};
pub use partition_router::{
    MockPartitionRouterProvider, PartitionRouterProvider, RebalanceMove, RebalancePlan, ShardInfo,
};
pub use quota::{DirectoryQuota, MockQuotaProvider, QuotaInfo, QuotaProvider, UserQuota};

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {}
}
