// Copyright (c) 2026 璇玑 RelGraph · 流程算法归一化核心 (Unified Process & Algorithm Core)
// Licensed under the MIT License.

//! 规则引擎（专家系统核心）
//!
//! 基于前向链推理（Forward Chaining）的产生式规则引擎。
//! 支持 Rete 算法的简化版本，用于高效匹配规则与事实。

use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{ProcessError, ProcessResult};
use crate::types::*;

/// 规则引擎
pub struct RuleEngine {
    /// 规则表：rule_set -> Vec<Rule>
    rule_sets: RwLock<HashMap<String, Vec<Rule>>>,
    /// 规则 ID 索引
    rule_index: RwLock<HashMap<String, String>>, // rule_id -> rule_set
    /// 总执行次数
    total_fires: AtomicU64,
}

impl RuleEngine {
    /// 创建规则引擎
    pub fn new() -> Self {
        Self {
            rule_sets: RwLock::new(HashMap::new()),
            rule_index: RwLock::new(HashMap::new()),
            total_fires: AtomicU64::new(0),
        }
    }

    /// 注册规则
    pub fn register_rule(&self, rule: Rule) -> ProcessResult<Rule> {
        if self.rule_index.read().contains_key(&rule.id) {
            return Err(ProcessError::AlreadyExists(format!(
                "rule id '{}' already exists",
                rule.id
            )));
        }

        self.rule_index
            .write()
            .insert(rule.id.clone(), rule.rule_set.clone());
        self.rule_sets
            .write()
            .entry(rule.rule_set.clone())
            .or_default()
            .push(rule.clone());

        Ok(rule)
    }

    /// 获取规则
    pub fn get_rule(&self, rule_id: &str) -> Option<Rule> {
        let rule_set = self.rule_index.read().get(rule_id)?.clone();
        let rules = self.rule_sets.read().get(&rule_set)?.clone();
        rules.into_iter().find(|r| r.id == rule_id)
    }

    /// 获取规则集的所有规则（按优先级排序）
    pub fn get_rules_by_set(&self, rule_set: &str) -> Vec<Rule> {
        let mut rules = self
            .rule_sets
            .read()
            .get(rule_set)
            .cloned()
            .unwrap_or_default();
        // 按优先级降序排列
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        rules
    }

    /// 执行规则集
    pub fn execute(&self, rule_set: &str, context: &mut ProcessContext) -> ProcessResult<Vec<String>> {
        let rules = self.get_rules_by_set(rule_set);
        if rules.is_empty() {
            context.log(LogLevel::Warn, &format!("rule set '{}' is empty", rule_set));
            return Ok(Vec::new());
        }

        let mut fired_rules = Vec::new();
        let mut fire_counts: HashMap<String, u32> = HashMap::new();
        let mut iterations = 0;
        const MAX_ITERATIONS: u32 = 1000; // 防止无限循环

        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                return Err(ProcessError::ExecutionError(
                    "rule execution exceeded max iterations (possible infinite loop)".to_string(),
                ));
            }

            let mut fired_any = false;

            for rule in &rules {
                if !rule.enabled {
                    continue;
                }

                // 检查最大触发次数
                if rule.max_fires > 0 {
                    let count = fire_counts.get(&rule.id).copied().unwrap_or(0);
                    if count >= rule.max_fires {
                        continue;
                    }
                }

                // 检查条件
                if self.evaluate_conditions(rule, context)? {
                    // 执行动作
                    self.execute_actions(rule, context)?;
                    fired_rules.push(rule.id.clone());
                    *fire_counts.entry(rule.id.clone()).or_insert(0) += 1;
                    self.total_fires.fetch_add(1, Ordering::Relaxed);
                    fired_any = true;
                    context.log(
                        LogLevel::Info,
                        &format!("rule '{}' fired", rule.name),
                    );

                    // 执行一条后重新评估（前向链）
                    break;
                }
            }

