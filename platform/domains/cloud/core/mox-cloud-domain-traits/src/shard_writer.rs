//! L4 分片写入抽象 —— 多副本分片写入与法定人数（quorum）确认。
//!
//! [`ShardWriter`] 定义了将一批分片数据并行写入多个 [`ShardLocation`] 的契约，
//! 支持写入法定人数配置、背压拒绝与 straggler 容忍。

use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ShardLocation 定义在 shard_reader 模块中，ShardWriter 的签名需要引用它。
use crate::shard_reader::ShardLocation;

// ---------------------------------------------------------------------------
// 写入法定人数与结果
// ---------------------------------------------------------------------------

/// 写入法定人数配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteQuorum {
    pub min_acks: usize,
    pub stall_timeout_ms: u64,
    pub absolute_cap: usize,
    pub allow_stragglers: bool,
}

impl Default for WriteQuorum {
    fn default() -> Self {
        Self { min_acks: 1, stall_timeout_ms: 5000, absolute_cap: 16, allow_stragglers: true }
    }
}

/// 写入结果，记录成功/失败/拖尾副本的索引。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteResult {
    pub succeeded: Vec<usize>,
    pub failed: Vec<usize>,
    pub stragglers: Vec<usize>,
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// 并发提示
// ---------------------------------------------------------------------------

/// 写入并发度提示。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConcurrencyHint {
    Sequential,
    Parallel(usize),
    Unlimited,
}

impl std::fmt::Display for ConcurrencyHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConcurrencyHint::Sequential => write!(f, "sequential"),
            ConcurrencyHint::Parallel(n) => write!(f, "parallel({})", n),
            ConcurrencyHint::Unlimited => write!(f, "unlimited"),
        }
    }
}

// ---------------------------------------------------------------------------
// 错误类型
// ---------------------------------------------------------------------------

/// 分片写入错误。
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("quorum not reached: {succeeded} succeeded, {required} required")]
    QuorumNotReached { succeeded: usize, required: usize },
    #[error("all replicas failed: {0:?}")]
    AllReplicasFailed(Vec<String>),
    #[error("write timeout after {0:?}")]
    Timeout(Duration),
    #[error("backpressure rejected")]
    BackpressureRejected,
    #[error("backend unavailable: {0}")]
    BackendUnavailable(String),
}

// ---------------------------------------------------------------------------
// 核心 trait
// ---------------------------------------------------------------------------

/// L4 分片写入抽象。
///
/// trait 是 object-safe 的，所有方法均为 `&self`。
#[async_trait]
pub trait ShardWriter: Send + Sync {
    /// 将一批分片并行写入多个位置，按法定人数配置确认成功。
    async fn write_shards(
        &self,
        shards: &[Bytes],
        locations: &[ShardLocation],
        quorum: &WriteQuorum,
    ) -> Result<WriteResult, WriteError>;

    /// 写入器推荐的并发度提示。
    fn concurrency_hint(&self) -> ConcurrencyHint;

    /// 写入器的 endpoint 标识。
    fn endpoint(&self) -> &str;
}

// ShardLocation 定义在 shard_reader 模块中，此处通过 use 引入避免重复定义。

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shard_reader::{ShardLocation, StorageTier};

    struct DummyWriter {
        ep: String,
    }

    #[async_trait]
    impl ShardWriter for DummyWriter {
        async fn write_shards(
            &self,
            shards: &[Bytes],
            locations: &[ShardLocation],
            quorum: &WriteQuorum,
        ) -> Result<WriteResult, WriteError> {
            let count = shards.len().min(locations.len());
            if count < quorum.min_acks {
                return Err(WriteError::QuorumNotReached {
                    succeeded: count,
                    required: quorum.min_acks,
                });
            }
            let succeeded: Vec<usize> = (0..count).collect();
            Ok(WriteResult { succeeded, failed: vec![], stragglers: vec![], duration_ms: 10 })
        }

        fn concurrency_hint(&self) -> ConcurrencyHint {
            ConcurrencyHint::Parallel(4)
        }

        fn endpoint(&self) -> &str {
            &self.ep
        }
    }

    #[test]
    fn test_types_construct() {
        let quorum = WriteQuorum::default();
        assert_eq!(quorum.min_acks, 1);
        assert_eq!(quorum.stall_timeout_ms, 5000);
        assert_eq!(quorum.absolute_cap, 16);
        assert!(quorum.allow_stragglers);

        let result = WriteResult {
            succeeded: vec![0, 1],
            failed: vec![2],
            stragglers: vec![3],
            duration_ms: 100,
        };
        assert_eq!(result.succeeded.len(), 2);
        assert_eq!(result.failed, vec![2]);
        assert_eq!(result.duration_ms, 100);

        assert_eq!(ConcurrencyHint::Sequential.to_string(), "sequential");
        assert_eq!(ConcurrencyHint::Parallel(8).to_string(), "parallel(8)");
        assert_eq!(ConcurrencyHint::Unlimited.to_string(), "unlimited");
    }

    #[tokio::test]
    async fn test_trait_object_safe() {
        let writer: Box<dyn ShardWriter> = Box::new(DummyWriter { ep: "writer-1".into() });

        assert_eq!(writer.endpoint(), "writer-1");
        assert_eq!(writer.concurrency_hint(), ConcurrencyHint::Parallel(4));

        let shards = vec![Bytes::from("data-0"), Bytes::from("data-1")];
        let locations = vec![
            ShardLocation {
                node_id: "n1".into(),
                endpoint: "e1".into(),
                shard_index: 0,
                tier: StorageTier::Local,
            },
            ShardLocation {
                node_id: "n2".into(),
                endpoint: "e2".into(),
                shard_index: 1,
                tier: StorageTier::SameRack,
            },
        ];
        let quorum = WriteQuorum::default();
        let result = writer.write_shards(&shards, &locations, &quorum).await.unwrap();
        assert_eq!(result.succeeded, vec![0, 1]);
        assert!(result.failed.is_empty());

        // quorum 不满足时应返回错误
        let strict_quorum = WriteQuorum { min_acks: 5, ..Default::default() };
        let err = writer.write_shards(&shards, &locations, &strict_quorum).await.unwrap_err();
        assert!(matches!(err, WriteError::QuorumNotReached { .. }));
    }
}
