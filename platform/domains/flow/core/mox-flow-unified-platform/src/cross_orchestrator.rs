// Copyright (c) 2026 璇玑 RelGraph · mox 模块化系统架构归一化统一平台 (Unified Platform)
// Licensed under the MIT License.

//! 跨系统协同编排引擎
//!
//! 六大归一化体系不是孤立存在的，而是通过编排引擎深度协同：
//! - AI 对话 → 权限校验 → 低代码生成 → 流程执行 → 前端渲染 → 架构输出
//!
//! 典型场景：
//! 用户说"帮我创建一个请假申请" → AI识别意图 → 校验请假权限 →
//! 低代码生成请假表单 → 启动审批流程 → 返回前端页面组件

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{PlatformError, PlatformResult};
use crate::types::NormalizationSystem;

/// 编排步骤类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrchestrationStepType {
    /// AI 意图理解
    AiIntent,
    /// 权限校验
    PermissionCheck,
    /// 低代码实体/表单生成
    LowcodeGenerate,
    /// 流程执行
    ProcessExecute,
    /// 前端组件渲染
    FrontendRender,
    /// 架构协议输出
    ArchOutput,
    /// 算法调用
    AlgoInvoke,
}

impl OrchestrationStepType {
    pub fn system(&self) -> NormalizationSystem {
        match self {
            OrchestrationStepType::AiIntent | OrchestrationStepType::AlgoInvoke => {
                NormalizationSystem::AiAssistant
            }
            OrchestrationStepType::PermissionCheck => NormalizationSystem::Permission,
            OrchestrationStepType::LowcodeGenerate => NormalizationSystem::Lowcode,
            OrchestrationStepType::ProcessExecute => NormalizationSystem::ProcessAlgo,
            OrchestrationStepType::FrontendRender => NormalizationSystem::Frontend,
            OrchestrationStepType::ArchOutput => NormalizationSystem::Architecture,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            OrchestrationStepType::AiIntent => "AI意图理解",
            OrchestrationStepType::PermissionCheck => "权限校验",
            OrchestrationStepType::LowcodeGenerate => "低代码生成",
            OrchestrationStepType::ProcessExecute => "流程执行",
            OrchestrationStepType::FrontendRender => "前端渲染",
            OrchestrationStepType::ArchOutput => "架构输出",
            OrchestrationStepType::AlgoInvoke => "算法调用",
        }
    }
}

/// 编排步骤状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// 等待中
    Pending,
    /// 执行中
    Running,
    /// 成功
    Success,
    /// 失败
    Failed,
    /// 跳过
    Skipped,
}

/// 编排步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationStep {
    /// 步骤 ID
    pub id: String,
    /// 步骤类型
    pub step_type: OrchestrationStepType,
    /// 步骤名称
    pub name: String,
    /// 状态
    pub status: StepStatus,
    /// 输入数据
    pub input: serde_json::Value,
    /// 输出数据
    pub output: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 依赖的前置步骤 ID
    pub depends_on: Vec<String>,
    /// 执行顺序
    pub order: u32,
}

/// 编排上下文 - 在各步骤间传递数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchestrationContext {
    /// 租户 ID
    pub tenant_id: String,
    /// 用户 ID
    pub user_id: String,
    /// 原始请求
    pub original_request: String,
    /// 上下文变量
    pub variables: HashMap<String, serde_json::Value>,
    /// 当前步骤索引
    pub current_step: usize,
}

/// 编排结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationResult {
    /// 编排 ID
    pub orchestration_id: String,
    /// 是否成功
    pub success: bool,
    /// 总步骤数
    pub total_steps: usize,
    /// 成功步骤数
    pub completed_steps: usize,
    /// 失败步骤数
    pub failed_steps: usize,
    /// 各步骤详情
    pub steps: Vec<OrchestrationStep>,
    /// 最终输出
    pub final_output: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
}

/// 跨系统编排引擎
pub struct CrossOrchestrator {
    /// 已注册的编排模板
    templates: HashMap<String, OrchestrationTemplate>,
}

/// 编排模板 - 预定义的跨系统协同流程
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationTemplate {
    /// 模板 ID
    pub id: String,
    /// 模板名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 步骤定义
    pub steps: Vec<TemplateStep>,
}

