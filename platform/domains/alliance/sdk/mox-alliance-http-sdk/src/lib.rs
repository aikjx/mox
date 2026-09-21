//! Alliance HTTP integration, independently mountable in an Axum host.
//! Authentication and tenant authorization remain the host's responsibility.
//! Remote failures never switch to the legacy preview repository.
pub mod alliance;
pub mod alliance_remote;
pub use alliance::{build_alliance_router, build_alliance_router_with};
pub use alliance_remote::RemoteAllianceClient;

// Re-export common-proto types that gateway and other hosts consume via this SDK.
// Keeps the host's declared-dep fan-out lower while still exposing the protocol types.
pub use mox_alliance_common_proto::{FusionStrategy, AllianceMode, Expert, ExpertStatus, TaskPriority, TaskStatus};

// 命名映射 SSOT（模式 / 融合策略的 serde 名 ↔ 展示名 ↔ 历史别名）转出。
// 上层（如网关）只需依赖本 SDK 即可拿到唯一真源，不必再直接依赖协议层 crate。
pub use mox_alliance_common_proto::{
    fusion_display, fusion_from_any, fusion_serde, mode_display, mode_from_any, mode_serde,
    ALL_FUSIONS, ALL_MODES,
};
