use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

#[derive(Default)]
pub struct Metrics {
    pub total_calls: AtomicU64,
    pub failed_calls: AtomicU64,
    pub latencies_ns: Mutex<Vec<u64>>,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total(&self) -> u64 {
        self.total_calls.load(Ordering::Relaxed)
    }

    pub fn failed(&self) -> u64 {
        self.failed_calls.load(Ordering::Relaxed)
    }

    pub fn fail_rate(&self) -> f64 {
        let t = self.total();
        if t == 0 {
            return 0.0;
        }
        self.failed() as f64 / t as f64
    }

    fn sorted_latencies(&self) -> Vec<u64> {
        let guard = match self.latencies_ns.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        let mut v = guard.clone();
        drop(guard);
        v.sort_unstable();
        v
    }

    pub fn percentile(&self, p: f64) -> Option<u64> {
        let v = self.sorted_latencies();
        if v.is_empty() { return None; }
        let idx = ((v.len() as f64 - 1.0) * p).round() as usize;
        Some(v[idx])
    }

    pub fn p50(&self) -> Option<u64> { self.percentile(0.5) }
    pub fn p90(&self) -> Option<u64> { self.percentile(0.9) }
    pub fn p99(&self) -> Option<u64> { self.percentile(0.99) }
}
