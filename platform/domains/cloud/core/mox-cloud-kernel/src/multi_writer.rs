// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 写仲裁模块 — MultiWriter + WriteProgressPolicy
//!
//! 多副本 / 多 shard 并发写入时，防止 black-hole peer（写入慢或无响应的节点）
//! 拖慢整体写入。通过 write_quorum 仲裁和 stall_timeout 超时控制，确保只要达到
//! 法定写入数就继续，慢节点被剔除。
//!
//! 算法参考：RustFS ecstore (Apache 2.0) 的 MultiWriter / WriteProgressPolicy
//! 流式编码管线模式；本模块为独立重写实现，未直接复制源码。

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

// ---------------------------------------------------------------------------
// 错误类型
// ---------------------------------------------------------------------------

/// 写仲裁错误
#[derive(Debug, Error)]
pub enum WriteError {
    /// 单个 shard 写入失败
    #[error("shard {0} write failed: {1}")]
    ShardWriteFailed(usize, String),

    /// 未达到写仲裁法定数
    #[error("write quorum not met: {succeeded}/{quorum}")]
    QuorumNotMet {
        /// 实际成功数
        succeeded: usize,
        /// 要求的法定数
        quorum: usize,
    },

    /// 写入超时（absolute_cap 触发）
    #[error("write timed out after {0:?}")]
    Timeout(Duration),
}

// ---------------------------------------------------------------------------
// 策略配置
// ---------------------------------------------------------------------------

/// 写入进度策略
///
/// 控制多 writer 并发写入时的超时与仲裁行为。
#[derive(Debug, Clone)]
pub struct WriteProgressPolicy {
    /// 每块写入超时（stall timeout），按块 re-arm，默认 30s
    pub stall_timeout: Duration,

    /// 绝对超时上限（防 slow-drip peer），默认 None（关闭）
    pub absolute_cap: Option<Duration>,

    /// 写仲裁法定数：成功写入数 >= write_quorum 即通过，默认 = data_shards + 1
    pub write_quorum: usize,
}

impl Default for WriteProgressPolicy {
    fn default() -> Self {
        Self {
            stall_timeout: Duration::from_secs(30),
            absolute_cap: None,
            // 使用时根据 profile 计算（通常 = data_shards + 1）
            write_quorum: 0,
        }
    }
}

impl WriteProgressPolicy {
    /// 根据 EcProfile 计算默认 write_quorum = data_shards + 1
    pub fn with_quorum_for_data_shards(mut self, data_shards: usize) -> Self {
        self.write_quorum = data_shards.saturating_add(1);
        self
    }

    /// 生效的法定数（至少为 1，避免 quorum=0 的退化情形）
    pub(crate) fn effective_quorum(&self) -> usize {
        self.write_quorum.max(1)
    }
}

// ---------------------------------------------------------------------------
// ShardWriter trait
// ---------------------------------------------------------------------------

/// 单个 shard 写入器抽象
///
/// 每个实现对应一个存储节点 / 磁盘端点，负责将指定 shard 的数据写入远端。
#[async_trait]
pub trait ShardWriter: Send + Sync {
    /// 写入指定 shard 的数据
    async fn write_shard(&self, shard_index: usize, data: Bytes) -> Result<(), WriteError>;

    /// 端点标识（用于日志和 locality 判断）
    fn endpoint(&self) -> &str;
}

// ---------------------------------------------------------------------------
// WriteResult
// ---------------------------------------------------------------------------

/// 批量写入结果
#[derive(Debug, Clone)]
pub struct WriteResult {
    /// 成功写入的 shard index 列表
    pub succeeded: Vec<usize>,

    /// 失败 / 超时 / 未完成的 shard index 列表
    pub failed: Vec<usize>,

    /// 本次写入总耗时
    pub duration: Duration,
}

// ---------------------------------------------------------------------------
// MultiWriter
// ---------------------------------------------------------------------------

/// 多 writer 并发写入仲裁器
///
/// 将一组 shard 数据并发写入多个 `ShardWriter`，达到 `write_quorum` 即返回成功。
/// 慢节点或失败节点被记录在 `WriteResult.failed` 中，调用方可在 commit 前剔除。
pub struct MultiWriter {
    writers: Vec<Arc<dyn ShardWriter>>,
    policy: WriteProgressPolicy,
}

impl MultiWriter {
    /// 创建新的 MultiWriter
    pub fn new(writers: Vec<Arc<dyn ShardWriter>>, policy: WriteProgressPolicy) -> Self {
        Self { writers, policy }
    }

    /// 获取当前策略（只读）
    pub fn policy(&self) -> &WriteProgressPolicy {
        &self.policy
    }

    /// 获取 writer 数量
    pub fn writer_count(&self) -> usize {
        self.writers.len()
    }

