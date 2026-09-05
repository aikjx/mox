//! Alliance HTTP integration, independently mountable in an Axum host.
//! Authentication and tenant authorization remain the host's responsibility.
//! Remote failures never switch to the legacy preview repository.
pub mod alliance;
pub mod alliance_remote;
pub use alliance::{build_alliance_router, build_alliance_router_with};
pub use alliance_remote::RemoteAllianceClient;
