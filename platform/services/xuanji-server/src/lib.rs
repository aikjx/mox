//! Xuanji v2.0 AIS-grade fusion single-binary server library.
//!
//! Exports:
//! - [`o11y`] — Prometheus P50/P99/P999 metrics and percentile helpers.
//! - [`cli`] — Pure-function clap-derived CLI dispatcher (testable in-process).

pub mod o11y;
pub mod cli;

pub use cli::{Cli, CliState, Command, run as cli_run};
pub use o11y::{BenchSamples, XuanjiMetrics, interpolated_percentile, METRIC_BASE_NAMES};
