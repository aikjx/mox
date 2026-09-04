// Copyright (c) 2026 璇玑 RelGraph · AI对话mox 模块化系统架构自动化核心 (AI Assistant Core)
// Licensed under the MIT License.

//! 任务分解器
//!
//! 将复杂任务拆解为有序的子任务序列，支持：
//! - 基于意图类型的分解模板
//! - 依赖关系管理
//! - 智能体角色分配
//! - 分解策略选择

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{AiError, AiResult};
use crate::types::*;

/// 子任务模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskTemplate {
    /// 子任务名称
    pub name: String,
    /// 子任务描述
    pub description: String,
    /// 分配的智能体角色
    pub agent_role: AgentRole,
    /// 顺序号
    pub order: u32,
    /// 依赖的子任务索引（基于0的索引）
    pub depends_on: Vec<usize>,
}

/// 分解模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionTemplate {
    /// 对应的意图类型
    pub intent: IntentType,
    /// 模板名称
    pub name: String,
    /// 子任务模板列表
    pub subtasks: Vec<SubTaskTemplate>,
    /// 模板描述
    pub description: String,
    /// 是否启用
    pub enabled: bool,
}

/// 任务分解器
pub struct TaskDecomposer {
    /// 分解模板：intent -> Vec<DecompositionTemplate>
    templates: RwLock<HashMap<IntentType, Vec<DecompositionTemplate>>>,
    /// 总分解次数
    total_decompositions: std::sync::atomic::AtomicU64,
}

impl TaskDecomposer {
    /// 创建任务分解器（内置默认模板）
    pub fn new() -> Self {
        let decomposer = Self {
            templates: RwLock::new(HashMap::new()),
            total_decompositions: std::sync::atomic::AtomicU64::new(0),
        };
        decomposer.register_default_templates();
        decomposer
    }

