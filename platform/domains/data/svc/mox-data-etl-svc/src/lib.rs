//! ETL near-data WASM plugin framework (ABI stubs).
//!
//! Three plugin kinds:
//!   - InlineGet: transform bytes on-the-fly during GET (e.g. md5sum, uppercase, decompress)
//!   - InlinePut: preprocess bytes during PUT before they land storage (compress, mask PII, fingerprint)
//!   - Offline: background xaction applied to every object inside a bucket
//!
//! Without feature "wasm-mox_platform_orchestrator_svc", only pure-Rust mock transforms are supported (unit tests + SDK stubs).

pub mod abi;
pub mod registry;
pub mod context;

pub use abi::{EtResult, InlineGet, InlinePut, Md5Sum, OfflineXaction};
pub use context::EtContext;
pub use registry::{PluginId, PluginKind, PluginRegistry};
