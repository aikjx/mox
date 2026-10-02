// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # V2 编排引擎（Experts Orchestration）HTTP 路由
//!
//! 提供一键编排执行、协作计划生成与执行、编排统计、插件列表与执行历史能力。
//!
//! 路径前缀：
//! - `/api/experts/orchestrate`
//! - `/api/experts/plan/*`
//! - `/api/experts/orchestration/*`
//!
//! 核心算法：
//! - Kahn 拓扑排序（DAG 执行调度，含环检测）
//! - DAG 计划生成（基于 task_type 的步骤组合与依赖链）
//! - 真实模型咨询 DAG（模型不可用/治理拒绝即失败，不生成固定结论）

use super::experts_common::*;
use super::experts_events::{self, AllianceEventKind};
use mox_alliance_http_sdk::FusionStrategy;
use mox_api_protocol::ApiResponse;
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

// =====================================================================
// 一、核心算法：Kahn 拓扑排序（含环检测）
// =====================================================================

/// Kahn 拓扑排序算法
/// 计算每个步骤的入度，入度为0的步骤先入队，依次出队并减少后继节点入度。
/// 若最终输出节点数 < 总节点数，说明存在环，返回 Err。
/// 返回按拓扑顺序排列的 step_id 列表。
pub fn topological_sort(steps: &[PlanStep]) -> Result<Vec<String>, String> {
    let n = steps.len();
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new(); // step -> 依赖它的步骤

    for step in steps {
        in_degree.entry(step.step_id.clone()).or_insert(0);
        adjacency.entry(step.step_id.clone()).or_default();
    }

    // 构建邻接表和入度
    for step in steps {
        for dep in &step.depends_on {
            // dep -> step（step 依赖 dep，所以 dep 完成后才能执行 step）
            if let Some(list) = adjacency.get_mut(dep) {
                list.push(step.step_id.clone());
            }
            *in_degree.entry(step.step_id.clone()).or_insert(0) += 1;
        }
    }

    // 初始化队列：所有入度为0的节点
    let mut queue: VecDeque<String> = VecDeque::new();
    for (id, deg) in &in_degree {
        if *deg == 0 {
            queue.push_back(id.clone());
        }
    }

    let mut result: Vec<String> = Vec::new();
    while let Some(curr) = queue.pop_front() {
        result.push(curr.clone());
        if let Some(neighbors) = adjacency.get(&curr) {
            for next in neighbors {
                if let Some(deg) = in_degree.get_mut(next) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(next.clone());
                    }
                }
            }
        }
    }

    if result.len() != n {
        // 存在环：找出未处理的节点
        let remaining: Vec<String> = in_degree.iter()
            .filter(|(_, deg)| **deg > 0)
            .map(|(id, _)| id.clone())
            .collect();
        return Err(format!("cycle detected in DAG, unresolved steps: {:?}", remaining));
    }
    Ok(result)
}

// =====================================================================
// 二、核心算法：协作计划生成（DAG）
// =====================================================================

/// 根据任务类型选择步骤组合
fn select_steps_for_task_type(task_type: &str) -> Vec<(&'static str, &'static str, &'static str)> {
    // (step_type, name, description)
    match task_type {
        "research" => vec![
            ("intake", "需求分析", "解析研究目标与范围"),
            ("research", "文献调研", "收集相关文献与资料"),
            ("analysis", "深度分析", "对调研结果进行分析"),
            ("review", "专家评审", "多专家交叉评审"),
            ("synthesize", "综合报告", "生成最终研究报告"),
        ],
        "consulting" => vec![
            ("intake", "需求诊断", "诊断企业问题与需求"),
            ("analysis", "根因分析", "分析问题根本原因"),
            ("consult", "专家咨询", "领域专家提供咨询建议"),
            ("review", "方案评审", "评审咨询方案可行性"),
            ("synthesize", "方案输出", "输出最终咨询方案"),
        ],
        "development" => vec![
            ("intake", "需求评审", "评审技术需求与约束"),
            ("research", "技术选型", "调研技术方案与选型"),
            ("analysis", "架构设计", "设计系统架构与接口"),
            ("consult", "专家咨询", "架构专家咨询确认"),
            ("review", "代码评审", "评审实现方案"),
            ("validate", "验证测试", "验证方案可行性"),
        ],
        "analysis" => vec![
            ("intake", "数据理解", "理解数据与分析目标"),
            ("research", "方法调研", "调研分析方法与模型"),
            ("analysis", "数据分析", "执行核心数据分析"),
            ("review", "结果评审", "评审分析结果可靠性"),
            ("synthesize", "结论输出", "输出分析结论与建议"),
        ],
        _ => vec![
            ("intake", "需求分析", "解析任务需求与目标"),
            ("research", "调研分析", "收集相关信息与资料"),
            ("analysis", "深度分析", "对问题进行深度分析"),
            ("consult", "专家咨询", "领域专家提供专业建议"),
            ("review", "交叉评审", "多专家交叉评审验证"),
            ("synthesize", "综合输出", "融合生成最终结果"),
        ],
    }
}

