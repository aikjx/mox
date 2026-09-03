// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 读仲裁模块 — HedgedReader + locality 优先
//!
//! 读取时向多个副本 / 节点发读请求，取最快返回的结果（hedged read），
//! 同时优先选择 locality 近的节点（本地 / 同节点 / 远端排序）。
//!
//! 算法参考：RustFS ecstore (Apache 2.0) 的 ParallelReader / hedge /
//! locality / lockstep 读路径；本模块为独立重写实现，未直接复制源码。
//!
//! HedgedReader 同时实现了高层 `ReaderCapability` trait（见 reader_capability
//! 模块），统一 hedged 与普通读路径的能力探测和组合式管线。

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::{FuturesUnordered, StreamExt};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

use crate::reader_capability::{ReadCapabilityError, ReaderCapability};

// ---------------------------------------------------------------------------
// 错误类型
// ---------------------------------------------------------------------------

/// 读仲裁错误
#[derive(Debug, Error)]
pub enum ReadError {
    /// 单个 shard 读取失败
    #[error("shard {0} read failed: {1}")]
    ShardReadFailed(usize, String),

    /// 所有 reader 均失败
    #[error("all readers failed for shard {0}")]
    AllReadersFailed(usize),

    /// 读取超时
    #[error("read timed out")]
    Timeout,
}

// ---------------------------------------------------------------------------
// Locality 排序
// ---------------------------------------------------------------------------

/// shard 读取成本（locality 排序依据）
///
/// 按从快到慢排序：Local < SameNode < Remote < Unknown。
/// 枚举变体的声明顺序即排序顺序（派生 Ord）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShardReadCost {
    /// 本地磁盘，最快
    Local,
    /// 同节点不同盘
    SameNode,
    /// 远端节点
    Remote,
    /// 未知，排最后
    Unknown,
}

impl Default for ShardReadCost {
    fn default() -> Self {
        ShardReadCost::Unknown
    }
}

// ---------------------------------------------------------------------------
// ShardReader trait
// ---------------------------------------------------------------------------

/// 单个 shard 读取器抽象
///
/// 每个实现对应一个存储节点 / 磁盘端点，负责从指定 shard 读取数据。
#[async_trait]
pub trait ShardReader: Send + Sync {
    /// 读取指定 shard 的数据
    async fn read_shard(&self, shard_index: usize) -> Result<Bytes, ReadError>;

    /// 该 reader 的 locality 成本
    fn read_cost(&self) -> ShardReadCost;

    /// 端点标识（用于日志）
    fn endpoint(&self) -> &str;
}

// ---------------------------------------------------------------------------
// 类型别名
// ---------------------------------------------------------------------------

/// boxed 后的读 future，用于 FuturesUnordered（不同 async block 类型统一）
type BoxedReadFuture = Pin<Box<dyn Future<Output = Result<Bytes, ReadError>> + Send>>;

// ---------------------------------------------------------------------------
// HedgedReader
// ---------------------------------------------------------------------------

/// 对冲读取器
///
/// 按 locality 排序后，先向最优的 reader 发读请求；如果 `hedge_delay` 内
/// 未返回成功结果，则向次优 reader 也发请求（hedge），取第一个成功返回的
/// 结果，取消其他请求。
///
/// 同时实现高层 `ReaderCapability` trait，`supports_hedged_read()` 返回 true。
pub struct HedgedReader {
    readers: Vec<Arc<dyn ShardReader>>,
    /// 对冲延迟：首个请求发出后，等待多久再发起备用请求
    hedge_delay: Duration,
    /// 缓存的端点标签（所有 reader endpoint 用逗号连接）
    endpoint_label: String,
}

impl HedgedReader {
    /// 创建新的 HedgedReader
    ///
    /// `hedge_delay` 建议取 `min(read_timeout, 100ms)`。
    pub fn new(readers: Vec<Arc<dyn ShardReader>>, hedge_delay: Duration) -> Self {
        let endpoint_label = readers
            .iter()
            .map(|r| r.endpoint().to_string())
            .collect::<Vec<_>>()
            .join(",");
        Self {
            readers,
            hedge_delay: hedge_delay.max(Duration::from_micros(1)),
            endpoint_label,
        }
    }

    /// 获取 reader 数量
    pub fn reader_count(&self) -> usize {
        self.readers.len()
    }

    /// 获取对冲延迟
    pub fn hedge_delay(&self) -> Duration {
        self.hedge_delay
    }

    /// 获取所有 reader 中的最优（最小）locality 成本
    pub fn min_read_cost(&self) -> ShardReadCost {
        self.readers
            .iter()
            .map(|r| r.read_cost())
            .min()
            .unwrap_or(ShardReadCost::Unknown)
    }

