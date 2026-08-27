// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 代码骨架 · 由关联图谱自动生成（mox_flow_primiflow_svc::assoc::primiflow_seed）
//! 溯源链路: R1 → F4 → B1 → A1 → T1 → C1
//! 数据设计: S1(Project), S6(TraceLink)
//! 说明: 状态机编排：需求→拓扑→文档，并写六维溯源。
//! 规格: primiflow/SPEC.md（§7 模块 / §10 DoD）

use crate::gen::c2::{RegularizeOutput, Scheduler};
use crate::gen::c3::AssetService;
use crate::gen::c4::{Domain, StructuredRequirement, TopologyOperator};
use crate::gen::c5::{DocContext, DocGenerator};
use crate::gen::c6::{SmokeReport, SmokeTester};
use crate::gen::c7::CanvasState;
use crate::gen::c8::AsrClient;
use crate::gen::schema::{Artifact, Asset, Project, Topology, TopologyStatus, TraceLink};
/// 依赖模块: C4, C2, C3
use mox_ai_flow_svc::model::FlowGraph;
use mox_ai_flow_svc::primitive::{KnowledgeBase, PrimitiveState, ResourceBudget};

/// 主链路输入：文本需求或音频路径
#[derive(Debug, Clone)]
pub enum Input {
    Text(String),
    Audio(String),
}

/// 编排结果状态
#[derive(Debug, Clone, PartialEq)]
pub enum OrchestrationStatus {
    /// 主链路全流程跑通，资产已冻结
    Completed,
    /// 超域（白名单外）需求，显式拒绝
    RejectedDomain,
    /// 冒烟校验未通过（幻觉/矛盾），回写对话重生成
    SmokeFailed,
    /// 主链路跑通但资产**暂缓冻结**（`FreezePolicy::deferred`）：
    /// 等待外部治理闸门（如 primiflow-fusion 的 full_gate）确认后再补冻结。
    /// 这是「闸门消费约束」的底层支撑：冻结不再先于闸门发生。
    CompletedPendingGate,
}

/// 一次编排的完整产出（用于画布渲染 + DocViewer + 资产库）
#[derive(Debug)]
pub struct OrchestrationResult {
    pub status: OrchestrationStatus,
    pub requirement: Option<StructuredRequirement>,
    pub state: Option<PrimitiveState>,
    pub regularize: Option<RegularizeOutput>,
    /// 最终合规拓扑（JSON 可直接喂画布）
    pub graph: Option<FlowGraph>,
    pub topology_record: Option<Topology>,
    pub trace_link: Option<TraceLink>,
    pub artifacts: Vec<Artifact>,
    pub smoke: Option<SmokeReport>,
    /// 冻结的资产 id（若有）
    pub frozen_asset_id: Option<uuid::Uuid>,
    /// κ 复用检索命中（第二次同类需求应有）
    pub reuse_hits: Vec<String>,
}

impl OrchestrationResult {
    pub fn summary(&self) -> String {
        match self.status {
            OrchestrationStatus::Completed => {
                let nodes = self.graph.as_ref().map(|g| g.nodes.len()).unwrap_or(0);
                let docs = self.artifacts.len();
                format!(
                    "✅ 主链路完成：拓扑 {} 节点 / {} 文档 / 复用命中 {} / 资产 {}",
                    nodes,
                    docs,
                    self.reuse_hits.len(),
                    self.frozen_asset_id.is_some()
                )
            }
            OrchestrationStatus::RejectedDomain => "⛔ 需求超出业务软件域白名单，已拒绝".into(),
            OrchestrationStatus::SmokeFailed => "⚠️ 冒烟校验未通过，已回写对话重生成".into(),
            OrchestrationStatus::CompletedPendingGate => {
                let nodes = self.graph.as_ref().map(|g| g.nodes.len()).unwrap_or(0);
                let docs = self.artifacts.len();
                format!(
                    "⏸ 主链路完成（待闸门确认）：拓扑 {} 节点 / {} 文档 / 资产暂缓冻结",
                    nodes, docs
                )
            }
        }
    }
}