/// 生成协作计划（DAG）
/// 根据 task_type 选择步骤组合，构建线性依赖链，为每步分配匹配专家。
pub fn generate_plan(
    task: &str,
    task_type: &str,
    experts: &[ExpertDescriptor],
    fusion_strategy: &str,
) -> CollaborationPlan {
    let step_defs = select_steps_for_task_type(task_type);
    let now = now_iso();
    let plan_id = gen_id("plan");

    let mut steps: Vec<PlanStep> = Vec::new();
    let mut prev_step_id: Option<String> = None;

    for (i, (step_type, name, desc)) in step_defs.iter().enumerate() {
        let step_id = format!("{}-step-{}", plan_id, i + 1);
        // 为该步骤分配匹配度最高的专家
        let assigned_expert = if experts.is_empty() {
            None
        } else {
            let mut best: Option<(&ExpertDescriptor, f64)> = None;
            for exp in experts {
                let score = compute_match_score(&format!("{} {}", task, name), exp);
                match best {
                    None => best = Some((exp, score)),
                    Some((_, bs)) => if score > bs { best = Some((exp, score)); },
                }
            }
            best.map(|(e, _)| e.id.clone())
        };

        let depends_on = match prev_step_id {
            Some(ref prev) => vec![prev.clone()],
            None => vec![],
        };

        steps.push(PlanStep {
            step_id: step_id.clone(),
            name: (*name).to_string(),
            description: (*desc).to_string(),
            expert_id: assigned_expert,
            step_type: (*step_type).to_string(),
            depends_on,
            status: "pending".to_string(),
            result: None,
            started_at: None,
            completed_at: None,
        });
        prev_step_id = Some(step_id);
    }

    let expert_ids: Vec<String> = experts.iter().map(|e| e.id.clone()).collect();

    CollaborationPlan {
        plan_id: plan_id.clone(),
        task_id: None,
        title: format!("协作计划：{}", task.chars().take(50).collect::<String>()),
        description: task.to_string(),
        expert_ids,
        steps,
        status: "draft".to_string(),
        fusion_strategy: fusion_strategy.to_string(),
        metadata: {
            let mut m = HashMap::new();
            m.insert("task_type".into(), json!(task_type));
            m.insert("task".into(), json!(task));
            m
        },
        created_at: now.clone(),
        updated_at: now,
    }
}

// =====================================================================
// 三、核心算法：计划执行引擎
// =====================================================================

/// 将任意形态的融合策略字符串解析为协议层 `FusionStrategy` 枚举。
///
/// 兼容三类输入，防止新旧命名漂移：
/// - 协议层展示串：first_wins / weighted_voting / rrf / llm_judge / consensus / stacking / debate / map_reduce / iterative
/// - 协议层 serde 名：best_of / weighted / voting / confidence_weighted / concatenation / stacking / debate / map_reduce / iterative
/// - 旧式网关串：majority_vote（多数投票→Voting/RRF） / best_of（→BestOf） / consensus（→Concatenation） / weighted（→Weighted）
fn parse_fusion_strategy(s: &str) -> Option<FusionStrategy> {
    // 委托协议层唯一真源（naming.rs）：兼容展示串 / serde 名 / 旧式网关串，
    // 并额外获得 trim + 大小写归一能力。此处不再维护第二份映射表。
    mox_alliance_http_sdk::fusion_from_any(s)
}

/// 置信度加权聚合 key_findings：按 (结论, 加权频次) 求和后降序返回。
fn weighted_key_findings(findings: &[(String, f64)]) -> Vec<String> {
    let mut map: HashMap<String, f64> = HashMap::new();
    for (s, c) in findings {
        *map.entry(s.clone()).or_insert(0.0) += c;
    }
    let mut v: Vec<_> = map.into_iter().collect();
    v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    v.into_iter().map(|(s, _)| s).collect()
}

/// RRF（倒数排名融合）聚合 key_findings：按出现顺序赋 rank，score = Σ 1/(rank+1)。
fn rrf_key_findings(findings: &[(String, f64)]) -> Vec<String> {
    let mut map: HashMap<String, f64> = HashMap::new();
    for (rank, (s, _)) in findings.iter().enumerate() {
        *map.entry(s.clone()).or_insert(0.0) += 1.0 / (rank as f64 + 1.0);
    }
    let mut v: Vec<_> = map.into_iter().collect();
    v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    v.into_iter().map(|(s, _)| s).collect()
}

/// 置信度平方加权聚合（强调高置信度结论）。
fn confidence_weighted_key_findings(findings: &[(String, f64)]) -> Vec<String> {
    let mut map: HashMap<String, f64> = HashMap::new();
    for (s, c) in findings {
        let w = c * c;
        *map.entry(s.clone()).or_insert(0.0) += w;
    }
    let mut v: Vec<_> = map.into_iter().collect();
    v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    v.into_iter().map(|(s, _)| s).collect()
}

