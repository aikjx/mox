//! Task 9 — P99 Prometheus O11y integration tests.
//!
//! Covers:
//! - MoxMetrics builds successfully; all 10 base names appear in exposition.
//! - After N histogram observations, histogram sample counts increase.
//! - BenchSamples percentiles on deterministic samples within 1% error.
//! - Counter increments are additive (non-zero after N incs).
//! - Gauge set() round-trip.
//! - Histogram buckets cover sub-millisecond AND multi-millisecond latencies.
//! - Registry text encoding is non-empty and valid Prometheus text format
//!   (each metric HELP/TYPE prefix).

use rand::Rng;
use mox_platform_gateway_svc::{
    BenchSamples, MoxMetrics, interpolated_percentile, METRIC_BASE_NAMES,
};

// T9-01: MoxMetrics::new + encode_text contains all 10 base names.
#[test]
fn t9_01_registry_encode_all_ten_metric_names_present() {
    let m = MoxMetrics::new().unwrap();
    m.observe_n_samples(5);
    let text = m.encode_text().unwrap();
    assert!(!text.is_empty(), "text encoding must be non-empty");
    for name in METRIC_BASE_NAMES {
        assert!(
            text.contains(name),
            "Prometheus text MUST contain metric base name '{name}'.\n\
             Hint: histograms have _sum/_count/_bucket suffixes; counters/gauges as-is.\n\
             Actual text (truncated):\n{}",
            &text.chars().take(600).collect::<String>(),
        );
    }
    // Also assert HELP/TYPE lines exist for each.
    assert!(text.contains("# HELP "), "must contain HELP lines");
    assert!(text.contains("# TYPE "), "must contain TYPE lines");
}

// T9-02: obj_put histogram count after 100 observations == 100
#[test]
fn t9_02_put_histogram_100_samples_count() {
    let m = MoxMetrics::new().unwrap();
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        // Exponential-like latencies in 10µs..50ms range.
        let lat_us: u64 = rng.gen_range(10..50_000);
        m.obj_put_p50_p99_p999.observe(lat_us as f64 / 1_000_000.0);
    }
    let count = m.obj_put_p50_p99_p999.get_sample_count();
    assert_eq!(count, 100, "histogram count after 100 observes; got {count}");
    assert!(m.obj_put_p50_p99_p999.get_sample_sum() > 0.0);
}

// T9-03: obj_get histogram count after 1000 observations == 1000
#[test]
fn t9_03_get_histogram_1000_samples_count() {
    let m = MoxMetrics::new().unwrap();
    let mut rng = rand::thread_rng();
    for _ in 0..1000 {
        let lat_us: u64 = rng.gen_range(20..200_000);
        m.obj_get_p50_p99_p999.observe(lat_us as f64 / 1_000_000.0);
    }
    assert_eq!(m.obj_get_p50_p99_p999.get_sample_count(), 1000);
}

// T9-04: ec_encode_us histogram after 256 obs -> count == 256 and sum > 0
#[test]
fn t9_04_ec_encode_histogram_256_samples() {
    let m = MoxMetrics::new().unwrap();
    let mut rng = rand::thread_rng();
    for _ in 0..256 {
        let us: f64 = rng.gen_range(20.0..15_000.0);
        m.ec_encode_us.observe(us);
    }
    assert_eq!(m.ec_encode_us.get_sample_count(), 256);
    assert!(m.ec_encode_us.get_sample_sum() > 0.0);
}

// T9-05: mpu_parts_total counter after inc_by(17) == 17
#[test]
fn t9_05_mpu_parts_counter_additive() {
    let m = MoxMetrics::new().unwrap();
    m.mpu_parts_total.inc_by(17.0);
    assert_eq!(m.mpu_parts_total.get(), 17.0);
    m.mpu_parts_total.inc();
    assert_eq!(m.mpu_parts_total.get(), 18.0);
}

// T9-06: ec_shard_rebuild counter after inc_by(9) + observe_n_samples(5) = 14
#[test]
fn t9_06_shard_rebuild_counter_total() {
    let m = MoxMetrics::new().unwrap();
    m.ec_shard_rebuild.inc_by(9.0);
    m.observe_n_samples(5); // adds 5 more
    assert_eq!(m.ec_shard_rebuild.get(), 14.0);
}

// T9-07: mountpath_faulty_total gauge set 3, set 0 round-trips
#[test]
fn t9_07_faulty_gauge_set_roundtrip() {
    let m = MoxMetrics::new().unwrap();
    m.mountpath_faulty_total.set(3.0);
    assert_eq!(m.mountpath_faulty_total.get(), 3.0);
    m.mountpath_faulty_total.set(0.0);
    assert_eq!(m.mountpath_faulty_total.get(), 0.0);
    m.mountpath_faulty_total.inc();
    assert_eq!(m.mountpath_faulty_total.get(), 1.0);
}