/// 模板步骤定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateStep {
    /// 步骤 ID（模板内唯一）
    pub id: String,
    /// 步骤类型
    pub step_type: OrchestrationStepType,
    /// 步骤名称
    pub name: String,
    /// 依赖步骤
    pub depends_on: Vec<String>,
    /// 输入映射（从上下文变量到步骤输入）
    pub input_mapping: HashMap<String, String>,
    /// 输出映射（从步骤输出到上下文变量）
    pub output_mapping: HashMap<String, String>,
    /// 是否可选（失败时跳过而非终止）
    pub optional: bool,
}

impl CrossOrchestrator {
    /// 创建编排引擎
    pub fn new() -> Self {
        let mut orchestrator = Self {
            templates: HashMap::new(),
        };
        orchestrator.register_builtin_templates();
        orchestrator
    }

    /// 注册内置编排模板
    fn register_builtin_templates(&mut self) {
        // 模板1：AI驱动的业务申请全流程
        self.templates.insert(
            "ai-business-request".to_string(),
            OrchestrationTemplate {
                id: "ai-business-request".to_string(),
                name: "AI驱动业务申请".to_string(),
                description: "AI理解用户请求 → 权限校验 → 生成低代码表单 → 启动审批流程 → 返回前端页面".to_string(),
                steps: vec![
                    TemplateStep {
                        id: "intent".to_string(),
                        step_type: OrchestrationStepType::AiIntent,
                        name: "意图理解".to_string(),
                        depends_on: vec![],
                        input_mapping: {
                            let mut m = HashMap::new();
                            m.insert("text".to_string(), "original_request".to_string());
                            m
                        },
                        output_mapping: {
                            let mut m = HashMap::new();
                            m.insert("intent".to_string(), "intent_name".to_string());
                            m.insert("entities".to_string(), "intent_entities".to_string());
                            m
                        },
                        optional: false,
                    },
                    TemplateStep {
                        id: "perm-check".to_string(),
                        step_type: OrchestrationStepType::PermissionCheck,
                        name: "权限校验".to_string(),
                        depends_on: vec!["intent".to_string()],
                        input_mapping: {
                            let mut m = HashMap::new();
                            m.insert("action".to_string(), "intent_name".to_string());
                            m.insert("resource".to_string(), "resource_type".to_string());
                            m
                        },
                        output_mapping: HashMap::new(),
                        optional: false,
                    },
                    TemplateStep {
                        id: "form-gen".to_string(),
                        step_type: OrchestrationStepType::LowcodeGenerate,
                        name: "生成表单".to_string(),
                        depends_on: vec!["perm-check".to_string()],
                        input_mapping: {
                            let mut m = HashMap::new();
                            m.insert("entity_type".to_string(), "intent_name".to_string());
                            m.insert("fields".to_string(), "intent_entities".to_string());
                            m
                        },
                        output_mapping: {
                            let mut m = HashMap::new();
                            m.insert("form_schema".to_string(), "form_schema".to_string());
                            m.insert("entity_id".to_string(), "entity_id".to_string());
                            m
                        },
                        optional: false,
                    },
                    TemplateStep {
                        id: "process-start".to_string(),
                        step_type: OrchestrationStepType::ProcessExecute,
                        name: "启动流程".to_string(),
                        depends_on: vec!["form-gen".to_string()],
                        input_mapping: {
                            let mut m = HashMap::new();
                            m.insert("process_key".to_string(), "intent_name".to_string());
                            m.insert("data".to_string(), "entity_id".to_string());
                            m
                        },
                        output_mapping: {
                            let mut m = HashMap::new();
                            m.insert("instance_id".to_string(), "process_instance_id".to_string());
                            m
                        },
                        optional: false,
                    },
                    TemplateStep {
                        id: "ui-render".to_string(),
                        step_type: OrchestrationStepType::FrontendRender,
                        name: "前端渲染".to_string(),
                        depends_on: vec!["process-start".to_string()],
                        input_mapping: {
                            let mut m = HashMap::new();
                            m.insert("form_schema".to_string(), "form_schema".to_string());
                            m.insert("process_status".to_string(), "process_instance_id".to_string());
                            m
                        },
                        output_mapping: {
                            let mut m = HashMap::new();
                            m.insert("page_config".to_string(), "page_config".to_string());
                            m
                        },
                        optional: false,
                    },
                    TemplateStep {
                        id: "response".to_string(),
                        step_type: OrchestrationStepType::ArchOutput,
                        name: "协议输出".to_string(),
                        depends_on: vec!["ui-render".to_string()],
                        input_mapping: {
                            let mut m = HashMap::new();
                            m.insert("payload".to_string(), "page_config".to_string());
                            m
                        },
                        output_mapping: HashMap::new(),
                        optional: false,
                    },
                ],
            },
        );

        // 模板2：纯AI问答（轻量）
        self.templates.insert(
            "ai-query-only".to_string(),
            OrchestrationTemplate {
                id: "ai-query-only".to_string(),
                name: "AI智能问答".to_string(),
                description: "仅使用AI对话能力回答用户问题".to_string(),
                steps: vec![
                    TemplateStep {
                        id: "intent".to_string(),
                        step_type: OrchestrationStepType::AiIntent,
                        name: "意图理解".to_string(),
                        depends_on: vec![],
                        input_mapping: {
                            let mut m = HashMap::new();
                            m.insert("text".to_string(), "original_request".to_string());
                            m
                        },
                        output_mapping: {
                            let mut m = HashMap::new();
                            m.insert("intent".to_string(), "intent_name".to_string());
                            m
                        },
                        optional: false,
                    },
                    TemplateStep {
                        id: "answer".to_string(),
                        step_type: OrchestrationStepType::AiIntent,
                        name: "生成回答".to_string(),
                        depends_on: vec!["intent".to_string()],
                        input_mapping: HashMap::new(),
                        output_mapping: {
                            let mut m = HashMap::new();
                            m.insert("answer".to_string(), "final_answer".to_string());
                            m
                        },
                        optional: false,
                    },
                ],
            },
        );

        // 模板3：算法分析流程
        self.templates.insert(
            "algo-analysis".to_string(),
            OrchestrationTemplate {
                id: "algo-analysis".to_string(),
                name: "算法分析流程".to_string(),
                description: "权限校验 → 算法调用 → 结果可视化 → 前端展示".to_string(),
                steps: vec![
                    TemplateStep {
                        id: "perm".to_string(),
                        step_type: OrchestrationStepType::PermissionCheck,
                        name: "权限校验".to_string(),
                        depends_on: vec![],
                        input_mapping: HashMap::new(),
                        output_mapping: HashMap::new(),
                        optional: false,
                    },
                    TemplateStep {
                        id: "algo".to_string(),
                        step_type: OrchestrationStepType::AlgoInvoke,
                        name: "算法执行".to_string(),
                        depends_on: vec!["perm".to_string()],
                        input_mapping: HashMap::new(),
                        output_mapping: {
                            let mut m = HashMap::new();
                            m.insert("result".to_string(), "algo_result".to_string());
                            m
                        },
                        optional: false,
                    },
                    TemplateStep {
                        id: "viz".to_string(),
                        step_type: OrchestrationStepType::FrontendRender,
                        name: "可视化渲染".to_string(),
                        depends_on: vec!["algo".to_string()],
                        input_mapping: HashMap::new(),
                        output_mapping: HashMap::new(),
                        optional: false,
                    },
                ],
            },
        );
    }

