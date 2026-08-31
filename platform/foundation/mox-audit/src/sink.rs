// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! AuditSink trait — 外部审计日志写入接口
//!
//! 任何存储后端实现此 trait 即可即插即用。
//! 内置实现：
//! - `NoopSink`：空操作（开发/测试）
//! - `MultiSink`：组合多个 Sink（同时写入）
//! - `SyslogSink`：RFC 5424 系统日志（syslog 模块）
//! - `S3Sink`：S3 兼容对象存储（s3 模块，feature = "s3-sink"）

use crate::error::AuditError;
use crate::event::AuditEvent;

/// 刷新策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushPolicy {
    /// 立即写入（每条事件都直接发送）
    Immediate,
    /// 批量写入（达到 max_events 条时刷新）
    Batch { max_events: usize },
    /// 周期写入（每隔 interval_ms 毫秒刷新）
    Periodic { interval_ms: u64 },
}

impl Default for FlushPolicy {
    fn default() -> Self {
        Self::Batch { max_events: 100 }
    }
}

/// 外部审计写入接口
///
/// 实现此 trait 可将审计事件写入任意外部系统。
/// 所有方法都有默认实现，只需按需覆盖。
pub trait AuditSink: Send + Sync {
    /// 同步写入单条事件
    fn append_sync(&self, event: &AuditEvent) -> Result<(), AuditError>;

    /// 异步写入单条事件（默认不实现，fire-and-forget）
    fn append_async(&self, _event: &AuditEvent) {}

    /// 批量写入（默认逐条调用 append_sync）
    fn append_batch(&self, events: &[AuditEvent]) -> Result<(), AuditError> {
        for ev in events {
            self.append_sync(ev)?;
        }
        Ok(())
    }

    /// 刷新缓冲（将缓冲中的事件全部写出）
    fn flush(&self) -> Result<(), AuditError> {
        Ok(())
    }

    /// 健康检查
    fn health_check(&self) -> Result<(), AuditError> {
        Ok(())
    }

    /// Sink 是否启用
    fn is_enabled(&self) -> bool {
        true
    }

    /// Sink 名称（用于日志和调试）
    fn name(&self) -> &str {
        "unnamed_sink"
    }
}

// =============================================================================
// MultiSink — 组合多个 Sink
// =============================================================================

/// 组合 Sink：同时写入多个外部存储，至少一个成功即可
///
/// 设计原则：审计写入"尽力而为"，单个 Sink 失败不应阻断其他 Sink。
/// 所有 Sink 都失败时才返回错误。
pub struct MultiSink {
    sinks: Vec<Box<dyn AuditSink>>,
}

impl MultiSink {
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    pub fn with_sink(mut self, sink: Box<dyn AuditSink>) -> Self {
        self.sinks.push(sink);
        self
    }

    pub fn add_sink(&mut self, sink: Box<dyn AuditSink>) {
        self.sinks.push(sink);
    }

    pub fn sink_count(&self) -> usize {
        self.sinks.len()
    }
}

impl Default for MultiSink {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditSink for MultiSink {
    fn append_sync(&self, event: &AuditEvent) -> Result<(), AuditError> {
        let enabled: Vec<_> = self.sinks.iter().filter(|s| s.is_enabled()).collect();
        if enabled.is_empty() {
            return Err(AuditError::Disabled);
        }

        let mut failed = 0usize;
        let mut last_err_msg = String::new();

        for sink in &enabled {
            if let Err(e) = sink.append_sync(event) {
                failed += 1;
                last_err_msg = format!("{}: {}", sink.name(), e);
                // 记录失败但不中断，让其他 sink 继续
                tracing::warn!(target: "audit", "sink '{}' write failed: {}", sink.name(), e);
            }
        }

        if failed == enabled.len() {
            Err(AuditError::WriteFailed(format!(
                "所有 {} 个 sink 均失败，最后错误: {}",
                enabled.len(), last_err_msg
            )))
        } else if failed > 0 {
            // 部分失败不返回错误，但通过 tracing 告警
            Ok(())
        } else {
            Ok(())
        }
    }