/// 资产冻结时机策略（验证系统「闸门消费约束」的开关）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FreezePolicy {
    /// 冒烟通过即冻结（历史行为，兼容直连调用方）
    #[default]
    Immediate,
    /// 冒烟通过后暂缓冻结，等外部治理闸门确认（`confirm_gate`）再真正落库；
    /// 闸门未通过则永久不冻结、知识不回灌。
    Deferred,
}

/// 编排器：状态机把八个模块串成主链路闭环
pub struct Orchestrator {
    asr: AsrClient,
    topo_op: TopologyOperator,
    scheduler: Scheduler,
    assets: AssetService,
    smoke: SmokeTester,
    doc_gen: DocGenerator,
    canvas: CanvasState,
    /// 跨多次运行累积的 κ‑τ 引擎（知识沉淀后复用）
    engine: mox_ai_flow_svc::primitive::PrimiEngine,
    /// 预算基准 C
    c_base: f64,
    /// 冻结时机策略（默认 Immediate，融合平台应设为 Deferred）
    freeze_policy: FreezePolicy,
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl Orchestrator {
    pub fn new() -> Self {
        let c_base = 10.0;
        Self {
            asr: AsrClient::new(),
            topo_op: TopologyOperator::new(),
            scheduler: Scheduler::new(),
            assets: AssetService::new(),
            smoke: SmokeTester::new(),
            doc_gen: DocGenerator::new(),
            canvas: CanvasState::new(),
            engine: mox_ai_flow_svc::primitive::PrimiEngine::new(
                c_base,
                KnowledgeBase::new(),
                ResourceBudget {
                    total_ms: 1_000_000,
                    per_pool: Default::default(),
                },
            ),
            c_base,
            freeze_policy: FreezePolicy::default(),
        }
    }

    /// 设置冻结时机策略（`Deferred` 表示等外部治理闸门确认后再冻结）
    pub fn set_freeze_policy(&mut self, policy: FreezePolicy) {
        self.freeze_policy = policy;
    }

    /// 主链路：解析需求 → 涌现拓扑 → ℛ̂ → 资产复用 → 冒烟 → 文档 → 六维溯源 → 资产冻结
    pub fn run(&mut self, project: &Project, input: &Input, slider_s: f64) -> OrchestrationResult {
        // 1) 输入模态归一（语音→文本）
        let text = match input {
            Input::Text(t) => t.clone(),
            Input::Audio(path) => match self.asr.asr_transcribe(path) {
                Ok(r) => r.text,
                Err(_) => {
                    return OrchestrationResult {
                        status: OrchestrationStatus::SmokeFailed,
                        requirement: None,
                        state: None,
                        regularize: None,
                        graph: None,
                        topology_record: None,
                        trace_link: None,
                        artifacts: Vec::new(),
                        smoke: None,
                        frozen_asset_id: None,
                        reuse_hits: Vec::new(),
                    };
                }
            },
        };

        // 2) 需求结构化
        let structured = self
            .topo_op
            .structure_requirement(&text, uuid::Uuid::new_v4().to_string());

        // 3) 域白名单：超域显式拒绝
        if !structured.is_in_scope() {
            return OrchestrationResult {
                status: OrchestrationStatus::RejectedDomain,
                requirement: Some(structured),
                state: None,
                regularize: None,
                graph: None,
                topology_record: None,
                trace_link: None,
                artifacts: Vec::new(),
                smoke: None,
                frozen_asset_id: None,
                reuse_hits: Vec::new(),
            };
        }

        // 4) κ‑τ 状态：滑块映射 + 历史资产复用压力
        let mut state = self.scheduler.from_slider(slider_s, self.c_base, 0.0);

        // 5) κ 复用检索：用已冻结资产做语义召回（第二次同类需求命中）
        let reuse = self.assets.search(&text, Domain::BusinessSoftware, 3);
        let reuse_hits: Vec<String> = reuse.iter().map(|h| h.asset.name.clone()).collect();

        // 6) 拓扑涌现：用历史资产沉淀的知识库驱动 SubFlow 复用
        let kb = self.assets.to_knowledge_base();
        let candidate = self.topo_op.emerge_topology(&structured.primi, &state, &kb);

        // 7) ℛ̂ 正则化
        let budget = ResourceBudget {
            total_ms: 1_000_000,
            per_pool: Default::default(),
        };
        let mut reg = self
            .scheduler
            .regularize(candidate.graph.clone(), state.clone(), budget);
        state = reg.state.clone();
        reg.state.q = self.engine.state.q; // 继承累积拓扑荷

        // 8) 冒烟校验（幻觉兜底，失败回写重生成，绝不静默放行）
        let smoke = self.smoke.smoke_test(&reg.graph);

        // 9) 写六维溯源
        let trace_link = self.bind_trace(project, &structured);

        let mut result = OrchestrationResult {
            status: OrchestrationStatus::Completed,
            requirement: Some(structured.clone()),
            state: Some(state.clone()),
            regularize: Some(reg.clone()),
            graph: Some(reg.graph.clone()),
            topology_record: None,
            trace_link: Some(trace_link.clone()),
            artifacts: Vec::new(),
            smoke: Some(smoke.clone()),
            frozen_asset_id: None,
            reuse_hits: reuse_hits.clone(),
        };

        if !smoke.ok {
            // 冒烟失败：标记 rejected，不冻结资产
            let mut topo_rec = Topology::new(
                project.id,
                state.kappa,
                state.tau,
                state.c,
                reg.delta,
                serde_json::to_string(&reg.graph).unwrap_or_default(),
            );
            topo_rec.set_status(TopologyStatus::Rejected);
            result.topology_record = Some(topo_rec);
            result.status = OrchestrationStatus::SmokeFailed;
            return result;
        }

        // 10) 生成 8 份文档
        let topo_rec = Topology::new(
            project.id,
            state.kappa,
            state.tau,
            state.c,
            reg.delta,
            serde_json::to_string(&reg.graph).unwrap_or_default(),
        );
        let links = vec![trace_link.clone()];
        let ctx = DocContext {
            project,
            topology: &topo_rec,
            graph: &reg.graph,
            trace_links: &links,
        };
        let artifacts = self.doc_gen.generate_docs(&ctx);

        // 11) 资产冻结 Q + 12) 知识回灌：时机由冻结策略决定
        //     Immediate：冒烟通过即冻结（历史行为）
        //     Deferred ：先返回 CompletedPendingGate，冻结延迟到外部治理闸门
        //                确认（confirm_gate）之后 —— 闸门未通过则永不冻结。
        if self.freeze_policy == FreezePolicy::Deferred {
            let mut topo_rec = topo_rec;
            topo_rec.set_status(TopologyStatus::Regularized);
            result.topology_record = Some(topo_rec);
            result.artifacts = artifacts;
            result.status = OrchestrationStatus::CompletedPendingGate;
            return result;
        }
        let (frozen_rec, asset) =
            self.freeze_and_accept(topo_rec, structured.name.clone(), &reg, &state);
        result.topology_record = Some(frozen_rec);
        result.artifacts = artifacts;
        result.frozen_asset_id = Some(asset.id);
        result
    }

    /// 延迟冻结确认（「闸门消费约束」的对外接口）：
    /// 仅当外部治理闸门通过时才真正冻结资产 + 回灌 κ‑τ 引擎；
    /// 闸门未通过则把拓扑标记为 Rejected，**永不冻结、不回灌**，
    /// 防止未通过全局闸门的拓扑污染复用资产库。
    ///
    /// 返回 `true` 表示资产已冻结；`false` 表示被闸门拦截（或结果非待闸门状态）。
    pub fn confirm_gate(&mut self, result: &mut OrchestrationResult, gate_passed: bool) -> bool {
        if result.status != OrchestrationStatus::CompletedPendingGate {
            return result.frozen_asset_id.is_some();
        }
        if !gate_passed {
            if let Some(rec) = result.topology_record.as_mut() {
                rec.set_status(TopologyStatus::Rejected);
            }
            return false;
        }
        let (Some(reg), Some(topo), Some(state)) = (
            result.regularize.clone(),
            result.topology_record.take(),
            result.state.clone(),
        ) else {
            return false;
        };
        let name = result
            .requirement
            .as_ref()
            .map(|r| r.name.clone())
            .unwrap_or_else(|| "未命名拓扑".into());
        let (frozen_rec, asset) = self.freeze_and_accept(topo, name, &reg, &state);
        result.topology_record = Some(frozen_rec);
        result.frozen_asset_id = Some(asset.id);
        result.status = OrchestrationStatus::Completed;
        true
    }

    /// 冻结资产 Q + 回灌 κ‑τ 引擎（Immediate / Deferred 共用的两件套）
    fn freeze_and_accept(
        &mut self,
        topo_rec: Topology,
        name: String,
        reg: &RegularizeOutput,
        state: &PrimitiveState,
    ) -> (Topology, Asset) {
        let (asset, _charge) =
            self.assets
                .freeze_asset(topo_rec.id, name, Domain::BusinessSoftware, &reg.graph);
        let emer = mox_ai_flow_svc::primitive::EmergenceResult {
            topology: mox_ai_flow_svc::primitive::CandidateTopology {
                graph: reg.graph.clone(),
                reused_subtasks: Vec::new(),
                explored_subtasks: Vec::new(),
                fanout: 1,
            },
            state: state.clone(),
            validation: mox_ai_flow_svc::primitive::ValidationReport {
                ok: true,
                violations: Vec::new(),
            },
            charge_estimate: 1.0,
            attempts: 0,
            status: mox_ai_flow_svc::primitive::EmergeStatus::Validated {
                regularized: reg.regularized,
            },
        };
        self.engine
            .accept(&emer, mox_ai_flow_svc::primitive::Outcome::Success { quality: 0.9 });

        let mut topo_rec = topo_rec;
        topo_rec.set_status(TopologyStatus::Frozen);
        (topo_rec, asset)
    }

    /// 六维溯源绑定：R → F → B → A → T → C
    pub fn bind_trace(&self, project: &Project, req: &StructuredRequirement) -> TraceLink {
        let requirement_id = req.id.clone();
        let feature_id = format!("F:{}", project.id);
        let business_id = format!("B:{}", Domain::BusinessSoftware.as_str());
        let algorithm_id = "A:kappa-tau-conservation".to_string();
        // 任务维度：取首个可执行子任务 id
        let task_id = req
            .primi
            .subtasks
            .first()
            .map(|s| s.id.clone())
            .unwrap_or_else(|| "T:root".to_string());
        // 代码维度：本编排层入口模块 C1
        let code_id = "C1".to_string();
        TraceLink::new(
            project.id,
            requirement_id,
            feature_id,
            business_id,
            algorithm_id,
            task_id,
            code_id,
        )
    }

    /// 暴露内部画布状态（编辑后重算 ℛ̂ 用）
    pub fn canvas_mut(&mut self) -> &mut CanvasState {
        &mut self.canvas
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_to_end_text_requirement_completes() {
        let project = Project::new("电商分析平台", Some("t1".into()), 0.7, 0.3, 1.0);
        let mut orch = Orchestrator::new();
        let r = orch.run(
            &project,
            &Input::Text("请抓取销售数据。清洗对账。生成图表报告。".into()),
            0.2,
        );
        assert_eq!(
            r.status,
            OrchestrationStatus::Completed,
            "主链路应跑通: {:?}",
            r.smoke
        );
        assert!(r.graph.is_some());
        assert_eq!(r.artifacts.len(), 8, "应生成 8 份文档");
        assert!(r.frozen_asset_id.is_some(), "应冻结资产");
        assert!(
            r.trace_link.as_ref().unwrap().is_complete(),
            "六维溯源应完整"
        );
    }

    #[test]
    fn out_of_domain_is_rejected() {
        let project = Project::new("诗歌生成", Some("t1".into()), 0.7, 0.3, 1.0);
        let mut orch = Orchestrator::new();
        let r = orch.run(&project, &Input::Text("帮我写一首关于春天的诗".into()), 0.2);
        assert_eq!(r.status, OrchestrationStatus::RejectedDomain);
    }

    #[test]
    fn second_similar_requirement_reuses_asset() {
        let project = Project::new("分析平台", Some("t1".into()), 0.7, 0.3, 1.0);
        let mut orch = Orchestrator::new();
        let r1 = orch.run(
            &project,
            &Input::Text("请抓取销售数据。清洗对账。生成图表报告。".into()),
            0.2,
        );
        assert_eq!(r1.status, OrchestrationStatus::Completed);
        assert!(r1.frozen_asset_id.is_some());

        // 第二次同类需求：应检索到首次冻结的资产并复用（κ 复用）
        let r2 = orch.run(
            &project,
            &Input::Text("我想做一份电商经营分析报告，需要销售数据抓取和图表生成。".into()),
            0.1,
        );
        assert_eq!(r2.status, OrchestrationStatus::Completed);
        assert!(!r2.reuse_hits.is_empty(), "第二次同类需求应命中历史资产 Q");
    }

    #[test]
    fn deferred_freeze_waits_for_gate_confirmation() {
        let project = Project::new("延迟冻结验证", Some("t1".into()), 0.7, 0.3, 1.0);
        let mut orch = Orchestrator::new();
        orch.set_freeze_policy(FreezePolicy::Deferred);
        let mut r = orch.run(
            &project,
            &Input::Text("请抓取销售数据。清洗对账。生成图表报告。".into()),
            0.2,
        );
        assert_eq!(
            r.status,
            OrchestrationStatus::CompletedPendingGate,
            "Deferred 策略应暂缓冻结"
        );
        assert!(r.frozen_asset_id.is_none(), "闸门确认前不得冻结资产");
        assert_eq!(r.artifacts.len(), 8, "文档照常生成");

        // 闸门通过 → 补冻结
        assert!(orch.confirm_gate(&mut r, true), "闸门通过应成功冻结");
        assert_eq!(r.status, OrchestrationStatus::Completed);
        assert!(r.frozen_asset_id.is_some(), "闸门通过后资产应冻结");
    }

    #[test]
    fn confirm_gate_blocks_freeze_when_gate_failed() {
        let project = Project::new("闸门拦截验证", Some("t1".into()), 0.7, 0.3, 1.0);
        let mut orch = Orchestrator::new();
        orch.set_freeze_policy(FreezePolicy::Deferred);
        let mut r = orch.run(
            &project,
            &Input::Text("请抓取销售数据。清洗对账。生成图表报告。".into()),
            0.2,
        );
        assert_eq!(r.status, OrchestrationStatus::CompletedPendingGate);

        // 闸门未通过 → 永不冻结，拓扑标记 Rejected
        assert!(!orch.confirm_gate(&mut r, false), "闸门未通过不得冻结");
        assert!(r.frozen_asset_id.is_none(), "闸门未通过资产必须保持未冻结");
        let rec = r.topology_record.as_ref().unwrap();
        assert_eq!(
            rec.status,
            TopologyStatus::Rejected.as_str(),
            "拓扑应标记为 Rejected"
        );
    }

    #[test]
    fn immediate_policy_keeps_legacy_behavior() {
        let project = Project::new("立即冻结回归", Some("t1".into()), 0.7, 0.3, 1.0);
        let mut orch = Orchestrator::new();
        let r = orch.run(
            &project,
            &Input::Text("请抓取销售数据。清洗对账。生成图表报告。".into()),
            0.2,
        );
        assert_eq!(
            r.status,
            OrchestrationStatus::Completed,
            "默认策略应保持即冻结行为"
        );
        assert!(r.frozen_asset_id.is_some());
    }

    #[test]
    fn audio_input_is_transcribed() {
        let project = Project::new("报销系统", Some("t1".into()), 0.7, 0.3, 1.0);
        let mut orch = Orchestrator::new();
        // 命中 LocalMockAsr 词表的音频文件名
        let r = orch.run(&project, &Input::Audio("报销审批录音.mp3".into()), 0.2);
        assert_eq!(r.status, OrchestrationStatus::Completed);
        assert!(r
            .requirement
            .unwrap()
            .primi
            .subtasks
            .iter()
            .any(|s| s.tool == mox_ai_flow_svc::model::ToolKind::Human));
    }
}