    /// 获取所有模板
    pub fn list_templates(&self) -> Vec<&OrchestrationTemplate> {
        self.templates.values().collect()
    }

    /// 获取模板
    pub fn get_template(&self, id: &str) -> Option<&OrchestrationTemplate> {
        self.templates.get(id)
    }

    /// 注册自定义模板
    pub fn register_template(&mut self, template: OrchestrationTemplate) -> PlatformResult<()> {
        if self.templates.contains_key(&template.id) {
            return Err(PlatformError::ModuleAlreadyExists(format!(
                "template '{}' already exists",
                template.id
            )));
        }
        self.templates.insert(template.id.clone(), template);
        Ok(())
    }

    /// 执行编排
    pub fn execute(
        &self,
        template_id: &str,
        context: OrchestrationContext,
    ) -> PlatformResult<OrchestrationResult> {
        let template = self
            .templates
            .get(template_id)
            .ok_or_else(|| PlatformError::ModuleNotFound(format!("template '{}' not found", template_id)))?;

        // 拓扑排序确定执行顺序
        let steps_order = self.topological_sort(&template.steps)?;

        let mut steps: Vec<OrchestrationStep> = steps_order
            .iter()
            .enumerate()
            .map(|(i, ts)| OrchestrationStep {
                id: ts.id.clone(),
                step_type: ts.step_type,
                name: ts.name.clone(),
                status: StepStatus::Pending,
                input: serde_json::json!({}),
                output: None,
                error: None,
                depends_on: ts.depends_on.clone(),
                order: i as u32,
            })
            .collect();

        let mut ctx = context;
        let mut success = true;
        let mut completed = 0usize;
        let mut failed = 0usize;
        let mut final_output: Option<serde_json::Value> = None;
        let mut last_error: Option<String> = None;

        for step_idx in 0..steps.len() {
            let step = &mut steps[step_idx];
            step.status = StepStatus::Running;
            ctx.current_step = step_idx;

            // 模拟步骤执行（实际生产中会调用各子系统真实API）
            let template_step = &template.steps.iter().find(|s| s.id == step.id).unwrap();
            let execution_result = self.execute_step(step, template_step, &ctx);

            match execution_result {
                Ok(output) => {
                    step.status = StepStatus::Success;
                    step.output = Some(output.clone());
                    completed += 1;
                    final_output = Some(output);

                    // 将输出映射到上下文变量
                    for (src_key, dst_key) in &template_step.output_mapping {
                        if let Some(val) = step.output.as_ref().and_then(|o| o.get(src_key).cloned()) {
                            ctx.variables.insert(dst_key.clone(), val);
                        }
                    }
                }
                Err(e) => {
                    if template_step.optional {
                        step.status = StepStatus::Skipped;
                        step.error = Some(e.to_string());
                    } else {
                        step.status = StepStatus::Failed;
                        step.error = Some(e.to_string());
                        failed += 1;
                        success = false;
                        last_error = Some(e.to_string());
                        break; // 非可选步骤失败，终止编排
                    }
                }
            }
        }

        Ok(OrchestrationResult {
            orchestration_id: uuid::Uuid::new_v4().to_string(),
            success,
            total_steps: steps.len(),
            completed_steps: completed,
            failed_steps: failed,
            steps,
            final_output,
            error: last_error,
        })
    }

