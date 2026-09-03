// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! ReaderCapability trait — 读能力抽象与组合式 reader 管线
//!
//! 用 trait 统一 hedged_reader 与普通读路径，支持能力探测（capability
//! probing）和组合式管线（ReaderPipeline）。底层 `ShardReader` trait 继续
//! 作为单端点抽象，`ReaderCapability` 是更高层的组合 reader 抽象。
//!
//! 算法参考：RustFS rio (Apache 2.0) 的 `ReaderCapabilities` /
//! `delegate_reader_capabilities` 宏模式；本模块为完全重写，采用单 trait
//! + 默认方法的能力探测设计，而非 RustFS 的多小 trait 组合方案。

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::{FuturesUnordered, StreamExt};
use std::{sync::Arc, time::Duration};
use thiserror::Error;

use crate::hedged_reader::{ReadError, ShardReadCost, ShardReader};

// ---------------------------------------------------------------------------
// 错误类型
// ---------------------------------------------------------------------------

/// ReaderCapability 读路径错误
#[derive(Debug, Error)]
pub enum ReadCapabilityError {
    /// 单个 shard 读取失败
    #[error("shard {0} read failed: {1}")]
    ReadFailed(usize, String),

    /// 所有 reader 均失败
    #[error("all readers failed for shard {0}")]
    AllFailed(usize),

    /// 读取超时
    #[error("read timed out after {0:?}")]
    Timeout(Duration),

    /// 能力不支持
    #[error("capability not supported: {0}")]
    NotSupported(String),
}

impl ReadCapabilityError {
    /// 从底层 ShardReader 的 ReadError 转换（带 shard_index 上下文）
    pub fn from_read_error(_shard_index: usize, e: ReadError) -> Self {
        match e {
            ReadError::ShardReadFailed(idx, msg) => ReadCapabilityError::ReadFailed(idx, msg),
            ReadError::AllReadersFailed(idx) => ReadCapabilityError::AllFailed(idx),
            ReadError::Timeout => ReadCapabilityError::Timeout(Duration::from_secs(30)),
        }
    }
}

// ---------------------------------------------------------------------------
// ReaderCapability trait
// ---------------------------------------------------------------------------

/// 读能力抽象：统一 hedged 与普通读路径的高层 trait
///
/// 每个实现代表一个可读端点或一组端点的组合，提供：
/// - `read_shard`：异步读取指定 shard
/// - `read_cost`：locality 成本（用于排序）
/// - `endpoint`：端点标识（日志/调试）
/// - 能力探测：`supports_hedged_read` / `supports_zero_copy` / `read_timeout`
///
/// 算法参考：RustFS rio `ReaderCapabilities` (Apache 2.0)，本设计为单 trait
/// + 默认方法的重写实现。
#[async_trait]
pub trait ReaderCapability: Send + Sync {
    /// 读取指定 shard，返回数据
    async fn read_shard(&self, shard_index: usize) -> Result<Bytes, ReadCapabilityError>;

    /// 读成本（用于 locality 排序，Local < SameNode < Remote < Unknown）
    fn read_cost(&self) -> ShardReadCost;

    /// 端点标识（用于日志和调试）
    fn endpoint(&self) -> &str;

    /// 是否支持 hedged read（默认 false，HedgedReader 返回 true）
    fn supports_hedged_read(&self) -> bool {
        false
    }

    /// 是否支持零拷贝（默认 false）
    fn supports_zero_copy(&self) -> bool {
        false
    }

    /// 读取超时（默认 30s）
    fn read_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
}

// ---------------------------------------------------------------------------
// SimpleReader — 普通直接读实现
// ---------------------------------------------------------------------------

/// 简单直接读 reader：包装单个底层 `ShardReader`，不需要 hedged 的场景
///
/// 这是 `ReaderCapability` 的最小实现，直接委托给底层 `ShardReader`。
pub struct SimpleReader {
    reader: Arc<dyn ShardReader>,
}

impl SimpleReader {
    /// 创建新的 SimpleReader
    pub fn new(reader: Arc<dyn ShardReader>) -> Self {
        Self { reader }
    }

    /// 获取底层 ShardReader 引用
    pub fn inner(&self) -> &Arc<dyn ShardReader> {
        &self.reader
    }
}

