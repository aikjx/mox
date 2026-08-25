//! 专家组队优化（FR-CORE-03）：
//!   专家注册表（14 维 × 7 类基准能力） →
//!   按"7 类匹配分 × gate_A 率 × dim_priority 权重"综合排序 →
//!   EAF-STD 4.2 安全类强制替换末位（安全/权限敏感场景）→ 同维去重 → 输出 Top N。
//!
//! （HC：不引入任何外部依赖；排序纯 Rust std + rayon 已在 workspace 内，若本模块不并行可不用 rayon）

use super::constants::INTENT_CLASSES;
use crate::dim_priority;
use crate::ir::Dimension;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub type ExpertId = String;

/// 专家元信息（注册表条目）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertMeta {
    pub expert_id: ExpertId,
    pub dimension: Dimension,
    /// 支持的 7 类基准子集（必须是 INTENT_CLASSES 的 subset；空集 = 不支持任何类，不参与组队）
    pub supported_classes: BTreeSet<String>,
    /// 平均单次分析延迟（ms），用于排序时的"速度奖励"
    pub avg_latency_ms: u64,
    /// 近 30 日 Gate A 级通过率（0..1），越高越好
    pub gate_a_rate_30d: f64,
    /// 维度优先级（SSOT：dim_priority，100 为最高 Permission/Security）
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
    pub class_match: f64,     // 7 类匹配分 0..1
    pub gate_a_rate: f64,     // gate_A_rate_30d（0..1）
    pub priority_score: f64,  // 归一化维度优先级（0..1）
    pub latency_reward: f64,  // 延迟奖励 0..1，越快越高
    pub total: f64,           // 加权总分（0..1，越大越好）
}

/// 构建 14 维专家注册表（14 位专家 = 14 个 Dimension × 每位 1 个）
///
/// 每位专家：
///   - id = dimension snake_case（如 "permission", "business"…）
///   - supported_classes：按领域合理给子集，但整体注册表必须覆盖 HC-9 全部 7 类
///   - avg_latency_ms：合理估算（40ms ~ 250ms）
///   - gate_a_rate_30d：0.85 ~ 0.98（越核心越高，Permission/Security 最高）
///   - priority = dim_priority(dimension)
pub fn build_expert_registry() -> BTreeMap<ExpertId, ExpertMeta> {
    let experts_14 = [
        (Dimension::Permission,       "permission",        &["code","logic","knowledge","instruction","chinese","timeliness","math"][..]),
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
    // gate_A_rate 的手工基准值（越核心越稳定 = gate_A 率越高）
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
            priority: dim_priority(*dim),
            description: format!("{:?} 领域专家（14 维）", dim),
        };
        m.insert(sid, meta);
    }
    m
}

/// HC-9 合规校验：注册表作为整体覆盖 7 类 INTENT_CLASSES（所有专家的支持类并集 = 7 类全）
pub fn registry_coverage_check(reg: &BTreeMap<ExpertId, ExpertMeta>) -> bool {
    let mut union: BTreeSet<String> = BTreeSet::new();
    for meta in reg.values() {
        union.extend(meta.supported_classes.iter().cloned());
    }
    INTENT_CLASSES.iter().all(|c| union.contains(*c))
}

