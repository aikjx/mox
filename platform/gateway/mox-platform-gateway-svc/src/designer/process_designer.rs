//! 审批流程设计器模型
//!
//! 支持可视化拖拽设计审批流程，包含节点、连线、条件分支等

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 流程定义（设计器保存的完整流程）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDefinition {
    /// 流程定义 ID
    pub process_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 流程名称
    pub name: String,
    /// 流程编码
    pub code: String,
    /// 流程分类：hr / finance / business / custom
    pub category: String,
    /// 流程描述
    pub description: Option<String>,
    /// 流程版本
    pub version: i32,
    /// 状态：draft / published / disabled
    pub status: String,
    /// 节点列表
    pub nodes: Vec<ProcessNode>,
    /// 连线列表
    pub edges: Vec<ProcessEdge>,
    /// 表单 ID（关联的动态表单）
    pub form_id: Option<String>,
    /// 发起人设置
    pub initiator_config: InitiatorConfig,
    /// 通知设置
    pub notification_config: NotificationConfig,
    /// 创建人
    pub created_by: Option<String>,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
    /// 发布时间
    pub published_at: Option<String>,
}

/// 流程节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessNode {
    /// 节点 ID
    pub node_id: String,
    /// 节点类型：start / end / approval / cc / condition / parallel / sub_process
    pub node_type: String,
    /// 节点名称
    pub name: String,
    /// 节点描述
    pub description: Option<String>,
    /// 节点位置（前端画布坐标）
    pub position: NodePosition,
    /// 节点配置（根据节点类型不同）
    pub config: NodeConfig,
}

/// 节点位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

/// 节点配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeConfig {
    /// 审批人类型：user / role / dept / initiator_manager / expression
    pub approver_type: Option<String>,
    /// 审批人 ID 列表
    pub approver_ids: Option<Vec<String>>,
    /// 审批策略：any / all / sequential / majority
    pub approval_policy: Option<String>,
    /// 多数通过比例（0-1）
    pub majority_ratio: Option<f64>,
    /// 超时时间（小时）
    pub timeout_hours: Option<i32>,
    /// 超时处理：auto_approve / auto_reject / escalate
    pub timeout_action: Option<String>,
    /// 条件表达式（条件节点使用）
    pub condition_expr: Option<String>,
    /// 条件分支（条件节点使用）
    pub conditions: Option<Vec<ConditionBranch>>,
    /// 抄送人 ID 列表
    pub cc_user_ids: Option<Vec<String>>,
    /// 允许加签
    pub allow_add_sign: Option<bool>,
    /// 允许转签
    pub allow_transfer: Option<bool>,
    /// 允许驳回
    pub allow_reject: Option<bool>,
    /// 驳回到：previous / start / 指定节点
    pub reject_to: Option<String>,
}

/// 条件分支
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionBranch {
    /// 分支 ID
    pub branch_id: String,
    /// 分支名称
    pub name: String,
    /// 条件表达式
    pub expression: String,
    /// 优先级（数字越小优先级越高）
    pub priority: i32,
    /// 是否默认分支
    pub is_default: bool,
}

/// 流程连线
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEdge {
    /// 连线 ID
    pub edge_id: String,
    /// 源节点 ID
    pub source_node_id: String,
    /// 目标节点 ID
    pub target_node_id: String,
    /// 连线标签（条件分支时使用）
    pub label: Option<String>,
    /// 条件表达式
    pub condition_expr: Option<String>,
}

/// 发起人设置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InitiatorConfig {
    /// 允许发起的角色 ID 列表
    pub allowed_roles: Option<Vec<String>>,
    /// 允许发起的部门 ID 列表
    pub allowed_depts: Option<Vec<String>>,
    /// 是否允许所有人发起
    pub allow_all: bool,
}

/// 通知设置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationConfig {
    /// 审批待办通知
    pub pending_approval: bool,
    /// 审批通过通知
    pub approved: bool,
    /// 审批驳回通知
    pub rejected: bool,
    /// 流程完成通知
    pub completed: bool,
    /// 通知方式：in_app / email / sms / feishu / dingtalk
    pub channels: Vec<String>,
}

/// 创建流程定义请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProcessDefinitionRequest {
    pub name: String,
    pub code: String,
    pub category: String,
    pub description: Option<String>,
    pub nodes: Vec<ProcessNode>,
    pub edges: Vec<ProcessEdge>,
    pub form_id: Option<String>,
    pub initiator_config: Option<InitiatorConfig>,
    pub notification_config: Option<NotificationConfig>,
}