    /// 对冲读取单个 shard
    ///
    /// 1. 按 `ShardReadCost` 排序 readers（Local < SameNode < Remote < Unknown）
    /// 2. 先向排序第一的 reader 发请求
    /// 3. 如果 `hedge_delay` 内未成功，向次优 reader 也发请求（不取消第一个）
    /// 4. 可以继续 hedge 到第三个、第四个
    /// 5. 第一个成功返回后，其他请求通过 drop future 取消（结构化并发）
    /// 6. 如果所有 reader 都失败，返回 `ReadError::AllReadersFailed`
    pub async fn read_hedged(&self, shard_index: usize) -> Result<Bytes, ReadError> {
        // 按 locality 成本排序（成本低的排前面）
        let mut sorted: Vec<&Arc<dyn ShardReader>> = self.readers.iter().collect();
        sorted.sort_by_key(|r| r.read_cost());

        if sorted.is_empty() {
            return Err(ReadError::AllReadersFailed(shard_index));
        }

        let mut active: FuturesUnordered<BoxedReadFuture> = FuturesUnordered::new();

        // 启动第一个（最优）reader
        {
            let reader = Arc::clone(sorted[0]);
            active.push(Box::pin(async move {
                reader.read_shard(shard_index).await
            }));
        }
        let mut next_idx: usize = 1;

        // 下一次 hedge 的截止时间
        let mut next_hedge_deadline = Instant::now() + self.hedge_delay;

        loop {
            // 安全网：所有 active future 已完成且均失败
            if active.is_empty() {
                if next_idx >= sorted.len() {
                    return Err(ReadError::AllReadersFailed(shard_index));
                }
                // 立即补下一个 reader（失败时不等待 hedge_delay）
                let reader = Arc::clone(sorted[next_idx]);
                active.push(Box::pin(async move {
                    reader.read_shard(shard_index).await
                }));
                next_idx += 1;
                next_hedge_deadline = Instant::now() + self.hedge_delay;
                continue;
            }

            // 计算到下一次 hedge 的剩余时间
            let now = Instant::now();
            let sleep_dur = if next_hedge_deadline > now {
                next_hedge_deadline - now
            } else {
                Duration::ZERO
            };

            tokio::select! {
                Some(result) = active.next() => {
                    match result {
                        Ok(data) => {
                            // 第一个成功结果：drop active 取消其余请求
                            return Ok(data);
                        }
                        Err(e) => {
                            tracing::debug!(
                                shard_index,
                                error = %e,
                                "hedged reader returned error, trying next"
                            );
                            // 当前 reader 失败：立即启动下一个（不等待 hedge_delay）
                            if next_idx < sorted.len() {
                                let reader = Arc::clone(sorted[next_idx]);
                                active.push(Box::pin(async move {
                                    reader.read_shard(shard_index).await
                                }));
                                next_idx += 1;
                                next_hedge_deadline = Instant::now() + self.hedge_delay;
                            }
                        }
                    }
                }
                _ = tokio::time::sleep(sleep_dur) => {
                    // hedge_delay 内无成功结果：发起下一个备用 reader
                    if next_idx < sorted.len() {
                        tracing::debug!(
                            shard_index,
                            next_reader_index = next_idx,
                            "hedge delay elapsed, adding backup reader"
                        );
                        let reader = Arc::clone(sorted[next_idx]);
                        active.push(Box::pin(async move {
                            reader.read_shard(shard_index).await
                        }));
                        next_idx += 1;
                        next_hedge_deadline = Instant::now() + self.hedge_delay;
                    }
                }
            }
        }
    }

    /// 并发读多个 shard，每个 shard 独立使用 hedged read
    ///
    /// 返回的结果按 shard index 排序。任一 shard 读取失败则整体返回错误。
    pub async fn read_multiple(
        &self,
        shard_indices: &[usize],
    ) -> Result<Vec<(usize, Bytes)>, ReadError> {
        if shard_indices.is_empty() {
            return Ok(Vec::new());
        }

        // Sequential hedged reads per shard.  Concurrent fan-out would require
        // 'static futures (FuturesUnordered cannot hold borrows of &self), so
        // we keep this path simple and correct; each shard still benefits from
        // the intra-shard hedging inside read_hedged.
        let mut results: Vec<(usize, Bytes)> = Vec::with_capacity(shard_indices.len());
        for &idx in shard_indices {
            let data = self.read_hedged(idx).await?;
            results.push((idx, data));
        }

        // 按 shard index 排序，保证输出确定性
        results.sort_by_key(|(idx, _)| *idx);
        Ok(results)
    }
}

// ---------------------------------------------------------------------------
// ReaderCapability trait 实现（高层抽象）
// ---------------------------------------------------------------------------

