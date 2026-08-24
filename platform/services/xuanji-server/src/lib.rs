//! Xuanji v2.0 AIS-grade fusion single-binary server library.
//!
//! Exports:
//! - [`o11y`] — Prometheus P50/P99/P999 metrics and percentile helpers.
//! - [`cli`] — Pure-function clap-derived CLI dispatcher (testable in-process).
//! - [`http_server`] — Single-node HTTP server (S3 + Graph + Metrics + Audit).

pub mod o11y;
pub mod cli;
pub mod http_server;

pub use cli::{Cli, CliState, Command, run as cli_run, ServerArgs};
pub use o11y::{BenchSamples, XuanjiMetrics, interpolated_percentile, METRIC_BASE_NAMES};
pub use http_server::{ServerState, serve_forever};
