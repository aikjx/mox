// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 审批流程管理（L5）：`/api/system/approval/*`
//!
//! # 功能
//! - 预置人事/财务/业务三类审批流程定义
//! - 发起审批（创建流程实例）
//! - 待审批列表（按审批人）
//! - 审批通过/拒绝
//! - 审批详情/历史记录
//! - 审批流程定义列表

use crate::GatewayState;
use crate::system::{ok, err, q_str, now_iso};
use mox_api_protocol::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use mox_flow_unified_process_core::{
    ProcessEngine, ProcessDef, ProcessStep, StepType, process_def::ApprovalPolicy,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

// ====================================================================
// 审批流程定义预置
// ====================================================================

/// 预置审批流程定义：人事审批、财务审批、业务审批
pub fn seed_approval_processes(engine: &Arc<ProcessEngine>) {
    // 人事审批：请假申请 → 部门经理审批 → HR审批 → 结束
    let hr_leave = build_hr_leave_process();
    let _ = engine.register_process(hr_leave);

    // 财务审批：报销申请 → 部门经理审批 → 财务审批 → 结束
    let finance_expense = build_finance_expense_process();
    let _ = engine.register_process(finance_expense);

    // 业务审批：合同申请 → 部门经理审批 → 法务审批 → 总经理审批 → 结束
    let business_contract = build_business_contract_process();
    let _ = engine.register_process(business_contract);
}

/// 构建人事请假审批流程
fn build_hr_leave_process() -> ProcessDef {
    let mut def = ProcessDef::new("人事请假审批", "hr");
    def.id = "hr-leave-approval".to_string();
    def.version = "1.0.0".to_string();
    def.description = Some("员工请假申请，需部门经理和HR审批".to_string());

    let start_id = def.start_step_id.clone();
    let end_id = def.end_step_id.clone();

    // 部门经理审批
    let mut mgr_approval = ProcessStep::new("部门经理审批", StepType::Approval);
    mgr_approval.id = "step-mgr-approval".to_string();
    mgr_approval.approvers = vec!["dept_manager".to_string()];
    mgr_approval.approval_policy = ApprovalPolicy::Any;
    mgr_approval.next_step_id = Some("step-hr-approval".to_string());
    def.steps.insert(mgr_approval.id.clone(), mgr_approval);

    // HR审批
    let mut hr_approval = ProcessStep::new("HR审批", StepType::Approval);
    hr_approval.id = "step-hr-approval".to_string();
    hr_approval.approvers = vec!["hr_specialist".to_string()];
    hr_approval.approval_policy = ApprovalPolicy::Any;
    hr_approval.next_step_id = Some(end_id.clone());
    def.steps.insert(hr_approval.id.clone(), hr_approval);

    // 开始 → 部门经理审批
    if let Some(start) = def.steps.get_mut(&start_id) {
        start.next_step_id = Some("step-mgr-approval".to_string());
    }

    def
}

/// 构建财务报销审批流程
fn build_finance_expense_process() -> ProcessDef {
    let mut def = ProcessDef::new("财务报销审批", "finance");
    def.id = "finance-expense-approval".to_string();
    def.version = "1.0.0".to_string();
    def.description = Some("费用报销申请，需部门经理和财务审批".to_string());

    let start_id = def.start_step_id.clone();
    let end_id = def.end_step_id.clone();

    // 部门经理审批
    let mut mgr_approval = ProcessStep::new("部门经理审批", StepType::Approval);
    mgr_approval.id = "step-mgr-approval".to_string();
    mgr_approval.approvers = vec!["dept_manager".to_string()];
    mgr_approval.approval_policy = ApprovalPolicy::Any;
    mgr_approval.next_step_id = Some("step-finance-approval".to_string());
    def.steps.insert(mgr_approval.id.clone(), mgr_approval);

    // 财务审批
    let mut finance_approval = ProcessStep::new("财务审批", StepType::Approval);
    finance_approval.id = "step-finance-approval".to_string();
    finance_approval.approvers = vec!["finance_specialist".to_string()];
    finance_approval.approval_policy = ApprovalPolicy::Any;
    finance_approval.next_step_id = Some(end_id.clone());
    def.steps.insert(finance_approval.id.clone(), finance_approval);

    // 开始 → 部门经理审批
    if let Some(start) = def.steps.get_mut(&start_id) {
        start.next_step_id = Some("step-mgr-approval".to_string());
    }

    def
}

/// 构建业务合同审批流程
fn build_business_contract_process() -> ProcessDef {
    let mut def = ProcessDef::new("业务合同审批", "business");
    def.id = "business-contract-approval".to_string();
    def.version = "1.0.0".to_string();
    def.description = Some("合同申请，需部门经理、法务、总经理审批".to_string());

    let start_id = def.start_step_id.clone();
    let end_id = def.end_step_id.clone();

    // 部门经理审批
    let mut mgr_approval = ProcessStep::new("部门经理审批", StepType::Approval);
    mgr_approval.id = "step-mgr-approval".to_string();
    mgr_approval.approvers = vec!["dept_manager".to_string()];
    mgr_approval.approval_policy = ApprovalPolicy::Any;
    mgr_approval.next_step_id = Some("step-legal-approval".to_string());
    def.steps.insert(mgr_approval.id.clone(), mgr_approval);

    // 法务审批
    let mut legal_approval = ProcessStep::new("法务审批", StepType::Approval);
    legal_approval.id = "step-legal-approval".to_string();
    legal_approval.approvers = vec!["legal_specialist".to_string()];
    legal_approval.approval_policy = ApprovalPolicy::Any;
    legal_approval.next_step_id = Some("step-gm-approval".to_string());
    def.steps.insert(legal_approval.id.clone(), legal_approval);

    // 总经理审批
    let mut gm_approval = ProcessStep::new("总经理审批", StepType::Approval);
    gm_approval.id = "step-gm-approval".to_string();
    gm_approval.approvers = vec!["general_manager".to_string()];
    gm_approval.approval_policy = ApprovalPolicy::Any;
    gm_approval.next_step_id = Some(end_id.clone());
    def.steps.insert(gm_approval.id.clone(), gm_approval);

    // 开始 → 部门经理审批
    if let Some(start) = def.steps.get_mut(&start_id) {
        start.next_step_id = Some("step-mgr-approval".to_string());
    }

    def
}

// ====================================================================
// API Handlers
// ====================================================================

/// 发起审批（创建流程实例）
pub async fn create_approval_handler(
    State(state): State<GatewayState>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let process_id = match body.get("processId").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return err("processId is required"),
    };
    let title = body.get("title").and_then(|v| v.as_str()).unwrap_or("未命名审批").to_string();
    let applicant = body.get("applicant").and_then(|v| v.as_str()).unwrap_or("admin-user").to_string();

    let mut variables = HashMap::new();
    variables.insert("title".to_string(), json!(title));
    variables.insert("applicant".to_string(), json!(applicant));
    variables.insert("created_at".to_string(), json!(now_iso()));
    if let Some(params) = body.get("params").and_then(|v| v.as_object()) {
        for (k, v) in params {
            variables.insert(k.clone(), v.clone());
        }
    }

    match state.process_engine.start_process(&process_id, variables) {
        Ok(instance_id) => {
            let instance = state.process_engine.get_instance(&instance_id)
                .expect("instance just created");
            ok(json!({
                "id": instance.instance_id,
                "processId": instance.process_id,
                "processName": instance.process_name,
                "status": format!("{:?}", instance.status),
                "title": title,
                "applicant": applicant,
                "createdAt": now_iso(),
            }))
        }
        Err(e) => err(&format!("start process failed: {:?}", e)),
    }
}

