// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 引擎主编排：开发专家联盟处理模式 + code agent 式自修复循环。
//!
//! 单轮：Intent → Build → Solve → Team → Diagnose → Verdict → Generate → Gate
//! 收敛：若裁决/闸门未过，[`repair`] 将诊断意见映射为图突变后**重入 Solve 轮**
//! （最多 `max_repair_rounds` 轮），全轨迹以 [`Stage::Refine`] 事件记录——
//! 这正是 2026 年主流 code agent「生成→验证→失败→修复→重试」的自修复闭环。
//! 交付时若调用方给出基线文件（[`crate::patch`]），已有文件产出 SEARCH/REPLACE
//! 最小补丁而非全量覆写（diff-first）。

use crate::case::{CaseBook, CodeCase};
use crate::experts::{self, DiagContext, ExpertOpinion};
use crate::gate::{self, GateReport};
use crate::graph::build_graph;
use crate::ir::{parse_requirement, Requirement};
use crate::patch::{self, EditBlock};
use crate::repair;
use crate::verdict::{self, Verdict, VerdictPolicy};
use mox_ai_flow_core::codegen::{self, CodeBundle};
use mox_ai_flow_core::model::FlowGraph;
use mox_ai_flow_core::pipeline::{self, OptimizationReport};
use mox_ai_flow_core::OptimizeConfig;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 联盟处理模式阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Intent,
    Build,
    Solve,
    Team,
    Diagnose,
    Verdict,
    Generate,
    Gate,
    /// 自修复：应用诊断驱动的图突变后重入评审轮
    Refine,
    Deliver,
    Learn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageEvent {
    pub stage: Stage,
    pub ok: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// 自动为敏感/高危步骤注入护栏
    pub auto_guard: bool,
    /// 求解配置（默认不出码——出码统一由引擎在裁决通过后执行）
    pub optimize: OptimizeConfig,
    /// 裁决权重与分数线
    pub policy: VerdictPolicy,
    /// code agent 式自修复循环（默认开）
    pub self_repair: bool,
    /// 自修复最大轮数（含首轮评审，默认 3）
    pub max_repair_rounds: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            auto_guard: true,
            optimize: OptimizeConfig {
                auto_repair: true,
                emit_code: false,
                fast_path_threshold: 0.15,
            },
            policy: VerdictPolicy::default(),
            self_repair: true,
            max_repair_rounds: 3,
        }
    }
}

/// 一轮评审的完整产物
#[derive(Debug, Clone, Serialize)]
struct Round {
    report: OptimizationReport,
    opinions: Vec<ExpertOpinion>,
    verdict: Verdict,
    bundle: Option<CodeBundle>,
    gates: GateReport,
}

/// 引擎运行产物
#[derive(Debug, Clone, Serialize)]
pub struct EngineResult {
    pub requirement: Requirement,
    pub raw_graph: FlowGraph,
    /// 最终轮的优化图（可能已被自修复突变）
    pub working_graph: FlowGraph,
    pub report: OptimizationReport,
    pub opinions: Vec<ExpertOpinion>,
    pub verdict: Verdict,
    pub bundle: Option<CodeBundle>,
    /// 已存在基线文件的 SEARCH/REPLACE 补丁（diff-first，仅当调用方提供基线）
    pub patches: Vec<EditBlock>,
    pub gates: GateReport,
    pub delivered: bool,
    /// 评审轮数（1 = 首轮即收敛）
    pub rounds: usize,
    /// 全部轮次应用的自修复动作
    pub repairs: Vec<String>,
    pub events: Vec<StageEvent>,
    pub case: CodeCase,
}

impl EngineResult {
    /// 交付摘要（人类可读）
    pub fn digest(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "== 代码引擎 · 需求「{}」({} 步 / {} 轮评审) ==\n",
            self.requirement.name,
            self.requirement.steps.len(),
            self.rounds
        ));
        s.push_str(&self.report.summary());
        for op in &self.opinions {
            s.push_str(&format!("  {}({}): score={:.2}\n", op.expert, op.role, op.score));
        }
        s.push_str(&format!("  {}\n", self.verdict.summary));
        for r in &self.repairs {
            s.push_str(&format!("  自修复: {r}\n"));
        }
        s.push_str(&format!("  闸门: {}\n", self.gates.summary()));
        s.push_str(&format!(
            "  交付: {}\n",
            match &self.bundle {
                Some(b) if self.delivered => format!(
                    "是（{} 文件 / {} 行 / {} 个增量补丁）",
                    b.files.len(),
                    b.total_lines(),
                    self.patches.len()
                ),
                _ => "否".into(),
            }
        ));
        s
    }
}

