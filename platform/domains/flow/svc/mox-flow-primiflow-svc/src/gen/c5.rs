// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 代码骨架 · 由关联图谱自动生成（mox_flow_primiflow_svc::assoc::primiflow_seed）
//! 溯源链路: R4 → F1 → B5 → A4 → T6 → C5
//! 数据设计: S5(Artifact)
//! 说明: 八份标准化说明书 + 代码骨架 + 导出（SPEC §8）。
//! 规格: primiflow/SPEC.md（§8 八文档 / §10 DoD）

use crate::gen::schema::{Artifact, ArtifactKind, Project, Topology, TraceLink};
/// 依赖模块: C1
use mox_ai_flow_sdk::model::{FlowGraph, NodeKind};

/// 数据 DDL（与 `emit_all` 产出的 `ddl.sql` 同源；此处保留本地确定性副本便于离线可跑）
pub const SCHEMA_DDL: &str = "\
CREATE TABLE projects (id UUID PRIMARY KEY, name TEXT, tenant_id TEXT, k_t_pref TEXT, budget_c REAL, created_at TIMESTAMPTZ);\n\
CREATE TABLE topologies (id UUID PRIMARY KEY, project_id UUID, status TEXT, k REAL, t REAL, c REAL, residual_delta REAL, graph_json TEXT, created_at TIMESTAMPTZ);\n\
CREATE TABLE assets (id UUID PRIMARY KEY, topology_id UUID, name TEXT, domain TEXT, graph_json TEXT, frozen_at TIMESTAMPTZ);";

/// 文档渲染上下文：把拓扑/项目/溯源聚合成可渲染数据
pub struct DocContext<'a> {
    pub project: &'a Project,
    pub topology: &'a Topology,
    pub graph: &'a FlowGraph,
    pub trace_links: &'a [TraceLink],
}

/// 导出包（画布即源码，导出即工程）
#[derive(Debug, Clone)]
pub struct ExportBundle {
    pub docs: Vec<Artifact>,
    pub code_skeleton: String,
    pub ddl: String,
}

/// 文档生成器：八份说明书 + 工程导出
#[derive(Debug, Clone, Default)]
pub struct DocGenerator;

impl DocGenerator {
    pub fn new() -> Self {
        Self
    }

    /// 按 SPEC §8 生成全部 8 份说明书（markdown）
    pub fn generate_docs(&self, ctx: &DocContext) -> Vec<Artifact> {
        ArtifactKind::all()
            .iter()
            .map(|k| {
                let content = render_doc(*k, ctx);
                Artifact::of(ctx.project.id, *k, content)
            })
            .collect()
    }

    /// 导出工程：8 文档 + Rust 风格代码骨架 + 数据 DDL
    pub fn export_project(&self, ctx: &DocContext) -> ExportBundle {
        let docs = self.generate_docs(ctx);
        let code_skeleton = render_code_skeleton(ctx.graph);
        let ddl = SCHEMA_DDL.to_string();
        ExportBundle {
            docs,
            code_skeleton,
            ddl,
        }
    }
}