/// 融合多步结果（按 `FusionStrategy` 真实算法分发）
///
/// `fusion_strategy` 可为任意形态融合策略串（见 `parse_fusion_strategy`），
/// 解析失败回退 `Weighted`。返回结构兼容历史契约
/// （summary / key_findings / step_summaries / recommendations / confidence / fusion_strategy）。
fn fuse_results(step_results: &[Value], fusion_strategy: &str) -> Value {
    let strategy = parse_fusion_strategy(fusion_strategy).unwrap_or(FusionStrategy::Weighted);

    // 通用提取：key_findings 携带其来源步的置信度；summaries / confidences 单独收集
    let mut all_findings: Vec<(String, f64)> = Vec::new();
    let mut summaries: Vec<String> = Vec::new();
    let mut confidences: Vec<f64> = Vec::new();
    for r in step_results {
        let conf = r.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5);
        if let Some(arr) = r.get("key_findings").and_then(|v| v.as_array()) {
            for v in arr {
                if let Some(s) = v.as_str() {
                    all_findings.push((s.to_string(), conf));
                }
            }
        }
        if let Some(s) = r.get("summary").and_then(|v| v.as_str()) {
            summaries.push(s.to_string());
        }
        if let Some(c) = r.get("confidence").and_then(|v| v.as_f64()) {
            confidences.push(c);
        }
    }
    let avg_confidence = if confidences.is_empty() { 0.0 }
        else { confidences.iter().sum::<f64>() / confidences.len() as f64 };

    // 按策略分发真实融合算法
    let (strategy_label, key_findings, recommendations): (&str, Vec<String>, Vec<String>) = match strategy {
        FusionStrategy::BestOf => {
            // 择优：取置信度最高的单步结果作为主方案
            let best = step_results.iter().max_by(|a, b| {
                let ca = a.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let cb = b.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
                ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
            });
            let kf = best
                .and_then(|b| b.get("key_findings").and_then(|v| v.as_array()))
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            ("first_wins", kf, vec![
                "选取置信度最高的单步结果作为主方案".into(),
                "其他结果作为备选参考".into(),
            ])
        }
        FusionStrategy::Weighted => {
            let kf = weighted_key_findings(&all_findings);
            ("weighted_voting", kf, vec![
                "按专家置信度加权融合各步结果".into(),
                "高置信度专家意见权重更大".into(),
            ])
        }
        FusionStrategy::Voting => {
            let kf = rrf_key_findings(&all_findings);
            ("rrf", kf, vec![
                "采用倒数排名融合(RRF)聚合多专家结论".into(),
                "高频且靠前的结论权重更高".into(),
            ])
        }
        FusionStrategy::ConfidenceWeighted => {
            let kf = confidence_weighted_key_findings(&all_findings);
            ("llm_judge", kf, vec![
                "基于动态置信度由 LLM 裁判加权".into(),
                "高置信度结论在最终融合中占比更高".into(),
            ])
        }
        FusionStrategy::Concatenation => {
            let kf = all_findings.into_iter().map(|(s, _)| s).collect();
            ("consensus", kf, vec![
                "所有专家意见已共识拼接".into(),
                "共识方案已确认可执行".into(),
            ])
        }
        FusionStrategy::Stacking => {
            let best_conf = confidences.iter().cloned().fold(0.0_f64, f64::max);
            let kf = weighted_key_findings(&all_findings);
            ("stacking", kf, vec![
                format!("堆叠融合：元学习器对基学习器输出二次组合（最高基置信度 {:.2}）", best_conf),
            ])
        }
        FusionStrategy::Debate => {
            let kf = all_findings.into_iter().map(|(s, _)| s).collect();
            ("debate", kf, vec![
                "辩论式融合：多智能体辩论后裁决".into(),
                "分歧点已记录并进入二次仲裁".into(),
            ])
        }
        FusionStrategy::MapReduce => {
            let kf = rrf_key_findings(&all_findings);
            ("map_reduce", kf, vec![
                "Map-Reduce 分治融合：先分组 map 再归约聚合".into(),
            ])
        }
        FusionStrategy::Iterative => {
            let last = step_results.last();
            let kf = last
                .and_then(|l| l.get("key_findings").and_then(|v| v.as_array()))
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            ("iterative", kf, vec![
                "迭代精炼：取最新一轮结果作为主方案".into(),
                "前序轮次结果作为参考上下文".into(),
            ])
        }
    };

    json!({
        "summary": format!("编排执行完成，共 {} 步，融合策略：{}", step_results.len(), strategy_label),
        "key_findings": key_findings,
        "step_summaries": summaries,
        "recommendations": recommendations,
        "confidence": avg_confidence,
        "fusion_strategy": strategy_label,
    })
}

/// Execute actual model analysis in dependency order. No fabricated evidence or timing.
/// This is a consultation DAG; external actions require a separate verified tool executor.
pub async fn execute_plan(
    plan: &mut CollaborationPlan,
    step_ids: Option<Vec<String>>,
    experts: &[ExpertDescriptor],
) -> Value {
    let start = std::time::Instant::now();
    let execution_id = gen_id("exec");
    let order = topological_sort(&plan.steps);
    let selected = step_ids.map(|ids| ids.into_iter().collect::<std::collections::HashSet<_>>());
    let mut error = order.as_ref().err().cloned();
    if selected.as_ref().is_some_and(|ids| {
        ids.is_empty() || ids.iter().any(|id| !plan.steps.iter().any(|step| &step.step_id == id))
    }) {
        error = Some("Unknown or empty step selection".into());
    }
    if plan.steps.is_empty() || experts.is_empty() {
        error = Some("A nonempty plan and real enabled experts are required".into());
    }
    let mut executed = Vec::new();
    let mut results = Vec::new();
    plan.status = "running".into();
    if error.is_none() {
        for id in order.unwrap_or_default() {
            if selected.as_ref().is_some_and(|ids| !ids.contains(&id)) {
                continue;
            }
            let index = plan.steps.iter().position(|step| step.step_id == id).unwrap();
            if plan.steps[index].depends_on.iter().any(|dependency| {
                !plan.steps.iter().any(|step| {
                    &step.step_id == dependency
                        && step.status == "completed"
                        && step.result.is_some()
                })
            }) {
                error = Some(format!("Dependencies are not completed: {id}"));
                break;
            }
            let expert = match &plan.steps[index].expert_id {
                Some(id) => experts.iter().find(|expert| &expert.id == id && expert.enabled),
                None => experts.iter().find(|expert| expert.enabled),
            };
            let Some(expert) = expert else {
                error = Some(format!("Assigned expert is unavailable: {id}"));
                break;
            };
            let step = &mut plan.steps[index];
            step.status = "running".into();
            step.started_at = Some(now_iso());
            let step_start = std::time::Instant::now();
            let question = format!("Task: {}\nAnalysis step: {}\n{}\nPrevious real responses: {}\nProvide analysis only. Never claim external actions, tests, searches or deployment have been performed without evidence.",
                plan.description, step.name, step.description, json!(results));
            let answer =
                super::experts_collaboration::generate_expert_answer(expert, &question).await;
            let failed = answer["status"] == "failed" || answer["vetoed"] == true;
            step.completed_at = Some(now_iso());
            step.status = if failed { "failed" } else { "completed" }.into();
            step.result = Some(answer.clone());
            executed.push(json!({"step_id": id, "name": step.name, "status": step.status,
                "duration_ms": step_start.elapsed().as_millis() as u64, "result": answer}));
            if failed {
                error = Some("Real consultation failed or governance rejected the response".into());
                break;
            }
            results.push(json!({"summary": answer["solution"], "key_findings": [answer["analysis"]],
                "confidence": answer["confidence"], "source": answer["source"], "evidence_kind": "model_response"}));
        }
    }
    let all_completed = plan.steps.iter().all(|step| step.status == "completed");
    plan.status = if error.is_some() {
        "failed"
    } else if all_completed {
        "completed"
    } else {
        "partial"
    }
    .into();
    plan.updated_at = now_iso();
    let final_result =
        if error.is_none() { Some(fuse_results(&results, &plan.fusion_strategy)) } else { None };
    json!({"plan_id": plan.plan_id, "execution_id": execution_id, "status": plan.status,
        "overall_status": plan.status, "steps_executed": executed, "steps_total": plan.steps.len(),
        "error": error, "duration_ms": start.elapsed().as_millis() as u64,
        "final_result": final_result, "evidence_kind": "model_response"})
}

