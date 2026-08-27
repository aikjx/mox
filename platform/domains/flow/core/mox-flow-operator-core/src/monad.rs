// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 单子模式实现（已迁移至 mox-platform-operator-core）
//!
//! 纯单子核心（Op/StateOp/IO）及 From<Result> 已迁移至 `mox-platform-operator-core::monad`。
//! 本模块重新导出以保持向后兼容，并通过扩展 trait 补充依赖 Operator/ExecutionContext 的方法。

use crate::operator::Operator;
use crate::state::StateVector;
use crate::ExecutionContext;

// ===== 重导出平台算子核心的单子模块 =====
pub use mox_platform_operator_core::monad::*;

/// 算子单子扩展 trait（依赖 Operator trait + ExecutionContext，所以保留在 flow 域）
pub trait OperatorMonadExt: Sized {
    /// 应用算子到 StateVector
    fn apply_operator<O: Operator>(self, op: &O, ctx: &mut ExecutionContext) -> Op<StateVector>;
}

impl<T> OperatorMonadExt for Op<T>
where
    T: Into<StateVector>,
{
    fn apply_operator<O: Operator>(self, op: &O, ctx: &mut ExecutionContext) -> Op<StateVector> {
        self.bind(|input| {
            let state = input.into();
            match op.apply(&state, ctx) {
                Ok(output) => Op::pure(output).log(format!("算子 {} 执行成功", op.metadata().name)),
                Err(e) => Op::fail(e.to_string()),
            }
        })
    }
}
