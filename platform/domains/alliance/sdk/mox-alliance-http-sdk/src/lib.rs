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
