//! PrimiPlatform 一体化编排层（融合 + 归一化 + 一体化的最终出口）
//!
//! 把已落地的 `primiflow` 八模块主链路（`Orchestrator`）与「多维度融合统一图」
//! （[`UnifiedGraph`]）通过全局治理闸门（守恒 R07 + 六维绑定 A4 + GR-STD 8 闸门）
//! 编织成**一个**可运行、可验证、可溯源的平台。

use crate::envelope::PTEnvelope;
use crate::registry::fuse_all;
use crate::sixdim::{now_ms, SixDimBinding, SixDimRegistry};
use crate::unified::{PlatformGate, PrimitiveCoords, UnifiedGraph};
use primiflow::gen::c1::{Input, OrchestrationResult, OrchestrationStatus, Orchestrator};
use primiflow::gen::schema::Project;

/// 一体化平台：六维绑定注册表（事实源）+ 融合统一图 + 主链路编排器
pub struct PrimiPlatform {
    /// 平台级六维绑定注册表（R06 真身，可累积 / 查询 / 持久化）
    pub registry: SixDimRegistry,
    /// 由注册表 + 能力融合派生的统一图（每次 synthesize 重建）
    pub graph: UnifiedGraph,
    /// 八模块主链路编排器（跨调用累积 κ‑τ 知识）
    orchestrator: Orchestrator,
    /// 注册表落盘路径（None 表示仅内存运行）
    registry_path: Option<std::path::PathBuf>,
}

/// 一次一体化合成的产出
pub struct PlatformReport {
    pub orchestration: OrchestrationResult,
    /// 合成后整图跑全局治理闸门的结果
    pub gate: PlatformGate,
    /// 本次合成新注册进注册表的绑定数
    pub registered: usize,
    /// 本次生成的 PT-DOC 文档数
    pub ptdocs: usize,
}

impl PrimiPlatform {
    /// 启动平台：融合全部 crate 能力 + 初始化主链路编排器 + 空注册表
    pub fn new() -> Self {
        Self {
            registry: SixDimRegistry::new(),
            graph: fuse_all(),
            orchestrator: Orchestrator::new(),
            registry_path: None,
        }
    }

    /// 以指定注册表落盘路径启动（跨重启复用历史绑定）
    pub fn with_persistence(path: std::path::PathBuf) -> Self {
        let registry = SixDimRegistry::load(&path).unwrap_or_default();
        let mut p = Self::new();
        p.registry = registry;
        p.registry_path = Some(path);
        p.rebuild_graph();
        p
    }

    /// 一体化合成：跑主链路 → 登记进六维注册表 → 重算统一图 → 跑全局闸门
    pub fn synthesize(&mut self, requirement: &str, slider_s: f64) -> PlatformReport {
        let proj_name = truncate(&format!("proj:{requirement}"), 48);
        let project = Project::new(&proj_name, Some("t1".into()), 0.7, 0.3, 1.0);
        let result = self
            .orchestrator
            .run(&project, &Input::Text(requirement.into()), slider_s);

        let binding = build_binding(&project, &result);
        let registered = if self.registry.by_requirement(&binding.req_id).is_some() {
            0
        } else {
            1
        };
        self.registry.register(binding);
        self.persist_registry();

        self.rebuild_graph();
        let gate = self.graph.full_gate();

        PlatformReport {
            orchestration: result,
            gate,
            registered,
            ptdocs: 0,
        }
    }

    /// 合成并自动导出 PT-DOC 标准文档集到目录
    pub fn synthesize_and_emit_docs(
        &mut self,
        requirement: &str,
        slider_s: f64,
        doc_dir: &std::path::Path,
    ) -> PlatformReport {
        let mut rep = self.synthesize(requirement, slider_s);
        let set = crate::ptdoc::PtdocSet::generate(&self.registry, &rep.gate, &self.graph);
        let _ = set.export(doc_dir);
        rep.ptdocs = set.docs.len();
        rep
    }

