//! Mox v2.0 AIS-grade fusion: T21 E2E Harness + Task12 Rubric grade calculator.
//!
//! The crate exposes the [`Rubric`] structure (Task 12 Grade S calculator) and
//! re-exports several mox subsystem symbols used by the 777+ integration
//! tests under `tests/`.

pub mod rubric;

pub use rubric::Rubric;

// Re-export multipart CRC-64/ECMA helper used by a5_crc64_roundtrip matrix.
pub use mox_data_plane_svc::multipart;

// Re-exports used by integration test matrices.
pub use mox_cloud_volume_svc::{reed_solomon, profile};
pub use mox_data_compliance_svc::miji;
pub use mox_kg_fusion_svc as fusion;
pub use mox_platform_gateway_svc::o11y;
pub use mox_data_etl_svc as etl;
pub use mox_data_etl_svc::abi::{PluginKindStr, TransformSummary, EtError};