    /// 执行单个步骤（模拟各子系统调用）
    fn execute_step(
        &self,
        step: &OrchestrationStep,
        template_step: &TemplateStep,
        context: &OrchestrationContext,
    ) -> PlatformResult<serde_json::Value> {
        match step.step_type {
            OrchestrationStepType::AiIntent => {
                // 模拟AI意图识别
                let intent = if context.original_request.contains("请假") {
                    "leave_application"
                } else if context.original_request.contains("报销") {
                    "expense_report"
                } else if context.original_request.contains("查询") || context.original_request.contains("搜索") {
                    "data_query"
                } else {
                    "general_chat"
                };

                Ok(serde_json::json!({
                    "intent": intent,
                    "confidence": 0.92,
                    "entities": {
                        "user_id": context.user_id,
                        "tenant_id": context.tenant_id,
                    }
                }))
            }
            OrchestrationStepType::PermissionCheck => {
                // 模拟权限校验 - 所有用户都有基础权限
                Ok(serde_json::json!({
                    "allowed": true,
                    "reason": "user has required permissions",
                    "check_time_ms": 2
                }))
            }
            OrchestrationStepType::LowcodeGenerate => {
                // 模拟低代码表单生成
                let intent_name = context
                    .variables
                    .get("intent_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                Ok(serde_json::json!({
                    "form_schema": {
                        "title": format!("{} 申请表单", intent_name),
                        "fields": [
                            {"name": "title", "type": "string", "label": "标题", "required": true},
                            {"name": "reason", "type": "text", "label": "原因", "required": true},
                            {"name": "start_date", "type": "date", "label": "开始日期", "required": true},
                            {"name": "end_date", "type": "date", "label": "结束日期", "required": true},
                        ]
                    },
                    "entity_id": format!("entity_{}", uuid::Uuid::new_v4()),
                    "generated_at": chrono_like_timestamp(),
                }))
            }
            OrchestrationStepType::ProcessExecute => {
                // 模拟流程启动
                Ok(serde_json::json!({
                    "process_instance_id": format!("proc_{}", uuid::Uuid::new_v4()),
                    "status": "running",
                    "current_node": "manager_approval",
                    "started_at": chrono_like_timestamp(),
                }))
            }
            OrchestrationStepType::FrontendRender => {
                // 模拟前端页面渲染
                Ok(serde_json::json!({
                    "page_config": {
                        "layout": "form-detail",
                        "theme": "default",
                        "components": [
                            {"type": "Form", "props": {}},
                            {"type": "ProcessTimeline", "props": {}},
                            {"type": "ActionBar", "props": {}},
                        ]
                    },
                    "render_time_ms": 5,
                }))
            }
            OrchestrationStepType::ArchOutput => {
                // 模拟架构协议输出
                let payload = context.variables.get("page_config")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                Ok(serde_json::json!({
                    "protocol": "rest+graphql",
                    "status_code": 200,
                    "payload": payload,
                    "trace_id": uuid::Uuid::new_v4().to_string(),
                }))
            }
            OrchestrationStepType::AlgoInvoke => {
                // 模拟算法调用
                Ok(serde_json::json!({
                    "algorithm": "auto_selected",
                    "result": {
                        "summary": "analysis completed",
                        "metrics": {"accuracy": 0.95, "latency_ms": 120}
                    },
                    "compute_time_ms": 120,
                }))
            }
        }
    }

    /// 拓扑排序（确定步骤执行顺序）
    fn topological_sort(&self, steps: &[TemplateStep]) -> PlatformResult<Vec<TemplateStep>> {
        let mut result: Vec<TemplateStep> = Vec::new();
        let mut visited: HashMap<String, bool> = HashMap::new();
        let mut in_stack: HashMap<String, bool> = HashMap::new();

        // 构建映射
        let step_map: HashMap<String, &TemplateStep> =
            steps.iter().map(|s| (s.id.clone(), s)).collect();

        fn visit(
            id: &str,
            step_map: &HashMap<String, &TemplateStep>,
            visited: &mut HashMap<String, bool>,
            in_stack: &mut HashMap<String, bool>,
            result: &mut Vec<TemplateStep>,
        ) -> PlatformResult<()> {
            if *in_stack.get(id).unwrap_or(&false) {
                return Err(PlatformError::InitError(format!(
                    "circular dependency detected at step '{}'",
                    id
                )));
            }
            if *visited.get(id).unwrap_or(&false) {
                return Ok(());
            }

            in_stack.insert(id.to_string(), true);

            if let Some(step) = step_map.get(id) {
                for dep in &step.depends_on {
                    visit(dep, step_map, visited, in_stack, result)?;
                }
                result.push((*step).clone());
            }

            in_stack.insert(id.to_string(), false);
            visited.insert(id.to_string(), true);

            Ok(())
        }

        for step in steps {
            visit(&step.id, &step_map, &mut visited, &mut in_stack, &mut result)?;
        }

        Ok(result)
    }

    /// 模板数量
    pub fn template_count(&self) -> usize {
        self.templates.len()
    }
}