#[async_trait]
impl ReaderCapability for HedgedReader {
    async fn read_shard(&self, shard_index: usize) -> Result<Bytes, ReadCapabilityError> {
        self.read_hedged(shard_index)
            .await
            .map_err(|e| ReadCapabilityError::from_read_error(shard_index, e))
    }

    fn read_cost(&self) -> ShardReadCost {
        self.min_read_cost()
    }

    fn endpoint(&self) -> &str {
        &self.endpoint_label
    }

    fn supports_hedged_read(&self) -> bool {
        true
    }

    // supports_zero_copy 默认 false
    // read_timeout 默认 30s
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock ShardReader：可配置延迟、locality、成功率和返回数据
    struct MockShardReader {
        endpoint: String,
        cost: ShardReadCost,
        delay: Duration,
        should_fail: bool,
        payload: Bytes,
    }

    impl MockShardReader {
        fn new(
            endpoint: &str,
            cost: ShardReadCost,
            delay: Duration,
            should_fail: bool,
            payload: Bytes,
        ) -> Self {
            Self {
                endpoint: endpoint.to_string(),
                cost,
                delay,
                should_fail,
                payload,
            }
        }
    }

    #[async_trait]
    impl ShardReader for MockShardReader {
        async fn read_shard(&self, _shard_index: usize) -> Result<Bytes, ReadError> {
            if self.delay > Duration::ZERO {
                tokio::time::sleep(self.delay).await;
            }
            if self.should_fail {
                Err(ReadError::ShardReadFailed(
                    _shard_index,
                    format!("mock reader {} failure", self.endpoint),
                ))
            } else {
                Ok(self.payload.clone())
            }
        }

        fn read_cost(&self) -> ShardReadCost {
            self.cost
        }

        fn endpoint(&self) -> &str {
            &self.endpoint
        }
    }

    // ----- 测试 1：第一个 reader 立即返回，不触发 hedge -----

    #[tokio::test]
    async fn test_hedged_reader_first_returns() {
        let payload = Bytes::from_static(b"hello-from-local");
        let readers: Vec<Arc<dyn ShardReader>> = vec![
            Arc::new(MockShardReader::new(
                "local",
                ShardReadCost::Local,
                Duration::ZERO,
                false,
                payload.clone(),
            )),
            Arc::new(MockShardReader::new(
                "remote",
                ShardReadCost::Remote,
                Duration::ZERO,
                false,
                Bytes::from_static(b"should-not-be-used"),
            )),
        ];

        let hr = HedgedReader::new(readers, Duration::from_millis(100));
        let result = hr.read_hedged(0).await.unwrap();
        assert_eq!(result, payload);
    }

    // ----- 测试 2：第一个 reader 慢，hedge 到第二个 -----

    #[tokio::test]
    async fn test_hedged_reader_hedge_to_second() {
        let fast_payload = Bytes::from_static(b"fast-remote-result");
        let readers: Vec<Arc<dyn ShardReader>> = vec![
            // 第一个 reader 很慢（远超 hedge_delay）
            Arc::new(MockShardReader::new(
                "slow-local",
                ShardReadCost::Local,
                Duration::from_secs(10),
                false,
                Bytes::from_static(b"slow-result"),
            )),
            // 第二个 reader 快
            Arc::new(MockShardReader::new(
                "fast-remote",
                ShardReadCost::Remote,
                Duration::ZERO,
                false,
                fast_payload.clone(),
            )),
        ];

        let hr = HedgedReader::new(readers, Duration::from_millis(50));
        let result = hr.read_hedged(0).await.unwrap();
        assert_eq!(result, fast_payload);
    }

    // ----- 测试 3：ShardReadCost 排序 -----

    #[test]
    fn test_hedged_reader_locality_ordering() {
        assert!(ShardReadCost::Local < ShardReadCost::SameNode);
        assert!(ShardReadCost::SameNode < ShardReadCost::Remote);
        assert!(ShardReadCost::Remote < ShardReadCost::Unknown);

        // 验证排序：将乱序的 cost 排序后应为 Local, SameNode, Remote, Unknown
        let mut costs = vec![
            ShardReadCost::Unknown,
            ShardReadCost::Local,
            ShardReadCost::Remote,
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
    }

    // ----- 测试 4：所有 reader 失败 -----

    #[tokio::test]
    async fn test_hedged_reader_all_fail() {
        let readers: Vec<Arc<dyn ShardReader>> = vec![
            Arc::new(MockShardReader::new(
                "fail-1",
                ShardReadCost::Local,
                Duration::ZERO,
                true,
                Bytes::new(),
            )),
            Arc::new(MockShardReader::new(
                "fail-2",
                ShardReadCost::Remote,
                Duration::ZERO,
                true,
                Bytes::new(),
            )),
        ];

        let hr = HedgedReader::new(readers, Duration::from_millis(10));
        let result = hr.read_hedged(7).await;
        match result {
            Err(ReadError::AllReadersFailed(shard_idx)) => {
                assert_eq!(shard_idx, 7);
            }
            other => panic!("expected AllReadersFailed, got {other:?}"),
        }
    }

    // ----- 额外测试：read_multiple 并发读取 -----

    #[tokio::test]
    async fn test_hedged_reader_read_multiple() {
        let readers: Vec<Arc<dyn ShardReader>> = vec![Arc::new(MockShardReader::new(
            "local",
            ShardReadCost::Local,
            Duration::ZERO,
            false,
            Bytes::from_static(b"data"),
        ))];

        let hr = HedgedReader::new(readers, Duration::from_millis(10));
        let results = hr.read_multiple(&[0, 1, 2]).await.unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, 0);
        assert_eq!(results[1].0, 1);
        assert_eq!(results[2].0, 2);
        for (_, data) in &results {
            assert_eq!(data, &Bytes::from_static(b"data"));
        }
    }

