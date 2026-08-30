// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # Operator Core - L6 Zero External Dependencies Kernel
//!
//! 纯内核层：定义算子系统的纯数据结构、输入输出 trait、标量运算。
//! 仅依赖标准库（std），绝不引入 serde / nalgebra / anyhow 等外部 crate。
//! 所有涉及序列化或矩阵库的能力由上层 kernel_ext.rs 通过 DIP 方式扩展。

pub mod conservation;
pub mod monad;
pub mod resource;
pub mod types;
pub mod vector;

// ===== 重导出所有公开类型（保持 API 兼容）=====
pub use conservation::{
    ConservationChecker, ConservationLaw, GraphNode, L1Conservation, L2Conservation,
    ResidualMonitor, SumConservation,
};
pub use monad::{IO, Op, StateOp};
pub use resource::{ResourceCost, ResourceLimits, ResourceUsage};
pub use types::{builtin, KernelError, TypeCheck, TypeIdentifier, TypePair, TypeTag};
pub use vector::{KernelStateVector, VectorOps};

// ============================================================
// 单元测试（纯内核，零外部依赖）
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ktype_id_of_static() {
        let t1 = TypeIdentifier::of::<i32>();
        let t2 = TypeIdentifier::of::<i32>();
        let t3 = TypeIdentifier::of::<f64>();
        assert_eq!(t1, t2);
        assert_ne!(t1, t3);
        assert!(t1.matches(&t2));
    }

    #[test]
    fn ktype_id_new_named() {
        let a = TypeIdentifier::new("A");
        let b = TypeIdentifier::new("B");
        assert!(!a.matches(&b));
        assert_eq!(a.name, "A");
    }

    #[test]
    fn ktype_pair_compose_ok() {
        let a = TypeIdentifier::new("A");
        let b = TypeIdentifier::new("B");
        let c = TypeIdentifier::new("C");
        let f = TypePair::new(a.clone(), b.clone());
        let g = TypePair::new(b, c.clone());
        assert!(f.can_compose(&g));
        let h = f.compose(&g).unwrap();
        assert_eq!(h.input, a);
        assert_eq!(h.output, c);
    }

    #[test]
    fn ktype_pair_compose_fail() {
        let a = TypeIdentifier::new("A");
        let b = TypeIdentifier::new("B");
        let c = TypeIdentifier::new("C");
        let f = TypePair::new(a.clone(), b.clone());
        let g = TypePair::new(a, c);
        assert!(!f.can_compose(&g));
        assert!(f.compose(&g).is_none());
    }

    #[test]
    fn kresource_cost_add() {
        let c1 = ResourceCost::new(100, 1000);
        let c2 = ResourceCost::new(200, 2000);
        let c3 = c1 + c2;
        assert_eq!(c3.cpu_cycles, 300);
        assert_eq!(c3.memory_bytes, 3000);
    }

    #[test]
    fn kresource_usage_sub_saturating() {
        let a = ResourceUsage {
            cpu_time_ms: 5,
            memory_peak_bytes: 100,
            disk_io_bytes: 3,
            network_bytes: 4,
        };
        let b = ResourceUsage {
            cpu_time_ms: 10,
            memory_peak_bytes: 200,
            disk_io_bytes: 0,
            network_bytes: 0,
        };
        let c = a - b;
        assert_eq!(c.cpu_time_ms, 0);
        assert_eq!(c.memory_peak_bytes, 0);
    }

    #[test]
    fn kresource_usage_within_limits() {
        let u = ResourceUsage {
            cpu_time_ms: 100,
            memory_peak_bytes: 1024 * 1024,
            disk_io_bytes: 0,
            network_bytes: 0,
        };
        assert!(u.within_limits(&ResourceLimits::default()));
    }
}
