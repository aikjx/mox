//! 关联图谱（需求→功能→业务→算法→任务→代码→数据设计→数据存储）
//!
//! 这是 PrimiFlow 的**唯一事实源**：所有功能、代码、数据都从这张带类型边的图谱派生。
//! 图谱直接对应 `primiflow/SPEC.md` 的六维溯源 `trace_links`
//! （requirement_id / feature_id / business_id / algorithm_id / task_id / code_id）。
//!
//! 通过 [`validate_correspondence`] 强制「一一对应」不变量：
//! 每个需求都能沿图谱向下追溯到代码，每个有状态的代码都能向上追溯到需求与数据存储。

use serde::{Deserialize, Serialize};

/// 八层节点类型（前六层 = SPEC 六维溯源，后两层 = 数据落地）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// 需求（来自 SPEC §10 DoD 验收项）
    Requirement,
    /// 功能模块（来自 SPEC §7 模块拆分）
    Feature,
    /// 业务过程（需求结构化 / 拓扑涌现 / 正则化 / 资产复用 / 溯源 / 导出）
    Business,
    /// 算法（κτ 调度 / ℛ̂ 正则化 / pgvector 检索 / 六维溯源绑定）
    Algorithm,
    /// 运行时任务（编排步骤）
    Task,
    /// 代码产物（Rust 模块骨架）
    Code,
    /// 数据设计（表 / 结构）
    DataSchema,
    /// 数据存储（PostgreSQL + pgvector）
    DataStore,
}

impl NodeKind {
    /// 中文名
    pub fn as_zh(&self) -> &'static str {
        match self {
            NodeKind::Requirement => "需求",
            NodeKind::Feature => "功能",
            NodeKind::Business => "业务",
            NodeKind::Algorithm => "算法",
            NodeKind::Task => "任务",
            NodeKind::Code => "代码",
            NodeKind::DataSchema => "数据设计",
            NodeKind::DataStore => "数据存储",
        }
    }

    /// Mermaid classDef 类名
    pub fn class(&self) -> &'static str {
        match self {
            NodeKind::Requirement => "req",
            NodeKind::Feature => "feat",
            NodeKind::Business => "biz",
            NodeKind::Algorithm => "algo",
            NodeKind::Task => "task",
            NodeKind::Code => "code",
            NodeKind::DataSchema => "ds",
            NodeKind::DataStore => "store",
        }
    }
}

/// 带类型的关系边
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// 需求 → 功能（需求被功能满足）
    Satisfies,
    /// 功能 → 业务（功能实现业务过程）
    Realizes,
    /// 业务 → 算法（业务由算法实现）
    Implements,
    /// 算法 → 任务（算法由任务执行）
    Executes,
    /// 任务 → 代码（任务落地为代码）
    Codes,
    /// 代码 → 数据设计（代码对应数据设计）
    Designs,
    /// 数据设计 → 数据存储（落库）
    Persists,
    /// 代码 → 代码（模块依赖）
    Depends,
}

impl EdgeKind {
    /// 边标签（Mermaid）
    pub fn label(&self) -> &'static str {
        match self {
            EdgeKind::Satisfies => "satisfied_by",
            EdgeKind::Realizes => "realizes",
            EdgeKind::Implements => "implemented_by",
            EdgeKind::Executes => "executed_by",
            EdgeKind::Codes => "coded_in",
            EdgeKind::Designs => "design_for",
            EdgeKind::Persists => "persisted_in",
            EdgeKind::Depends => "depends_on",
        }
    }
}

/// 字段（用于数据设计 → 结构生成）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    /// Rust 类型字符串，如 `Uuid` / `String` / `DateTime<Utc>`
    pub ty: String,
}

/// 图谱节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    pub label: String,
    /// 设计说明 / 落点
    pub doc: String,
    /// 数据设计节点的字段（仅 DataSchema 使用）
    #[serde(default)]
    pub fields: Vec<Field>,
    /// 是否为有状态代码（需映射到数据设计与存储）
    #[serde(default)]
    pub stateful: bool,
}

