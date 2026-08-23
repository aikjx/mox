//! ⛨ 璇玑验证网关（最高权限）
//!
//! 在 flow-ai 求解之后、治理闸门之前插入。所有检查均为**数学/语义正确性**判定，
//! 任何 RBAC / 合规 / 权限专家的结论都不可覆盖本层结论。
//! 任一阻断级检查失败 → `vetoed = true` → 治理闸门必须 BLOCK（记录 `algorithm_veto`）。
//!
//! 模块布局（5 个守恒不变量各一文件 + 测试 + CEM 优化）：
//! - `topology`：5a 拓扑守恒（原始节点全保留 + 真数据依赖可达性守恒）
//! - `data_dep`：5b 数据依赖守恒（剪除伪依赖不破坏真依赖 + RAW 冒险探测）
//! - `conflict`：5c 冲突消解守恒（0 阻塞冲突 + 无悬空异常边）
//! - `gains`：5d 收益可信（speedup≥1 且并行不慢于串行）
//! - `code_rt`：5e 代码往返一致（仅 emit_code 时，告警不阻断）
//! - `cem`：T9 多目标 CEM 搜索（memoization + 并行 fitness + 目标感知剪枝）

mod cem;
mod code_rt;
mod conflict;
mod data_dep;
mod gains;
mod topology;
#[cfg(test)]
mod tests;

pub use cem::{
    cem_deep_chain_with_defaults, CemConfig, CemResult, CemStopReason, ConstraintSpec,
    EvalCacheKey, EvalMemo, ObjectiveSpec,
};
pub use code_rt::code_roundtrip_invariant;
pub use conflict::conflict_invariant;
pub use data_dep::{data_dependency_invariant, path_preserves_data_dep};
pub use gains::credible_gains_invariant;
pub use topology::topology_invariant;

use flow_ai::model::FlowGraph;
use flow_ai::pipeline::OptimizationReport;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// 单条验证结论
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    /// 检查名：topology / data_dep / conflict / gains / code_rt
    pub name: String,
    /// 是否通过
    pub passed: bool,
    /// 是否阻断级（失败则整体否决）
    pub blocking: bool,
    /// 人类可读说明；失败时为反例
    pub detail: String,
}

/// 璇玑验证报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgoVerification {
    pub checks: Vec<Check>,
    /// 全部通过
    pub all_passed: bool,
    /// 任一阻断级检查失败 → 治理必须 BLOCK
    pub vetoed: bool,
    pub summary: String,
}

impl AlgoVerification {
    pub fn check(&self, name: &str) -> Option<&Check> {
        self.checks.iter().find(|c| c.name == name)
    }
}

/// 最高权限验证：优化前图 vs 优化报告
///
/// 性能说明（T9）：
///   · 5 项检查相互独立 → 用 rayon 并行派发；
///   · `topology` 与 `data_dep` 都要计算 `after.reachability()`，复用一次即可。
pub fn verify(before: &FlowGraph, opt: &OptimizationReport) -> AlgoVerification {
    let after = &opt.optimized_graph;

    // （一次性）计算拓扑守恒 / 数据依赖守恒都要用的 after 可达性
    // 深链下 after.reachability() ≈ 70% 的 verify 耗时；只算一次能省 ~2 次再算。
    let after_reach_owned = after.reachability_owned();

    // 并行：把 5 项独立检查作为 trait object 动态派发（捕获上下文的闭包，Rayon 可收集）
    let steps: Vec<Box<dyn FnOnce() -> Check + Send>> = vec![
        Box::new(|| topology::topology_invariant_with_reach(before, after, Some(&after_reach_owned))),
        Box::new(|| {
            data_dep::data_dependency_invariant_with_reach(
                before,
                after,
                opt,
                Some(&after_reach_owned),
            )
        }),
        Box::new(|| conflict::conflict_invariant(after, opt)),
        Box::new(|| gains::credible_gains_invariant(opt)),
        Box::new(|| code_rt::code_roundtrip_invariant(opt)),
    ];
    let checks: Vec<Check> = steps.into_par_iter().map(|f| f()).collect();

    let vetoed = checks.iter().any(|c| c.blocking && !c.passed);
    let all_passed = checks.iter().all(|c| c.passed);
    let summary = if vetoed {
        format!(
            "⛨ 算法否决：{} 项阻断级检查未通过（语义/依赖/一致性被破坏）",
            checks.iter().filter(|c| c.blocking && !c.passed).count()
        )
    } else {
        format!("⛨ 算法验证通过：{} 项检查全部可信", checks.len())
    };

    AlgoVerification {
        checks,
        all_passed,
        vetoed,
        summary,
    }
}
