// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 智能驱动信息闭环引擎（八段闭环）。
//!
//! ```text
//! ① 感知 Sense    → 从企业各知识库拉取变更
//! ② 归一 Normalize→ 本体映射 + URN 身份归一
//! ③ 关联 Link     → 建边、跨库实体合并
//! ④ 推理 Reason   → 影响面、热点、需求溯源
//! ⑤ 决策 Decide   → 依据推理结论判定动作（璇玑裁决入口）
//! ⑥ 执行 Act      → 触发自动化动作
//! ⑦ 校验 Verify   → 三重闸门 + 偏离检测
//! ⑧ 沉淀 Persist  → 回写图与快照，进入下一轮感知
//! ```
//!
//! 闭环的意义在于**信息不是被动存储，而是主动驱动动作**：
//! 每一段都留痕（`StageTrace`），使"为什么做了这个决定"可完整复盘。
//! 若校验不通过，闭环在 ⑦ 拦停并回滚决策，绝不让不合规变更沉淀进事实源。

use std::time::Instant;

use mox_platform_graph_core::UnifiedGraph;
use serde::{Deserialize, Serialize};

use crate::govern::{self, GovernanceSummary};
use crate::ingest::{Connector, GraphSink, IngestStat};
use crate::reason;

/// 闭环阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Sense,
    Normalize,
    Link,
    Reason,
    Decide,
    Act,
    Verify,
    Persist,
}

impl Stage {
    pub fn zh(&self) -> &'static str {
        match self {
            Stage::Sense => "感知",
            Stage::Normalize => "归一",
            Stage::Link => "关联",
            Stage::Reason => "推理",
            Stage::Decide => "决策",
            Stage::Act => "执行",
            Stage::Verify => "校验",
            Stage::Persist => "沉淀",
        }
    }
    pub fn code(&self) -> &'static str {
        match self {
            Stage::Sense => "S1",
            Stage::Normalize => "S2",
            Stage::Link => "S3",
            Stage::Reason => "S4",
            Stage::Decide => "S5",
            Stage::Act => "S6",
            Stage::Verify => "S7",
            Stage::Persist => "S8",
        }
    }
}

/// 全部八段，按序
pub const STAGES: [Stage; 8] = [
    Stage::Sense,
    Stage::Normalize,
    Stage::Link,
    Stage::Reason,
    Stage::Decide,
    Stage::Act,
    Stage::Verify,
    Stage::Persist,
];

/// 单段留痕
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageTrace {
    pub stage: Stage,
    pub code: String,
    pub name: String,
    pub ok: bool,
    pub detail: String,
    pub elapsed_ms: u128,
}

/// 决策动作
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// 放行沉淀
    Accept,
    /// 需人工/璇玑复核（有偏离但未越硬闸门）
    Review,
    /// 拦停（闸门失败）
    Reject,
}

impl Decision {
    pub fn zh(&self) -> &'static str {
        match self {
            Decision::Accept => "放行",
            Decision::Review => "转璇玑复核",
            Decision::Reject => "拦停",
        }
    }
}

/// 闭环执行报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopReport {
    pub traces: Vec<StageTrace>,
    pub ingest: Vec<IngestStat>,
    pub decision: Decision,
    pub decision_reason: String,
    pub governance: GovernanceSummary,
    pub hotspots: Vec<reason::Hotspot>,
    /// 是否成功沉淀（Reject 时为 false）
    pub persisted: bool,
    pub node_count: usize,
    pub edge_count: usize,
    pub total_elapsed_ms: u128,
}

/// 闭环配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    pub tenant: String,
    /// 热点取前 N
    pub hotspot_top: usize,
    /// 是否在 Reject 时仍沉淀（默认 false：不合规不入库）
    pub persist_on_reject: bool,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            tenant: crate::urn::DEFAULT_TENANT.to_string(),
            hotspot_top: 10,
            persist_on_reject: false,
        }
    }
}

