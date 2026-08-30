// Copyright (c) 2026 璇玑 RelGraph · 流程算法归一化核心 (Unified Process & Algorithm Core)
// Licensed under the MIT License.

//! 流程定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::types::StepType;

/// 决策分支
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionBranch {
    /// 分支 ID
    pub id: String,
    /// 分支名称
    pub name: String,
    /// 条件表达式
    pub condition: String,
    /// 目标步骤 ID
    pub target_step_id: String,
    /// 优先级
    pub priority: u32,
}

/// 流程步骤定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStep {
    /// 步骤 ID
    pub id: String,
    /// 步骤名称
    pub name: String,
    /// 步骤类型
    pub step_type: StepType,
    /// 描述
    pub description: Option<String>,
    /// 下一步骤 ID（串行时）
    pub next_step_id: Option<String>,
    /// 决策分支（决策步骤使用）
    pub branches: Vec<DecisionBranch>,
    /// 并行分支（并行步骤使用）
    pub parallel_branches: Vec<String>,
    /// 汇聚步骤 ID（并行汇聚时）
    pub join_step_id: Option<String>,
    /// 子流程 ID（子流程步骤使用）
    pub sub_process_id: Option<String>,
    /// 算法 ID（算法步骤使用）
    pub algorithm_id: Option<String>,
    /// 算法参数
    pub algorithm_params: HashMap<String, serde_json::Value>,
    /// 规则集 ID（规则步骤使用）
    pub rule_set_id: Option<String>,
    /// 决策表 ID（决策步骤使用）
    pub decision_table_id: Option<String>,
    /// 脚本内容（脚本步骤使用）
    pub script: Option<String>,
    /// 审批人（审批步骤使用）
    pub approvers: Vec<String>,
    /// 审批策略
    pub approval_policy: ApprovalPolicy,
    /// 超时时间（秒）
    pub timeout_seconds: Option<u64>,
    /// 输入变量映射：流程变量名 -> 步骤输入名
    pub input_mapping: HashMap<String, String>,
    /// 输出变量映射：步骤输出名 -> 流程变量名
    pub output_mapping: HashMap<String, String>,
    /// 重试次数
    pub retry_count: u32,
    /// 重试间隔（秒）
    pub retry_interval_seconds: u64,
    /// 是否异步执行
    pub async_execution: bool,
}

/// 审批策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicy {
    /// 或签（任一审批人通过即可）
    Any,
    /// 会签（所有审批人都要通过）
    All,
    /// 顺序签（按顺序审批）
    Sequential,
    /// 多数通过
    Majority,
}

impl Default for ApprovalPolicy {
    fn default() -> Self {
        ApprovalPolicy::Any
    }
}

impl ProcessStep {
    /// 创建步骤
    pub fn new(name: &str, step_type: StepType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            step_type,
            description: None,
            next_step_id: None,
            branches: Vec::new(),
            parallel_branches: Vec::new(),
            join_step_id: None,
            sub_process_id: None,
            algorithm_id: None,
            algorithm_params: HashMap::new(),
            rule_set_id: None,
            decision_table_id: None,
            script: None,
            approvers: Vec::new(),
            approval_policy: ApprovalPolicy::default(),
            timeout_seconds: None,
            input_mapping: HashMap::new(),
            output_mapping: HashMap::new(),
            retry_count: 0,
            retry_interval_seconds: 5,
            async_execution: false,
        }
    }

    /// 设置下一步
    pub fn with_next(mut self, next_step_id: &str) -> Self {
        self.next_step_id = Some(next_step_id.to_string());
        self
    }

    /// 设置算法 ID
    pub fn with_algorithm(mut self, algo_id: &str) -> Self {
        self.algorithm_id = Some(algo_id.to_string());
        self
    }

    /// 设置规则集
    pub fn with_rule_set(mut self, rule_set: &str) -> Self {
        self.rule_set_id = Some(rule_set.to_string());
        self
    }

    /// 添加决策分支
    pub fn add_branch(&mut self, name: &str, condition: &str, target: &str, priority: u32) {
        self.branches.push(DecisionBranch {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            condition: condition.to_string(),
            target_step_id: target.to_string(),
            priority,
        });
        self.branches.sort_by(|a, b| b.priority.cmp(&a.priority));
    }
}

