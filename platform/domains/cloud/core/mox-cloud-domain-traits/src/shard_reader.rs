//! L4 分片读取抽象 —— 跨节点/跨区域的分片数据读取与对冲（hedged read）。
//!
//! [`ShardReader`] 定义了从指定 [`ShardLocation`] 读取分片数据的契约，
//! 支持对冲读取配置与读取代价评估，用于副本选择和延迟优化。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ---------------------------------------------------------------------------
// 位置与层级
// ---------------------------------------------------------------------------

/// 分片所在的存储层级（用于局部性排序）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageTier {
    Local,
    SameRack,
    SameRegion,
    Remote,
}

impl std::fmt::Display for StorageTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageTier::Local => write!(f, "local"),
            StorageTier::SameRack => write!(f, "same-rack"),
            StorageTier::SameRegion => write!(f, "same-region"),
            StorageTier::Remote => write!(f, "remote"),
        }
    }
}

/// 分片的物理位置描述。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardLocation {
    pub node_id: String,
    pub endpoint: String,
    pub shard_index: usize,
    pub tier: StorageTier,
}

// ---------------------------------------------------------------------------
// 对冲读取配置
// ---------------------------------------------------------------------------

/// 对冲读取（hedged read）配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HedgeConfig {
    pub hedge_delay_ms: u64,
    pub max_attempts: u32,
    pub locality_sort: bool,
}

impl Default for HedgeConfig {
    fn default() -> Self {
        Self {
            hedge_delay_ms: 500,
            max_attempts: 3,
            locality_sort: true,
        }
    }
}

// ---------------------------------------------------------------------------
// 读取代价
// ---------------------------------------------------------------------------

/// 分片读取代价分级，派生 Ord 用于副本排序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ShardReadCost {
    Local,
    SameNode,
    Remote,
    Unknown,
}

impl std::fmt::Display for ShardReadCost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShardReadCost::Local => write!(f, "local"),
            ShardReadCost::SameNode => write!(f, "same-node"),
            ShardReadCost::Remote => write!(f, "remote"),
            ShardReadCost::Unknown => write!(f, "unknown"),
        }
    }
}

// ---------------------------------------------------------------------------
// 错误类型
// ---------------------------------------------------------------------------

/// 分片读取错误。
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("all replicas failed: {0:?}")]
    AllReplicasFailed(Vec<String>),
    #[error("read timeout after {0:?}")]
    Timeout(Duration),
    #[error("shard corrupted at {location}: expected checksum {expected}, got {actual}")]
    ShardCorrupted {
        location: String,
        expected: String,
        actual: String,
    },
    #[error("backend unavailable: {0}")]
    BackendUnavailable(String),
}

// ---------------------------------------------------------------------------
// 核心 trait
// ---------------------------------------------------------------------------

/// L4 分片读取抽象。
///
/// trait 是 object-safe 的，所有方法均为 `&self`。
#[async_trait]
pub trait ShardReader: Send + Sync {
    /// 从指定位置读取分片数据，遵循对冲读取配置。
    async fn read_shard(
        &self,
        location: &ShardLocation,
        hedge: &HedgeConfig,
    ) -> Result<Vec<u8>, ReadError>;

    /// 评估从指定位置读取的代价（用于副本排序）。
    fn read_cost(&self, location: &ShardLocation) -> ShardReadCost;

    /// 读取器的 endpoint 标识。
    fn endpoint(&self) -> &str;

    /// 是否支持读取取消，默认 `false`。
    fn supports_cancellation(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyReader {
        ep: String,
    }

    #[async_trait]
    impl ShardReader for DummyReader {
        async fn read_shard(
            &self,
            location: &ShardLocation,
            _hedge: &HedgeConfig,
        ) -> Result<Vec<u8>, ReadError> {
            Ok(format!("shard-{}", location.shard_index).into_bytes())
        }

        fn read_cost(&self, location: &ShardLocation) -> ShardReadCost {
            match location.tier {
                StorageTier::Local => ShardReadCost::Local,
                _ => ShardReadCost::Remote,
            }
        }

        fn endpoint(&self) -> &str {
            &self.ep
        }
    }

    #[test]
    fn test_types_construct() {
        let loc = ShardLocation {
            node_id: "node-1".into(),
            endpoint: "10.0.0.1:8080".into(),
            shard_index: 3,
            tier: StorageTier::SameRegion,
        };
        assert_eq!(loc.shard_index, 3);
        assert_eq!(loc.tier, StorageTier::SameRegion);

        let hedge = HedgeConfig::default();
        assert_eq!(hedge.hedge_delay_ms, 500);
        assert_eq!(hedge.max_attempts, 3);
        assert!(hedge.locality_sort);

        // ShardReadCost 排序验证
        let mut costs = vec![
            ShardReadCost::Unknown,
            ShardReadCost::Remote,
            ShardReadCost::Local,
            ShardReadCost::SameNode,
        ];
        costs.sort();
        assert_eq!(
            costs,
            vec![
                ShardReadCost::Local,
                ShardReadCost::SameNode,
                ShardReadCost::Remote,
                ShardReadCost::Unknown,
            ]
        );

        assert_eq!(StorageTier::Remote.to_string(), "remote");
        assert_eq!(ShardReadCost::Local.to_string(), "local");
    }

    #[tokio::test]
    async fn test_trait_object_safe() {
        let reader: Box<dyn ShardReader> = Box::new(DummyReader {
            ep: "reader-1".into(),
        });

        assert_eq!(reader.endpoint(), "reader-1");
        assert!(!reader.supports_cancellation());

        let loc = ShardLocation {
            node_id: "n1".into(),
            endpoint: "e1".into(),
            shard_index: 7,
            tier: StorageTier::Local,
        };
        assert_eq!(reader.read_cost(&loc), ShardReadCost::Local);

        let hedge = HedgeConfig::default();
        let data = reader.read_shard(&loc, &hedge).await.unwrap();
        assert_eq!(data, b"shard-7");
    }
}
