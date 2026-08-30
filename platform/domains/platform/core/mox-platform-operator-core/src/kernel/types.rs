// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 纯类型系统：运行时类型标识、类型对、内置类型、类型检查 trait。
//!
//! 零外部依赖，仅依赖标准库。

use std::any::TypeId;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// 运行时类型标识（纯内核版，零外部依赖）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeIdentifier {
    pub name: String,
    pub id: u64,
}

impl TypeIdentifier {
    /// 从 Rust 静态类型构造标识。
    pub fn of<T: 'static>() -> Self {
        let name = std::any::type_name::<T>().to_string();
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        Self {
            name,
            id: hasher.finish(),
        }
    }

    /// 从自定义名称构造标识。
    pub fn new(name: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        Self {
            name: name.to_string(),
            id: hasher.finish(),
        }
    }

    /// 基于 id 判定类型匹配。
    pub fn matches(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl fmt::Display for TypeIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// 编译期类型标签。
pub struct TypeTag<T: 'static>(PhantomData<T>);

impl<T: 'static> TypeTag<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }

    pub fn type_id() -> TypeIdentifier {
        TypeIdentifier::of::<T>()
    }
}

impl<T: 'static> Default for TypeTag<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// 算子类型对（输入 → 输出）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypePair {
    pub input: TypeIdentifier,
    pub output: TypeIdentifier,
}

impl TypePair {
    pub fn new(input: TypeIdentifier, output: TypeIdentifier) -> Self {
        Self { input, output }
    }

    /// 判定 self.output 是否能接 other.input（即能否复合）。
    pub fn can_compose(&self, next: &TypePair) -> bool {
        self.output.matches(&next.input)
    }

    /// 复合两个类型对。
    pub fn compose(&self, next: &TypePair) -> Option<TypePair> {
        if self.can_compose(next) {
            Some(TypePair::new(self.input.clone(), next.output.clone()))
        } else {
            None
        }
    }
}

impl fmt::Display for TypePair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.input, self.output)
    }
}

/// 内置纯类型定义。需要外部依赖的类型别名（如 Json）由上层 types 模块补充。
pub mod builtin {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Unit;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Any;

    pub type Str = String;
    pub type Bytes = Vec<u8>;
    pub type Number = f64;
    pub type Integer = i64;
    pub type Bool = bool;
    pub type Vector = Vec<f64>;
    pub type Matrix = Vec<Vec<f64>>;
    pub type Error = String;

    pub fn unit_type() -> TypeIdentifier {
        TypeIdentifier::of::<Unit>()
    }

    pub fn any_type() -> TypeIdentifier {
        TypeIdentifier::of::<Any>()
    }

    pub fn state_vector_type() -> TypeIdentifier {
        TypeIdentifier::new("StateVector")
    }

    pub fn tensor_product_type() -> TypeIdentifier {
        TypeIdentifier::new("TensorProduct")
    }
}

/// 类型检查 trait：任何算子/组件均可声明其输入输出类型。
pub trait TypeCheck {
    fn input_type(&self) -> TypeIdentifier;
    fn output_type(&self) -> TypeIdentifier;

    fn type_pair(&self) -> TypePair {
        TypePair::new(self.input_type(), self.output_type())
    }

    fn check_input(&self, expected: &TypeIdentifier) -> bool {
        self.input_type().matches(expected) || self.input_type().matches(&builtin::any_type())
    }

    fn check_output(&self, expected: &TypeIdentifier) -> bool {
        self.output_type().matches(expected) || self.output_type().matches(&builtin::any_type())
    }
}

/// 内核最简错误枚举（仅 std，不依赖 thiserror / anyhow）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    DimensionMismatch { a: usize, b: usize },
    IndexOutOfBounds { idx: usize, len: usize },
    TypeMismatch { expected: u64, actual: u64 },
    Other(String),
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::DimensionMismatch { a, b } => {
                write!(f, "维度不匹配: {} vs {}", a, b)
            }
            KernelError::IndexOutOfBounds { idx, len } => {
                write!(f, "索引越界: {} / {}", idx, len)
            }
            KernelError::TypeMismatch { expected, actual } => {
                write!(
                    f,
                    "类型不匹配: expected_id={}, actual_id={}",
                    expected, actual
                )
            }
            KernelError::Other(s) => write!(f, "{}", s),
        }
    }
}

impl From<KernelError> for TypeId {
    fn from(_: KernelError) -> Self {
        TypeId::of::<KernelError>()
    }
}
