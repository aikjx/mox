// =============================================================================
// MOX 统一归一化契约层（SSOT - Single Source Of Truth）
// =============================================================================
//
// 本 crate 是跨 Rust / Python / 前端三端的统一类型定义来源。
// 所有业务域必须依赖此 crate，禁止自定义重复类型。
//
// 模块清单：
// - error:      统一错误码体系（域代码 + 模块代码 + 序号）
// - quality:    质量分与等级（A/B/C/D 四级门禁）
// - event:      统一事件格式（AllianceEvent / StreamEvent）
// - trace:      追踪ID规范（trace_id / span_id / 全链路透传）
// - response:   统一响应信封（code / msg / data / trace_id）
// - pagination: 分页/排序/过滤的统一请求格式
// - normalize:  归一化工具函数（分数clamp / 权重归一化 / 共识度计算）
//
// 设计原则：
// 1. 零业务依赖：只依赖 serde/chrono/uuid，不依赖任何业务域 crate
// 2. 向前兼容：所有字段使用 #[serde(default)]，新增字段不破坏旧版本
// 3. 跨端一致：Python/前端必须使用相同的 JSON 序列化格式
// 4. 可审计：所有类型必须可序列化为 JSON，便于日志和审计
// =============================================================================

pub mod error;
pub mod quality;
pub mod event;
pub mod trace;
pub mod response;
pub mod pagination;
pub mod normalize;

// ── 重导出 ────────────────────────────────────────────────────────────────

// 错误码
pub use error::{MoxError, MoxResult, ErrorLevel, ErrorDomain, ErrorCode};

// 质量分
pub use quality::{QualityGrade, QualityScore, GateResult, GATE_THRESHOLDS};

// 事件
pub use event::{MoxEvent, EventPhase, EventType, StreamEvent};

// 追踪
pub use trace::{TraceId, TraceContext, current_trace_id, with_trace_context};

// 响应信封
pub use response::{ApiResponse, ApiSuccess, ApiError, PagedResponse};

// 分页
pub use pagination::{PaginationRequest, SortOrder, FilterCondition, QueryRequest, SortCondition, FilterOperator};

// 归一化工具
pub use normalize::{clamp_score, normalize_weights, compute_consensus, weighted_average};

// ── Crate 元数据 ──────────────────────────────────────────────────────────

/// Crate 唯一标识（用于注册表和审计）
pub const CRATE_ID: &str = "mox-unified-contract";

/// Crate 版本
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 契约版本（跨端对齐用，Python/前端必须使用相同版本）
pub const CONTRACT_VERSION: &str = "1.0.0";

/// 支持的最低契约版本（向后兼容边界）
pub const MIN_CONTRACT_VERSION: &str = "1.0.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_version_is_semver() {
        assert!(CONTRACT_VERSION.split('.').count() == 3);
        assert!(MIN_CONTRACT_VERSION.split('.').count() == 3);
    }

    #[test]
    fn crate_id_not_empty() {
        assert!(!CRATE_ID.is_empty());
    }
}