    fn append_batch(&self, events: &[AuditEvent]) -> Result<(), AuditError> {
        let enabled: Vec<_> = self.sinks.iter().filter(|s| s.is_enabled()).collect();
        if enabled.is_empty() {
            return Err(AuditError::Disabled);
        }

        for sink in &enabled {
            if let Err(e) = sink.append_batch(events) {
                tracing::warn!(target: "audit", "sink '{}' batch write failed: {}", sink.name(), e);
            }
        }
        Ok(())
    }

    fn flush(&self) -> Result<(), AuditError> {
        for sink in &self.sinks {
            if sink.is_enabled() {
                if let Err(e) = sink.flush() {
                    tracing::warn!(target: "audit", "sink '{}' flush failed: {}", sink.name(), e);
                }
            }
        }
        Ok(())
    }

    fn health_check(&self) -> Result<(), AuditError> {
        let enabled: Vec<_> = self.sinks.iter().filter(|s| s.is_enabled()).collect();
        if enabled.is_empty() {
            return Err(AuditError::Disabled);
        }
        for sink in &enabled {
            sink.health_check()?;
        }
        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.sinks.iter().any(|s| s.is_enabled())
    }

    fn name(&self) -> &str {
        "multi_sink"
    }
}

// =============================================================================
// NoopSink — 空操作 Sink
// =============================================================================

/// 空操作 Sink（开发/测试环境）
///
/// 写入即丢弃，但视为「已启用」，因此可作为 MultiSink 中唯一 sink 使用
/// （不会触发 Disabled 错误）。
pub struct NoopSink;

impl AuditSink for NoopSink {
    fn append_sync(&self, _e: &AuditEvent) -> Result<(), AuditError> {
        Ok(())
    }

    fn flush(&self) -> Result<(), AuditError> {
        Ok(())
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "noop_sink"
    }
}

// =============================================================================
// 单元测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::test_event;

    #[test]
    fn noop_append_ok() {
        assert!(NoopSink.append_sync(&test_event()).is_ok());
    }

    #[test]
    fn noop_is_enabled() {
        assert!(NoopSink.is_enabled());
    }

    #[test]
    fn multi_empty_disabled() {
        assert!(!MultiSink::new().is_enabled());
        assert!(matches!(
            MultiSink::new().append_sync(&test_event()),
            Err(AuditError::Disabled)
        ));
    }

    #[test]
    fn multi_noop_is_usable() {
        let m = MultiSink::new().with_sink(Box::new(NoopSink));
        assert!(m.is_enabled());
        assert!(m.append_sync(&test_event()).is_ok());
        assert_eq!(m.sink_count(), 1);
    }

    #[test]
    fn multi_partial_failure_still_ok() {
        struct FailingSink;
        impl AuditSink for FailingSink {
            fn append_sync(&self, _e: &AuditEvent) -> Result<(), AuditError> {
                Err(AuditError::WriteFailed("test failure".into()))
            }
            fn name(&self) -> &str { "failing" }
        }

        let m = MultiSink::new()
            .with_sink(Box::new(FailingSink))
            .with_sink(Box::new(NoopSink));

        // 一个成功一个失败 → 整体成功
        assert!(m.append_sync(&test_event()).is_ok());
    }

    #[test]
    fn multi_all_failure_returns_error() {
        struct FailingSink;
        impl AuditSink for FailingSink {
            fn append_sync(&self, _e: &AuditEvent) -> Result<(), AuditError> {
                Err(AuditError::WriteFailed("fail".into()))
            }
            fn name(&self) -> &str { "failing" }
        }

        let m = MultiSink::new()
            .with_sink(Box::new(FailingSink))
            .with_sink(Box::new(FailingSink));

        let result = m.append_sync(&test_event());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AuditError::WriteFailed(_)));
        assert!(err.to_string().contains("所有"));
    }

    #[test]
    fn multi_flush_ok() {
        let m = MultiSink::new().with_sink(Box::new(NoopSink));
        assert!(m.flush().is_ok());
    }

    #[test]
    fn multi_health_check() {
        let m = MultiSink::new().with_sink(Box::new(NoopSink));
        assert!(m.health_check().is_ok());
    }

    #[test]
    fn flush_policy_default() {
        assert_eq!(FlushPolicy::default(), FlushPolicy::Batch { max_events: 100 });
    }

    #[test]
    fn add_sink_method() {
        let mut m = MultiSink::new();
        m.add_sink(Box::new(NoopSink));
        assert_eq!(m.sink_count(), 1);
    }
}