#[async_trait]
impl ReaderCapability for SimpleReader {
    async fn read_shard(&self, shard_index: usize) -> Result<Bytes, ReadCapabilityError> {
        self.reader
            .read_shard(shard_index)
            .await
            .map_err(|e| ReadCapabilityError::from_read_error(shard_index, e))
    }

    fn read_cost(&self) -> ShardReadCost {
        self.reader.read_cost()
    }

    fn endpoint(&self) -> &str {
        self.reader.endpoint()
    }

    // supports_hedged_read 默认 false
    // supports_zero_copy 默认 false
    // read_timeout 默认 30s
}

// ---------------------------------------------------------------------------
// ReaderPipeline — 组合式 reader 管线
// ---------------------------------------------------------------------------

/// 组合式 reader 管线：持有多个 `ReaderCapability`，支持并发读和顺序读
///
/// 按 locality 成本排序后，可选择：
/// - `read_first_success`：并发读所有 reader，取最快成功返回
/// - `read_sequential`：按顺序读，第一个成功即返回
///
/// `ReaderPipeline` 自身也实现 `ReaderCapability`（`read_shard` 委托给
/// `read_first_success`），因此可以嵌套组合。
pub struct ReaderPipeline {
    readers: Vec<Arc<dyn ReaderCapability>>,
    /// 缓存的端点标签（所有 reader endpoint 用逗号连接）
    endpoint_label: String,
}

impl ReaderPipeline {
    /// 创建空管线
    pub fn new() -> Self {
        Self { readers: Vec::new(), endpoint_label: String::new() }
    }

    /// Builder：添加一个 reader
    pub fn with_reader(mut self, reader: Arc<dyn ReaderCapability>) -> Self {
        if !self.endpoint_label.is_empty() {
            self.endpoint_label.push(',');
        }
        self.endpoint_label.push_str(reader.endpoint());
        self.readers.push(reader);
        self
    }

    /// 构建为 `Arc<dyn ReaderCapability>`（可嵌套组合）
    pub fn build(self) -> Arc<dyn ReaderCapability> {
        Arc::new(self)
    }

    /// 获取 reader 数量
    pub fn reader_count(&self) -> usize {
        self.readers.len()
    }

    /// 按 locality 排序后的 reader 引用列表（成本低的在前）
    fn sorted_readers(&self) -> Vec<&Arc<dyn ReaderCapability>> {
        let mut sorted: Vec<&Arc<dyn ReaderCapability>> = self.readers.iter().collect();
        sorted.sort_by_key(|r| r.read_cost());
        sorted
    }

    /// 并发读所有 reader，取第一个成功返回的结果
    ///
    /// 使用 `FuturesUnordered` 并发发起所有读请求，第一个 `Ok` 立即返回；
    /// 其余请求通过 drop future 取消（结构化并发）。全部失败返回
    /// `ReadCapabilityError::AllFailed`。
    pub async fn read_first_success(
        &self,
        shard_index: usize,
    ) -> Result<Bytes, ReadCapabilityError> {
        let sorted = self.sorted_readers();
        if sorted.is_empty() {
            return Err(ReadCapabilityError::AllFailed(shard_index));
        }

        let mut futures: FuturesUnordered<_> = FuturesUnordered::new();
        for reader in &sorted {
            let r = Arc::clone(*reader);
            let idx = shard_index;
            futures.push(async move { r.read_shard(idx).await });
        }

        let mut last_error: Option<ReadCapabilityError> = None;
        while let Some(result) = futures.next().await {
            match result {
                Ok(data) => return Ok(data),
                Err(e) => {
                    last_error = Some(e);
                },
            }
        }

        // 所有 reader 均失败
        Err(last_error.unwrap_or(ReadCapabilityError::AllFailed(shard_index)))
    }

    /// 按顺序读，第一个成功即返回
    ///
    /// 按 locality 排序后逐个尝试，第一个 `Ok` 立即返回；全部失败返回
    /// `ReadCapabilityError::AllFailed`。
    pub async fn read_sequential(&self, shard_index: usize) -> Result<Bytes, ReadCapabilityError> {
        let sorted = self.sorted_readers();
        if sorted.is_empty() {
            return Err(ReadCapabilityError::AllFailed(shard_index));
        }

        let mut last_error: Option<ReadCapabilityError> = None;
        for reader in &sorted {
            match reader.read_shard(shard_index).await {
                Ok(data) => return Ok(data),
                Err(e) => {
                    last_error = Some(e);
                },
            }
        }

        Err(last_error.unwrap_or(ReadCapabilityError::AllFailed(shard_index)))
    }
}