            if !fired_any {
                break;
            }
        }

        Ok(fired_rules)
    }

    /// 评估规则条件
    fn evaluate_conditions(&self, rule: &Rule, context: &ProcessContext) -> ProcessResult<bool> {
        if rule.conditions.is_empty() {
            return Ok(true);
        }

        match rule.condition_logic {
            ConditionLogic::And => {
                for cond in &rule.conditions {
                    if !self.evaluate_condition(cond, context)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ConditionLogic::Or => {
                for cond in &rule.conditions {
                    if self.evaluate_condition(cond, context)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }

    /// 评估单个条件
    fn evaluate_condition(
        &self,
        cond: &RuleCondition,
        context: &ProcessContext,
    ) -> ProcessResult<bool> {
        let fact_value = context
            .get_fact(&cond.fact)
            .map(|f| f.value.clone())
            .unwrap_or(Value::Null);

        let result = match cond.operator.as_str() {
            "==" => compare_eq(&fact_value, &cond.value),
            "!=" => !compare_eq(&fact_value, &cond.value),
            ">" => compare_gt(&fact_value, &cond.value),
            ">=" => compare_ge(&fact_value, &cond.value),
            "<" => compare_lt(&fact_value, &cond.value),
            "<=" => compare_le(&fact_value, &cond.value),
            "contains" => contains(&fact_value, &cond.value),
            "in" => contains(&cond.value, &fact_value), // 反过来：fact 在 value 列表中
            "startsWith" => starts_with(&fact_value, &cond.value),
            "endsWith" => ends_with(&fact_value, &cond.value),
            "isNull" => fact_value.is_null(),
            "notNull" => !fact_value.is_null(),
            "isEmpty" => is_empty(&fact_value),
            "notEmpty" => !is_empty(&fact_value),
            op => {
                return Err(ProcessError::RuleError(format!(
                    "unsupported operator: {}",
                    op
                )));
            }
        };

        Ok(result)
    }

    /// 执行规则动作
    fn execute_actions(&self, rule: &Rule, context: &mut ProcessContext) -> ProcessResult<()> {
        for action in &rule.actions {
            match action.action_type {
                RuleActionType::Set => {
                    let value = action.value.clone().unwrap_or(Value::Null);
                    context.add_fact(Fact::new(&action.target, value));
                }
                RuleActionType::Add => {
                    // 数组追加
                    if let Some(existing) = context.get_fact(&action.target) {
                        if let Some(arr) = existing.value.as_array() {
                            let mut new_arr = arr.clone();
                            if let Some(v) = &action.value {
                                new_arr.push(v.clone());
                            }
                            context.add_fact(Fact::new(&action.target, Value::Array(new_arr)));
                        }
                    } else if let Some(v) = &action.value {
                        context.add_fact(Fact::new(&action.target, Value::Array(vec![v.clone()])));
                    }
                }
                RuleActionType::Remove => {
                    context.facts.remove(&action.target);
                }
                RuleActionType::Trigger => {
                    context.log(LogLevel::Info, &format!("trigger: {}", action.target));
                }
                RuleActionType::RaiseError => {
                    let msg = action
                        .value
                        .as_ref()
                        .and_then(|v| v.as_str())
                        .unwrap_or("rule raised error");
                    return Err(ProcessError::RuleError(msg.to_string()));
                }
                RuleActionType::Log => {
                    let msg = action
                        .value
                        .as_ref()
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    context.log(LogLevel::Info, msg);
                }
            }
        }
        Ok(())
    }

    /// 获取规则集数量
    pub fn rule_set_count(&self) -> usize {
        self.rule_sets.read().len()
    }

    /// 获取规则总数
    pub fn rule_count(&self) -> usize {
        self.rule_index.read().len()
    }

    /// 获取总触发次数
    pub fn total_fires(&self) -> u64 {
        self.total_fires.load(Ordering::Relaxed)
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ===== 比较辅助函数 =====

fn compare_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            if let (Some(xi), Some(yi)) = (x.as_i64(), y.as_i64()) {
                xi == yi
            } else {
                x.as_f64().unwrap_or(0.0) == y.as_f64().unwrap_or(0.0)
            }
        }
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        (Value::Array(x), Value::Array(y)) => x == y,
        (Value::Object(x), Value::Object(y)) => x == y,
        _ => false,
    }
}

fn compare_gt(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            if let (Some(xi), Some(yi)) = (x.as_i64(), y.as_i64()) {
                xi > yi
            } else {
                x.as_f64().unwrap_or(0.0) > y.as_f64().unwrap_or(0.0)
            }
        }
        (Value::String(x), Value::String(y)) => x > y,
        _ => false,
    }
}

fn compare_ge(a: &Value, b: &Value) -> bool {
    compare_gt(a, b) || compare_eq(a, b)
}

fn compare_lt(a: &Value, b: &Value) -> bool {
    compare_gt(b, a)
}

fn compare_le(a: &Value, b: &Value) -> bool {
    compare_lt(a, b) || compare_eq(a, b)
}

fn contains(container: &Value, item: &Value) -> bool {
    match (container, item) {
        (Value::String(s), Value::String(pattern)) => s.contains(pattern.as_str()),
        (Value::Array(arr), _) => arr.iter().any(|v| compare_eq(v, item)),
        _ => false,
    }
}

fn starts_with(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(s), Value::String(prefix)) => s.starts_with(prefix.as_str()),
        _ => false,
    }
}

fn ends_with(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(s), Value::String(suffix)) => s.ends_with(suffix.as_str()),
        _ => false,
    }
}

