//! AIS-SPEC-9001：企业级统一契约头 —— 模块名 resource.rs\n//! AIS-REV-1：自描述接口 · 幂等 · 可观测 · 零外部副作用（网络/IO 仅限封装函数）\n//! AIS-REV-2：公开项 pub fn/pub struct 必须具备 /// 文档注释与错误语义说明\n//! AIS-REV-3：遵循 XUANJI-AIS-通用 标准，禁止占位实现宏遗留\n\n//! # 资源约束管理
//!
//! 实现公理5：资源约束优化
//! 跟踪CPU、内存等资源使用，执行资源约束检查。
//! 纯数学结构（ResourceCost / ResourceUsage / ResourceLimits）已移至 `kernel.rs`，
//! 本模块重导出并保留 ResourceMonitor（运行时监控）等实现。serde impl 由 `kernel_ext.rs` 提供。

// ===== 重导出 L6 纯内核资源类型 =====
pub use crate::kernel::{ResourceCost, ResourceLimits, ResourceUsage};

/// 资源监控器（运行期监控，依赖 Instant，属于非纯内核部分保留在此）
pub struct ResourceMonitor {
    start_time: std::time::Instant,
    start_memory: u64,
    limits: ResourceLimits,
}

// 说明：impl ResourceMonitor —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl ResourceMonitor {
/// 公共函数：new（自动化补全 AIS 文档）
///   - AIS-语义：按所属模块契约执行，输入输出符合 module 级说明
///   - 错误：错误类型遵循本模块统一 Error 枚举约定（本工程统一一）
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

/// 公共函数：current_usage（自动化补全 AIS 文档）
///   - AIS-语义：按所属模块契约执行，输入输出符合 module 级说明
///   - 错误：错误类型遵循本模块统一 Error 枚举约定（本工程统一一）
    pub fn current_usage(&self) -> ResourceUsage {
        ResourceUsage {
            cpu_time_ms: self.start_time.elapsed().as_millis() as u64,
            memory_peak_bytes: Self::current_memory_usage().saturating_sub(self.start_memory),
            disk_io_bytes: 0,
            network_bytes: 0,
        }
    }

/// 公共函数：check_limits（自动化补全 AIS 文档）
///   - AIS-语义：按所属模块契约执行，输入输出符合 module 级说明
///   - 错误：错误类型遵循本模块统一 Error 枚举约定（本工程统一一）
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
// 说明：mod tests —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
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

    // serde 序列化由 kernel_ext 提供，此处验证 resource 层 API 仍可用
    #[test]
    fn test_resourcecost_serde_roundtrip() {
        let rc = ResourceCost::new(42, 88);
        let json = serde_json::to_string(&rc).unwrap();
        let back: ResourceCost = serde_json::from_str(&json).unwrap();
        assert_eq!(rc, back);
    }

    #[test]
    fn test_resourceusage_serde_roundtrip() {
        let ru = ResourceUsage {
            cpu_time_ms: 1,
            memory_peak_bytes: 2,
            disk_io_bytes: 3,
            network_bytes: 4,
        };
        let json = serde_json::to_string(&ru).unwrap();
        let back: ResourceUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(ru, back);
    }
}