    /// 注册默认分解模板
    fn register_default_templates(&self) {
        // 知识图谱查询任务分解
        let graph_query = DecompositionTemplate {
            intent: IntentType::GraphQuery,
            name: "图谱查询标准流程".to_string(),
            description: "图谱查询的标准分解流程".to_string(),
            enabled: true,
            subtasks: vec![
                SubTaskTemplate {
                    name: "需求理解与实体识别".to_string(),
                    description: "分析用户查询，识别涉及的实体类型和关系".to_string(),
                    agent_role: AgentRole::Coordinator,
                    order: 1,
                    depends_on: Vec::new(),
                },
                SubTaskTemplate {
                    name: "图谱检索".to_string(),
                    description: "在知识图谱中执行检索查询".to_string(),
                    agent_role: AgentRole::GraphExpert,
                    order: 2,
                    depends_on: vec![0],
                },
                SubTaskTemplate {
                    name: "结果分析与整理".to_string(),
                    description: "分析检索结果，整理成可读格式".to_string(),
                    agent_role: AgentRole::DataAnalyst,
                    order: 3,
                    depends_on: vec![1],
                },
                SubTaskTemplate {
                    name: "生成回答".to_string(),
                    description: "生成最终的自然语言回答".to_string(),
                    agent_role: AgentRole::GeneralAssistant,
                    order: 4,
                    depends_on: vec![2],
                },
            ],
        };
        self.register_template(graph_query).unwrap();

        // 数据分析任务分解
        let data_analysis = DecompositionTemplate {
            intent: IntentType::DataAnalysis,
            name: "数据分析标准流程".to_string(),
            description: "数据分析的标准分解流程".to_string(),
            enabled: true,
            subtasks: vec![
                SubTaskTemplate {
                    name: "明确分析目标".to_string(),
                    description: "理解分析需求，确定分析维度".to_string(),
                    agent_role: AgentRole::Coordinator,
                    order: 1,
                    depends_on: Vec::new(),
                },
                SubTaskTemplate {
                    name: "数据采集".to_string(),
                    description: "从各数据源采集需要的数据".to_string(),
                    agent_role: AgentRole::DataAnalyst,
                    order: 2,
                    depends_on: vec![0],
                },
                SubTaskTemplate {
                    name: "数据清洗与处理".to_string(),
                    description: "清洗数据，处理缺失值和异常值".to_string(),
                    agent_role: AgentRole::DataAnalyst,
                    order: 3,
                    depends_on: vec![1],
                },
                SubTaskTemplate {
                    name: "统计分析".to_string(),
                    description: "执行统计分析，生成分析结论".to_string(),
                    agent_role: AgentRole::DataAnalyst,
                    order: 4,
                    depends_on: vec![2],
                },
                SubTaskTemplate {
                    name: "可视化与报告".to_string(),
                    description: "生成可视化图表和分析报告".to_string(),
                    agent_role: AgentRole::GeneralAssistant,
                    order: 5,
                    depends_on: vec![3],
                },
            ],
        };
        self.register_template(data_analysis).unwrap();

        // 算法执行任务分解
        let algo_run = DecompositionTemplate {
            intent: IntentType::AlgorithmRun,
            name: "算法执行标准流程".to_string(),
            description: "算法执行的标准分解流程".to_string(),
            enabled: true,
            subtasks: vec![
                SubTaskTemplate {
                    name: "算法选择".to_string(),
                    description: "根据需求选择合适的算法和参数".to_string(),
                    agent_role: AgentRole::AlgorithmEngineer,
                    order: 1,
                    depends_on: Vec::new(),
                },
                SubTaskTemplate {
                    name: "数据准备".to_string(),
                    description: "准备算法需要的输入数据".to_string(),
                    agent_role: AgentRole::DataAnalyst,
                    order: 2,
                    depends_on: vec![0],
                },
                SubTaskTemplate {
                    name: "算法执行".to_string(),
                    description: "调用算法联盟执行算法".to_string(),
                    agent_role: AgentRole::AlgorithmEngineer,
                    order: 3,
                    depends_on: vec![1],
                },
                SubTaskTemplate {
                    name: "结果评估".to_string(),
                    description: "评估算法执行结果的质量".to_string(),
                    agent_role: AgentRole::AlgorithmEngineer,
                    order: 4,
                    depends_on: vec![2],
                },
            ],
        };
        self.register_template(algo_run).unwrap();

        // 报表生成任务分解
        let report = DecompositionTemplate {
            intent: IntentType::ReportGenerate,
            name: "报表生成标准流程".to_string(),
            description: "报表生成的标准分解流程".to_string(),
            enabled: true,
            subtasks: vec![
                SubTaskTemplate {
                    name: "确定报表内容".to_string(),
                    description: "明确报表需要包含的内容和格式".to_string(),
                    agent_role: AgentRole::Coordinator,
                    order: 1,
                    depends_on: Vec::new(),
                },
                SubTaskTemplate {
                    name: "数据收集".to_string(),
                    description: "收集报表所需的各类数据".to_string(),
                    agent_role: AgentRole::DataAnalyst,
                    order: 2,
                    depends_on: vec![0],
                },
                SubTaskTemplate {
                    name: "报表生成".to_string(),
                    description: "生成最终报表文件".to_string(),
                    agent_role: AgentRole::GeneralAssistant,
                    order: 3,
                    depends_on: vec![1],
                },
            ],
        };
        self.register_template(report).unwrap();

        // 通用/未知意图模板
        let unknown = DecompositionTemplate {
            intent: IntentType::Unknown,
            name: "通用处理流程".to_string(),
            description: "未知意图的通用处理流程".to_string(),
            enabled: true,
            subtasks: vec![
                SubTaskTemplate {
                    name: "需求理解".to_string(),
                    description: "理解用户需求，明确任务目标".to_string(),
                    agent_role: AgentRole::Coordinator,
                    order: 1,
                    depends_on: Vec::new(),
                },
                SubTaskTemplate {
                    name: "生成回答".to_string(),
                    description: "根据理解生成回复".to_string(),
                    agent_role: AgentRole::GeneralAssistant,
                    order: 2,
                    depends_on: vec![0],
                },
            ],
        };
        self.register_template(unknown).unwrap();
    }

    /// 注册分解模板
    pub fn register_template(&self, template: DecompositionTemplate) -> AiResult<()> {
        self.templates
            .write()
            .entry(template.intent)
            .or_default()
            .push(template);
        Ok(())
    }