/// 待审批列表（按审批人）
pub async fn pending_approvals_handler(
    State(state): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let approver = q_str(&q, "approver", "dept_manager");
    let pending = state.process_engine.get_pending_approvals(&approver);
    ok(json!({
        "total": pending.len(),
        "rows": pending,
    }))
}

/// 审批通过
pub async fn approve_handler(
    State(state): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let step_id = match body.get("stepId").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return err("stepId is required"),
    };
    let approver = body.get("approver").and_then(|v| v.as_str()).unwrap_or("dept_manager").to_string();
    let comment = body.get("comment").and_then(|v| v.as_str());

    match state.process_engine.approve_step(&id, &step_id, &approver, comment) {
        Ok(_) => ok(json!({
            "id": id,
            "stepId": step_id,
            "action": "approved",
            "approver": approver,
            "approvedAt": now_iso(),
        })),
        Err(e) => err(&format!("approve failed: {:?}", e)),
    }
}

/// 审批拒绝
pub async fn reject_handler(
    State(state): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let step_id = match body.get("stepId").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return err("stepId is required"),
    };
    let approver = body.get("approver").and_then(|v| v.as_str()).unwrap_or("dept_manager").to_string();
    let comment = body.get("comment").and_then(|v| v.as_str());
    let reject_to_start = body.get("rejectToStart").and_then(|v| v.as_bool()).unwrap_or(false);

    match state.process_engine.reject_step(&id, &step_id, &approver, comment, reject_to_start) {
        Ok(_) => ok(json!({
            "id": id,
            "stepId": step_id,
            "action": "rejected",
            "approver": approver,
            "rejectToStart": reject_to_start,
            "rejectedAt": now_iso(),
        })),
        Err(e) => err(&format!("reject failed: {:?}", e)),
    }
}

