//! 冲突融合 / 增量合并：按策略合并来自多 source 的同 id 记录。
//!
//! MergeStrategy:
//! - HighestConfidenceFirst: 高置信度覆盖；同置信取新鲜度。
//! - UnionAttributes: 取属性并集；冲突时按源优先级字典决断（确定性）。
//! - SourceAuthority(src_order): 按 src_order 优先保留权威来源字段。

use super::NormRecord;
use ahash::RandomState;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

/// 自定义冲突合并函数签名：key → 冲突值列表 → strategy tag → 胜出值
pub type ConflictMergeFn = fn(&str, &[serde_json::Value], &str) -> serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "strategy")]
pub enum MergeStrategy {
    HighestConfidenceFirst,
    UnionAttributes,
    SourceAuthority { src_order: Vec<String> },
    /// FFI 兼容：最后写入胜出（新鲜度主，source 字典序副决断 ties）
    LastWriteWins,
    /// FFI 兼容：众数表决（每字段取出现最多的值；ties 取首次出现）
    Majority,
    /// FFI 兼容：并集兜底（与 UnionAttributes 语义等价；别名便于前端调用）
    UnionFields,
}

impl Default for MergeStrategy {
    fn default() -> Self { MergeStrategy::HighestConfidenceFirst }
}

