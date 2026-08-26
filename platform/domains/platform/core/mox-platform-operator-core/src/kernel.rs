//! # Operator Core - L6 Zero External Dependencies Kernel
//!
//! 纯内核层：定义算子系统的纯数据结构、输入输出 trait、标量运算。
//! 仅依赖标准库（std），绝不引入 serde / nalgebra / anyhow 等外部 crate。
//! 所有涉及序列化或矩阵库的能力由上层 kernel_ext.rs 通过 DIP 方式扩展。

// ============================================================
// §1 纯类型系统（对应 types.rs 的纯数学核心）
// ============================================================

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

// ============================================================
// §2 纯向量运算抽象 + KernelStateVector（Vec<f64> 实现）
// ============================================================

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

// ============================================================
// §3 纯资源模型（对应 resource.rs 的纯数学核心）
// ============================================================

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

// ============================================================
// §4 纯守恒律系统（基于 VectorOps 抽象，不绑定具体实现）
// ============================================================

/// 守恒律 trait：通过 `dyn VectorOps` 接收任意向量实现。
pub trait ConservationLaw {
    fn name(&self) -> &str;
    fn check(&self, state: &dyn VectorOps) -> f64;

    fn is_satisfied(&self, state: &dyn VectorOps, threshold: f64) -> bool {
        self.check(state).abs() < threshold
    }
}

pub struct L1Conservation {
    expected_sum: f64,
}

impl L1Conservation {
    pub fn new(expected_sum: f64) -> Self {
        Self { expected_sum }
    }

    pub fn probability() -> Self {
        Self::new(1.0)
    }
}

impl ConservationLaw for L1Conservation {
    fn name(&self) -> &str {
        "L1范数守恒（概率守恒）"
    }

    fn check(&self, state: &dyn VectorOps) -> f64 {
        (state.norm_l1() - self.expected_sum).abs()
    }
}

pub struct L2Conservation {
    expected_norm: f64,
}

impl L2Conservation {
    pub fn new(expected_norm: f64) -> Self {
        Self { expected_norm }
    }

    pub fn unit_energy() -> Self {
        Self::new(1.0)
    }
}

impl ConservationLaw for L2Conservation {
    fn name(&self) -> &str {
        "L2范数守恒（能量守恒）"
    }

    fn check(&self, state: &dyn VectorOps) -> f64 {
        (state.norm_l2() - self.expected_norm).abs()
    }
}

pub struct SumConservation {
    expected_sum: f64,
}

impl SumConservation {
    pub fn new(expected_sum: f64) -> Self {
        Self { expected_sum }
    }
}

impl ConservationLaw for SumConservation {
    fn name(&self) -> &str {
        "元素总和守恒"
    }

    fn check(&self, state: &dyn VectorOps) -> f64 {
        (state.sum() - self.expected_sum).abs()
    }
}

pub struct ConservationChecker {
    laws: Vec<Box<dyn ConservationLaw>>,
    threshold: f64,
}

impl ConservationChecker {
    pub fn new(threshold: f64) -> Self {
        Self {
            laws: Vec::new(),
            threshold,
        }
    }

    pub fn with_default_laws(threshold: f64) -> Self {
        let mut checker = Self::new(threshold);
        checker.add_law(L1Conservation::probability());
        checker.add_law(L2Conservation::unit_energy());
        checker
    }

    pub fn add_law<L: ConservationLaw + 'static>(&mut self, law: L) {
        self.laws.push(Box::new(law));
    }

    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    pub fn check_all(&self, state: &dyn VectorOps) -> Result<(), KernelError> {
        for law in &self.laws {
            let residual = law.check(state);
            if residual.abs() >= self.threshold {
                return Err(KernelError::Other(format!(
                    "守恒律违反: {}, residual={}",
                    law.name(),
                    residual
                )));
            }
        }
        Ok(())
    }

    pub fn check_all_residuals(&self, state: &dyn VectorOps) -> Vec<(&str, f64)> {
        self.laws
            .iter()
            .map(|law| (law.name(), law.check(state)))
            .collect()
    }
}

