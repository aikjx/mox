//! Compatibility facade and CLI host for the flow algorithms.
//! Pure algorithms live in `mox_ai_flow_core`; public module paths remain compatible.

pub use mox_ai_flow_core::*;

pub const CRATE_ID: &str = "2fcd3eac-e894-5876-b007-fb33c56c0d65";
pub const ENGINE_NAME: &str = "mox::mox_ai_flow_svc";
pub const CRATE_META: mox_platform_foundation::CrateMeta = mox_platform_foundation::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: mox_platform_foundation::AisLayer::L4Services,
    owner: "mox-core",
};
