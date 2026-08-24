//! Task 12 – Rubric Grade Calculator (AIS-grade fusion "S" grade rubric).
//!
//! Provides the 8-dimension weighted score used by Xuanji v2.0 AIS-grade
//! deliveries.  Dimensions and weights (sum to 1.0):
//!
//! | Dimension          | Weight |
//! |--------------------|--------|
//! | algorithm_innovation | 15%  |
//! | biz_coverage         | 15%  |
//! | code_quality         | 10%  |
//! | performance          | 15%  |
//! | reliability          | 15%  |
//! | deployability        | 10%  |
//! | security             | 10%  |
//! | scalability          | 10%  |

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Weights used in the weighted sum, guaranteed to sum to 1.0 (verified in tests).
pub const WEIGHTS: [f64; 8] = [
    0.15, // algorithm_innovation
    0.15, // biz_coverage
    0.10, // code_quality
    0.15, // performance
    0.15, // reliability
    0.10, // deployability
    0.10, // security
    0.10, // scalability
];

const DIM_NAMES: [&str; 8] = [
    "algorithm_innovation",
    "biz_coverage",
    "code_quality",
    "performance",
    "reliability",
    "deployability",
    "security",
    "scalability",
];

/// Score above which a grade letter is awarded.  Grades follow: S >= 90,
/// A >= 80, B >= 70, C >= 60, else D.
pub const GRADE_THRESHOLDS: [(f64, &str); 4] = [
    (90.0, "S"),
    (80.0, "A"),
    (70.0, "B"),
    (60.0, "C"),
];

/// Error returned when a Rubric field falls outside the 0..=100 inclusive range.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RubricError {
    #[error("field {field} out of range 0..=100: got {value}")]
    OutOfRange { field: &'static str, value: u8 },
}

/// 8-dimension AIS-grade delivery rubric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rubric {
    pub algorithm_innovation: u8,
    pub biz_coverage: u8,
    pub code_quality: u8,
    pub performance: u8,
    pub reliability: u8,
    pub deployability: u8,
    pub security: u8,
    pub scalability: u8,
}

impl Rubric {
    /// Return the ordered 8 dimension values as an array.
    pub fn as_array(&self) -> [u8; 8] {
        [
            self.algorithm_innovation,
            self.biz_coverage,
            self.code_quality,
            self.performance,
            self.reliability,
            self.deployability,
            self.security,
            self.scalability,
        ]
    }

    /// Validate that all 8 dimensions are in the inclusive range 0..=100.
    pub fn validate(&self) -> Result<(), String> {
        for (name, &value) in DIM_NAMES.iter().zip(self.as_array().iter()) {
            if value > 100 {
                return Err(format!(
                    "field {name} out of range 0..=100: got {value}"
                ));
            }
        }
        Ok(())
    }

    /// Computed weighted score as `f64` in `[0, 100]`.
    pub fn score(&self) -> f64 {
        self.as_array()
            .iter()
            .zip(WEIGHTS.iter())
            .map(|(&v, &w)| v as f64 * w)
            .sum()
    }

    /// Map the weighted score to a letter grade: S/A/B/C/D.
    pub fn grade(&self) -> String {
        let s = self.score();
        for (thresh, letter) in GRADE_THRESHOLDS.iter() {
            if s >= *thresh {
                return (*letter).to_string();
            }
        }
        "D".to_string()
    }

    /// Construct a Rubric with all dimensions set to 95 — the canonical
    /// delivery baseline producing a grade of "S".
    pub fn default_s() -> Self {
        Self {
            algorithm_innovation: 95,
            biz_coverage: 95,
            code_quality: 95,
            performance: 95,
            reliability: 95,
            deployability: 95,
            security: 95,
            scalability: 95,
        }
    }
}

impl From<[u8; 8]> for Rubric {
    /// Build a Rubric from the ordered 8-dimension array in the same order as
    /// the struct fields (algo..scalability).
    fn from(arr: [u8; 8]) -> Self {
        Self {
            algorithm_innovation: arr[0],
            biz_coverage: arr[1],
            code_quality: arr[2],
            performance: arr[3],
            reliability: arr[4],
            deployability: arr[5],
            security: arr[6],
            scalability: arr[7],
        }
    }
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn weights_sum_exactly_one() {
        let s: f64 = WEIGHTS.iter().sum();
        assert!((s - 1.0).abs() < 1e-9, "weights sum = {s}, expected 1.0");
    }

    #[test]
    fn default_s_gives_s_grade() {
        let r = Rubric::default_s();
        assert_eq!(r.grade(), "S");
    }

    #[test]
    fn validate_rejects_out_of_range() {
        let bad = Rubric::from([101, 50, 50, 50, 50, 50, 50, 50]);
        assert!(bad.validate().is_err());
    }
}
