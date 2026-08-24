//! Prometheus observability metrics and percentile computation helpers.
//!
//! Provides the Xuanji v2.0 AIS-grade O11y surface:
//! - P50/P99/P999 latency histograms for object PUT and GET
//! - EC encode timing histogram
//! - Counters / gauges for MPU parts, shard rebuilds,
//!   faulty mountpaths, legal-hold objects, MiJi denied ops, CRC mismatches.
//!
//! Also exports `BenchSamples` which can compute percentiles (p50/p99/p999)
//! from a `&[u64]` of duration samples (unit agnostic — usually microseconds).

use prometheus::{
    register_counter_with_registry, register_gauge_with_registry,
    register_histogram_with_registry, Counter, Gauge, Histogram,
    HistogramOpts, Registry, TextEncoder, Encoder,
};

/// Histogram bucket configuration for latencies in the 0..1s range.
/// Log-ish spacing: 10µs..1s with ~18 buckets, good enough to cover
/// fast memory paths up through slow HDD tail latencies.
#[rustfmt::skip]
pub const LATENCY_SEC_BUCKETS: &[f64] = &[
    0.000_010, 0.000_025, 0.000_050, 0.000_100,
    0.000_250, 0.000_500, 0.001,     0.002_5,
    0.005,     0.010,     0.025,     0.050,
    0.100,     0.250,     0.500,     1.0,
];

/// Histogram buckets for EC encode microseconds — wider range since RS(n+k)
/// over large stripes can run into milliseconds.
#[rustfmt::skip]
pub const EC_ENCODE_US_BUCKETS: &[f64] = &[
    10.0,    25.0,     50.0,     100.0,
    250.0,   500.0,    1_000.0,  2_500.0,
    5_000.0, 10_000.0, 25_000.0, 50_000.0,
    100_000.0,
];

/// Typed Prometheus metric registry for Xuanji v2.0.
#[derive(Debug)]
pub struct XuanjiMetrics {
    pub registry: Registry,
    /// PUT object request latency (seconds). Bucketed 0..1s.
    pub obj_put_p50_p99_p999: Histogram,
    /// GET object request latency (seconds). Bucketed 0..1s.
    pub obj_get_p50_p99_p999: Histogram,
    /// EC encode time per stripe (microseconds).
    pub ec_encode_us: Histogram,
    /// Total multipart-upload parts uploaded across all MPU sessions.
    pub mpu_parts_total: Counter,
    /// Total EC shard-rebuild operations executed.
    pub ec_shard_rebuild: Counter,
    /// Number of mountpaths currently marked Faulty.
    pub mountpath_faulty_total: Gauge,
    /// Number of objects that currently have an active LegalHold.
    pub legalhold_active_objects: Gauge,
    /// Total MiJi read accesses denied (Bell-LaPadula read-up rule).
    pub miji_denied_read_total: Counter,
    /// Total MiJi write accesses denied (Bell-LaPadula *-property write-down).
    pub miji_denied_write_total: Counter,
    /// Total CRC mismatches detected on read-after-write or rebuild paths.
    pub crc_mismatch_total: Counter,
}

impl Default for XuanjiMetrics {
    fn default() -> Self {
        Self::new().expect("XuanjiMetrics::new default registry build must succeed")
    }
}