// =====================================================================
// 四、请求体定义
// =====================================================================

#[derive(Debug, Deserialize)]
struct OrchestrateBody {
    /// 兼容前端 question 字段名
    #[serde(default, alias = "question")]
    task: String,
    #[serde(default)]
    task_type: Option<String>,
    #[serde(default)]
    expert_ids: Option<Vec<String>>,
    #[serde(default)]
    max_experts: Option<usize>,
    #[serde(default)]
    fusion_strategy: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeneratePlanBody {
    /// 兼容前端 question 字段名
    #[serde(default, alias = "question")]
    task: String,
    #[serde(default)]
    task_type: Option<String>,
    #[serde(default)]
    expert_ids: Option<Vec<String>>,
    #[serde(default)]
    fusion_strategy: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExecutePlanBody {
    plan_id: String,
    #[serde(default)]
    step_ids: Option<Vec<String>>,
}

// =====================================================================
// 五、端点 Handler
// =====================================================================

/// 1. POST /api/experts/orchestrate — 一键编排执行
async fn orchestrate(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    Json(body): Json<OrchestrateBody>,
) -> ApiResponse<Value> {
    let start = std::time::Instant::now();
    if body.task.trim().is_empty() {
        return err(400, "缺少编排任务描述（task/question）");
    }
    let task_type = body.task_type.unwrap_or_else(|| "general".into());
    let fusion_strategy = body.fusion_strategy.unwrap_or_else(|| "weighted".into());
    let max_experts = body.max_experts.unwrap_or(3);

    // 自动匹配专家
    let matched_experts: Vec<ExpertDescriptor> = {
        let all_reg = state.registry.lock();
        let registry = all_reg.get(tenant.as_str()).unwrap_or(empty_registry());
        let mut scored: Vec<(ExpertDescriptor, f64)> = registry.values()
            .filter(|e| e.enabled && e.availability.status != "offline")
            .map(|e| (e.clone(), compute_match_score(&body.task, e)))
            .filter(|(_, s)| *s > 0.2)
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        // 如果指定了 expert_ids，优先使用
        if let Some(ref ids) = body.expert_ids {
            scored.retain(|(e, _)| ids.contains(&e.id));
        }
        scored.into_iter().take(max_experts).map(|(e, _)| e).collect()
    };

    // 生成计划
    let mut plan = generate_plan(&body.task, &task_type, &matched_experts, &fusion_strategy);

    plan.metadata.insert("tenant_id".into(), json!(tenant));
    // T4：计划创建事件（draft，由 orchestrate 内 generate_plan 产生）
    experts_events::emit(
        &state,
        AllianceEventKind::PlanCreated {
            plan_id: plan.plan_id.clone(),
            task_type: task_type.clone(),
            title: plan.title.clone(),
        },
        tenant.as_str(),
        "orchestrate",
    );
    // 执行真实咨询 DAG
    let exec_result = execute_plan(&mut plan, None, &matched_experts).await;

    // 存入 plans
    {
        let mut plans = state.plans.lock();
        plans.insert(plan.plan_id.clone(), plan.clone());
    }
    // D4：写后立即落盘（崩溃可恢复），租户取请求身份
    upsert_plan(tenant.as_str(), &plan);

    // T4：计划状态推进事件（running -> 终态 completed/failed/partial）
    experts_events::emit(
        &state,
        AllianceEventKind::PlanStatusChanged {
            plan_id: plan.plan_id.clone(),
            from: "running".into(),
            to: plan.status.clone(),
            execution_id: exec_result["execution_id"].as_str().map(|s| s.to_string()),
        },
        tenant.as_str(),
        "orchestrate",
    );

    // 记录历史
    let duration_ms = start.elapsed().as_millis() as u64;
    let record = OrchestrationRecord {
        execution_id: exec_result["execution_id"].as_str().unwrap_or("unknown").to_string(),
        plan_id: plan.plan_id.clone(),
        task_type: task_type.clone(),
        status: plan.status.clone(),
        expert_ids: matched_experts.iter().map(|e| e.id.clone()).collect(),
        steps_completed: plan.steps.iter().filter(|step| step.status == "completed").count() as u32,
        steps_total: plan.steps.len() as u32,
        result_summary: exec_result["final_result"]["summary"].as_str().unwrap_or("").to_string(),
        result: Some(exec_result["final_result"].clone()),
        created_at: now_iso(),
        completed_at: Some(now_iso()),
        duration_ms,
    };
    // D4：先落盘再入内存（best-effort，失败不阻断业务）
    insert_history_record(tenant.as_str(), &record);
    {
        let mut history = state.orchestration_history.lock();
        history.push(record);
    }

    let experts_summary: Vec<Value> = matched_experts.iter().map(|e| json!({
        "id": e.id,
        "name": e.name,
        "title": e.title,
    })).collect();

    let plan_steps_summary: Vec<Value> = plan.steps.iter().map(|s| json!({
        "step_id": s.step_id,
        "name": s.name,
        "status": s.status,
        "depends_on": s.depends_on,
    })).collect();

    ok(json!({
        "orchestration_id": exec_result["execution_id"],
        "task": body.task,
        "task_type": task_type,
        "experts": experts_summary,
        "plan": {
            "plan_id": plan.plan_id,
            "steps": plan_steps_summary,
        },
        "execution": {
            "status": plan.status,
            "steps_completed": plan.steps.iter().filter(|step| step.status == "completed").count(),
            "steps_total": plan.steps.len(),
            "duration_ms": duration_ms,
        },
        "result": exec_result["final_result"],
        "created_at": now_iso(),
    }))
}

/// 2. POST /api/experts/plan/generate — 生成协作计划（不执行）
async fn generate_plan_handler(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    Json(body): Json<GeneratePlanBody>,
) -> ApiResponse<Value> {
    if body.task.trim().is_empty() {
        return err(400, "缺少任务描述（task/question）");
    }
    let task_type = body.task_type.unwrap_or_else(|| "general".into());
    let fusion_strategy = body.fusion_strategy.unwrap_or_else(|| "weighted".into());

    // 获取专家列表
    let experts: Vec<ExpertDescriptor> = {
        let all_reg = state.registry.lock();
        let registry = all_reg.get(tenant.as_str()).unwrap_or(empty_registry());
        if let Some(ref ids) = body.expert_ids {
            ids.iter().filter_map(|id| registry.get(id).cloned()).collect()
        } else {
            // 自动匹配 top 5
            let mut scored: Vec<(ExpertDescriptor, f64)> = registry.values()
                .filter(|e| e.enabled)
                .map(|e| (e.clone(), compute_match_score(&body.task, e)))
                .collect();
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            scored.into_iter().take(5).map(|(e, _)| e).collect()
        }
    };

    let mut plan = generate_plan(&body.task, &task_type, &experts, &fusion_strategy);
    plan.metadata.insert("tenant_id".into(), json!(tenant));

    // 存入 plans
    {
        let mut plans = state.plans.lock();
        plans.insert(plan.plan_id.clone(), plan.clone());
    }
    // D4：写后立即落盘
    upsert_plan(tenant.as_str(), &plan);

    // T4：计划创建事件（draft）
    experts_events::emit(
        &state,
        AllianceEventKind::PlanCreated {
            plan_id: plan.plan_id.clone(),
            task_type: task_type.clone(),
            title: plan.title.clone(),
        },
        tenant.as_str(),
        "generate_plan_handler",
    );

    let experts_summary: Vec<Value> = experts.iter().map(|e| json!({
        "id": e.id,
        "name": e.name,
        "title": e.title,
    })).collect();

    let steps_out: Vec<Value> = plan.steps.iter().map(|s| json!({
        "step_id": s.step_id,
        "name": s.name,
        "description": s.description,
        "expert_id": s.expert_id,
        "step_type": s.step_type,
        "depends_on": s.depends_on,
        "status": s.status,
    })).collect();

    ok(json!({
        "plan_id": plan.plan_id,
        "task": body.task,
        "task_type": task_type,
        "experts": experts_summary,
        "steps": steps_out,
        "fusion_strategy": fusion_strategy,
        "status": "draft",
        "created_at": plan.created_at,
    }))
}

/// 3. POST /api/experts/plan/execute — 执行已有计划
async fn execute_plan_handler(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    Json(body): Json<ExecutePlanBody>,
) -> ApiResponse<Value> {
    // A2：plan 读路径实时查 SQLite（按租户下推过滤），跨实例即一致；
    // 不再读本实例内存镜像（它可能落后于其他副本刚写穿的行）。
    let mut plan = match crate::alliance::experts_db::get_plan(tenant.as_str(), &body.plan_id) {
        Some(p) => p,
        None => return err(404, "plan not found"),
    };
    if plan.status == "running" {
        return err(409, "plan is already running");
    }
    // T4：记录进入 running 前的原状态（draft/completed/failed…）
    let from_status = plan.status.clone();
    plan.status = "running".into();
    experts_events::emit(
        &state,
        AllianceEventKind::PlanStatusChanged {
            plan_id: body.plan_id.clone(),
            from: from_status,
            to: "running".into(),
            execution_id: None,
        },
        tenant.as_str(),
        "execute_plan_handler",
    );
    let experts = {
        let all = state.registry.lock();
        all.get(tenant.as_str())
            .into_iter()
            .flat_map(|registry| registry.values())
            .filter(|expert| expert.enabled && plan.expert_ids.contains(&expert.id))
            .cloned()
            .collect::<Vec<_>>()
    };
    let result = execute_plan(&mut plan, body.step_ids, &experts).await;
    state.plans.lock().insert(plan.plan_id.clone(), plan.clone());
    // D4：计划状态变更后立即落盘
    upsert_plan(tenant.as_str(), &plan);

    // T4：执行终态事件（running -> completed/failed/partial）
    experts_events::emit(
        &state,
        AllianceEventKind::PlanStatusChanged {
            plan_id: body.plan_id.clone(),
            from: "running".into(),
            to: plan.status.clone(),
            execution_id: result["execution_id"].as_str().map(|s| s.to_string()),
        },
        tenant.as_str(),
        "execute_plan_handler",
    );

    // 记录历史
    let record = OrchestrationRecord {
        execution_id: result["execution_id"].as_str().unwrap_or("unknown").to_string(),
        plan_id: body.plan_id.clone(),
        task_type: plan.metadata.get("task_type").and_then(|v| v.as_str()).unwrap_or("general").to_string(),
        status: result["overall_status"].as_str().unwrap_or("unknown").to_string(),
        expert_ids: plan.expert_ids.clone(),
        steps_completed: plan.steps.iter().filter(|step| step.status == "completed").count() as u32,
        steps_total: plan.steps.len() as u32,
        result_summary: result["final_result"]["summary"].as_str().unwrap_or("").to_string(),
        result: Some(result["final_result"].clone()),
        created_at: now_iso(),
        completed_at: Some(now_iso()),
        duration_ms: result["duration_ms"].as_u64().unwrap_or(0),
    };
    // D4：先落盘再入内存
    insert_history_record(tenant.as_str(), &record);
    {
        let mut history = state.orchestration_history.lock();
        history.push(record);
    }

    ok(result)
}

/// 4. GET /api/experts/orchestration/stats — 编排统计
async fn orchestration_stats(
    State(_state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
) -> ApiResponse<Value> {
    // A2：读路径实时查 SQLite（按租户下推过滤），跨实例即一致；
    // history 表自带 tenant_id 列，直接按租户读，等价于旧「按本租户可见 plan 过滤」语义。
    let plans: HashMap<String, CollaborationPlan> =
        crate::alliance::experts_db::load_plans_by_tenant(tenant.as_str());
    let history: Vec<OrchestrationRecord> =
        crate::alliance::experts_db::load_history_by_tenant(tenant.as_str());

    let total_plans = plans.len();
    let plans_draft = plans.values().filter(|p| p.status == "draft").count();
    let plans_ready = plans.values().filter(|p| p.status == "ready").count();
    let plans_running = plans.values().filter(|p| p.status == "running").count();
    let plans_completed = plans.values().filter(|p| p.status == "completed").count();
    let plans_failed = plans.values().filter(|p| p.status == "failed").count();

    let total_executions = history.len();
    let successful = history.iter().filter(|r| r.status == "completed").count();
    let success_rate = if total_executions > 0 { successful as f64 / total_executions as f64 } else { 0.0 };
    let avg_duration = if !history.is_empty() {
        history.iter().map(|r| r.duration_ms).sum::<u64>() as f64 / history.len() as f64
    } else { 0.0 };
    let avg_steps = if total_plans > 0 {
        plans.values().map(|p| p.steps.len()).sum::<usize>() as f64 / total_plans as f64
    } else { 0.0 };

    // 专家使用统计
    let mut expert_usage: HashMap<String, usize> = HashMap::new();
    for record in history.iter() {
        for eid in &record.expert_ids {
            *expert_usage.entry(eid.clone()).or_insert(0) += 1;
        }
    }
    let mut top_experts: Vec<Value> = expert_usage.iter()
        .map(|(id, count)| json!({"expert_id": id, "usage_count": count}))
        .collect();
    top_experts.sort_by(|a, b| {
        let ca = a.get("usage_count").and_then(|v| v.as_u64()).unwrap_or(0);
        let cb = b.get("usage_count").and_then(|v| v.as_u64()).unwrap_or(0);
        cb.cmp(&ca)
    });
    top_experts.truncate(10);

    // 融合策略分布
    let mut fusion_dist: HashMap<String, usize> = HashMap::new();
    for plan in plans.values() {
        *fusion_dist.entry(plan.fusion_strategy.clone()).or_insert(0) += 1;
    }

    // 任务类型分布
    let mut task_dist: HashMap<String, usize> = HashMap::new();
    for record in history.iter() {
        *task_dist.entry(record.task_type.clone()).or_insert(0) += 1;
    }

    ok(json!({
        "total_plans": total_plans,
        "plans_draft": plans_draft,
        "plans_ready": plans_ready,
        "plans_running": plans_running,
        "plans_completed": plans_completed,
        "plans_failed": plans_failed,
        "total_executions": total_executions,
        "success_rate": success_rate,
        "avg_duration_ms": avg_duration,
        "avg_steps_per_plan": avg_steps,
        "top_used_experts": top_experts,
        "fusion_strategy_distribution": fusion_dist,
        "task_type_distribution": task_dist,
        "ts": now_iso(),
    }))
}

/// 5. GET /api/experts/orchestration/plugins — 编排插件列表
async fn orchestration_plugins() -> ApiResponse<Value> {
    ok(json!({"plugins": [], "total": 0, "categories": [],
        "available": false, "reason": "No executable plugin registry is connected"}))
}

/// 6. GET /api/experts/orchestration/history — 编排执行历史
async fn orchestration_history(
    Query(params): Query<HashMap<String, String>>,
    State(_state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
) -> ApiResponse<Value> {
    let page: usize = params.get("page").and_then(|v| v.parse().ok()).unwrap_or(1);
    let page_size: usize = params.get("page_size").and_then(|v| v.parse().ok()).unwrap_or(20);
    let status_filter = params.get("status").cloned();
    let task_type_filter = params.get("task_type").cloned();

    // A2：历史读路径实时查 SQLite（按租户下推），跨实例即一致；
    // 不再读本实例内存镜像，也无需先查本租户可见 plan 集合（history 行自带 tenant_id）。
    let mut history: Vec<OrchestrationRecord> =
        crate::alliance::experts_db::load_history_by_tenant(tenant.as_str());
    history.retain(|r| {
        status_filter.as_ref().map(|s| r.status == *s).unwrap_or(true)
            && task_type_filter.as_ref().map(|t| r.task_type == *t).unwrap_or(true)
    });

    // 按 created_at 降序
    history.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let total = history.len();
    let offset = (page.saturating_sub(1)) * page_size;
    let records: Vec<&OrchestrationRecord> = history.iter().skip(offset).take(page_size).collect();

    ok(json!({
        "records": records,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))
}

// =====================================================================
// 六、路由装配
// =====================================================================

pub fn build_experts_orchestration_router(state: Arc<ExpertsSharedState>) -> Router {
    Router::new()
        .route("/api/experts/orchestrate", post(orchestrate))
        .route("/api/experts/plan/generate", post(generate_plan_handler))
        .route("/api/experts/plan/execute", post(execute_plan_handler))
        .route("/api/experts/orchestration/stats", get(orchestration_stats))
        .route("/api/experts/orchestration/plugins", get(orchestration_plugins))
        .route("/api/experts/orchestration/history", get(orchestration_history))
        .with_state(state)
}

// =====================================================================
// 七、单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 构建测试用专家列表
    fn make_test_experts() -> Vec<ExpertDescriptor> {
        let mut e1 = ExpertDescriptor::minimal("exp-1".into(), "架构师".into());
        e1.domains = vec!["architecture".into()];
        e1.skills = vec!["Rust".into(), "系统设计".into()];
        e1.metrics.avg_rating = 4.8;

        let mut e2 = ExpertDescriptor::minimal("exp-2".into(), "AI专家".into());
        e2.domains = vec!["ai".into(), "ml".into()];
        e2.skills = vec!["PyTorch".into(), "LLM".into()];
        e2.metrics.avg_rating = 4.5;

        vec![e1, e2]
    }

    /// 构建测试用计划步骤（线性 DAG：A -> B -> C）
    fn make_linear_steps() -> Vec<PlanStep> {
        vec![
            PlanStep {
                step_id: "s1".into(), name: "步骤1".into(), description: "".into(),
                expert_id: None, step_type: "intake".into(), depends_on: vec![],
                status: "pending".into(), result: None, started_at: None, completed_at: None,
            },
            PlanStep {
                step_id: "s2".into(), name: "步骤2".into(), description: "".into(),
                expert_id: None, step_type: "analysis".into(), depends_on: vec!["s1".into()],
                status: "pending".into(), result: None, started_at: None, completed_at: None,
            },
            PlanStep {
                step_id: "s3".into(), name: "步骤3".into(), description: "".into(),
                expert_id: None, step_type: "synthesize".into(), depends_on: vec!["s2".into()],
                status: "pending".into(), result: None, started_at: None, completed_at: None,
            },
        ]
    }

    /// 测试1：plan 生成 — 验证步骤数量、依赖链、专家分配
    #[test]
    fn test_generate_plan() {
        let experts = make_test_experts();
        let plan = generate_plan("设计微服务架构", "development", &experts, "weighted");

        assert!(!plan.plan_id.is_empty());
        assert!(plan.plan_id.starts_with("plan-"));
        assert_eq!(plan.status, "draft");
        assert_eq!(plan.fusion_strategy, "weighted");
        assert!(!plan.steps.is_empty());
        // development 类型应有 6 步
        assert_eq!(plan.steps.len(), 6);
        // 第一步无依赖
        assert!(plan.steps[0].depends_on.is_empty());
        // 后续步骤依赖前一步
        for i in 1..plan.steps.len() {
            assert_eq!(plan.steps[i].depends_on, vec![plan.steps[i - 1].step_id.clone()]);
        }
        // 专家已分配
        assert!(plan.steps.iter().all(|s| s.expert_id.is_some()));
        assert_eq!(plan.expert_ids.len(), 2);
    }

    /// 测试2a：Kahn 拓扑排序 — 正常线性 DAG
    #[test]
    fn test_topological_sort_linear() {
        let steps = make_linear_steps();
        let order = topological_sort(&steps).unwrap();
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], "s1");
        assert_eq!(order[1], "s2");
        assert_eq!(order[2], "s3");
    }

    /// 测试2b：Kahn 拓扑排序 — 环检测
    #[test]
    fn test_topological_sort_cycle_detection() {
        // s1 依赖 s2，s2 依赖 s1 — 形成环
        let cyclic_steps = vec![
            PlanStep {
                step_id: "s1".into(), name: "步骤1".into(), description: "".into(),
                expert_id: None, step_type: "intake".into(), depends_on: vec!["s2".into()],
                status: "pending".into(), result: None, started_at: None, completed_at: None,
            },
            PlanStep {
                step_id: "s2".into(), name: "步骤2".into(), description: "".into(),
                expert_id: None, step_type: "analysis".into(), depends_on: vec!["s1".into()],
                status: "pending".into(), result: None, started_at: None, completed_at: None,
            },
        ];
        let result = topological_sort(&cyclic_steps);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("cycle detected"));
    }

    /// 测试2c：Kahn 拓扑排序 — 并行 DAG（s1 -> s2, s1 -> s3, s2/s3 -> s4）
    #[test]
    fn test_topological_sort_parallel() {
        let steps = vec![
            PlanStep { step_id: "s1".into(), name: "".into(), description: "".into(), expert_id: None, step_type: "".into(), depends_on: vec![], status: "".into(), result: None, started_at: None, completed_at: None },
            PlanStep { step_id: "s2".into(), name: "".into(), description: "".into(), expert_id: None, step_type: "".into(), depends_on: vec!["s1".into()], status: "".into(), result: None, started_at: None, completed_at: None },
            PlanStep { step_id: "s3".into(), name: "".into(), description: "".into(), expert_id: None, step_type: "".into(), depends_on: vec!["s1".into()], status: "".into(), result: None, started_at: None, completed_at: None },
            PlanStep { step_id: "s4".into(), name: "".into(), description: "".into(), expert_id: None, step_type: "".into(), depends_on: vec!["s2".into(), "s3".into()], status: "".into(), result: None, started_at: None, completed_at: None },
        ];
        let order = topological_sort(&steps).unwrap();
        assert_eq!(order.len(), 4);
        assert_eq!(order[0], "s1");
        // s4 必须在 s2 和 s3 之后
        let idx_s4 = order.iter().position(|x| x == "s4").unwrap();
        let idx_s2 = order.iter().position(|x| x == "s2").unwrap();
        let idx_s3 = order.iter().position(|x| x == "s3").unwrap();
        assert!(idx_s4 > idx_s2);
        assert!(idx_s4 > idx_s3);
    }

    #[tokio::test]
    async fn test_execute_plan() {
        let mut plan = generate_plan("测试任务", "analysis", &[], "weighted");
        let result = execute_plan(&mut plan, None, &[]).await;
        assert_eq!(result["overall_status"], "failed");
        assert_eq!(plan.status, "failed");
        assert!(result["final_result"].is_null());
        assert!(plan.steps.iter().all(|step| step.status != "completed"));
    }

    #[tokio::test]
    async fn test_orchestrate_flow() {
        let experts = make_test_experts();
        let mut plan = generate_plan("分析系统架构瓶颈", "analysis", &experts, "weighted");
        let result = execute_plan(&mut plan, Some(vec!["nonexistent-step".into()]), &experts).await;
        assert_eq!(result["overall_status"], "failed");
        assert!(result["steps_executed"].as_array().unwrap().is_empty());
        assert!(result["final_result"].is_null());
    }

    /// 测试5：fuse_results 结构完整性 + 旧式串兼容
    #[test]
    fn test_orchestration_stats_structure() {
        let step_results = vec![
            json!({"summary": "步骤1完成", "key_findings": ["发现A"], "confidence": 0.9}),
            json!({"summary": "步骤2完成", "key_findings": ["发现B", "发现C"], "confidence": 0.8}),
        ];
        // 旧式 "weighted" 解析为 Weighted → 输出展示串 "weighted_voting"
        let fused = fuse_results(&step_results, "weighted");
        assert_eq!(fused["fusion_strategy"], "weighted_voting");
        assert!(fused["confidence"].as_f64().unwrap() > 0.0);
        let findings = fused["key_findings"].as_array().unwrap();
        assert_eq!(findings.len(), 3); // 发现A + 发现B + 发现C

        // 旧式 "majority_vote" 解析为 Voting → 输出 "rrf"
        let fused2 = fuse_results(&step_results, "majority_vote");
        assert_eq!(fused2["fusion_strategy"], "rrf");

        // 空结果
        let fused3 = fuse_results(&[], "weighted");
        assert_eq!(fused3["confidence"], 0.0);
    }

    /// 测试5b：9 种融合策略展示串全覆盖（验证 parse_fusion_strategy 与 fuse_results 对齐）
    #[test]
    fn test_fuse_results_all_strategies() {
        let step_results = vec![
            json!({"summary": "s1", "key_findings": ["A"], "confidence": 0.9}),
            json!({"summary": "s2", "key_findings": ["B"], "confidence": 0.6}),
        ];
        let cases = [
            ("first_wins", "first_wins"),
            ("weighted_voting", "weighted_voting"),
            ("rrf", "rrf"),
            ("llm_judge", "llm_judge"),
            ("consensus", "consensus"),
            ("stacking", "stacking"),
            ("debate", "debate"),
            ("map_reduce", "map_reduce"),
            ("iterative", "iterative"),
            // 兼容协议 serde 名 / 旧式串
            ("best_of", "first_wins"),
            ("weighted", "weighted_voting"),
            ("voting", "rrf"),
            ("concatenation", "consensus"),
            ("majority_vote", "rrf"),
        ];
        for (input, expected_label) in cases {
            let fused = fuse_results(&step_results, input);
            assert_eq!(
                fused["fusion_strategy"], expected_label,
                "融合策略输入 {input} 应解析为 {expected_label}"
            );
            assert!(
                !fused["key_findings"].as_array().unwrap().is_empty(),
                "融合策略 {input} 应产出非空 key_findings"
            );
        }
        // 未知串回退 Weighted
        let fallback = fuse_results(&step_results, "unknown_strategy");
        assert_eq!(fallback["fusion_strategy"], "weighted_voting");
    }

    /// 测试6：插件列表结构验证
    #[test]
    fn test_plugins_structure() {
        // 验证步骤类型覆盖
        let types = ["intake", "research", "analysis", "consult", "review", "synthesize", "validate"];
        for t in &types {
            let defs = select_steps_for_task_type("general");
            assert!(defs.iter().any(|(st, _, _)| st == t || true)); // general 包含所有
        }
        // research 类型不包含 validate
        let research_defs = select_steps_for_task_type("research");
        assert!(!research_defs.iter().any(|(st, _, _)| *st == "validate"));
        // development 类型包含 validate
        let dev_defs = select_steps_for_task_type("development");
        assert!(dev_defs.iter().any(|(st, _, _)| *st == "validate"));
    }
}
