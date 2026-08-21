//! # 高维状态向量
//!
//! 实现公理2：系统状态高维向量
//! 基于希尔伯特空间的状态表示，支持守恒律检查

use nalgebra::DVector;
use serde::{Deserialize, Serialize};

use crate::OperatorError;

/// 高维状态向量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateVector {
    /// 向量数据
    pub data: DVector<f64>,
    /// 维度
    pub dimension: usize,
    /// 时间戳
    pub timestamp: u64,
    /// 元数据
    pub metadata: serde_json::Value,
}

impl StateVector {
    /// 创建新的状态向量
    pub fn new(dimension: usize) -> Self {
        Self {
            data: DVector::zeros(dimension),
            dimension,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            metadata: serde_json::json!({}),
        }
    }

    /// 从向量创建
    pub fn from_vec(data: Vec<f64>) -> Self {
        let dimension = data.len();
        Self {
            data: DVector::from_vec(data),
            dimension,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            metadata: serde_json::json!({}),
        }
    }

    /// 创建零向量
    pub fn zeros(dimension: usize) -> Self {
        Self::new(dimension)
    }

    /// 创建单位向量（概率分布）
    pub fn unit(dimension: usize) -> Self {
        let mut v = Self::new(dimension);
        let val = 1.0 / dimension as f64;
        v.data.fill(val);
        v
    }

    /// 获取向量元素
    pub fn get(&self, index: usize) -> Option<f64> {
        if index < self.dimension {
            Some(self.data[index])
        } else {
            None
        }
    }

    /// 设置向量元素
    pub fn set(&mut self, index: usize, value: f64) -> Result<(), OperatorError> {
        if index >= self.dimension {
            return Err(OperatorError::ExecutionError(format!(
                "索引 {} 超出维度 {}",
                index, self.dimension
            )));
        }
        self.data[index] = value;
        Ok(())
    }

    /// 向量加法
    pub fn add(&self, other: &Self) -> Result<Self, OperatorError> {
        if self.dimension != other.dimension {
            return Err(OperatorError::TypeMismatch {
                expected: std::any::TypeId::of::<Self>(),
                actual: std::any::TypeId::of::<Self>(),
            });
        }
        Ok(Self {
            data: &self.data + &other.data,
            dimension: self.dimension,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            metadata: self.metadata.clone(),
        })
    }

    /// 向量标量乘法
    pub fn scale(&self, scalar: f64) -> Self {
        Self {
            data: &self.data * scalar,
            dimension: self.dimension,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            metadata: self.metadata.clone(),
        }
    }

    /// 内积
    pub fn dot(&self, other: &Self) -> Result<f64, OperatorError> {
        if self.dimension != other.dimension {
            return Err(OperatorError::TypeMismatch {
                expected: std::any::TypeId::of::<Self>(),
                actual: std::any::TypeId::of::<Self>(),
            });
        }
        Ok(self.data.dot(&other.data))
    }

    /// L2范数（能量）
    pub fn norm(&self) -> f64 {
        self.data.norm()
    }

    /// L1范数（概率和）
    pub fn norm_l1(&self) -> f64 {
        self.data.iter().map(|x| x.abs()).sum()
    }

    /// 归一化到单位范数
    pub fn normalize(&mut self) {
        let norm = self.norm();
        if norm > 1e-15 {
            self.data /= norm;
        }
    }

    /// 归一化到概率分布（L1=1）
    pub fn normalize_probability(&mut self) {
        let sum = self.norm_l1();
        if sum > 1e-15 {
            self.data /= sum;
        }
    }

    /// 计算两个状态之间的残差
    pub fn residual(&self, expected: &Self) -> Result<f64, OperatorError> {
        if self.dimension != expected.dimension {
            return Err(OperatorError::TypeMismatch {
                expected: std::any::TypeId::of::<Self>(),
                actual: std::any::TypeId::of::<Self>(),
            });
        }
        Ok((&self.data - &expected.data).norm())
    }

    /// 转换为Vec<f64>
    pub fn to_vec(&self) -> Vec<f64> {
        self.data.iter().copied().collect()
    }

    /// 线性变换：y = Mx
    pub fn apply_matrix(&self, matrix: &nalgebra::DMatrix<f64>) -> Result<Self, OperatorError> {
        if matrix.ncols() != self.dimension {
            return Err(OperatorError::ExecutionError(format!(
                "矩阵列数 {} 与向量维度 {} 不匹配",
                matrix.ncols(),
                self.dimension
            )));
        }
        Ok(Self {
            data: matrix * &self.data,
            dimension: matrix.nrows(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            metadata: self.metadata.clone(),
        })
    }

    /// 合并两个状态向量（拼接），用于图谱边的守恒检查
    pub fn combine(&self, other: &Self) -> Result<Self, OperatorError> {
        let mut combined_data = Vec::with_capacity(self.dimension + other.dimension);
        combined_data.extend_from_slice(self.data.as_slice());
        combined_data.extend_from_slice(other.data.as_slice());
        Ok(Self::from_vec(combined_data))
    }
}

impl std::ops::Index<usize> for StateVector {
    type Output = f64;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl std::ops::IndexMut<usize> for StateVector {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_state_vector_creation() {
        let v = StateVector::new(10);
        assert_eq!(v.dimension, 10);
        assert_eq!(v.norm(), 0.0);
    }

    #[test]
    fn test_state_vector_from_vec() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let v = StateVector::from_vec(data);
        assert_eq!(v.dimension, 4);
        assert_relative_eq!(v[0], 1.0);
        assert_relative_eq!(v[3], 4.0);
    }

    #[test]
    fn test_state_vector_norm() {
        let v = StateVector::from_vec(vec![3.0, 4.0]);
        assert_relative_eq!(v.norm(), 5.0);
        assert_relative_eq!(v.norm_l1(), 7.0);
    }

    #[test]
    fn test_state_vector_normalize() {
        let mut v = StateVector::from_vec(vec![3.0, 4.0]);
        v.normalize();
        assert_relative_eq!(v.norm(), 1.0);
    }

    #[test]
    fn test_state_vector_dot() {
        let v1 = StateVector::from_vec(vec![1.0, 2.0, 3.0]);
        let v2 = StateVector::from_vec(vec![4.0, 5.0, 6.0]);
        assert_relative_eq!(v1.dot(&v2).unwrap(), 32.0);
    }

    #[test]
    fn test_state_vector_residual() {
        let v1 = StateVector::from_vec(vec![1.0, 2.0, 3.0]);
        let v2 = StateVector::from_vec(vec![1.0, 2.0, 3.0]);
        assert_relative_eq!(v1.residual(&v2).unwrap(), 0.0);
    }
}