/// 执行一轮完整闭环。
///
/// `connectors` 为本轮感知到的知识源。返回值包含全部八段留痕，
/// 便于把"智能驱动信息"的每一步都摊开审计。
pub fn run(connectors: &[&dyn Connector], cfg: &LoopConfig) -> LoopReport {
    let t0 = Instant::now();
    let mut traces: Vec<StageTrace> = Vec::new();
    let mut ingest_stats: Vec<IngestStat> = Vec::new();

    // ① 感知
    let s = Instant::now();
    let names: Vec<String> = connectors.iter().map(|c| c.name()).collect();
    traces.push(StageTrace {
        stage: Stage::Sense,
        code: Stage::Sense.code().into(),
        name: Stage::Sense.zh().into(),
        ok: true,
        detail: format!("感知知识源 {} 个：{}", names.len(), names.join(", ")),
        elapsed_ms: s.elapsed().as_millis(),
    });

    // ②③ 归一 + 关联（由 GraphSink 在同一次写入中完成：
    //     本体映射即归一，URN 幂等合并与建边即关联）
    let s = Instant::now();
    let mut sink = GraphSink::new(&cfg.tenant);
    let mut errs: Vec<String> = Vec::new();
    for c in connectors {
        match c.ingest(&mut sink) {
            Ok(st) => ingest_stats.push(st),
            Err(e) => errs.push(format!("{}: {e}", c.name())),
        }
    }
    let nodes_new: usize = ingest_stats.iter().map(|s| s.nodes_new).sum();
    let nodes_merged: usize = ingest_stats.iter().map(|s| s.nodes_merged).sum();
    let edges_new: usize = ingest_stats.iter().map(|s| s.edges_new).sum();
    let dangling: usize = ingest_stats.iter().map(|s| s.edges_dangling).sum();
    let norm_ms = s.elapsed().as_millis();

    traces.push(StageTrace {
        stage: Stage::Normalize,
        code: Stage::Normalize.code().into(),
        name: Stage::Normalize.zh().into(),
        ok: errs.is_empty(),
        detail: if errs.is_empty() {
            format!("归一新增 {nodes_new} 节点，跨源合并 {nodes_merged} 个")
        } else {
            format!("归一存在失败源：{}", errs.join("; "))
        },
        elapsed_ms: norm_ms,
    });
    traces.push(StageTrace {
        stage: Stage::Link,
        code: Stage::Link.code().into(),
        name: Stage::Link.zh().into(),
        ok: true,
        detail: format!("建立关联 {edges_new} 条，悬挂边 {dangling} 条"),
        elapsed_ms: 0,
    });

    let graph = sink.into_graph();

    // ④ 推理
    let s = Instant::now();
    let hotspots = reason::hotspots(&graph, cfg.hotspot_top);
    let isolated_n = reason::isolated(&graph).len();
    traces.push(StageTrace {
        stage: Stage::Reason,
        code: Stage::Reason.code().into(),
        name: Stage::Reason.zh().into(),
        ok: true,
        detail: format!(
            "识别知识热点 {} 个，孤立知识 {isolated_n} 个",
            hotspots.len()
        ),
        elapsed_ms: s.elapsed().as_millis(),
    });

    // ⑦ 校验先行计算（决策依赖校验结论，故先算闸门再定动作）
    let s = Instant::now();
    let governance = govern::summarize(&graph);
    let verify_ms = s.elapsed().as_millis();

    // ⑤ 决策
    let s = Instant::now();
    let (decision, reason_text) = decide(&governance, dangling);
    traces.push(StageTrace {
        stage: Stage::Decide,
        code: Stage::Decide.code().into(),
        name: Stage::Decide.zh().into(),
        ok: decision != Decision::Reject,
        detail: format!("{}：{}", decision.zh(), reason_text),
        elapsed_ms: s.elapsed().as_millis(),
    });

    // ⑥ 执行
    let s = Instant::now();
    let act_detail = match decision {
        Decision::Accept => "无需干预，进入沉淀".to_string(),
        Decision::Review => format!(
            "派发璇玑复核：{} 项偏离待裁决",
            governance.deviation.deviations.len()
        ),
        Decision::Reject => format!("拦停并告警：闸门错误 {} 项", governance.gate.error_count),
    };
    traces.push(StageTrace {
        stage: Stage::Act,
        code: Stage::Act.code().into(),
        name: Stage::Act.zh().into(),
        ok: true,
        detail: act_detail,
        elapsed_ms: s.elapsed().as_millis(),
    });

    traces.push(StageTrace {
        stage: Stage::Verify,
        code: Stage::Verify.code().into(),
        name: Stage::Verify.zh().into(),
        ok: governance.passed,
        detail: format!(
            "闸门{} / 偏离{} / 覆盖率 {:.1}%",
            if governance.gate.passed {
                "通过"
            } else {
                "失败"
            },
            if governance.deviation.passed {
                "通过"
            } else {
                "失败"
            },
            governance.deviation.coverage
        ),
        elapsed_ms: verify_ms,
    });

    // ⑧ 沉淀
    let s = Instant::now();
    let persisted = decision != Decision::Reject || cfg.persist_on_reject;
    traces.push(StageTrace {
        stage: Stage::Persist,
        code: Stage::Persist.code().into(),
        name: Stage::Persist.zh().into(),
        ok: persisted,
        detail: if persisted {
            format!(
                "沉淀 {} 节点 / {} 边为事实源",
                graph.nodes.len(),
                graph.edges.len()
            )
        } else {
            "拦停未沉淀，事实源保持上一致状态".to_string()
        },
        elapsed_ms: s.elapsed().as_millis(),
    });

    LoopReport {
        traces,
        ingest: ingest_stats,
        decision,
        decision_reason: reason_text,
        node_count: graph.nodes.len(),
        edge_count: graph.edges.len(),
        hotspots,
        governance,
        persisted,
        total_elapsed_ms: t0.elapsed().as_millis(),
    }
}