    /// 把注册表 + 能力融合重建成统一图（事实源：注册表累积的运行时绑定 ∪ 静态能力融合）
    fn rebuild_graph(&mut self) {
        let mut g = fuse_all();
        let rg = self.registry.to_unified_graph();
        for (id, n) in rg.nodes {
            g.nodes.entry(id).or_insert(n);
        }
        g.edges.extend(rg.edges);
        self.graph = g;
    }

    fn persist_registry(&self) {
        if let Some(path) = &self.registry_path {
            let _ = self.registry.save(path);
        }
    }

    /// 取一张贯穿 L1→L7 的归一化信封示例（证明跨层消息一体化）
    pub fn sample_envelope(requirement: &str) -> PTEnvelope {
        use crate::unified::Layer;
        PTEnvelope::new(
            format!("trace:{}", requirement.len()),
            Layer::RequirementSemantic,
            Layer::Governance,
            PrimitiveCoords::from_kt(0.7, 0.3),
            serde_json::json!({ "requirement": requirement }),
            vec!["REQ-1".into(), "FUN-1".into(), "BIZ-1".into()],
        )
    }
}

impl Default for PrimiPlatform {
    fn default() -> Self {
        Self::new()
    }
}

fn requirement_text(r: &OrchestrationResult) -> String {
    r.requirement
        .as_ref()
        .map(|s| s.name.clone())
        .unwrap_or_else(|| "<未结构化>".into())
}