impl XuanjiMetrics {
    /// Build a new registry with all 10 Xuanji metrics registered.
    pub fn new() -> prometheus::Result<Self> {
        let registry = Registry::new();

        let obj_put_p50_p99_p999 = register_histogram_with_registry!(
            HistogramOpts::new(
                "xuanji_obj_put_p50_p99_p999",
                "PUT object end-to-end latency in seconds (P50/P99/P999 via histogram buckets).",
            )
            .buckets(LATENCY_SEC_BUCKETS.to_vec()),
            &registry,
        )?;

        let obj_get_p50_p99_p999 = register_histogram_with_registry!(
            HistogramOpts::new(
                "xuanji_obj_get_p50_p99_p999",
                "GET object end-to-end latency in seconds.",
            )
            .buckets(LATENCY_SEC_BUCKETS.to_vec()),
            &registry,
        )?;

        let ec_encode_us = register_histogram_with_registry!(
            HistogramOpts::new(
                "xuanji_ec_encode_us",
                "Per-stripe EC encode elapsed time in microseconds.",
            )
            .buckets(EC_ENCODE_US_BUCKETS.to_vec()),
            &registry,
        )?;

        let mpu_parts_total = register_counter_with_registry!(
            "xuanji_mpu_parts_total",
            "Total multipart-upload parts accepted across all sessions.",
            &registry,
        )?;

        let ec_shard_rebuild = register_counter_with_registry!(
            "xuanji_ec_shard_rebuild_total",
            "Total EC shard-rebuild operations executed.",
            &registry,
        )?;

        let mountpath_faulty_total = register_gauge_with_registry!(
            "xuanji_mountpath_faulty_total",
            "Number of mountpaths currently in Faulty state.",
            &registry,
        )?;

        let legalhold_active_objects = register_gauge_with_registry!(
            "xuanji_legalhold_active_objects",
            "Number of objects currently under an active LegalHold.",
            &registry,
        )?;

        let miji_denied_read_total = register_counter_with_registry!(
            "xuanji_miji_denied_read_total",
            "MiJi Bell-LaPadula simple-security read-up denies.",
            &registry,
        )?;

        let miji_denied_write_total = register_counter_with_registry!(
            "xuanji_miji_denied_write_total",
            "MiJi Bell-LaPadula star-property write-down denies.",
            &registry,
        )?;

        let crc_mismatch_total = register_counter_with_registry!(
            "xuanji_crc_mismatch_total",
            "Total CRC mismatches detected on read, rebuild, or replica-verification paths.",
            &registry,
        )?;

        Ok(Self {
            registry,
            obj_put_p50_p99_p999,
            obj_get_p50_p99_p999,
            ec_encode_us,
            mpu_parts_total,
            ec_shard_rebuild,
            mountpath_faulty_total,
            legalhold_active_objects,
            miji_denied_read_total,
            miji_denied_write_total,
            crc_mismatch_total,
        })
    }

    /// Encode the registry state as Prometheus text-format exposition.
    pub fn encode_text(&self) -> prometheus::Result<String> {
        let mut buf = Vec::new();
        let encoder = TextEncoder::new();
        let metrics = self.registry.gather();
        encoder.encode(&metrics, &mut buf)?;
        String::from_utf8(buf).map_err(|e| prometheus::Error::Msg(format!("utf8: {e}")))
    }

    /// Observe an already-sorted sample set against all 10 metrics to quickly
    /// exercise each counter/histogram/gauge for unit tests.
    ///
    /// Histogram/put: N× 0.5ms samples.
    /// Histogram/get: N× 1.0ms samples.
    /// EC encode: N× 500µs samples.
    /// Counters: mpu_parts + N, ec_shard_rebuild + N, denied_r + N, denied_w + N, crc + N.
    /// Gauges: mountpath faulty = 2, legalhold active = 5.
    pub fn observe_n_samples(&self, n: u64) {
        for _ in 0..n {
            self.obj_put_p50_p99_p999.observe(0.000_5);
            self.obj_get_p50_p99_p999.observe(0.001_0);
            self.ec_encode_us.observe(500.0);
            self.mpu_parts_total.inc();
            self.ec_shard_rebuild.inc();
            self.miji_denied_read_total.inc();
            self.miji_denied_write_total.inc();
            self.crc_mismatch_total.inc();
        }
        self.mountpath_faulty_total.set(2.0);
        self.legalhold_active_objects.set(5.0);
    }
}

/// Convenience helper for benchmarks: given a slice of raw duration samples
/// (e.g. microseconds), sort them once and then compute p50/p99/p999 using
/// linear interpolation between adjacent sorted samples — the standard
/// "percentile with interpolation" rule used by Prometheus histograms too.
///
/// Unit-agnostic: the output uses the same unit as the inputs.
#[derive(Debug, Clone, Default)]
pub struct BenchSamples {
    pub count: usize,
    pub p50: f64,
    pub p99: f64,
    pub p999: f64,
    pub min: u64,
    pub max: u64,
    pub avg: f64,
}