/// 更新流程定义请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProcessDefinitionRequest {
    pub name: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub nodes: Option<Vec<ProcessNode>>,
    pub edges: Option<Vec<ProcessEdge>>,
    pub form_id: Option<String>,
    pub initiator_config: Option<InitiatorConfig>,
    pub notification_config: Option<NotificationConfig>,
}

/// 流程模板（用于快速创建）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessTemplate {
    pub template_id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub nodes: Vec<ProcessNode>,
    pub edges: Vec<ProcessEdge>,
    pub icon: Option<String>,
}

/// 内置流程模板
pub fn builtin_templates() -> Vec<ProcessTemplate> {
    vec![
        ProcessTemplate {
            template_id: "tpl_hr_leave".to_string(),
            name: "人事请假审批".to_string(),
            category: "hr".to_string(),
            description: "员工请假 → 部门经理审批 → HR备案".to_string(),
            nodes: vec![
                ProcessNode { node_id: "start".to_string(), node_type: "start".to_string(), name: "开始".to_string(), description: None, position: NodePosition { x: 100.0, y: 200.0 }, config: NodeConfig::default() },
                ProcessNode { node_id: "dept_manager".to_string(), node_type: "approval".to_string(), name: "部门经理审批".to_string(), description: None, position: NodePosition { x: 300.0, y: 200.0 }, config: NodeConfig { approver_type: Some("initiator_manager".to_string()), approval_policy: Some("any".to_string()), allow_reject: Some(true), ..Default::default() } },
                ProcessNode { node_id: "hr".to_string(), node_type: "approval".to_string(), name: "HR备案".to_string(), description: None, position: NodePosition { x: 500.0, y: 200.0 }, config: NodeConfig { approver_type: Some("role".to_string()), approver_ids: Some(vec!["role_hr".to_string()]), approval_policy: Some("any".to_string()), ..Default::default() } },
                ProcessNode { node_id: "end".to_string(), node_type: "end".to_string(), name: "结束".to_string(), description: None, position: NodePosition { x: 700.0, y: 200.0 }, config: NodeConfig::default() },
            ],
            edges: vec![
                ProcessEdge { edge_id: "e1".to_string(), source_node_id: "start".to_string(), target_node_id: "dept_manager".to_string(), label: None, condition_expr: None },
                ProcessEdge { edge_id: "e2".to_string(), source_node_id: "dept_manager".to_string(), target_node_id: "hr".to_string(), label: None, condition_expr: None },
                ProcessEdge { edge_id: "e3".to_string(), source_node_id: "hr".to_string(), target_node_id: "end".to_string(), label: None, condition_expr: None },
            ],
            icon: None,
        },
        ProcessTemplate {
            template_id: "tpl_finance_expense".to_string(),
            name: "财务报销审批".to_string(),
            category: "finance".to_string(),
            description: "员工报销 → 部门经理 → 财务审核 → 总经理（金额>1万）".to_string(),
            nodes: vec![
                ProcessNode { node_id: "start".to_string(), node_type: "start".to_string(), name: "开始".to_string(), description: None, position: NodePosition { x: 100.0, y: 200.0 }, config: NodeConfig::default() },
                ProcessNode { node_id: "dept_manager".to_string(), node_type: "approval".to_string(), name: "部门经理审批".to_string(), description: None, position: NodePosition { x: 300.0, y: 200.0 }, config: NodeConfig { approver_type: Some("initiator_manager".to_string()), approval_policy: Some("any".to_string()), ..Default::default() } },
                ProcessNode { node_id: "finance".to_string(), node_type: "approval".to_string(), name: "财务审核".to_string(), description: None, position: NodePosition { x: 500.0, y: 200.0 }, config: NodeConfig { approver_type: Some("role".to_string()), approver_ids: Some(vec!["role_finance".to_string()]), approval_policy: Some("any".to_string()), ..Default::default() } },
                ProcessNode { node_id: "condition".to_string(), node_type: "condition".to_string(), name: "金额判断".to_string(), description: None, position: NodePosition { x: 700.0, y: 200.0 }, config: NodeConfig { conditions: Some(vec![
                    ConditionBranch { branch_id: "b1".to_string(), name: "金额>1万".to_string(), expression: "amount > 10000".to_string(), priority: 1, is_default: false },
                    ConditionBranch { branch_id: "b2".to_string(), name: "默认".to_string(), expression: "true".to_string(), priority: 100, is_default: true },
                ]), ..Default::default() } },
                ProcessNode { node_id: "gm".to_string(), node_type: "approval".to_string(), name: "总经理审批".to_string(), description: None, position: NodePosition { x: 900.0, y: 100.0 }, config: NodeConfig { approver_type: Some("role".to_string()), approver_ids: Some(vec!["role_gm".to_string()]), approval_policy: Some("any".to_string()), ..Default::default() } },
                ProcessNode { node_id: "end".to_string(), node_type: "end".to_string(), name: "结束".to_string(), description: None, position: NodePosition { x: 900.0, y: 300.0 }, config: NodeConfig::default() },
            ],
            edges: vec![
                ProcessEdge { edge_id: "e1".to_string(), source_node_id: "start".to_string(), target_node_id: "dept_manager".to_string(), label: None, condition_expr: None },
                ProcessEdge { edge_id: "e2".to_string(), source_node_id: "dept_manager".to_string(), target_node_id: "finance".to_string(), label: None, condition_expr: None },
                ProcessEdge { edge_id: "e3".to_string(), source_node_id: "finance".to_string(), target_node_id: "condition".to_string(), label: None, condition_expr: None },
                ProcessEdge { edge_id: "e4".to_string(), source_node_id: "condition".to_string(), target_node_id: "gm".to_string(), label: Some("金额>1万".to_string()), condition_expr: Some("amount > 10000".to_string()) },
                ProcessEdge { edge_id: "e5".to_string(), source_node_id: "condition".to_string(), target_node_id: "end".to_string(), label: Some("默认".to_string()), condition_expr: None },
                ProcessEdge { edge_id: "e6".to_string(), source_node_id: "gm".to_string(), target_node_id: "end".to_string(), label: None, condition_expr: None },
            ],
            icon: None,
        },
        ProcessTemplate {
            template_id: "tpl_business_contract".to_string(),
            name: "业务合同审批".to_string(),
            category: "business".to_string(),
            description: "合同申请 → 部门经理 → 法务审核 → 总经理审批".to_string(),
            nodes: vec![
                ProcessNode { node_id: "start".to_string(), node_type: "start".to_string(), name: "开始".to_string(), description: None, position: NodePosition { x: 100.0, y: 200.0 }, config: NodeConfig::default() },
                ProcessNode { node_id: "dept_manager".to_string(), node_type: "approval".to_string(), name: "部门经理审批".to_string(), description: None, position: NodePosition { x: 300.0, y: 200.0 }, config: NodeConfig { approver_type: Some("initiator_manager".to_string()), approval_policy: Some("any".to_string()), ..Default::default() } },
                ProcessNode { node_id: "legal".to_string(), node_type: "approval".to_string(), name: "法务审核".to_string(), description: None, position: NodePosition { x: 500.0, y: 200.0 }, config: NodeConfig { approver_type: Some("role".to_string()), approver_ids: Some(vec!["role_legal".to_string()]), approval_policy: Some("all".to_string()), ..Default::default() } },
                ProcessNode { node_id: "gm".to_string(), node_type: "approval".to_string(), name: "总经理审批".to_string(), description: None, position: NodePosition { x: 700.0, y: 200.0 }, config: NodeConfig { approver_type: Some("role".to_string()), approver_ids: Some(vec!["role_gm".to_string()]), approval_policy: Some("any".to_string()), ..Default::default() } },
                ProcessNode { node_id: "end".to_string(), node_type: "end".to_string(), name: "结束".to_string(), description: None, position: NodePosition { x: 900.0, y: 200.0 }, config: NodeConfig::default() },
            ],
            edges: vec![
                ProcessEdge { edge_id: "e1".to_string(), source_node_id: "start".to_string(), target_node_id: "dept_manager".to_string(), label: None, condition_expr: None },
                ProcessEdge { edge_id: "e2".to_string(), source_node_id: "dept_manager".to_string(), target_node_id: "legal".to_string(), label: None, condition_expr: None },
                ProcessEdge { edge_id: "e3".to_string(), source_node_id: "legal".to_string(), target_node_id: "gm".to_string(), label: None, condition_expr: None },
                ProcessEdge { edge_id: "e4".to_string(), source_node_id: "gm".to_string(), target_node_id: "end".to_string(), label: None, condition_expr: None },
            ],
            icon: None,
        },
    ]
}