pub struct ResidualMonitor {
    history: Vec<f64>,
    threshold: f64,
}

impl ResidualMonitor {
    pub fn new(threshold: f64) -> Self {
        Self {
            history: Vec::new(),
            threshold,
        }
    }

    pub fn record(&mut self, residual: f64) {
        self.history.push(residual);
    }

    pub fn is_converged(&self, window: usize) -> bool {
        if self.history.len() < window {
            return false;
        }
        let recent = &self.history[self.history.len() - window..];
        let max = recent.iter().fold(0.0f64, |a, &b| a.max(b.abs()));
        max < self.threshold
    }

    pub fn max_residual(&self) -> f64 {
        self.history.iter().fold(0.0f64, |a, &b| a.max(b.abs()))
    }

    pub fn mean_residual(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        self.history.iter().sum::<f64>() / self.history.len() as f64
    }
}

/// 通用图谱节点：任何携带状态向量的结构均可实现。
pub trait GraphNode {
    fn state_vector_dyn(&self) -> &dyn VectorOps;
}

// ============================================================
// §5 纯单子系统（对应 monad.rs 的核心）
// ============================================================

#[derive(Debug)]
pub struct Op<T> {
    value: Option<T>,
    error: Option<String>,
    logs: Vec<String>,
}

impl<T> Op<T> {
    pub fn pure(value: T) -> Self {
        Self {
            value: Some(value),
            error: None,
            logs: Vec::new(),
        }
    }

    pub fn fail(error: impl Into<String>) -> Self {
        Self {
            value: None,
            error: Some(error.into()),
            logs: Vec::new(),
        }
    }

    pub fn log(mut self, msg: impl Into<String>) -> Self {
        self.logs.push(msg.into());
        self
    }

    pub fn is_ok(&self) -> bool {
        self.value.is_some() && self.error.is_none()
    }

    pub fn is_err(&self) -> bool {
        self.error.is_some()
    }

    pub fn unwrap(self) -> T {
        self.value.expect("Op was in error state")
    }

    pub fn unwrap_err(self) -> String {
        self.error.expect("Op was in ok state")
    }

    pub fn logs(&self) -> &[String] {
        &self.logs
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Op<U> {
        match self.value {
            Some(v) => Op {
                value: Some(f(v)),
                error: self.error,
                logs: self.logs,
            },
            None => Op {
                value: None,
                error: self.error,
                logs: self.logs,
            },
        }
    }

    pub fn bind<U, F: FnOnce(T) -> Op<U>>(self, f: F) -> Op<U> {
        match self.value {
            Some(v) => {
                let mut result = f(v);
                let mut logs = self.logs;
                logs.append(&mut result.logs);
                result.logs = logs;
                result
            }
            None => Op {
                value: None,
                error: self.error,
                logs: self.logs,
            },
        }
    }
}

pub struct StateOp<S, A> {
    run: Box<dyn FnOnce(S) -> (A, S)>,
}

impl<S: 'static, A: 'static> StateOp<S, A> {
    pub fn new<F: FnOnce(S) -> (A, S) + 'static>(f: F) -> Self {
        Self { run: Box::new(f) }
    }

    pub fn pure(a: A) -> Self
    where
        A: Clone,
    {
        Self::new(move |s| (a.clone(), s))
    }

    pub fn bind<B: 'static, F: FnOnce(A) -> StateOp<S, B> + 'static>(self, f: F) -> StateOp<S, B> {
        StateOp::new(move |s| {
            let (a, s1) = (self.run)(s);
            (f(a).run)(s1)
        })
    }

    pub fn map<B: 'static, F: FnOnce(A) -> B + 'static>(self, f: F) -> StateOp<S, B> {
        StateOp::new(move |s| {
            let (a, s1) = (self.run)(s);
            (f(a), s1)
        })
    }

