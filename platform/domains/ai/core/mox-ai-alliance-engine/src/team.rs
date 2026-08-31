// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 专家组队器（FR-CORE-03）：
//!   专家注册表（14 维 × 7 类基准能力） →
//!   按"7 类匹配分 × gate_A 率 × dim_priority 权重"综合排序 →
//!   EAF-STD 4.2 安全类强制替换末位（安全/权限敏感场景）→ 同维去重 → 输出 Top N。
//!
//! # 设计
//! - `ExpertRegistry` — 可注入的专家注册表 trait（支持自定义实现）
//! - `TeamAssembler` — 组队优化器，封装排序/去重/EAF 规则
//! - 默认提供 `build_default_registry()` 构建 14 维内置专家

use crate::constants::INTENT_CLASSES;
use crate::intent::IntentResult;
use mox_ai_expert_proto::Dimension;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub type ExpertId = String;

/// 专家元信息（注册表条目）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertMeta {
    pub expert_id: ExpertId,
    pub dimension: Dimension,
    /// 支持的 7 类基准子集（必须是 INTENT_CLASSES 的 subset）
    pub supported_classes: BTreeSet<String>,
    /// 平均单次分析延迟（ms），用于排序时的"速度奖励"
    pub avg_latency_ms: u64,
    /// 近 30 日 Gate A 级通过率（0..1），越高越好
    pub gate_a_rate_30d: f64,
    /// 维度优先级（SSOT：Dimension::priority()，100 为最高 Permission/Security）
    pub priority: i32,
    /// 自由备注
    pub description: String,
}

/// 组队结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamResult {
    /// 最终胜出的专家 id 列表（长度 <= team_size）
    pub team_ids: Vec<ExpertId>,
    /// 被 EAF 4.2 安全规则强制替换的说明（为空表示没触发）
    pub forced_replacements: Vec<String>,
    /// 可解释性矩阵：每个入选专家的打分拆解，便于前端显示
    pub reasoning_matrix: BTreeMap<ExpertId, ScoreBreakdown>,
    /// 诊断 id
    pub diagnose_id: Uuid,
}

/// 单专家打分拆解
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub class_match: f64,
    pub gate_a_rate: f64,
    pub priority_score: f64,
    pub latency_reward: f64,
    pub total: f64,
}

// ================== 专家注册表 trait ==================

/// 专家注册表 trait（可注入）
///
/// 联盟引擎通过此 trait 获取专家列表，不依赖具体的注册中心实现。
/// 上游服务可实现此 trait 对接数据库 / 配置中心 / 远程服务等。
pub trait ExpertRegistry: Send + Sync + std::fmt::Debug {
    /// 获取所有专家元信息
    fn all_experts(&self) -> Vec<ExpertMeta>;

    /// 按 id 获取专家
    fn get_expert(&self, id: &str) -> Option<ExpertMeta> {
        self.all_experts().into_iter().find(|e| e.expert_id == id)
    }

    /// 注册表整体是否覆盖 7 类（HC-9 合规）
    fn covers_7_classes(&self) -> bool {
        let mut union: BTreeSet<String> = BTreeSet::new();
        for meta in self.all_experts() {
            union.extend(meta.supported_classes.iter().cloned());
        }
        INTENT_CLASSES.iter().all(|c| union.contains(*c))
    }
}

/// 内置 14 维专家注册表（默认实现）
#[derive(Debug, Clone, Default)]
pub struct DefaultExpertRegistry {
    experts: BTreeMap<ExpertId, ExpertMeta>,
}

impl DefaultExpertRegistry {
    /// 构建默认 14 维专家注册表
    pub fn new() -> Self {
        Self {
            experts: build_default_experts(),
        }
    }

    /// 获取专家数量
    pub fn len(&self) -> usize {
        self.experts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.experts.is_empty()
    }
}

impl ExpertRegistry for DefaultExpertRegistry {
    fn all_experts(&self) -> Vec<ExpertMeta> {
        self.experts.values().cloned().collect()
    }

    fn get_expert(&self, id: &str) -> Option<ExpertMeta> {
        self.experts.get(id).cloned()
    }
}

// ================== 组队器 ==================

