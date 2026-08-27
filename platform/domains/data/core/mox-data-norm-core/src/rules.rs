// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 规则引擎：属性条件 + 动作（设置 / 映射 / 重命名）。
//!
//! 条件：`field OP value`，OP ∈ {==, !=, >, >=, <, <=, contains, regex, in}。
//! 动作：`SET field = value` / `RENAME from to` / `MAP field using table`。

use super::NormRecord;
use ahash::RandomState;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub conditions: Vec<Condition>,
    pub actions: Vec<Action>,
    #[serde(default)]
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub field: String,
    pub op: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "args")]
pub enum Action {
    Set { field: String, value: serde_json::Value },
    Rename { from: String, to: String },
    Map { field: String, table: HashMap<String, serde_json::Value, RandomState> },
    Delete { field: String },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleOutcome {
    pub rules_applied: Vec<String>,
    pub modified: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RuleEngine {
    rules: Vec<Rule>,
}

/// 便捷入口：对一整批 records 依次执行所有规则（返回修改后的副本）。
/// 绑定层直接调用，省去外部循环。
pub fn resolve_rules(records: &[NormRecord], engine: &RuleEngine) -> Vec<NormRecord> {
    records
        .iter()
        .map(|r| {
            let mut out = r.clone();
            let _ = engine.apply(&mut out);
            out
        })
        .collect()
}

impl RuleEngine {
    pub fn new(mut rules: Vec<Rule>) -> Self {
        rules.sort_by(|a, b| a.priority.cmp(&b.priority).then(a.id.cmp(&b.id)));
        Self { rules }
    }

    pub fn apply(&self, rec: &mut NormRecord) -> RuleOutcome {
        let mut applied = Vec::new();
        let mut modified = false;
        for rule in &self.rules {
            if rule.conditions.iter().all(|c| eval(rec, c)) {
                for act in &rule.actions {
                    if execute(rec, act) { modified = true; }
                }
                applied.push(rule.id.clone());
            }
        }
        RuleOutcome { rules_applied: applied, modified }
    }
}

fn eval(rec: &NormRecord, c: &Condition) -> bool {
    let actual = match c.field.as_str() {
        "$id" => Some(serde_json::Value::String(rec.id.clone())),
        "$source" => Some(serde_json::Value::String(rec.source.clone())),
        other => rec.attributes.get(other).cloned(),
    };
    let Some(a) = actual else { return false };
    match c.op.as_str() {
        "==" => a == c.value,
        "!=" => a != c.value,
        ">" => num_cmp(&a, &c.value, |a, b| a > b),
        ">=" => num_cmp(&a, &c.value, |a, b| a >= b),
        "<" => num_cmp(&a, &c.value, |a, b| a < b),
        "<=" => num_cmp(&a, &c.value, |a, b| a <= b),
        "contains" => str_op(&a, &c.value, |hay, needle| hay.contains(needle)),
        "in" => {
            if let serde_json::Value::Array(arr) = &c.value {
                arr.iter().any(|v| v == &a)
            } else { false }
        }
        "regex" => {
            // 轻量 contains 替代：避免依赖 regex crate
            str_op(&a, &c.value, |hay, needle| hay.contains(needle))
        }
        _ => false,
    }
}

fn num_cmp<F: Fn(f64, f64) -> bool>(a: &serde_json::Value, b: &serde_json::Value, f: F) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => f(x, y),
        _ => false,
    }
}

fn str_op<F: Fn(&str, &str) -> bool>(a: &serde_json::Value, b: &serde_json::Value, f: F) -> bool {
    match (a.as_str(), b.as_str()) {
        (Some(x), Some(y)) => f(x, y),
        _ => false,
    }
}

fn execute(rec: &mut NormRecord, act: &Action) -> bool {
    match act {
        Action::Set { field, value } => {
            let existed = rec.attributes.insert(field.clone(), value.clone());
            existed.as_ref() != Some(value)
        }
        Action::Rename { from, to } => {
            if let Some(v) = rec.attributes.remove(from) {
                rec.attributes.insert(to.clone(), v);
                true
            } else { false }
        }
        Action::Map { field, table } => {
            if let Some(v) = rec.attributes.get(field).cloned() {
                if let Some(new) = table.get(&json_key(&v)) {
                    let before = rec.attributes.insert(field.clone(), new.clone());
                    return before.as_ref() != Some(new);
                }
            }
            false
        }
        Action::Delete { field } => rec.attributes.remove(field).is_some(),
    }
}

fn json_key(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_rec(id: &str, kv: &[(&str, &str)]) -> NormRecord {
        let mut attrs: HashMap<String, serde_json::Value, RandomState> =
            HashMap::with_hasher(RandomState::new());
        for (k, v) in kv { attrs.insert((*k).into(), serde_json::Value::String((*v).into())); }
        NormRecord {
            id: id.into(),
            attributes: attrs,
            source: "test".into(),
            updated_at_ms: 1,
            confidence: 0.5,
        }
    }

    #[test]
    fn set_if_equals() {
        let rule = Rule {
            id: "r1".into(),
            conditions: vec![Condition { field: "x".into(), op: "==".into(), value: "a".into() }],
            actions: vec![Action::Set { field: "y".into(), value: "b".into() }],
            priority: 0,
        };
        let eng = RuleEngine::new(vec![rule]);
        let mut r = mk_rec("1", &[("x", "a")]);
        let out = eng.apply(&mut r);
        assert!(out.modified);
        assert_eq!(out.rules_applied.as_slice(), &["r1".to_string()]);
        assert_eq!(r.attributes["y"], "b");
    }
}
