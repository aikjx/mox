// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 纯向量运算抽象 + KernelStateVector（Vec<f64> 实现）。
//!
//! 通过 VectorOps trait 实现依赖倒置：上层 nalgebra::DVector 或任何向量实现均可 impl。

use crate::kernel::types::KernelError;

/// 纯向量运算接口（DIP 抽象）。上层 nalgebra::DVector 或任何向量实现均可 impl。
pub trait VectorOps {
    fn dimension(&self) -> usize;
    fn as_slice(&self) -> &[f64];

    #[inline]
    fn norm_l2(&self) -> f64 {
        let s: f64 = self.as_slice().iter().map(|x| x * x).sum();
        s.sqrt()
    }

    #[inline]
    fn norm_l1(&self) -> f64 {
        self.as_slice().iter().map(|x| x.abs()).sum()
    }

    #[inline]
    fn sum(&self) -> f64 {
        self.as_slice().iter().sum()
    }
}

/// 纯内核状态向量：完全基于 `Vec<f64>`，不依赖 nalgebra。
#[derive(Debug, Clone)]
pub struct KernelStateVector {
    pub data: Vec<f64>,
    pub timestamp: u64,
}

impl KernelStateVector {
    pub fn new(dimension: usize) -> Self {
        Self {
            data: vec![0.0; dimension],
            timestamp: default_timestamp_ms(),
        }
    }

    pub fn from_vec(data: Vec<f64>) -> Self {
        Self {
            data,
            timestamp: default_timestamp_ms(),
        }
    }

    pub fn zeros(dimension: usize) -> Self {
        Self::new(dimension)
    }

    pub fn unit(dimension: usize) -> Self {
        let val = if dimension > 0 {
            1.0 / dimension as f64
        } else {
            0.0
        };
        Self {
            data: vec![val; dimension],
            timestamp: 0,
        }
    }

    #[inline]
    pub fn dimension(&self) -> usize {
        self.data.len()
    }

    pub fn get(&self, idx: usize) -> Option<f64> {
        self.data.get(idx).copied()
    }

    pub fn set(&mut self, idx: usize, value: f64) -> Result<(), KernelError> {
        if idx >= self.data.len() {
            return Err(KernelError::IndexOutOfBounds {
                idx,
                len: self.data.len(),
            });
        }
        self.data[idx] = value;
        Ok(())
    }

    pub fn add(&self, other: &Self) -> Result<Self, KernelError> {
        if self.dimension() != other.dimension() {
            return Err(KernelError::DimensionMismatch {
                a: self.dimension(),
                b: other.dimension(),
            });
        }
        let data = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| a + b)
            .collect();
        Ok(Self {
            data,
            timestamp: default_timestamp_ms(),
        })
    }

    pub fn scale(&self, scalar: f64) -> Self {
        Self {
            data: self.data.iter().map(|x| x * scalar).collect(),
            timestamp: default_timestamp_ms(),
        }
    }

    pub fn dot(&self, other: &Self) -> Result<f64, KernelError> {
        if self.dimension() != other.dimension() {
            return Err(KernelError::DimensionMismatch {
                a: self.dimension(),
                b: other.dimension(),
            });
        }
        Ok(self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| a * b)
            .sum())
    }

    pub fn normalize(&mut self) {
        let n = self.norm_l2();
        if n > 1e-15 {
            for x in &mut self.data {
                *x /= n;
            }
        }
    }

    pub fn normalize_probability(&mut self) {
        let s = self.norm_l1();
        if s > 1e-15 {
            for x in &mut self.data {
                *x /= s;
            }
        }
    }

    pub fn residual(&self, expected: &Self) -> Result<f64, KernelError> {
        if self.dimension() != expected.dimension() {
            return Err(KernelError::DimensionMismatch {
                a: self.dimension(),
                b: expected.dimension(),
            });
        }
        let s: f64 = self
            .data
            .iter()
            .zip(expected.data.iter())
            .map(|(a, b)| (a - b) * (a - b))
            .sum();
        Ok(s.sqrt())
    }

    pub fn to_vec(&self) -> Vec<f64> {
        self.data.clone()
    }

    pub fn combine(&self, other: &Self) -> Self {
        let mut data = Vec::with_capacity(self.dimension() + other.dimension());
        data.extend_from_slice(&self.data);
        data.extend_from_slice(&other.data);
        Self {
            data,
            timestamp: default_timestamp_ms(),
        }
    }
}

impl VectorOps for KernelStateVector {
    #[inline]
    fn dimension(&self) -> usize {
        self.data.len()
    }
    #[inline]
    fn as_slice(&self) -> &[f64] {
        &self.data
    }
}

impl std::ops::Index<usize> for KernelStateVector {
    type Output = f64;
    fn index(&self, i: usize) -> &f64 {
        &self.data[i]
    }
}

impl std::ops::IndexMut<usize> for KernelStateVector {
    fn index_mut(&mut self, i: usize) -> &mut f64 {
        &mut self.data[i]
    }
}

fn default_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
