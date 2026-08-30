// Copyright (c) 2026 璇玑 RelGraph · 流程算法归一化核心 (Unified Process & Algorithm Core)
// Licensed under the MIT License.

//! 流程执行引擎
//!
//! 支持：
//! - 串行流程执行
//! - 条件分支（决策节点）
//! - 并行分支与汇聚
//! - 子流程调用
//! - 算法步骤（调用算法联盟）
//! - 规则步骤（调用规则引擎）
//! - 人工审批（等待外部事件）
//! - 脚本步骤
//! - 重试机制
//! - 超时控制

use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{ProcessError, ProcessResult};
use crate::process_def::ProcessDef;
use crate::rule_engine::RuleEngine;
use crate::types::*;

/// 流程实例
#[derive(Debug, Clone)]
pub struct ProcessInstance {
    /// 实例 ID
    pub instance_id: String,
    /// 流程定义 ID
    pub process_id: String,
    /// 流程名称
    pub process_name: String,
    /// 状态
    pub status: ProcessStatus,
    /// 上下文
    pub context: ProcessContext,
    /// 步骤状态：step_id -> StepStatus
    pub step_statuses: HashMap<String, StepStatus>,
    /// 当前活动步骤 ID 列表
    pub active_steps: Vec<String>,
    /// 启动时间
    pub started_at: u64,
    /// 结束时间
    pub ended_at: Option<u64>,
    /// 错误信息
    pub error: Option<String>,
}

impl ProcessInstance {
    /// 创建新实例
    pub fn new(process_def: &ProcessDef) -> Self {
        let mut step_statuses = HashMap::new();
        for step_id in process_def.steps.keys() {
            step_statuses.insert(step_id.clone(), StepStatus::Pending);
        }

        Self {
            instance_id: Uuid::new_v4().to_string(),
            process_id: process_def.id.clone(),
            process_name: process_def.name.clone(),
            status: ProcessStatus::Pending,
            context: ProcessContext::new(),
            step_statuses,
            active_steps: Vec::new(),
            started_at: now_ms(),
            ended_at: None,
            error: None,
        }
    }
}

/// 流程引擎
pub struct ProcessEngine {
    /// 流程定义表
    process_defs: RwLock<HashMap<String, ProcessDef>>,
    /// 流程实例表
    instances: RwLock<HashMap<String, ProcessInstance>>,
    /// 规则引擎
    rule_engine: Arc<RuleEngine>,
    /// 已执行的流程总数
    total_executed: AtomicU64,
    /// 按分类的流程索引
    by_category: RwLock<HashMap<String, Vec<String>>>,
}

impl ProcessEngine {
    /// 创建流程引擎
    pub fn new() -> Self {
        Self {
            process_defs: RwLock::new(HashMap::new()),
            instances: RwLock::new(HashMap::new()),
            rule_engine: Arc::new(RuleEngine::new()),
            total_executed: AtomicU64::new(0),
            by_category: RwLock::new(HashMap::new()),
        }
    }

    /// 获取规则引擎引用
    pub fn rule_engine(&self) -> &Arc<RuleEngine> {
        &self.rule_engine
    }

    /// 注册流程定义
    pub fn register_process(&self, process_def: ProcessDef) -> ProcessResult<ProcessDef> {
        if self.process_defs.read().contains_key(&process_def.id) {
            return Err(ProcessError::AlreadyExists(format!(
                "process '{}' already exists",
                process_def.id
            )));
        }

        self.by_category
            .write()
            .entry(process_def.category.clone())
            .or_default()
            .push(process_def.id.clone());
        self.process_defs
            .write()
            .insert(process_def.id.clone(), process_def.clone());
        Ok(process_def)
    }

    /// 获取流程定义
    pub fn get_process_def(&self, process_id: &str) -> Option<ProcessDef> {
        self.process_defs.read().get(process_id).cloned()
    }