/// 专家组队器
///
/// 封装组队优化逻辑，支持可注入的专家注册表。
#[derive(Debug, Clone)]
pub struct TeamAssembler {
    registry: std::sync::Arc<dyn ExpertRegistry>,
}

impl TeamAssembler {
    /// 使用默认 14 维专家注册表创建组队器
    pub fn new() -> Self {
        Self {
            registry: std::sync::Arc::new(DefaultExpertRegistry::new()),
        }
    }

    /// 使用自定义注册表创建组队器
    pub fn with_registry<R: ExpertRegistry + 'static>(registry: R) -> Self {
        Self {
            registry: std::sync::Arc::new(registry),
        }
    }

    /// 执行组队优化
    ///
    /// - `intent`: 意图识别结果
    /// - `team_size`: 目标组队大小（3~7 推荐 4）
    /// - `is_sensitive`: 是否为敏感场景（安全/权限命中则强制 EAF 4.2）
    pub fn assemble(&self, intent: &IntentResult, team_size: usize, is_sensitive: bool) -> TeamResult {
        let experts = self.registry.all_experts();
        let registry_map: BTreeMap<ExpertId, ExpertMeta> = experts
            .into_iter()
            .map(|e| (e.expert_id.clone(), e))
            .collect();
        optimize_team_inner(intent, &registry_map, team_size, is_sensitive)
    }

    /// 获取注册表引用
    pub fn registry(&self) -> &dyn ExpertRegistry {
        &*self.registry
    }
}

impl Default for TeamAssembler {
    fn default() -> Self {
        Self::new()
    }
}

// ================== 函数式 API（向后兼容） ==================

/// 构建默认 14 维专家注册表（兼容旧代码）
pub fn build_expert_registry() -> BTreeMap<ExpertId, ExpertMeta> {
    build_default_experts()
}

/// HC-9 合规校验：注册表作为整体覆盖 7 类
pub fn registry_coverage_check(reg: &BTreeMap<ExpertId, ExpertMeta>) -> bool {
    let mut union: BTreeSet<String> = BTreeSet::new();
    for meta in reg.values() {
        union.extend(meta.supported_classes.iter().cloned());
    }
    INTENT_CLASSES.iter().all(|c| union.contains(*c))
}

/// 组队优化（函数式 API，向后兼容）
pub fn optimize_team(
    intent: &IntentResult,
    registry: &BTreeMap<ExpertId, ExpertMeta>,
    team_size: usize,
    is_sensitive: bool,
) -> TeamResult {
    optimize_team_inner(intent, registry, team_size, is_sensitive)
}

// ================== 内部实现 ==================