impl Node {
    pub fn new(id: impl Into<String>, kind: NodeKind, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            label: label.into(),
            doc: String::new(),
            fields: Vec::new(),
            stateful: false,
        }
    }
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = doc.into();
        self
    }
    pub fn with_fields(mut self, fields: &[(&str, &str)]) -> Self {
        self.fields = fields
            .iter()
            .map(|(n, t)| Field {
                name: n.to_string(),
                ty: t.to_string(),
            })
            .collect();
        self
    }
    pub fn stateful(mut self, v: bool) -> Self {
        self.stateful = v;
        self
    }
}

/// 关系边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

impl Edge {
    pub fn new(from: impl Into<String>, to: impl Into<String>, kind: EdgeKind) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            kind,
        }
    }
}

/// 关联图谱
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssocGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl AssocGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, n: Node) -> &mut Self {
        self.nodes.push(n);
        self
    }

    pub fn link(&mut self, from: &str, to: &str, kind: EdgeKind) -> &mut Self {
        self.edges.push(Edge::new(from, to, kind));
        self
    }

    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    fn out_edges(&self, id: &str, kind: EdgeKind) -> Vec<&Edge> {
        self.edges
            .iter()
            .filter(|e| e.from == id && e.kind == kind)
            .collect()
    }

    /// 反向追溯：从某节点沿指定边类型一路向上找到所有上游 id
    fn upstream_ids(&self, start: &str, kinds: &[EdgeKind]) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![start.to_string()];
        let mut seen = std::collections::HashSet::new();
        while let Some(cur) = stack.pop() {
            for e in self.edges.iter() {
                if e.to == cur && kinds.contains(&e.kind) && seen.insert(e.from.clone()) {
                    out.push(e.from.clone());
                    stack.push(e.from.clone());
                }
            }
        }
        out
    }

    /// 取某代码节点的完整六维溯源链路（R → F → B → A → T → C），用于 doc 注释。
    ///
    /// 沿真实边逐跳回溯（任务←算法←业务←功能←需求），得到一条语义一致的路径，
    /// 而不是各层独立取首个上游（那样会出现跨链错配）。
    pub fn trace_chain(&self, code_id: &str) -> Vec<String> {
        let mut chain = vec![code_id.to_string()];
        // 任务：Codes 边 to == code
        if let Some(task) = self
            .edges
            .iter()
            .find(|e| e.kind == EdgeKind::Codes && e.to == code_id)
        {
            chain.insert(0, task.from.clone());
            // 算法：Executes 边 to == task
            if let Some(algo) = self
                .edges
                .iter()
                .find(|e| e.kind == EdgeKind::Executes && e.to == task.from)
            {
                chain.insert(0, algo.from.clone());
                // 业务：Implements 边 to == algo
                if let Some(biz) = self
                    .edges
                    .iter()
                    .find(|e| e.kind == EdgeKind::Implements && e.to == algo.from)
                {
                    chain.insert(0, biz.from.clone());
                    // 功能：Realizes 边 to == biz
                    if let Some(feat) = self
                        .edges
                        .iter()
                        .find(|e| e.kind == EdgeKind::Realizes && e.to == biz.from)
                    {
                        chain.insert(0, feat.from.clone());
                        // 需求：Satisfies 边 to == feat
                        if let Some(req) = self
                            .edges
                            .iter()
                            .find(|e| e.kind == EdgeKind::Satisfies && e.to == feat.from)
                        {
                            chain.insert(0, req.from.clone());
                        }
                    }
                }
            }
        }
        chain
    }

    /// 取某代码节点对应的数据设计节点 id（Designs 边）
    pub fn data_schemas_of(&self, code_id: &str) -> Vec<String> {
        self.out_edges(code_id, EdgeKind::Designs)
            .iter()
            .map(|e| e.to.clone())
            .collect()
    }

    /// 「一一对应」校验：返回所有违反不变量的描述
    pub fn validate_correspondence(&self) -> Vec<String> {
        let mut errs = Vec::new();
        let has = |from: &str, kind: EdgeKind| {
            self.edges.iter().any(|e| e.from == from && e.kind == kind)
        };

        // 每类节点都要有下游（或存储落点）
        for n in &self.nodes {
            match n.kind {
                NodeKind::Requirement => {
                    if !has(&n.id, EdgeKind::Satisfies) {
                        errs.push(format!("需求 {} 无对应功能（断裂于 F 层）", n.id));
                    }
                }
                NodeKind::Feature => {
                    if !has(&n.id, EdgeKind::Realizes) {
                        errs.push(format!("功能 {} 无对应业务（断裂于 B 层）", n.id));
                    }
                }
                NodeKind::Business => {
                    if !has(&n.id, EdgeKind::Implements) {
                        errs.push(format!("业务 {} 无对应算法（断裂于 A 层）", n.id));
                    }
                }
                NodeKind::Algorithm => {
                    if !has(&n.id, EdgeKind::Executes) {
                        errs.push(format!("算法 {} 无对应任务（断裂于 T 层）", n.id));
                    }
                }
                NodeKind::Task => {
                    if !has(&n.id, EdgeKind::Codes) {
                        errs.push(format!("任务 {} 无对应代码（断裂于 C 层）", n.id));
                    }
                }
                NodeKind::Code => {
                    if n.stateful && !has(&n.id, EdgeKind::Designs) {
                        errs.push(format!("有状态代码 {} 无对应数据设计", n.id));
                    }
                }
                NodeKind::DataSchema => {
                    if !has(&n.id, EdgeKind::Persists) {
                        errs.push(format!("数据设计 {} 未落存储", n.id));
                    }
                }
                NodeKind::DataStore => {}
            }
        }

        // 每个需求都能端到端追溯到至少一个代码节点
        for n in &self.nodes {
            if n.kind == NodeKind::Requirement {
                let codes: Vec<&Edge> = self
                    .edges
                    .iter()
                    .filter(|e| e.kind == EdgeKind::Codes)
                    .collect();
                let reaches_code = self
                    .upstream_ids(
                        &n.id,
                        &[
                            EdgeKind::Satisfies,
                            EdgeKind::Realizes,
                            EdgeKind::Implements,
                            EdgeKind::Executes,
                        ],
                    )
                    .iter()
                    .any(|id| {
                        codes
                            .iter()
                            .any(|e| e.from == *id || self.upstream_ids(&e.from, &[]).contains(id))
                    });
                // 简化判定：存在 T→C 边且其上游可达本需求即可
                let mut ok = false;
                for ce in &codes {
                    let up = self.upstream_ids(
                        &ce.from,
                        &[
                            EdgeKind::Executes,
                            EdgeKind::Implements,
                            EdgeKind::Realizes,
                            EdgeKind::Satisfies,
                        ],
                    );
                    if up.contains(&n.id) {
                        ok = true;
                        break;
                    }
                }
                if !ok {
                    errs.push(format!("需求 {} 无法追溯到任何代码节点", n.id));
                }
                let _ = reaches_code;
            }
        }
        errs
    }

    /// 导出 Mermaid 可视化关联关系图
    pub fn to_mermaid(&self) -> String {
        let mut s = String::from("flowchart LR\n");
        s.push_str("  classDef req fill:#ffe0b2,stroke:#e65100;\n");
        s.push_str("  classDef feat fill:#c8e6c9,stroke:#1b5e20;\n");
        s.push_str("  classDef biz fill:#bbdefb,stroke:#0d47a1;\n");
        s.push_str("  classDef algo fill:#e1bee7,stroke:#4a148c;\n");
        s.push_str("  classDef task fill:#fff9c4,stroke:#f57f17;\n");
        s.push_str("  classDef code fill:#d7ccc8,stroke:#3e2723;\n");
        s.push_str("  classDef ds fill:#b2dfdb,stroke:#004d40;\n");
        s.push_str("  classDef store fill:#cfd8dc,stroke:#263238;\n");
        for n in &self.nodes {
            let lbl = n.label.replace('"', "'");
            s.push_str(&format!("  {}[\"{} · {}\"]\n", n.id, n.kind.as_zh(), lbl));
            s.push_str(&format!("  class {} {}\n", n.id, n.kind.class()));
        }
        for e in &self.edges {
            s.push_str(&format!("  {} -->|{}| {}\n", e.from, e.kind.label(), e.to));
        }
        s
    }
}

