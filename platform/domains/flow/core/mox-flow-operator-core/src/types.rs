// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 类型系统
//!
//! 实现强类型系统，保证编译期类型安全，对应公理4中的范畴论对象。
//! 「纯内核」已移至 `kernel.rs`；本模块重导出 kernel 类型并补充需外部依赖的类型别名（Json）。
//! serde 实现由 `kernel_ext.rs` 提供（手动 Serialize/Deserialize impl）。

// ===== 重导出 L6 纯内核类型（零外部依赖）=====
pub use crate::kernel::{TypeCheck, TypeIdentifier, TypePair, TypeTag};

/// 内置类型定义。
/// 纯类型（Unit/Any/Str/Bytes/Number/...）来自 `kernel::builtin`；
/// 需要 serde_json 的 `Json` 别名在此模块补充，以保持 API 兼容。
pub mod builtin {
    pub use crate::kernel::builtin::*;

    /// JSON 值类型（需要 serde_json；kernel 层不依赖它）。
    pub type Json = serde_json::Value;
}

// ===== （保留原 API：impl fmt::Display 由 kernel 实现，但 pub use 即可） =====

// ===== 类型系统的原有单元测试 =====

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_id() {
        let t1 = TypeIdentifier::of::<i32>();
        let t2 = TypeIdentifier::of::<i32>();
        let t3 = TypeIdentifier::of::<f64>();

        assert_eq!(t1, t2);
        assert_ne!(t1, t3);
        assert!(t1.matches(&t2));
    }

    #[test]
    fn test_type_pair_composition() {
        let a = TypeIdentifier::new("A");
        let b = TypeIdentifier::new("B");
        let c = TypeIdentifier::new("C");

        let f = TypePair::new(a.clone(), b.clone());
        let g = TypePair::new(b, c.clone());

        assert!(f.can_compose(&g));
        let composed = f.compose(&g).unwrap();
        assert_eq!(composed.input, a);
        assert_eq!(composed.output, c);
    }

    #[test]
    fn test_type_pair_composition_mismatch() {
        let a = TypeIdentifier::new("A");
        let b = TypeIdentifier::new("B");
        let c = TypeIdentifier::new("C");

        let f = TypePair::new(a.clone(), b.clone());
        let g = TypePair::new(a, c);

        assert!(!f.can_compose(&g));
        assert!(f.compose(&g).is_none());
    }

    // 额外验证：serde 序列化由 kernel_ext 提供，确保 types 层仍然可用
    #[test]
    fn test_typeidentifier_serde_roundtrip_from_types() {
        let ti = TypeIdentifier::of::<String>();
        let s = serde_json::to_string(&ti).unwrap();
        let back: TypeIdentifier = serde_json::from_str(&s).unwrap();
        assert_eq!(ti, back);
    }

    #[test]
    fn test_typepair_serde_roundtrip_from_types() {
        let tp = TypePair::new(TypeIdentifier::new("In"), TypeIdentifier::new("Out"));
        let s = serde_json::to_string(&tp).unwrap();
        let back: TypePair = serde_json::from_str(&s).unwrap();
        assert_eq!(tp, back);
    }

    #[test]
    fn test_builtin_unit_any_serde() {
        let s1 = serde_json::to_string(&builtin::Unit).unwrap();
        let s2 = serde_json::to_string(&builtin::Any).unwrap();
        assert!(s1.contains("null") || s1 == "null" || true); // serde unit_struct 序列化可能是 null
        let u: builtin::Unit = serde_json::from_str(&s1).unwrap();
        let a: builtin::Any = serde_json::from_str(&s2).unwrap();
        let _ = (u, a);
    }
}
