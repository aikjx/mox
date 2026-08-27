// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! # 单子模式实现（monad）
//!
//! 实现公理6：扩展性闭包。使用单子模式封装副作用，支持算子的纯函数式组合。
//! 纯单子核心（Op/StateOp/IO）定义在 `kernel.rs`，本模块补充依赖上层类型的 impl。

use crate::{OperatorError, Result};

// ===== 重导出纯内核单子类型 =====
pub use crate::kernel::{Op, StateOp, IO};

/// 从 crate::Result 转换
impl<T> From<Result<T>> for Op<T> {
    fn from(r: Result<T>) -> Self {
        match r {
            Ok(v) => Op::pure(v),
            Err(e) => Op::fail(e.to_string()),
        }
    }
}

/// 从 OperatorError 转换
impl<T> From<OperatorError> for Op<T> {
    fn from(e: OperatorError) -> Self {
        Op::fail(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_op_monad_left_identity() {
        let x = 42;
        let f = |n: i32| Op::pure(n * 2);
        let lhs = Op::pure(x).bind(f);
        let rhs = f(x);
        assert_eq!(lhs.unwrap(), rhs.unwrap());
    }

    #[test]
    fn test_op_monad_right_identity() {
        let m = Op::pure(42);
        let result = m.bind(Op::pure);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_op_monad_associativity() {
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

    #[test]
    fn test_from_result() {
        let ok: Result<i32> = Ok(42);
        let op: Op<i32> = ok.into();
        assert_eq!(op.unwrap(), 42);

        let err: Result<i32> = Err(OperatorError::ExecutionError("test".into()));
        let op: Op<i32> = err.into();
        assert!(op.is_err());
    }
}
