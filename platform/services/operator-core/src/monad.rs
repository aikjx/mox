//! # 单子模式实现
//!
//! 实现公理6：扩展性闭包
//! 使用单子模式封装副作用，支持算子的纯函数式组合。
//! 纯单子核心（Op/StateOp/IO）已移至 `kernel.rs`，本模块重导出并补充：
//! - `From<Result<T>>`（依赖外部 OperatorError）
//! - `apply_operator` 扩展方法（依赖 Operator trait + ExecutionContext）

use crate::operator::Operator;
use crate::state::StateVector;
use crate::{ExecutionContext, Result};

// ===== 重导出 L6 纯内核单子 =====
pub use crate::kernel::{Op, StateOp, IO};

// ===== 为 Op 补充依赖上层能力的 impl =====
impl<T> Op<T> {
    /// 应用算子到 StateVector（依赖 Operator trait；非纯内核能力所以在此扩展）
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

/// 从 crate::Result 转换（Kernel 层看不到 OperatorError，所以在此补充）
impl<T> From<Result<T>> for Op<T> {
    fn from(r: Result<T>) -> Self {
        match r {
            Ok(v) => Op::pure(v),
            Err(e) => Op::fail(e.to_string()),
        }
    }
}

// ===== 原有单子单元测试 =====

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