impl Default for ReaderPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReaderCapability for ReaderPipeline {
    async fn read_shard(&self, shard_index: usize) -> Result<Bytes, ReadCapabilityError> {
        self.read_first_success(shard_index).await
    }

    fn read_cost(&self) -> ShardReadCost {
        // 返回所有 reader 中的最优（最小）成本
        self.readers
            .iter()
            .map(|r| r.read_cost())
            .min()
            .unwrap_or(ShardReadCost::Unknown)
    }

    fn endpoint(&self) -> &str {
        &self.endpoint_label
    }

    fn supports_hedged_read(&self) -> bool {
        self.readers.iter().any(|r| r.supports_hedged_read())
    }

    fn supports_zero_copy(&self) -> bool {
        self.readers.iter().any(|r| r.supports_zero_copy())
    }

    fn read_timeout(&self) -> Duration {
        // 返回所有 reader 中的最小超时（最严格的约束）
        self.readers
            .iter()
            .map(|r| r.read_timeout())
            .min()
            .unwrap_or_else(|| Duration::from_secs(30))
    }
}

// ---------------------------------------------------------------------------
// 能力探测
// ---------------------------------------------------------------------------

/// 一组 reader 的能力摘要
#[derive(Debug, Clone)]
pub struct ReaderCapabilitiesSummary {
    /// reader 总数
    pub total_readers: usize,
    /// 支持 hedged read 的 reader 数
    pub hedged_enabled_count: usize,
    /// 支持零拷贝的 reader 数
    pub zero_copy_count: usize,
    /// 最优（最小）读成本
    pub min_read_cost: ShardReadCost,
    /// 最大超时（最宽松的约束）
    pub max_timeout: Duration,
}