    /// 并发写所有 shard，达到 write_quorum 即返回成功
    ///
    /// `shards` 中的第 i 项由 `writers[i]` 负责写入（按位置 1:1 配对）。
    /// 若 shards 与 writers 长度不一致，取较短者进行配对。
    ///
    /// 达到法定数后立即返回，尚未完成的写入被取消并计入 `failed`。
    pub async fn write_all(&self, shards: Vec<(usize, Bytes)>) -> Result<WriteResult, WriteError> {
        let start = Instant::now();
        let pair_count = shards.len().min(self.writers.len());
        let quorum = self.policy.effective_quorum();

        if pair_count == 0 {
            return Ok(WriteResult {
                succeeded: Vec::new(),
                failed: Vec::new(),
                duration: Duration::ZERO,
            });
        }

        // 记录所有待写入的 shard index，用于在提前返回时标记未完成者
        let all_indices: Vec<usize> = shards[..pair_count].iter().map(|(idx, _)| *idx).collect();
        let mut pending: Vec<usize> = all_indices.clone();

        // 构建并发写入 futures
        let mut futures: FuturesUnordered<_> = FuturesUnordered::new();
        for i in 0..pair_count {
            let writer = Arc::clone(&self.writers[i]);
            let (shard_idx, data) = shards[i].clone();
            let stall = self.policy.stall_timeout;
            futures.push(async move {
                let outcome = tokio::time::timeout(stall, writer.write_shard(shard_idx, data)).await;
                (shard_idx, outcome)
            });
        }

        let mut succeeded: Vec<usize> = Vec::new();
        let mut failed: Vec<usize> = Vec::new();

        while let Some((shard_idx, outcome)) = futures.next().await {
            // 检查绝对超时上限
            if let Some(cap) = self.policy.absolute_cap {
                if start.elapsed() > cap {
                    return Err(WriteError::Timeout(cap));
                }
            }

            // 从 pending 中移除
            if let Some(pos) = pending.iter().position(|&x| x == shard_idx) {
                pending.swap_remove(pos);
            }

            match outcome {
                // timeout 内返回了 Ok
                Ok(Ok(())) => {
                    succeeded.push(shard_idx);

                    // 达到法定数：将剩余 pending 计入 failed 并提前返回
                    if succeeded.len() >= quorum {
                        failed.extend(pending.drain(..));
                        return Ok(WriteResult {
                            succeeded,
                            failed,
                            duration: start.elapsed(),
                        });
                    }
                }
                // timeout 内返回了 Err
                Ok(Err(e)) => {
                    tracing::warn!(
                        shard_index = shard_idx,
                        endpoint = %self.writers.iter()
                            .find(|w| w.endpoint().ends_with(&shard_idx.to_string()))
                            .map(|w| w.endpoint().to_string())
                            .unwrap_or_default(),
                        error = %e,
                        "shard write failed"
                    );
                    failed.push(shard_idx);
                }
                // stall timeout 触发
                Err(_) => {
                    tracing::warn!(shard_index = shard_idx, "shard write stalled (timeout)");
                    failed.push(shard_idx);
                }
            }
        }

        // 所有 future 均已完成
        let duration = start.elapsed();

        if succeeded.len() >= quorum {
            Ok(WriteResult {
                succeeded,
                failed,
                duration,
            })
        } else {
            Err(WriteError::QuorumNotMet {
                succeeded: succeeded.len(),
                quorum,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock ShardWriter：可配置延迟和成功率
    struct MockShardWriter {
        endpoint: String,
        delay: Duration,
        should_fail: bool,
    }

    impl MockShardWriter {
        fn new(endpoint: &str, delay: Duration, should_fail: bool) -> Self {
            Self {
                endpoint: endpoint.to_string(),
                delay,
                should_fail,
            }
        }
    }

    #[async_trait]
    impl ShardWriter for MockShardWriter {
        async fn write_shard(&self, _shard_index: usize, _data: Bytes) -> Result<(), WriteError> {
            if self.delay > Duration::ZERO {
                tokio::time::sleep(self.delay).await;
            }
            if self.should_fail {
                Err(WriteError::ShardWriteFailed(
                    _shard_index,
                    "mock writer failure".into(),
                ))
            } else {
                Ok(())
            }
        }

        fn endpoint(&self) -> &str {
            &self.endpoint
        }
    }

    fn make_writers(count: usize, delay: Duration, should_fail: bool) -> Vec<Arc<dyn ShardWriter>> {
        (0..count)
            .map(|i| {
                Arc::new(MockShardWriter::new(
                    &format!("node-{i}"),
                    delay,
                    should_fail,
                )) as Arc<dyn ShardWriter>
            })
            .collect()
    }

    fn make_shards(count: usize) -> Vec<(usize, Bytes)> {
        (0..count)
            .map(|i| (i, Bytes::from(vec![i as u8; 64])))
            .collect()
    }

    // ----- 测试 1：全部成功 -----

    #[tokio::test]
    async fn test_multi_writer_all_succeed() {
        let writers = make_writers(3, Duration::ZERO, false);
        let policy = WriteProgressPolicy {
            write_quorum: 3,
            ..Default::default()
        };
        let mw = MultiWriter::new(writers, policy);

        let result = mw.write_all(make_shards(3)).await.unwrap();
        assert_eq!(result.succeeded.len(), 3);
        assert!(result.failed.is_empty());
        assert!(result.duration >= Duration::ZERO);
    }

    // ----- 测试 2：达到法定数，慢节点被剔除 -----

    #[tokio::test]
    async fn test_multi_writer_quorum_met() {
        // 3 个快 writer + 2 个慢 writer（会超时）
        let mut writers: Vec<Arc<dyn ShardWriter>> = Vec::new();
        for i in 0..3 {
            writers.push(Arc::new(MockShardWriter::new(
                &format!("fast-{i}"),
                Duration::ZERO,
                false,
            )));
        }
        for i in 0..2 {
            writers.push(Arc::new(MockShardWriter::new(
                &format!("slow-{i}"),
                Duration::from_secs(60), // 远超 stall_timeout
                false,
            )));
        }

        let policy = WriteProgressPolicy {
            stall_timeout: Duration::from_millis(100),
            write_quorum: 3,
            ..Default::default()
        };
        let mw = MultiWriter::new(writers, policy);

        let result = mw.write_all(make_shards(5)).await.unwrap();
        assert_eq!(result.succeeded.len(), 3);
        // 慢节点在 quorum 达成时仍 pending，被计入 failed
        assert_eq!(result.failed.len(), 2);
    }

    // ----- 测试 3：未达到法定数 -----

    #[tokio::test]
    async fn test_multi_writer_quorum_not_met() {
        // 1 个成功 + 2 个失败
        let mut writers: Vec<Arc<dyn ShardWriter>> = Vec::new();
        writers.push(Arc::new(MockShardWriter::new("ok-0", Duration::ZERO, false)));
        writers.push(Arc::new(MockShardWriter::new("fail-1", Duration::ZERO, true)));
        writers.push(Arc::new(MockShardWriter::new("fail-2", Duration::ZERO, true)));

        let policy = WriteProgressPolicy {
            write_quorum: 3,
            ..Default::default()
        };
        let mw = MultiWriter::new(writers, policy);

        let result = mw.write_all(make_shards(3)).await;
        match result {
            Err(WriteError::QuorumNotMet { succeeded, quorum }) => {
                assert_eq!(succeeded, 1);
                assert_eq!(quorum, 3);
            }
            other => panic!("expected QuorumNotMet, got {other:?}"),
        }
    }

    // ----- 测试 4：默认 policy 值 -----

    #[test]
    fn test_write_progress_policy_default() {
        let p = WriteProgressPolicy::default();
        assert_eq!(p.stall_timeout, Duration::from_secs(30));
        assert_eq!(p.absolute_cap, None);
        assert_eq!(p.write_quorum, 0);
    }

    // ----- 额外测试：with_quorum_for_data_shards -----

    #[test]
    fn test_policy_with_quorum_for_data_shards() {
        let p = WriteProgressPolicy::default().with_quorum_for_data_shards(4);
        assert_eq!(p.write_quorum, 5); // data_shards + 1

        let p2 = WriteProgressPolicy::default().with_quorum_for_data_shards(0);
        assert_eq!(p2.write_quorum, 1); // saturating_add
    }

    // ----- 额外测试：absolute_cap 超时 -----

    #[tokio::test]
    async fn test_multi_writer_absolute_cap() {
        let writers = make_writers(3, Duration::from_secs(10), false);
        let policy = WriteProgressPolicy {
            stall_timeout: Duration::from_secs(30),
            absolute_cap: Some(Duration::from_millis(50)),
            write_quorum: 2,
        };
        let mw = MultiWriter::new(writers, policy);

        let result = mw.write_all(make_shards(3)).await;
        assert!(matches!(result, Err(WriteError::Timeout(_))));
    }

    // ----- 额外测试：空输入 -----

    #[tokio::test]
    async fn test_multi_writer_empty() {
        let writers = make_writers(3, Duration::ZERO, false);
        let policy = WriteProgressPolicy {
            write_quorum: 1,
            ..Default::default()
        };
        let mw = MultiWriter::new(writers, policy);

        let result = mw.write_all(vec![]).await.unwrap();
        assert!(result.succeeded.is_empty());
        assert!(result.failed.is_empty());
    }
}
