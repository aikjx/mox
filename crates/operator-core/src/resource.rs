//! # 资源约束管理
//!
//! 实现公理5：资源约束优化
//! 跟踪CPU、内存等资源使用，执行资源约束检查

use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

/// 算子资源消耗模型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ResourceCost {
    /// CPU周期数估计
    pub cpu_cycles: u64,
    /// 内存字节数估计
    pub memory_bytes: u64,
    /// 磁盘IO字节数
    pub disk_io_bytes: u64,
    /// 网络IO字节数
    pub network_bytes: u64,
}

impl ResourceCost {
    pub fn new(cpu_cycles: u64, memory_bytes: u64) -> Self {
        Self {
            cpu_cycles,
            memory_bytes,
            disk_io_bytes: 0,
            network_bytes: 0,
        }
    }

    pub fn zero() -> Self {
        Self {
            cpu_cycles: 0,
            memory_bytes: 0,
            disk_io_bytes: 0,
            network_bytes: 0,
        }
    }

    pub fn minimal() -> Self {
        Self {
            cpu_cycles: 100,
            memory_bytes: 1024,
            disk_io_bytes: 0,
            network_bytes: 0,
        }
    }
}

impl Default for ResourceCost {
    fn default() -> Self {
        Self {
            cpu_cycles: 1000,
            memory_bytes: 4096,
            disk_io_bytes: 0,
            network_bytes: 0,
        }
    }
}

impl Add for ResourceCost {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            cpu_cycles: self.cpu_cycles + rhs.cpu_cycles,
            memory_bytes: self.memory_bytes + rhs.memory_bytes,
            disk_io_bytes: self.disk_io_bytes + rhs.disk_io_bytes,
            network_bytes: self.network_bytes + rhs.network_bytes,
        }
    }
}

/// 实际资源使用情况
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ResourceUsage {
    /// CPU时间(毫秒)
    pub cpu_time_ms: u64,
    /// 内存使用峰值(字节)
    pub memory_peak_bytes: u64,
    /// 实际磁盘IO字节数
    pub disk_io_bytes: u64,
    /// 实际网络IO字节数
    pub network_bytes: u64,
}

impl ResourceUsage {
    pub fn zero() -> Self {
        Self {
            cpu_time_ms: 0,
            memory_peak_bytes: 0,
            disk_io_bytes: 0,
            network_bytes: 0,
        }
    }

    /// 检查是否在资源限制内
    pub fn within_limits(&self, limits: &ResourceLimits) -> bool {
        self.cpu_time_ms <= limits.max_cpu_time_ms
            && self.memory_peak_bytes <= limits.max_memory_bytes
            && self.disk_io_bytes <= limits.max_disk_io_bytes
            && self.network_bytes <= limits.max_network_bytes
    }
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self::zero()
    }
}

impl Add for ResourceUsage {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            cpu_time_ms: self.cpu_time_ms + rhs.cpu_time_ms,
            memory_peak_bytes: std::cmp::max(self.memory_peak_bytes, rhs.memory_peak_bytes),
            disk_io_bytes: self.disk_io_bytes + rhs.disk_io_bytes,
            network_bytes: self.network_bytes + rhs.network_bytes,
        }
    }
}

impl Sub for ResourceUsage {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            cpu_time_ms: self.cpu_time_ms.saturating_sub(rhs.cpu_time_ms),
            memory_peak_bytes: self.memory_peak_bytes.saturating_sub(rhs.memory_peak_bytes),
            disk_io_bytes: self.disk_io_bytes.saturating_sub(rhs.disk_io_bytes),
            network_bytes: self.network_bytes.saturating_sub(rhs.network_bytes),
        }
    }
}

/// 资源限制
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_cpu_time_ms: u64,
    pub max_memory_bytes: u64,
    pub max_disk_io_bytes: u64,
    pub max_network_bytes: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_time_ms: 30000,
            max_memory_bytes: 1024 * 1024 * 1024, // 1GB
            max_disk_io_bytes: 100 * 1024 * 1024, // 100MB
            max_network_bytes: 100 * 1024 * 1024, // 100MB
        }
    }
}

/// 资源监控器
pub struct ResourceMonitor {
    start_time: std::time::Instant,
    start_memory: u64,
    limits: ResourceLimits,
}

impl ResourceMonitor {
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            start_memory: Self::current_memory_usage(),
            limits,
        }
    }

    fn current_memory_usage() -> u64 {
        // 简化实现，实际系统会读取/proc/self/status
        0
    }

    pub fn current_usage(&self) -> ResourceUsage {
        ResourceUsage {
            cpu_time_ms: self.start_time.elapsed().as_millis() as u64,
            memory_peak_bytes: Self::current_memory_usage().saturating_sub(self.start_memory),
            disk_io_bytes: 0,
            network_bytes: 0,
        }
    }

    pub fn check_limits(&self) -> Result<(), String> {
        let usage = self.current_usage();
        if !usage.within_limits(&self.limits) {
            Err(format!("资源超出限制: {:?}", usage))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_cost_add() {
        let c1 = ResourceCost::new(100, 1000);
        let c2 = ResourceCost::new(200, 2000);
        let c3 = c1 + c2;
        assert_eq!(c3.cpu_cycles, 300);
        assert_eq!(c3.memory_bytes, 3000);
    }

    #[test]
    fn test_resource_usage_within_limits() {
        let usage = ResourceUsage {
            cpu_time_ms: 100,
            memory_peak_bytes: 1024 * 1024,
            disk_io_bytes: 0,
            network_bytes: 0,
        };
        let limits = ResourceLimits::default();
        assert!(usage.within_limits(&limits));
    }
}
