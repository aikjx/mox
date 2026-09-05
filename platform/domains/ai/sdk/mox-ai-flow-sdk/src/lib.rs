//! Public in-process flow optimization client.
//! Cross-domain consumers use this SDK; algorithm implementation lives in the AI core.
//! The SDK does not start a service, access credentials or require an HTTP runtime.
pub use mox_ai_flow_core::*;
