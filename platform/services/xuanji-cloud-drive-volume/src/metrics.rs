//! EC engine fake metrics registry (no prometheus dependency).
//!
//! Exposes three simulated metrics:
//! - `xuanji_ec_rebuild_count` (counter) → `REBUILD_COUNT: AtomicU64`
//! - `xuanji_ec_shards_lost_total` (counter) → `SHARDS_LOST_TOTAL: AtomicU64`
//! - `xuanji_ec_encode_us` (histogram) → samples stored in a lock-free
//!   append-only `Vec<u64>` (bounded by `MAX_HISTOGRAM_SAMPLES`) plus
//!   a global atomic counter for every observe call.

use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// Samples ring capacity for the simulated histogram.  Older samples are
/// dropped on overflow so a buggy test cannot balloon memory.
pub const MAX_HISTOGRAM_SAMPLES: usize = 1 << 16;

/// Counter: `xuanji_ec_rebuild_count` – every successful rebuild job bumps.
pub static REBUILD_COUNT: AtomicU64 = AtomicU64::new(0);

/// Counter: `xuanji_ec_shards_lost_total` – every detected missing shard.
pub static SHARDS_LOST_TOTAL: AtomicU64 = AtomicU64::new(0);

/// Histogram sample counter: total number of `observe_encode_us()` calls.
pub static ENCODE_US_COUNT: AtomicU64 = AtomicU64::new(0);

/// Ring buffer of the last `MAX_HISTOGRAM_SAMPLES` encode latency samples.
static ENCODE_US_SAMPLES: Mutex<Vec<u64>> = Mutex::new(Vec::new());

/// Push a latency sample (microseconds) for `xuanji_ec_encode_us`.
pub fn observe_encode_us(micros: u64) {
    ENCODE_US_COUNT.fetch_add(1, Ordering::Relaxed);
    let mut guard = ENCODE_US_SAMPLES.lock();
    if guard.len() >= MAX_HISTOGRAM_SAMPLES {
        // drop oldest (rough ring buffer)
        let drop = MAX_HISTOGRAM_SAMPLES / 2;
        guard.drain(0..drop);
    }
    guard.push(micros);
}

/// Snapshot of current histogram samples (drained copy).
pub fn encode_us_samples_snapshot() -> Vec<u64> {
    ENCODE_US_SAMPLES.lock().clone()
}

/// Clears all counters / samples (useful to make tests hermetic).
pub fn reset_all() {
    REBUILD_COUNT.store(0, Ordering::SeqCst);
    SHARDS_LOST_TOTAL.store(0, Ordering::SeqCst);
    ENCODE_US_COUNT.store(0, Ordering::SeqCst);
    ENCODE_US_SAMPLES.lock().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_samples_and_reset() {
        reset_all();
        observe_encode_us(12);
        observe_encode_us(34);
        assert_eq!(ENCODE_US_COUNT.load(Ordering::SeqCst), 2);
        assert_eq!(encode_us_samples_snapshot(), vec![12, 34]);
        REBUILD_COUNT.fetch_add(7, Ordering::SeqCst);
        SHARDS_LOST_TOTAL.fetch_add(1, Ordering::SeqCst);
        reset_all();
        assert_eq!(ENCODE_US_COUNT.load(Ordering::SeqCst), 0);
        assert_eq!(REBUILD_COUNT.load(Ordering::SeqCst), 0);
        assert_eq!(SHARDS_LOST_TOTAL.load(Ordering::SeqCst), 0);
        assert!(encode_us_samples_snapshot().is_empty());
    }
}
