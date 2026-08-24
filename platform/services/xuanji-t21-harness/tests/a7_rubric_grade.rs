//! A7 – Rubric grade calculator (12 individual tests).
//!
//! Covers: default_s → S, exactly 90 → S, 89 → A, single 0 → D, invalid
//! validation error, weights sum == 1.0, From<[u8;8]> roundtrip, and several
//! edge case grade boundaries.

use xuanji_t21_harness::rubric::{Rubric, WEIGHTS};

#[test]
fn a7_r01_default_s_grade_s() {
    let r = Rubric::default_s();
    assert_eq!(r.grade(), "S");
}

#[test]
fn a7_r02_exactly_all_90_is_s() {
    let r = Rubric::from([90, 90, 90, 90, 90, 90, 90, 90]);
    assert_eq!(r.score(), 90.0);
    assert_eq!(r.grade(), "S");
}

#[test]
fn a7_r03_all_89_floors_to_a() {
    // Weighted sum for all 89 = 89.0 → < 90 → "A"
    let r = Rubric::from([89, 89, 89, 89, 89, 89, 89, 89]);
    assert!((r.score() - 89.0).abs() < 1e-9);
    assert_eq!(r.grade(), "A");
}

#[test]
fn a7_r04_single_zero_gives_d() {
    // set all 8 dims to 0 → score 0.0, definitely grade D
    let r = Rubric::from([0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(r.score(), 0.0);
    assert_eq!(r.grade(), "D");
}

#[test]
fn a7_r05_invalid_101_validate_err() {
    let r = Rubric::from([101, 50, 50, 50, 50, 50, 50, 50]);
    let res = r.validate();
    assert!(res.is_err());
    let msg = res.unwrap_err();
    assert!(msg.contains("algorithm_innovation"));
    assert!(msg.contains("101"));
}

#[test]
fn a7_r06_weights_sum_is_one() {
    let s: f64 = WEIGHTS.iter().sum();
    assert!((s - 1.0).abs() < 1e-9, "sum = {s}, expected 1.0");
}

#[test]
fn a7_r07_from_array_roundtrip_as_array() {
    let arr: [u8; 8] = [10, 20, 30, 40, 50, 60, 70, 80];
    let r = Rubric::from(arr);
    assert_eq!(r.as_array(), arr);
    assert_eq!(r.algorithm_innovation, 10);
    assert_eq!(r.scalability, 80);
}

#[test]
fn a7_r08_grade_boundary_c_exactly_60() {
    // score exactly 60 → "C"
    let r = Rubric::from([60, 60, 60, 60, 60, 60, 60, 60]);
    assert_eq!(r.score(), 60.0);
    assert_eq!(r.grade(), "C");
}

#[test]
fn a7_r09_grade_boundary_b_exactly_70() {
    let r = Rubric::from([70, 70, 70, 70, 70, 70, 70, 70]);
    assert_eq!(r.score(), 70.0);
    assert_eq!(r.grade(), "B");
}

#[test]
fn a7_r10_grade_boundary_59_is_d() {
    // Uniform 59 → score 59.0 → "D" (below C threshold 60)
    let r = Rubric::from([59, 59, 59, 59, 59, 59, 59, 59]);
    assert_eq!(r.score(), 59.0);
    assert_eq!(r.grade(), "D");
}

#[test]
fn a7_r11_validate_all_100_ok_and_grade_s() {
    let r = Rubric::from([100, 100, 100, 100, 100, 100, 100, 100]);
    assert!(r.validate().is_ok());
    assert_eq!(r.score(), 100.0);
    assert_eq!(r.grade(), "S");
}

#[test]
fn a7_r12_validate_zero_is_ok_score_below_60_grade_d() {
    let r = Rubric::from([0, 0, 0, 0, 0, 0, 0, 0]);
    assert!(r.validate().is_ok());
    assert_eq!(r.score(), 0.0);
    assert_eq!(r.grade(), "D");
}
