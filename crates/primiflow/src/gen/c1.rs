//! 代码骨架 · 由关联图谱自动生成（primiflow::assoc::primiflow_seed）
//! 溯源链路: R1 → F4 → B1 → A1 → T1 → C1
//! 数据设计: S1(Project), S6(TraceLink)
//! 说明: 状态机编排：需求→拓扑→文档，并写六维溯源。
//! 规格: primiflow/SPEC.md（§7 模块 / §10 DoD）

/// 依赖模块: C4, C2, C3
use flow_ai::model::FlowGraph;
use flow_ai::primitive::{KnowledgeBase, PrimitiveState, ResourceBudget};
use crate::gen::c2::{RegularizeOutput, Scheduler};
use crate::gen::c3::AssetService;
use crate::gen::c4::{Domain, StructuredRequirement, TopologyOperator};
use crate::gen::c5::{DocContext, DocGenerator};
use crate::gen::c6::{SmokeReport, SmokeTester};
use crate::gen::c7::CanvasState;
use crate::gen::c8::AsrClient;
use crate::gen::schema::{Artifact, Project, Topology, TopologyStatus, TraceLink};

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
        }
    }
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
    engine: flow_ai::primitive::PrimiEngine,
    /// 预算基准 C
    c_base: f64,
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
            engine: flow_ai::primitive::PrimiEngine::new(
                c_base,
                KnowledgeBase::new(),
                ResourceBudget { total_ms: 1_000_000, per_pool: Default::default() },
            ),
            c_base,
        }
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
        let structured = self.topo_op.structure_requirement(&text, uuid::Uuid::new_v4().to_string());

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
        let budget = ResourceBudget { total_ms: 1_000_000, per_pool: Default::default() };
        let mut reg = self.scheduler.regularize(candidate.graph.clone(), state.clone(), budget);
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

        // 11) 资产冻结 Q（第二次同类需求可复用）
        let (asset, _charge) = self.assets.freeze_asset(
            topo_rec.id,
            structured.name.clone(),
            Domain::BusinessSoftware,
            &reg.graph,
        );

        // 12) 把本次成功回灌 κ‑τ 引擎（巩固复用，知识越用越厚）
        let emer = flow_ai::primitive::EmergenceResult {
            topology: flow_ai::primitive::CandidateTopology {
                graph: reg.graph.clone(),
                reused_subtasks: Vec::new(),
                explored_subtasks: Vec::new(),
                fanout: 1,
            },
            state: state.clone(),
            validation: flow_ai::primitive::ValidationReport { ok: true, violations: Vec::new() },
            charge_estimate: 1.0,
            attempts: 0,
            status: flow_ai::primitive::EmergeStatus::Validated { regularized: reg.regularized },
        };
        self.engine.accept(&emer, flow_ai::primitive::Outcome::Success { quality: 0.9 });

        let mut topo_rec = topo_rec;
        topo_rec.set_status(TopologyStatus::Frozen);

        result.topology_record = Some(topo_rec);
        result.artifacts = artifacts;
        result.frozen_asset_id = Some(asset.id);
        result
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
        let r = orch.run(&project, &Input::Text("请抓取销售数据。清洗对账。生成图表报告。".into()), 0.2);
        assert_eq!(r.status, OrchestrationStatus::Completed, "主链路应跑通: {:?}", r.smoke);
        assert!(r.graph.is_some());
        assert_eq!(r.artifacts.len(), 8, "应生成 8 份文档");
        assert!(r.frozen_asset_id.is_some(), "应冻结资产");
        assert!(r.trace_link.as_ref().unwrap().is_complete(), "六维溯源应完整");
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
        let r1 = orch.run(&project, &Input::Text("请抓取销售数据。清洗对账。生成图表报告。".into()), 0.2);
        assert_eq!(r1.status, OrchestrationStatus::Completed);
        assert!(r1.frozen_asset_id.is_some());

        // 第二次同类需求：应检索到首次冻结的资产并复用（κ 复用）
        let r2 = orch.run(&project, &Input::Text("我想做一份电商经营分析报告，需要销售数据抓取和图表生成。".into()), 0.1);
        assert_eq!(r2.status, OrchestrationStatus::Completed);
        assert!(!r2.reuse_hits.is_empty(), "第二次同类需求应命中历史资产 Q");
    }

    #[test]
    fn audio_input_is_transcribed() {
        let project = Project::new("报销系统", Some("t1".into()), 0.7, 0.3, 1.0);
        let mut orch = Orchestrator::new();
        // 命中 LocalMockAsr 词表的音频文件名
        let r = orch.run(&project, &Input::Audio("报销审批录音.mp3".into()), 0.2);
        assert_eq!(r.status, OrchestrationStatus::Completed);
        assert!(r.requirement.unwrap().primi.subtasks.iter().any(|s| s.tool == flow_ai::model::ToolKind::Human));
    }
}