/// 用 `primiflow/SPEC.md` 的种子构建完整关联图谱。
///
/// 节点与边严格对应 SPEC §4（数据模型）、§5（κ/τ 调度）、§7（模块）、§8（八文档）、§10（DoD）。
pub fn primiflow_seed() -> AssocGraph {
    let mut g = AssocGraph::new();

    // ── 需求 R（SPEC §10 DoD）──
    g.add(
        Node::new("R1", NodeKind::Requirement, "自然语言需求→可渲染DAG画布")
            .with_doc("输入自然语言需求+滑块，端到端产出可渲染 DAG 画布，无人工画流程图。"),
    );
    g.add(
        Node::new("R2", NodeKind::Requirement, "ℛ̂ 合规裁剪")
            .with_doc("ℛ̂ 对任意超预算/矛盾拓扑产出 Δ≥0 合规 DAG，或显式 rejected 触发重生成。"),
    );
    g.add(
        Node::new("R3", NodeKind::Requirement, "八份说明书自动生成")
            .with_doc("8 份文档可从拓扑自动生成并在 DocViewer 查看；#7 为骨架/桩。"),
    );
    g.add(
        Node::new("R4", NodeKind::Requirement, "六维溯源绑定")
            .with_doc("trace_links 对每条 需求-功能-业务-算法-任务-代码 建立可追溯绑定。"),
    );
    g.add(
        Node::new("R5", NodeKind::Requirement, "κ 复用资产 Q")
            .with_doc("第二次同类需求能检索到首次冻结的资产 Q 并优先复用。"),
    );
    g.add(
        Node::new("R6", NodeKind::Requirement, "冒烟兜底主链路")
            .with_doc("schema 校验+smoke 冒烟覆盖主链路，失败回写对话重生成，不静默放行。"),
    );

    // ── 功能 F（SPEC §7 模块）──
    g.add(
        Node::new("F1", NodeKind::Feature, "orchestrator 编排状态机")
            .with_doc("requirement→topology→docs 状态机，写六维溯源绑定。"),
    );
    g.add(Node::new("F2", NodeKind::Feature, "scheduler κ/τ+ℛ̂").with_doc("κ/τ 预算 + ℛ̂ 裁剪。"));
    g.add(
        Node::new("F3", NodeKind::Feature, "asset 资产检索/冻结").with_doc("pgvector 检索与冻结。"),
    );
    g.add(
        Node::new("F4", NodeKind::Feature, "topology_operator 需求→DAG")
            .with_doc("需求结构化 + 拓扑涌现。"),
    );
    g.add(
        Node::new("F5", NodeKind::Feature, "doc_generator 八文档")
            .with_doc("生成 8 份说明书 + 代码骨架 + 导出。"),
    );
    g.add(
        Node::new("F6", NodeKind::Feature, "smoke_tester 校验/冒烟")
            .with_doc("schema 校验 + 冒烟测试。"),
    );
    g.add(
        Node::new("F7", NodeKind::Feature, "canvas 可视化可编辑画布")
            .with_doc("Cytoscape 渲染 + 拖拽编辑，改完重算 ℛ̂。"),
    );
    g.add(
        Node::new("F8", NodeKind::Feature, "asr 语音转写")
            .with_doc("语音→文本，作为需求输入模态。"),
    );

    // ── 业务 B ──
    g.add(
        Node::new("B1", NodeKind::Business, "需求结构化").with_doc("NL 需求 → 结构化需求树/约束。"),
    );
    g.add(
        Node::new("B2", NodeKind::Business, "拓扑涌现")
            .with_doc("需求 → DAG（节点=功能/模块，边=依赖/数据流）。"),
    );
    g.add(Node::new("B3", NodeKind::Business, "正则化裁剪").with_doc("Δ=C²−(κ²+τ²)≥0 合规裁剪。"));
    g.add(
        Node::new("B4", NodeKind::Business, "资产冻结复用")
            .with_doc("合格产出冻结为 Q 资产，pgvector 召回复用。"),
    );
    g.add(
        Node::new("B5", NodeKind::Business, "六维溯源")
            .with_doc("需求↔功能↔业务↔算法↔任务↔代码 全绑定。"),
    );
    g.add(
        Node::new("B6", NodeKind::Business, "导出工程")
            .with_doc("导出代码骨架工程 / 迁移包 / 部署清单。"),
    );

    // ── 算法 A ──
    g.add(
        Node::new("A1", NodeKind::Algorithm, "κτ 调度")
            .with_doc("滑动 θ→κ=cosθ,τ=sinθ，预算 C 上界；复用偏置抬高 κ。"),
    );
    g.add(
        Node::new("A2", NodeKind::Algorithm, "ℛ̂ 正则化")
            .with_doc("Δ<0 或 cost>C 时按最低优先级裁剪边/节点直至 Δ≥0。"),
    );
    g.add(
        Node::new("A3", NodeKind::Algorithm, "pgvector 检索")
            .with_doc("按 domain + embedding 相似度检索 Top-K 候选资产。"),
    );
    g.add(
        Node::new("A4", NodeKind::Algorithm, "六维溯源绑定")
            .with_doc("每条产出写入 trace_links 六维绑定。"),
    );

    // ── 任务 T ──
    g.add(Node::new("T0", NodeKind::Task, "asr_transcribe").with_doc("语音转写。"));
    g.add(Node::new("T1", NodeKind::Task, "parse_requirement").with_doc("需求解析入图。"));
    g.add(Node::new("T2", NodeKind::Task, "emerge_topology").with_doc("涌现拓扑。"));
    g.add(Node::new("T3", NodeKind::Task, "regularize").with_doc("正则化裁剪。"));
    g.add(Node::new("T4", NodeKind::Task, "freeze_asset").with_doc("检索/冻结资产。"));
    g.add(Node::new("T5", NodeKind::Task, "bind_trace").with_doc("写六维溯源。"));
    g.add(Node::new("T6", NodeKind::Task, "generate_docs").with_doc("生成 8 文档。"));
    g.add(Node::new("T7", NodeKind::Task, "smoke_test").with_doc("schema+冒烟校验。"));
    g.add(Node::new("T8", NodeKind::Task, "export_project").with_doc("导出工程。"));
    g.add(Node::new("T9", NodeKind::Task, "edit_canvas").with_doc("画布编辑后重算。"));

    // ── 代码 C（Rust 模块骨架）──
    g.add(
        Node::new("C1", NodeKind::Code, "Orchestrator")
            .stateful(true)
            .with_doc("状态机编排：需求→拓扑→文档，并写六维溯源。"),
    );
    g.add(
        Node::new("C2", NodeKind::Code, "Scheduler")
            .stateful(true)
            .with_doc("κ/τ 预算 + ℛ̂ 裁剪。"),
    );
    g.add(
        Node::new("C3", NodeKind::Code, "AssetService")
            .stateful(true)
            .with_doc("pgvector 检索 / 冻结。"),
    );
    g.add(
        Node::new("C4", NodeKind::Code, "TopologyOperator")
            .stateful(true)
            .with_doc("需求结构化 + 拓扑涌现。"),
    );
    g.add(
        Node::new("C5", NodeKind::Code, "DocGenerator")
            .stateful(true)
            .with_doc("8 文档 + 代码骨架 + 导出。"),
    );
    g.add(
        Node::new("C6", NodeKind::Code, "SmokeTester")
            .stateful(true)
            .with_doc("schema 校验 + 冒烟。"),
    );
    g.add(
        Node::new("C7", NodeKind::Code, "CanvasState")
            .stateful(true)
            .with_doc("画布状态 + 编辑后重算 ℛ̂。"),
    );
    g.add(
        Node::new("C8", NodeKind::Code, "AsrClient")
            .stateful(true)
            .with_doc("语音转写客户端。"),
    );

    // ── 数据设计 S（SPEC §4）──
    g.add(
        Node::new("S1", NodeKind::DataSchema, "Project").with_fields(&[
            ("id", "Uuid"),
            ("name", "String"),
            ("tenant_id", "Option<String>"),
            ("k_t_pref", "String"),
            ("budget_c", "f32"),
            ("created_at", "DateTime<Utc>"),
        ]),
    );
    g.add(
        Node::new("S2", NodeKind::DataSchema, "Conversation").with_fields(&[
            ("id", "Uuid"),
            ("project_id", "Uuid"),
            ("role", "String"),
            ("content", "String"),
            ("meta", "Option<String>"),
            ("created_at", "DateTime<Utc>"),
        ]),
    );
    g.add(
        Node::new("S3", NodeKind::DataSchema, "Topology").with_fields(&[
            ("id", "Uuid"),
            ("project_id", "Uuid"),
            ("status", "String"),
            ("k", "f32"),
            ("t", "f32"),
            ("c", "f32"),
            ("residual_delta", "f32"),
            ("graph_json", "String"),
            ("created_at", "DateTime<Utc>"),
        ]),
    );
    g.add(
        Node::new("S4", NodeKind::DataSchema, "Asset").with_fields(&[
            ("id", "Uuid"),
            ("topology_id", "Uuid"),
            ("name", "String"),
            ("domain", "Option<String>"),
            ("graph_json", "String"),
            ("frozen_at", "DateTime<Utc>"),
        ]),
    );
    g.add(
        Node::new("S5", NodeKind::DataSchema, "Artifact").with_fields(&[
            ("id", "Uuid"),
            ("project_id", "Uuid"),
            ("kind", "String"),
            ("title", "String"),
            ("content", "String"),
            ("created_at", "DateTime<Utc>"),
        ]),
    );
    g.add(
        Node::new("S6", NodeKind::DataSchema, "TraceLink").with_fields(&[
            ("id", "Uuid"),
            ("project_id", "Uuid"),
            ("requirement_id", "String"),
            ("feature_id", "String"),
            ("business_id", "String"),
            ("algorithm_id", "String"),
            ("task_id", "String"),
            ("code_id", "String"),
        ]),
    );

    // ── 数据存储 D ──
    g.add(
        Node::new("D1", NodeKind::DataStore, "PostgreSQL + pgvector")
            .with_doc("主存储 + 资产语义检索（κ 复用）。"),
    );

    // ── 边：需求 → 功能 ──
    g.link("R1", "F4", EdgeKind::Satisfies);
    g.link("R1", "F7", EdgeKind::Satisfies);
    g.link("R2", "F2", EdgeKind::Satisfies);
    g.link("R3", "F5", EdgeKind::Satisfies);
    g.link("R4", "F1", EdgeKind::Satisfies);
    g.link("R5", "F3", EdgeKind::Satisfies);
    g.link("R6", "F6", EdgeKind::Satisfies);

    // ── 功能 → 业务 ──
    g.link("F1", "B5", EdgeKind::Realizes);
    g.link("F1", "B6", EdgeKind::Realizes);
    g.link("F2", "B3", EdgeKind::Realizes);
    g.link("F3", "B4", EdgeKind::Realizes);
    g.link("F4", "B1", EdgeKind::Realizes);
    g.link("F4", "B2", EdgeKind::Realizes);
    g.link("F5", "B6", EdgeKind::Realizes);
    g.link("F6", "B3", EdgeKind::Realizes);
    g.link("F7", "B2", EdgeKind::Realizes);
    g.link("F8", "B1", EdgeKind::Realizes);

    // ── 业务 → 算法 ──
    g.link("B1", "A1", EdgeKind::Implements);
    g.link("B2", "A1", EdgeKind::Implements);
    g.link("B3", "A1", EdgeKind::Implements);
    g.link("B3", "A2", EdgeKind::Implements);
    g.link("B4", "A3", EdgeKind::Implements);
    g.link("B5", "A4", EdgeKind::Implements);
    g.link("B6", "A4", EdgeKind::Implements);

    // ── 算法 → 任务 ──
    g.link("A1", "T0", EdgeKind::Executes);
    g.link("A1", "T1", EdgeKind::Executes);
    g.link("A1", "T2", EdgeKind::Executes);
    g.link("A1", "T3", EdgeKind::Executes);
    g.link("A2", "T3", EdgeKind::Executes);
    g.link("A2", "T7", EdgeKind::Executes);
    g.link("A2", "T9", EdgeKind::Executes);
    g.link("A3", "T4", EdgeKind::Executes);
    g.link("A4", "T5", EdgeKind::Executes);
    g.link("A4", "T6", EdgeKind::Executes);
    g.link("A4", "T8", EdgeKind::Executes);

    // ── 任务 → 代码 ──
    g.link("T0", "C8", EdgeKind::Codes);
    g.link("T1", "C1", EdgeKind::Codes);
    g.link("T2", "C4", EdgeKind::Codes);
    g.link("T3", "C2", EdgeKind::Codes);
    g.link("T4", "C3", EdgeKind::Codes);
    g.link("T5", "C1", EdgeKind::Codes);
    g.link("T6", "C5", EdgeKind::Codes);
    g.link("T7", "C6", EdgeKind::Codes);
    g.link("T8", "C5", EdgeKind::Codes);
    g.link("T9", "C7", EdgeKind::Codes);

    // ── 代码 → 数据设计 ──
    g.link("C1", "S1", EdgeKind::Designs);
    g.link("C1", "S6", EdgeKind::Designs);
    g.link("C2", "S3", EdgeKind::Designs);
    g.link("C3", "S4", EdgeKind::Designs);
    g.link("C4", "S3", EdgeKind::Designs);
    g.link("C5", "S5", EdgeKind::Designs);
    g.link("C6", "S5", EdgeKind::Designs);
    g.link("C7", "S3", EdgeKind::Designs);
    g.link("C8", "S2", EdgeKind::Designs);

    // ── 数据设计 → 数据存储 ──
    for s in ["S1", "S2", "S3", "S4", "S5", "S6"] {
        g.link(s, "D1", EdgeKind::Persists);
    }

    // ── 代码依赖（模块间）──
    g.link("C1", "C4", EdgeKind::Depends);
    g.link("C1", "C2", EdgeKind::Depends);
    g.link("C1", "C3", EdgeKind::Depends);
    g.link("C4", "C2", EdgeKind::Depends);
    g.link("C5", "C1", EdgeKind::Depends);
    g.link("C6", "C4", EdgeKind::Depends);

    g
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_is_fully_correspondent() {
        let g = primiflow_seed();
        let errs = g.validate_correspondence();
        assert!(errs.is_empty(), "图谱不满足一一对应:\n{:?}", errs);
    }

    #[test]
    fn code_node_has_full_trace_chain() {
        let g = primiflow_seed();
        let chain = g.trace_chain("C4");
        // C4 (TopologyOperator) 应沿真实边追溯到 R1→F4→B1→A1→T2→C4
        assert_eq!(
            chain,
            vec!["R1", "F4", "B1", "A1", "T2", "C4"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn mermaid_export_is_nonempty() {
        let g = primiflow_seed();
        let mmd = g.to_mermaid();
        assert!(mmd.contains("flowchart LR"));
        assert!(mmd.contains("R1"));
        assert!(mmd.contains("satisfied_by"));
    }
}
