// Copyright (c) 2026 璇玑 RelGraph · 流程算法归一化核心 (Unified Process & Algorithm Core)
// Licensed under the MIT License.

//! 核心类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 流程状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessStatus {
    /// 待启动
    Pending,
    /// 运行中
    Running,
    /// 等待（人工审批/外部事件）
    Waiting,
    /// 已暂停
    Suspended,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
    /// 已取消
    Cancelled,
    /// 超时
    Timeout,
}

/// 步骤类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    /// 开始
    Start,
    /// 结束
    End,
    /// 算法步骤（调用算法联盟）
    Algorithm,
    /// 规则步骤（执行专家规则）
    Rule,
    /// 决策步骤（条件分支）
    Decision,
    /// 并行步骤
    Parallel,
    /// 子流程
    SubProcess,
    /// 人工审批
    Approval,
    /// 脚本步骤
    Script,
    /// 通知步骤
    Notification,
    /// 数据转换
    Transform,
}

/// 步骤状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 已跳过
    Skipped,
    /// 失败
    Failed,
    /// 等待中
    Waiting,
}

/// 事实（用于规则引擎）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub name: String,
    pub value: serde_json::Value,
    pub source: Option<String>,
    pub timestamp: u64,
}

impl Fact {
    pub fn new(name: &str, value: serde_json::Value) -> Self {
        Self {
            name: name.to_string(),
            value,
            source: None,
            timestamp: now_ms(),
        }
    }
}

/// 规则条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    /// 事实名
    pub fact: String,
    /// 操作符
    pub operator: String, // ==, !=, >, <, >=, <=, contains, in, regex
    /// 比较值
    pub value: serde_json::Value,
}

/// 规则动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleAction {
    /// 动作类型
    pub action_type: RuleActionType,
    /// 目标事实名
    pub target: String,
    /// 值（用于 set）
    pub value: Option<serde_json::Value>,
    /// 表达式
    pub expression: Option<String>,
}

/// 规则动作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleActionType {
    /// 设置事实
    Set,
    /// 新增事实
    Add,
    /// 删除事实
    Remove,
    /// 触发事件
    Trigger,
    /// 抛出错误
    RaiseError,
    /// 记录日志
    Log,
}

/// 规则定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// 规则 ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 规则描述
    pub description: Option<String>,
    /// 优先级（数值越大越优先）
    pub priority: u32,
    /// 条件列表（全部满足才触发）
    pub conditions: Vec<RuleCondition>,
    /// 条件逻辑：and / or
    pub condition_logic: ConditionLogic,
    /// 动作列表
    pub actions: Vec<RuleAction>,
    /// 是否启用
    pub enabled: bool,
    /// 所属规则集
    pub rule_set: String,
    /// 最大触发次数（0 = 无限，默认1）
    pub max_fires: u32,
}

/// 条件逻辑
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionLogic {
    /// 全部满足
    And,
    /// 任一满足
    Or,
}

impl Rule {
    /// 创建规则
    pub fn new(name: &str, rule_set: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: None,
            priority: 100,
            conditions: Vec::new(),
            condition_logic: ConditionLogic::And,
            actions: Vec::new(),
            enabled: true,
            rule_set: rule_set.to_string(),
            max_fires: 1,
        }
    }

    /// 添加条件
    pub fn with_condition(mut self, fact: &str, operator: &str, value: serde_json::Value) -> Self {
        self.conditions.push(RuleCondition {
            fact: fact.to_string(),
            operator: operator.to_string(),
            value,
        });
        self
    }

    /// 添加设值动作
    pub fn with_set_action(mut self, target: &str, value: serde_json::Value) -> Self {
        self.actions.push(RuleAction {
            action_type: RuleActionType::Set,
            target: target.to_string(),
            value: Some(value),
            expression: None,
        });
        self
    }
}

/// 流程变量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessVariable {
    pub name: String,
    pub var_type: String,
    pub value: serde_json::Value,
    pub description: Option<String>,
}

/// 流程执行上下文
#[derive(Debug, Clone, Default)]
pub struct ProcessContext {
    /// 变量
    pub variables: HashMap<String, serde_json::Value>,
    /// 事实（用于规则引擎）
    pub facts: HashMap<String, Fact>,
    /// 执行日志
    pub logs: Vec<ProcessLogEntry>,
    /// 当前步骤 ID
    pub current_step_id: Option<String>,
    /// 启动时间
    pub started_at: Option<u64>,
    /// 结束时间
    pub ended_at: Option<u64>,
}

/// 流程日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessLogEntry {
    pub timestamp: u64,
    pub step_id: Option<String>,
    pub level: LogLevel,
    pub message: String,
}

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

impl ProcessContext {
    /// 创建新上下文
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            facts: HashMap::new(),
            logs: Vec::new(),
            current_step_id: None,
            started_at: Some(now_ms()),
            ended_at: None,
        }
    }

    /// 设置变量
    pub fn set_variable(&mut self, name: &str, value: serde_json::Value) {
        self.variables.insert(name.to_string(), value);
    }

    /// 获取变量
    pub fn get_variable(&self, name: &str) -> Option<&serde_json::Value> {
        self.variables.get(name)
    }

    /// 添加事实
    pub fn add_fact(&mut self, fact: Fact) {
        self.facts.insert(fact.name.clone(), fact);
    }

    /// 获取事实
    pub fn get_fact(&self, name: &str) -> Option<&Fact> {
        self.facts.get(name)
    }

    /// 记录日志
    pub fn log(&mut self, level: LogLevel, message: &str) {
        self.logs.push(ProcessLogEntry {
            timestamp: now_ms(),
            step_id: self.current_step_id.clone(),
            level,
            message: message.to_string(),
        });
    }
}

/// 获取当前时间戳（毫秒）
pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_fact_creation() {
        let fact = Fact::new("temperature", json!(25));
        assert_eq!(fact.name, "temperature");
        assert_eq!(fact.value, json!(25));
    }

    #[test]
    fn test_rule_builder() {
        let rule = Rule::new("high_temp_alert", "climate")
            .with_condition("temperature", ">", json!(30))
            .with_set_action("alert", json!(true));

        assert_eq!(rule.conditions.len(), 1);
        assert_eq!(rule.actions.len(), 1);
        assert_eq!(rule.priority, 100);
        assert!(rule.enabled);
    }

    #[test]
    fn test_process_context() {
        let mut ctx = ProcessContext::new();
        ctx.set_variable("order_id", json!("ORD-123"));
        ctx.add_fact(Fact::new("amount", json!(99.99)));
        ctx.log(LogLevel::Info, "process started");

        assert_eq!(ctx.get_variable("order_id").unwrap(), &json!("ORD-123"));
        assert!(ctx.get_fact("amount").is_some());
        assert_eq!(ctx.logs.len(), 1);
        assert_eq!(ctx.logs[0].level, LogLevel::Info);
    }

    #[test]
    fn test_process_status() {
        assert_eq!(format!("{:?}", ProcessStatus::Running), "Running");
    }
}
