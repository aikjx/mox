//! PrimiFlow 企业级端到端运行器
//!
//! 把 `flow-ai` 的 κ‑τ 自涌现引擎（大脑）与 `primiflow` 的六维溯源图谱（唯一事实源）打通，
//! 对每个自然语言需求跑完整闭环：
//!
//! ```text
//! 需求结构化 → 原语初始化(C,κ,τ) → κτ 自涌现 → 守恒/因果/资源三道校验
//!           → ℛ̂ 正则化(必要时) → 执行反馈(注荷/湮灭) → 六维溯源绑定 → 文档自生成 → Mermaid 可视化
//! ```
//!
//! `run_pipeline` 同时被示例 [`crate::examples`] 与集成测试复用，保证「可运行」与「可验证」同源。
//! 引擎在多次调用间保持状态（闭环节点状态自动继承：成功注荷 Q、失败抬高探索 τ），
//! 因此第二个同类需求会命中知识库、自动抬高 κ 复用成熟链路。

use anyhow::Result;
use flow_ai::model::ToolKind;
use flow_ai::primitive::{
    DeliveryPolicy, EmergeStatus, Outcome, PrimiEngine, Requirement as EngineRequirement, SubTask,
};
use flow_ai::to_mermaid;
use serde::Serialize;
use std::fmt;
use std::path::Path;

use crate::assoc::{AssocGraph, EdgeKind, Node, NodeKind};
use crate::executor::{execute_chain, SubtaskTriple};

/// 守恒残差允许的浮点误差（对齐 flow-ai::primitive 内部 `CONSERVATION_EPS = 1e-6`）
const CONSERVATION_EPS: f64 = 1e-6;

/// 单步验证结果（企业级分步验证的最小单元）
#[derive(Debug, Clone, Serialize)]
pub struct Step {
    pub name: &'static str,
    pub ok: bool,
    pub detail: String,
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = if self.ok { "PASS" } else { "FAIL" };
        write!(f, "  [{tag}] {:<18} {}", self.name, self.detail)
    }
}

/// 一个需求跑完闭环后的企业级验证报告
#[derive(Debug, Clone, Serialize)]
pub struct PipelineReport {
    pub requirement: String,
    pub policy: &'static str,
    pub steps: Vec<Step>,
    pub kappa: f64,
    pub tau: f64,
    pub conserved: bool,
    pub acyclic: bool,
    pub reused: usize,
    pub explored: usize,
    pub fanout: usize,
    pub regularized: bool,
    pub charge_estimate: f64,
    pub topology_nodes: usize,
    pub q_before: f64,
    pub q_after: f64,
    pub bound_nodes: usize,
    pub bound_edges: usize,
    /// 真实执行每一条子任务算子的记录（需求→执行闭环的实证证据）
    pub execution: Vec<crate::executor::ExecRecord>,
}

impl PipelineReport {
    /// 所有分步验证是否全绿
    pub fn all_ok(&self) -> bool {
        self.steps.iter().all(|s| s.ok)
    }
}

impl fmt::Display for PipelineReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "需求「{}」 (策略: {})", self.requirement, self.policy)?;
        for s in &self.steps {
            writeln!(f, "{s}")?;
        }
        writeln!(
            f,
            "  汇总: κ={:.3} τ={:.3} 守恒={} 无环={} 复用{} 探索{} 分叉{} 正则化={} 预估荷{:.2} 拓扑节点{} 绑定{}/{}",
            self.kappa,
            self.tau,
            self.conserved,
            self.acyclic,
            self.reused,
            self.explored,
            self.fanout,
            self.regularized,
            self.charge_estimate,
            self.topology_nodes,
            self.bound_nodes,
            self.bound_edges
        )?;
        if !self.execution.is_empty() {
            writeln!(f, "  算子真实执行:")?;
            for r in &self.execution {
                writeln!(f, "    - {}", r.short())?;
            }
        }
        Ok(())
    }
}

/// 子任务规格：ascii `key` 同时作为任务/代码节点标识（保证生成代码标识符合法），
/// 中文 `label` 写入节点 doc 供文档/画布展示。
#[derive(Debug, Clone)]
pub struct SubtaskSpec {
    pub key: String,
    pub label: String,
    pub tool: ToolKind,
    pub ms: u64,
}

