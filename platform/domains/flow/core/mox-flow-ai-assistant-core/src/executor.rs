// Copyright (c) 2026 璇玑 RelGraph · AI对话mox 模块化系统架构自动化核心 (AI Assistant Core)
// Licensed under the MIT License.

//! 任务执行器
//!
//! 编排多智能体协同执行任务，负责：
//! - 任务调度
//! - 子任务分配
//! - 执行进度跟踪
//! - 结果汇总

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::AgentRegistry;
use crate::error::{AiError, AiResult};
use crate::intent::IntentRecognizer;
use crate::task_decomposer::TaskDecomposer;
use crate::tool_registry::ToolRegistry;
use crate::types::*;

/// 任务执行器
pub struct TaskExecutor {
    /// 任务表
    tasks: RwLock<HashMap<String, Task>>,
    /// 意图识别器
    intent_recognizer: IntentRecognizer,
    /// 任务分解器
    task_decomposer: TaskDecomposer,
    /// 智能体注册表
    agent_registry: Arc<AgentRegistry>,
    /// 工具注册表
    tool_registry: Arc<ToolRegistry>,
    /// 已完成的任务数
    completed_count: std::sync::atomic::AtomicU64,
}

impl TaskExecutor {
    /// 创建任务执行器
    pub fn new(
        intent_recognizer: IntentRecognizer,
        task_decomposer: TaskDecomposer,
        agent_registry: Arc<AgentRegistry>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            intent_recognizer,
            task_decomposer,
            agent_registry,
            tool_registry,
            completed_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 从用户输入创建并执行任务
    pub fn process_user_input(&self, user_input: &str, conversation_id: Option<&str>) -> AiResult<Task> {
        // 1. 意图识别
        let intent_match = self.intent_recognizer.top_intent(user_input);

        // 2. 创建任务
        let mut task = Task::new(user_input, intent_match.intent);
        task.description = user_input.to_string();
        task.conversation_id = conversation_id.map(|s| s.to_string());

        // 提取参数
        for (k, v) in &intent_match.entities {
            task.params
                .insert(k.clone(), serde_json::Value::String(v.clone()));
        }

        // 3. 任务分解
        self.task_decomposer.decompose(&mut task)?;

        // 4. 分配子任务给智能体
        self.assign_subtasks(&mut task)?;

        // 5. 执行任务（模拟）
        self.execute_task(&mut task)?;

        // 保存任务
        self.tasks
            .write()
            .insert(task.id.clone(), task.clone());

        Ok(task)
    }

    /// 分配子任务给合适的智能体
    fn assign_subtasks(&self, task: &mut Task) -> AiResult<()> {
        for subtask in &mut task.subtasks {
            let agents = self.agent_registry.get_by_role(subtask.agent_role);
            if let Some(agent) = agents.first() {
                subtask.assigned_agent = Some(agent.id.clone());
            }
        }
        Ok(())
    }

    /// 执行任务（模拟执行）
    fn execute_task(&self, task: &mut Task) -> AiResult<()> {
        task.status = TaskStatus::Running;

        // 按顺序执行子任务（使用索引迭代避免 borrow 冲突）
        let subtask_count = task.subtasks.len();
        for i in 0..subtask_count {
            // 检查依赖（先从只读副本获取依赖状态）
            let deps = task.subtasks[i].dependencies.clone();
            for dep_id in &deps {
                let dep = task
                    .subtasks
                    .iter()
                    .find(|s| s.id == *dep_id)
                    .ok_or_else(|| {
                        AiError::InternalError(format!(
                            "dependency '{}' not found",
                            dep_id
                        ))
                    })?;
                if dep.status != TaskStatus::Completed {
                    return Err(AiError::TaskFailed(format!(
                        "dependency '{}' not completed",
                        dep_id
                    )));
                }
            }

            let subtask = &mut task.subtasks[i];
            subtask.status = TaskStatus::Running;

            // 模拟执行
            if subtask.assigned_agent.is_some() {
                subtask.result = Some(serde_json::json!({
                    "status": "completed",
                    "output": format!("{} 执行完成", subtask.name),
                }));
                subtask.status = TaskStatus::Completed;
            } else {
                subtask.status = TaskStatus::Completed;
                subtask.result = Some(serde_json::json!({
                    "status": "completed",
                }));
            }
        }

        // 所有子任务完成后，任务完成
        if task
            .subtasks
            .iter()
            .all(|s| s.status == TaskStatus::Completed)
        {
            task.status = TaskStatus::Completed;
            task.completed_at = Some(now_ms());
            task.result = Some(serde_json::json!({
                "status": "success",
                "subtasks_completed": task.subtasks.len(),
                "summary": "所有子任务已完成",
            }));
            self.completed_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        Ok(())
    }

    /// 获取任务
    pub fn get_task(&self, task_id: &str) -> Option<Task> {
        self.tasks.read().get(task_id).cloned()
    }

    /// 获取所有任务
    pub fn list_tasks(&self) -> Vec<Task> {
        let mut tasks: Vec<Task> = self.tasks.read().values().cloned().collect();
        tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        tasks
    }

    /// 按状态获取任务
    pub fn list_by_status(&self, status: TaskStatus) -> Vec<Task> {
        self.tasks
            .read()
            .values()
            .filter(|t| t.status == status)
            .cloned()
            .collect()
    }

    /// 获取任务进度
    pub fn get_progress(&self, task_id: &str) -> AiResult<f64> {
        let task = self
            .tasks
            .read()
            .get(task_id)
            .cloned()
            .ok_or_else(|| AiError::NotFound(format!("task '{}' not found", task_id)))?;
        Ok(task.progress())
    }

    /// 任务总数
    pub fn task_count(&self) -> usize {
        self.tasks.read().len()
    }

    /// 已完成任务数
    pub fn completed_count(&self) -> u64 {
        self.completed_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

fn now_ms() -> u64 {
    crate::types::now_ms()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_executor() -> TaskExecutor {
        let tool_registry = Arc::new(ToolRegistry::new());
        let agent_registry = Arc::new(AgentRegistry::new(tool_registry.clone()));
        let intent_recognizer = IntentRecognizer::new();
        let task_decomposer = TaskDecomposer::new();

        TaskExecutor::new(
            intent_recognizer,
            task_decomposer,
            agent_registry,
            tool_registry,
        )
    }

    #[test]
    fn test_process_graph_query() {
        let executor = create_executor();
        let task = executor
            .process_user_input("帮我查询知识图谱中的关系", None)
            .unwrap();

        assert_eq!(task.intent, IntentType::GraphQuery);
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(!task.subtasks.is_empty());
        assert!(task.result.is_some());
        assert_eq!(executor.completed_count(), 1);
    }

    #[test]
    fn test_process_data_analysis() {
        let executor = create_executor();
        let task = executor
            .process_user_input("分析一下数据趋势", None)
            .unwrap();

        assert_eq!(task.intent, IntentType::DataAnalysis);
        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.subtasks.len(), 5);
    }

    #[test]
    fn test_subtasks_assigned() {
        let executor = create_executor();
        let task = executor
            .process_user_input("运行图谱算法", None)
            .unwrap();

        for subtask in &task.subtasks {
            assert!(subtask.assigned_agent.is_some());
        }
    }

    #[test]
    fn test_get_task() {
        let executor = create_executor();
        let task = executor
            .process_user_input("测试查询", None)
            .unwrap();

        let retrieved = executor.get_task(&task.id).unwrap();
        assert_eq!(retrieved.id, task.id);
    }

    #[test]
    fn test_list_tasks() {
        let executor = create_executor();
        executor.process_user_input("任务1", None).unwrap();
        executor.process_user_input("任务2", None).unwrap();

        assert_eq!(executor.task_count(), 2);
        let tasks = executor.list_tasks();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_list_by_status() {
        let executor = create_executor();
        executor.process_user_input("查询图谱", None).unwrap();

        let completed = executor.list_by_status(TaskStatus::Completed);
        assert!(!completed.is_empty());
    }

    #[test]
    fn test_get_progress() {
        let executor = create_executor();
        let task = executor
            .process_user_input("分析数据", None)
            .unwrap();

        let progress = executor.get_progress(&task.id).unwrap();
        assert_eq!(progress, 1.0); // 全部完成
    }

    #[test]
    fn test_with_conversation_id() {
        let executor = create_executor();
        let task = executor
            .process_user_input("查询知识", Some("conv-123"))
            .unwrap();

        assert_eq!(task.conversation_id.as_deref(), Some("conv-123"));
    }
}