    // ----- 额外测试：空 reader 列表 -----

    #[tokio::test]
    async fn test_hedged_reader_empty() {
        let hr = HedgedReader::new(vec![], Duration::from_millis(10));
        let result = hr.read_hedged(0).await;
        assert!(matches!(result, Err(ReadError::AllReadersFailed(0))));
    }

    // ----- 额外测试：第一个失败后立即切换到第二个 -----

    #[tokio::test]
    async fn test_hedged_reader_first_fails_immediate_switch() {
        let second_payload = Bytes::from_static(b"second-ok");
        let readers: Vec<Arc<dyn ShardReader>> = vec![
            // 第一个立即失败
            Arc::new(MockShardReader::new(
                "fail-first",
                ShardReadCost::Local,
                Duration::ZERO,
                true,
                Bytes::new(),
            )),
            // 第二个成功
            Arc::new(MockShardReader::new(
                "ok-second",
                ShardReadCost::Remote,
                Duration::ZERO,
                false,
                second_payload.clone(),
            )),
        ];

        let hr = HedgedReader::new(readers, Duration::from_millis(100));
        let result = hr.read_hedged(0).await.unwrap();
        assert_eq!(result, second_payload);
    }

    // ----- 额外测试：default ShardReadCost -----

    #[test]
    fn test_shard_read_cost_default() {
        assert_eq!(ShardReadCost::default(), ShardReadCost::Unknown);
    }

    // ----- 新增测试：HedgedReader 实现 ReaderCapability trait -----

    #[tokio::test]
    async fn test_hedged_reader_implements_capability() {
        let payload = Bytes::from_static(b"hedged-cap-data");
        let readers: Vec<Arc<dyn ShardReader>> = vec![
            Arc::new(MockShardReader::new(
                "node-a",
                ShardReadCost::Local,
                Duration::ZERO,
                false,
                payload.clone(),
            )),
            Arc::new(MockShardReader::new(
                "node-b",
                ShardReadCost::Remote,
                Duration::ZERO,
                false,
                Bytes::from_static(b"backup"),
            )),
        ];

        let hr = HedgedReader::new(readers, Duration::from_millis(50));

        // 通过 ReaderCapability trait 读取
        let result = hr.read_shard(0).await.unwrap();
        assert_eq!(result, payload);

        // 能力探测
        assert!(hr.supports_hedged_read());
        assert!(!hr.supports_zero_copy());
        assert_eq!(hr.read_timeout(), Duration::from_secs(30));

        // read_cost 返回最优（Local）
        assert_eq!(hr.read_cost(), ShardReadCost::Local);

        // endpoint 是所有 reader 的拼接
        assert_eq!(hr.endpoint(), "node-a,node-b");

        // reader_count
        assert_eq!(hr.reader_count(), 2);
    }

    // ----- 新增测试：HedgedReader 可作为 ReaderCapability 放入 ReaderPipeline -----

    #[tokio::test]
    async fn test_hedged_reader_in_pipeline() {
        use crate::reader_capability::ReaderPipeline;

        let hedged_payload = Bytes::from_static(b"from-hedged");
        let hedged_readers: Vec<Arc<dyn ShardReader>> = vec![Arc::new(MockShardReader::new(
            "hedged-node",
            ShardReadCost::Remote,
            Duration::ZERO,
            false,
            hedged_payload.clone(),
        ))];
        let hedged = Arc::new(HedgedReader::new(hedged_readers, Duration::from_millis(10)));

        let pipeline = ReaderPipeline::new().with_reader(hedged);
        let result = pipeline.read_shard(0).await.unwrap();
        assert_eq!(result, hedged_payload);

        // 管线能力聚合：hedged reader 支持 hedged_read
        assert!(pipeline.supports_hedged_read());
    }
}