impl BenchSamples {
    /// Compute percentiles over `samples`. The unit (µs / ms / ns) doesn't
    /// matter to the computation — the output keeps the same unit.
    pub fn from_durations(samples: &[u64]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }
        let mut sorted = samples.to_vec();
        sorted.sort_unstable();
        let count = sorted.len();
        let min = sorted[0];
        let max = sorted[sorted.len() - 1];
        let sum: u64 = sorted.iter().sum();
        let avg = sum as f64 / count as f64;

        let p50 = interpolated_percentile(&sorted, 0.50);
        let p99 = interpolated_percentile(&sorted, 0.99);
        let p999 = interpolated_percentile(&sorted, 0.999);

        Self { count, p50, p99, p999, min, max, avg }
    }
}

/// Linear-interpolation percentile. `sorted` must be sorted ascending.
///
/// Rank = (n - 1) * p; floor rank + fractional remainder; if rank is an
/// integer the result is the exact sample; otherwise we linearly blend
/// between the two nearest samples.
pub fn interpolated_percentile(sorted: &[u64], p: f64) -> f64 {
    assert!(!sorted.is_empty(), "interpolated_percentile requires non-empty slice");
    assert!((0.0..=1.0).contains(&p), "percentile p must be in [0,1]");
    let n = sorted.len();
    if n == 1 {
        return sorted[0] as f64;
    }
    let rank: f64 = (n - 1) as f64 * p;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    if lo == hi {
        return sorted[lo] as f64;
    }
    let frac = rank - lo as f64;
    let a = sorted[lo] as f64;
    let b = sorted[hi] as f64;
    a + (b - a) * frac
}

/// Canonical list of the 10 Xuanji metric **base names** (without the
/// `_sum`/`_count`/`_bucket` suffixes that Prometheus appends to
/// histograms, or the labels). Useful for tests verifying exposition text.
pub const METRIC_BASE_NAMES: &[&str] = &[
    "xuanji_obj_put_p50_p99_p999",
    "xuanji_obj_get_p50_p99_p999",
    "xuanji_ec_encode_us",
    "xuanji_mpu_parts_total",
    "xuanji_ec_shard_rebuild_total",
    "xuanji_mountpath_faulty_total",
    "xuanji_legalhold_active_objects",
    "xuanji_miji_denied_read_total",
    "xuanji_miji_denied_write_total",
    "xuanji_crc_mismatch_total",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_build_registers_all_ten() {
        let m = XuanjiMetrics::new().unwrap();
        let text = m.encode_text().unwrap();
        for name in METRIC_BASE_NAMES {
            // Histograms have _sum/_count/_bucket variants so the base name
            // will appear multiple times; counters/gauges appear as-is.
            assert!(
                text.contains(name),
                "metric exposition must contain base name '{name}'. Got:\n{text}"
            );
        }
    }

    #[test]
    fn percentile_100_identical_samples_gives_same_value() {
        let s = vec![42u64; 100];
        let r = BenchSamples::from_durations(&s);
        assert_eq!(r.count, 100);
        assert_eq!(r.min, 42);
        assert_eq!(r.max, 42);
        assert_eq!(r.avg, 42.0);
        assert!((r.p50 - 42.0).abs() < 1e-9);
        assert!((r.p99 - 42.0).abs() < 1e-9);
        assert!((r.p999 - 42.0).abs() < 1e-9);
    }

    #[test]
    fn percentile_linear_0_to_100_p99_equals_99() {
        // 101 samples: 0,1,2,...,100 — exactly known percentiles.
        let s: Vec<u64> = (0..=100).collect();
        let r = BenchSamples::from_durations(&s);
        // rank_p50 = 100 * 0.5 = 50 exactly -> sample[50] = 50
        assert!((r.p50 - 50.0).abs() < 1e-9);
        // rank_p99 = 100 * 0.99 = 99 exactly -> sample[99] = 99
        assert!((r.p99 - 99.0).abs() < 1e-9);
        // rank_p999 = 100 * 0.999 = 99.9 -> interpolate between 99 and 100
        // expected = 99 + 0.9 * (100 - 99) = 99.9
        assert!((r.p999 - 99.9).abs() < 1e-9, "p999 got {}", r.p999);
    }
}
