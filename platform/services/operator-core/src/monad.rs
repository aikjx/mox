//! # 单子模式实现
//!
//! 实现公理6：扩展性闭包
//! 使用单子模式封装副作用，支持算子的纯函数式组合

use crate::operator::Operator;
use crate::state::StateVector;
use crate::{ExecutionContext, Result};

/// Op单子：封装可能失败的算子计算
/// 满足单子三定律：左单位、右单位、结合律
#[derive(Debug)]
pub struct Op<T> {
    value: Option<T>,
    error: Option<String>,
    logs: Vec<String>,
}

impl<T> Op<T> {
    /// return操作：将纯值注入单子
    pub fn pure(value: T) -> Self {
        Self {
            value: Some(value),
            error: None,
            logs: Vec::new(),
        }
    }

    /// 创建失败的Op
    pub fn fail(error: impl Into<String>) -> Self {
        Self {
            value: None,
            error: Some(error.into()),
            logs: Vec::new(),
        }
    }

    /// 添加日志
    pub fn log(mut self, msg: impl Into<String>) -> Self {
        self.logs.push(msg.into());
        self
    }

    /// 检查是否成功
    pub fn is_ok(&self) -> bool {
        self.value.is_some() && self.error.is_none()
    }

    /// 检查是否失败
    pub fn is_err(&self) -> bool {
        self.error.is_some()
    }

    /// 获取值
    pub fn unwrap(self) -> T {
        self.value.expect("Op was in error state")
    }

    /// 获取错误
    pub fn unwrap_err(self) -> String {
        self.error.expect("Op was in ok state")
    }

    /// 获取日志
    pub fn logs(&self) -> &[String] {
        &self.logs
    }

    /// 映射函数（函子）
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

    /// bind操作（>>=）：链式组合
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

    /// 应用算子到StateVector
    pub fn apply_operator<O: Operator>(self, op: &O, ctx: &mut ExecutionContext) -> Op<StateVector>
    where
        T: Into<StateVector>,
    {
        self.bind(|input| {
            let state = input.into();
            match op.apply(&state, ctx) {
                Ok(output) => Op::pure(output).log(format!("算子 {} 执行成功", op.metadata().name)),
                Err(e) => Op::fail(e.to_string()),
            }
        })
    }
}

/// 从Result转换
impl<T> From<Result<T>> for Op<T> {
    fn from(r: Result<T>) -> Self {
        match r {
            Ok(v) => Op::pure(v),
            Err(e) => Op::fail(e.to_string()),
        }
    }
}

/// 状态单子：封装带状态的计算
pub struct StateOp<S, A> {
    run: Box<dyn FnOnce(S) -> (A, S)>,
}

impl<S: 'static, A: 'static> StateOp<S, A> {
    pub fn new<F: FnOnce(S) -> (A, S) + 'static>(f: F) -> Self {
        Self { run: Box::new(f) }
    }

    /// return：左单位律
    pub fn pure(a: A) -> Self
    where
        A: Clone,
    {
        Self::new(move |s| (a.clone(), s))
    }

    /// bind操作
    pub fn bind<B: 'static, F: FnOnce(A) -> StateOp<S, B> + 'static>(self, f: F) -> StateOp<S, B> {
        StateOp::new(move |s| {
            let (a, s1) = (self.run)(s);
            (f(a).run)(s1)
        })
    }

    /// map操作
    pub fn map<B: 'static, F: FnOnce(A) -> B + 'static>(self, f: F) -> StateOp<S, B> {
        StateOp::new(move |s| {
            let (a, s1) = (self.run)(s);
            (f(a), s1)
        })
    }

    /// 运行状态计算
    pub fn run(self, initial_state: S) -> (A, S) {
        (self.run)(initial_state)
    }

    /// 只获取结果值
    pub fn eval(self, initial_state: S) -> A {
        self.run(initial_state).0
    }

    /// 只获取最终状态
    pub fn exec(self, initial_state: S) -> S {
        self.run(initial_state).1
    }
}

impl<S: 'static> StateOp<S, S> {
    /// 获取状态
    pub fn get() -> Self
    where
        S: Clone,
    {
        Self::new(|s| (s.clone(), s))
    }
}

impl<S: 'static> StateOp<S, ()> {
    /// 设置状态
    pub fn put(new_state: S) -> Self {
        Self::new(move |_| ((), new_state))
    }

    /// 修改状态
    pub fn modify<F: FnOnce(S) -> S + 'static>(f: F) -> Self {
        Self::new(move |s| ((), f(s)))
    }
}

/// IO单子：封装IO操作
pub struct IO<A> {
    perform: Box<dyn FnOnce() -> A>,
}

impl<A: 'static> IO<A> {
    pub fn new<F: FnOnce() -> A + 'static>(f: F) -> Self {
        Self {
            perform: Box::new(f),
        }
    }

    pub fn pure(a: A) -> Self
    where
        A: 'static,
    {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_op_monad_left_identity() {
        // 左单位律：return a >>= f = f a
        let x = 42;
        let f = |n: i32| Op::pure(n * 2);
        let lhs = Op::pure(x).bind(f);
        let rhs = f(x);
        assert_eq!(lhs.unwrap(), rhs.unwrap());
    }

    #[test]
    fn test_op_monad_right_identity() {
        // 右单位律：m >>= return = m
        let m = Op::pure(42);
        let result = m.bind(Op::pure);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_op_monad_associativity() {
        // 结合律：(m >>= f) >>= g = m >>= (\x -> f x >>= g)
        let f = |n: i32| Op::pure(n + 1);
        let g = |n: i32| Op::pure(n * 3);

        let lhs = Op::pure(2).bind(f).bind(g);
        let rhs = Op::pure(2).bind(|x| f(x).bind(g));
        assert_eq!(lhs.unwrap(), rhs.unwrap());
    }

    #[test]
    fn test_state_monad() {
        let counter = StateOp::get()
            .bind(|n: i32| StateOp::put(n + 1))
            .bind(|_| StateOp::get())
            .bind(|n| StateOp::pure(n * 2));

        let (result, final_state) = counter.run(0);
        assert_eq!(result, 2);
        assert_eq!(final_state, 1);
    }
}