    /// 分解任务
    pub fn decompose(&self, task: &mut Task) -> AiResult<Vec<SubTask>> {
        let templates = self
            .templates
            .read()
            .get(&task.intent)
            .cloned()
            .unwrap_or_default();

        let template = templates
            .iter()
            .find(|t| t.enabled)
            .ok_or_else(|| {
                AiError::NotFound(format!(
                    "no decomposition template for intent: {:?}",
                    task.intent
                ))
            })?;

        let mut subtasks: Vec<SubTask> = Vec::new();

        for (_idx, st_template) in template.subtasks.iter().enumerate() {
            let mut subtask = SubTask::new(
                &st_template.name,
                st_template.agent_role,
                st_template.order,
            );
            subtask.description = st_template.description.clone();

            // 建立依赖关系（通过子任务ID）
            for dep_idx in &st_template.depends_on {
                if *dep_idx < subtasks.len() {
                    subtask.dependencies.push(subtasks[*dep_idx].id.clone());
                }
            }

            subtasks.push(subtask);
        }

        // 更新任务
        task.subtasks = subtasks.clone();
        task.status = TaskStatus::Decomposed;

        self.total_decompositions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(subtasks)
    }

    /// 获取某意图的所有模板
    pub fn get_templates(&self, intent: IntentType) -> Vec<DecompositionTemplate> {
        self.templates
            .read()
            .get(&intent)
            .cloned()
            .unwrap_or_default()
    }

    /// 获取总分解次数
    pub fn total_decompositions(&self) -> u64 {
        self.total_decompositions
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 模板总数
    pub fn template_count(&self) -> usize {
        self.templates.read().values().map(|v| v.len()).sum()
    }
}

impl Default for TaskDecomposer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_templates() {
        let decomposer = TaskDecomposer::new();
        assert!(decomposer.template_count() >= 4);
    }

    #[test]
    fn test_decompose_graph_query() {
        let decomposer = TaskDecomposer::new();
        let mut task = Task::new("图谱查询", IntentType::GraphQuery);

        let subtasks = decomposer.decompose(&mut task).unwrap();
        assert_eq!(subtasks.len(), 4);
        assert_eq!(task.status, TaskStatus::Decomposed);
        assert_eq!(task.subtasks.len(), 4);

        // 检查顺序
        assert_eq!(subtasks[0].order, 1);
        assert_eq!(subtasks[1].order, 2);
        assert_eq!(subtasks[3].order, 4);

        // 检查角色分配
        assert_eq!(subtasks[0].agent_role, AgentRole::Coordinator);
        assert_eq!(subtasks[1].agent_role, AgentRole::GraphExpert);
    }

    #[test]
    fn test_decompose_data_analysis() {
        let decomposer = TaskDecomposer::new();
        let mut task = Task::new("数据分析", IntentType::DataAnalysis);

        let subtasks = decomposer.decompose(&mut task).unwrap();
        assert_eq!(subtasks.len(), 5);

        // 检查依赖关系
        // 第3个子任务（数据清洗）应该依赖第2个（数据采集）
        assert!(!subtasks[2].dependencies.is_empty());
    }

    #[test]
    fn test_decompose_algorithm() {
        let decomposer = TaskDecomposer::new();
        let mut task = Task::new("算法执行", IntentType::AlgorithmRun);

        let subtasks = decomposer.decompose(&mut task).unwrap();
        assert_eq!(subtasks.len(), 4);
        assert_eq!(subtasks[0].agent_role, AgentRole::AlgorithmEngineer);
    }

    #[test]
    fn test_task_progress_after_decompose() {
        let decomposer = TaskDecomposer::new();
        let mut task = Task::new("test", IntentType::GraphQuery);
        decomposer.decompose(&mut task).unwrap();

        // 分解后所有子任务都是 Pending，进度应为 0
        assert_eq!(task.progress(), 0.0);
    }

    #[test]
    fn test_total_decompositions() {
        let decomposer = TaskDecomposer::new();
        assert_eq!(decomposer.total_decompositions(), 0);

        let mut task = Task::new("test", IntentType::GraphQuery);
        decomposer.decompose(&mut task).unwrap();

        assert_eq!(decomposer.total_decompositions(), 1);
    }

    #[test]
    fn test_get_templates() {
        let decomposer = TaskDecomposer::new();
        let templates = decomposer.get_templates(IntentType::GraphQuery);
        assert!(!templates.is_empty());
    }
}
