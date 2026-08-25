//! Xuanji Fusion: PutObject -> Tag -> CDC -> Graph fusion stage.

pub mod audit_sync;
pub mod cdc_stage;
pub mod graph_projection_bridge;
pub mod graph_writer;
pub mod tag_parser;

pub use audit_sync::{AuditBlock, AuditChain, AuditEvent, AuditRecordKind};
pub use cdc_stage::{tag_cdc_graph_stage, ObjectTagged};
pub use graph_projection_bridge::ProjectionBridge;
pub use graph_writer::{
    Error as GraphError, GraphWriter, GraphWriterStats, ObjectMeta, Result as GraphResult, TagMeta,
};
pub use tag_parser::{Tag, TagSet};
