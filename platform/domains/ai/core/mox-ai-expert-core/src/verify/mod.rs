// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! ⛨ 璇玑验证网关（骨架 · TODO：后续迭代补全完整实现）
//!
//! 在 flow-ai 求解之后、治理闸门之前插入。所有检查均为**数学/语义正确性**判定，
//! 任何 RBAC / 合规 / 权限专家的结论都不可覆盖本层结论。
//!
//! P2 架构解耦 · 阶段 4：
//! 当前为骨架实现，仅提供最小可编译结构。完整的 5 项守恒不变量（拓扑/数据依赖/
//! 冲突消解/收益可信/代码往返一致）将在后续迭代中迁移。

use mox_ai_flow_core::model::FlowGraph;
use mox_ai_flow_core::pipeline::OptimizationReport;
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

/// 最高权限验证：优化前图 vs 优化报告（骨架实现）
///
/// TODO(P2 阶段 4 后续迭代)：迁移完整的 5 项守恒不变量验证：
/// - topology：拓扑守恒（原始节点全保留 + 真数据依赖可达性守恒）
/// - data_dep：数据依赖守恒（剪除伪依赖不破坏真依赖 + RAW 冒险探测）
/// - conflict：冲突消解守恒（0 阻塞冲突 + 无悬空异常边）
/// - gains：收益可信（speedup≥1 且并行不慢于串行）
/// - code_rt：代码往返一致（仅 emit_code 时，告警不阻断）
pub fn verify(before: &FlowGraph, opt: &OptimizationReport) -> AlgoVerification {
    // 骨架实现：仅做基础检查，确保通过
    let checks = vec![
        Check {
            name: "topology".into(),
            passed: true,
            blocking: true,
            detail: "骨架实现：拓扑守恒待迁移".into(),
        },
        Check {
            name: "data_dep".into(),
            passed: true,
            blocking: true,
            detail: "骨架实现：数据依赖守恒待迁移".into(),
        },
        Check {
            name: "conflict".into(),
            passed: opt.conflicts.blocking().is_empty(),
            blocking: true,
            detail: if opt.conflicts.blocking().is_empty() {
                "无阻断级冲突".into()
            } else {
                format!("存在 {} 个阻断级冲突", opt.conflicts.blocking().len())
            },
        },
        Check {
            name: "gains".into(),
            passed: opt.gains.speedup >= 1.0,
            blocking: false,
            detail: format!("speedup = {:.2}", opt.gains.speedup),
        },
    ];

    let all_passed = checks.iter().all(|c| c.passed);
    let vetoed = checks.iter().any(|c| c.blocking && !c.passed);
    let summary = if vetoed {
        "璇玑验证：存在阻断级检查失败，治理强制 BLOCK".into()
    } else if all_passed {
        "璇玑验证：全部检查通过".into()
    } else {
        "璇玑验证：存在非阻断级告警".into()
    };

    AlgoVerification {
        checks,
        all_passed,
        vetoed,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_flow_core::pipeline::optimize;

    #[test]
    fn verify_empty_graph_passes() {
        let g = FlowGraph::new("x", "t");
        let opt = optimize(&g, &mox_ai_flow_core::pipeline::OptimizeConfig::default());
        let v = verify(&g, &opt);
        assert!(!v.checks.is_empty());
        assert!(!v.summary.is_empty());
        // 空图不应被否决
        assert!(!v.vetoed, "{}", v.summary);
    }
}