/// 流程定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDef {
    /// 流程 ID
    pub id: String,
    /// 流程名称
    pub name: String,
    /// 流程版本
    pub version: String,
    /// 所属分类
    pub category: String,
    /// 描述
    pub description: Option<String>,
    /// 开始步骤 ID
    pub start_step_id: String,
    /// 结束步骤 ID
    pub end_step_id: String,
    /// 所有步骤
    pub steps: HashMap<String, ProcessStep>,
    /// 流程变量定义
    pub variables: Vec<ProcessVariableDef>,
    /// 是否启用
    pub enabled: bool,
    /// 创建时间
    pub created_at: u64,
    /// 更新时间
    pub updated_at: u64,
}

/// 流程变量定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessVariableDef {
    pub name: String,
    pub var_type: String,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub required: bool,
}

impl ProcessDef {
    /// 创建流程定义
    pub fn new(name: &str, category: &str) -> Self {
        let start = ProcessStep::new("开始", StepType::Start);
        let end = ProcessStep::new("结束", StepType::End);
        let start_id = start.id.clone();
        let end_id = end.id.clone();

        let mut steps = HashMap::new();
        steps.insert(start.id.clone(), start);
        steps.insert(end.id.clone(), end);

        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            category: category.to_string(),
            description: None,
            start_step_id: start_id,
            end_step_id: end_id,
            steps,
            variables: Vec::new(),
            enabled: true,
            created_at: crate::types::now_ms(),
            updated_at: crate::types::now_ms(),
        }
    }

    /// 添加步骤
    pub fn add_step(&mut self, step: ProcessStep) -> String {
        let id = step.id.clone();
        self.steps.insert(step.id.clone(), step);
        self.updated_at = crate::types::now_ms();
        id
    }

    /// 获取步骤
    pub fn get_step(&self, step_id: &str) -> Option<&ProcessStep> {
        self.steps.get(step_id)
    }

    /// 连接两个步骤
    pub fn connect(&mut self, from_step_id: &str, to_step_id: &str) {
        if let Some(step) = self.steps.get_mut(from_step_id) {
            step.next_step_id = Some(to_step_id.to_string());
        }
        self.updated_at = crate::types::now_ms();
    }

    /// 获取步骤数量
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_def_creation() {
        let process = ProcessDef::new("leave_request", "hr");
        assert_eq!(process.name, "leave_request");
        assert_eq!(process.category, "hr");
        assert_eq!(process.step_count(), 2); // 开始 + 结束
        assert!(process.enabled);
    }

    #[test]
    fn test_add_steps_and_connect() {
        let mut process = ProcessDef::new("simple", "test");

        let step1 = ProcessStep::new("Step1", StepType::Script);
        let step1_id = process.add_step(step1);

        let step2 = ProcessStep::new("Step2", StepType::Script);
        let step2_id = process.add_step(step2);

        assert_eq!(process.step_count(), 4);

        process.connect(&process.start_step_id.clone(), &step1_id);
        process.connect(&step1_id, &step2_id);
        process.connect(&step2_id, &process.end_step_id.clone());

        let start = process.get_step(&process.start_step_id).unwrap();
        assert_eq!(start.next_step_id.as_deref(), Some(step1_id.as_str()));
    }

    #[test]
    fn test_decision_branches() {
        let mut step = ProcessStep::new("CheckAmount", StepType::Decision);
        step.add_branch("high", "amount > 10000", "manager_approval", 100);
        step.add_branch("low", "amount <= 10000", "auto_approve", 50);

        assert_eq!(step.branches.len(), 2);
        assert_eq!(step.branches[0].name, "high"); // 高优先级排前面
    }

    #[test]
    fn test_step_builder() {
        let step = ProcessStep::new("RunAlgo", StepType::Algorithm)
            .with_algorithm("page_rank_v2")
            .with_rule_set("scoring");

        assert_eq!(step.algorithm_id.as_deref(), Some("page_rank_v2"));
        assert_eq!(step.rule_set_id.as_deref(), Some("scoring"));
    }
}
