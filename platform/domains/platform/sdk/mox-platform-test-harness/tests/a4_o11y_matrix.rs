// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! A4 — O11y Prometheus + percentile matrix (40 tests)
//!
//! 10 metric names × exposure assert = 10
//! BenchSamples percentiles × 10 cases = 10
//! observe_n_samples monotonicity × 5 N = 5
//! Percentile interpolation edge cases × 15 = 15

use mox_platform_test_harness::o11y::{BenchSamples, MoxMetrics, interpolated_percentile, METRIC_BASE_NAMES};

// 10 metric exposure tests
macro_rules! metric_contains {
    ($name:ident, $idx:expr) => {
        #[test]
        fn $name() {
            let m = MoxMetrics::new().unwrap();
            let text = m.encode_text().unwrap();
            let name = METRIC_BASE_NAMES[$idx];
            assert!(text.contains(name), "missing metric {name}");
        }
    };
}

metric_contains!(a4_metric_00_obj_put, 0);
metric_contains!(a4_metric_01_obj_get, 1);
metric_contains!(a4_metric_02_ec_encode, 2);
metric_contains!(a4_metric_03_mpu_parts, 3);
metric_contains!(a4_metric_04_ec_rebuild, 4);
metric_contains!(a4_metric_05_mountpath, 5);
metric_contains!(a4_metric_06_legalhold, 6);
metric_contains!(a4_metric_07_denied_r, 7);
metric_contains!(a4_metric_08_denied_w, 8);
metric_contains!(a4_metric_09_crc, 9);

// BenchSamples: 10 cases with various distributions
#[test] fn a4_bench_01_single_sample() {
    let r = BenchSamples::from_durations(&[7]);
    assert_eq!(r.count, 1); assert_eq!(r.min, 7); assert_eq!(r.max, 7);
    assert!((r.p50 - 7.0).abs() < 1e-9);
}
#[test] fn a4_bench_02_two_samples_1_9() {
    let r = BenchSamples::from_durations(&[1, 9]);
    assert_eq!(r.p50, 5.0); // (9-1)*0.5 + 1 = 5.0
}
#[test] fn a4_bench_03_two_samples_0_100_p99() {
    let r = BenchSamples::from_durations(&[0, 100]);
    assert!((r.p99 - 99.0).abs() < 1e-9);
}
#[test] fn a4_bench_04_1000_identical_555() {
    let v = vec![555u64; 1000];
    let r = BenchSamples::from_durations(&v);
    assert!((r.p50 - 555.0).abs() < 1e-9);
    assert!((r.p99 - 555.0).abs() < 1e-9);
    assert!((r.p999 - 555.0).abs() < 1e-9);
}
#[test] fn a4_bench_05_range_0_1000_count() {
    let v: Vec<u64> = (0..=1000).collect();
    let r = BenchSamples::from_durations(&v);
    assert_eq!(r.count, 1001);
    assert_eq!(r.min, 0); assert_eq!(r.max, 1000);
    assert!((r.p50 - 500.0).abs() < 1e-9);
    assert!((r.p99 - 990.0).abs() < 1e-9);
}
#[test] fn a4_bench_06_unsorted_input() {
    let v = [9u64, 1, 7, 3, 5];
    let r = BenchSamples::from_durations(&v);
    assert_eq!(r.min, 1); assert_eq!(r.max, 9);
    assert!((r.p50 - 5.0).abs() < 1e-9);
}
#[test] fn a4_bench_07_avg_of_0_to_99() {
    let v: Vec<u64> = (0..100).collect();
    let r = BenchSamples::from_durations(&v);
    // sum = 4950, n=100, avg=49.5
    assert!((r.avg - 49.5).abs() < 1e-9);
}
#[test] fn a4_bench_08_empty_default() {
    let r = BenchSamples::from_durations(&[]);
    assert_eq!(r.count, 0); assert_eq!(r.min, 0); assert_eq!(r.max, 0);
    assert_eq!(r.p50, 0.0);
}
#[test] fn a4_bench_09_three_1_2_3() {
    let r = BenchSamples::from_durations(&[1, 2, 3]);
    assert!((r.p50 - 2.0).abs() < 1e-9);
}
#[test] fn a4_bench_10_1001_elements_p999_is_999_9() {
    let v: Vec<u64> = (0..=1000).collect();
    let r = BenchSamples::from_durations(&v);
    // rank = 1000 * 0.999 = 999.0 exactly -> v[999] = 999
    assert!((r.p999 - 999.0).abs() < 1e-9, "p999={}", r.p999);
}

