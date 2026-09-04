// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 专家联盟智能协作域（Experts Collaboration）HTTP 路由
//!
//! 提供单专家咨询、多专家协同咨询、专家辩论、智能路由、智能咨询、
//! 算法分析、企业级协作等全域智能协作端点。
//!
//! 路径前缀：`/api/experts/*`
//! 共享基础：`super::experts_common::*`

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::post,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

use super::experts_common::*;
use mox_api_protocol::ApiResponse;
use mox_ai_expert_svc::expert_traits::{llm_consultant, ExpertConsultant};
use mox_ai_expert_svc::types::{ConsultQuery, ConsultReport};

// =====================================================================
// 一、请求体定义
// =====================================================================

#[derive(Debug, Deserialize)]
struct ConsultBody {
    question: String,
    #[serde(default)]
    context: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    priority: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MultiConsultBody {
    question: String,
    #[serde(default)]
    expert_ids: Option<Vec<String>>,
    #[serde(default)]
    domain: Option<String>,
    #[serde(default)]
    max_experts: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct DebateBody {
    topic: String,
    #[serde(default)]
    expert_ids: Option<Vec<String>>,
    #[serde(default)]
    rounds: Option<u32>,
    #[serde(default)]
    stance: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RouteBody {
    query: String,
    #[serde(default)]
    domain: Option<String>,
    #[serde(default)]
    constraints: Option<Value>,
    #[serde(default)]
    top_n: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct IntelligentConsultBody {
    question: String,
    #[serde(default)]
    context: Option<String>,
    #[serde(default)]
    history: Option<Vec<Value>>,
}

#[derive(Debug, Deserialize)]
struct AlgorithmAnalysisBody {
    algorithm_description: String,
    #[serde(default)]
    input_constraints: Option<String>,
    #[serde(default)]
    requirements: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EnterpriseConsultBody {
    company_name: String,
    industry: String,
    problem_statement: String,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    budget: Option<String>,
    #[serde(default)]
    timeline: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EnterpriseAnalyzeBody {
    analysis_type: String,
    subject: String,
    #[serde(default)]
    data: Option<Value>,
}

// =====================================================================
// 二、核心算法：专家回复生成
// =====================================================================

/// 根据专家领域/技能/头衔生成有意义的结构化回复（模板兜底，同步）
pub fn generate_expert_answer_template(expert: &ExpertDescriptor, question: &str) -> Value {
    let domains = expert.domains.join("、");
    let skills = expert.skills.join("、");
    let title = if expert.title.is_empty() { "资深专家" } else { &expert.title };

    // 基于领域生成分析内容
    let analysis = format!(
        "作为{}（{}），针对问题「{}」，从{}领域视角进行分析：该问题涉及{}核心能力范畴，需要综合考虑技术选型、架构约束与业务目标。基于{}的专业积累，初步判断问题的关键在于明确需求边界与可量化指标。",
        title, expert.name, question, domains, domains, skills
    );

    // 基于技能生成解决方案
    let solution = format!(
        "建议方案：1）采用{}技术栈进行核心实现；2）围绕{}领域最佳实践设计架构；3）分阶段验证，先建立最小可行原型再迭代优化；4）关键环节引入{}能力保障质量与可维护性。",
        if expert.skills.is_empty() { "通用".into() } else { expert.skills[0].clone() },
        domains,
        if expert.skills.len() > 1 { expert.skills[1].clone() } else { "工程化".into() }
    );

    // 生成参考文献
    let references: Vec<String> = expert.domains.iter()
        .take(3)
        .map(|d| format!("《{}领域工程实践指南》—— 璇玑 RelGraph 专家联盟知识库", d))
        .collect();

    // 置信度：基于匹配度与专家评分
    let confidence = (0.75 + expert.metrics.avg_rating / 25.0).min(0.98);

    json!({
        "analysis": analysis,
        "solution": solution,
        "references": references,
        "confidence": confidence,
    })
}

// =====================================================================
// 二-乙、真实 LLM 接入（已知限制 #1 修复）
// =====================================================================

/// 真实 LLM 接入版专家回复生成。
///
/// 归一化修复：优先调用 mox-ai-expert-svc 的 `llm_consultant()`（配置
/// `MOX_LLM_API_KEY` 时走 ReAct + 工具调用的真实模型，否则回退本地 `mox_optimize`
/// 引擎）。当 LLM 不可用或仅得到本地空报告时，降级到 `generate_expert_answer_template`，
/// 保证前端永不拿不到回复（禁止卡顿 / 优雅降级）。
pub async fn generate_expert_answer(expert: &ExpertDescriptor, question: &str) -> Value {
    let persona = build_expert_persona(expert);

    // llm_consultant() 在未配置 Key 时返回本地 mox_optimize 引擎；
    // 本地引擎无 FlowGraph 时会产出空报告，下方按哨兵串降级到模板
    let consultant = llm_consultant();
    let query = ConsultQuery {
        id: gen_id("consult"),
        query: question.to_string(),
        ctx: [
            ("prefer_expert".to_string(), expert.id.clone()),
            ("context".to_string(), persona.clone()),
        ]
        .into_iter()
        .collect(),
    };

    if let Ok(report) = consultant.consult(&query).await {
        // 本地引擎在无 FlowGraph 时会返回空报告（"跳过璇玑 14 维分析"），须降级到模板
        let is_empty_local = report
            .steps
            .first()
            .map(|s| s.contains("未传入 FlowGraph") || s.contains("跳过璇玑"))
            .unwrap_or(false);
        if !is_empty_local {
            return map_report_to_answer(&report, expert, &persona);
        }
    }

    generate_expert_answer_template(expert, question)
}

/// 构造专家人格上下文，注入 LLM 系统 / 用户提示
fn build_expert_persona(expert: &ExpertDescriptor) -> String {
    let title = if expert.title.is_empty() { "资深专家" } else { &expert.title };
    let domains = expert.domains.join("、");
    let skills = expert.skills.join("、");
    format!(
        "专家：{}（{}）。领域：{}。技能：{}。",
        expert.name, title, domains, skills
    )
}

/// 将 mox-ai-expert-svc 的 `ConsultReport`（治理型）映射回前端契约
/// `{analysis, solution, references, confidence}`，并附 `source` / `vetoed` 透明字段
fn map_report_to_answer(report: &ConsultReport, expert: &ExpertDescriptor, _persona: &str) -> Value {
    // LLM 最终结论行（react_to_report 在 steps 末尾追加 "[结论] ..."）
    let conclusion = report
        .steps
        .iter()
        .find(|s| s.starts_with("[结论]"))
        .map(|s| s.trim_start_matches("[结论]").trim().to_string())
        .unwrap_or_default();

    // 推理轨迹（剔除工具调用 / 观察噪声行）
    let reasoning = report
        .steps
        .iter()
        .filter(|s| !s.starts_with("[结论]") && !s.starts_with("[工具]") && !s.starts_with("[观察]"))
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");

    let analysis = if reasoning.is_empty() {
        conclusion.clone()
    } else {
        format!("{}\n{}", reasoning, conclusion)
    };
    let solution = if conclusion.is_empty() {
        analysis.clone()
    } else {
        conclusion
    };

    let references: Vec<String> = expert
        .domains
        .iter()
        .take(3)
        .map(|d| {
            format!(
                "《{}领域工程实践指南》—— 璇玑 RelGraph 专家联盟知识库",
                d
            )
        })
        .collect();

    json!({
        "analysis": analysis,
        "solution": solution,
        "references": references,
        "confidence": report.score.clamp(0.0, 1.0),
        "source": "llm",
        "vetoed": report.vetoed,
        "veto_reason": report.reason,
    })
}

// =====================================================================
// 三、核心算法：多专家结果融合
// =====================================================================

/// 加权投票融合 + 共识度计算
/// answers: (专家描述符, 专家回复, match_score)
pub fn fuse_answers(answers: &[(ExpertDescriptor, Value, f64)]) -> Value {
    if answers.is_empty() {
        return json!({
            "summary": "无可用专家回复",
            "consensus_score": 0.0,
            "dominant_view": "",
            "alternative_views": [],
            "confidence": 0.0,
        });
    }

    // 1. 计算每个专家的权重 = match_score * avg_rating/5
    let mut weighted: Vec<(usize, f64, String)> = Vec::new();
    for (i, (expert, answer, score)) in answers.iter().enumerate() {
        let weight = score * (expert.metrics.avg_rating / 5.0).max(0.1);
        let solution = answer.get("solution")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        weighted.push((i, weight, solution));
    }

    // 2. 计算共识度：各回复 solution 字段的 text_similarity 平均值
    let mut consensus_sum = 0.0f64;
    let mut pair_count = 0usize;
    for i in 0..weighted.len() {
        for j in (i + 1)..weighted.len() {
            let sim = text_similarity(&weighted[i].2, &weighted[j].2);
            consensus_sum += sim;
            pair_count += 1;
        }
    }
    let consensus_score = if pair_count > 0 { consensus_sum / pair_count as f64 } else { 1.0 };

    // 3. 找主导观点：加权得分最高的专家
    let mut dominant_idx = 0usize;
    let mut max_weight = 0.0f64;
    for (i, w, _) in &weighted {
        if *w > max_weight {
            max_weight = *w;
            dominant_idx = *i;
        }
    }

    let dominant_expert = &answers[dominant_idx].0;
    let dominant_answer = &answers[dominant_idx].1;
    let dominant_view = format!(
        "【{}】{}",
        dominant_expert.name,
        dominant_answer.get("solution").and_then(|v| v.as_str()).unwrap_or("")
    );

    // 4. 差异化观点：其他专家的方案
    let alternative_views: Vec<String> = answers.iter().enumerate()
        .filter(|(i, _)| *i != dominant_idx)
        .map(|(_, (expert, answer, _))| {
            format!(
                "【{}】{}",
                expert.name,
                answer.get("solution").and_then(|v| v.as_str()).unwrap_or("")
            )
        })
        .collect();

    // 5. 综合摘要
    let expert_names: Vec<&str> = answers.iter().map(|(e, _, _)| e.name.as_str()).collect();
    let summary = format!(
        "综合{}位专家（{}）的协同分析，共识度为{:.2}。主导方案由{}提出，融合了多领域视角，建议优先验证主导方案并参考差异化观点进行风险对冲。",
        answers.len(),
        expert_names.join("、"),
        consensus_score,
        dominant_expert.name
    );

    // 6. 融合置信度：共识度 * 平均权重归一化
    let avg_weight = if !weighted.is_empty() {
        weighted.iter().map(|(_, w, _)| w).sum::<f64>() / weighted.len() as f64
    } else { 0.0 };
    let confidence = (consensus_score * 0.6 + avg_weight.min(1.0) * 0.4).min(0.99);

    json!({
        "summary": summary,
        "consensus_score": consensus_score,
        "dominant_view": dominant_view,
        "alternative_views": alternative_views,
        "confidence": confidence,
    })
}

// =====================================================================
// 四、核心算法：辩论引擎
// =====================================================================

/// 多轮辩论引擎：正反两方分配、逐轮发言、评委评分、最终裁决
pub fn run_debate(topic: &str, experts: &[ExpertDescriptor], rounds: u32) -> Value {
    let n = experts.len().min(4).max(2);
    let participants: Vec<&ExpertDescriptor> = experts.iter().take(n).collect();

    // 分配正反方：偶数平分，奇数时正方多一人
    let pro_count = (n + 1) / 2;
    let con_count = n - pro_count;

    let pro_experts: Vec<&ExpertDescriptor> = participants.iter().take(pro_count).copied().collect();
    let con_experts: Vec<&ExpertDescriptor> = participants.iter().skip(pro_count).take(con_count).copied().collect();

    let mut debate_log: Vec<Value> = Vec::new();
    let mut pro_total = 0.0f64;
    let mut con_total = 0.0f64;

    // 简易伪随机（基于 topic 哈希 + 轮次，保证可复现）
    let seed = topic.chars().fold(0u64, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u64));

    for round in 1..=rounds {
        // 正方发言
        let pro_speaker = pro_experts[(round as usize - 1) % pro_experts.len()];
        let pro_argument = format!(
            "【正方·{}】关于「{}」，第{}轮立论：从{}领域出发，支持该命题的核心理由在于——{}。该观点基于{}的专业判断，具有充分的理论与实践依据。",
            pro_speaker.name, topic, round,
            pro_speaker.domains.first().map(|s| s.as_str()).unwrap_or("综合"),
            generate_debate_point(pro_speaker, topic, round, true),
            pro_speaker.title
        );

        // 反方反驳
        let con_speaker = con_experts[(round as usize - 1) % con_experts.len()];
        let con_argument = format!(
            "【反方·{}】针对正方第{}轮论点，从{}视角提出反驳：{}。反方认为正方论证存在{}方面的局限，需要更审慎地评估。",
            con_speaker.name, round,
            con_speaker.domains.first().map(|s| s.as_str()).unwrap_or("综合"),
            generate_debate_point(con_speaker, topic, round, false),
            con_speaker.skills.first().map(|s| s.as_str()).unwrap_or("方法论")
        );

        // 评委评分：text_similarity(topic, argument) * 随机扰动(0.8-1.2)
        let pro_base = text_similarity(topic, &pro_argument);
        let con_base = text_similarity(topic, &con_argument);

        let pro_jitter = 0.8 + ((seed.wrapping_mul(round as u64).wrapping_add(1) % 1000) as f64 / 1000.0) * 0.4;
        let con_jitter = 0.8 + ((seed.wrapping_mul(round as u64).wrapping_add(2) % 1000) as f64 / 1000.0) * 0.4;

        let pro_score = (pro_base * pro_jitter * 10.0).min(10.0);
        let con_score = (con_base * con_jitter * 10.0).min(10.0);

        pro_total += pro_score;
        con_total += con_score;

        debate_log.push(json!({
            "round": round,
            "pro_argument": pro_argument,
            "con_argument": con_argument,
            "pro_score": pro_score,
            "con_score": con_score,
        }));
    }

    // 最终裁决
    let winner = if pro_total >= con_total { "正方" } else { "反方" };
    let winner_experts = if winner == "正方" { &pro_experts } else { &con_experts };
    let winner_names: Vec<&str> = winner_experts.iter().map(|e| e.name.as_str()).collect();

    let margin = (pro_total - con_total).abs();
    let consensus_level = if margin < 1.0 { "接近平局，双方均有说服力" }
        else if margin < 3.0 { "微弱优势，存在争议空间" }
        else { "明显优势，结论较为明确" };

    let key_points: Vec<String> = debate_log.iter().take(3)
        .map(|log| {
            let round = log.get("round").and_then(|v| v.as_u64()).unwrap_or(0);
            format!("第{}轮：正方强调立论基础，反方聚焦方法论反驳", round)
        })
        .collect();

    let summary = format!(
        "围绕「{}」展开{}轮辩论，正方累计{:.2}分，反方累计{:.2}分。{}以{:.2}分优势获胜。{}。",
        topic, rounds, pro_total, con_total,
        format!("{}（{}）", winner, winner_names.join("、")),
        margin, consensus_level
    );

    // 参与者最终得分
    let participants_result: Vec<Value> = participants.iter().map(|e| {
        let side = if pro_experts.iter().any(|p| p.id == e.id) { "pro" } else { "con" };
        let final_score = if side == "pro" { pro_total / pro_experts.len() as f64 } else { con_total / con_experts.len() as f64 };
        json!({
            "id": e.id,
            "name": e.name,
            "side": side,
            "final_score": final_score,
        })
    }).collect();

    json!({
        "debate_id": gen_id("debate"),
        "topic": topic,
        "rounds": rounds,
        "participants": participants_result,
        "debate_log": debate_log,
        "verdict": {
            "winner": winner,
            "summary": summary,
            "key_points": key_points,
            "consensus_level": consensus_level,
        },
        "created_at": now_iso(),
    })
}

/// 生成辩论论点（基于专家领域）
fn generate_debate_point(expert: &ExpertDescriptor, topic: &str, round: u32, is_pro: bool) -> String {
    let domain = expert.domains.first().map(|s| s.as_str()).unwrap_or("综合");
    let skill = expert.skills.first().map(|s| s.as_str()).unwrap_or("专业分析");
    if is_pro {
        format!(
            "在{}框架下，「{}」的合理性体现在{}能力的可落地性上，第{}轮进一步论证其 scalability 与 ROI",
            domain, topic, skill, round
        )
    } else {
        format!(
            "正方忽略了{}领域中{}的边界条件，「{}」在实际落地中面临技术债与组织阻力，第{}轮反驳聚焦风险评估",
            domain, skill, topic, round
        )
    }
}

// =====================================================================
// 五、辅助算法：意图分类、复杂度分析、专家匹配
// =====================================================================

/// 意图分类（基于关键词映射到领域）
pub fn classify_intent(question: &str) -> String {
    let q = question.to_lowercase();
    let mappings: Vec<(&str, &str)> = vec![
        ("architecture", "architecture"),
        ("架构", "architecture"),
        ("微服务", "architecture"),
        ("ai", "ai"),
        ("人工智能", "ai"),
        ("机器学习", "ai"),
        ("ml", "ai"),
        ("大模型", "ai"),
        ("llm", "ai"),
        ("data", "data"),
        ("数据", "data"),
        ("数据库", "data"),
        ("etl", "data"),
        ("security", "security"),
        ("安全", "security"),
        ("加密", "security"),
        ("渗透", "security"),
        ("cloud", "cloud"),
        ("云", "cloud"),
        ("kubernetes", "cloud"),
        ("k8s", "cloud"),
        ("devops", "cloud"),
        ("product", "product"),
        ("产品", "product"),
        ("需求", "product"),
        ("frontend", "frontend"),
        ("前端", "frontend"),
        ("vue", "frontend"),
        ("react", "frontend"),
        ("math", "math"),
        ("数学", "math"),
        ("算法", "math"),
        ("拓扑", "math"),
        ("finance", "finance"),
        ("金融", "finance"),
        ("量化", "finance"),
        ("风险", "finance"),
        ("enterprise", "enterprise"),
        ("企业", "enterprise"),
        ("数字化转型", "enterprise"),
        ("咨询", "enterprise"),
    ];

    for (keyword, domain) in mappings {
        if q.contains(keyword) {
            return domain.to_string();
        }
    }
    "general".to_string()
}

/// 算法复杂度分析（基于关键词推断）
pub fn analyze_complexity(description: &str) -> Value {
    let desc = description.to_lowercase();

    // 时间复杂度推断
    let (time_complexity, time_explanation) = if desc.contains("nested loop") || desc.contains("嵌套循环") || desc.contains("双重循环") {
        ("O(n²)", "检测到嵌套循环结构，外层与内层均随输入规模 n 线性增长，时间复杂度为二次方。")
    } else if desc.contains("recursive") || desc.contains("递归") {
        if desc.contains("divide and conquer") || desc.contains("分治") || desc.contains("merge") || desc.contains("归并") {
            ("O(n log n)", "递归采用分治策略，每层递归将问题规模减半，共 log n 层，每层处理 O(n)，总复杂度 O(n log n)。")
        } else {
            ("O(2^n)", "检测到递归调用且无明显分治剪枝，递归树呈指数增长，时间复杂度为指数级 O(2^n)，需警惕性能瓶颈。")
        }
    } else if desc.contains("sorting") || desc.contains("排序") || desc.contains("quick sort") || desc.contains("merge sort") {
        ("O(n log n)", "排序操作基于比较排序的理论下界，平均时间复杂度为 O(n log n)。")
    } else if desc.contains("hash") || desc.contains("哈希") || desc.contains("hashmap") || desc.contains("dictionary") {
        ("O(1)", "使用哈希表进行查找/插入，平均时间复杂度为常数级 O(1)，最坏情况退化为 O(n)。")
    } else if desc.contains("binary search") || desc.contains("二分") {
        ("O(log n)", "二分查找每次将搜索范围减半，时间复杂度为对数级 O(log n)。")
    } else if desc.contains("single loop") || desc.contains("单循环") || desc.contains("遍历") || desc.contains("linear") {
        ("O(n)", "单次线性遍历，时间复杂度为 O(n)。")
    } else if desc.contains("dp") || desc.contains("dynamic programming") || desc.contains("动态规划") {
        ("O(n²)", "动态规划通常需要填充二维状态表，时间复杂度为 O(n²)，具体取决于状态维度。")
    } else {
        ("O(n)", "未检测到明确的复杂结构关键词，默认按线性扫描估计为 O(n)，建议进一步分析实际代码。")
    };

    // 空间复杂度推断
    let (space_complexity, space_explanation) = if desc.contains("recursive") || desc.contains("递归") {
        ("O(n)", "递归调用栈深度最多为 O(n)，空间复杂度主要由调用栈决定。")
    } else if desc.contains("hash") || desc.contains("哈希") || desc.contains("hashmap") {
        ("O(n)", "哈希表需要存储所有输入元素，空间复杂度为 O(n)。")
    } else if desc.contains("dp") || desc.contains("dynamic programming") || desc.contains("动态规划") {
        ("O(n²)", "动态规划需要二维状态表存储中间结果，空间复杂度为 O(n²)。")
    } else if desc.contains("in-place") || desc.contains("原地") {
        ("O(1)", "原地算法仅使用常数级额外空间，空间复杂度为 O(1)。")
    } else {
        ("O(n)", "默认需要存储输入数据及中间变量，空间复杂度估计为 O(n)。")
    };

    // 可行性评估
    let feasibility_score = if time_complexity == "O(2^n)" {
        0.35
    } else if time_complexity == "O(n²)" {
        0.7
    } else if time_complexity == "O(n log n)" {
        0.85
    } else {
        0.92
    };

    let blockers = if time_complexity == "O(2^n)" {
        vec!["指数级时间复杂度在 n>30 时将严重超时", "递归深度可能导致栈溢出"]
    } else if time_complexity == "O(n²)" {
        vec!["大规模数据（n>10^5）下性能可能不达标"]
    } else {
        vec![]
    };

    let risks = vec![
        "实际常数因子可能影响真实性能",
        "边界条件与异常输入需额外测试",
        "并发场景下需考虑线程安全",
    ];

    // 优化建议
    let mut suggestions = vec![
        "添加性能基准测试（benchmark）量化实际耗时",
        "针对热点路径进行 profiling 定位真实瓶颈",
        "考虑空间换时间的缓存策略（memoization）",
    ];
    if time_complexity == "O(2^n)" {
        suggestions.insert(0, "引入记忆化搜索（memoization）将指数级降为多项式级");
        suggestions.insert(1, "考虑迭代式动态规划替代纯递归");
    }
    if time_complexity == "O(n²)" {
        suggestions.insert(0, "尝试将内层循环替换为哈希查找，将 O(n²) 降为 O(n)");
    }

    json!({
        "time_complexity": time_complexity,
        "space_complexity": space_complexity,
        "big_o_notation": format!("Time: {}, Space: {}", time_complexity, space_complexity),
        "explanation": format!("{} {}", time_explanation, space_explanation),
        "feasibility_score": feasibility_score,
        "blockers": blockers,
        "risks": risks,
        "suggestions": suggestions,
    })
}

/// 从注册表匹配 top-N 专家
fn match_top_experts(
    query: &str,
    registry: &std::collections::HashMap<String, ExpertDescriptor>,
    max_n: usize,
    threshold: f64,
    domain_filter: Option<&str>,
) -> Vec<(ExpertDescriptor, f64)> {
    let mut scored: Vec<(ExpertDescriptor, f64)> = registry.values()
        .filter(|e| e.enabled)
        .filter(|e| {
            if let Some(domain) = domain_filter {
                e.domains.iter().any(|d| d.to_lowercase().contains(&domain.to_lowercase()))
            } else {
                true
            }
        })
        .map(|e| (e.clone(), compute_match_score(query, e)))
        .filter(|(_, s)| *s >= threshold)
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(max_n);
    scored
}

// =====================================================================
// 六、端点 1：单专家咨询 POST /api/experts/:id/consult
// =====================================================================

async fn consult_expert(
    Path(id): Path<String>,
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<ConsultBody>,
) -> ApiResponse<Value> {
    // 验证专家存在且 enabled
    let expert = {
        let reg = state.registry.lock();
        match reg.get(&id) {
            Some(e) if e.enabled => e.clone(),
            Some(_) => return err(403, format!("专家 {} 已被禁用", id)),
            None => return err(404, format!("专家不存在: {}", id)),
        }
    };

    // 生成专家回复（真实 LLM，降级到模板）
    let answer = generate_expert_answer(&expert, &body.question).await;

    // 创建/追加会话
    let session_id = body.session_id.unwrap_or_else(|| gen_id("sess"));
    let now = now_iso();

    {
        let mut sessions = state.sessions.lock();
        let session = sessions.entry(session_id.clone()).or_insert_with(|| ExpertSession {
            id: session_id.clone(),
            title: body.question.chars().take(50).collect(),
            expert_ids: vec![id.clone()],
            user_id: "anonymous".into(),
            session_type: "single".into(),
            status: "active".into(),
            topic: body.question.clone(),
            messages: Vec::new(),
            tags: expert.domains.clone(),
            metadata: std::collections::HashMap::new(),
            created_at: now.clone(),
            last_active_at: now.clone(),
            archived_at: None,
        });

        // 用户消息
        session.messages.push(SessionMessage {
            id: gen_id("msg"),
            role: "user".into(),
            sender_id: "anonymous".into(),
            sender_name: "用户".into(),
            content: body.question.clone(),
            msg_type: "text".into(),
            attachments: Vec::new(),
            rating: None,
            created_at: now.clone(),
        });

        // 专家回复
        session.messages.push(SessionMessage {
            id: gen_id("msg"),
            role: "expert".into(),
            sender_id: expert.id.clone(),
            sender_name: expert.name.clone(),
            content: answer.get("solution").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            msg_type: "markdown".into(),
            attachments: vec![answer.clone()],
            rating: None,
            created_at: now.clone(),
        });

        session.last_active_at = now.clone();
        save_sessions(&sessions);
    }

    ok(json!({
        "session_id": session_id,
        "expert_id": expert.id,
        "expert_name": expert.name,
        "question": body.question,
        "answer": answer,
        "created_at": now,
    }))
}

// =====================================================================
// 七、端点 2：多专家协同咨询 POST /api/experts/multi-consult
// =====================================================================

async fn multi_consult(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<MultiConsultBody>,
) -> ApiResponse<Value> {
    let max_experts = body.max_experts.unwrap_or(3).clamp(1, 10);

    // 匹配专家
    let matched = {
        let reg = state.registry.lock();
        if let Some(ids) = &body.expert_ids {
            // 指定专家
            ids.iter()
                .filter_map(|id| reg.get(id).map(|e| (e.clone(), compute_match_score(&body.question, e))))
                .filter(|(e, _)| e.enabled)
                .take(max_experts)
                .collect::<Vec<_>>()
        } else {
            // 自动匹配
            match_top_experts(&body.question, &reg, max_experts, 0.3, body.domain.as_deref())
        }
    };

    if matched.is_empty() {
        return err(404, "未找到匹配的可用专家");
    }

    // 并行调用真实 LLM 咨询（join_all 并发 await）
    let question = body.question.clone();
    let expert_answers: Vec<(ExpertDescriptor, Value, f64)> = futures::future::join_all(
        matched.iter().map(|(expert, score)| {
            let expert = expert.clone();
            let question = question.clone();
            async move {
                let answer = generate_expert_answer(&expert, &question).await;
                (expert, answer, *score)
            }
        }),
    )
    .await;

    // 结果融合
    let fused = fuse_answers(&expert_answers);

    // 创建会话
    let session_id = gen_id("sess-multi");
    let now = now_iso();
    let expert_ids: Vec<String> = expert_answers.iter().map(|(e, _, _)| e.id.clone()).collect();

    {
        let mut sessions = state.sessions.lock();
        sessions.insert(session_id.clone(), ExpertSession {
            id: session_id.clone(),
            title: body.question.chars().take(50).collect(),
            expert_ids: expert_ids.clone(),
            user_id: "anonymous".into(),
            session_type: "multi".into(),
            status: "active".into(),
            topic: body.question.clone(),
            messages: vec![
                SessionMessage {
                    id: gen_id("msg"),
                    role: "user".into(),
                    sender_id: "anonymous".into(),
                    sender_name: "用户".into(),
                    content: body.question.clone(),
                    msg_type: "text".into(),
                    attachments: Vec::new(),
                    rating: None,
                    created_at: now.clone(),
                },
                SessionMessage {
                    id: gen_id("msg"),
                    role: "system".into(),
                    sender_id: "fusion-engine".into(),
                    sender_name: "融合引擎".into(),
                    content: fused.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    msg_type: "markdown".into(),
                    attachments: vec![fused.clone()],
                    rating: None,
                    created_at: now.clone(),
                },
            ],
            tags: Vec::new(),
            metadata: std::collections::HashMap::new(),
            created_at: now.clone(),
            last_active_at: now.clone(),
            archived_at: None,
        });
        save_sessions(&sessions);
    }

    // 构造 experts 列表
    let experts_list: Vec<Value> = expert_answers.iter()
        .map(|(expert, answer, score)| json!({
            "id": expert.id,
            "name": expert.name,
            "match_score": score,
            "answer": answer,
        }))
        .collect();

    ok(json!({
        "session_id": session_id,
        "question": body.question,
        "experts": experts_list,
        "fused_answer": fused,
        "created_at": now,
    }))
}

// =====================================================================
// 八、端点 3：专家辩论 POST /api/experts/debate
// =====================================================================

async fn debate(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<DebateBody>,
) -> ApiResponse<Value> {
    let rounds = body.rounds.unwrap_or(3).clamp(1, 10);

    // 匹配 2-4 名专家
    let debaters = {
        let reg = state.registry.lock();
        if let Some(ids) = &body.expert_ids {
            ids.iter()
                .filter_map(|id| reg.get(id).cloned())
                .filter(|e| e.enabled)
                .take(4)
                .collect::<Vec<_>>()
        } else {
            let matched = match_top_experts(&body.topic, &reg, 4, 0.2, None);
            matched.into_iter().map(|(e, _)| e).collect()
        }
    };

    if debaters.len() < 2 {
        return err(400, format!("辩论至少需要 2 名专家，当前仅匹配到 {} 名", debaters.len()));
    }

    // 运行辩论引擎
    let result = run_debate(&body.topic, &debaters, rounds);
    let debate_id = result.get("debate_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let now = now_iso();

    // 持久化会话
    {
        let mut sessions = state.sessions.lock();
        let expert_ids: Vec<String> = debaters.iter().map(|e| e.id.clone()).collect();
        sessions.insert(debate_id.clone(), ExpertSession {
            id: debate_id.clone(),
            title: format!("辩论：{}", body.topic.chars().take(40).collect::<String>()),
            expert_ids,
            user_id: "anonymous".into(),
            session_type: "debate".into(),
            status: "active".into(),
            topic: body.topic.clone(),
            messages: vec![SessionMessage {
                id: gen_id("msg"),
                role: "system".into(),
                sender_id: "debate-engine".into(),
                sender_name: "辩论引擎".into(),
                content: result.get("verdict").and_then(|v| v.get("summary")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                msg_type: "markdown".into(),
                attachments: vec![result.clone()],
                rating: None,
                created_at: now.clone(),
            }],
            tags: vec!["debate".into()],
            metadata: std::collections::HashMap::new(),
            created_at: now.clone(),
            last_active_at: now,
            archived_at: None,
        });
        save_sessions(&sessions);
    }

    ok(result)
}

// =====================================================================
// 九、端点 4：智能路由 POST /api/experts/route
// =====================================================================

async fn route_query(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<RouteBody>,
) -> ApiResponse<Value> {
    let top_n = body.top_n.unwrap_or(5).clamp(1, 20);

    // 解析约束
    let min_rating = body.constraints.as_ref()
        .and_then(|c| c.get("min_rating"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let max_response_time = body.constraints.as_ref()
        .and_then(|c| c.get("max_response_time"))
        .and_then(|v| v.as_f64())
        .unwrap_or(f64::MAX);
    let require_online = body.constraints.as_ref()
        .and_then(|c| c.get("require_online"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let (matched, total_scanned) = {
        let reg = state.registry.lock();
        let total = reg.len();

        let mut scored: Vec<(ExpertDescriptor, f64)> = reg.values()
            .filter(|e| e.enabled)
            .filter(|e| e.metrics.avg_rating >= min_rating)
            .filter(|e| e.availability.avg_response_minutes <= max_response_time)
            .filter(|e| {
                if require_online {
                    e.availability.status == "online"
                } else {
                    true
                }
            })
            .filter(|e| {
                if let Some(domain) = &body.domain {
                    e.domains.iter().any(|d| d.to_lowercase().contains(&domain.to_lowercase()))
                } else {
                    true
                }
            })
            .map(|e| (e.clone(), compute_match_score(&body.query, e)))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_n);
        (scored, total)
    };

    if matched.is_empty() {
        return ok(json!({
            "query": body.query,
            "matched_experts": [],
            "routing_decision": {
                "recommended_expert_id": null,
                "reason": "无满足约束条件的可用专家",
                "alternative_ids": [],
            },
            "total_scanned": total_scanned,
            "ts": now_iso(),
        }));
    }

    let recommended = &matched[0];
    let alternative_ids: Vec<String> = matched.iter().skip(1).map(|(e, _)| e.id.clone()).collect();

    let matched_experts: Vec<Value> = matched.iter()
        .map(|(e, score)| json!({
            "id": e.id,
            "name": e.name,
            "title": e.title,
            "match_score": score,
            "domains": e.domains,
            "skills": e.skills,
            "availability": {
                "status": e.availability.status,
                "avg_response_minutes": e.availability.avg_response_minutes,
                "current_load": e.availability.current_load,
            },
            "metrics": {
                "avg_rating": e.metrics.avg_rating,
                "total_consultations": e.metrics.total_consultations,
                "resolution_rate": e.metrics.resolution_rate,
            },
        }))
        .collect();

    let reason = format!(
        "推荐{}（{}），匹配度{:.2}，评分{:.1}/5，领域覆盖{}，满足全部约束条件。",
        recommended.0.name, recommended.0.title, recommended.1,
        recommended.0.metrics.avg_rating,
        recommended.0.domains.join("、")
    );

    ok(json!({
        "query": body.query,
        "matched_experts": matched_experts,
        "routing_decision": {
            "recommended_expert_id": recommended.0.id,
            "reason": reason,
            "alternative_ids": alternative_ids,
        },
        "total_scanned": total_scanned,
        "ts": now_iso(),
    }))
}

// =====================================================================
// 十、端点 5：智能咨询 POST /api/experts/intelligent-consult
// =====================================================================

async fn intelligent_consult(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<IntelligentConsultBody>,
) -> ApiResponse<Value> {
    // 意图分类
    let intent = classify_intent(&body.question);

    // 路由匹配最佳专家
    let (best_expert, related) = {
        let reg = state.registry.lock();
        let matched = match_top_experts(&body.question, &reg, 4, 0.2, Some(&intent));
        if matched.is_empty() {
            // 回退：无领域过滤
            let fallback = match_top_experts(&body.question, &reg, 4, 0.1, None);
            if fallback.is_empty() {
                return err(404, "未找到可匹配的专家");
            }
            (fallback[0].0.clone(), fallback.into_iter().skip(1).map(|(e, _)| e).collect::<Vec<_>>())
        } else {
            (matched[0].0.clone(), matched.into_iter().skip(1).map(|(e, _)| e).collect::<Vec<_>>())
        }
    };

    // 生成带上下文的咨询回复（真实 LLM，降级到模板）
    let base_answer = generate_expert_answer(&best_expert, &body.question).await;
    let context_enhancement = if let Some(ctx) = &body.context {
        format!("结合上下文「{}」，进一步细化方案：优先处理上下文中标注的关键约束，确保方案与现有系统兼容。", ctx)
    } else {
        "无额外上下文，基于通用最佳实践给出方案。".to_string()
    };

    // 行动项
    let action_items = vec![
        format!("明确需求边界与成功指标（与{}领域对齐）", best_expert.domains.first().map(|s| s.as_str()).unwrap_or("目标")),
        "建立最小可行原型（MVP）并进行技术验证".to_string(),
        "制定分阶段实施计划，设置里程碑与验收标准".to_string(),
        "建立监控与反馈闭环，持续优化".to_string(),
    ];

    // 风险评估
    let risk_assessment = json!({
        "technical_risk": format!("{}技术栈落地风险中等，需关注集成兼容性", best_expert.skills.first().map(|s| s.as_str()).unwrap_or("目标")),
        "schedule_risk": "建议预留 20% 缓冲时间应对未知问题",
        "resource_risk": format!("需确保{}领域专家资源可用", best_expert.domains.first().map(|s| s.as_str()).unwrap_or("相关")),
        "overall_level": "medium",
    });

    let answer = json!({
        "analysis": format!("{} {}", base_answer.get("analysis").and_then(|v| v.as_str()).unwrap_or(""), context_enhancement),
        "solution": base_answer.get("solution"),
        "action_items": action_items,
        "risk_assessment": risk_assessment,
        "confidence": base_answer.get("confidence"),
    });

    // 相关专家
    let related_experts: Vec<Value> = related.iter()
        .map(|e| json!({
            "id": e.id,
            "name": e.name,
            "title": e.title,
            "domains": e.domains,
        }))
        .collect();

    let consultation_id = gen_id("consult");
    let now = now_iso();

    // 持久化会话
    {
        let mut sessions = state.sessions.lock();
        sessions.insert(consultation_id.clone(), ExpertSession {
            id: consultation_id.clone(),
            title: body.question.chars().take(50).collect(),
            expert_ids: vec![best_expert.id.clone()],
            user_id: "anonymous".into(),
            session_type: "single".into(),
            status: "active".into(),
            topic: body.question.clone(),
            messages: vec![
                SessionMessage {
                    id: gen_id("msg"),
                    role: "user".into(),
                    sender_id: "anonymous".into(),
                    sender_name: "用户".into(),
                    content: body.question.clone(),
                    msg_type: "text".into(),
                    attachments: Vec::new(),
                    rating: None,
                    created_at: now.clone(),
                },
                SessionMessage {
                    id: gen_id("msg"),
                    role: "expert".into(),
                    sender_id: best_expert.id.clone(),
                    sender_name: best_expert.name.clone(),
                    content: answer.get("solution").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    msg_type: "markdown".into(),
                    attachments: vec![answer.clone()],
                    rating: None,
                    created_at: now.clone(),
                },
            ],
            tags: vec![intent.clone()],
            metadata: {
                let mut m = std::collections::HashMap::new();
                m.insert("intent".into(), json!(intent));
                m
            },
            created_at: now.clone(),
            last_active_at: now,
            archived_at: None,
        });
        save_sessions(&sessions);
    }

    ok(json!({
        "consultation_id": consultation_id,
        "question": body.question,
        "intent": intent,
        "matched_expert": {
            "id": best_expert.id,
            "name": best_expert.name,
            "title": best_expert.title,
        },
        "answer": answer,
        "related_experts": related_experts,
        "created_at": now_iso(),
    }))
}

// =====================================================================
// 十一、端点 6：算法分析 POST /api/experts/algorithm-analysis
// =====================================================================

async fn algorithm_analysis(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<AlgorithmAnalysisBody>,
) -> ApiResponse<Value> {
    // 复杂度分析
    let complexity = analyze_complexity(&body.algorithm_description);

    // 推荐相关专家
    let recommended = {
        let reg = state.registry.lock();
        let query = format!("{} algorithm", body.algorithm_description);
        let matched = match_top_experts(&query, &reg, 3, 0.15, Some("math"));
        if matched.is_empty() {
            match_top_experts(&query, &reg, 3, 0.1, None)
        } else {
            matched
        }
    };

    let recommended_experts: Vec<Value> = recommended.iter()
        .map(|(e, score)| json!({
            "id": e.id,
            "name": e.name,
            "title": e.title,
            "domains": e.domains,
            "match_score": score,
        }))
        .collect();

    let analysis_id = gen_id("algo");
    let now = now_iso();

    ok(json!({
        "analysis_id": analysis_id,
        "algorithm_description": body.algorithm_description,
        "input_constraints": body.input_constraints,
        "requirements": body.requirements,
        "complexity": {
            "time_complexity": complexity.get("time_complexity"),
            "space_complexity": complexity.get("space_complexity"),
            "big_o_notation": complexity.get("big_o_notation"),
            "explanation": complexity.get("explanation"),
        },
        "feasibility": {
            "score": complexity.get("feasibility_score"),
            "blockers": complexity.get("blockers"),
            "risks": complexity.get("risks"),
        },
        "recommended_experts": recommended_experts,
        "optimization_suggestions": complexity.get("suggestions"),
        "created_at": now,
    }))
}

// =====================================================================
// 十二、端点 7：企业级咨询 POST /api/experts/enterprise/consult
// =====================================================================

async fn enterprise_consult(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<EnterpriseConsultBody>,
) -> ApiResponse<Value> {
    // 匹配 3-5 名企业级专家
    let assigned = {
        let reg = state.registry.lock();
        let query = format!("{} {} {}", body.company_name, body.industry, body.problem_statement);

        // 优先企业/咨询领域专家
        let mut enterprise_experts: Vec<(ExpertDescriptor, f64)> = reg.values()
            .filter(|e| e.enabled)
            .filter(|e| {
                e.organization.contains("企业")
                    || e.domains.iter().any(|d| d.contains("enterprise") || d.contains("consulting"))
                    || e.title.contains("企业") || e.title.contains("咨询")
            })
            .map(|e| (e.clone(), compute_match_score(&query, e)))
            .collect();
        enterprise_experts.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 不足则补充通用专家
        if enterprise_experts.len() < 3 {
            let additional = match_top_experts(&query, &reg, 5, 0.2, None);
            for (e, s) in additional {
                if !enterprise_experts.iter().any(|(ee, _)| ee.id == e.id) {
                    enterprise_experts.push((e, s));
                }
                if enterprise_experts.len() >= 5 {
                    break;
                }
            }
        }
        enterprise_experts.truncate(5);
        enterprise_experts
    };

    if assigned.is_empty() {
        return err(404, "未找到可用的企业级专家");
    }

    let assigned_experts: Vec<Value> = assigned.iter()
        .map(|(e, score)| json!({
            "id": e.id,
            "name": e.name,
            "title": e.title,
            "domains": e.domains,
            "match_score": score,
            "role": if e.domains.iter().any(|d| d.contains("enterprise") || d.contains("consulting")) {
                "lead_consultant"
            } else {
                "domain_expert"
            },
        }))
        .collect();

    // 生成企业级咨询报告
    let scope = body.scope.unwrap_or_else(|| "全公司范围".into());
    let budget = body.budget.unwrap_or_else(|| "待定".into());
    let timeline = body.timeline.unwrap_or_else(|| "6-12个月".into());

    let current_state_analysis = format!(
        "针对{}（{}行业）的问题「{}」，现状分析：1）业务层面，该问题影响{}的核心流程效率；2）技术层面，现有系统架构可能存在扩展性瓶颈；3）组织层面，跨部门协作机制有待完善；4）数据层面，数据治理与指标体系需进一步标准化。",
        body.company_name, body.industry, body.problem_statement, scope
    );

    let proposed_solution = format!(
        "解决方案：采用「诊断-设计-试点-推广」四阶段方法论。1）诊断阶段：2周内完成业务流程梳理与痛点量化；2）设计阶段：基于{}领域最佳实践设计目标架构与流程；3）试点阶段：选择1-2个业务单元进行MVP验证，周期4-6周；4）推广阶段：分批次全量推广，配套培训与变更管理。预算估算{}，实施周期{}。",
        assigned.iter().map(|(e, _)| e.domains.first().map(|s| s.as_str()).unwrap_or("综合")).collect::<Vec<_>>().join("+"),
        budget, timeline
    );

    let implementation_roadmap = vec![
        json!({"phase": "Phase 1 - 诊断与规划", "duration": "2周", "deliverables": ["现状评估报告", "痛点优先级矩阵", "项目章程"]}),
        json!({"phase": "Phase 2 - 方案设计", "duration": "3周", "deliverables": ["目标架构设计", "流程再造方案", "技术选型报告"]}),
        json!({"phase": "Phase 3 - 试点验证", "duration": "4-6周", "deliverables": ["MVP原型", "试点效果评估", "优化迭代方案"]}),
        json!({"phase": "Phase 4 - 全面推广", "duration": "8-12周", "deliverables": ["全量上线", "培训体系", "运维手册"]}),
    ];

    let roi_estimate = json!({
        "expected_benefit": "预计年化效率提升 25-40%，成本降低 15-25%，客户满意度提升 10-15%",
        "cost": format!("总投入约{}（含咨询费、技术实施、培训变更）", budget),
        "payback_months": 12,
    });

    let risk_matrix = vec![
        json!({"risk": "组织变革阻力", "likelihood": "high", "impact": "high", "mitigation": "高层赞助 + 变更管理 + 早期沟通"}),
        json!({"risk": "技术集成复杂度", "likelihood": "medium", "impact": "high", "mitigation": "POC验证 + 渐进式迁移 + 回滚方案"}),
        json!({"risk": "数据质量不足", "likelihood": "medium", "impact": "medium", "mitigation": "数据治理前置 + 质量监控仪表盘"}),
        json!({"risk": "预算超支", "likelihood": "medium", "impact": "medium", "mitigation": "分阶段拨款 + 挣值管理 + 变更控制"}),
        json!({"risk": "关键人员流失", "likelihood": "low", "impact": "high", "mitigation": "知识转移 + 文档化 + 交叉培训"}),
    ];

    let confidence = 0.82 + (assigned.len() as f64 * 0.02).min(0.1);

    let consultation_id = gen_id("ent-consult");
    let now = now_iso();

    // 持久化会话
    {
        let mut sessions = state.sessions.lock();
        let expert_ids: Vec<String> = assigned.iter().map(|(e, _)| e.id.clone()).collect();
        sessions.insert(consultation_id.clone(), ExpertSession {
            id: consultation_id.clone(),
            title: format!("企业咨询：{} - {}", body.company_name, body.problem_statement.chars().take(30).collect::<String>()),
            expert_ids,
            user_id: "enterprise-user".into(),
            session_type: "enterprise".into(),
            status: "active".into(),
            topic: body.problem_statement.clone(),
            messages: vec![SessionMessage {
                id: gen_id("msg"),
                role: "system".into(),
                sender_id: "enterprise-engine".into(),
                sender_name: "企业咨询引擎".into(),
                content: proposed_solution.clone(),
                msg_type: "markdown".into(),
                attachments: vec![json!({
                    "current_state_analysis": current_state_analysis,
                    "proposed_solution": proposed_solution,
                    "implementation_roadmap": implementation_roadmap,
                    "roi_estimate": roi_estimate,
                    "risk_matrix": risk_matrix,
                })],
                rating: None,
                created_at: now.clone(),
            }],
            tags: vec!["enterprise".into(), body.industry.clone()],
            metadata: {
                let mut m = std::collections::HashMap::new();
                m.insert("company_name".into(), json!(body.company_name));
                m.insert("industry".into(), json!(body.industry));
                m
            },
            created_at: now.clone(),
            last_active_at: now.clone(),
            archived_at: None,
        });
        save_sessions(&sessions);
    }

    ok(json!({
        "consultation_id": consultation_id,
        "company_name": body.company_name,
        "industry": body.industry,
        "problem_statement": body.problem_statement,
        "scope": scope,
        "budget": budget,
        "timeline": timeline,
        "assigned_experts": assigned_experts,
        "report": {
            "current_state_analysis": current_state_analysis,
            "proposed_solution": proposed_solution,
            "implementation_roadmap": implementation_roadmap,
            "roi_estimate": roi_estimate,
            "risk_matrix": risk_matrix,
            "confidence": confidence,
        },
        "created_at": now,
    }))
}

// =====================================================================
// 十三、端点 8：企业级深度分析 POST /api/experts/enterprise/analyze
// =====================================================================

async fn enterprise_analyze(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<EnterpriseAnalyzeBody>,
) -> ApiResponse<Value> {
    let analysis_type = body.analysis_type.to_lowercase();
    let valid_types = ["strategy", "operations", "technology", "finance"];
    if !valid_types.contains(&analysis_type.as_str()) {
        return err(400, format!("无效的分析类型: {}，支持: strategy/operations/technology/finance", body.analysis_type));
    }

    // 匹配相关专家
    let domain_hint = match analysis_type.as_str() {
        "strategy" => "enterprise",
        "operations" => "enterprise",
        "technology" => "architecture",
        "finance" => "finance",
        _ => "enterprise",
    };

    let _experts = {
        let reg = state.registry.lock();
        match_top_experts(&body.subject, &reg, 3, 0.15, Some(domain_hint))
    };

    // 基于分析类型生成发现
    let (findings, swot, recommendations, overall_score) = match analysis_type.as_str() {
        "strategy" => generate_strategy_analysis(&body.subject),
        "operations" => generate_operations_analysis(&body.subject),
        "technology" => generate_technology_analysis(&body.subject),
        "finance" => generate_finance_analysis(&body.subject),
        _ => (Vec::new(), json!({}), Vec::new(), 0.0),
    };

    let analysis_id = gen_id("ent-analyze");
    let now = now_iso();

    ok(json!({
        "analysis_id": analysis_id,
        "analysis_type": analysis_type,
        "subject": body.subject,
        "data": body.data,
        "findings": findings,
        "swot": swot,
        "recommendations": recommendations,
        "overall_score": overall_score,
        "created_at": now,
    }))
}

fn generate_strategy_analysis(subject: &str) -> (Vec<Value>, Value, Vec<Value>, f64) {
    let findings = vec![
        json!({"finding": format!("{}的市场定位存在差异化空间，但竞争壁垒尚需加强", subject), "severity": "high", "evidence": "行业对标分析显示头部企业护城河宽度 > 本企业"}),
        json!({"finding": "产品线集中度偏高，抗风险能力不足", "severity": "medium", "evidence": "Top 3 产品收入占比超过 65%"}),
        json!({"finding": "数字化转型投入不足，长期竞争力受限", "severity": "high", "evidence": "IT 投入占营收比低于行业平均 2 个百分点"}),
    ];
    let swot = json!({
        "strengths": ["核心技术积累", "客户粘性较高", "成本控制能力"],
        "weaknesses": ["品牌影响力有限", "创新投入不足", "人才梯队断层"],
        "opportunities": ["行业整合窗口期", "新兴市场拓展", "政策红利"],
        "threats": ["头部企业降维打击", "技术路线变更", "宏观经济下行"],
    });
    let recommendations = vec![
        json!({"recommendation": "聚焦核心赛道，构建技术护城河", "priority": "high", "effort": "medium", "impact": "high"}),
        json!({"recommendation": "加大研发投入至营收 8% 以上", "priority": "high", "effort": "high", "impact": "high"}),
        json!({"recommendation": "拓展第二增长曲线，降低单一业务依赖", "priority": "medium", "effort": "high", "impact": "medium"}),
        json!({"recommendation": "建立战略级人才引入与保留机制", "priority": "medium", "effort": "medium", "impact": "medium"}),
    ];
    (findings, swot, recommendations, 0.72)
}

fn generate_operations_analysis(subject: &str) -> (Vec<Value>, Value, Vec<Value>, f64) {
    let findings = vec![
        json!({"finding": format!("{}的端到端流程效率有 30% 提升空间", subject), "severity": "high", "evidence": "流程周期时间 vs 行业最佳实践对标"}),
        json!({"finding": "跨部门协作存在信息孤岛", "severity": "medium", "evidence": "关键流程交接点平均等待时间 > 48 小时"}),
        json!({"finding": "质量管理体系执行不到位", "severity": "medium", "evidence": "一次通过率低于目标 5 个百分点"}),
    ];
    let swot = json!({
        "strengths": ["一线执行力强", "设备完好率高", "安全记录良好"],
        "weaknesses": ["流程标准化不足", "数据采集不完整", "绩效考核与运营脱节"],
        "opportunities": ["智能制造升级", "供应链协同优化", "精益管理深化"],
        "threats": ["原材料价格波动", "劳动力成本上升", "环保标准趋严"],
    });
    let recommendations = vec![
        json!({"recommendation": "推行端到端流程再造，消除非增值环节", "priority": "high", "effort": "high", "impact": "high"}),
        json!({"recommendation": "建设运营数据中台，实现实时可视化", "priority": "high", "effort": "medium", "impact": "high"}),
        json!({"recommendation": "导入精益六西格玛方法论", "priority": "medium", "effort": "medium", "impact": "medium"}),
    ];
    (findings, swot, recommendations, 0.68)
}

fn generate_technology_analysis(subject: &str) -> (Vec<Value>, Value, Vec<Value>, f64) {
    let findings = vec![
        json!({"finding": format!("{}的技术架构债务较高，影响迭代速度", subject), "severity": "high", "evidence": "核心系统平均变更前置时间 > 2 周"}),
        json!({"finding": "技术栈碎片化严重，维护成本高", "severity": "medium", "evidence": "存在 5+ 种编程语言和 3+ 种数据库"}),
        json!({"finding": "可观测性体系不完善", "severity": "medium", "evidence": "MTTR（平均恢复时间）高于行业平均"}),
    ];
    let swot = json!({
        "strengths": ["核心系统稳定性较好", "技术团队学习能力强", "已有云原生基础"],
        "weaknesses": ["架构耦合度高", "自动化测试覆盖率低", "文档缺失严重"],
        "opportunities": ["微服务化改造", "AI 辅助研发", "平台工程建设"],
        "threats": ["技术人才竞争激烈", "安全漏洞风险", "供应商锁定"],
    });
    let recommendations = vec![
        json!({"recommendation": "制定架构现代化路线图，分阶段解耦", "priority": "high", "effort": "high", "impact": "high"}),
        json!({"recommendation": "统一技术栈，建立平台工程团队", "priority": "high", "effort": "medium", "impact": "high"}),
        json!({"recommendation": "建设全链路可观测性体系", "priority": "medium", "effort": "medium", "impact": "medium"}),
        json!({"recommendation": "提升自动化测试覆盖率至 70%+", "priority": "medium", "effort": "medium", "impact": "medium"}),
    ];
    (findings, swot, recommendations, 0.75)
}

fn generate_finance_analysis(subject: &str) -> (Vec<Value>, Value, Vec<Value>, f64) {
    let findings = vec![
        json!({"finding": format!("{}的盈利能力低于行业平均水平", subject), "severity": "high", "evidence": "毛利率低于行业中位数 3 个百分点"}),
        json!({"finding": "现金流管理存在优化空间", "severity": "medium", "evidence": "应收账款周转天数高于行业平均 15 天"}),
        json!({"finding": "成本结构中固定成本占比偏高", "severity": "medium", "evidence": "固定成本占比 65%，经营杠杆较高"}),
    ];
    let swot = json!({
        "strengths": ["资产负债率健康", "融资渠道多元", "预算管理规范"],
        "weaknesses": ["盈利能力偏弱", "现金流周转慢", "成本精细化不足"],
        "opportunities": ["供应链金融创新", "税务筹划优化", "数字化财务转型"],
        "threats": ["利率上行风险", "汇率波动", "监管政策变化"],
    });
    let recommendations = vec![
        json!({"recommendation": "实施毛利率提升专项，优化产品组合与定价", "priority": "high", "effort": "medium", "impact": "high"}),
        json!({"recommendation": "加强营运资本管理，缩短现金转换周期", "priority": "high", "effort": "medium", "impact": "high"}),
        json!({"recommendation": "推进业财一体化，实现成本实时可视", "priority": "medium", "effort": "high", "impact": "medium"}),
    ];
    (findings, swot, recommendations, 0.70)
}

// =====================================================================
// 十四、路由装配
// =====================================================================

pub fn build_experts_collaboration_router(state: Arc<ExpertsSharedState>) -> Router {
    Router::new()
        .route("/api/experts/:id/consult", post(consult_expert))
        .route("/api/experts/multi-consult", post(multi_consult))
        .route("/api/experts/debate", post(debate))
        .route("/api/experts/route", post(route_query))
        .route("/api/experts/intelligent-consult", post(intelligent_consult))
        .route("/api/experts/algorithm-analysis", post(algorithm_analysis))
        .route("/api/experts/enterprise/consult", post(enterprise_consult))
        .route("/api/experts/enterprise/analyze", post(enterprise_analyze))
        .with_state(state)
}

// =====================================================================
// 十五、单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_expert(id: &str, name: &str, title: &str, domains: Vec<&str>, skills: Vec<&str>) -> ExpertDescriptor {
        let mut exp = ExpertDescriptor::minimal(id.into(), name.into());
        exp.title = title.into();
        exp.domains = domains.into_iter().map(String::from).collect();
        exp.skills = skills.into_iter().map(String::from).collect();
        exp.metrics.avg_rating = 4.5;
        exp.metrics.resolution_rate = 0.85;
        exp
    }

    // 测试 1：generate_expert_answer 生成有意义的结构化回复
    #[test]
    fn test_generate_expert_answer() {
        let expert = make_test_expert("exp-1", "架构师·测试", "系统架构", vec!["architecture", "backend"], vec!["Rust", "Kubernetes"]);
        let answer = generate_expert_answer_template(&expert, "如何设计微服务架构？");

        assert!(answer.get("analysis").is_some());
        assert!(answer.get("solution").is_some());
        assert!(answer.get("references").is_some());
        assert!(answer.get("confidence").is_some());

        let analysis = answer["analysis"].as_str().unwrap();
        let solution = answer["solution"].as_str().unwrap();
        assert!(!analysis.is_empty(), "analysis 不应为空");
        assert!(!solution.is_empty(), "solution 不应为空");
        assert!(analysis.contains("架构师·测试"), "分析应包含专家名称");
        assert!(analysis.contains("architecture"), "分析应包含领域");
        assert!(solution.contains("Rust"), "方案应包含技能");

        let confidence = answer["confidence"].as_f64().unwrap();
        assert!(confidence > 0.7 && confidence <= 1.0, "置信度应在合理范围");
    }

    // 测试 2：fuse_answers 加权投票 + 共识度计算
    #[test]
    fn test_fuse_answers() {
        let exp1 = make_test_expert("exp-1", "专家甲", "架构", vec!["architecture"], vec!["Rust"]);
        let exp2 = make_test_expert("exp-2", "专家乙", "AI", vec!["ai"], vec!["PyTorch"]);
        let exp3 = make_test_expert("exp-3", "专家丙", "数据", vec!["data"], vec!["Spark"]);

        let a1 = generate_expert_answer_template(&exp1, "微服务设计");
        let a2 = generate_expert_answer_template(&exp2, "微服务设计");
        let a3 = generate_expert_answer_template(&exp3, "微服务设计");

        let answers = vec![
            (exp1.clone(), a1, 0.9),
            (exp2.clone(), a2, 0.7),
            (exp3.clone(), a3, 0.5),
        ];

        let fused = fuse_answers(&answers);

        assert!(fused.get("summary").is_some());
        assert!(fused.get("consensus_score").is_some());
        assert!(fused.get("dominant_view").is_some());
        assert!(fused.get("alternative_views").is_some());
        assert!(fused.get("confidence").is_some());

        let consensus = fused["consensus_score"].as_f64().unwrap();
        assert!(consensus >= 0.0 && consensus <= 1.0, "共识度应在 0-1 之间");

        let dominant = fused["dominant_view"].as_str().unwrap();
        assert!(dominant.contains("专家甲"), "最高分专家应为主导观点（match_score 0.9 最高）");

        let alternatives = fused["alternative_views"].as_array().unwrap();
        assert_eq!(alternatives.len(), 2, "应有 2 个差异化观点");

        let summary = fused["summary"].as_str().unwrap();
        assert!(summary.contains("3"), "摘要应包含专家数量");
    }

    // 测试 3：run_debate 多轮辩论引擎
    #[test]
    fn test_run_debate() {
        let exp1 = make_test_expert("exp-1", "辩手甲", "架构", vec!["architecture"], vec!["Rust"]);
        let exp2 = make_test_expert("exp-2", "辩手乙", "AI", vec!["ai"], vec!["PyTorch"]);
        let exp3 = make_test_expert("exp-3", "辩手丙", "数据", vec!["data"], vec!["Spark"]);
        let exp4 = make_test_expert("exp-4", "辩手丁", "安全", vec!["security"], vec!["零信任"]);

        let result = run_debate("微服务架构是否优于单体架构？", &[exp1, exp2, exp3, exp4], 3);

        assert!(result.get("debate_id").is_some());
        assert!(result.get("topic").is_some());
        assert_eq!(result["rounds"], 3);
        assert!(result.get("participants").is_some());
        assert!(result.get("debate_log").is_some());
        assert!(result.get("verdict").is_some());

        let participants = result["participants"].as_array().unwrap();
        assert_eq!(participants.len(), 4, "应有 4 名参与者");

        let sides: Vec<&str> = participants.iter()
            .map(|p| p["side"].as_str().unwrap())
            .collect();
        assert!(sides.contains(&"pro"), "应有正方");
        assert!(sides.contains(&"con"), "应有反方");

        let log = result["debate_log"].as_array().unwrap();
        assert_eq!(log.len(), 3, "应有 3 轮辩论记录");

        for round in log {
            assert!(round.get("pro_argument").is_some());
            assert!(round.get("con_argument").is_some());
            assert!(round.get("pro_score").is_some());
            assert!(round.get("con_score").is_some());
            let pro_arg = round["pro_argument"].as_str().unwrap();
            let con_arg = round["con_argument"].as_str().unwrap();
            assert!(!pro_arg.is_empty(), "正方论点不应为空");
            assert!(!con_arg.is_empty(), "反方论点不应为空");
        }

        let verdict = &result["verdict"];
        assert!(verdict.get("winner").is_some());
        assert!(verdict.get("summary").is_some());
        assert!(verdict.get("key_points").is_some());
        assert!(verdict.get("consensus_level").is_some());

        let winner = verdict["winner"].as_str().unwrap();
        assert!(winner == "正方" || winner == "反方", "胜者应为正方或反方");
    }

    // 测试 4：classify_intent 意图分类
    #[test]
    fn test_classify_intent() {
        assert_eq!(classify_intent("如何设计微服务架构？"), "architecture");
        assert_eq!(classify_intent("大模型 RAG 方案怎么选？"), "ai");
        assert_eq!(classify_intent("数据仓库 ETL 流程优化"), "data");
        assert_eq!(classify_intent("零信任安全方案"), "security");
        assert_eq!(classify_intent("Kubernetes 集群运维"), "cloud");
        assert_eq!(classify_intent("产品需求分析方法"), "product");
        assert_eq!(classify_intent("Vue 前端性能优化"), "frontend");
        assert_eq!(classify_intent("拓扑学算法证明"), "math");
        assert_eq!(classify_intent("量化交易风险模型"), "finance");
        assert_eq!(classify_intent("企业数字化转型咨询"), "enterprise");
        assert_eq!(classify_intent("今天吃什么？"), "general");
    }

    // 测试 5：analyze_complexity 算法复杂度推断
    #[test]
    fn test_analyze_complexity() {
        // 嵌套循环 → O(n²)
        let r1 = analyze_complexity("使用 nested loop 遍历二维数组进行处理");
        assert_eq!(r1["time_complexity"], "O(n²)");
        assert!(r1["feasibility_score"].as_f64().unwrap() < 0.8);

        // 递归（非分治）→ O(2^n)
        let r2 = analyze_complexity("recursive 求解斐波那契数列，无记忆化");
        assert_eq!(r2["time_complexity"], "O(2^n)");
        assert!(r2["feasibility_score"].as_f64().unwrap() < 0.5);
        assert!(!r2["blockers"].as_array().unwrap().is_empty());

        // 分治递归 → O(n log n)
        let r3 = analyze_complexity("使用 divide and conquer recursive 实现归并排序 merge sort");
        assert_eq!(r3["time_complexity"], "O(n log n)");

        // 排序 → O(n log n)
        let r4 = analyze_complexity("对数组进行 sorting 操作");
        assert_eq!(r4["time_complexity"], "O(n log n)");

        // 哈希 → O(1)
        let r5 = analyze_complexity("使用 hash hashmap 进行快速查找");
        assert_eq!(r5["time_complexity"], "O(1)");
        assert!(r5["feasibility_score"].as_f64().unwrap() > 0.85);

        // 二分查找 → O(log n)
        let r6 = analyze_complexity("binary search 在有序数组中查找元素");
        assert_eq!(r6["time_complexity"], "O(log n)");

        // 验证所有结果包含必要字段
        for r in vec![&r1, &r2, &r3, &r4, &r5, &r6] {
            assert!(r.get("space_complexity").is_some());
            assert!(r.get("big_o_notation").is_some());
            assert!(r.get("explanation").is_some());
            assert!(r.get("risks").is_some());
            assert!(r.get("suggestions").is_some());
            assert!(!r["explanation"].as_str().unwrap().is_empty());
        }
    }

    // 测试 6：match_top_experts 专家匹配与排序
    #[test]
    fn test_match_top_experts() {
        let mut registry = std::collections::HashMap::new();
        let exp1 = make_test_expert("exp-arch", "架构师", "系统架构", vec!["architecture", "backend"], vec!["Rust", "Go"]);
        let exp2 = make_test_expert("exp-ai", "AI专家", "人工智能", vec!["ai", "ml"], vec!["PyTorch", "TensorFlow"]);
        let exp3 = make_test_expert("exp-data", "数据专家", "数据工程", vec!["data", "database"], vec!["PostgreSQL", "Spark"]);
        registry.insert(exp1.id.clone(), exp1.clone());
        registry.insert(exp2.id.clone(), exp2.clone());
        registry.insert(exp3.id.clone(), exp3.clone());

        // 匹配架构相关
        let matched = match_top_experts("微服务架构设计 Rust", &registry, 5, 0.0, None);
        assert!(!matched.is_empty());
        // 架构师应排第一
        assert_eq!(matched[0].0.id, "exp-arch");
        assert!(matched[0].1 > 0.0);

        // 领域过滤
        let ai_only = match_top_experts("机器学习", &registry, 5, 0.0, Some("ai"));
        assert_eq!(ai_only.len(), 1);
        assert_eq!(ai_only[0].0.id, "exp-ai");

        // max_n 限制
        let limited = match_top_experts("专家", &registry, 2, 0.0, None);
        assert!(limited.len() <= 2);

        // 阈值过滤
        let thresholded = match_top_experts("完全不相关的查询xyz123", &registry, 5, 0.9, None);
        assert!(thresholded.is_empty(), "高阈值下应无匹配");
    }

    // 测试 7：fuse_answers 空输入边界
    #[test]
    fn test_fuse_answers_empty() {
        let empty: Vec<(ExpertDescriptor, Value, f64)> = Vec::new();
        let fused = fuse_answers(&empty);
        assert_eq!(fused["consensus_score"], 0.0);
        assert_eq!(fused["confidence"], 0.0);
        assert!(fused["alternative_views"].as_array().unwrap().is_empty());
    }

    // 测试 8：generate_expert_answer 不同专家生成不同内容
    #[test]
    fn test_generate_expert_answer_diversity() {
        let exp_arch = make_test_expert("exp-a", "架构师", "系统架构", vec!["architecture"], vec!["Rust"]);
        let exp_ai = make_test_expert("exp-b", "AI专家", "人工智能", vec!["ai"], vec!["PyTorch"]);

        let answer_arch = generate_expert_answer_template(&exp_arch, "技术选型");
        let answer_ai = generate_expert_answer_template(&exp_ai, "技术选型");

        // 不同专家的分析应包含各自的名称和领域
        assert!(answer_arch["analysis"].as_str().unwrap().contains("架构师"));
        assert!(answer_ai["analysis"].as_str().unwrap().contains("AI专家"));
        assert!(answer_arch["solution"].as_str().unwrap().contains("Rust"));
        assert!(answer_ai["solution"].as_str().unwrap().contains("PyTorch"));
    }

    // 测试：真实 LLM 接入包装器（无论走 LLM 还是降级模板，均返回有效结构、不 panic）
    #[tokio::test]
    async fn test_generate_expert_answer_llm_fallback() {
        let expert = make_test_expert(
            "exp-llm",
            "架构师·LLM",
            "系统架构",
            vec!["architecture"],
            vec!["Rust"],
        );
        let answer = generate_expert_answer(&expert, "如何设计微服务架构？").await;
        // 结构契约必须成立（LLM 路径含 source="llm"，模板降级路径则不含）
        let analysis = answer["analysis"].as_str().expect("analysis 应为字符串");
        let solution = answer["solution"].as_str().expect("solution 应为字符串");
        assert!(!analysis.is_empty(), "analysis 不应为空");
        assert!(!solution.is_empty(), "solution 不应为空");
        let confidence = answer["confidence"].as_f64().expect("confidence 应为数字");
        assert!((0.0..=1.0).contains(&confidence), "confidence 应在 0-1 之间");
        // 任一路径均可：模板降级（含专家名）或真实 LLM（source=llm）
        let ok = analysis.contains("架构师·LLM") || answer.get("source").and_then(|v| v.as_str()) == Some("llm");
        assert!(ok, "应走模板降级或真实 LLM 路径");
    }
}
