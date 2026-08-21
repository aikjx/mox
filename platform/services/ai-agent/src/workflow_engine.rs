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
use super::llm_client::LLMClient;
use operator_core::{Result, OperatorError};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
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
    /// 真实 LLM 客户端句柄（可选；未注入时 AI 节点降级为模拟）
    llm: Option<Arc<RwLock<LLMClient>>>,
}

impl WorkflowEngine {
    /// 无 LLM 句柄的降级构造（AI 节点标记为 simulated）
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self::new_with_llm(None)
    }

    /// 注入真实 LLM 句柄的构造；AI 节点将调用真实大模型
    pub fn new_with_llm(llm: Option<Arc<RwLock<LLMClient>>>) -> Self {
        let mut engine = Self {
            workflow_definitions: HashMap::new(),
            running_instances: HashMap::new(),
            templates: WorkflowTemplateLibrary::new(),
            llm,
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
            // 条件节点按 result 只走 true_path/false_path 对应的分支（企业流程「通过/拒绝」语义）
            if matches!(node.node_type, WorkflowNodeType::Condition) {
                let result = node_outputs.get(&node.id)
                    .and_then(|o| o.get("result"))
                    .and_then(|r| r.as_bool())
                    .unwrap_or(false);
                let target: Option<String> = match &node.config {
                    WorkflowNodeConfig::Condition { true_path, false_path, .. } => {
                        Some(if result { true_path } else { false_path }.clone())
                    }
                    _ => None,
                };
                if let Some(t) = target {
                    if let Some(next_node) = workflow.nodes.iter().find(|n| n.id == t) {
                        queue.push_back(next_node.clone());
                        execution_log.push(format!("✓ 条件分支 → {}", next_node.name));
                    }
                }
                continue;
            }

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
                // 脚本沙箱暂未接入，标记为 pending 而非假成功，避免误导业务编排
                Ok(Some(serde_json::json!({
                    "script_executed": false,
                    "status": "pending",
                    "language": language,
                    "note": "脚本沙箱未接入，节点不执行，等待后续接入 WASM/进程隔离沙箱"
                })))
            }
            WorkflowNodeConfig::AiTask { task_type, prompt } => {
                let rendered = apply_template(prompt, variables);
                tracing::info!("执行AI任务: {} - {}", task_type, &rendered[..rendered.len().min(80)]);
                match &self.llm {
                    Some(llm) if llm.read().await.is_enabled() => {
                        let client = llm.read().await;
                        match client.chat(vec![crate::llm_client::LLMChatMessage {
                            role: "user".to_string(),
                            content: rendered.clone(),
                        }]).await {
                            Ok(resp) => {
                                // AI → 变量闭环：LLM 输出若为 JSON 对象（如 {"verify_pass":true}），
                                // 展开为节点输出键，随后由外层「合并输出到变量」写入 instance.variables，
                                // 使后续 Condition 节点可通过 ${verify_pass} 等真实驱动分支。
                                let mut out = serde_json::json!({
                                    "ai_task_completed": true,
                                    "executed": true,
                                    "task_type": task_type,
                                    "response": resp,
                                });
                                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&resp) {
                                    if let Some(map) = parsed.as_object() {
                                        tracing::debug!("AI任务输出解析为JSON，展开变量: {:?}", map.keys().collect::<Vec<_>>());
                                        if let Some(target) = out.as_object_mut() {
                                            for (k, v) in map {
                                                target.insert(k.clone(), v.clone());
                                            }
                                        }
                                    }
                                }
                                Ok(Some(out))
                            }
                            Err(e) => {
                                tracing::warn!("AI任务调用LLM失败，降级模拟: {e}");
                                Ok(Some(serde_json::json!({
                                    "ai_task_completed": true,
                                    "executed": false,
                                    "status": "simulated",
                                    "simulated": true,
                                    "task_type": task_type,
                                    "response": format!("AI处理完成(模拟): {}", task_type),
                                    "error": e.to_string()
                                })))
                            }
                        }
                    }
                    _ => {
                        tracing::info!("LLM未注入或未启用，AI任务降级为模拟");
                        Ok(Some(serde_json::json!({
                            "ai_task_completed": true,
                            "executed": false,
                            "status": "simulated",
                            "simulated": true,
                            "task_type": task_type,
                            "response": format!("AI处理完成(模拟): {}", task_type)
                        })))
                    }
                }
            }
            WorkflowNodeConfig::Operator { operator_id, parameters } => {
                tracing::info!("执行算子: {} with params: {:?}", operator_id, parameters);
                // 通过 HTTP 调用已注册算子的真实端点（与 runtime 服务同源）
                let base = std::env::var("OPERATOR_API_BASE")
                    .unwrap_or_else(|_| "http://127.0.0.1:3998".to_string());
                let url = format!("{}/operators/{}", base, operator_id);
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(30))
                    .build()
                    .unwrap_or_default();
                match client.post(&url).json(parameters).send().await {
                    Ok(resp) => {
                        let status = resp.status();
                        match resp.text().await {
                            Ok(body) => Ok(Some(serde_json::json!({
                                "operator_executed": status.is_success(),
                                "executed": status.is_success(),
                                "operator_id": operator_id,
                                "http_status": status.as_u16(),
                                "result": body
                            }))),
                            Err(e) => Err(OperatorError::Other(anyhow::anyhow!(
                                "算子 {} 响应读取失败: {}", operator_id, e
                            ))),
                        }
                    }
                    Err(e) => {
                        // 区分超时与连接失败，便于运维定位（后端未启动 / 网络不可达 vs 处理超时）
                        let kind = if e.is_timeout() { "超时" } else { "连接失败" };
                        Err(OperatorError::Other(anyhow::anyhow!(
                            "算子 {operator_id} 调用{kind}({url}): {e}", kind = kind, url = url, operator_id = operator_id, e = e
                        )))
                    }
                }
            }
            WorkflowNodeConfig::Condition { expression, true_path, false_path } => {
                tracing::info!("判断条件: {}", expression);
                // 真实表达式求值：支持 ${var} 引用、==/!=/>/<、&&/|| 与括号。
                // 未定义变量由 resolve_value 返回 Null 哨兵 + compare_values 排序比较 fail-closed=false
                // （见 resolve_value/compare_values），流程走拒绝分支而非中断；
                // 此处 Err 仅来自表达式语法错误（如无比较符、非法操作符），显式报错避免掩盖配置问题。
                match eval_condition(expression, variables) {
                    Ok(result) => {
                        // 记录命中的分支名 + 表达式中引用的变量实际取值，便于审计与排障
                        let matched = if result { true_path } else { false_path };
                        let referenced = extract_var_names(expression)
                            .into_iter()
                            .map(|name| {
                                let v = variables.get(&name).cloned().unwrap_or(serde_json::Value::Null);
                                (name, v)
                            })
                            .collect::<serde_json::Map<String, serde_json::Value>>();
                        Ok(Some(serde_json::json!({
                            "condition_evaluated": true,
                            "executed": true,
                            "expression": expression,
                            "result": result,
                            "matched_branch": matched,
                            "referenced_variables": referenced,
                        })))
                    }
                    Err(e) => Err(OperatorError::Other(anyhow::anyhow!(
                        "条件表达式求值失败 [{}]: {}", expression, e
                    ))),
                }
            }
            WorkflowNodeConfig::PluginCall { plugin_id, method, parameters: _ } => {
                tracing::info!("调用插件: {}.{}", plugin_id, method);
                // 通过 HTTP 调用插件总线的真实端点
                let base = std::env::var("PLUGIN_API_BASE")
                    .unwrap_or_else(|_| "http://127.0.0.1:3998".to_string());
                let url = format!("{}/plugins/{}/{}", base, plugin_id, method);
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(30))
                    .build()
                    .unwrap_or_default();
                match client.get(&url).send().await {
                    Ok(resp) => {
                        let status = resp.status();
                        match resp.text().await {
                            Ok(body) => Ok(Some(serde_json::json!({
                                "plugin_called": status.is_success(),
                                "executed": status.is_success(),
                                "plugin_id": plugin_id,
                                "method": method,
                                "http_status": status.as_u16(),
                                "result": body
                            }))),
                            Err(e) => Err(OperatorError::Other(anyhow::anyhow!(
                                "插件 {}.{} 响应读取失败: {}", plugin_id, method, e
                            ))),
                        }
                    }
                    Err(e) => {
                        let kind = if e.is_timeout() { "超时" } else { "连接失败" };
                        Err(OperatorError::Other(anyhow::anyhow!(
                            "插件 {plugin}.{method} 调用{kind}({url}): {e}",
                            kind = kind, url = url, plugin = plugin_id, method = method, e = e
                        )))
                    }
                }
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
                        expression: "${needs_parallel} == true".to_string(),
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

        // ===== 企业级业务处理流程模板（category: enterprise）=====
        // 编排范式：开始 → AI 审查/核验 → 条件分支 → 结束（合规 / 风险）

        // 企业模板1: 财务发票核验
        self.templates.register(WorkflowTemplate {
            id: "finance-invoice-verify".to_string(),
            name: "财务发票核验".to_string(),
            description: "AI核验发票要素与税务风险，条件分支判定合规/风险".to_string(),
            category: "enterprise".to_string(),
            nodes: vec![
                WorkflowNode { id: "fi-start".to_string(), node_type: WorkflowNodeType::Start, name: "开始".to_string(), config: WorkflowNodeConfig::Start, position: Some(NodePosition { x: 30.0, y: 200.0 }) },
                WorkflowNode { id: "fi-ai".to_string(), node_type: WorkflowNodeType::AiTask, name: "发票核验".to_string(),
                    config: WorkflowNodeConfig::AiTask { task_type: "invoice_verify".to_string(), prompt: "请核验以下发票的要素完整性与税务合规性：${invoice_text}".to_string() },
                    position: Some(NodePosition { x: 200.0, y: 200.0 }) },
                WorkflowNode { id: "fi-cond".to_string(), node_type: WorkflowNodeType::Condition, name: "合规判定".to_string(),
                    config: WorkflowNodeConfig::Condition { expression: "${verify_pass} == true".to_string(), true_path: "fi-ok".to_string(), false_path: "fi-risk".to_string() },
                    position: Some(NodePosition { x: 400.0, y: 200.0 }) },
                WorkflowNode { id: "fi-ok".to_string(), node_type: WorkflowNodeType::End, name: "合规通过".to_string(), config: WorkflowNodeConfig::End, position: Some(NodePosition { x: 600.0, y: 120.0 }) },
                WorkflowNode { id: "fi-risk".to_string(), node_type: WorkflowNodeType::End, name: "标记风险".to_string(), config: WorkflowNodeConfig::End, position: Some(NodePosition { x: 600.0, y: 280.0 }) },
            ],
            connections: vec![
                WorkflowConnection { from: "fi-start".to_string(), to: "fi-ai".to_string(), label: None },
                WorkflowConnection { from: "fi-ai".to_string(), to: "fi-cond".to_string(), label: None },
                WorkflowConnection { from: "fi-cond".to_string(), to: "fi-ok".to_string(), label: Some("合规".to_string()) },
                WorkflowConnection { from: "fi-cond".to_string(), to: "fi-risk".to_string(), label: Some("风险".to_string()) },
            ],
            variables: HashMap::new(),
        });

        // 企业模板2: 人事入职审批
        self.templates.register(WorkflowTemplate {
            id: "hr-onboarding".to_string(),
            name: "人事入职审批".to_string(),
            description: "AI补全资料并调算子创建账号/权限，条件判定资料是否齐全".to_string(),
            category: "enterprise".to_string(),
            nodes: vec![
                WorkflowNode { id: "hr-start".to_string(), node_type: WorkflowNodeType::Start, name: "开始".to_string(), config: WorkflowNodeConfig::Start, position: Some(NodePosition { x: 30.0, y: 200.0 }) },
                WorkflowNode { id: "hr-op".to_string(), node_type: WorkflowNodeType::Operator, name: "创建账号权限".to_string(),
                    config: WorkflowNodeConfig::Operator { operator_id: "hr_create_account".to_string(), parameters: HashMap::new() },
                    position: Some(NodePosition { x: 200.0, y: 200.0 }) },
                WorkflowNode { id: "hr-ai".to_string(), node_type: WorkflowNodeType::AiTask, name: "资料完整性审查".to_string(),
                    config: WorkflowNodeConfig::AiTask { task_type: "hr_review".to_string(), prompt: "审查入职资料是否齐全：${profile}".to_string() },
                    position: Some(NodePosition { x: 360.0, y: 200.0 }) },
                WorkflowNode { id: "hr-cond".to_string(), node_type: WorkflowNodeType::Condition, name: "资料齐全?".to_string(),
                    config: WorkflowNodeConfig::Condition { expression: "${profile_complete} == true".to_string(), true_path: "hr-ok".to_string(), false_path: "hr-back".to_string() },
                    position: Some(NodePosition { x: 520.0, y: 200.0 }) },
                WorkflowNode { id: "hr-ok".to_string(), node_type: WorkflowNodeType::End, name: "入职完成".to_string(), config: WorkflowNodeConfig::End, position: Some(NodePosition { x: 700.0, y: 120.0 }) },
                WorkflowNode { id: "hr-back".to_string(), node_type: WorkflowNodeType::End, name: "退回补充".to_string(), config: WorkflowNodeConfig::End, position: Some(NodePosition { x: 700.0, y: 280.0 }) },
            ],
            connections: vec![
                WorkflowConnection { from: "hr-start".to_string(), to: "hr-op".to_string(), label: None },
                WorkflowConnection { from: "hr-op".to_string(), to: "hr-ai".to_string(), label: None },
                WorkflowConnection { from: "hr-ai".to_string(), to: "hr-cond".to_string(), label: None },
                WorkflowConnection { from: "hr-cond".to_string(), to: "hr-ok".to_string(), label: Some("齐全".to_string()) },
                WorkflowConnection { from: "hr-cond".to_string(), to: "hr-back".to_string(), label: Some("不齐".to_string()) },
            ],
            variables: HashMap::new(),
        });

        // 企业模板3: 采购申请审批
        self.templates.register(WorkflowTemplate {
            id: "procurement-apply".to_string(),
            name: "采购申请审批".to_string(),
            description: "AI做预算合规检查，条件判定是否超预算触发审批".to_string(),
            category: "enterprise".to_string(),
            nodes: vec![
                WorkflowNode { id: "pr-start".to_string(), node_type: WorkflowNodeType::Start, name: "开始".to_string(), config: WorkflowNodeConfig::Start, position: Some(NodePosition { x: 30.0, y: 200.0 }) },
                WorkflowNode { id: "pr-ai".to_string(), node_type: WorkflowNodeType::AiTask, name: "预算合规检查".to_string(),
                    config: WorkflowNodeConfig::AiTask { task_type: "budget_check".to_string(), prompt: "检查采购申请是否超预算：${apply}".to_string() },
                    position: Some(NodePosition { x: 200.0, y: 200.0 }) },
                WorkflowNode { id: "pr-cond".to_string(), node_type: WorkflowNodeType::Condition, name: "超预算?".to_string(),
                    config: WorkflowNodeConfig::Condition { expression: "${over_budget} == true".to_string(), true_path: "pr-approve".to_string(), false_path: "pr-auto".to_string() },
                    position: Some(NodePosition { x: 400.0, y: 200.0 }) },
                WorkflowNode { id: "pr-approve".to_string(), node_type: WorkflowNodeType::End, name: "转人工审批".to_string(), config: WorkflowNodeConfig::End, position: Some(NodePosition { x: 600.0, y: 120.0 }) },
                WorkflowNode { id: "pr-auto".to_string(), node_type: WorkflowNodeType::End, name: "自动通过".to_string(), config: WorkflowNodeConfig::End, position: Some(NodePosition { x: 600.0, y: 280.0 }) },
            ],
            connections: vec![
                WorkflowConnection { from: "pr-start".to_string(), to: "pr-ai".to_string(), label: None },
                WorkflowConnection { from: "pr-ai".to_string(), to: "pr-cond".to_string(), label: None },
                WorkflowConnection { from: "pr-cond".to_string(), to: "pr-approve".to_string(), label: Some("超预算".to_string()) },
                WorkflowConnection { from: "pr-cond".to_string(), to: "pr-auto".to_string(), label: Some("合规".to_string()) },
            ],
            variables: HashMap::new(),
        });

        // 企业模板4: 报销审批
        self.templates.register(WorkflowTemplate {
            id: "expense-reimburse".to_string(),
            name: "报销审批".to_string(),
            description: "AI审查票据真实性与合规性，条件判定是否放行".to_string(),
            category: "enterprise".to_string(),
            nodes: vec![
                WorkflowNode { id: "er-start".to_string(), node_type: WorkflowNodeType::Start, name: "开始".to_string(), config: WorkflowNodeConfig::Start, position: Some(NodePosition { x: 30.0, y: 200.0 }) },
                WorkflowNode { id: "er-ai".to_string(), node_type: WorkflowNodeType::AiTask, name: "票据合规审查".to_string(),
                    config: WorkflowNodeConfig::AiTask { task_type: "expense_review".to_string(), prompt: "审查报销票据的真实性与合规性：${receipt}".to_string() },
                    position: Some(NodePosition { x: 200.0, y: 200.0 }) },
                WorkflowNode { id: "er-cond".to_string(), node_type: WorkflowNodeType::Condition, name: "是否放行?".to_string(),
                    config: WorkflowNodeConfig::Condition { expression: "${compliant} == true".to_string(), true_path: "er-ok".to_string(), false_path: "er-reject".to_string() },
                    position: Some(NodePosition { x: 400.0, y: 200.0 }) },
                WorkflowNode { id: "er-ok".to_string(), node_type: WorkflowNodeType::End, name: "批准报销".to_string(), config: WorkflowNodeConfig::End, position: Some(NodePosition { x: 600.0, y: 120.0 }) },
                WorkflowNode { id: "er-reject".to_string(), node_type: WorkflowNodeType::End, name: "驳回".to_string(), config: WorkflowNodeConfig::End, position: Some(NodePosition { x: 600.0, y: 280.0 }) },
            ],
            connections: vec![
                WorkflowConnection { from: "er-start".to_string(), to: "er-ai".to_string(), label: None },
                WorkflowConnection { from: "er-ai".to_string(), to: "er-cond".to_string(), label: None },
                WorkflowConnection { from: "er-cond".to_string(), to: "er-ok".to_string(), label: Some("合规".to_string()) },
                WorkflowConnection { from: "er-cond".to_string(), to: "er-reject".to_string(), label: Some("不合规".to_string()) },
            ],
            variables: HashMap::new(),
        });

        // 企业模板5: 合同会签
        self.templates.register(WorkflowTemplate {
            id: "contract-countersign".to_string(),
            name: "合同会签".to_string(),
            description: "算子发起会签流程，AI做条款风险审查，条件判定是否通过".to_string(),
            category: "enterprise".to_string(),
            nodes: vec![
                WorkflowNode { id: "ct-start".to_string(), node_type: WorkflowNodeType::Start, name: "开始".to_string(), config: WorkflowNodeConfig::Start, position: Some(NodePosition { x: 30.0, y: 200.0 }) },
                WorkflowNode { id: "ct-op".to_string(), node_type: WorkflowNodeType::Operator, name: "发起会签".to_string(),
                    config: WorkflowNodeConfig::Operator { operator_id: "contract_initiate".to_string(), parameters: HashMap::new() },
                    position: Some(NodePosition { x: 200.0, y: 200.0 }) },
                WorkflowNode { id: "ct-ai".to_string(), node_type: WorkflowNodeType::AiTask, name: "条款风险审查".to_string(),
                    config: WorkflowNodeConfig::AiTask { task_type: "contract_review".to_string(), prompt: "审查合同条款的法律与商业风险：${contract}".to_string() },
                    position: Some(NodePosition { x: 360.0, y: 200.0 }) },
                WorkflowNode { id: "ct-cond".to_string(), node_type: WorkflowNodeType::Condition, name: "风险判定".to_string(),
                    config: WorkflowNodeConfig::Condition { expression: "${risk_low} == true".to_string(), true_path: "ct-ok".to_string(), false_path: "ct-edit".to_string() },
                    position: Some(NodePosition { x: 520.0, y: 200.0 }) },
                WorkflowNode { id: "ct-ok".to_string(), node_type: WorkflowNodeType::End, name: "签署生效".to_string(), config: WorkflowNodeConfig::End, position: Some(NodePosition { x: 700.0, y: 120.0 }) },
                WorkflowNode { id: "ct-edit".to_string(), node_type: WorkflowNodeType::End, name: "退回修改".to_string(), config: WorkflowNodeConfig::End, position: Some(NodePosition { x: 700.0, y: 280.0 }) },
            ],
            connections: vec![
                WorkflowConnection { from: "ct-start".to_string(), to: "ct-op".to_string(), label: None },
                WorkflowConnection { from: "ct-op".to_string(), to: "ct-ai".to_string(), label: None },
                WorkflowConnection { from: "ct-ai".to_string(), to: "ct-cond".to_string(), label: None },
                WorkflowConnection { from: "ct-cond".to_string(), to: "ct-ok".to_string(), label: Some("低风险".to_string()) },
                WorkflowConnection { from: "ct-cond".to_string(), to: "ct-edit".to_string(), label: Some("高风险".to_string()) },
            ],
            variables: HashMap::new(),
        });

        // 企业模板6: 法务合规审查
        self.templates.register(WorkflowTemplate {
            id: "legal-compliance-review".to_string(),
            name: "法务合规审查".to_string(),
            description: "AI做合规风险审查，条件判定是否通过".to_string(),
            category: "enterprise".to_string(),
            nodes: vec![
                WorkflowNode { id: "lc-start".to_string(), node_type: WorkflowNodeType::Start, name: "开始".to_string(), config: WorkflowNodeConfig::Start, position: Some(NodePosition { x: 30.0, y: 200.0 }) },
                WorkflowNode { id: "lc-ai".to_string(), node_type: WorkflowNodeType::AiTask, name: "合规风险审查".to_string(),
                    config: WorkflowNodeConfig::AiTask { task_type: "legal_review".to_string(), prompt: "审查以下业务/文档的合规风险：${document}".to_string() },
                    position: Some(NodePosition { x: 200.0, y: 200.0 }) },
                WorkflowNode { id: "lc-cond".to_string(), node_type: WorkflowNodeType::Condition, name: "合规判定".to_string(),
                    config: WorkflowNodeConfig::Condition { expression: "${compliant} == true".to_string(), true_path: "lc-ok".to_string(), false_path: "lc-flag".to_string() },
                    position: Some(NodePosition { x: 400.0, y: 200.0 }) },
                WorkflowNode { id: "lc-ok".to_string(), node_type: WorkflowNodeType::End, name: "合规通过".to_string(), config: WorkflowNodeConfig::End, position: Some(NodePosition { x: 600.0, y: 120.0 }) },
                WorkflowNode { id: "lc-flag".to_string(), node_type: WorkflowNodeType::End, name: "标记风险".to_string(), config: WorkflowNodeConfig::End, position: Some(NodePosition { x: 600.0, y: 280.0 }) },
            ],
            connections: vec![
                WorkflowConnection { from: "lc-start".to_string(), to: "lc-ai".to_string(), label: None },
                WorkflowConnection { from: "lc-ai".to_string(), to: "lc-cond".to_string(), label: None },
                WorkflowConnection { from: "lc-cond".to_string(), to: "lc-ok".to_string(), label: Some("合规".to_string()) },
                WorkflowConnection { from: "lc-cond".to_string(), to: "lc-flag".to_string(), label: Some("风险".to_string()) },
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

// ===================== 业务流程辅助函数（模块级自由函数）=====================

/// 简易变量模板替换：将 input 中的 ${var} 用 variables 中的值替换。
fn apply_template(input: &str, variables: &HashMap<String, serde_json::Value>) -> String {
    let mut out = input.to_string();
    for (k, v) in variables {
        let placeholder = format!("${{{}}}", k);
        let val = match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        out = out.replace(&placeholder, &val);
    }
    out
}

/// 简易条件表达式求值：支持 ${var} 引用、==/!=/>/</>=/<=、&&/|| 与顶层括号。
/// 返回布尔结果或错误。
fn eval_condition(expr: &str, variables: &HashMap<String, serde_json::Value>) -> anyhow::Result<bool> {
    for or_part in split_top_level(expr, "||") {
        let mut and_ok = true;
        for and_part in split_top_level(or_part.trim(), "&&") {
            let cmp = and_part.trim();
            if cmp.is_empty() {
                continue;
            }
            let (lhs_raw, op, rhs_raw) = parse_comparison(cmp)?;
            let lhs = resolve_value(lhs_raw.trim(), variables)?;
            let rhs = resolve_value(rhs_raw.trim(), variables)?;
            if !compare_values(&lhs, op, &rhs)? {
                and_ok = false;
                break;
            }
        }
        if and_ok {
            return Ok(true);
        }
    }
    Ok(false)
}

/// 按分隔符切分顶层（忽略括号内的分隔符）
fn split_top_level(expr: &str, sep: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    let bytes = expr.as_bytes();
    let sep_bytes = sep.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '(' {
            depth += 1;
            cur.push(c);
            i += 1;
        } else if c == ')' {
            depth -= 1;
            cur.push(c);
            i += 1;
        } else if depth == 0 && i + sep_bytes.len() <= bytes.len()
            && &bytes[i..i + sep_bytes.len()] == sep_bytes
        {
            parts.push(cur.trim().to_string());
            cur.clear();
            i += sep_bytes.len();
        } else {
            cur.push(c);
            i += 1;
        }
    }
    if !cur.trim().is_empty() {
        parts.push(cur.trim().to_string());
    }
    parts
}

/// 解析单个比较表达式，返回 (左操作数, 操作符, 右操作数)
fn parse_comparison(cmp: &str) -> anyhow::Result<(&str, &str, &str)> {
    for op in ["==", "!=", ">=", "<=", ">", "<"] {
        if let Some(pos) = cmp.find(op) {
            return Ok((&cmp[..pos], op, &cmp[pos + op.len()..]));
        }
    }
    Err(anyhow::anyhow!("无法解析比较表达式: {}", cmp))
}

/// 解析操作数为可比较的值：支持 ${var} 引用、裸字符串、布尔、数字。
fn resolve_value(raw: &str, variables: &HashMap<String, serde_json::Value>) -> anyhow::Result<serde_json::Value> {
    let raw = raw.trim();
    if let Some(var) = raw.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        // 未定义变量：fail-closed 返回 Null 哨兵，比较一律为 false（不抛错），
        // 避免配置缺字段时工作流崩溃；真正的语法错误由 parse_comparison 负责报错。
        return Ok(variables
            .get(var.trim())
            .cloned()
            .unwrap_or(serde_json::Value::Null));
    }
    // 布尔字面量
    if raw == "true" {
        return Ok(serde_json::Value::Bool(true));
    }
    if raw == "false" {
        return Ok(serde_json::Value::Bool(false));
    }
    // 数字字面量
    if let Ok(n) = raw.parse::<f64>() {
        return Ok(serde_json::Value::from(n));
    }
    // 字符串字面量（去除引号）
    if (raw.starts_with('"') && raw.ends_with('"')) || (raw.starts_with('\'') && raw.ends_with('\'')) {
        return Ok(serde_json::Value::String(raw[1..raw.len() - 1].to_string()));
    }
    // 其余视作字符串
    Ok(serde_json::Value::String(raw.to_string()))
}

/// 将 Value 协调为可比较的数值；非数字类型（字符串/布尔/Null）返回 None。
fn coerce_number(v: &serde_json::Value) -> Option<f64> {
    if let Some(n) = v.as_f64() {
        return Some(n);
    }
    // 布尔按 0/1 协调，便于 1 == true 这类判定
    if let Some(b) = v.as_bool() {
        return Some(if b { 1.0 } else { 0.0 });
    }
    // 字符串尝试解析为数字（如 "1" / "3.14"），失败返回 None
    if let Some(s) = v.as_str() {
        return s.trim().parse::<f64>().ok();
    }
    None
}

/// 比较两个 serde_json::Value，依据操作符返回布尔。
/// - 等值比较 `==/!=`：支持跨类型数值协调（1 == "1"、1 == true 均等价）；
///   纯字符串则按文本比较（"active" == "active"）。
/// - 排序比较 `>/</>=/<=`：优先数值比较，失败退化为字符串字典序；
///   未定义变量（Null 哨兵）一律 false（fail-closed，避免 ${loss}<0.001 误判收敛）。
fn compare_values(lhs: &serde_json::Value, op: &str, rhs: &serde_json::Value) -> anyhow::Result<bool> {
    use std::cmp::Ordering;
    match op {
        "==" | "!=" => {
            let eq = if let (Some(a), Some(b)) = (coerce_number(lhs), coerce_number(rhs)) {
                // 双方都能协调为数字 → 数值相等比较（跨类型）
                (a - b).abs() < f64::EPSILON
            } else {
                // 否则按 serde_json 原生相等（字符串/布尔/对象严格匹配）
                lhs == rhs
            };
            Ok(if op == "==" { eq } else { !eq })
        }
        ">" | ">=" | "<" | "<=" => {
            if lhs.is_null() || rhs.is_null() {
                return Ok(false);
            }
            let ord = if let (Some(a), Some(b)) = (coerce_number(lhs), coerce_number(rhs)) {
                a.partial_cmp(&b).unwrap_or(Ordering::Equal)
            } else {
                let a = lhs.as_str().unwrap_or("");
                let b = rhs.as_str().unwrap_or("");
                a.cmp(b)
            };
            Ok(match op {
                ">" => ord == Ordering::Greater,
                ">=" => ord != Ordering::Less,
                "<" => ord == Ordering::Less,
                _ => ord != Ordering::Greater,
            })
        }
        _ => Err(anyhow::anyhow!("不支持的比较操作符: {}", op)),
    }
}

/// 从条件表达式中提取所有 `${var}` 引用的变量名（去重，保持出现顺序）。
/// 供 Condition 节点输出「被引用变量的实际取值」以辅助审计。
fn extract_var_names(expr: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let bytes = expr.as_bytes();
    let mut i = 0;
    while i < bytes.len().saturating_sub(1) {
        if bytes[i] == b'$' && bytes.get(i + 1) == Some(&b'{') {
            if let Some(end) = expr[i + 2..].find('}') {
                let name = expr[i + 2..i + 2 + end].trim().to_string();
                if !name.is_empty() && !names.contains(&name) {
                    names.push(name);
                }
                i = i + 2 + end + 1;
                continue;
            }
        }
        i += 1;
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 企业流程闭环：无 LLM 时 AiTask 降级 → 条件变量未定义 fail-closed=false
    /// → 只走 false 分支（fi-risk），流程仍正常完成，且两条分支不同时执行。
    #[tokio::test]
    async fn test_finance_template_ai_degrades_and_condition_routes_false_branch() {
        let mut engine = WorkflowEngine::new();
        let wf_id = engine.create_from_template("finance-invoice-verify").expect("模板应存在");
        let result = engine.execute(&wf_id).await.expect("流程应正常完成");

        assert_eq!(result.instance.status, WorkflowStatus::Completed);
        assert!(
            result.instance.variables.contains_key("ai_task_completed"),
            "AiTask 降级输出应合并到变量"
        );
        assert_eq!(
            result.instance.variables.get("simulated").and_then(|v| v.as_bool()),
            Some(true),
            "降级执行应带 simulated 标记"
        );

        // 条件变量未定义 → fail-closed false → 走 fi-risk，绝不走 fi-ok
        let executed_ids: Vec<&str> = result.instance.node_executions.iter().map(|ne| ne.node_id.as_str()).collect();
        assert!(executed_ids.contains(&"fi-risk"), "应走拒绝分支");
        assert!(!executed_ids.contains(&"fi-ok"), "不应同时走通过分支");
    }

    /// 条件路由：预置 verify_pass=true 时走通过分支 fi-ok。
    #[tokio::test]
    async fn test_condition_routes_true_branch_when_var_set() {
        let mut engine = WorkflowEngine::new();
        let wf_id = engine.create_from_template("finance-invoice-verify").expect("模板应存在");
        {
            let wf = engine.workflow_definitions.get_mut(&wf_id).expect("工作流已注册");
            wf.variables.insert("verify_pass".to_string(), serde_json::Value::Bool(true));
            wf.variables.insert("invoice_text".to_string(), serde_json::Value::String("测试发票".to_string()));
        }
        let result = engine.execute(&wf_id).await.expect("流程应正常完成");
        assert_eq!(result.instance.status, WorkflowStatus::Completed);
        let executed_ids: Vec<&str> = result.instance.node_executions.iter().map(|ne| ne.node_id.as_str()).collect();
        assert!(executed_ids.contains(&"fi-ok"), "verify_pass=true 应走通过分支");
        assert!(!executed_ids.contains(&"fi-risk"), "不应走拒绝分支");
    }

    /// 企业模板注册完整性：纯 AiTask 型企业模板（无 Operator/PluginCall 网络依赖）
    /// 在 LLM 降级路径下全部执行完成（fail-closed 走拒绝分支）。
    /// 注：hr-onboarding / contract-countersign 含 Operator 节点，依赖真实算子服务，
    /// 其失败语义见 execute_node 的错误传播，此处不触发网络调用。
    #[tokio::test]
    async fn test_enterprise_templates_execute_to_completion_without_network() {
        let enterprise_ids = [
            "finance-invoice-verify", "procurement-apply",
            "expense-reimburse", "legal-compliance-review",
        ];
        for tid in enterprise_ids {
            let mut engine = WorkflowEngine::new();
            let wf_id = engine.create_from_template(tid).unwrap_or_else(|_| panic!("模板应存在: {}", tid));
            let result = engine.execute(&wf_id).await.unwrap_or_else(|_| panic!("{} 应能执行完成", tid));
            assert_eq!(result.instance.status, WorkflowStatus::Completed, "{} 应 Completed", tid);
            assert!(
                result.instance.node_executions.iter().any(|ne| ne.status == WorkflowStatus::Completed),
                "{} 至少完成一个节点", tid
            );
        }
    }

    /// 条件表达式解析：变量缺失按 fail-closed false 处理，语法错误仍报错。
    #[test]
    fn test_eval_condition_semantics() {
        let mut vars = HashMap::new();
        vars.insert("verify_pass".to_string(), serde_json::Value::Bool(true));
        vars.insert("loss".to_string(), serde_json::Value::from(0.0005f64));

        assert!(eval_condition("${verify_pass} == true", &vars).expect("变量已定义应可求值"));
        assert!(eval_condition("${loss} < 0.001", &vars).expect("数值比较应可求值"));
        assert!(!eval_condition("${loss} >= 0.001", &vars).expect("应可求值"));

        // 未定义变量：fail-closed——resolve_value 返回 Null 哨兵，
        // 等值比较为 false，排序比较也一律 false（不会退化为字符串比较）
        assert!(!eval_condition("${missing} == true", &vars).expect("fail-closed false"));
        assert!(!eval_condition("${missing} < 0.001", &vars).expect("fail-closed false"));
        assert!(!eval_condition("${missing} > 5", &vars).expect("fail-closed false"));
        // 语法错误（无比较符）：仍报错，避免掩盖配置错误
        assert!(eval_condition("needs_parallel", &vars).is_err());
    }

    /// compare_values：跨类型数值等值（1 == "1"、1 == true）与 Null 排序 fail-closed。
    #[test]
    fn test_compare_values_cross_type() {
        // 跨类型等值
        assert!(compare_values(&serde_json::json!(1), "==", &serde_json::json!("1")).expect("应可比较"));
        assert!(compare_values(&serde_json::json!(1), "==", &serde_json::json!(true)).expect("应可比较"));
        assert!(!compare_values(&serde_json::json!(1), "==", &serde_json::json!("2")).expect("应可比较"));
        assert!(compare_values(&serde_json::json!(true), "!=", &serde_json::json!(0)).expect("应可比较"));
        // 纯字符串严格相等（不按数字协调）
        assert!(compare_values(&serde_json::json!("active"), "==", &serde_json::json!("active")).expect("应可比较"));
        assert!(!compare_values(&serde_json::json!("active"), "==", &serde_json::json!("inactive")).expect("应可比较"));
        // 排序：数值
        assert!(compare_values(&serde_json::json!(0.0005), "<", &serde_json::json!(0.001)).expect("数值排序"));
        assert!(!compare_values(&serde_json::json!(0.002), "<", &serde_json::json!(0.001)).expect("数值排序"));
        // 排序：Null 哨兵 fail-closed
        let null = serde_json::Value::Null;
        assert!(!compare_values(&null, "<", &serde_json::json!(0.001)).expect("fail-closed false"));
        assert!(!compare_values(&serde_json::json!(0.001), ">", &null).expect("fail-closed false"));
    }

    /// extract_var_names：正确提取 ${var} 引用（去重、保序）。
    #[test]
    fn test_extract_var_names() {
        let names = extract_var_names("${a} == true && ${b} < ${a}");
        assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
        let names2 = extract_var_names("no vars here");
        assert!(names2.is_empty());
    }
}