fn build_default_experts() -> BTreeMap<ExpertId, ExpertMeta> {
    let experts_14: [(Dimension, &str, &[&str]); 14] = [
        (Dimension::Permission,       "permission",        &["code","logic","knowledge","instruction","chinese","timeliness","math"]),
        (Dimension::Security,         "security",          &["code","logic","knowledge","instruction","chinese","timeliness","math"]),
        (Dimension::Architecture,     "architecture",      &["code","logic","knowledge","instruction","timeliness"]),
        (Dimension::SecurityCode,     "security_code",     &["code","logic","knowledge","instruction"]),
        (Dimension::Resource,         "resource",          &["code","knowledge","instruction","timeliness"]),
        (Dimension::Data,             "data",              &["code","math","logic","knowledge","timeliness","chinese"]),
        (Dimension::CodeQuality,      "code_quality",      &["code","logic","instruction","knowledge"]),
        (Dimension::Performance,      "performance",       &["code","math","logic","instruction","timeliness","knowledge"]),
        (Dimension::Algorithm,        "algorithm",         &["math","logic","code","knowledge","instruction"]),
        (Dimension::Testing,          "testing",           &["code","logic","instruction","knowledge","timeliness"]),
        (Dimension::Business,         "business",          &["knowledge","chinese","timeliness","instruction","logic","code"]),
        (Dimension::Documentation,    "documentation",     &["knowledge","chinese","code","instruction","logic"]),
        (Dimension::Observability,    "observability",     &["code","logic","instruction","timeliness","knowledge"]),
        (Dimension::Maintainability,  "maintainability",   &["code","logic","knowledge","instruction","chinese"]),
    ];

    let mut m = BTreeMap::new();
    let gate_a_map: BTreeMap<Dimension, f64> = [
        (Dimension::Permission, 0.98), (Dimension::Security, 0.98),
        (Dimension::Architecture, 0.96), (Dimension::SecurityCode, 0.96),
        (Dimension::Data, 0.94), (Dimension::Algorithm, 0.94),
        (Dimension::Resource, 0.92), (Dimension::CodeQuality, 0.93),
        (Dimension::Performance, 0.92), (Dimension::Testing, 0.91),
        (Dimension::Business, 0.90), (Dimension::Documentation, 0.89),
        (Dimension::Observability, 0.88), (Dimension::Maintainability, 0.87),
    ].into_iter().collect();
    let latency_map: BTreeMap<Dimension, u64> = [
        (Dimension::Permission, 120), (Dimension::Security, 150),
        (Dimension::Architecture, 200), (Dimension::SecurityCode, 180),
        (Dimension::Data, 90), (Dimension::Algorithm, 220),
        (Dimension::Resource, 100), (Dimension::CodeQuality, 80),
        (Dimension::Performance, 250), (Dimension::Testing, 160),
        (Dimension::Business, 60), (Dimension::Documentation, 50),
        (Dimension::Observability, 70), (Dimension::Maintainability, 75),
    ].into_iter().collect();

    for (dim, id, classes) in experts_14.iter() {
        let sid: ExpertId = (*id).to_string();
        let supported: BTreeSet<String> = classes.iter().map(|s| s.to_string()).collect();
        let meta = ExpertMeta {
            expert_id: sid.clone(),
            dimension: *dim,
            supported_classes: supported,
            avg_latency_ms: *latency_map.get(dim).unwrap_or(&150),
            gate_a_rate_30d: *gate_a_map.get(dim).unwrap_or(&0.90),
            priority: dim.priority(),
            description: format!("{:?} 领域专家（14 维）", dim),
        };
        m.insert(sid, meta);
    }
    m
}

