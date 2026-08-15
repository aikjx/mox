//! AuditSink trait — 外部审计日志写入接口

use super::event::ExtAuditEvent;
use super::AuditError;

/// 刷新策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushPolicy {
    Immediate,
    Batch { max_events: usize },
    Periodic { interval_ms: u64 },
}

impl Default for FlushPolicy {
    fn default() -> Self { Self::Batch { max_events: 100 } }
}

/// 外部审计写入接口
pub trait AuditSink: Send + Sync {
    fn append_sync(&self, event: &ExtAuditEvent) -> Result<(), AuditError>;
    fn append_async(&self, _event: &ExtAuditEvent) {}
    fn append_batch(&self, events: &[ExtAuditEvent]) -> Result<(), AuditError> {
        for ev in events { self.append_sync(ev)?; }
        Ok(())
    }
    fn flush(&self) -> Result<(), AuditError> { Ok(()) }
    fn health_check(&self) -> Result<(), AuditError> { Ok(()) }
    fn is_enabled(&self) -> bool { true }
}

/// 组合 Sink：同时写入多个外部存储，至少一个成功即可
pub struct MultiSink { sinks: Vec<Box<dyn AuditSink>> }

impl MultiSink {
    pub fn new() -> Self { Self { sinks: Vec::new() } }
    pub fn add(mut self, sink: Box<dyn AuditSink>) -> Self { self.sinks.push(sink); self }
}

impl Default for MultiSink { fn default() -> Self { Self::new() } }

impl AuditSink for MultiSink {
    fn append_sync(&self, event: &ExtAuditEvent) -> Result<(), AuditError> {
        let enabled: Vec<_> = self.sinks.iter().filter(|s| s.is_enabled()).collect();
        if enabled.is_empty() {
            return Err(AuditError::Disabled);
        }
        let mut last_err = Ok(());
        for sink in enabled {
            if sink.append_sync(event).is_err() {
                // 记录失败但不中断，让其他 sink 继续
                last_err = Err(AuditError::WriteFailed("one or more sinks failed".into()));
            }
        }
        last_err
    }

    fn flush(&self) -> Result<(), AuditError> {
        for sink in &self.sinks { if sink.is_enabled() { sink.flush()?; } }
        Ok(())
    }

    fn is_enabled(&self) -> bool { self.sinks.iter().any(|s| s.is_enabled()) }
}

/// 空操作 Sink（开发/测试环境）：写入即丢弃，但视为「已启用」，
/// 因此可作为 MultiSink 中唯一 sink 使用（不会触发 Disabled 错误）。
pub struct NoopSink;
impl AuditSink for NoopSink {
    fn append_sync(&self, _e: &ExtAuditEvent) -> Result<(), AuditError> { Ok(()) }
    fn flush(&self) -> Result<(), AuditError> { Ok(()) }
    fn is_enabled(&self) -> bool { true }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn noop_append_ok() { assert!(NoopSink.append_sync(&super::super::event::test_event()).is_ok()); }
    #[test]
    fn multi_empty_disabled() { assert!(!MultiSink::new().is_enabled()); }
    #[test]
    fn multi_noop_is_usable() {
        let m = MultiSink::new().add(Box::new(NoopSink));
        assert!(m.append_sync(&super::super::event::test_event()).is_ok());
    }
}