/// 企业级需求规格（用于示例与测试共享同一组真实场景）
#[derive(Debug, Clone)]
pub struct Spec {
    pub id: String,
    pub name: String,
    pub policy: DeliveryPolicy,
    pub subtasks: Vec<SubtaskSpec>,
}

impl Spec {
    pub fn new(id: &str, name: &str, policy: DeliveryPolicy) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            policy,
            subtasks: Vec::new(),
        }
    }

    pub fn sub(mut self, key: &str, label: &str, tool: ToolKind, ms: u64) -> Self {
        self.subtasks.push(SubtaskSpec {
            key: key.into(),
            label: label.into(),
            tool,
            ms,
        });
        self
    }

    /// 转成 flow-ai 引擎的结构化需求
    pub fn requirement(&self) -> EngineRequirement {
        let mut r = EngineRequirement::new(&self.id, &self.name);
        for s in &self.subtasks {
            r = r.with_subtask(SubTask::new(&s.key, s.label.clone(), s.tool, s.ms));
        }
        r
    }
}

/// 一组覆盖「均衡 / 紧急复用 / 探索分叉 / 超预算正则化」的代表性企业需求。
///
/// 注意 r2、r3、r4 都复用了 r1 的 `report`（生成图表报告）子任务名，
/// 因此 r1 成功注库后，后续需求命中知识库会自动抬高 κ 复用成熟链路。
pub fn enterprise_specs() -> Vec<Spec> {
    vec![
        Spec::new("r1", "电商月度经营分析报告", DeliveryPolicy::Balanced)
            .sub("fetch", "抓取销售数据", ToolKind::Http, 300)
            .sub("clean", "清洗对账", ToolKind::Compute, 200)
            .sub("report", "生成图表报告", ToolKind::Llm, 400),
        Spec::new("r2", "零售周度库存预警", DeliveryPolicy::Urgent)
            .sub("fetch", "抓取销售数据", ToolKind::Http, 300)
            .sub("stock", "库存核算", ToolKind::Database, 250)
            .sub("report", "生成图表报告", ToolKind::Llm, 400),
        Spec::new("r3", "智能客服工单聚类", DeliveryPolicy::Exploratory)
            .sub("pull", "抓取工单", ToolKind::Http, 200)
            .sub("embed", "文本向量化", ToolKind::Compute, 300)
            .sub("cluster", "聚类分析", ToolKind::Compute, 350)
            .sub("report", "生成图表报告", ToolKind::Llm, 400),
        Spec::new("r4", "实时风控流式监控", DeliveryPolicy::Exploratory)
            .sub("ingest", "流数据接入", ToolKind::Http, 500)
            .sub("feature", "特征计算", ToolKind::Compute, 600)
            .sub("model", "模型推理", ToolKind::Llm, 700)
            .sub("alert", "告警下发", ToolKind::Shell, 400)
            .sub("report", "生成图表报告", ToolKind::Llm, 400),
    ]
}

