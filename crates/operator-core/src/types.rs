//! # 类型系统
//!
//! 实现强类型系统，保证编译期类型安全，对应公理4中的范畴论对象。

use std::fmt;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

/// 类型标识，用于运行时类型检查
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeIdentifier {
    pub name: String,
    pub id: u64,
}

impl TypeIdentifier {
    /// 从Rust类型创建TypeIdentifier
    pub fn of<T: 'static>() -> Self {
        let name = std::any::type_name::<T>().to_string();
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        Self {
            name,
            id: hasher.finish(),
        }
    }

    /// 创建自定义类型
    pub fn new(name: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        Self {
            name: name.to_string(),
            id: hasher.finish(),
        }
    }

    /// 检查类型是否匹配
    pub fn matches(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl fmt::Display for TypeIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// 类型标记，用于编译期类型安全
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

/// 类型对，表示算子的输入输出类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypePair {
    pub input: TypeIdentifier,
    pub output: TypeIdentifier,
}

impl TypePair {
    pub fn new(input: TypeIdentifier, output: TypeIdentifier) -> Self {
        Self { input, output }
    }

    /// 检查两个算子是否可以复合: f: A->B, g: B->C => g∘f: A->C
    pub fn can_compose(&self, next: &TypePair) -> bool {
        self.output.matches(&next.input)
    }

    /// 复合后的类型
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

/// 内置类型定义
pub mod builtin {
    use super::*;

    /// 空类型，用于无输入/输出
    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct Unit;

    /// 任意类型，用于泛型算子
    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct Any;

    /// 字符串类型
    pub type Str = String;

    /// 字节数组类型
    pub type Bytes = Vec<u8>;

    /// JSON值类型
    pub type Json = serde_json::Value;

    /// 数值类型
    pub type Number = f64;

    /// 整数类型
    pub type Integer = i64;

    /// 布尔类型
    pub type Bool = bool;

    /// 向量类型
    pub type Vector = Vec<f64>;

    /// 矩阵类型
    pub type Matrix = Vec<Vec<f64>>;

    /// 错误类型
    pub type Error = String;

    /// 单元类型ID
    pub fn unit_type() -> TypeIdentifier {
        TypeIdentifier::of::<Unit>()
    }

    /// 任意类型ID
    pub fn any_type() -> TypeIdentifier {
        TypeIdentifier::of::<Any>()
    }

    /// 状态向量类型ID（算子系统最常用的类型）
    pub fn state_vector_type() -> TypeIdentifier {
        TypeIdentifier::new("StateVector")
    }

    /// 张量积类型ID
    pub fn tensor_product_type() -> TypeIdentifier {
        TypeIdentifier::new("TensorProduct")
    }
}

/// 类型检查trait
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
        assert!(!t1.matches(&t3));
    }

    #[test]
    fn test_type_pair_composition() {
        let a = TypeIdentifier::new("A");
        let b = TypeIdentifier::new("B");
        let c = TypeIdentifier::new("C");

        let f = TypePair::new(a.clone(), b.clone());
        let g = TypePair::new(b.clone(), c.clone());

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
        let g = TypePair::new(a.clone(), c.clone());

        assert!(!f.can_compose(&g));
        assert!(f.compose(&g).is_none());
    }
}
