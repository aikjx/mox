//! Shared provider access, registry and model routing. AI-domain reasoning is separate.
pub mod providers;
pub mod registry;
pub mod router;
pub use providers::*;
pub use registry::ProviderRegistry;
pub use router::{ModelRouter, RouteEntry, RoutingStrategy};
