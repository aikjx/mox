//! # Xuanji Graph Streams — Flink-compatible CDC Source
//!
//! ```text
//! FlinkCdcSource::new(Arc<CdcSource>)
//!   ├─ next_blocking(timeout) → Option<CdcEvent>
//!   ├─ resume(offset)         → reposition cursor
//!   └─ stats()                → Stats { received, duplicates, lost, lag_ms }
//! ```

pub mod flink_source;

pub use flink_source::{FlinkCdcSource, FlinkSourceStats, IdempotentWriter, DedupIdempotentReport};
