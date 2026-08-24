//! 架构专家（开发维度）：审查代码架构、模块边界、依赖关系
//!
//! 分析基于 `CodeUnit` 的**预分析真字段**（`coupling`）与 `dependencies` 拓扑，
//! 循环依赖用真实 DFS 检测（不再返回固定值）。

use crate::context::ExpertContext;
use crate::expert::{Expert, ExpertOpinion, Risk, Suggestion};
use crate::ir::{CodeUnit, Dimension, ExpertId};
use flow_ai::model::Severity;
use std::collections::HashMap;

/// 架构专家：审查系统架构设计
pub struct ArchitectureExpert;

impl Expert for ArchitectureExpert {
    fn id(&self) -> ExpertId {
        "architecture".to_string()
    }

    fn dimension(&self) -> Dimension {
        Dimension::Architecture
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        let mut risks = Vec::new();
        let mut suggestions = Vec::new();
        let mut score = 1.0;

        if let Some(code_ir) = &ctx.code_ir {
            // 1. 模块循环依赖（真实拓扑 DFS 检测）
            if has_circular_dependency(&code_ir.units) {
                risks.push(Risk {
                    severity: Severity::Blocking,
                    nodes: code_ir.units.iter().map(|u| u.id.clone()).collect(),
                    dimension: Dimension::Architecture,
                    message: "检测到模块循环依赖".to_string(),
                    remediation: Some("重构模块依赖关系，打破循环".to_string()),
                    veto: false,
                });
                score *= 0.6;
            }

            // 2. 模块大小（行数）
            for unit in &code_ir.units {
                if unit.lines_of_code > 500 {
                    suggestions.push(Suggestion::Split);
                    score *= 0.9;
                }
            }

            // 3. 依赖深度（基于 dependencies 链式 DFS，而非简单计数）
            let dep_map = build_dep_map(&code_ir.units);
            let max_depth = dep_map
                .keys()
                .map(|id| max_dependency_depth(id, &dep_map, &mut HashMap::new()))
                .max()
                .unwrap_or(0);
            if max_depth > 5 {
                risks.push(Risk {
                    severity: Severity::Warning,
                    nodes: Vec::new(),
                    dimension: Dimension::Architecture,
                    message: format!("依赖深度过深（{}层），增加维护成本", max_depth),
                    remediation: Some("扁平化依赖结构".to_string()),
                    veto: false,
                });
                score *= 0.85;
            }

            // 4. 耦合度（预分析字段 0..1）
            for unit in &code_ir.units {
                if unit.coupling > 0.7 {
                    risks.push(Risk {
                        severity: Severity::Warning,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::Architecture,
                        message: format!("模块 {} 耦合度过高（{:.1}）", unit.name, unit.coupling),
                        remediation: Some("解耦模块、缩小接口面".to_string()),
                        veto: false,
                    });
                    score *= 0.85;
                }
            }
        }

        ExpertOpinion {
            expert: self.id(),
            dimension: Dimension::Architecture,
            constraints: Vec::new(),
            risks,
            score,
            metrics: Default::default(),
            suggestions,
            skipped: false,
            skip_reason: None,
        }
    }
}

/// 基于依赖列表构建 id -> 依赖集合 的映射
fn build_dep_map(units: &[CodeUnit]) -> HashMap<String, Vec<String>> {
    units
        .iter()
        .map(|u| (u.id.clone(), u.dependencies.clone()))
        .collect()
}

/// DFS 计算从 `id` 出发的最长依赖链深度
fn max_dependency_depth(
    id: &str,
    dep_map: &HashMap<String, Vec<String>>,
    cache: &mut HashMap<String, usize>,
) -> usize {
    if let Some(&d) = cache.get(id) {
        return d;
    }
    let deps = dep_map.get(id).cloned().unwrap_or_default();
    let depth = if deps.is_empty() {
        1
    } else {
        deps.iter()
            .map(|d| max_dependency_depth(d, dep_map, cache) + 1)
            .max()
            .unwrap_or(1)
    };
    cache.insert(id.to_string(), depth);
    depth
}

/// 检测依赖图中是否存在环（DFS + 三色标记）
fn has_circular_dependency(units: &[CodeUnit]) -> bool {
    let dep_map = build_dep_map(units);
    let mut color: HashMap<String, u8> = HashMap::new(); // 0=白 1=灰 2=黑
    for id in dep_map.keys() {
        if color.get(id).copied().unwrap_or(0) == 0 && dfs_has_cycle(id, &dep_map, &mut color) {
            return true;
        }
    }
    false
}

fn dfs_has_cycle(
    id: &str,
    dep_map: &HashMap<String, Vec<String>>,
    color: &mut HashMap<String, u8>,
) -> bool {
    color.insert(id.to_string(), 1);
    for next in dep_map.get(id).cloned().unwrap_or_default() {
        match color.get(&next).copied().unwrap_or(0) {
            1 => return true, // 灰节点 => 环
            0 if dfs_has_cycle(&next, dep_map, color) => {
                return true;
            }
            _ => {}
        }
    }
    color.insert(id.to_string(), 2);
    false
}
