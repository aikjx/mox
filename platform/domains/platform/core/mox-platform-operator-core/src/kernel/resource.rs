// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 纯资源模型：资源消耗、资源使用量、资源限制。
//!
//! 纯数学核心，零外部依赖。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceCost {
    pub cpu_cycles: u64,
    pub memory_bytes: u64,
    pub disk_io_bytes: u64,
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

impl std::ops::Add for ResourceCost {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            cpu_cycles: self.cpu_cycles + rhs.cpu_cycles,
            memory_bytes: self.memory_bytes + rhs.memory_bytes,
            disk_io_bytes: self.disk_io_bytes + rhs.disk_io_bytes,
            network_bytes: self.network_bytes + rhs.network_bytes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceUsage {
    pub cpu_time_ms: u64,
    pub memory_peak_bytes: u64,
    pub disk_io_bytes: u64,
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

impl std::ops::Add for ResourceUsage {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            cpu_time_ms: self.cpu_time_ms + rhs.cpu_time_ms,
            memory_peak_bytes: std::cmp::max(self.memory_peak_bytes, rhs.memory_peak_bytes),
            disk_io_bytes: self.disk_io_bytes + rhs.disk_io_bytes,
            network_bytes: self.network_bytes + rhs.network_bytes,
        }
    }
}

impl std::ops::Sub for ResourceUsage {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            cpu_time_ms: self.cpu_time_ms.saturating_sub(rhs.cpu_time_ms),
            memory_peak_bytes: self.memory_peak_bytes.saturating_sub(rhs.memory_peak_bytes),
            disk_io_bytes: self.disk_io_bytes.saturating_sub(rhs.disk_io_bytes),
            network_bytes: self.network_bytes.saturating_sub(rhs.network_bytes),
        }
    }
}

#[derive(Debug, Clone, Copy)]
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
            max_memory_bytes: 1024 * 1024 * 1024,
            max_disk_io_bytes: 100 * 1024 * 1024,
            max_network_bytes: 100 * 1024 * 1024,
        }
    }
}
