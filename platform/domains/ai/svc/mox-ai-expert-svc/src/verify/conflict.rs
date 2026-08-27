// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 5c 冲突消解守恒：0 阻塞冲突 + 无悬空异常边

use crate::verify::Check;
use mox_ai_flow_svc::model::{FlowGraph, NodeKind};
use mox_ai_flow_svc::pipeline::OptimizationReport;

/// 5c 冲突消解守恒：0 阻塞冲突 + 无悬空异常边
pub fn conflict_invariant(after: &FlowGraph, opt: &OptimizationReport) -> Check {
    if opt.conflicts.has_blocking() {
        return Check {
            name: "conflict".into(),
            passed: false,
            blocking: true,
            detail: format!(
                "优化后仍存在 {} 个阻塞级冲突",
                opt.conflicts.blocking().len()
            ),
        };
    }
    // 悬空异常边：目标节点不存在，或不是 Guard/Handler/End 类型。
    // 放宽说明：异常 → 普通 End 作为「异常归档/终止」是合法业务语义，
    // 此前要求必须是 Guard/Handler 约束过严（迭代 4-① 优化项）。
    let mut dangling = Vec::new();
    for e in &after.edges {
        if matches!(e.kind, mox_ai_flow_svc::model::EdgeKind::Exception) {
            match after.node(&e.to) {
                None => dangling.push(format!("{}→{} 目标缺失", e.from, e.to)),
                Some(n) => {
                    if !matches!(n.kind, NodeKind::Guard | NodeKind::End)
                        && !is_handler_name(&n.name)
                    {
                        dangling.push(format!("{}→{} 目标非 Handler/Guard/End", e.from, e.to));
                    }
                }
            }
        }
    }
    if !dangling.is_empty() {
        return Check {
            name: "conflict".into(),
            passed: false,
            blocking: true,
            detail: format!("悬空异常边: {:?}", &dangling[..dangling.len().min(5)]),
        };
    }
    Check {
        name: "conflict".into(),
        passed: true,
        blocking: true,
        detail: format!(
            "阻塞冲突 0，异常边全部落点有效（{} 条）",
            opt.conflicts.conflicts.len()
        ),
    }
}

pub fn is_handler_name(name: &str) -> bool {
    name.contains("error")
        || name.contains("handler")
        || name.contains("错误处理")
        || name.starts_with("__")
}