// observe_n_samples monotonicity: 5 cases
fn check_observe_n(n: u64) {
    let m = MoxMetrics::new().unwrap();
    m.observe_n_samples(n);
    let text = m.encode_text().unwrap();
    // Simple assertion: exposition contains expected gauge lines and counter
    // values (formatted as "metric_name value").
    assert!(text.len() > 500, "exposition must be large enough");
    assert!(text.contains(&format!("mox_mountpath_faulty_total 2")));
    assert!(text.contains(&format!("mox_legalhold_active_objects 5")));
    // All counters multiplied by n
    let expect = n.to_string();
    for (i, name) in ["mox_mpu_parts_total", "mox_ec_shard_rebuild_total",
                      "mox_miji_denied_read_total", "mox_miji_denied_write_total",
                      "mox_crc_mismatch_total"].iter().enumerate() {
        let needle = format!("{name} {expect}");
        assert!(text.contains(&needle),
                "case n={} i={}: missing '{needle}' in exposition (check counter)",
                n, i);
    }
}

#[test] fn a4_obs_01_n0() { check_observe_n(0); }
#[test] fn a4_obs_02_n1() { check_observe_n(1); }
#[test] fn a4_obs_03_n5() { check_observe_n(5); }
#[test] fn a4_obs_04_n10() { check_observe_n(10); }
#[test] fn a4_obs_05_n100() { check_observe_n(100); }

// Percentile interpolation edge cases: 15
macro_rules! pctl_case {
    ($name:ident, $samples:expr, $p:expr, $expect:expr) => {
        #[test]
        fn $name() {
            let s: Vec<u64> = $samples;
            let r = interpolated_percentile(&s, $p);
            assert!((r - $expect).abs() < 1e-9, "got {r}, expect {}", $expect);
        }
    };
}

pctl_case!(a4_p_01_min_1elem, vec![100], 0.0, 100.0);
pctl_case!(a4_p_02_max_1elem, vec![100], 1.0, 100.0);
pctl_case!(a4_p_03_min_101, (0..=100).collect(), 0.0, 0.0);
pctl_case!(a4_p_04_max_101, (0..=100).collect(), 1.0, 100.0);
pctl_case!(a4_p_05_p25_101, (0..=100).collect(), 0.25, 25.0);
pctl_case!(a4_p_06_p75_101, (0..=100).collect(), 0.75, 75.0);
pctl_case!(a4_p_07_p001_1001, (0..=1000).collect(), 0.001, 1.0);
pctl_case!(a4_p_08_fraction_0_100_half_point, vec![0,100], 0.5, 50.0);
pctl_case!(a4_p_09_fraction_0_100_quarter, vec![0,100], 0.25, 25.0);
pctl_case!(a4_p_10_fraction_0_100_three_quarter, vec![0,100], 0.75, 75.0);
pctl_case!(a4_p_11_3elem_0_10_20_p50, vec![0,10,20], 0.5, 10.0);
pctl_case!(a4_p_12_3elem_0_10_20_p25, vec![0,10,20], 0.25, 5.0);
pctl_case!(a4_p_13_3elem_0_10_20_p75, vec![0,10,20], 0.75, 15.0);
pctl_case!(a4_p_14_5elem_0_2_4_6_8_p30, vec![0,2,4,6,8], 0.3, 2.4);
pctl_case!(a4_p_15_5elem_0_2_4_6_8_p90, vec![0,2,4,6,8], 0.9, 7.2);