/// FFI 便捷入口：允许传入闭包风格的 ConflictMergeFn 做逐字段合并。
/// records 按 id 分桶；同 id 的记录合并为一条。
pub fn merge_conflicts(records: &[NormRecord], merge_fn: &mut ConflictMergeFn) -> Vec<NormRecord> {
    let mut groups: HashMap<String, Vec<usize>, RandomState> =
        HashMap::with_hasher(RandomState::new());
    for (i, r) in records.iter().enumerate() {
        groups.entry(r.id.clone()).or_default().push(i);
    }
    let mut out = Vec::with_capacity(groups.len());
    for (_id, idxs) in groups {
        if idxs.len() == 1 {
            out.push(records[idxs[0]].clone());
            continue;
        }
        // 取更新最"新"的作为基底（保留元数据）
        let mut base_idx = 0usize;
        let mut base_ts = i64::MIN;
        for &i in &idxs {
            if records[i].updated_at_ms > base_ts {
                base_ts = records[i].updated_at_ms;
                base_idx = i;
            }
        }
        let mut merged = records[base_idx].clone();
        // 逐字段冲突决断
        let mut all_keys: hashbrown::HashSet<String, RandomState> =
            hashbrown::HashSet::with_hasher(RandomState::new());
        for &i in &idxs {
            for k in records[i].attributes.keys() {
                all_keys.insert(k.clone());
            }
        }
        for k in all_keys {
            let vals: Vec<serde_json::Value> = idxs
                .iter()
                .filter_map(|&i| records[i].attributes.get(&k).cloned())
                .collect();
            if !vals.is_empty() {
                let strategy_tag = match vals.len() {
                    1 => "single",
                    _ => "conflict",
                };
                let winner = merge_fn(&k, &vals, strategy_tag);
                merged.attributes.insert(k, winner);
            }
        }
        // 对 confidence 取 max；元数据取最新
        merged.confidence = idxs.iter().map(|&i| records[i].confidence).fold(0.0f32, f32::max);
        out.push(merged);
    }
    out
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MergeResult {
    pub merged: Vec<NormRecord>,
    pub merged_groups: usize,
    pub conflicts_resolved: usize,
}

pub fn merge_records(records: &[NormRecord], strategy: &MergeStrategy) -> MergeResult {
    // 按 id 分组
    let mut groups: HashMap<String, Vec<usize>, RandomState> =
        HashMap::with_hasher(RandomState::new());
    for (i, r) in records.iter().enumerate() {
        groups.entry(r.id.clone()).or_default().push(i);
    }

    let mut result = Vec::with_capacity(groups.len());
    let mut groups_count = 0usize;
    let mut conflicts = 0usize;

    for (_id, idxs) in groups {
        if idxs.len() == 1 {
            result.push(records[idxs[0]].clone());
            continue;
        }
        groups_count += 1;
        // 探测冲突
        let has_conflict = has_attr_conflict(idxs.iter().map(|&i| &records[i]));
        if has_conflict { conflicts += 1; }

        let group: Vec<&NormRecord> = idxs.iter().map(|&i| &records[i]).collect();
        let m = match strategy {
            MergeStrategy::HighestConfidenceFirst => merge_highest_conf(&group),
            MergeStrategy::UnionAttributes | MergeStrategy::UnionFields => merge_union(&group),
            MergeStrategy::SourceAuthority { src_order } => merge_authority(&group, src_order),
            MergeStrategy::LastWriteWins => merge_last_write_wins(&group),
            MergeStrategy::Majority => merge_majority(&group),
        };
        result.push(m);
    }

    MergeResult { merged: result, merged_groups: groups_count, conflicts_resolved: conflicts }
}

fn has_attr_conflict<'a, I: Iterator<Item = &'a NormRecord>>(mut it: I) -> bool {
    let first = match it.next() { Some(r) => r, None => return false };
    for other in it {
        for (k, v1) in &first.attributes {
            if let Some(v2) = other.attributes.get(k) {
                if v1 != v2 { return true; }
            }
        }
    }
    false
}

fn merge_highest_conf(group: &[&NormRecord]) -> NormRecord {
    // 选综合分最高作为基底（confidence 主，freshness 次）。
    let mut sorted: Vec<&NormRecord> = group.to_vec();
    sorted.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap()
        .then(b.updated_at_ms.cmp(&a.updated_at_ms))
        .then(a.source.cmp(&b.source)));
    let base = sorted[0];
    let mut out = base.clone();
    // 其余记录只在缺失字段补充
    for rec in sorted.iter().skip(1) {
        for (k, v) in &rec.attributes {
            out.attributes.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    out
}

fn merge_union(group: &[&NormRecord]) -> NormRecord {
    // 取 fresh 最新的为元数据基础
    let mut fresh: Vec<&NormRecord> = group.to_vec();
    fresh.sort_by(|a, b| b.updated_at_ms.cmp(&a.updated_at_ms)
        .then(b.confidence.partial_cmp(&a.confidence).unwrap()));
    let base = fresh[0];
    let mut out = base.clone();
    for rec in fresh.iter().skip(1) {
        for (k, v) in &rec.attributes {
            out.attributes.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    out
}

fn merge_authority(group: &[&NormRecord], order: &[String]) -> NormRecord {
    // 按 source 权威度排序（order 靠前优先）
    let mut with_rank: Vec<(usize, &NormRecord)> = group
        .iter()
        .map(|r| (order.iter().position(|s| s == &r.source).unwrap_or(usize::MAX), *r))
        .collect();
    with_rank.sort_by(|a, b| a.0.cmp(&b.0)
        .then(b.1.updated_at_ms.cmp(&a.1.updated_at_ms)));
    // 基低用最权威来源，缺失字段用后续补
    let base = with_rank[0].1;
    let mut out = base.clone();
    for (_, rec) in with_rank.iter().skip(1) {
        for (k, v) in &rec.attributes {
            out.attributes.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    out
}

fn merge_last_write_wins(group: &[&NormRecord]) -> NormRecord {
    // 按 updated_at_ms 降序；ties 按 source 字典序确保确定
    let mut sorted: Vec<&NormRecord> = group.to_vec();
    sorted.sort_by(|a, b| b.updated_at_ms.cmp(&a.updated_at_ms).then(a.source.cmp(&b.source)));
    let base = sorted[0];
    let mut out = base.clone();
    // 其余记录只补充缺失字段（LWW：每个字段取最新写入者；由于 base 已是最新，非缺失字段就是最新）
    for rec in sorted.iter().skip(1) {
        for (k, v) in &rec.attributes {
            out.attributes.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    out
}

fn merge_majority(group: &[&NormRecord]) -> NormRecord {
    // 每字段众数表决；ties 取 group 中首次出现的值。
    // 以新鲜度最高的记录为元数据基底。
    let mut fresh: Vec<&NormRecord> = group.to_vec();
    fresh.sort_by(|a, b| b.updated_at_ms.cmp(&a.updated_at_ms));
    let base = fresh[0];
    let mut out = base.clone();
    let mut keys: hashbrown::HashSet<String, RandomState> =
        hashbrown::HashSet::with_hasher(RandomState::new());
    for r in group {
        for k in r.attributes.keys() {
            keys.insert(k.clone());
        }
    }
    for k in keys {
        // 统计频次 + 首次出现索引（ties 决断用）
        let mut counts: HashMap<String, (usize, usize), RandomState> =
            HashMap::with_hasher(RandomState::new());
        let mut order: usize = 0;
        for (i, r) in group.iter().enumerate() {
            if let Some(v) = r.attributes.get(&k) {
                let key = v.to_string();
                let entry = counts.entry(key).or_insert((0, usize::MAX));
                entry.0 += 1;
                if order < entry.1 {
                    entry.1 = i;  // 用 group 内位置代替 order，更确定
                }
                order += 1;
            }
        }
        if counts.is_empty() {
            continue;
        }
        // 选 (count desc, first_seen asc) 最佳
        let best = counts
            .into_iter()
            .max_by(|a, b| a.1 .0.cmp(&b.1 .0).then(b.1 .1.cmp(&a.1 .1)))
            .map(|(k, _)| k);
        // 从首次出现的记录中取原始 Value（避免 to_string 反序列化丢失精度）
        if let Some(target_str) = best {
            for r in group {
                if let Some(v) = r.attributes.get(&k) {
                    if v.to_string() == target_str {
                        out.attributes.insert(k.clone(), v.clone());
                        break;
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ahash::RandomState;

    fn rec(id: &str, src: &str, conf: f32, ts: i64, attrs: &[(&str, &str)]) -> NormRecord {
        let mut a: HashMap<_, _, _> = HashMap::with_hasher(RandomState::new());
        for (k, v) in attrs { a.insert((*k).into(), serde_json::Value::String((*v).into())); }
        NormRecord { id: id.into(), attributes: a, source: src.into(), updated_at_ms: ts, confidence: conf }
    }

    #[test]
    fn highest_confidence_wins() {
        let a = rec("id1", "s1", 0.5, 10, &[("k", "A")]);
        let b = rec("id1", "s2", 0.9, 5, &[("k", "B")]);
        let r = merge_records(&[a, b], &MergeStrategy::HighestConfidenceFirst);
        assert_eq!(r.merged_groups, 1);
        assert_eq!(r.conflicts_resolved, 1);
        assert_eq!(r.merged[0].attributes["k"], "B");
        assert_eq!(r.merged[0].confidence, 0.9);
    }

    #[test]
    fn union_unifies_fields() {
        let a = rec("id1", "s1", 0.5, 10, &[("x", "1")]);
        let b = rec("id1", "s2", 0.5, 5, &[("y", "2")]);
        let r = merge_records(&[a, b], &MergeStrategy::UnionAttributes);
        assert_eq!(r.merged[0].attributes.len(), 2);
        assert!(r.merged[0].attributes.contains_key("x"));
        assert!(r.merged[0].attributes.contains_key("y"));
    }

    #[test]
    fn authority_prefers_ordered_source() {
        let a = rec("id", "ingest", 0.9, 100, &[("k", "ingest_val")]); // 冲突：ingest_val
        let b = rec("id", "registry", 0.5, 1, &[("k", "registry_val")]); // 更权威
        let strat = MergeStrategy::SourceAuthority { src_order: vec!["registry".into(), "ingest".into()] };
        let r = merge_records(&[a, b], &strat);
        assert_eq!(r.merged[0].attributes["k"], "registry_val");
        assert_eq!(r.merged[0].source, "registry");
    }
}