/// 把引擎涌现出的拓扑结构，按六维溯源绑定到 [`AssocGraph`]：
///
/// ```text
/// R(需求) → F(功能) → B(业务) → A(算法:κτ/ℛ̂) → T(任务×子任务) → C(代码×子任务) → S(数据) → D(存储)
/// ```
///
/// 同时按子任务先后顺序建立 `C → C` 依赖边，反映涌现 DAG 的无环性。
fn bind_to_graph(g: &mut AssocGraph, req_id: &str, req_label: &str, subtask_keys: &[String]) {
    let r = format!("R_{req_id}");
    g.add(
        Node::new(&r, NodeKind::Requirement, req_label).with_doc("由 PrimiEngine 自涌现闭环生成"),
    );

    let f = format!("F_{req_id}");
    g.add(
        Node::new(&f, NodeKind::Feature, format!("编排·{req_label}"))
            .with_doc("κτ 编排状态机：需求→拓扑→文档，并写六维溯源"),
    );
    g.link(&r, &f, EdgeKind::Satisfies);

    let b = format!("B_{req_id}");
    g.add(Node::new(&b, NodeKind::Business, "拓扑涌现+正则化").with_doc("需求→DAG，ℛ̂ 裁剪至合规"));
    g.link(&f, &b, EdgeKind::Realizes);

    let a1 = format!("A_kt_{req_id}");
    g.add(
        Node::new(&a1, NodeKind::Algorithm, "κτ 调度")
            .with_doc("θ→κ,τ；预算 C 上界；复用偏置抬高 κ"),
    );
    let a2 = format!("A_r_{req_id}");
    g.add(
        Node::new(&a2, NodeKind::Algorithm, "ℛ̂ 正则化")
            .with_doc("Δ<0 或超预算时按最低优先级裁剪直至 Δ≥0"),
    );
    g.link(&b, &a1, EdgeKind::Implements);
    g.link(&b, &a2, EdgeKind::Implements);

    // 数据设计 + 存储（绑定首个代码节点即满足 correspondence 不变量）
    let s = format!("S_{req_id}");
    g.add(
        Node::new(&s, NodeKind::DataSchema, format!("data_{req_id}")).with_fields(&[
            ("id", "Uuid"),
            ("project_id", "Uuid"),
            ("graph_json", "String"),
            ("created_at", "DateTime<Utc>"),
        ]),
    );
    let d = format!("D_{req_id}");
    g.add(
        Node::new(&d, NodeKind::DataStore, "PostgreSQL + pgvector")
            .with_doc("主存储 + 资产语义检索（κ 复用）"),
    );
    g.link(&s, &d, EdgeKind::Persists);

    let mut prev_c: Option<String> = None;
    for (i, key) in subtask_keys.iter().enumerate() {
        let t = format!("T_{req_id}_{i}");
        g.add(
            Node::new(&t, NodeKind::Task, key.clone())
                .with_doc(format!("编排步骤（对应子任务 {key}）")),
        );
        g.link(&a1, &t, EdgeKind::Executes);
        g.link(&a2, &t, EdgeKind::Executes);

        let c = format!("C_{req_id}_{i}");
        g.add(
            Node::new(&c, NodeKind::Code, key.clone())
                .stateful(true)
                .with_doc(format!("由拓扑自动派生的代码骨架（子任务 {key}）")),
        );
        g.link(&t, &c, EdgeKind::Codes);
        g.link(&c, &s, EdgeKind::Designs);

        if let Some(pc) = prev_c {
            g.link(&pc, &c, EdgeKind::Depends);
        }
        prev_c = Some(c);
    }
}

