//! Xuanji v2.0 AIS-grade fusion: T21 E2E Harness + Task12 Rubric grade calculator.
//!
//! The crate exposes the [`Rubric`] structure (Task 12 Grade S calculator) and
//! re-exports several xuanji subsystem symbols used by the 777+ integration
//! tests under `tests/`.

pub mod rubric;

pub use rubric::Rubric;

// Re-export multipart CRC-64/ECMA helper used by a5_crc64_roundtrip matrix.
pub use xuanji_data_plane::multipart;

// Re-exports used by integration test matrices.
pub use xuanji_cloud_drive_volume::{reed_solomon, profile};
pub use xuanji_compliance::miji;
pub use xuanji_fusion as fusion;
pub use xuanji_server::o11y;
pub use xuanji_etl_wasm as etl;
pub use xuanji_etl_wasm::abi::{PluginKindStr, TransformSummary, EtError};
