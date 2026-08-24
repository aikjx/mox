//! # Xuanji Graph Spark Connector (contract-only)
//!
//! In-process contract simulating the Spark <-> Xuanji Graph integration:
//!
//! - [`GraphSparkReader`]: paged nodes / edges frames with Long/String/Map schema fields.
//! - [`GraphSparkWriter`]: bulk upsert with idempotent key `(source, target, label)` for edges,
//!   `id` for nodes. Returns WrittenStats. Round-trip symmetry checked by tests.

pub mod graph_spark_reader;
pub mod graph_spark_writer;

pub use graph_spark_reader::{GraphSparkReader, NodeFrame, EdgeFrame, GraphSchemaField, GraphSchema};
pub use graph_spark_writer::{GraphSparkWriter, WrittenStats, SparkRow, IdempotencyKey};
