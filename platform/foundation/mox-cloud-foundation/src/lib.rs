// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Mox V5 L5 Domain Abstractions — 10 core traits (5 Cloud Drive + 5 Graph).

pub mod cdc_publisher;
pub mod chunk_manager;
pub mod graph_algo_single;
pub mod graph_meta;
pub mod graph_query;
pub mod iam;
pub mod iam_standard_policies;
pub mod meta_storage;
pub mod object_storage;
pub mod partition_router;
pub mod quota;
pub mod sts_ttl900;

pub use cdc_publisher::{CdcEvent, CdcPublisherProvider, CdcSubscription, ConsumerLag, MockCdcPublisherProvider};
pub use chunk_manager::{ChunkManagerProvider, ChunkStats, MockChunkManagerProvider};
pub use graph_algo_single::{AlgoOutput, GraphAlgoSingleProvider, MockGraphAlgoSingleProvider};
pub use graph_meta::{EdgeTypeDef, GraphMetaProvider, HostInfo, MockGraphMetaProvider, SpaceInfo, TagDef};
pub use graph_query::{AlgoSingleResult, Edge, GraphQueryError, GraphQueryProvider, MockGraphQueryProvider, QueryResultSet, Subgraph, Vertex};
pub use iam::{IamProvider, MockIamProvider, PolicyStatement, RoleInfo, StsCredentials, UserInfo};
pub use iam_standard_policies::{evaluate_policies, standard_10_policies, EvalContext, STANDARD_10_SIDS};
pub use meta_storage::{FileStat, MetaStorageProvider, MockMetaStorageProvider, StatFs};
pub use object_storage::{ListResult, MockObjectStorageProvider, ObjectHead, ObjectStorageProvider, PartETag};
pub use partition_router::{MockPartitionRouterProvider, PartitionRouterProvider, RebalanceMove, RebalancePlan, ShardInfo};
pub use quota::{DirectoryQuota, MockQuotaProvider, QuotaInfo, QuotaProvider, UserQuota};
pub use sts_ttl900::{StsAssumeRoleResult, StsService, StsVerifyExt, STS_ALLOWED_TTL_SECS};

#[cfg(test)]
mod tests {
    use crate::iam_standard_policies as _isp;
    use crate::sts_ttl900 as _sts;
    #[allow(dead_code)]
    fn _touch_submodules_for_tests() {
        let _ = _isp::standard_10_policies();
        let _ = _sts::StsService::new(b"x");
    }
    #[test] fn it_compiles() {}
}