/// 决策规则（按错误性质分级，而非一刀切）：
/// 1. **结构性错误** → `Reject`：图本身不可信（空图 / 悬空边 / 缺证据 / 守恒不自洽），
///    此类图若沉淀会污染事实源，必须拦停。
/// 2. **内容债务 / 偏离 / 悬挂** → `Review`：图结构有效，存在待治理项，
///    仍可沉淀但需璇玑复核——存量债务不应阻断知识接入。
/// 3. 全绿 → `Accept`。
fn decide(g: &GovernanceSummary, dangling: usize) -> (Decision, String) {
    if !g.gate.structural_passed {
        return (
            Decision::Reject,
            format!(
                "结构性闸门未通过（{} 项）：{}",
                g.gate.structural_errors.len(),
                g.gate.structural_errors.join("；")
            ),
        );
    }

    let mut reasons: Vec<String> = Vec::new();
    if !g.gate.debt_errors.is_empty() {
        reasons.push(format!("治理债务 {} 项", g.gate.debt_errors.len()));
    }
    if !g.deviation.passed {
        let hard = g
            .deviation
            .deviations
            .iter()
            .filter(|d| d.code != "GR-E3")
            .count();
        reasons.push(format!(
            "需求对齐覆盖率 {:.1}%、硬偏离 {hard} 项",
            g.deviation.coverage
        ));
    }
    if dangling > 0 {
        reasons.push(format!("悬挂关联 {dangling} 条待补全端点"));
    }

    if reasons.is_empty() {
        return (
            Decision::Accept,
            format!(
                "结构与需求对齐全绿（覆盖率 {:.1}%），知识变更可信",
                g.deviation.coverage
            ),
        );
    }
    (Decision::Review, reasons.join("；"))
}

/// 生成闭环的 Mermaid 流程图——把本轮真实执行结果画出来，而非静态示意图。
pub fn to_mermaid(r: &LoopReport) -> String {
    let mut s = String::from("flowchart LR\n");
    for (i, t) in r.traces.iter().enumerate() {
        let shape = if t.ok { "([%s])" } else { "[/%s/]" };
        let label = format!("{} {}", t.code, t.name);
        let body = shape.replace("%s", &label);
        s.push_str(&format!("  N{i}{body}\n"));
    }
    for i in 0..r.traces.len().saturating_sub(1) {
        s.push_str(&format!("  N{i} --> N{}\n", i + 1));
    }
    s.push_str(&format!(
        "  N{} -.->|下一轮| N0\n",
        r.traces.len().saturating_sub(1)
    ));
    s
}