/// 全自研 AI 代码引擎
#[derive(Default)]
pub struct CodeEngine {
    pub config: EngineConfig,
    pub book: CaseBook,
    counter: u64,
}

impl CodeEngine {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            book: CaseBook::default(),
            counter: 0,
        }
    }

    /// 以当前配置运行一条需求文本（自动生成需求 id）。
    pub fn run(&mut self, text: &str) -> EngineResult {
        self.counter += 1;
        let id = format!("req-{:04}", self.counter);
        let name: String = text.chars().take(20).collect();
        self.run_named(&id, &name, text)
    }

    /// 带基线文件的运行：交付时对已有文件产出 SEARCH/REPLACE 最小补丁。
    pub fn run_with_context(
        &mut self,
        id: &str,
        name: &str,
        text: &str,
        known_files: &BTreeMap<String, String>,
    ) -> EngineResult {
        let mut r = self.run_named(id, name, text);
        if let Some(bundle) = &r.bundle {
            r.patches = patch::diff_against(bundle, known_files);
        }
        r
    }

    /// 全链路执行入口（含自修复收敛循环）。
    pub fn run_named(&mut self, id: &str, name: &str, text: &str) -> EngineResult {
        let mut events: Vec<StageEvent> = Vec::new();

        // ① Intent
        let req = parse_requirement(id, name, text);
        events.push(StageEvent {
            stage: Stage::Intent,
            ok: !req.steps.is_empty(),
            note: format!("解析出 {} 个步骤", req.steps.len()),
        });

        // ② Build
        let raw = build_graph(&req, self.config.auto_guard);
        events.push(StageEvent {
            stage: Stage::Build,
            ok: true,
            note: format!("建图 {} 节点 / {} 边", raw.nodes.len(), raw.edges.len()),
        });

        // 评审-修复循环（③~⑧ 为每轮内容）
        let mut working = raw.clone();
        let mut repairs: Vec<String> = Vec::new();
        let max_rounds = self.config.max_repair_rounds.max(1);
        let mut round_idx = 0usize;
        let round = loop {
            round_idx += 1;
            let r = self.review_round(&req, &raw, &working, &mut events);
            let converged = r.verdict.approved && r.gates.passed;
            if converged || !self.config.self_repair || round_idx >= max_rounds {
                break (r, working.clone());
            }
            let outcome = repair::repair_graph(&mut working, &r.opinions);
            if !outcome.changed() {
                break (r, working.clone());
            }
            repairs.extend(outcome.applied.iter().cloned());
            events.push(StageEvent {
                stage: Stage::Refine,
                ok: true,
                note: format!(
                    "第 {round_idx} 轮未收敛，应用 {} 项自修复后重入评审：{}",
                    outcome.applied.len(),
                    outcome.applied.join("；")
                ),
            });
        };
        let (round, working_graph) = round;
        let rounds = round_idx;

        // ⑨ Deliver
        let delivered = round.verdict.approved && round.gates.passed;
        events.push(StageEvent {
            stage: Stage::Deliver,
            ok: delivered,
            note: if delivered {
                format!("代码包交付（{rounds} 轮收敛）")
            } else {
                "拒绝交付（裁决或闸门未过）".into()
            },
        });

        // ⑩ Learn
        let mut result = EngineResult {
            requirement: req,
            raw_graph: raw,
            working_graph,
            report: round.report,
            opinions: round.opinions,
            verdict: round.verdict,
            bundle: round.bundle,
            patches: Vec::new(),
            gates: round.gates,
            delivered,
            rounds,
            repairs,
            events,
            case: CodeCase {
                req_id: id.into(),
                req_name: name.into(),
                steps: 0,
                verdict_score: 0.0,
                approved: false,
                vetoed: false,
                gates_passed: false,
                delivered: false,
                speedup: 0.0,
                files: 0,
                lines: 0,
                rounds: 0,
                repairs_applied: 0,
            },
        };
        result.case = CaseBook::record(&result);
        let rate_before = self.book.delivery_rate();
        self.book.push(result.case.clone());
        result.events.push(StageEvent {
            stage: Stage::Learn,
            ok: true,
            note: format!("案例入库，历史交付率 {:.0}%", rate_before * 100.0),
        });
        result
    }

    /// 单轮：Solve → Team → Diagnose → Verdict → Generate → Gate
    fn review_round(
        &self,
        req: &Requirement,
        raw: &FlowGraph,
        working: &FlowGraph,
        events: &mut Vec<StageEvent>,
    ) -> Round {
        // Solve（并行化→冲突→修复→CPM/RCPSP→算力路由）
        let report = pipeline::optimize(working, &self.config.optimize);
        events.push(StageEvent {
            stage: Stage::Solve,
            ok: true,
            note: format!(
                "加速比 {:.2}x，剪伪依赖 {} 条，冲突 {}/{} 已修复",
                report.gains.speedup,
                report.gains.removed_false_deps,
                report.gains.conflicts_auto_fixed,
                report.gains.conflicts_found
            ),
        });

        // Team + Diagnose
        events.push(StageEvent {
            stage: Stage::Team,
            ok: true,
            note: "analyst / builder / auditor / coordinator 四人评审团就位".into(),
        });
        let ctx = DiagContext {
            req,
            raw: working,
            report: &report,
        };
        let opinions = experts::default_panel(&ctx);
        let findings: usize = opinions.iter().map(|o| o.findings.len()).sum();
        events.push(StageEvent {
            stage: Stage::Diagnose,
            ok: true,
            note: format!("{} 位专家共产出 {} 条意见", opinions.len(), findings),
        });

        // Verdict
        let verdict = verdict::fuse(&opinions, &self.config.policy);
        events.push(StageEvent {
            stage: Stage::Verdict,
            ok: verdict.approved,
            note: verdict.summary.clone(),
        });

        // Generate（裁决通过才出码——C1 红线：未过裁决不得生成）
        let bundle = if verdict.approved {
            let b = codegen::generate(
                &report.optimized_graph,
                &report.plan,
                &report.schedule,
                &report.conflicts,
            );
            events.push(StageEvent {
                stage: Stage::Generate,
                ok: !b.rejected,
                note: if b.rejected {
                    format!("出码被拒: {:?}", b.reject_reasons)
                } else {
                    format!("{} 文件 / {} 行", b.files.len(), b.total_lines())
                },
            });
            Some(b)
        } else {
            events.push(StageEvent {
                stage: Stage::Generate,
                ok: false,
                note: "裁决未通过，跳过出码".into(),
            });
            None
        };

        // Gate（三证守恒）
        let gates = gate::run_gates(raw, &report.optimized_graph, bundle.as_ref());
        events.push(StageEvent {
            stage: Stage::Gate,
            ok: gates.passed,
            note: gates.summary(),
        });

        Round {
            report,
            opinions,
            verdict,
            bundle,
            gates,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_to_end_delivers_clean_requirement() {
        let mut engine = CodeEngine::default();
        let r = engine.run(
            "读取 input.csv 文件；把解析结果写入 db.orders 表；汇总计算 total_amount 字段并导出报表 output.xlsx",
        );
        assert!(r.verdict.approved, "verdict: {}", r.verdict.summary);
        assert!(r.gates.passed, "gates: {}", r.gates.summary());
        assert!(r.delivered);
        assert_eq!(r.rounds, 1, "干净需求应首轮收敛");
        assert!(r.repairs.is_empty());
        // 单轮事件 = Intent Build Solve Team Diagnose Verdict Generate Gate Deliver Learn
        assert_eq!(r.events.len(), 10);
        let bundle = r.bundle.expect("应产出代码包");
        assert!(!bundle.rejected);
        assert!(bundle.files.len() >= 4);
        assert!(r.case.delivered);
    }

    #[test]
    fn sensitive_without_guard_is_vetoed_when_repair_off() {
        let cfg = EngineConfig {
            auto_guard: false,
            self_repair: false,
            ..Default::default()
        };
        let mut engine = CodeEngine::new(cfg);
        let r = engine.run("查询 db.citizen_idcard 表并导出报表 output.xlsx");
        assert!(r.verdict.vetoed, "auditor 应一票否决");
        assert!(!r.delivered);
        assert!(r.bundle.is_none(), "否决后不得出码");
    }

    #[test]
    fn self_repair_loop_converges_to_delivery() {
        // code agent 式自修复：关自动护栏 + 开自修复 → 诊断出缺陷 → 修复 → 二轮收敛
        let cfg = EngineConfig {
            auto_guard: false,
            self_repair: true,
            max_repair_rounds: 3,
            ..Default::default()
        };
        let mut engine = CodeEngine::new(cfg);
        let r = engine.run("查询 db.citizen_idcard 表并导出报表 output.xlsx");
        assert_eq!(r.rounds, 2, "应恰好在第 2 轮收敛");
        assert_eq!(r.repairs.len(), 1, "修复动作: {:?}", r.repairs);
        assert!(r.repairs[0].contains("desensitize"));
        assert!(r.delivered, "自修复后应交付: {}", r.digest());
        assert!(r
            .events
            .iter()
            .any(|e| e.stage == Stage::Refine && e.ok));
        assert!(r
            .working_graph
            .nodes
            .iter()
            .any(|n| n.tags.iter().any(|t| t == "desensitize")));
    }

    #[test]
    fn repair_gives_up_after_max_rounds() {
        // 不可修复缺陷（空需求）：无适用修复规则 → 一轮即止，不空转
        let mut engine = CodeEngine::default();
        let r = engine.run("嗯");
        assert_eq!(r.rounds, 1);
        assert!(!r.delivered);
        assert!(r.repairs.is_empty());
    }

    #[test]
    fn diff_first_patches_for_known_files() {
        let mut engine = CodeEngine::default();
        let mut known: BTreeMap<String, String> = BTreeMap::new();
        // 伪造一个与生成 tasks.py 相近的基线（首行相同以测压缩）
        let first = engine.run("读取 input.csv 文件");
        let base_file = first
            .bundle
            .as_ref()
            .expect("应出码")
            .files
            .iter()
            .find(|f| f.path.ends_with(".py") && !f.content.is_empty())
            .expect("应有非空 py 文件");
        known.insert(base_file.path.clone(), base_file.content.clone());
        let r = engine.run_with_context("req-x", "基线", "读取 input.csv 文件", &known);
        assert!(r.delivered);
        assert_eq!(r.patches.len(), 1, "同名基线应产出 1 个补丁块");
        let p = &r.patches[0];
        assert_eq!(p.path, base_file.path);
        // 内容一致 → 压缩后 search/replace 皆空（无差异），可安全跳过覆写
        assert!(p.search.is_empty() && p.replace.is_empty(), "无差异应为空块");
        // 有差异 → 补丁应用后可还原新内容
        let mut modified = known.clone();
        let v = modified.get_mut(&p.path).unwrap();
        let mut lines: Vec<&str> = v.lines().collect();
        lines.push("# legacy marker");
        *v = lines.join("\n");
        *v = format!("{v}\n");
        let blocks = patch::diff_against(r.bundle.as_ref().unwrap(), &modified);
        let blk = blocks.iter().find(|b| b.path == p.path).expect("差异文件应有块");
        assert!(!blk.search.is_empty() || !blk.replace.is_empty());
        let (files, report) = patch::apply_blocks(&modified, std::slice::from_ref(blk));
        assert!(report.ok(), "补丁应用应全成功: {:?}", report.results);
        assert_eq!(files[&p.path], r.bundle.as_ref().unwrap().file(&p.path).unwrap().content);
    }

    #[test]
    fn empty_requirement_blocked_at_intent() {
        let mut engine = CodeEngine::default();
        let r = engine.run("嗯");
        assert!(!r.events[0].ok, "Intent 阶段应标记失败");
        assert!(!r.delivered);
    }

    #[test]
    fn case_book_accumulates_across_runs() {
        let mut engine = CodeEngine::default();
        engine.run("读取 input.csv 文件");
        engine.run("计算 sum_total 的合");
        assert_eq!(engine.book.cases.len(), 2);
        assert!(engine.book.delivery_rate() > 0.0);
    }
}