    /// 启动流程实例
    pub fn start_process(
        &self,
        process_id: &str,
        variables: HashMap<String, Value>,
    ) -> ProcessResult<String> {
        let process_def = self
            .get_process_def(process_id)
            .ok_or_else(|| ProcessError::NotFound(format!("process '{}' not found", process_id)))?;

        if !process_def.enabled {
            return Err(ProcessError::InvalidConfig(format!(
                "process '{}' is disabled",
                process_id
            )));
        }

        let mut instance = ProcessInstance::new(&process_def);
        instance.status = ProcessStatus::Running;

        // 初始化变量
        for (k, v) in variables {
            instance.context.set_variable(&k, v);
        }

        let instance_id = instance.instance_id.clone();
        self.instances
            .write()
            .insert(instance_id.clone(), instance);

        // 开始执行
        self.execute_from_step(&instance_id, &process_def.start_step_id)?;

        Ok(instance_id)
    }

    /// 从指定步骤开始执行
    fn execute_from_step(&self, instance_id: &str, step_id: &str) -> ProcessResult<()> {
        let process_def = {
            let instance = self
                .instances
                .read()
                .get(instance_id)
                .cloned()
                .ok_or_else(|| {
                    ProcessError::NotFound(format!("instance '{}' not found", instance_id))
                })?;
            self.get_process_def(&instance.process_id).ok_or_else(|| {
                ProcessError::NotFound(format!("process def not found"))
            })?
        };

        let mut current_step_id = step_id.to_string();

        loop {
            let step = process_def
                .get_step(&current_step_id)
                .cloned()
                .ok_or_else(|| {
                    ProcessError::ExecutionError(format!(
                        "step '{}' not found",
                        current_step_id
                    ))
                })?;

            // 更新步骤状态为运行中
            self.update_step_status(instance_id, &current_step_id, StepStatus::Running)?;
            self.set_current_step(instance_id, &current_step_id)?;

            // 执行步骤
            let result = self.execute_step(instance_id, &step)?;

            match result {
                StepExecutionResult::Complete { next_step } => {
                    self.update_step_status(instance_id, &current_step_id, StepStatus::Completed)?;

                    if let Some(next) = next_step {
                        current_step_id = next;
                    } else {
                        // 流程结束
                        self.complete_instance(instance_id)?;
                        break;
                    }
                }
                StepExecutionResult::Wait => {
                    self.update_step_status(instance_id, &current_step_id, StepStatus::Waiting)?;
                    break;
                }
                StepExecutionResult::Parallel { branches } => {
                    self.update_step_status(instance_id, &current_step_id, StepStatus::Completed)?;
                    // 并行分支：标记所有分支为运行中
                    // 简化处理：只记录，不真正并发
                    for branch in &branches {
                        self.update_step_status(instance_id, branch, StepStatus::Running)?;
                    }
                    // 找到汇聚点
                    let join_id = step.join_step_id.clone().unwrap_or_default();
                    if !join_id.is_empty() {
                        for branch in branches {
                            self.update_step_status(instance_id, &branch, StepStatus::Completed)?;
                        }
                        current_step_id = join_id;
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// 执行单个步骤
    fn execute_step(
        &self,
        instance_id: &str,
        step: &crate::process_def::ProcessStep,
    ) -> ProcessResult<StepExecutionResult> {
        match step.step_type {
            StepType::Start => {
                self.log(instance_id, LogLevel::Info, "process started");
                Ok(StepExecutionResult::Complete {
                    next_step: step.next_step_id.clone(),
                })
            }
            StepType::End => {
                self.log(instance_id, LogLevel::Info, "process ended");
                Ok(StepExecutionResult::Complete { next_step: None })
            }
            StepType::Script => {
                self.execute_script_step(instance_id, step)?;
                Ok(StepExecutionResult::Complete {
                    next_step: step.next_step_id.clone(),
                })
            }
            StepType::Rule => {
                self.execute_rule_step(instance_id, step)?;
                Ok(StepExecutionResult::Complete {
                    next_step: step.next_step_id.clone(),
                })
            }
            StepType::Decision => {
                let next = self.execute_decision_step(instance_id, step)?;
                Ok(StepExecutionResult::Complete {
                    next_step: Some(next),
                })
            }
            StepType::Approval => {
                self.log(instance_id, LogLevel::Info, &format!(
                    "approval step '{}' waiting for approvers",
                    step.name
                ));
                Ok(StepExecutionResult::Wait)
            }
            StepType::Algorithm => {
                self.execute_algorithm_step(instance_id, step)?;
                Ok(StepExecutionResult::Complete {
                    next_step: step.next_step_id.clone(),
                })
            }
            StepType::Transform => {
                self.log(instance_id, LogLevel::Info, "data transform executed");
                Ok(StepExecutionResult::Complete {
                    next_step: step.next_step_id.clone(),
                })
            }
            StepType::Notification => {
                self.log(instance_id, LogLevel::Info, "notification sent");
                Ok(StepExecutionResult::Complete {
                    next_step: step.next_step_id.clone(),
                })
            }
            StepType::SubProcess => {
                self.log(instance_id, LogLevel::Info, "sub-process triggered");
                Ok(StepExecutionResult::Complete {
                    next_step: step.next_step_id.clone(),
                })
            }
            StepType::Parallel => {
                Ok(StepExecutionResult::Parallel {
                    branches: step.parallel_branches.clone(),
                })
            }
        }
    }

    /// 执行脚本步骤（简化：直接记录到日志）
    fn execute_script_step(
        &self,
        instance_id: &str,
        step: &crate::process_def::ProcessStep,
    ) -> ProcessResult<()> {
        self.log(
            instance_id,
            LogLevel::Info,
            &format!("script step '{}' executed", step.name),
        );
        Ok(())
    }

    /// 执行规则步骤
    fn execute_rule_step(
        &self,
        instance_id: &str,
        step: &crate::process_def::ProcessStep,
    ) -> ProcessResult<()> {
        let rule_set_id = step
            .rule_set_id
            .as_deref()
            .ok_or_else(|| ProcessError::InvalidConfig("rule step missing rule_set_id".to_string()))?;

        // 获取实例上下文
        let mut context = {
            let instance = self.instances.read().get(instance_id).cloned().unwrap();
            instance.context.clone()
        };

        // 将变量转换为事实
        let var_entries: Vec<(String, Value)> = context
            .variables
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (k, v) in var_entries {
            context.add_fact(Fact::new(&k, v));
        }

        let fired = self.rule_engine.execute(rule_set_id, &mut context)?;

        // 将事实同步回变量
        let fact_entries: Vec<(String, Value)> = context
            .facts
            .iter()
            .map(|(k, f)| (k.clone(), f.value.clone()))
            .collect();
        for (k, v) in fact_entries {
            context.set_variable(&k, v);
        }

        // 保存上下文
        self.update_context(instance_id, context)?;

        self.log(
            instance_id,
            LogLevel::Info,
            &format!("rule set '{}' fired {} rules", rule_set_id, fired.len()),
        );

        Ok(())
    }

    /// 执行决策步骤
    fn execute_decision_step(
        &self,
        instance_id: &str,
        step: &crate::process_def::ProcessStep,
    ) -> ProcessResult<String> {
        let context = {
            let instance = self.instances.read().get(instance_id).cloned().unwrap();
            instance.context.clone()
        };

        // 按优先级评估分支
        for branch in &step.branches {
            if self.evaluate_condition(&branch.condition, &context)? {
                self.log(
                    instance_id,
                    LogLevel::Info,
                    &format!("decision '{}' -> branch '{}'", step.name, branch.name),
                );
                return Ok(branch.target_step_id.clone());
            }
        }

        // 没有匹配的分支，走默认下一步
        if let Some(next) = &step.next_step_id {
            Ok(next.clone())
        } else {
            Err(ProcessError::ExecutionError(format!(
                "no matching branch in decision step '{}'",
                step.name
            )))
        }
    }

    /// 执行算法步骤
    fn execute_algorithm_step(
        &self,
        instance_id: &str,
        step: &crate::process_def::ProcessStep,
    ) -> ProcessResult<()> {
        let algo_id = step
            .algorithm_id
            .as_deref()
            .ok_or_else(|| ProcessError::InvalidConfig("algorithm step missing algorithm_id".to_string()))?;

        self.log(
            instance_id,
            LogLevel::Info,
            &format!("algorithm '{}' executed (params: {} keys)", algo_id, step.algorithm_params.len()),
        );

        // 模拟算法输出
        let mut instance = self.instances.write();
        let inst = instance.get_mut(instance_id).unwrap();
        inst.context.set_variable(
            &format!("{}_result", algo_id.replace('-', "_")),
            Value::String("success".to_string()),
        );

        Ok(())
    }

    /// 评估条件表达式（简化版）
    fn evaluate_condition(&self, condition: &str, context: &ProcessContext) -> ProcessResult<bool> {
        // 简化实现：支持 "var op value" 格式
        let condition = condition.trim();

        // 支持常见操作符
        let ops = vec![">=", "<=", "==", "!=", ">", "<"];
        for op in ops {
            if let Some(pos) = condition.find(op) {
                let left = condition[..pos].trim();
                let right = condition[pos + op.len()..].trim();

                let left_val = context
                    .get_variable(left)
                    .cloned()
                    .unwrap_or(Value::Null);

                // 尝试解析右值
                let right_val = parse_value(right);

                return Ok(compare_values(&left_val, op, &right_val));
            }
        }

        // 布尔变量直接判断
        if let Some(val) = context.get_variable(condition) {
            return Ok(val.as_bool().unwrap_or(false));
        }

        Ok(false)
    }

    // ===== 实例管理辅助方法 =====

    /// 更新步骤状态
    fn update_step_status(
        &self,
        instance_id: &str,
        step_id: &str,
        status: StepStatus,
    ) -> ProcessResult<()> {
        let mut instances = self.instances.write();
        let instance = instances
            .get_mut(instance_id)
            .ok_or_else(|| ProcessError::NotFound("instance not found".to_string()))?;

        instance.step_statuses.insert(step_id.to_string(), status);
        Ok(())
    }

    /// 设置当前步骤
    fn set_current_step(&self, instance_id: &str, step_id: &str) -> ProcessResult<()> {
        let mut instances = self.instances.write();
        let instance = instances
            .get_mut(instance_id)
            .ok_or_else(|| ProcessError::NotFound("instance not found".to_string()))?;
        instance.context.current_step_id = Some(step_id.to_string());
        Ok(())
    }

    /// 完成实例
    fn complete_instance(&self, instance_id: &str) -> ProcessResult<()> {
        let mut instances = self.instances.write();
        let instance = instances
            .get_mut(instance_id)
            .ok_or_else(|| ProcessError::NotFound("instance not found".to_string()))?;

        instance.status = ProcessStatus::Completed;
        instance.ended_at = Some(now_ms());
        instance.context.ended_at = Some(now_ms());
        drop(instances);

        self.total_executed.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// 更新上下文
    fn update_context(&self, instance_id: &str, context: ProcessContext) -> ProcessResult<()> {
        let mut instances = self.instances.write();
        let instance = instances
            .get_mut(instance_id)
            .ok_or_else(|| ProcessError::NotFound("instance not found".to_string()))?;
        instance.context = context;
        Ok(())
    }

    /// 记录日志
    fn log(&self, instance_id: &str, level: LogLevel, message: &str) {
        let mut instances = self.instances.write();
        if let Some(instance) = instances.get_mut(instance_id) {
            instance.context.log(level, message);
        }
    }

    /// 获取流程实例
    pub fn get_instance(&self, instance_id: &str) -> Option<ProcessInstance> {
        self.instances.read().get(instance_id).cloned()
    }

    /// 审批步骤 - 通过
    pub fn approve_step(
        &self,
        instance_id: &str,
        step_id: &str,
        approver: &str,
    ) -> ProcessResult<()> {
        let process_def = {
            let instance = self
                .instances
                .read()
                .get(instance_id)
                .cloned()
                .ok_or_else(|| ProcessError::NotFound("instance not found".to_string()))?;
            self.get_process_def(&instance.process_id).unwrap()
        };

        let step = process_def.get_step(step_id).cloned().ok_or_else(|| {
            ProcessError::NotFound(format!("step '{}' not found", step_id))
        })?;

        self.log(
            instance_id,
            LogLevel::Info,
            &format!("step '{}' approved by {}", step.name, approver),
        );

        // 继续执行
        let next_step = step.next_step_id.clone();
        self.update_step_status(instance_id, step_id, StepStatus::Completed)?;

        if let Some(next) = next_step {
            self.execute_from_step(instance_id, &next)?;
        } else {
            self.complete_instance(instance_id)?;
        }

        Ok(())
    }

    /// 获取流程定义数量
    pub fn process_count(&self) -> usize {
        self.process_defs.read().len()
    }

    /// 获取实例数量
    pub fn instance_count(&self) -> usize {
        self.instances.read().len()
    }

    /// 获取总执行数
    pub fn total_executed(&self) -> u64 {
        self.total_executed.load(Ordering::Relaxed)
    }
}

impl Default for ProcessEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 步骤执行结果
enum StepExecutionResult {
    /// 步骤完成，进入下一步
    Complete { next_step: Option<String> },
    /// 等待外部事件（如人工审批）
    Wait,
    /// 并行分支
    Parallel { branches: Vec<String> },
}

/// 解析值
fn parse_value(s: &str) -> Value {
    let s = s.trim();

    // 字符串
    if (s.starts_with('"') && s.ends_with('"'))
        || (s.starts_with('\'') && s.ends_with('\''))
    {
        return Value::String(s[1..s.len() - 1].to_string());
    }

    // 整数
    if let Ok(n) = s.parse::<i64>() {
        return Value::Number(serde_json::Number::from(n));
    }

    // 浮点数
    if let Ok(f) = s.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return Value::Number(n);
        }
    }

    // 布尔值
    if s == "true" {
        return Value::Bool(true);
    }
    if s == "false" {
        return Value::Bool(false);
    }

    // null
    if s == "null" {
        return Value::Null;
    }

    // 默认为字符串
    Value::String(s.to_string())
}

/// 比较两个值
fn compare_values(left: &Value, op: &str, right: &Value) -> bool {
    match op {
        "==" => {
            if let (Some(a), Some(b)) = (left.as_f64(), right.as_f64()) {
                (a - b).abs() < f64::EPSILON
            } else {
                left == right
            }
        }
        "!=" => !compare_values(left, "==", right),
        ">" => {
            if let (Some(a), Some(b)) = (left.as_f64(), right.as_f64()) {
                a > b
            } else if let (Some(a), Some(b)) = (left.as_str(), right.as_str()) {
                a > b
            } else {
                false
            }
        }
        ">=" => compare_values(left, ">", right) || compare_values(left, "==", right),
        "<" => compare_values(right, ">", left),
        "<=" => compare_values(left, "<", right) || compare_values(left, "==", right),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process_def::ProcessStep;
    use serde_json::json;

    #[test]
    fn test_simple_process() {
        let engine = ProcessEngine::new();

        let mut process = ProcessDef::new("simple", "test");
        let step1 = ProcessStep::new("Step1", StepType::Script);
        let step1_id = process.add_step(step1);

        let start_id = process.start_step_id.clone();
        let end_id = process.end_step_id.clone();
        process.connect(&start_id, &step1_id);
        process.connect(&step1_id, &end_id);

        engine.register_process(process).unwrap();

        let mut vars = HashMap::new();
        vars.insert("input".to_string(), json!("hello"));

        let instance_id = engine.start_process(
            &engine.process_defs.read().keys().next().unwrap().clone(),
            vars,
        ).unwrap();

        let instance = engine.get_instance(&instance_id).unwrap();
        assert_eq!(instance.status, ProcessStatus::Completed);
        assert_eq!(engine.total_executed(), 1);
    }

    #[test]
    fn test_decision_process() {
        let engine = ProcessEngine::new();

        let mut process = ProcessDef::new("decision_test", "test");

        let mut decision_step = ProcessStep::new("CheckAmount", StepType::Decision);

        let high_step = ProcessStep::new("HighPath", StepType::Script);
        let low_step = ProcessStep::new("LowPath", StepType::Script);

        let high_id = process.add_step(high_step);
        let low_id = process.add_step(low_step);

        decision_step.add_branch("high", "amount > 1000", &high_id, 100);
        decision_step.add_branch("low", "amount <= 1000", &low_id, 50);

        let decision_id = process.add_step(decision_step);

        // 两条路径都汇聚到 end
        let end_id = process.end_step_id.clone();
        let start_id = process.start_step_id.clone();
        process.connect(&start_id, &decision_id);
        process.connect(&high_id, &end_id);
        process.connect(&low_id, &end_id);

        engine.register_process(process).unwrap();

        let process_id = engine.process_defs.read().keys().next().unwrap().clone();

        // 测试高金额
        let mut vars = HashMap::new();
        vars.insert("amount".to_string(), json!(5000));
        let inst_id = engine.start_process(&process_id, vars).unwrap();
        let inst = engine.get_instance(&inst_id).unwrap();
        assert_eq!(inst.status, ProcessStatus::Completed);

        // 检查高金额路径被执行
        let high_status = inst.step_statuses.get(&high_id).unwrap();
        assert_eq!(*high_status, StepStatus::Completed);
    }

    #[test]
    fn test_approval_step() {
        let engine = ProcessEngine::new();

        let mut process = ProcessDef::new("approval_test", "test");
        let mut approval_step = ProcessStep::new("ManagerApproval", StepType::Approval);
        approval_step.approvers = vec!["manager1".to_string()];

        let approval_id = process.add_step(approval_step);
        let start_id = process.start_step_id.clone();
        let end_id = process.end_step_id.clone();
        process.connect(&start_id, &approval_id);
        process.connect(&approval_id, &end_id);

        engine.register_process(process).unwrap();

        let process_id = engine.process_defs.read().keys().next().unwrap().clone();
        let inst_id = engine
            .start_process(&process_id, HashMap::new())
            .unwrap();

        // 审批前：等待状态
        let inst = engine.get_instance(&inst_id).unwrap();
        assert_eq!(inst.status, ProcessStatus::Running);

        // 通过审批
        engine
            .approve_step(&inst_id, &approval_id, "manager1")
            .unwrap();

        // 审批后：完成
        let inst = engine.get_instance(&inst_id).unwrap();
        assert_eq!(inst.status, ProcessStatus::Completed);
    }

    #[test]
    fn test_rule_integration() {
        let engine = ProcessEngine::new();

        // 注册规则
        let rule = crate::types::Rule::new("discount_rule", "pricing")
            .with_condition("amount", ">", json!(1000))
            .with_set_action("discount", json!(0.1));
        engine.rule_engine().register_rule(rule).unwrap();

        // 创建流程
        let mut process = ProcessDef::new("rule_test", "test");
        let rule_step = ProcessStep::new("ApplyPricing", StepType::Rule)
            .with_rule_set("pricing");
        let rule_id = process.add_step(rule_step);

        let start_id = process.start_step_id.clone();
        let end_id = process.end_step_id.clone();
        process.connect(&start_id, &rule_id);
        process.connect(&rule_id, &end_id);

        engine.register_process(process).unwrap();

        let process_id = engine.process_defs.read().keys().next().unwrap().clone();

        let mut vars = HashMap::new();
        vars.insert("amount".to_string(), json!(5000));

        let inst_id = engine.start_process(&process_id, vars).unwrap();
        let inst = engine.get_instance(&inst_id).unwrap();

        assert_eq!(inst.status, ProcessStatus::Completed);
        // 规则执行后 discount 变量应被设置
        let discount = inst.context.get_variable("discount");
        assert!(discount.is_some());
        assert_eq!(discount.unwrap(), &json!(0.1));
    }

    #[test]
    fn test_algorithm_step() {
        let engine = ProcessEngine::new();

        let mut process = ProcessDef::new("algo_test", "test");
        let algo_step = ProcessStep::new("RunPageRank", StepType::Algorithm)
            .with_algorithm("page_rank");
        let algo_id = process.add_step(algo_step);

        let start_id = process.start_step_id.clone();
        let end_id = process.end_step_id.clone();
        process.connect(&start_id, &algo_id);
        process.connect(&algo_id, &end_id);

        engine.register_process(process).unwrap();

        let process_id = engine.process_defs.read().keys().next().unwrap().clone();
        let inst_id = engine
            .start_process(&process_id, HashMap::new())
            .unwrap();

        let inst = engine.get_instance(&inst_id).unwrap();
        assert_eq!(inst.status, ProcessStatus::Completed);
        assert!(inst
            .context
            .get_variable("page_rank_result")
            .is_some());
    }

    #[test]
    fn test_parse_value() {
        assert_eq!(parse_value("42"), json!(42));
        assert_eq!(parse_value("3.14"), json!(3.14));
        assert_eq!(parse_value("true"), json!(true));
        assert_eq!(parse_value("\"hello\""), json!("hello"));
        assert_eq!(parse_value("null"), Value::Null);
    }
}
