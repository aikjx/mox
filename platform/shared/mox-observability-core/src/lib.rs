// =============================================================================
// MOX 统一可观测性核心（mox-observability-core）
// =============================================================================
//
// 企业级可观测性基础设施，提供：
//
// 1. **指标收集**（metrics）— Prometheus 格式指标，Counter/Gauge/Histogram
// 2. **分布式追踪**（tracing）— TraceId/SpanId 传播，上下文管理
// 3. **结构化日志**（logging）— JSON 格式日志，字段化记录
// 4. **健康检查**（health）— 存活/就绪探针，依赖检查
//
// 设计原则：
// - 三大支柱：指标 + 日志 + 追踪，统一关联
// - 低开销：采样 + 批量，生产环境可忽略的性能影响
// - 标准化：OpenTelemetry 兼容格式，便于生态集成
// - 可配置：通过环境变量和配置文件控制采样率和输出
// =============================================================================

pub mod metrics;
pub mod tracing;
pub mod logging;
pub mod health;

// ── 重导出 ────────────────────────────────────────────────────────────────

pub use metrics::{MetricsRegistry, MetricsCollector, MetricType, MetricValue};
pub use tracing::{TraceContext, Span, SpanId, TraceId, TraceState};
pub use logging::{LogLevel, LogEntry, LogCollector, StructuredLogger};
pub use health::{HealthChecker, HealthStatus, ComponentHealth, HealthReport};

// ── Crate 元数据 ──────────────────────────────────────────────────────────

pub const CRATE_ID: &str = "mox-observability-core";
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

use serde::{Deserialize, Serialize};

/// 可观测性错误
#[derive(Debug, thiserror::Error)]
pub enum ObservabilityError {
    #[error("指标注册失败: {0}")]
    MetricRegistrationFailed(String),
    #[error("追踪上下文错误: {0}")]
    TraceContextError(String),
    #[error("健康检查失败: {0}")]
    HealthCheckFailed(String),
    #[error("内部错误: {0}")]
    InternalError(String),
}

/// 可观测性结果类型
pub type ObservabilityResult<T> = Result<T, ObservabilityError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observability_error_display() {
        let err = ObservabilityError::MetricRegistrationFailed("test".to_string());
        assert!(format!("{}", err).contains("test"));
    }
}