/// 探测一组 reader 的能力摘要
///
/// 遍历所有 reader，统计能力分布，用于调度决策和日志观测。
pub fn probe_capabilities(readers: &[Arc<dyn ReaderCapability>]) -> ReaderCapabilitiesSummary {
    let total = readers.len();
    let mut hedged_count = 0usize;
    let mut zero_copy_count = 0usize;
    let mut min_cost = ShardReadCost::Unknown;
    let mut max_timeout = Duration::ZERO;

    for r in readers {
        if r.supports_hedged_read() {
            hedged_count += 1;
        }
        if r.supports_zero_copy() {
            zero_copy_count += 1;
        }
        let cost = r.read_cost();
        if cost < min_cost {
            min_cost = cost;
        }
        let timeout = r.read_timeout();
        if timeout > max_timeout {
            max_timeout = timeout;
        }
    }

    ReaderCapabilitiesSummary {
        total_readers: total,
        hedged_enabled_count: hedged_count,
        zero_copy_count,
        min_read_cost: min_cost,
        max_timeout,
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Mock ShardReader：可配置延迟、成功率和返回数据
    struct MockShardReader {
        endpoint: String,
        cost: ShardReadCost,
        delay: Duration,
        should_fail: bool,
        payload: Bytes,
        call_count: AtomicUsize,
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
                call_count: AtomicUsize::new(0),
            }
        }

        #[allow(dead_code)]
        fn call_count(&self) -> usize {
            self.call_count.load(Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl ShardReader for MockShardReader {
        async fn read_shard(&self, _shard_index: usize) -> Result<Bytes, ReadError> {
            self.call_count.fetch_add(1, Ordering::Relaxed);
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

    /// Mock ReaderCapability：直接实现 trait，用于测试管线
    struct MockCapabilityReader {
        endpoint: String,
        cost: ShardReadCost,
        delay: Duration,
        should_fail: bool,
        payload: Bytes,
        hedged: bool,
        zero_copy: bool,
        timeout: Duration,
    }

    #[allow(clippy::too_many_arguments)]
    impl MockCapabilityReader {
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
                hedged: false,
                zero_copy: false,
                timeout: Duration::from_secs(30),
            }
        }

        fn with_hedged(mut self, v: bool) -> Self {
            self.hedged = v;
            self
        }

        fn with_zero_copy(mut self, v: bool) -> Self {
            self.zero_copy = v;
            self
        }

        fn with_timeout(mut self, t: Duration) -> Self {
            self.timeout = t;
            self
        }
    }

    #[async_trait]
    impl ReaderCapability for MockCapabilityReader {
        async fn read_shard(&self, _shard_index: usize) -> Result<Bytes, ReadCapabilityError> {
            if self.delay > Duration::ZERO {
                tokio::time::sleep(self.delay).await;
            }
            if self.should_fail {
                Err(ReadCapabilityError::ReadFailed(
                    _shard_index,
                    format!("mock capability {} failure", self.endpoint),
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

        fn supports_hedged_read(&self) -> bool {
            self.hedged
        }

        fn supports_zero_copy(&self) -> bool {
            self.zero_copy
        }

        fn read_timeout(&self) -> Duration {
            self.timeout
        }
    }

    // ----- 测试 1：SimpleReader 实现 ReaderCapability -----

    #[tokio::test]
    async fn test_simple_reader_implements_capability() {
        let payload = Bytes::from_static(b"simple-data");
        let mock = Arc::new(MockShardReader::new(
            "node-1",
            ShardReadCost::Local,
            Duration::ZERO,
            false,
            payload.clone(),
        ));
        let reader = SimpleReader::new(mock);

        // 读成功
        let result = reader.read_shard(0).await.unwrap();
        assert_eq!(result, payload);

        // read_cost / endpoint 正确委托
        assert_eq!(reader.read_cost(), ShardReadCost::Local);
        assert_eq!(reader.endpoint(), "node-1");

        // 默认能力
        assert!(!reader.supports_hedged_read());
        assert!(!reader.supports_zero_copy());
        assert_eq!(reader.read_timeout(), Duration::from_secs(30));
    }

    // ----- 测试 2：SimpleReader 错误转换 -----

    #[tokio::test]
    async fn test_simple_reader_error_conversion() {
        let mock = Arc::new(MockShardReader::new(
            "fail-node",
            ShardReadCost::Remote,
            Duration::ZERO,
            true,
            Bytes::new(),
        ));
        let reader = SimpleReader::new(mock);

        let result = reader.read_shard(5).await;
        match result {
            Err(ReadCapabilityError::ReadFailed(idx, msg)) => {
                assert_eq!(idx, 5);
                assert!(msg.contains("mock reader fail-node failure"));
            },
            other => panic!("expected ReadFailed, got {other:?}"),
        }
    }

    // ----- 测试 3：ReaderPipeline 并发读（first_success）-----

    #[tokio::test]
    async fn test_reader_pipeline_first_success() {
        let fast_payload = Bytes::from_static(b"fast-result");
        let pipeline = ReaderPipeline::new()
            .with_reader(Arc::new(MockCapabilityReader::new(
                "slow-local",
                ShardReadCost::Local,
                Duration::from_secs(10), // 很慢
                false,
                Bytes::from_static(b"slow"),
            )))
            .with_reader(Arc::new(MockCapabilityReader::new(
                "fast-remote",
                ShardReadCost::Remote,
                Duration::ZERO, // 立即返回
                false,
                fast_payload.clone(),
            )));

        assert_eq!(pipeline.reader_count(), 2);

        // 并发读应取最快成功的（fast-remote）
        let result = pipeline.read_first_success(0).await.unwrap();
        assert_eq!(result, fast_payload);
    }

    // ----- 测试 4：ReaderPipeline 顺序读（sequential）-----

    #[tokio::test]
    async fn test_reader_pipeline_sequential() {
        let second_payload = Bytes::from_static(b"second-ok");
        let pipeline = ReaderPipeline::new()
            .with_reader(Arc::new(MockCapabilityReader::new(
                "fail-first",
                ShardReadCost::Local, // locality 最优，但会失败
                Duration::ZERO,
                true,
                Bytes::new(),
            )))
            .with_reader(Arc::new(MockCapabilityReader::new(
                "ok-second",
                ShardReadCost::Remote,
                Duration::ZERO,
                false,
                second_payload.clone(),
            )));

        // 顺序读：第一个失败后尝试第二个，成功返回
        let result = pipeline.read_sequential(3).await.unwrap();
        assert_eq!(result, second_payload);
    }

    // ----- 测试 5：ReaderPipeline 全部失败 -----

    #[tokio::test]
    async fn test_reader_pipeline_all_fail() {
        let pipeline = ReaderPipeline::new()
            .with_reader(Arc::new(MockCapabilityReader::new(
                "fail-1",
                ShardReadCost::Local,
                Duration::ZERO,
                true,
                Bytes::new(),
            )))
            .with_reader(Arc::new(MockCapabilityReader::new(
                "fail-2",
                ShardReadCost::Remote,
                Duration::ZERO,
                true,
                Bytes::new(),
            )));

        let result = pipeline.read_first_success(7).await;
        assert!(matches!(result, Err(ReadCapabilityError::ReadFailed(7, _))));

        let result2 = pipeline.read_sequential(7).await;
        assert!(matches!(result2, Err(ReadCapabilityError::ReadFailed(7, _))));
    }

    // ----- 测试 6：ReaderPipeline 空管线 -----

    #[tokio::test]
    async fn test_reader_pipeline_empty() {
        let pipeline = ReaderPipeline::new();
        assert_eq!(pipeline.reader_count(), 0);

        let result = pipeline.read_first_success(0).await;
        assert!(matches!(result, Err(ReadCapabilityError::AllFailed(0))));

        let result2 = pipeline.read_sequential(0).await;
        assert!(matches!(result2, Err(ReadCapabilityError::AllFailed(0))));
    }

    // ----- 测试 7：ReaderPipeline 实现 ReaderCapability（可嵌套）-----

    #[tokio::test]
    async fn test_reader_pipeline_implements_capability() {
        let payload = Bytes::from_static(b"nested-data");
        let inner = ReaderPipeline::new()
            .with_reader(Arc::new(MockCapabilityReader::new(
                "inner-1",
                ShardReadCost::Local,
                Duration::ZERO,
                false,
                payload.clone(),
            )))
            .build();

        // 外层管线嵌套内层
        let outer = ReaderPipeline::new().with_reader(inner);
        assert_eq!(outer.reader_count(), 1);

        // 通过 trait 方法读取
        let result = outer.read_shard(0).await.unwrap();
        assert_eq!(result, payload);

        // 能力聚合
        assert_eq!(outer.read_cost(), ShardReadCost::Local);
        assert!(outer.endpoint().contains("inner-1"));
    }

    // ----- 测试 8：probe_capabilities 能力探测 -----

    #[test]
    fn test_probe_capabilities() {
        let readers: Vec<Arc<dyn ReaderCapability>> = vec![
            Arc::new(
                MockCapabilityReader::new(
                    "r1",
                    ShardReadCost::Local,
                    Duration::ZERO,
                    false,
                    Bytes::new(),
                )
                .with_hedged(true)
                .with_zero_copy(true)
                .with_timeout(Duration::from_secs(10)),
            ),
            Arc::new(
                MockCapabilityReader::new(
                    "r2",
                    ShardReadCost::Remote,
                    Duration::ZERO,
                    false,
                    Bytes::new(),
                )
                .with_timeout(Duration::from_secs(60)),
            ),
            Arc::new(
                MockCapabilityReader::new(
                    "r3",
                    ShardReadCost::SameNode,
                    Duration::ZERO,
                    false,
                    Bytes::new(),
                )
                .with_hedged(true),
            ),
        ];

        let summary = probe_capabilities(&readers);

        assert_eq!(summary.total_readers, 3);
        assert_eq!(summary.hedged_enabled_count, 2); // r1, r3
        assert_eq!(summary.zero_copy_count, 1); // r1
        assert_eq!(summary.min_read_cost, ShardReadCost::Local); // r1
        assert_eq!(summary.max_timeout, Duration::from_secs(60)); // r2
    }

    // ----- 测试 9：probe_capabilities 空输入 -----

    #[test]
    fn test_probe_capabilities_empty() {
        let readers: Vec<Arc<dyn ReaderCapability>> = vec![];
        let summary = probe_capabilities(&readers);

        assert_eq!(summary.total_readers, 0);
        assert_eq!(summary.hedged_enabled_count, 0);
        assert_eq!(summary.zero_copy_count, 0);
        assert_eq!(summary.min_read_cost, ShardReadCost::Unknown);
        assert_eq!(summary.max_timeout, Duration::ZERO);
    }

    // ----- 测试 10：ReadCapabilityError Display -----

    #[test]
    fn test_read_capability_error_display() {
        let e1 = ReadCapabilityError::ReadFailed(3, "io error".into());
        assert!(format!("{e1}").contains("shard 3 read failed"));

        let e2 = ReadCapabilityError::AllFailed(5);
        assert!(format!("{e2}").contains("all readers failed for shard 5"));

        let e3 = ReadCapabilityError::Timeout(Duration::from_secs(10));
        assert!(format!("{e3}").contains("read timed out"));

        let e4 = ReadCapabilityError::NotSupported("zero_copy".into());
        assert!(format!("{e4}").contains("capability not supported"));
    }

    // ----- 测试 11：ReaderPipeline locality 排序 -----

    #[tokio::test]
    async fn test_reader_pipeline_locality_ordering() {
        // 三个 reader：Remote 快、Local 慢、SameNode 中
        // sequential 应按 Local → SameNode → Remote 顺序尝试
        let call_order = Arc::new(parking_lot::Mutex::new(Vec::new()));

        struct OrderedReader {
            name: String,
            cost: ShardReadCost,
            call_order: Arc<parking_lot::Mutex<Vec<String>>>,
        }

        #[async_trait]
        impl ReaderCapability for OrderedReader {
            async fn read_shard(&self, _shard_index: usize) -> Result<Bytes, ReadCapabilityError> {
                self.call_order.lock().push(self.name.clone());
                // 都失败，让 sequential 遍历所有 reader
                Err(ReadCapabilityError::ReadFailed(_shard_index, format!("{} fail", self.name)))
            }

            fn read_cost(&self) -> ShardReadCost {
                self.cost
            }

            fn endpoint(&self) -> &str {
                &self.name
            }
        }

        let pipeline = ReaderPipeline::new()
            .with_reader(Arc::new(OrderedReader {
                name: "remote".into(),
                cost: ShardReadCost::Remote,
                call_order: call_order.clone(),
            }))
            .with_reader(Arc::new(OrderedReader {
                name: "local".into(),
                cost: ShardReadCost::Local,
                call_order: call_order.clone(),
            }))
            .with_reader(Arc::new(OrderedReader {
                name: "same_node".into(),
                cost: ShardReadCost::SameNode,
                call_order: call_order.clone(),
            }));

        let _ = pipeline.read_sequential(0).await;

        let order = call_order.lock();
        assert_eq!(*order, vec!["local", "same_node", "remote"]);
    }
}


#[cfg(test)]
mod additional_tests {
    use super::*;

    struct MockCap {
        endpoint: String,
        cost: ShardReadCost,
        delay: Duration,
        should_fail: bool,
        payload: Bytes,
        hedged: bool,
        zero_copy: bool,
        timeout: Duration,
    }

    #[allow(clippy::too_many_arguments)]
    impl MockCap {
        fn new(endpoint: &str, cost: ShardReadCost, payload: Bytes) -> Self {
            Self {
                endpoint: endpoint.to_string(),
                cost,
                delay: Duration::ZERO,
                should_fail: false,
                payload,
                hedged: false,
                zero_copy: false,
                timeout: Duration::from_secs(30),
            }
        }
        fn with_hedged(mut self, v: bool) -> Self {
            self.hedged = v;
            self
        }
        fn with_zero_copy(mut self, v: bool) -> Self {
            self.zero_copy = v;
            self
        }
        fn with_timeout(mut self, t: Duration) -> Self {
            self.timeout = t;
            self
        }
        #[allow(dead_code)]
        fn with_fail(mut self, v: bool) -> Self {
            self.should_fail = v;
            self
        }
    }

    #[async_trait]
    impl ReaderCapability for MockCap {
        async fn read_shard(&self, _shard_index: usize) -> Result<Bytes, ReadCapabilityError> {
            if self.delay > Duration::ZERO {
                tokio::time::sleep(self.delay).await;
            }
            if self.should_fail {
                Err(ReadCapabilityError::ReadFailed(
                    _shard_index,
                    format!("{} fail", self.endpoint),
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
        fn supports_hedged_read(&self) -> bool {
            self.hedged
        }
        fn supports_zero_copy(&self) -> bool {
            self.zero_copy
        }
        fn read_timeout(&self) -> Duration {
            self.timeout
        }
    }

    #[test]
    fn test_read_capability_error_display_all() {
        let e1 = ReadCapabilityError::ReadFailed(1, "err".into());
        assert!(format!("{e1}").contains("shard 1 read failed"));
        let e2 = ReadCapabilityError::AllFailed(2);
        assert!(format!("{e2}").contains("all readers failed for shard 2"));
        let e3 = ReadCapabilityError::Timeout(Duration::from_secs(5));
        assert!(format!("{e3}").contains("read timed out"));
        let e4 = ReadCapabilityError::NotSupported("zero_copy".into());
        assert!(format!("{e4}").contains("capability not supported"));
    }

    #[test]
    fn test_reader_pipeline_default() {
        let p = ReaderPipeline::default();
        assert_eq!(p.reader_count(), 0);
    }

    #[test]
    fn test_reader_pipeline_build() {
        let p = ReaderPipeline::new()
            .with_reader(Arc::new(MockCap::new(
                "a",
                ShardReadCost::Local,
                Bytes::from_static(b"x"),
            )))
            .build();
        // p is Arc<dyn ReaderCapability>
        assert_eq!(p.endpoint(), "a");
    }

    #[tokio::test]
    async fn test_reader_pipeline_read_shard_via_trait() {
        let payload = Bytes::from_static(b"via-trait");
        let p = ReaderPipeline::new().with_reader(Arc::new(MockCap::new(
            "a",
            ShardReadCost::Local,
            payload.clone(),
        )));
        let result = p.read_shard(0).await.unwrap();
        assert_eq!(result, payload);
    }

    #[test]
    fn test_reader_pipeline_capability_aggregation() {
        let p = ReaderPipeline::new()
            .with_reader(Arc::new(
                MockCap::new("a", ShardReadCost::Local, Bytes::new())
                    .with_hedged(true)
                    .with_zero_copy(true)
                    .with_timeout(Duration::from_secs(10)),
            ))
            .with_reader(Arc::new(
                MockCap::new("b", ShardReadCost::Remote, Bytes::new())
                    .with_timeout(Duration::from_secs(60)),
            ));
        assert!(p.supports_hedged_read());
        assert!(p.supports_zero_copy());
        assert_eq!(p.read_cost(), ShardReadCost::Local);
        assert_eq!(p.read_timeout(), Duration::from_secs(10)); // min timeout
        assert!(p.endpoint().contains("a"));
        assert!(p.endpoint().contains("b"));
    }

    #[test]
    fn test_reader_pipeline_empty_capabilities() {
        let p = ReaderPipeline::new();
        assert!(!p.supports_hedged_read());
        assert!(!p.supports_zero_copy());
        assert_eq!(p.read_cost(), ShardReadCost::Unknown);
        assert_eq!(p.read_timeout(), Duration::from_secs(30));
    }

    #[test]
    fn test_reader_capabilities_summary_fields() {
        let summary = ReaderCapabilitiesSummary {
            total_readers: 5,
            hedged_enabled_count: 3,
            zero_copy_count: 2,
            min_read_cost: ShardReadCost::Local,
            max_timeout: Duration::from_secs(60),
        };
        assert_eq!(summary.total_readers, 5);
        assert_eq!(summary.hedged_enabled_count, 3);
        assert_eq!(summary.zero_copy_count, 2);
        assert_eq!(summary.min_read_cost, ShardReadCost::Local);
        assert_eq!(summary.max_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_simple_reader_inner() {
        struct DummyReader;
        #[async_trait]
        impl ShardReader for DummyReader {
            async fn read_shard(&self, _idx: usize) -> Result<Bytes, ReadError> {
                Ok(Bytes::new())
            }
            fn read_cost(&self) -> ShardReadCost {
                ShardReadCost::Local
            }
            fn endpoint(&self) -> &str {
                "dummy"
            }
        }
        let reader = SimpleReader::new(Arc::new(DummyReader));
        let _inner = reader.inner();
        assert_eq!(reader.endpoint(), "dummy");
    }

    #[tokio::test]
    async fn test_read_capability_error_from_read_error() {
        let e = ReadError::ShardReadFailed(5, "io".into());
        let cap_e = ReadCapabilityError::from_read_error(5, e);
        assert!(matches!(cap_e, ReadCapabilityError::ReadFailed(5, _)));

        let e2 = ReadError::AllReadersFailed(3);
        let cap_e2 = ReadCapabilityError::from_read_error(3, e2);
        assert!(matches!(cap_e2, ReadCapabilityError::AllFailed(3)));

        let e3 = ReadError::Timeout;
        let cap_e3 = ReadCapabilityError::from_read_error(1, e3);
        assert!(matches!(cap_e3, ReadCapabilityError::Timeout(_)));
    }
}