fn optimize_team_inner(
    intent: &IntentResult,
    registry: &BTreeMap<ExpertId, ExpertMeta>,
    team_size: usize,
    is_sensitive: bool,
) -> TeamResult {
    let team_size = team_size.clamp(2, 7);
    let diagnose_id = Uuid::new_v4();

    let max_priority: i32 = 100;
    let max_latency_ms: u64 = 500;
    let mut scores: Vec<(ExpertId, ScoreBreakdown)> = registry
        .iter()
        .map(|(id, meta)| {
            let mut class_match = 0.1_f64;
            let mut top2: Vec<String> = {
                let mut pairs: Vec<(String, f64)> = intent
                    .rrf_scores
                    .iter()
                    .map(|(a, b)| (a.clone(), *b))
                    .collect();
                pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                pairs.into_iter().take(2).map(|(k, _)| k).collect()
            };
            if top2.is_empty() {
                top2.push(intent.intent_id.clone());
            }
            for (i, cls) in top2.iter().enumerate() {
                if meta.supported_classes.contains(cls) {
                    class_match = class_match.max(if i == 0 { 1.0 } else { 0.5 });
                }
            }

            let gate_a = meta.gate_a_rate_30d.clamp(0.0, 1.0);
            let priority_score = (meta.priority as f64 / max_priority as f64).clamp(0.0, 1.0);
            let latency_reward = (1.0 - (meta.avg_latency_ms as f64 / max_latency_ms as f64)).max(0.0);

            let total = 0.60 * class_match
                + 0.25 * gate_a
                + 0.10 * priority_score
                + 0.05 * latency_reward;

            (
                id.clone(),
                ScoreBreakdown {
                    class_match,
                    gate_a_rate: gate_a,
                    priority_score,
                    latency_reward,
                    total,
                },
            )
        })
        .collect();

    scores.sort_by(|a, b| {
        b.1.total
            .partial_cmp(&a.1.total)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut seen_dim = BTreeSet::new();
    let mut team_ids: Vec<ExpertId> = Vec::new();
    let mut reasoning: BTreeMap<ExpertId, ScoreBreakdown> = BTreeMap::new();
    for (id, bd) in &scores {
        let Some(meta) = registry.get(id) else { continue };
        if !seen_dim.insert(meta.dimension) {
            continue;
        }
        if meta.supported_classes.is_empty() {
            continue;
        }
        reasoning.insert(id.clone(), bd.clone());
        team_ids.push(id.clone());
        if team_ids.len() >= team_size {
            break;
        }
    }

    // EAF-STD 4.2 强制替换
    let mut forced_replacements = Vec::new();
    let eaf_trigger = is_sensitive
        || (matches!(
            intent.intent_id.as_str(),
            "code" | "logic" | "knowledge" | "instruction"
        ) && intent.conf > 0.6);

    if eaf_trigger {
        if is_sensitive {
            for gate in [Dimension::Security, Dimension::Permission] {
                let already = team_ids
                    .iter()
                    .position(|id| {
                        registry
                            .get(id)
                            .map(|m| m.dimension == gate)
                            .unwrap_or(false)
                    });
                if let Some(pos) = already {
                    let removed = team_ids.remove(pos);
                    reasoning.remove(&removed);
                    let tag = if matches!(gate, Dimension::Security) {
                        "SECURITY"
                    } else {
                        "PERMISSION"
                    };
                    forced_replacements.push(format!(
                        "EAF-4.2({tag}): 显式敏感场景强制重走门禁路径：先移除自然入选的专家 {removed}，后续再强制 {tag} 入队（审计留痕）",
                        tag = tag,
                        removed = removed,
                    ));
                }
            }
        }

        let need_security = !team_ids.iter().any(|id| registry.get(id).map(|m| matches!(m.dimension, Dimension::Security)).unwrap_or(false));
        let need_permission = !team_ids.iter().any(|id| registry.get(id).map(|m| matches!(m.dimension, Dimension::Permission)).unwrap_or(false));

        if need_security {
            let security_id: Option<ExpertId> = registry
                .iter()
                .find(|(_, m)| matches!(m.dimension, Dimension::Security) && !team_ids.contains(&m.expert_id))
                .map(|(k, _)| k.clone());
            if let Some(new_id) = security_id {
                if !team_ids.is_empty() {
                    let kicked = team_ids.pop().unwrap();
                    forced_replacements.push(format!(
                        "EAF-4.2(SECURITY): 强制替换末位专家 {} → Security 专家 {}（敏感场景门禁）",
                        kicked, new_id
                    ));
                    reasoning.remove(&kicked);
                    if let Some(m) = registry.get(&new_id) {
                        reasoning.insert(new_id.clone(), ScoreBreakdown {
                            class_match: 0.8,
                            gate_a_rate: m.gate_a_rate_30d,
                            priority_score: (m.priority as f64 / 100.0).clamp(0.0,1.0),
                            latency_reward: (1.0 - (m.avg_latency_ms as f64 / 500.0)).max(0.0),
                            total: 0.90,
                        });
                    }
                    team_ids.push(new_id);
                }
            }
        }
        if need_permission {
            let permission_id: Option<ExpertId> = registry
                .iter()
                .find(|(_, m)| matches!(m.dimension, Dimension::Permission) && !team_ids.contains(&m.expert_id))
                .map(|(k, _)| k.clone());
            if let Some(new_id) = permission_id {
                if !team_ids.is_empty() {
                    if team_ids.len() >= team_size {
                        let kicked = team_ids.pop().unwrap();
                        forced_replacements.push(format!(
                            "EAF-4.2(PERMISSION): 强制替换末位专家 {} → Permission 专家 {}（敏感场景门禁）",
                            kicked, new_id
                        ));
                        reasoning.remove(&kicked);
                    } else {
                        forced_replacements.push(format!(
                            "EAF-4.2(PERMISSION): 敏感场景追加 Permission 专家 {}（不替换，队未满）",
                            new_id
                        ));
                    }
                    if let Some(m) = registry.get(&new_id) {
                        reasoning.insert(new_id.clone(), ScoreBreakdown {
                            class_match: 0.8,
                            gate_a_rate: m.gate_a_rate_30d,
                            priority_score: (m.priority as f64 / 100.0).clamp(0.0,1.0),
                            latency_reward: (1.0 - (m.avg_latency_ms as f64 / 500.0)).max(0.0),
                            total: 0.90,
                        });
                    }
                    team_ids.push(new_id);
                }
            }
        }

        while team_ids.len() < team_size {
            let mut added = false;
            for (id, bd) in &scores {
                let Some(meta) = registry.get(id) else { continue };
                if team_ids.contains(id) {
                    continue;
                }
                if seen_dim.contains(&meta.dimension) {
                    continue;
                }
                if meta.supported_classes.is_empty() {
                    continue;
                }
                seen_dim.insert(meta.dimension);
                reasoning.insert(id.clone(), bd.clone());
                team_ids.push(id.clone());
                added = true;
                break;
            }
            if !added {
                break;
            }
        }
    }

    TeamResult {
        team_ids,
        forced_replacements,
        reasoning_matrix: reasoning,
        diagnose_id,
    }
}

// ================== TDD 测试 ==================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::IntentResult;

    fn fake_intent(winner: &str) -> IntentResult {
        let mut rrf = BTreeMap::new();
        rrf.insert(winner.into(), 0.90);
        for c in INTENT_CLASSES {
            rrf.entry(c.to_string()).or_insert(0.0);
        }
        IntentResult {
            intent_id: winner.into(),
            conf: 0.9,
            keyword_scores: Default::default(),
            spread_scores: Default::default(),
            rrf_scores: rrf,
            degraded: false,
            degrade_reason: None,
            seeds_hit: vec![],
            trace_log: String::new(),
            diagnose_id: uuid::Uuid::new_v4(),
        }
    }

    #[test]
    fn test_team_security_replace() {
        let reg = build_expert_registry();
        assert!(reg.get("security").is_some());
        assert!(reg.get("permission").is_some());

        let intent = fake_intent("code");
        let res = optimize_team(&intent, &reg, 3, true);
        assert_eq!(res.team_ids.len(), 3);
        assert!(
            res.team_ids.iter().any(|id| id == "security"),
            "EAF 4.2 强制替换失败：team 内无 security 专家；team={:?}",
            res.team_ids
        );
        assert!(!res.forced_replacements.is_empty());
    }

    #[test]
    fn test_team_size_four_and_no_dup_dimension() {
        let reg = build_expert_registry();
        assert!(registry_coverage_check(&reg));
        assert_eq!(reg.len(), 14);

        for sz in [3usize, 4, 5] {
            let intent = fake_intent("knowledge");
            let res = optimize_team(&intent, &reg, sz, false);
            assert!(
                res.team_ids.len() == sz || res.team_ids.len() == sz + 1,
                "组队大小 {} 偏差：实际 {}",
                sz, res.team_ids.len()
            );
            let mut seen = std::collections::HashSet::new();
            for id in &res.team_ids {
                let dim = reg.get(id).expect("team id must exist in registry").dimension;
                assert!(seen.insert(dim), "同维度重复进队：dim={:?}, id={}", dim, id);
            }
        }
    }

    #[test]
    fn registry_hc9_7_classes_full_coverage() {
        let reg = build_expert_registry();
        assert!(registry_coverage_check(&reg));
    }

    // TeamAssembler 结构体测试
    #[test]
    fn team_assembler_struct_works() {
        let assembler = TeamAssembler::new();
        let intent = fake_intent("code");
        let res = assembler.assemble(&intent, 4, true);
        assert!(!res.team_ids.is_empty());
        assert!(res.team_ids.iter().any(|id| id == "security"));
    }

    #[test]
    fn default_expert_registry_covers_7_classes() {
        let reg = DefaultExpertRegistry::new();
        assert!(reg.covers_7_classes());
        assert_eq!(reg.len(), 14);
    }

    #[test]
    fn expert_meta_priority_from_dimension() {
        let reg = build_expert_registry();
        let perm = reg.get("permission").unwrap();
        let algo = reg.get("algorithm").unwrap();
        assert!(perm.priority > algo.priority, "Permission 优先级应高于 Algorithm");
        assert_eq!(perm.priority, Dimension::Permission.priority());
    }
}