/// 审批详情
pub async fn approval_detail_handler(
    State(state): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match state.process_engine.get_instance(&id) {
        Some(instance) => {
            let records = state.process_engine.approval_records.read();
            let history: Vec<Value> = records.iter()
                .filter(|r| r.instance_id == id)
                .map(|r| json!({
                    "recordId": r.record_id,
                    "stepId": r.step_id,
                    "approver": r.approver,
                    "action": r.action,
                    "comment": r.comment,
                    "approvedAt": r.approved_at,
                }))
                .collect();

            ok(json!({
                "id": instance.instance_id,
                "processId": instance.process_id,
                "processName": instance.process_name,
                "status": format!("{:?}", instance.status),
                "currentStepId": instance.context.current_step_id,
                "variables": instance.context.variables,
                "history": history,
                "createdAt": instance.context.started_at,
            }))
        }
        None => err(&format!("approval '{}' not found", id)),
    }
}

/// 审批列表（所有）
pub async fn list_approvals_handler(
    State(state): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let status_filter = q.get("status").map(|s| s.as_str());
    let instances = state.process_engine.instances.read();
    let rows: Vec<Value> = instances.iter()
        .filter(|(_, inst)| {
            if let Some(sf) = status_filter {
                format!("{:?}", inst.status).to_lowercase() == sf.to_lowercase()
            } else {
                true
            }
        })
        .map(|(_, inst)| json!({
            "id": inst.instance_id,
            "processId": inst.process_id,
            "processName": inst.process_name,
            "status": format!("{:?}", inst.status),
            "title": inst.context.variables.get("title").cloned().unwrap_or(json!("")),
            "applicant": inst.context.variables.get("applicant").cloned().unwrap_or(json!("")),
            "createdAt": inst.context.started_at,
        }))
        .collect();

    ok(json!({
        "total": rows.len(),
        "rows": rows,
    }))
}

/// 审批流程定义列表
pub async fn approval_definitions_handler(
    State(state): State<GatewayState>,
) -> ApiResponse<Value> {
    let defs = state.process_engine.process_defs.read();
    let rows: Vec<Value> = defs.iter()
        .map(|(_, def)| json!({
            "id": def.id,
            "name": def.name,
            "version": def.version,
            "category": def.category,
            "description": def.description,
            "stepCount": def.steps.len(),
            "enabled": def.enabled,
        }))
        .collect();

    ok(json!({
        "total": rows.len(),
        "rows": rows,
    }))
}

/// 审批历史记录
pub async fn approval_history_handler(
    State(state): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    let records = state.process_engine.approval_records.read();
    let history: Vec<Value> = records.iter()
        .filter(|r| r.instance_id == id)
        .map(|r| json!({
            "recordId": r.record_id,
            "stepId": r.step_id,
            "approver": r.approver,
            "action": r.action,
            "comment": r.comment,
            "attachments": r.attachments,
            "approvedAt": r.approved_at,
        }))
        .collect();

    ok(json!({
        "instanceId": id,
        "total": history.len(),
        "rows": history,
    }))
}

// ====================================================================
// 路由装配
// ====================================================================

pub fn build_approval_router() -> Router<GatewayState> {
    Router::new()
        .route("/api/system/approval", get(list_approvals_handler).post(create_approval_handler))
        .route("/api/system/approval/definitions", get(approval_definitions_handler))
        .route("/api/system/approval/pending", get(pending_approvals_handler))
        .route("/api/system/approval/:id", get(approval_detail_handler))
        .route("/api/system/approval/:id/approve", post(approve_handler))
        .route("/api/system/approval/:id/reject", post(reject_handler))
        .route("/api/system/approval/:id/history", get(approval_history_handler))
}