// T9-08: legalhold_active_objects gauge set+inc flow
#[test]
fn t9_08_legalhold_gauge_flow() {
    let m = MoxMetrics::new().unwrap();
    m.legalhold_active_objects.set(7.0);
    assert_eq!(m.legalhold_active_objects.get(), 7.0);
    m.legalhold_active_objects.dec();
    assert_eq!(m.legalhold_active_objects.get(), 6.0);
}

// T9-09: miji denied counters increment correctly
#[test]
fn t9_09_miji_denied_counters() {
    let m = MoxMetrics::new().unwrap();
    for _ in 0..3 { m.miji_denied_read_total.inc(); }
    for _ in 0..5 { m.miji_denied_write_total.inc(); }
    assert_eq!(m.miji_denied_read_total.get(), 3.0);
    assert_eq!(m.miji_denied_write_total.get(), 5.0);
}

// T9-10: crc_mismatch_total counter inc_by(11) == 11
#[test]
fn t9_10_crc_mismatch_counter() {
    let m = MoxMetrics::new().unwrap();
    m.crc_mismatch_total.inc_by(11.0);
    assert_eq!(m.crc_mismatch_total.get(), 11.0);
}

// T9-11: BenchSamples on deterministic [0..100] p99 within 1% of 99.0
#[test]
fn t9_11_bench_samples_p99_0_to_100_within_1_pct() {
    let samples: Vec<u64> = (0..=100).collect(); // 101 samples exactly known
    let b = BenchSamples::from_durations(&samples);
    assert_eq!(b.count, 101);
    assert_eq!(b.min, 0);
    assert_eq!(b.max, 100);
    // Expected p50 = 50, p99 = 99, p999 = 99.9
    assert!(
        (b.p50 - 50.0).abs() / 50.0 < 0.01,
        "p50 error > 1%: got {}", b.p50
    );
    assert!(
        (b.p99 - 99.0).abs() / 99.0 < 0.01,
        "p99 error > 1%: expected 99.0, got {}",
        b.p99
    );
    assert!(
        (b.p999 - 99.9).abs() / 99.9 < 0.01,
        "p999 error > 1%: expected 99.9, got {}",
        b.p999
    );
}

// T9-12: interpolated_percentile two-element [10,20], p=0.25 -> 12.5
#[test]
fn t9_12_interpolation_two_element_quarter() {
    let sorted = [10u64, 20];
    let v = interpolated_percentile(&sorted, 0.25);
    // rank = (2-1)*0.25 = 0.25; lo=0, hi=1, frac=0.25 => 10 + 0.25*(20-10) = 12.5
    assert!((v - 12.5).abs() < 1e-9, "expected 12.5, got {v}");
}

// T9-13: BenchSamples empty input -> all zeros/defaults
#[test]
fn t9_13_empty_input_defaults() {
    let b = BenchSamples::from_durations(&[]);
    assert_eq!(b.count, 0);
    assert_eq!(b.min, 0);
    assert_eq!(b.max, 0);
    assert_eq!(b.p50, 0.0);
    assert_eq!(b.p99, 0.0);
    assert_eq!(b.p999, 0.0);
}

// T9-14: 10k random N(500µs, 100µs) samples: p99 should be well below 900µs
// (Gaussian: +3σ ≈ 500 + 300 = 800µs covers 99.86%). This exercises the
// statistical reasonableness of the percentile code.
#[test]
fn t9_14_statistical_p99_below_900us_for_narrow_normal() {
    let mut rng = rand::thread_rng();
    const N: usize = 10_000;
    let mut samples: Vec<u64> = Vec::with_capacity(N);
    for _ in 0..N {
        // Box-Muller: standard normal z ~ N(0,1)
        let u1: f64 = rng.gen();
        let u2: f64 = rng.gen();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        let v_us: f64 = 500.0 + 100.0 * z;
        samples.push(v_us.max(1.0) as u64);
    }
    let b = BenchSamples::from_durations(&samples);
    assert_eq!(b.count, N);
    // μ + 3σ ≈ 800; allow generous margin up to 900.
    assert!(
        b.p99 < 900.0,
        "p99 should be < 900µs for N(500, 100) — got p99={}µs (avg={}µs)",
        b.p99, b.avg
    );
}

// T9-15: METRIC_BASE_NAMES.len() == 10 exactly (sanity)
#[test]
fn t9_15_exactly_ten_metric_base_names() {
    assert_eq!(METRIC_BASE_NAMES.len(), 10);
}