    pub fn run(self, initial: S) -> (A, S) {
        (self.run)(initial)
    }

    pub fn eval(self, initial: S) -> A {
        self.run(initial).0
    }

    pub fn exec(self, initial: S) -> S {
        self.run(initial).1
    }
}

impl<S: 'static> StateOp<S, S> {
    pub fn get() -> Self
    where
        S: Clone,
    {
        Self::new(|s| (s.clone(), s))
    }
}

impl<S: 'static> StateOp<S, ()> {
    pub fn put(new_state: S) -> Self {
        Self::new(move |_| ((), new_state))
    }

    pub fn modify<F: FnOnce(S) -> S + 'static>(f: F) -> Self {
        Self::new(move |s| ((), f(s)))
    }
}

pub struct IO<A> {
    perform: Box<dyn FnOnce() -> A>,
}

impl<A: 'static> IO<A> {
    pub fn new<F: FnOnce() -> A + 'static>(f: F) -> Self {
        Self {
            perform: Box::new(f),
        }
    }

    pub fn pure(a: A) -> Self {
        Self::new(move || a)
    }

    pub fn bind<B: 'static, F: FnOnce(A) -> IO<B> + 'static>(self, f: F) -> IO<B> {
        IO::new(move || {
            let a = (self.perform)();
            (f(a).perform)()
        })
    }

    pub fn map<B: 'static, F: FnOnce(A) -> B + 'static>(self, f: F) -> IO<B> {
        IO::new(move || f((self.perform)()))
    }

    pub fn run(self) -> A {
        (self.perform)()
    }
}

// ============================================================
// §6 纯内核错误类型（为上层 Result 保留）
// ============================================================

impl From<KernelError> for TypeId {
    fn from(_: KernelError) -> Self {
        TypeId::of::<KernelError>()
    }
}

// ============================================================
// 内部辅助
// ============================================================

