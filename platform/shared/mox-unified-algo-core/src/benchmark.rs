// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! # 算法性能基准测试工具
//!
//! 提供统一的性能测量接口，用于跨算法、跨版本的性能对比。

use std::time::{Duration, Instant};

/// 基准测试结果
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub algo_name: String,
    pub input_size: usize,
    pub avg_duration_ms: f64,
    pub min_duration_ms: f64,
    pub max_duration_ms: f64,
    pub std_dev_ms: f64,
    pub iterations: usize,
    pub throughput: f64, // 每秒处理量
}

/// 基准测试运行器
pub struct BenchmarkRunner {
    pub warmup_iterations: usize,
    pub measurement_iterations: usize,
}

impl Default for BenchmarkRunner {
    fn default() -> Self {
        Self {
            warmup_iterations: 10,
            measurement_iterations: 100,
        }
    }
}

impl BenchmarkRunner {
    /// 运行基准测试
    pub fn run<F, I, O>(&self, name: &str, input_size: usize, mut f: F, input: &I) -> BenchmarkResult
    where
        F: FnMut(&I) -> O,
    {
        // 预热
        for _ in 0..self.warmup_iterations {
            let _ = f(input);
        }

        // 测量
        let mut durations = Vec::with_capacity(self.measurement_iterations);
        for _ in 0..self.measurement_iterations {
            let start = Instant::now();
            let _ = f(input);
            let dur = start.elapsed();
            durations.push(dur.as_secs_f64() * 1000.0);
        }

        let avg = durations.iter().sum::<f64>() / durations.len() as f64;
        let min = *durations
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let max = *durations
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let variance = durations.iter().map(|d| (d - avg) * (d - avg)).sum::<f64>() / durations.len() as f64;
        let std_dev = variance.sqrt();

        let throughput = if avg > 0.0 {
            input_size as f64 / (avg / 1000.0)
        } else {
            f64::INFINITY
        };

        BenchmarkResult {
            algo_name: name.to_string(),
            input_size,
            avg_duration_ms: avg,
            min_duration_ms: min,
            max_duration_ms: max,
            std_dev_ms: std_dev,
            iterations: self.measurement_iterations,
            throughput,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_runner() {
        let runner = BenchmarkRunner {
            warmup_iterations: 2,
            measurement_iterations: 10,
        };

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = runner.run("test_sum", input.len(), |v: &Vec<f64>| v.iter().sum::<f64>(), &input);

        assert_eq!(result.algo_name, "test_sum");
        assert_eq!(result.input_size, 5);
        assert_eq!(result.iterations, 10);
        assert!(result.avg_duration_ms >= 0.0);
        assert!(result.min_duration_ms <= result.avg_duration_ms);
        assert!(result.max_duration_ms >= result.avg_duration_ms);
        assert!(result.throughput > 0.0);
    }
}
