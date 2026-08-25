//! 去重：ahash 指纹 + 二级属性比对。
//!
//! - 指纹 = ahash64(id) XOR ahash64(主要属性 keys 拼接)；
//! - 同指纹记录再进行全属性 JSON 值比对（保留最新/高置信度版本）。

use super::NormRecord;
use ahash::{AHasher, RandomState};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::hash::{BuildHasher, Hasher};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DedupReport {
    pub input: usize,
    pub unique: usize,
    pub duplicates_removed: usize,
    pub kept_by_confidence: usize,
    pub kept_by_freshness: usize,
}

pub fn dedup_records(records: &[NormRecord]) -> (Vec<NormRecord>, DedupReport) {
    let hasher = RandomState::new();
    // 指纹 → Vec<index>
    let mut buckets: HashMap<u64, Vec<usize>, RandomState> =
        HashMap::with_hasher(RandomState::new());
    let fingerprints: Vec<u64> = records.iter().map(|r| fingerprint(r, &hasher)).collect();
    for (i, &fp) in fingerprints.iter().enumerate() {
        buckets.entry(fp).or_default().push(i);
    }

    let mut kept = Vec::with_capacity(records.len());
    let mut kept_by_conf = 0usize;
    let mut kept_by_fresh = 0usize;
    let mut dup = 0usize;

    for (_fp, idxs) in buckets {
        if idxs.len() == 1 {
            kept.push(records[idxs[0]].clone());
            continue;
        }
        // 桶内：两两比对 attributes
        let mut winners: Vec<usize> = Vec::new();
        'outer: for &candidate in &idxs {
            let cr = &records[candidate];
            for &w in &winners {
                let wr = &records[w];
                if is_duplicate(cr, wr) {
                    // 比较可信度 > 新鲜度，胜者保留
                    let (winner, by_conf) = pick_later(cr, wr);
                    if winner == candidate {
                        // 替换 winners 中的 w 为 candidate
                        winners.retain(|&x| x != w);
                        winners.push(candidate);
                    }
                    if by_conf { kept_by_conf += 1; } else { kept_by_fresh += 1; }
                    dup += 1;
                    continue 'outer;
                }
            }
            winners.push(candidate);
        }
        for w in winners { kept.push(records[w].clone()); }
    }

    let report = DedupReport {
        input: records.len(),
        unique: kept.len(),
        duplicates_removed: dup,
        kept_by_confidence: kept_by_conf,
        kept_by_freshness: kept_by_fresh,
    };
    (kept, report)
}

fn fingerprint(r: &NormRecord, h: &RandomState) -> u64 {
    let mut a = AHasher::default();
    // 基础 id 贡献
    a.write(r.id.as_bytes());
    // 再对 attributes keys 排序后拼接，作为结构指纹
    let mut keys: Vec<&String> = r.attributes.keys().collect();
    keys.sort_unstable();
    for k in keys { a.write(k.as_bytes()); a.write_u8(0x01); }
    h.hash_one(a.finish())
}

fn is_duplicate(a: &NormRecord, b: &NormRecord) -> bool {
    if a.id != b.id { return false; }
    if a.attributes.len() != b.attributes.len() { return false; }
    for (k, va) in &a.attributes {
        match b.attributes.get(k) {
            Some(vb) if va == vb => continue,
            _ => return false,
        }
    }
    true
}

/// 返回 winner_idx（candidate 或 existing）。第二字段 true = 因置信度胜出；false = 新鲜度。
fn pick_later(candidate: &NormRecord, existing: &NormRecord) -> (usize, bool) {
    // winner index: 0 = candidate, 1 = existing
    let (which, by_conf) = if (candidate.confidence - existing.confidence).abs() > 1e-6 {
        ((candidate.confidence > existing.confidence) as usize, true)
    } else {
        ((candidate.updated_at_ms >= existing.updated_at_ms) as usize, false)
    };
    (if which == 0 { 0 } else { 1 }, by_conf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: &str, src: &str, conf: f32, ts: i64) -> NormRecord {
        NormRecord {
            id: id.into(),
            attributes: HashMap::with_hasher(RandomState::new()),
            source: src.into(),
            updated_at_ms: ts,
            confidence: conf,
        }
    }

    #[test]
    fn exact_dup_removes_one() {
        let list = vec![rec("a", "s", 0.5, 10), rec("a", "s", 0.5, 10)];
        let (kept, report) = dedup_records(&list);
        assert_eq!(kept.len(), 1);
        assert_eq!(report.duplicates_removed, 1);
        assert_eq!(report.input, 2);
        assert_eq!(report.unique, 1);
    }

    #[test]
    fn pick_higher_confidence() {
        let low = rec("a", "s", 0.5, 10);
        let high = rec("a", "s2", 0.9, 5); // 旧时间但更可信
        let list = vec![low, high];
        let (kept, rep) = dedup_records(&list);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].confidence, 0.9);
        assert!(rep.kept_by_confidence >= 1);
    }
}
