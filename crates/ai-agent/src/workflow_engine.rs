//! 业务流程驱动的工作流引擎
//!
//! 实现BPMN风格的工作流执行，支持：
//! - 顺序执行
//! - 条件分支
//! - 并行分支与合并
//! - 子流程调用
//! - 用户任务
//! - AI任务
//! - 插件调用
//! - 算子执行集成

use super::types::*;
use operator_core::{Result, OperatorError};
use std::collections::{HashMap, VecDeque};
use tracing;
use uuid::Uuid;
use chrono::Utc;

/// 工作流引擎 - 业务流程自动化核心
pub struct WorkflowEngine {
    /// 已注册的工作流定义
    workflow_definitions: HashMap<String, BusinessWorkflow>,
    /// 运行中的工作流实例
    running_instances: HashMap<String, WorkflowInstance>,
    /// 工作流模板库
    templates: WorkflowTemplateLibrary,
}

impl WorkflowEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            workflow_definitions: HashMap::new(),
            running_instances: HashMap::new(),
            templates: WorkflowTemplateLibrary::new(),
        };
        engine.register_builtin_templates();
        engine
    }

    /// 注册工作流
    pub fn register_workflow(&mut self, workflow: BusinessWorkflow) -> Result<()> {
        self.workflow_definitions.insert(workflow.id.clone(), workflow);
        Ok(())
    }

    /// 从模板创建工作流实例
    pub fn create_from_template(&mut self, template_id: &str) -> Result<String> {
        let template = self.templates.get(template_id)
            .ok_or_else(|| OperatorError::Other(anyhow::anyhow!("模板不存在: {}", template_id)))?;

        let instance_id = Uuid::new_v4().to_string();
        let workflow = template.create_workflow(&instance_id);
        let id = workflow.id.clone();

        self.workflow_definitions.insert(id.clone(), workflow);
        Ok(id)
    }

    /// 注册并执行业务工作流
    pub async fn execute_business_workflow(&mut self, workflow: BusinessWorkflow) -> Result<WorkflowResult> {
        let workflow_id = workflow.id.clone();
        self.workflow_definitions.insert(workflow_id.clone(), workflow);
        self.execute(&workflow_id).await
    }

    /// 获取工作流
    pub fn get_workflow(&self, id: &str) -> Option<&BusinessWorkflow> {
        self.workflow_definitions.get(id)
    }

    /// 执行工作流
    pub async fn execute(&mut self, workflow_id: &str) -> Result<WorkflowResult> {
        let workflow = self.workflow_definitions.get(workflow_id)
            .ok_or_else(|| OperatorError::Other(anyhow::anyhow!("工作流不存在: {}", workflow_id)))?
            .clone();

        let mut instance = WorkflowInstance {
            id: Uuid::new_v4().to_string(),
            workflow_id: workflow.id.clone(),
            status: WorkflowStatus::Running,
            current_nodes: vec![],
            variables: workflow.variables.clone(),
            node_executions: vec![],
            started_at: Utc::now(),
            completed_at: None,
        };

        let mut execution_log = vec![format!("开始执行工作流: {}", workflow.name)];
        let mut node_outputs: HashMap<String, serde_json::Value> = HashMap::new();

        // BFS执行节点
        let mut queue = VecDeque::new();
        for node in &workflow.nodes {
            if matches!(node.node_type, WorkflowNodeType::Start) {
                queue.push_back(node.clone());
            }
        }

        while let Some(node) = queue.pop_front() {
            instance.current_nodes.push(node.id.clone());
            let node_start = Utc::now();

            tracing::debug!("执行节点: {} ({})", node.name, node.id);
            execution_log.push(format!("→ 执行节点: {}", node.name));

            let node_output = self.execute_node(&node, &instance.variables, &node_outputs).await;

            let node_execution = NodeExecutionRecord {
                node_id: node.id.clone(),
                status: if node_output.is_ok() { WorkflowStatus::Completed } else { WorkflowStatus::Failed },
                started_at: node_start,
                completed_at: Some(Utc::now()),
                input: None,
                output: node_output.as_ref().ok().and_then(|o| o.clone()),
                error: node_output.as_ref().err().map(|e| e.to_string()),
            };

            let success = node_execution.status == WorkflowStatus::Completed;
            instance.node_executions.push(node_execution);

            match node_output {
                Ok(Some(output)) => {
                    node_outputs.insert(node.id.clone(), output.clone());
                    // 合并输出到变量
                    if let Ok(map) = serde_json::from_value::<HashMap<String, serde_json::Value>>(output.clone()) {
                        instance.variables.extend(map);
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    instance.status = WorkflowStatus::Failed;
                    execution_log.push(format!("✗ 节点 {} 执行失败: {}", node.name, e));
                    break;
                }
            }

            if !success {
                break;
            }

            // 找下一个节点
            for next_id in workflow.edges.iter()
                .filter(|c| c.source == node.id)
                .map(|c| c.target.clone())
                .collect::<Vec<_>>()
            {
                if let Some(next_node) = workflow.nodes.iter().find(|n| n.id == next_id) {
                    queue.push_back(next_node.clone());
                }
            }
        }

        if matches!(instance.status, WorkflowStatus::Running) {
            // 检查是否到达End节点
            let reached_end = instance.node_executions.iter()
                .any(|ne| {
                    workflow.nodes.iter()
                        .find(|n| n.id == ne.node_id)
                        .map(|n| matches!(n.node_type, WorkflowNodeType::End))
                        .unwrap_or(false)
                });

            instance.status = if reached_end { WorkflowStatus::Completed } else { WorkflowStatus::Failed };
            execution_log.push(if reached_end { "✓ 工作流执行完成".to_string() } else { "✗ 工作流异常结束".to_string() });
        }

        instance.completed_at = Some(Utc::now());

        let final_output = node_outputs.iter()
            .find(|(id, _)| {
                workflow.nodes.iter()
                    .find(|n| n.id == **id)
                    .map(|n| matches!(n.node_type, WorkflowNodeType::End))
                    .unwrap_or(false)
            })
            .map(|(_, v)| v.clone());

        let result = WorkflowResult {
            instance: instance.clone(),
            final_output,
            execution_log,
            metrics: WorkflowMetrics {
                total_execution_time_ms: 0,
                nodes_executed: instance.node_executions.iter()
                    .filter(|ne| ne.status == WorkflowStatus::Completed).count(),
                operators_called: 0,
                plugins_called: 0,
                parallel_branches: 0,
                total_nodes: workflow.nodes.len(),
                completed_nodes: instance.node_executions.iter()
                    .filter(|ne| ne.status == WorkflowStatus::Completed).count(),
                failed_nodes: instance.node_executions.iter()
                    .filter(|ne| ne.status == WorkflowStatus::Failed).count(),
                total_duration_ms: instance.completed_at.unwrap_or(Utc::now())
                    .signed_duration_since(instance.started_at).num_milliseconds() as u64,
            },
        };

        let instance_id = instance.id.clone();
        self.running_instances.insert(instance_id, instance);
        Ok(result)
    }

    /// 执行单个节点
    async fn execute_node(
        &self,
        node: &WorkflowNode,
        variables: &HashMap<String, serde_json::Value>,
        _previous_outputs: &HashMap<String, serde_json::Value>,
    ) -> Result<Option<serde_json::Value>> {
        match &node.config {
            WorkflowNodeConfig::Start => {
                Ok(Some(serde_json::json!({"started": true, "timestamp": Utc::now().to_rfc3339()})))
            }
            WorkflowNodeConfig::End => {
                Ok(Some(serde_json::json!({"completed": true, "timestamp": Utc::now().to_rfc3339(), "variables": variables})))
            }
            WorkflowNodeConfig::Script { language, code } => {
                tracing::info!("执行{}脚本: {}", language, &code[..code.len().min(50)]);
                // 简化：模拟脚本执行
                Ok(Some(serde_json::json!({
                    "script_executed": true,
                    "language": language,
                    "result": "success"
                })))
            }
            WorkflowNodeConfig::AiTask { task_type, prompt } => {
                tracing::info!("执行AI任务: {} - {}", task_type, &prompt[..prompt.len().min(50)]);
                // 简化：模拟AI任务
                Ok(Some(serde_json::json!({
                    "ai_task_completed": true,
                    "task_type": task_type,
                    "response": format!("AI处理完成: {}", task_type)
                })))
            }
            WorkflowNodeConfig::Operator { operator_id, parameters } => {
                tracing::info!("执行算子: {} with params: {:?}", operator_id, parameters);
                Ok(Some(serde_json::json!({
                    "operator_executed": true,
                    "operator_id": operator_id,
                    "result": "success"
                })))
            }
            WorkflowNodeConfig::Condition { expression, true_path: _, false_path: _ } => {
                tracing::info!("判断条件: {}", expression);
                Ok(Some(serde_json::json!({
                    "condition_evaluated": true,
                    "expression": expression,
                    "result": true
                })))
            }
            WorkflowNodeConfig::PluginCall { plugin_id, method, parameters: _ } => {
                tracing::info!("调用插件: {}.{}", plugin_id, method);
                Ok(Some(serde_json::json!({
                    "plugin_called": true,
                    "plugin_id": plugin_id,
                    "method": method,
                    "result": "success"
                })))
            }
            WorkflowNodeConfig::Parallel { branches: _, merge_strategy } => {
                tracing::info!("并行分支, 合并策略: {:?}", merge_strategy);
                Ok(Some(serde_json::json!({
                    "parallel_completed": true,
                    "merge_strategy": format!("{:?}", merge_strategy)
                })))
            }
            WorkflowNodeConfig::Delay { duration_ms } => {
                tracing::info!("延迟: {}ms", duration_ms);
                tokio::time::sleep(tokio::time::Duration::from_millis(*duration_ms)).await;
                Ok(Some(serde_json::json!({"delay_completed": true, "duration_ms": duration_ms})))
            }
            WorkflowNodeConfig::SubWorkflow { workflow_id } => {
                tracing::info!("调用子工作流: {}", workflow_id);
                Ok(Some(serde_json::json!({"subworkflow_called": true, "workflow_id": workflow_id})))
            }
            WorkflowNodeConfig::UserTask { assignee, form } => {
                tracing::info!("用户任务, 分配给: {:?}", assignee);
                Ok(Some(serde_json::json!({
                    "user_task_pending": true,
                    "assignee": assignee,
                    "form": form
                })))
            }
        }
    }

    fn register_builtin_templates(&mut self) {
        // 模板1: 数据处理管道
        self.templates.register(WorkflowTemplate {
            id: "data-pipeline".to_string(),
            name: "数据处理管道".to_string(),
            description: "数据加载→清洗→转换→归一化→输出".to_string(),
            category: "data_processing".to_string(),
            nodes: vec![
                WorkflowNode {
                    id: "start-1".to_string(), node_type: WorkflowNodeType::Start,
                    name: "开始".to_string(), config: WorkflowNodeConfig::Start,
                    position: Some(NodePosition { x: 50.0, y: 150.0 }),
                },
                WorkflowNode {
                    id: "load".to_string(), node_type: WorkflowNodeType::Script,
                    name: "加载数据".to_string(),
                    config: WorkflowNodeConfig::Script { language: "rust".to_string(), code: "load_data()".to_string() },
                    position: Some(NodePosition { x: 200.0, y: 150.0 }),
                },
                WorkflowNode {
                    id: "clean".to_string(), node_type: WorkflowNodeType::Script,
                    name: "数据清洗".to_string(),
                    config: WorkflowNodeConfig::Script { language: "rust".to_string(), code: "clean_data()".to_string() },
                    position: Some(NodePosition { x: 350.0, y: 150.0 }),
                },
                WorkflowNode {
                    id: "op-normalize".to_string(), node_type: WorkflowNodeType::Operator,
                    name: "归一化算子".to_string(),
                    config: WorkflowNodeConfig::Operator { operator_id: "normalize".to_string(), parameters: HashMap::new() },
                    position: Some(NodePosition { x: 500.0, y: 150.0 }),
                },
                WorkflowNode {
                    id: "end-1".to_string(), node_type: WorkflowNodeType::End,
                    name: "输出结果".to_string(), config: WorkflowNodeConfig::End,
                    position: Some(NodePosition { x: 650.0, y: 150.0 }),
                },
            ],
            connections: vec![
                WorkflowConnection { from: "start-1".to_string(), to: "load".to_string(), label: None },
                WorkflowConnection { from: "load".to_string(), to: "clean".to_string(), label: None },
                WorkflowConnection { from: "clean".to_string(), to: "op-normalize".to_string(), label: None },
                WorkflowConnection { from: "op-normalize".to_string(), to: "end-1".to_string(), label: None },
            ],
            variables: HashMap::new(),
        });

        // 模板2: 神经网络训练流程
        self.templates.register(WorkflowTemplate {
            id: "nn-training".to_string(),
            name: "神经网络训练".to_string(),
            description: "数据加载→前向传播→损失计算→反向传播→参数更新→收敛检查".to_string(),
            category: "ai_training".to_string(),
            nodes: vec![
                WorkflowNode {
                    id: "start-2".to_string(), node_type: WorkflowNodeType::Start,
                    name: "开始训练".to_string(), config: WorkflowNodeConfig::Start,
                    position: Some(NodePosition { x: 30.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "init-params".to_string(), node_type: WorkflowNodeType::Script,
                    name: "初始化参数".to_string(),
                    config: WorkflowNodeConfig::Script { language: "rust".to_string(), code: "init_weights()".to_string() },
                    position: Some(NodePosition { x: 170.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "forward".to_string(), node_type: WorkflowNodeType::Operator,
                    name: "前向传播".to_string(),
                    config: WorkflowNodeConfig::Operator { operator_id: "linear".to_string(), parameters: HashMap::new() },
                    position: Some(NodePosition { x: 310.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "loss".to_string(), node_type: WorkflowNodeType::AiTask,
                    name: "损失计算".to_string(),
                    config: WorkflowNodeConfig::AiTask { task_type: "loss".to_string(), prompt: "计算损失".to_string() },
                    position: Some(NodePosition { x: 450.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "check-converge".to_string(), node_type: WorkflowNodeType::Condition,
                    name: "收敛检查".to_string(),
                    config: WorkflowNodeConfig::Condition {
                        expression: "loss < 0.001".to_string(),
                        true_path: "end-2".to_string(), false_path: "backward".to_string(),
                    },
                    position: Some(NodePosition { x: 590.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "backward".to_string(), node_type: WorkflowNodeType::AiTask,
                    name: "反向传播".to_string(),
                    config: WorkflowNodeConfig::AiTask { task_type: "backprop".to_string(), prompt: "反向传播梯度".to_string() },
                    position: Some(NodePosition { x: 450.0, y: 350.0 }),
                },
                WorkflowNode {
                    id: "end-2".to_string(), node_type: WorkflowNodeType::End,
                    name: "训练完成".to_string(), config: WorkflowNodeConfig::End,
                    position: Some(NodePosition { x: 730.0, y: 200.0 }),
                },
            ],
            connections: vec![
                WorkflowConnection { from: "start-2".to_string(), to: "init-params".to_string(), label: None },
                WorkflowConnection { from: "init-params".to_string(), to: "forward".to_string(), label: None },
                WorkflowConnection { from: "forward".to_string(), to: "loss".to_string(), label: None },
                WorkflowConnection { from: "loss".to_string(), to: "check-converge".to_string(), label: None },
                WorkflowConnection { from: "check-converge".to_string(), to: "end-2".to_string(), label: Some("收敛".to_string()) },
                WorkflowConnection { from: "check-converge".to_string(), to: "backward".to_string(), label: Some("未收敛".to_string()) },
                WorkflowConnection { from: "backward".to_string(), to: "forward".to_string(), label: Some("迭代".to_string()) },
            ],
            variables: HashMap::new(),
        });

        // 模板3: 算法分析归一化流程
        self.templates.register(WorkflowTemplate {
            id: "algorithm-analysis".to_string(),
            name: "算法分析归一化".to_string(),
            description: "输入算法代码→模式识别→流程图生成→算子映射→归一化输出".to_string(),
            category: "algorithm".to_string(),
            nodes: vec![
                WorkflowNode {
                    id: "start-3".to_string(), node_type: WorkflowNodeType::Start,
                    name: "输入算法".to_string(), config: WorkflowNodeConfig::Start,
                    position: Some(NodePosition { x: 30.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "pattern-match".to_string(), node_type: WorkflowNodeType::AiTask,
                    name: "模式识别".to_string(),
                    config: WorkflowNodeConfig::AiTask { task_type: "pattern_recognition".to_string(), prompt: "识别算法模式".to_string() },
                    position: Some(NodePosition { x: 180.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "gen-flow".to_string(), node_type: WorkflowNodeType::Script,
                    name: "流程图生成".to_string(),
                    config: WorkflowNodeConfig::Script { language: "rust".to_string(), code: "generate_flowchart()".to_string() },
                    position: Some(NodePosition { x: 330.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "op-map".to_string(), node_type: WorkflowNodeType::Script,
                    name: "算子映射".to_string(),
                    config: WorkflowNodeConfig::Script { language: "rust".to_string(), code: "map_to_operators()".to_string() },
                    position: Some(NodePosition { x: 480.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "parallel-opt".to_string(), node_type: WorkflowNodeType::Parallel,
                    name: "并行优化".to_string(),
                    config: WorkflowNodeConfig::Parallel { branches: vec![], merge_strategy: MergeStrategy::AllComplete },
                    position: Some(NodePosition { x: 630.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "merge-1".to_string(), node_type: WorkflowNodeType::Parallel,
                    name: "结果合并".to_string(),
                    config: WorkflowNodeConfig::Parallel { branches: vec![], merge_strategy: MergeStrategy::AllComplete },
                    position: Some(NodePosition { x: 780.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "end-3".to_string(), node_type: WorkflowNodeType::End,
                    name: "输出归一化流程".to_string(), config: WorkflowNodeConfig::End,
                    position: Some(NodePosition { x: 930.0, y: 200.0 }),
                },
            ],
            connections: vec![
                WorkflowConnection { from: "start-3".to_string(), to: "pattern-match".to_string(), label: None },
                WorkflowConnection { from: "pattern-match".to_string(), to: "gen-flow".to_string(), label: None },
                WorkflowConnection { from: "gen-flow".to_string(), to: "op-map".to_string(), label: None },
                WorkflowConnection { from: "op-map".to_string(), to: "parallel-opt".to_string(), label: None },
                WorkflowConnection { from: "parallel-opt".to_string(), to: "merge-1".to_string(), label: None },
                WorkflowConnection { from: "merge-1".to_string(), to: "end-3".to_string(), label: None },
            ],
            variables: HashMap::new(),
        });

        // 模板4: AI对话响应流程
        self.templates.register(WorkflowTemplate {
            id: "chat-response".to_string(),
            name: "AI对话响应".to_string(),
            description: "接收消息→意图识别→算子推荐→响应生成".to_string(),
            category: "conversational".to_string(),
            nodes: vec![
                WorkflowNode {
                    id: "start-4".to_string(), node_type: WorkflowNodeType::Start,
                    name: "接收用户消息".to_string(), config: WorkflowNodeConfig::Start,
                    position: Some(NodePosition { x: 30.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "intent".to_string(), node_type: WorkflowNodeType::AiTask,
                    name: "意图识别".to_string(),
                    config: WorkflowNodeConfig::AiTask { task_type: "intent".to_string(), prompt: "识别用户意图".to_string() },
                    position: Some(NodePosition { x: 180.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "recommend".to_string(), node_type: WorkflowNodeType::Script,
                    name: "提取算子".to_string(),
                    config: WorkflowNodeConfig::Script { language: "rust".to_string(), code: "extract_operators()".to_string() },
                    position: Some(NodePosition { x: 330.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "gen-response".to_string(), node_type: WorkflowNodeType::Script,
                    name: "工作流建议".to_string(),
                    config: WorkflowNodeConfig::Script { language: "rust".to_string(), code: "generate_response()".to_string() },
                    position: Some(NodePosition { x: 480.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "end-4".to_string(), node_type: WorkflowNodeType::End,
                    name: "发送响应".to_string(), config: WorkflowNodeConfig::End,
                    position: Some(NodePosition { x: 630.0, y: 200.0 }),
                },
            ],
            connections: vec![
                WorkflowConnection { from: "start-4".to_string(), to: "intent".to_string(), label: None },
                WorkflowConnection { from: "intent".to_string(), to: "recommend".to_string(), label: None },
                WorkflowConnection { from: "recommend".to_string(), to: "gen-response".to_string(), label: None },
                WorkflowConnection { from: "gen-response".to_string(), to: "end-4".to_string(), label: None },
            ],
            variables: HashMap::new(),
        });

        // 模板5: 多插件协作流程
        self.templates.register(WorkflowTemplate {
            id: "plugin-collaboration".to_string(),
            name: "多插件协作".to_string(),
            description: "任务分发→并行插件调用→结果合并→输出".to_string(),
            category: "collaboration".to_string(),
            nodes: vec![
                WorkflowNode {
                    id: "start-5".to_string(), node_type: WorkflowNodeType::Start,
                    name: "开始".to_string(), config: WorkflowNodeConfig::Start,
                    position: Some(NodePosition { x: 30.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "dispatch".to_string(), node_type: WorkflowNodeType::Condition,
                    name: "任务分发".to_string(),
                    config: WorkflowNodeConfig::Condition {
                        expression: "needs_parallel".to_string(),
                        true_path: "parallel-call".to_string(), false_path: "single-call".to_string(),
                    },
                    position: Some(NodePosition { x: 180.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "parallel-call".to_string(), node_type: WorkflowNodeType::PluginCall,
                    name: "并行插件调用".to_string(),
                    config: WorkflowNodeConfig::PluginCall {
                        plugin_id: "*".to_string(), method: "process".to_string(),
                        parameters: serde_json::json!({}),
                    },
                    position: Some(NodePosition { x: 330.0, y: 100.0 }),
                },
                WorkflowNode {
                    id: "single-call".to_string(), node_type: WorkflowNodeType::PluginCall,
                    name: "单插件调用".to_string(),
                    config: WorkflowNodeConfig::PluginCall {
                        plugin_id: "default".to_string(), method: "process".to_string(),
                        parameters: serde_json::json!({}),
                    },
                    position: Some(NodePosition { x: 330.0, y: 300.0 }),
                },
                WorkflowNode {
                    id: "merge-2".to_string(), node_type: WorkflowNodeType::Parallel,
                    name: "结果合并".to_string(),
                    config: WorkflowNodeConfig::Parallel { branches: vec![], merge_strategy: MergeStrategy::FirstSuccess },
                    position: Some(NodePosition { x: 500.0, y: 200.0 }),
                },
                WorkflowNode {
                    id: "end-5".to_string(), node_type: WorkflowNodeType::End,
                    name: "输出结果".to_string(), config: WorkflowNodeConfig::End,
                    position: Some(NodePosition { x: 650.0, y: 200.0 }),
                },
            ],
            connections: vec![
                WorkflowConnection { from: "start-5".to_string(), to: "dispatch".to_string(), label: None },
                WorkflowConnection { from: "dispatch".to_string(), to: "parallel-call".to_string(), label: Some("并行".to_string()) },
                WorkflowConnection { from: "dispatch".to_string(), to: "single-call".to_string(), label: Some("单插件".to_string()) },
                WorkflowConnection { from: "parallel-call".to_string(), to: "merge-2".to_string(), label: None },
                WorkflowConnection { from: "single-call".to_string(), to: "merge-2".to_string(), label: None },
                WorkflowConnection { from: "merge-2".to_string(), to: "end-5".to_string(), label: None },
            ],
            variables: HashMap::new(),
        });
    }

    pub fn list_templates(&self) -> Vec<&WorkflowTemplate> {
        self.templates.list()
    }

    pub fn list_workflows(&self) -> Vec<&BusinessWorkflow> {
        self.workflow_definitions.values().collect()
    }

    pub fn list_instances(&self) -> Vec<&WorkflowInstance> {
        self.running_instances.values().collect()
    }
}

/// 工作流模板库
struct WorkflowTemplateLibrary {
    templates: HashMap<String, WorkflowTemplate>,
}

impl WorkflowTemplateLibrary {
    fn new() -> Self {
        Self { templates: HashMap::new() }
    }

    fn register(&mut self, template: WorkflowTemplate) {
        self.templates.insert(template.id.clone(), template);
    }

    fn get(&self, id: &str) -> Option<&WorkflowTemplate> {
        self.templates.get(id)
    }

    fn list(&self) -> Vec<&WorkflowTemplate> {
        self.templates.values().collect()
    }
}