/// 供外部构图后直接治理：不经连接器，直接对既有图跑校验+决策。
pub fn verify_graph(graph: &UnifiedGraph) -> (Decision, GovernanceSummary) {
    let g = govern::summarize(graph);
    let (d, _) = decide(&g, 0);
    (d, g)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{InfoGraphConnector, KnowledgeBaseConnector, KnowledgeItem};

    /// 完全对齐的知识源：REQ 绑定到实现，应放行
    fn aligned_json() -> &'static str {
        r#"{
          "nodes": [
            {"id":"Requirement:D01","kind":"Requirement","name":"D01 算子内核","path":"REQ/D01","summary":"算子内核与执行","external":false},
            {"id":"CodeFile:crates/operator-core/src/lib.rs","kind":"CodeFile","name":"lib.rs","path":"crates/operator-core/src/lib.rs","summary":"","external":false}
          ],
          "edges": [
            {"id":"e1","from":"Requirement:D01","to":"CodeFile:crates/operator-core/src/lib.rs","kind":"Bind","label":"六维绑定","evidence":"guantu.req.json"}
          ]
        }"#
    }

    /// 含未溯源核心代码：应转复核
    fn deviated_json() -> &'static str {
        r#"{
          "nodes": [
            {"id":"Requirement:D01","kind":"Requirement","name":"D01","path":"REQ/D01","summary":"","external":false},
            {"id":"CodeFile:a.rs","kind":"CodeFile","name":"a.rs","path":"a.rs","summary":"","external":false},
            {"id":"CodeFile:orphan.rs","kind":"CodeFile","name":"orphan.rs","path":"orphan.rs","summary":"","external":false}
          ],
          "edges": [
            {"id":"e1","from":"Requirement:D01","to":"CodeFile:a.rs","kind":"Bind","label":"bind","evidence":"x"}
          ]
        }"#
    }

    #[test]
    fn eight_stages_all_execute_in_order() {
        let c = InfoGraphConnector::from_str(aligned_json());
        let r = run(&[&c], &LoopConfig::default());
        assert_eq!(r.traces.len(), 8, "八段闭环必须全部留痕");
        for (i, st) in STAGES.iter().enumerate() {
            assert_eq!(r.traces[i].stage, *st, "第 {i} 段顺序错误");
        }
    }

    #[test]
    fn aligned_source_is_accepted_and_persisted() {
        let c = InfoGraphConnector::from_str(aligned_json());
        let r = run(&[&c], &LoopConfig::default());
        assert_eq!(
            r.decision,
            Decision::Accept,
            "全绿应放行: {}",
            r.decision_reason
        );
        assert!(r.persisted);
        assert_eq!(r.governance.deviation.coverage, 100.0);
        assert_eq!(r.node_count, 2);
        assert_eq!(r.edge_count, 1);
    }

    #[test]
    fn deviated_source_goes_to_expert_review() {
        let c = InfoGraphConnector::from_str(deviated_json());
        let r = run(&[&c], &LoopConfig::default());
        assert_eq!(r.decision, Decision::Review, "50% 覆盖率应转复核");
        // 转复核仍沉淀：偏离是待治理债务，不是非法数据
        assert!(r.persisted);
        assert_eq!(r.governance.deviation.coverage, 50.0);
        let s5 = r.traces.iter().find(|t| t.stage == Stage::Decide).unwrap();
        assert!(s5.detail.contains("复核"));
    }

    #[test]
    fn multi_source_ingest_merges_and_all_stats_recorded() {
        let c1 = InfoGraphConnector::from_str(aligned_json());
        let c2 = KnowledgeBaseConnector {
            source: "wiki".into(),
            items: vec![KnowledgeItem {
                key: "kb/x.md".into(),
                title: "X".into(),
                body: "说明".into(),
                kind: None,
                evidence: String::new(),
                refs: vec![],
            }],
        };
        let r = run(&[&c1, &c2], &LoopConfig::default());
        assert_eq!(r.ingest.len(), 2, "每个知识源都必须有独立统计");
        assert_eq!(r.ingest[0].source, c1.name());
        assert_eq!(r.ingest[1].source, "wiki");
        assert_eq!(r.node_count, 3);
    }

    #[test]
    fn sense_stage_lists_all_sources() {
        let c1 = InfoGraphConnector::from_str(aligned_json());
        let r = run(&[&c1], &LoopConfig::default());
        let s1 = &r.traces[0];
        assert_eq!(s1.stage, Stage::Sense);
        assert!(s1.detail.contains("info-graph"));
    }

    #[test]
    fn malformed_source_marks_normalize_failed_without_panic() {
        let bad = InfoGraphConnector::from_str("{ this is not json");
        let r = run(&[&bad], &LoopConfig::default());
        let s2 = r
            .traces
            .iter()
            .find(|t| t.stage == Stage::Normalize)
            .unwrap();
        assert!(!s2.ok, "非法源必须标记归一失败");
        assert!(s2.detail.contains("失败"));
        // 单源失败不应导致 panic，闭环仍需走完八段
        assert_eq!(r.traces.len(), 8);
    }

    #[test]
    fn empty_run_is_rejected_not_panicking() {
        let r = run(&[], &LoopConfig::default());
        // 空图触发 G1（结构性）——空事实源不可沉淀，拦停是正确行为
        assert_eq!(r.traces.len(), 8, "即使拦停也必须走完八段留痕");
        assert_eq!(r.node_count, 0);
        assert_eq!(r.decision, Decision::Reject);
        assert!(!r.persisted, "拦停不得沉淀");
        assert!(r.decision_reason.contains("结构性"));
    }

    #[test]
    fn dead_code_is_debt_not_structural_rejection() {
        // orphan.rs 是无任何关联的核心代码（G5 孤儿）+ 无需求溯源（GR-E6）。
        // 这是治理债务，绝不能因此拒收整批知识——否则真实代码库永远无法接入。
        let c = InfoGraphConnector::from_str(deviated_json());
        let r = run(&[&c], &LoopConfig::default());
        assert_eq!(r.decision, Decision::Review);
        assert!(r.persisted, "债务应沉淀待治理，而非拒收");
        assert!(
            r.governance.gate.structural_passed,
            "死代码不构成结构性错误"
        );
        assert!(
            !r.governance.gate.debt_errors.is_empty(),
            "但必须作为债务显式暴露"
        );
    }

    #[test]
    fn missing_edge_evidence_is_structural_rejection() {
        // 边缺 evidence（G4）= 未证实的关系，属结构性错误，必须拦停
        let json = r#"{
          "nodes": [
            {"id":"Requirement:D01","kind":"Requirement","name":"D01","path":"REQ/D01","summary":"","external":false},
            {"id":"CodeFile:a.rs","kind":"CodeFile","name":"a.rs","path":"a.rs","summary":"","external":false}
          ],
          "edges": [
            {"id":"e1","from":"Requirement:D01","to":"CodeFile:a.rs","kind":"Bind","label":"","evidence":""}
          ]
        }"#;
        let r = run(
            &[&InfoGraphConnector::from_str(json)],
            &LoopConfig::default(),
        );
        assert_eq!(r.decision, Decision::Reject, "未证实关系必须拦停");
        assert!(!r.persisted);
        assert!(r
            .governance
            .gate
            .structural_errors
            .iter()
            .any(|e| e.starts_with("G4")));
    }

    #[test]
    fn mermaid_renders_all_stages_and_loops_back() {
        let c = InfoGraphConnector::from_str(aligned_json());
        let r = run(&[&c], &LoopConfig::default());
        let m = to_mermaid(&r);
        assert!(m.starts_with("flowchart LR"));
        for st in STAGES {
            assert!(m.contains(st.zh()), "缺少阶段 {}", st.zh());
        }
        assert!(m.contains("下一轮"), "必须体现闭环回流");
    }

    #[test]
    fn verify_graph_shortcut_matches_full_loop() {
        let c = InfoGraphConnector::from_str(aligned_json());
        let full = run(&[&c], &LoopConfig::default());
        let mut sink = GraphSink::new("default");
        c.ingest(&mut sink).unwrap();
        let (d, g) = verify_graph(sink.graph());
        assert_eq!(d, full.decision);
        assert_eq!(g.node_count, full.governance.node_count);
    }

    #[test]
    fn stage_codes_are_unique_s1_to_s8() {
        let codes: std::collections::HashSet<&str> = STAGES.iter().map(|s| s.code()).collect();
        assert_eq!(codes.len(), 8);
        assert!(codes.contains("S1") && codes.contains("S8"));
    }
}