fn default_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ============================================================
// §7 单元测试（纯内核，零外部依赖）
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

    #[test]
    fn kstate_vector_norm_l2_and_l1() {
        let v = KernelStateVector::from_vec(vec![3.0, 4.0]);
        assert!((v.norm_l2() - 5.0).abs() < 1e-12, "norm_l2={}", v.norm_l2());
        assert!((v.norm_l1() - 7.0).abs() < 1e-12, "norm_l1={}", v.norm_l1());
    }

    #[test]
    fn kstate_vector_normalize() {
        let mut v = KernelStateVector::from_vec(vec![3.0, 4.0]);
        v.normalize();
        assert!((v.norm_l2() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn kstate_vector_normalize_probability() {
        let mut v = KernelStateVector::from_vec(vec![1.0, 2.0, 3.0, 4.0]);
        v.normalize_probability();
        assert!((v.norm_l1() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn kstate_vector_dot_and_add() {
        let a = KernelStateVector::from_vec(vec![1.0, 2.0, 3.0]);
        let b = KernelStateVector::from_vec(vec![4.0, 5.0, 6.0]);
        assert!((a.dot(&b).unwrap() - 32.0).abs() < 1e-12);
        let s = a.add(&b).unwrap();
        assert_eq!(s.data, vec![5.0, 7.0, 9.0]);
    }

    #[test]
    fn kstate_vector_dim_mismatch_returns_err() {
        let a = KernelStateVector::from_vec(vec![1.0, 2.0]);
        let b = KernelStateVector::from_vec(vec![1.0, 2.0, 3.0]);
        assert!(a.dot(&b).is_err());
        assert!(a.add(&b).is_err());
    }

    #[test]
    fn kconservation_l1_satisfied() {
        let law = L1Conservation::probability();
        let s = KernelStateVector::from_vec(vec![0.25, 0.25, 0.25, 0.25]);
        assert!(law.is_satisfied(&s, 1e-10));
        let modified = KernelStateVector::from_vec(vec![0.5, 0.25, 0.25, 0.25]);
        assert!(!law.is_satisfied(&modified, 1e-10));
    }

    #[test]
    fn kconservation_l2_satisfied() {
        let law = L2Conservation::unit_energy();
        let s = KernelStateVector::from_vec(vec![1.0, 0.0, 0.0]);
        assert!(law.is_satisfied(&s, 1e-10));
        let modified = KernelStateVector::from_vec(vec![2.0, 0.0, 0.0]);
        assert!(!law.is_satisfied(&modified, 1e-10));
    }

    #[test]
    fn kconservation_sum() {
        let law = SumConservation::new(10.0);
        let s = KernelStateVector::from_vec(vec![5.0, 5.0]);
        assert!(law.is_satisfied(&s, 1e-10));
    }

    #[test]
    fn kconservation_checker_default() {
        let mut checker = ConservationChecker::new(1e-10);
        checker.add_law(L2Conservation::unit_energy());
        let mut s = KernelStateVector::from_vec(vec![0.5, 0.5]);
        s.normalize();
        assert!(checker.check_all(&s).is_ok());
    }

    #[test]
    fn kresidual_monitor_convergence() {
        let mut m = ResidualMonitor::new(1e-6);
        m.record(0.1);
        m.record(0.01);
        m.record(0.0000001);
        assert!(m.is_converged(1));
        assert!((m.max_residual() - 0.1).abs() < 1e-12);
    }

    #[test]
    fn kop_monad_left_identity() {
        let x = 42;
        let f = |n: i32| Op::pure(n * 2);
        let lhs = Op::pure(x).bind(f);
        let rhs = f(x);
        assert_eq!(lhs.unwrap(), rhs.unwrap());
    }

    #[test]
    fn kop_monad_right_identity() {
        let m = Op::pure(42);
        let r = m.bind(Op::pure);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn kop_monad_associativity() {
        let f = |n: i32| Op::pure(n + 1);
        let g = |n: i32| Op::pure(n * 3);
        let lhs = Op::pure(2).bind(f).bind(g);
        let rhs = Op::pure(2).bind(|x| f(x).bind(g));
        assert_eq!(lhs.unwrap(), rhs.unwrap());
    }

    #[test]
    fn kstate_monad_counter() {
        let counter = StateOp::get()
            .bind(|n: i32| StateOp::put(n + 1))
            .bind(|_| StateOp::get())
            .bind(|n| StateOp::pure(n * 2));
        let (result, final_state) = counter.run(0);
        assert_eq!(result, 2);
        assert_eq!(final_state, 1);
    }

    #[test]
    fn kio_monad_pure_and_run() {
        let io = IO::pure(7).map(|x| x * 3);
        assert_eq!(io.run(), 21);
    }

    #[test]
    fn kbuiltin_types() {
        assert_eq!(builtin::Unit, builtin::Unit);
        assert_eq!(builtin::unit_type(), TypeIdentifier::of::<builtin::Unit>());
        assert_eq!(
            builtin::state_vector_type(),
            TypeIdentifier::new("StateVector")
        );
    }

    #[test]
    fn ktype_tag_default() {
        let _t: TypeTag<i32> = Default::default();
        assert_eq!(TypeTag::<i32>::type_id(), TypeIdentifier::of::<i32>());
    }

    #[test]
    fn ktypecheck_trait_provides_default_impls() {
        struct Foo;
        impl TypeCheck for Foo {
            fn input_type(&self) -> TypeIdentifier {
                builtin::unit_type()
            }
            fn output_type(&self) -> TypeIdentifier {
                builtin::any_type()
            }
        }
        let foo = Foo;
        assert!(foo.check_input(&builtin::unit_type()));
        assert!(foo.check_output(&builtin::state_vector_type())); // any 匹配任意
        let pair = foo.type_pair();
        assert_eq!(pair.input, builtin::unit_type());
        assert_eq!(pair.output, builtin::any_type());
    }
}