fn is_empty(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        Value::Array(arr) => arr.is_empty(),
        Value::Object(obj) => obj.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_register_and_get_rule() {
        let engine = RuleEngine::new();
        let rule = Rule::new("test_rule", "test");
        let registered = engine.register_rule(rule.clone()).unwrap();

        assert_eq!(engine.rule_count(), 1);
        assert_eq!(engine.rule_set_count(), 1);

        let retrieved = engine.get_rule(&registered.id).unwrap();
        assert_eq!(retrieved.name, "test_rule");
    }

    #[test]
    fn test_simple_rule_fire() {
        let engine = RuleEngine::new();

        let rule = Rule::new("high_score", "scoring")
            .with_condition("score", ">", json!(90))
            .with_set_action("grade", json!("A"))
            .with_set_action("passed", json!(true));

        engine.register_rule(rule).unwrap();

        let mut context = ProcessContext::new();
        context.add_fact(Fact::new("score", json!(95)));

        let fired = engine.execute("scoring", &mut context).unwrap();
        assert_eq!(fired.len(), 1);

        let grade = context.get_fact("grade").unwrap();
        assert_eq!(grade.value, json!("A"));

        let passed = context.get_fact("passed").unwrap();
        assert_eq!(passed.value, json!(true));
    }

    #[test]
    fn test_chained_rules() {
        let engine = RuleEngine::new();

        // 规则1：高分 -> 优等生
        let rule1 = Rule::new("high_score", "edu")
            .with_condition("score", ">=", json!(90))
            .with_set_action("level", json!("excellent"));

        // 规则2：优等生 -> 发奖学金
        let rule2 = Rule::new("scholarship", "edu")
            .with_condition("level", "==", json!("excellent"))
            .with_set_action("scholarship", json!(5000));

        engine.register_rule(rule1).unwrap();
        engine.register_rule(rule2).unwrap();

        let mut context = ProcessContext::new();
        context.add_fact(Fact::new("score", json!(92)));

        let fired = engine.execute("edu", &mut context).unwrap();
        assert_eq!(fired.len(), 2); // 两条规则都触发

        let scholarship = context.get_fact("scholarship").unwrap();
        assert_eq!(scholarship.value, json!(5000));
    }

    #[test]
    fn test_rule_not_fired() {
        let engine = RuleEngine::new();

        let rule = Rule::new("check", "test")
            .with_condition("value", ">", json!(100))
            .with_set_action("flag", json!(true));

        engine.register_rule(rule).unwrap();

        let mut context = ProcessContext::new();
        context.add_fact(Fact::new("value", json!(50)));

        let fired = engine.execute("test", &mut context).unwrap();
        assert!(fired.is_empty());
        assert!(context.get_fact("flag").is_none());
    }

    #[test]
    fn test_or_condition() {
        let engine = RuleEngine::new();

        let mut rule = Rule::new("or_test", "test")
            .with_condition("a", "==", json!(1))
            .with_set_action("result", json!("matched"));
        rule.condition_logic = ConditionLogic::Or;
        rule.conditions.push(RuleCondition {
            fact: "b".to_string(),
            operator: "==".to_string(),
            value: json!(2),
        });

        engine.register_rule(rule).unwrap();

        let mut context = ProcessContext::new();
        context.add_fact(Fact::new("b", json!(2)));

        let fired = engine.execute("test", &mut context).unwrap();
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn test_max_fires() {
        let engine = RuleEngine::new();

        let mut rule = Rule::new("limited", "test")
            .with_condition("counter", "<", json!(5))
            .with_set_action("counter", json!(5)); // 设为5后不再满足
        rule.max_fires = 2;

        // 替换动作：让 counter 自增
        rule.actions = vec![RuleAction {
            action_type: RuleActionType::Set,
            target: "counter".to_string(),
            value: Some(json!(10)),
            expression: None,
        }];

        engine.register_rule(rule).unwrap();

        let mut context = ProcessContext::new();
        context.add_fact(Fact::new("counter", json!(0)));

        let fired = engine.execute("test", &mut context).unwrap();
        // 只会触发1次（因为 counter 设为 10 后不再 < 5）
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn test_contains_operator() {
        let engine = RuleEngine::new();

        let rule = Rule::new("email_check", "test")
            .with_condition("email", "contains", json!("@"))
            .with_set_action("valid", json!(true));

        engine.register_rule(rule).unwrap();

        let mut context = ProcessContext::new();
        context.add_fact(Fact::new("email", json!("user@example.com")));

        let fired = engine.execute("test", &mut context).unwrap();
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn test_duplicate_rule() {
        let engine = RuleEngine::new();
        let rule = Rule::new("r1", "test");
        engine.register_rule(rule.clone()).unwrap();
        assert!(engine.register_rule(rule).is_err());
    }
}