/// 把一次主链路产出构造成六维绑定记录（六维实体 id 全部以 project.id 派生，保证
/// 跨多次 synthesize 唯一，使注册表累积图守恒闭合、无共享节点串扰）。
fn build_binding(project: &Project, result: &OrchestrationResult) -> SixDimBinding {
    let pid = project.id.to_string();
    let (kappa, tau, c) = result
        .state
        .as_ref()
        .map(|s| (s.kappa, s.tau, s.c))
        .unwrap_or((0.0, 0.0, 0.0));
    let q = if result.status == OrchestrationStatus::Completed {
        1.0
    } else {
        0.0
    };
    let coords = if c > 0.0 {
        PrimitiveCoords { kappa, tau, c, q }
    } else {
        PrimitiveCoords::from_kt(kappa, tau)
    };
    let topo_nodes = result.graph.as_ref().map(|g| g.nodes.len()).unwrap_or(0);
    SixDimBinding {
        req_id: format!("REQ:{pid}"),
        req_text: requirement_text(result),
        project_id: pid.clone(),
        status: format!("{:?}", result.status),
        coords,
        requirement: format!("REQ:{pid}"),
        feature: format!("FUN:{pid}"),
        business: format!("BIZ:{pid}"),
        algorithm: format!("ALG:{pid}"),
        task: format!("TSK:{pid}"),
        code: format!("COD:{pid}"),
        topo_nodes,
        timestamp_ms: now_ms(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        let t: String = s.chars().take(max).collect();
        format!("{t}…")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_synthesize_completes_and_passes_gate() {
        let mut p = PrimiPlatform::new();
        let rep = p.synthesize("请抓取销售数据。清洗对账。生成图表报告。", 0.2);
        assert_eq!(rep.orchestration.status, OrchestrationStatus::Completed);
        assert!(rep.registered >= 1, "应至少注册 1 条六维绑定");
        assert!(
            rep.gate.passed,
            "一体化合成后整图应通过全局闸门：{:?}",
            rep.gate
        );
    }

    #[test]
    fn platform_reports_out_of_domain() {
        let mut p = PrimiPlatform::new();
        let rep = p.synthesize("帮我写一首关于春天的诗", 0.2);
        assert_eq!(
            rep.orchestration.status,
            OrchestrationStatus::RejectedDomain
        );
        // 即便拒绝，需求节点已注册，整图仍应满足治理闸门（无悬空/孤儿）
        assert!(rep.gate.passed);
    }

    #[test]
    fn platform_envelope_is_conserved() {
        let env = PrimiPlatform::sample_envelope("抓取销售数据生成报告");
        assert!(env.is_conserved(1e-3));
        assert_eq!(env.bind_ids.len(), 3);
    }

    #[test]
    fn platform_accumulates_bindings_and_keeps_gate_green() {
        let mut p = PrimiPlatform::new();
        let r1 = p.synthesize("请抓取销售数据。清洗对账。生成图表报告。", 0.2);
        let r2 = p.synthesize(
            "我想做一份电商经营分析报告，需要销售数据抓取和图表生成。",
            0.1,
        );
        assert_eq!(p.registry.len(), 2, "两次 synthesize 应累积 2 条绑定");
        assert!(r1.gate.passed && r2.gate.passed);
        // 累积图（能力融合 ∪ 2 条运行时绑定）仍通过全闸门
        assert!(p.graph.full_gate().passed, "累积统一图应通过全局闸门");
    }

    #[test]
    fn platform_emits_ptdoc_and_persists() {
        let dir = std::env::temp_dir().join("primiflow_fusion_test_platform_doc");
        let reg_path = std::env::temp_dir().join("primiflow_fusion_test_platform_reg.json");
        let _ = std::fs::remove_file(&reg_path);

        let mut p = PrimiPlatform::with_persistence(reg_path.clone());
        let rep = p.synthesize_and_emit_docs("请抓取销售数据。清洗对账。生成图表报告。", 0.2, &dir);
        assert!(rep.registered >= 1);
        assert_eq!(rep.ptdocs, 10, "应生成 10 份 PT-DOC");
        assert!(dir.join("PT-DOC-01.md").exists());
        assert!(dir.join("INDEX.md").exists());

        // 跨重启：从同一落盘文件恢复注册表，绑定应保留
        let restored = PrimiPlatform::with_persistence(reg_path.clone());
        assert_eq!(restored.registry.len(), 1, "重启后应恢复 1 条绑定");

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&reg_path);
    }

    /// P3 出口闸认证（骨架 §6）：六维零孤儿 + 连通 REQ。
    /// 直接把规范出口标准编码为可机器持续校验的断言，使 P3 认证可回归。
    #[test]
    fn p3_exit_gate_zero_orphan_connected_to_req() {
        use crate::unified::{EntityKind, RelKind};

        let mut p = PrimiPlatform::new();
        p.synthesize("请抓取销售数据。清洗对账。生成图表报告。", 0.2);
        p.synthesize(
            "我想做一份电商经营分析报告，需要销售数据抓取和图表生成。",
            0.1,
        );

        // 1) 六维零孤儿（A4）
        let binding = p.graph.binding_report();
        assert!(binding.passed, "六维应零孤儿，实际：{:?}", binding.orphans);
        assert!(binding.six_dim_nodes > 0, "应存在六维实体节点");

        // 2) 连通 REQ：每个声明 bind_id 的六维实体都应沿 Bind 链回溯到某 REQ 根
        let bind_kinds = [RelKind::Bind];
        for n in p.graph.nodes.values() {
            if !n.kind.is_six_dim() || n.bind_id.is_none() {
                continue;
            }
            let is_req = n.kind == EntityKind::Requirement;
            let reaches_req = is_req
                || p.graph.upstream_ids(&n.id, &bind_kinds).iter().any(|u| {
                    p.graph
                        .node(u)
                        .map(|m| m.kind == EntityKind::Requirement)
                        .unwrap_or(false)
                });
            assert!(reaches_req, "六维实体 {} 未连通到 REQ 根", n.id);
        }

        // 3) 数据表已挂接（R07/R08「数据表挂接 crate」），消除孤岛
        for t in [
            "PROJECTS",
            "CONVERSATIONS",
            "TOPOLOGYS",
            "ASSETS",
            "ARTIFACTS",
            "TRACE_LINKS",
        ] {
            let id = format!("store:{t}");
            assert!(p.graph.node(&id).is_some(), "缺数据表 {t}");
            assert!(
                p.graph.edges.iter().any(|e| e.from == id || e.to == id),
                "数据表 {t} 仍为孤岛"
            );
        }

        // 4) 平台级全闸门通过（守恒 + 绑定 + 治理）
        assert!(p.graph.full_gate().passed, "P3 出口闸应整体通过");
    }
}