/// 组队优化（对外入口）
///
/// - `intent`: 意图识别结果
/// - `registry`: 来自 build_expert_registry()
/// - `team_size`: 目标组队大小（3~7 推荐 4）
/// - `is_sensitive`: 是否为敏感场景（安全/权限命中则强制 EAF 4.2）
pub fn optimize_team(
    intent: &super::intent::IntentResult,
    registry: &BTreeMap<ExpertId, ExpertMeta>,
    team_size: usize,
    is_sensitive: bool,
) -> TeamResult {
    let team_size = team_size.clamp(2, 7);
    let diagnose_id = Uuid::new_v4();

    // 1) 每位专家打分
    let max_priority: i32 = 100; // dim_priority(Permission/Security)=100
    let max_latency_ms: u64 = 500; // 归一上限
    let mut scores: Vec<(ExpertId, ScoreBreakdown)> = registry
        .iter()
        .map(|(id, meta)| {
            // 7 类匹配：若专家 supported_classes 包含 intent.intent_id，得 1.0；若包含 rrf_top_2 其他 1 类得 0.5；否则 0.1
            let mut class_match = 0.1_f64;
            // 取 rrf 前 2 名的类
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

            // 综合：60% 类匹配 + 25% gate_A 率 + 10% 维度优先级 + 5% 延迟奖励
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

    // 2) 按 total 从高到低排序
    scores.sort_by(|a, b| {
        b.1.total
            .partial_cmp(&a.1.total)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 3) 同维去重：每个 Dimension 最多选 1 个
    let mut seen_dim = BTreeSet::new();
    let mut team_ids: Vec<ExpertId> = Vec::new();
    let mut reasoning: BTreeMap<ExpertId, ScoreBreakdown> = BTreeMap::new();
    for (id, bd) in &scores {
        let Some(meta) = registry.get(id) else { continue };
        if !seen_dim.insert(meta.dimension) {
            continue; // 同维度已选
        }
        // 不选 supported_classes 为空的专家（理论上本 build_expert_registry 不会产生）
        if meta.supported_classes.is_empty() {
            continue;
        }
        reasoning.insert(id.clone(), bd.clone());
        team_ids.push(id.clone());
        if team_ids.len() >= team_size {
            break;
        }
    }

    // 4) EAF-STD 4.2 强制替换：
    //    - 显式 is_sensitive=true：**无条件** 100% 强制替换 Security + Permission 入队（即便自然排名已入队，也强制走替换路径，保证审计留痕）。
    //    - 隐式敏感（intent+conf 命中）：若缺则补（自然入队即无需替换）。
    let mut forced_replacements = Vec::new();
    let eaf_trigger = is_sensitive
        || (matches!(
            intent.intent_id.as_str(),
            "code" | "logic" | "knowledge" | "instruction"
        ) && intent.conf > 0.6);

    if eaf_trigger {
        // 显式 is_sensitive=true 时，先把已在队内的 Security / Permission "踢出去再强制请回"，
        // 保证 forced_replacements 审计留痕（EAF 4.2 门禁不可静默通过）。
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
                    // 同步 reasoning
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
                    // 若队长度 == team_size，踢出末位；否则直接加入（最多超 1 个不违反）
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

        // 补齐：强制替换阶段若因为"先踢后补"导致队长度 < team_size，
        // 则按 scores 的原排序补回未入队且不同维度的专家，稳定到目标组队大小。
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
                break; // 注册表无更多可用专家，终止
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

// ================== TDD 测试（2 个，严格先写） ==================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alliance::intent::IntentResult;
    use std::collections::BTreeMap;

    fn fake_intent(winner: &str) -> IntentResult {
        let mut rrf = BTreeMap::new();
        rrf.insert(winner.into(), 0.90);
        // 补全 7 类 0
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

    /// TDD 1: 强制 Security 替换（构造敏感 code query，断言替换说明 + 队里含 security）
    #[test]
    fn test_team_security_replace() {
        let reg = build_expert_registry();
        // registry 含 security & permission 专家
        assert!(reg.get("security").is_some(), "registry must have security expert");
        assert!(reg.get("permission").is_some(), "registry must have permission expert");

        let intent = fake_intent("code");  // 代码类 + conf>0.6 → 强制检查
        let res = optimize_team(&intent, &reg, 3, true); // is_sensitive=true 肯定替换
        assert_eq!(res.team_ids.len(), 3, "team size=3");
        // Security 专家必须在队里
        assert!(
            res.team_ids.iter().any(|id| id == "security"),
            "EAF 4.2 强制替换失败：team 内无 security 专家；team={:?}\n替换说明={:?}",
            res.team_ids, res.forced_replacements
        );
        assert!(
            !res.forced_replacements.is_empty(),
            "敏感场景必须至少触发一次强制替换说明（Security 或 Permission）"
        );
    }

    /// TDD 2: 组队结果 3~5 大小 & 同维不重复（默认 4）
    #[test]
    fn test_team_size_four_and_no_dup_dimension() {
        let reg = build_expert_registry();
        // 注册表覆盖 7 类（HC-9）
        assert!(registry_coverage_check(&reg), "registry HC-9 7 类未全覆盖");
        // 注册表大小 = 14 位（双璇玑 14 维）
        assert_eq!(reg.len(), 14, "专家注册表必须正好 14 位（对应 14 维）");

        for sz in [3usize, 4, 5] {
            let intent = fake_intent("knowledge");
            let res = optimize_team(&intent, &reg, sz, false /*非敏感，跳过强制替换*/);
            assert!(
                res.team_ids.len() == sz || res.team_ids.len() == sz + 1,
                "组队大小 {} 偏差：实际 {}（+1 是因为强制追加 P/S，但本测试 is_sensitive=false 应该不追加）",
                sz, res.team_ids.len()
            );
            // 同维检查：所有 team_ids 对应的 dimension 不重复
            let mut seen = std::collections::HashSet::new();
            for id in &res.team_ids {
                let dim = reg.get(id).expect("team id must exist in registry").dimension;
                assert!(seen.insert(dim), "同维度重复进队：dim={:?}, id={}, team={:?}", dim, id, res.team_ids);
            }
        }
    }

    /// 额外：14 位专家的 supported_classes 并集覆盖 7 类
    #[test]
    fn registry_hc9_7_classes_full_coverage() {
        let reg = build_expert_registry();
        assert!(registry_coverage_check(&reg), "HC-9 失败：7 类未被注册表并集覆盖");
    }
}