/// 渲染单份文档（按种类定制模板，内容绑定真实拓扑节点/边）
fn render_doc(kind: ArtifactKind, ctx: &DocContext) -> String {
    let g = ctx.graph;
    let node_list = g
        .nodes
        .iter()
        .map(|n| format!("- `{}` ({:?}) {}", n.id, n.kind, n.name))
        .collect::<Vec<_>>()
        .join("\n");
    let edge_list = g
        .edges
        .iter()
        .map(|e| format!("- `{}` → `{}` ({:?})", e.from, e.to, e.kind))
        .collect::<Vec<_>>()
        .join("\n");

    match kind {
        ArtifactKind::RequirementSpec => format!(
            "# 需求规格说明书\n\n项目：**{}**\n\n## 结构化需求树\n{}\n\n## 约束\n- 业务域：business_software（域白名单内）\n- κ={:.2} τ={:.2} C={:.2}\n\n## 验收标准\n- [ ] 端到端产出可渲染 DAG 画布\n- [ ] 六维溯源对每条需求建立可追溯绑定\n",
            ctx.project.name, node_list, ctx.topology.k, ctx.topology.t, ctx.topology.c
        ),
        ArtifactKind::FeatureDesign => format!(
            "# 功能设计说明书\n\n## 功能清单\n{}\n\n## 模块划分\n- 接入层：ASR + 文本 ChatPanel\n- 编排层：Orchestrator + Scheduler(κ/τ+ℛ̂)\n- 算子层：TopologyOperator / DocGenerator / SmokeTester\n- 数据层：PostgreSQL + pgvector(资产 Q)\n\n## 用例\n- 输入自然语言需求 → 自动拆解子任务\n- 用户编辑画布 → 重算 ℛ̂\n",
            node_list
        ),
        ArtifactKind::BusinessProcess => format!(
            "# 业务流程说明书\n\n## 主流程（DAG）\n{}\n\n## 边依赖\n{}\n\n## 角色\n- 客户：输入需求 / 编辑画布\n- 系统：自动拆解 / 涌现拓扑 / 生成文档 / 沉淀资产\n\n## 异常\n- 超预算 / 矛盾环 → ℛ̂ 裁剪或 rejected 重生成\n- 幻觉传导 → schema 校验 + 冒烟兜底\n",
            node_list, edge_list
        ),
        ArtifactKind::DataModel => {
            let rw = g
                .nodes
                .iter()
                .filter(|n| !n.accesses.is_empty())
                .map(|n| {
                    let reads = n.read_set().into_iter().collect::<Vec<_>>().join(", ");
                    let writes = n.write_set().into_iter().collect::<Vec<_>>().join(", ");
                    format!("- `{}` 读[{}] 写[{}]", n.name, reads, writes)
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "# 数据模型说明书\n\n## 核心表\n- projects / conversations / topologies / assets / artifacts / trace_links\n\n## 拓扑节点数据流\n{}\n\n## 索引\n- topologies(project_id)\n- assets(domain, embedding) —— pgvector 相似度检索\n- trace_links(UNIQUE 六维)\n",
                rw
            )
        }
        ArtifactKind::ApiContract => "# 接口契约说明书\n\n## REST 契约\n- POST /api/projects\n- POST /api/projects/:id/messages  (自然语言 + 滑块 s)\n- GET /api/topologies/:id\n- POST /api/topologies/:id/regularize\n- POST /api/topologies/:id/freeze\n- GET /api/assets?q=&domain=\n\n## 错误码\n- RFC9457 application/problem+json\n- 域外需求 → 422（白名单拒绝）\n".to_string(),
        ArtifactKind::ScheduledTask => format!(
            "# 定时任务说明书\n\n## 调度周期\n- 资产复用率统计：每日 03:00\n- κ/τ 消耗指标回写：每小时\n\n## 幂等\n- 所有写接口以 topology_id / project_id 为幂等键\n\n## 失败重试\n- ℛ̂ 最多重试 {} 次，失败则 rejected 回写对话\n",
            g.nodes.len().max(3)
        ),
        ArtifactKind::CodeProject => format!(
            "# 代码工程说明书\n\n## 目录结构\n- crates/primiflow/ (Orchestrator/Scheduler/Asset/TopologyOperator/DocGenerator/SmokeTester/CanvasState/AsrClient)\n- crates/flow-ai/ (κ‑τ 引擎 + FlowGraph IR)\n\n## 关键模块\n{}\n\n## 代码骨架\n```rust\n{}\n```\n",
            node_list,
            render_code_skeleton(g)
        ),
        ArtifactKind::Deployment => "# 部署运维说明书\n\n## 依赖\n- Rust (cargo) —— 编排层\n- PostgreSQL 16 + pgvector —— 主存储 / 资产检索\n\n## 环境变量\n- DATABASE_URL\n- PRIMIFLOW_BUDGET_C (默认 C)\n\n## 观测\n- κ/τ 消耗、ℛ̂ 裁剪次数、资产命中率（Prometheus）\n\n## 回滚\n- 拓扑 rejected 自动回写对话重生成；资产冻结前先 dry‑run 冒烟\n".to_string(),
    }
}

/// 由拓扑节点渲染一份 Rust 风格代码骨架（导出用）
fn render_code_skeleton(g: &FlowGraph) -> String {
    let mut out = String::from("// 由 PrimiFlow 拓扑自动导出的代码骨架\n");
    for n in &g.nodes {
        if n.kind == NodeKind::Start || n.kind == NodeKind::End {
            continue;
        }
        let fn_name = n.id.replace(['-', ' ', '.'], "_");
        out.push_str(&format!(
            "/// {} (工具: {:?})\nfn {}(input: &Value) -> anyhow::Result<Value> {{ todo!(\"实现 {}\") }}\n\n",
            n.name, n.tool, fn_name, n.name
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_flow_sdk::model::{FlowNode, ToolKind};

    #[test]
    fn generates_all_eight_docs() {
        let p = Project::new("demo", Some("t1".into()), 0.7, 0.3, 1.0);
        let mut g = FlowGraph::new("topo:r1", "demo");
        g.add_node(FlowNode::task("a", "抓取销售数据", ToolKind::Http, 300));
        g.add_node(FlowNode::task("b", "清洗对账", ToolKind::Compute, 200));
        let topo = Topology::new(p.id, 0.7, 0.3, 1.0, 0.0, serde_json::to_string(&g).unwrap());
        let tl = TraceLink::new(p.id, "R1", "F1", "B1", "A1", "T1", "C1");
        let links = vec![tl];
        let ctx = DocContext {
            project: &p,
            topology: &topo,
            graph: &g,
            trace_links: &links,
        };
        let dg = DocGenerator::new();
        let docs = dg.generate_docs(&ctx);
        assert_eq!(docs.len(), 8, "应生成 8 份文档");
        for d in &docs {
            assert!(!d.content.is_empty(), "文档内容不应为空");
            assert!(ArtifactKind::parse(&d.kind).is_some(), "kind 应合法");
        }
        // 文档应包含拓扑节点名，证明与拓扑绑定
        let bp = docs.iter().find(|d| d.kind == "business_process").unwrap();
        assert!(bp.content.contains("抓取销售数据"));
    }

    #[test]
    fn export_includes_code_and_ddl() {
        let p = Project::new("demo", Some("t1".into()), 0.7, 0.3, 1.0);
        let mut g = FlowGraph::new("topo:r1", "demo");
        g.add_node(FlowNode::task("a", "抓取销售数据", ToolKind::Http, 300));
        let topo = Topology::new(p.id, 0.7, 0.3, 1.0, 0.0, serde_json::to_string(&g).unwrap());
        let links = vec![TraceLink::new(p.id, "R1", "F1", "B1", "A1", "T1", "C1")];
        let ctx = DocContext {
            project: &p,
            topology: &topo,
            graph: &g,
            trace_links: &links,
        };
        let dg = DocGenerator::new();
        let bundle = dg.export_project(&ctx);
        assert!(bundle.code_skeleton.contains("todo!"));
        assert!(bundle.ddl.contains("CREATE TABLE"));
    }
}