/// 跑一个需求的完整闭环，并把结果绑定进 `master`（跨需求累积的六维溯源主图）。
///
/// `engine` 在调用间保持状态（闭环节点状态自动继承）。返回该企业级分步验证报告。
pub fn run_pipeline(
    engine: &mut PrimiEngine,
    req: &EngineRequirement,
    policy: DeliveryPolicy,
    master: &mut AssocGraph,
    out_dir: &Path,
) -> Result<PipelineReport> {
    std::fs::create_dir_all(out_dir)?;

    let req_id = req.id.clone();
    let req_label = req.name.clone();
    let subtask_keys: Vec<String> = req.subtasks.iter().map(|s| s.id.clone()).collect();
    let policy_name: &'static str = match policy {
        DeliveryPolicy::Urgent => "Urgent",
        DeliveryPolicy::Balanced => "Balanced",
        DeliveryPolicy::Exploratory => "Exploratory",
        DeliveryPolicy::Custom { .. } => "Custom",
    };
    let q_before = engine.state.q;

    let mut steps = Vec::new();

    // Step 1 · 需求结构化
    steps.push(Step {
        name: "需求结构化",
        ok: !req.subtasks.is_empty(),
        detail: format!("子任务 {} 项", req.subtasks.len()),
    });

    // Step 2 · κτ 自涌现（生成 → 校验 → 必要时 ℛ̂ 正则化重试）
    let result = engine.emerge(req, Some(policy));
    let conserved = result.state.is_conserved(CONSERVATION_EPS);
    steps.push(Step {
        name: "κτ自涌现",
        ok: matches!(result.status, EmergeStatus::Validated { .. }),
        detail: result.summary(),
    });

    // Step 3 · 守恒公理 C² = κ² + τ²
    steps.push(Step {
        name: "守恒校验",
        ok: conserved,
        detail: format!("残差 {:.2e}", result.state.conservation_residual()),
    });

    // Step 4 · 因果无环（DAG 拓扑序存在）
    let acyclic = result.topology.graph.topo_order().is_ok();
    steps.push(Step {
        name: "因果无环",
        ok: acyclic,
        detail: if acyclic {
            "DAG 拓扑序存在".into()
        } else {
            "检测到环".into()
        },
    });

    // Step 5 · 资源预算闸门（守恒/因果/资源三道闸门）
    steps.push(Step {
        name: "资源校验",
        ok: result.validation.ok,
        detail: if result.validation.ok {
            "守恒/因果/资源三道闸门通过".into()
        } else {
            format!("{} 项违例", result.validation.violations.len())
        },
    });

    // Step 6 · ℛ̂ 正则化（仅在需要时标记）
    let regularized = matches!(result.status, EmergeStatus::Validated { regularized: true });
    steps.push(Step {
        name: "ℛ̂正则化",
        ok: matches!(result.status, EmergeStatus::Validated { .. }),
        detail: if regularized {
            "已触发裁剪直至合规".into()
        } else {
            "无需裁剪".into()
        },
    });

    // Step 7 · 算子真实执行（按子任务顺序派发确定性实现，构成真实数据流）
    let subtasks: Vec<SubtaskTriple> = req
        .subtasks
        .iter()
        .map(|s| (s.id.clone(), s.name.clone(), s.tool))
        .collect();
    let seed = serde_json::json!({ "rows": 8, "requirement": req_label });
    let (exec_records, exec_q) = execute_chain(&subtasks, &seed);
    let all_exec_ok = exec_records.iter().all(|r| r.ok);
    steps.push(Step {
        name: "算子真实执行",
        ok: all_exec_ok,
        detail: if all_exec_ok {
            format!("{} 个算子全部执行成功", exec_records.len())
        } else {
            format!(
                "{} 个中 {} 个失败",
                exec_records.len(),
                exec_records.iter().filter(|r| !r.ok).count()
            )
        },
    });

    // Step 8 · 执行反馈 → 注荷/湮灭（以**真实**执行质量回灌引擎）
    engine.accept(&result, Outcome::Success { quality: exec_q });
    let q_after = engine.state.q;
    steps.push(Step {
        name: "执行反馈注荷",
        ok: q_after > q_before,
        detail: format!("Q: {q_before:.2} → {q_after:.2} (执行质量 {exec_q:.2})"),
    });

    // Step 8 · 六维溯源绑定 + 一一对应不变量校验
    bind_to_graph(master, &req_id, &req_label, &subtask_keys);
    let errs = master.validate_correspondence();
    steps.push(Step {
        name: "六维溯源",
        ok: errs.is_empty(),
        detail: if errs.is_empty() {
            "一一对应不变量成立".into()
        } else {
            format!("{} 处断裂", errs.len())
        },
    });

    // 落盘真实涌现 DAG 的 Mermaid（κ‑τ 拓扑生成 → 可视化拓扑）
    let topo_mmd = to_mermaid(&result.topology.graph);
    std::fs::write(out_dir.join(format!("topo_{req_id}.mmd")), &topo_mmd)?;

    // 落盘真实执行记录（需求→执行闭环的实证产物，供审计 / API 透出）
    let exec_json = serde_json::to_string_pretty(&exec_records)?;
    std::fs::write(out_dir.join(format!("exec_{req_id}.json")), &exec_json)?;

    Ok(PipelineReport {
        requirement: req_label,
        policy: policy_name,
        steps,
        kappa: result.state.kappa,
        tau: result.state.tau,
        conserved,
        acyclic,
        reused: result.topology.reused_subtasks.len(),
        explored: result.topology.explored_subtasks.len(),
        fanout: result.topology.fanout,
        regularized,
        charge_estimate: result.charge_estimate,
        topology_nodes: result.topology.graph.nodes.len(),
        q_before,
        q_after,
        bound_nodes: master.nodes.len(),
        bound_edges: master.edges.len(),
        execution: exec_records,
    })
}

/// 跑一组需求并产出全部落地产物（六维溯源主图 → 代码骨架 / DDL / Mermaid / 溯源矩阵）。
///
/// 返回每个需求的报告；跨需求共享同一引擎（知识库在闭环间累积，演示 κ 复用）。
pub fn run_all(
    engine: &mut PrimiEngine,
    specs: &[Spec],
    out_dir: &Path,
) -> Result<Vec<PipelineReport>> {
    let mut master = AssocGraph::new();
    let mut reports = Vec::new();
    for spec in specs {
        let req = spec.requirement();
        let rep = run_pipeline(engine, &req, spec.policy, &mut master, out_dir)?;
        reports.push(rep);
    }
    // 文档自生成：统一导出六维溯源主图的全部产物
    crate::generate::emit_all(&master, out_dir)?;
    Ok(reports)
}