impl Default for CrossOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

/// 生成一个类似时间戳的数字（避免引入chrono依赖）
fn chrono_like_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_templates() {
        let orchestrator = CrossOrchestrator::new();
        assert_eq!(orchestrator.template_count(), 3);
        assert!(orchestrator.get_template("ai-business-request").is_some());
        assert!(orchestrator.get_template("ai-query-only").is_some());
        assert!(orchestrator.get_template("algo-analysis").is_some());
    }

    #[test]
    fn test_execute_ai_business_request() {
        let orchestrator = CrossOrchestrator::new();

        let ctx = OrchestrationContext {
            tenant_id: "tenant-1".to_string(),
            user_id: "user-1".to_string(),
            original_request: "我想申请请假".to_string(),
            variables: HashMap::new(),
            current_step: 0,
        };

        let result = orchestrator.execute("ai-business-request", ctx).unwrap();
        assert!(result.success);
        assert_eq!(result.total_steps, 6);
        assert_eq!(result.completed_steps, 6);
        assert_eq!(result.failed_steps, 0);

        // 验证步骤顺序
        let step_types: Vec<OrchestrationStepType> =
            result.steps.iter().map(|s| s.step_type).collect();
        assert_eq!(step_types[0], OrchestrationStepType::AiIntent);
        assert_eq!(step_types[1], OrchestrationStepType::PermissionCheck);
        assert_eq!(step_types[2], OrchestrationStepType::LowcodeGenerate);
        assert_eq!(step_types[3], OrchestrationStepType::ProcessExecute);
        assert_eq!(step_types[4], OrchestrationStepType::FrontendRender);
        assert_eq!(step_types[5], OrchestrationStepType::ArchOutput);

        // 所有步骤都应该成功
        for step in &result.steps {
            assert_eq!(step.status, StepStatus::Success, "step {} failed: {:?}", step.name, step.error);
        }

        // 验证最终输出
        assert!(result.final_output.is_some());
    }

    #[test]
    fn test_execute_ai_query_only() {
        let orchestrator = CrossOrchestrator::new();

        let ctx = OrchestrationContext {
            tenant_id: "t1".to_string(),
            user_id: "u1".to_string(),
            original_request: "你好".to_string(),
            variables: HashMap::new(),
            current_step: 0,
        };

        let result = orchestrator.execute("ai-query-only", ctx).unwrap();
        assert!(result.success);
        assert_eq!(result.total_steps, 2);
    }

    #[test]
    fn test_execute_algo_analysis() {
        let orchestrator = CrossOrchestrator::new();

        let ctx = OrchestrationContext {
            tenant_id: "t1".to_string(),
            user_id: "u1".to_string(),
            original_request: "分析数据".to_string(),
            variables: HashMap::new(),
            current_step: 0,
        };

        let result = orchestrator.execute("algo-analysis", ctx).unwrap();
        assert!(result.success);
        assert_eq!(result.total_steps, 3);
    }

    #[test]
    fn test_unknown_template() {
        let orchestrator = CrossOrchestrator::new();
        let ctx = OrchestrationContext::default();
        let result = orchestrator.execute("nonexistent", ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_register_custom_template() {
        let mut orchestrator = CrossOrchestrator::new();
        let initial = orchestrator.template_count();

        let template = OrchestrationTemplate {
            id: "custom".to_string(),
            name: "自定义流程".to_string(),
            description: "测试".to_string(),
            steps: vec![
                TemplateStep {
                    id: "s1".to_string(),
                    step_type: OrchestrationStepType::AiIntent,
                    name: "步骤1".to_string(),
                    depends_on: vec![],
                    input_mapping: HashMap::new(),
                    output_mapping: HashMap::new(),
                    optional: false,
                },
            ],
        };

        orchestrator.register_template(template).unwrap();
        assert_eq!(orchestrator.template_count(), initial + 1);
    }

    #[test]
    fn test_duplicate_template_fails() {
        let mut orchestrator = CrossOrchestrator::new();
        let template = OrchestrationTemplate {
            id: "ai-query-only".to_string(),
            name: "重复".to_string(),
            description: "测试重复".to_string(),
            steps: vec![],
        };
        assert!(orchestrator.register_template(template).is_err());
    }

    #[test]
    fn test_topological_sort_with_deps() {
        let orchestrator = CrossOrchestrator::new();
        let template = orchestrator.get_template("ai-business-request").unwrap();

        // 验证每个步骤的依赖都在它之前执行
        let steps = orchestrator.topological_sort(&template.steps).unwrap();
        let pos: HashMap<String, usize> = steps
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.clone(), i))
            .collect();

        for step in &steps {
            for dep in &step.depends_on {
                assert!(
                    pos[dep] < pos[&step.id],
                    "step {} should come after its dependency {}",
                    step.id,
                    dep
                );
            }
        }
    }

    #[test]
    fn test_list_templates() {
        let orchestrator = CrossOrchestrator::new();
        let templates = orchestrator.list_templates();
        assert_eq!(templates.len(), 3);
    }

    #[test]
    fn test_step_system_mapping() {
        assert_eq!(
            OrchestrationStepType::AiIntent.system(),
            NormalizationSystem::AiAssistant
        );
        assert_eq!(
            OrchestrationStepType::PermissionCheck.system(),
            NormalizationSystem::Permission
        );
        assert_eq!(
            OrchestrationStepType::LowcodeGenerate.system(),
            NormalizationSystem::Lowcode
        );
        assert_eq!(
            OrchestrationStepType::ProcessExecute.system(),
            NormalizationSystem::ProcessAlgo
        );
        assert_eq!(
            OrchestrationStepType::FrontendRender.system(),
            NormalizationSystem::Frontend
        );
        assert_eq!(
            OrchestrationStepType::ArchOutput.system(),
            NormalizationSystem::Architecture
        );
    }
}
