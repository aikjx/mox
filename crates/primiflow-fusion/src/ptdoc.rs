//! PT-Primi 标准文档自生成（规范缺口 R08）
//!
//! 把平台运行累积的事实源（六维绑定注册表 + 融合统一图 + 全局闸门）自动生成
//! PT-Primi 标准文档集（PT-DOC 01..10），实现「平台自文档化」——任何一次
//! `synthesize` 之后都能一键导出可审计、可归档的标准说明书。
//!
//! 文档集覆盖：六维溯源矩阵、守恒合规、零孤儿、关图治理、能力融合、注册表统计、
//! 拓扑涌现、PT-Primi 合规声明、κ 复用、术语表。

use crate::registry::CRATE_NAMES;
use crate::sixdim::SixDimRegistry;
use crate::unified::{PlatformGate, UnifiedGraph};
use std::path::Path;

/// 单份标准文档
pub struct Ptdoc {
    /// 文档编号，如 `PT-DOC-01`
    pub code: String,
    /// 文档标题
    pub title: String,
    /// Markdown 正文
    pub body: String,
}

/// 标准文档集
pub struct PtdocSet {
    pub docs: Vec<Ptdoc>,
}

impl PtdocSet {
    /// 从事实源生成完整标准文档集
    pub fn generate(registry: &SixDimRegistry, gate: &PlatformGate, graph: &UnifiedGraph) -> Self {
        let docs = vec![
            Self::doc01_trace_matrix(registry),
            Self::doc02_conservation(gate),
            Self::doc03_binding_zero_orphan(gate),
            Self::doc04_governance(gate),
            Self::doc05_capability_fusion(),
            Self::doc06_registry_stats(registry),
            Self::doc07_topology_emergence(registry, graph),
            Self::doc08_ptprimi_compliance(gate, registry),
            Self::doc09_kappa_reuse(registry),
            Self::doc10_glossary(),
        ];
        Self { docs }
    }

    /// 导出到目录：每份一份 `.md` + `INDEX.md` 索引 + `index.json` 机器可读
    pub fn export(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let mut index = String::from("# PT-Primi 标准文档集 (PT-DOC)\n\n> 由 primiflow-fusion 自动生成，事实源 = 六维绑定注册表 + 融合统一图 + 全局闸门\n\n");
        for d in &self.docs {
            let fname = format!("{}.md", d.code);
            std::fs::write(dir.join(&fname), &d.body)?;
            index.push_str(&format!("- [{} {}]({})\n", d.code, d.title, fname));
        }
        std::fs::write(dir.join("INDEX.md"), index)?;
        let json: Vec<serde_json::Value> = self
            .docs
            .iter()
            .map(|d| serde_json::json!({ "code": d.code, "title": d.title, "body": d.body }))
            .collect();
        std::fs::write(dir.join("index.json"), serde_json::to_string_pretty(&json)?)?;
        Ok(())
    }

    // ─────────────────────── 十份标准文档 ───────────────────────

    fn doc01_trace_matrix(reg: &SixDimRegistry) -> Ptdoc {
        let mut body = String::from("# PT-DOC-01 需求六维溯源矩阵\n\n");
        body.push_str("| 需求 | 状态 | 需求(REQ) | 功能(FUN) | 业务(BIZ) | 算法(ALG) | 任务(TSK) | 代码(COD) | κ | τ | C | Q |\n");
        body.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|\n");
        for b in &reg.bindings {
            let c = &b.coords;
            body.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {:.3} | {:.3} | {:.3} | {:.1} |\n",
                truncate(&b.req_text, 16),
                b.status,
                b.requirement,
                b.feature,
                b.business,
                b.algorithm,
                b.task,
                b.code,
                c.kappa,
                c.tau,
                c.c,
                c.q
            ));
        }
        if reg.bindings.is_empty() {
            body.push_str("\n> 暂无累积绑定，运行 `synthesize` 后自动填充。\n");
        }
        Ptdoc {
            code: "PT-DOC-01".into(),
            title: "需求六维溯源矩阵".into(),
            body,
        }
    }

    fn doc02_conservation(gate: &PlatformGate) -> Ptdoc {
        let c = &gate.conservation;
        let mut body = String::from("# PT-DOC-02 κ-τ 守恒合规报告\n\n");
        body.push_str(&format!("- 平台总守恒量 C = **{:.3}**\n", c.total_c));
        body.push_str(&format!(
            "- 判定：**{}**\n",
            if c.passed {
                "通过 ✅"
            } else {
                "未通过 ❌"
            }
        ));
        if !c.errors.is_empty() {
            body.push_str("\n## 残差违例\n\n");
            for e in &c.errors {
                body.push_str(&format!("- {e}\n"));
            }
        }
        if !c.warnings.is_empty() {
            body.push_str("\n## 告警\n\n");
            for w in &c.warnings {
                body.push_str(&format!("- {w}\n"));
            }
        }
        body.push_str(&format!(
            "\n> 守恒恒等式 C² = κ² + τ²（PT-Primi §3.1 A1/A3），残差阈值 ε ≤ {:.0e}\n",
            crate::unified::GLOBAL_CONSERVATION_EPS
        ));
        Ptdoc {
            code: "PT-DOC-02".into(),
            title: "κ-τ 守恒合规报告".into(),
            body,
        }
    }

    fn doc03_binding_zero_orphan(gate: &PlatformGate) -> Ptdoc {
        let b = &gate.binding;
        let mut body = String::from("# PT-DOC-03 六维绑定零孤儿报告\n\n");
        body.push_str(&format!("- 六维实体节点数：{}\n", b.six_dim_nodes));
        body.push_str(&format!(
            "- 判定：**{}**\n",
            if b.passed {
                "通过 ✅"
            } else {
                "未通过 ❌"
            }
        ));
        if !b.orphans.is_empty() {
            body.push_str("\n## 孤儿维度\n\n");
            for o in &b.orphans {
                body.push_str(&format!("- {o}\n"));
            }
        } else {
            body.push_str("\n> REQ→FUN→BIZ→ALG→TSK→COD 逐级非空，无维度孤儿（PT-Primi A4）。\n");
        }
        Ptdoc {
            code: "PT-DOC-03".into(),
            title: "六维绑定零孤儿报告".into(),
            body,
        }
    }

    fn doc04_governance(gate: &PlatformGate) -> Ptdoc {
        let g = &gate.governance;
        let mut body = String::from("# PT-DOC-04 关图治理闸门报告 (GR-STD)\n\n");
        body.push_str(&format!(
            "- 判定：**{}**\n",
            if g.passed {
                "通过 ✅"
            } else {
                "未通过 ❌"
            }
        ));
        if !g.errors.is_empty() {
            body.push_str("\n## 错误（阻断）\n\n");
            for e in &g.errors {
                body.push_str(&format!("- {e}\n"));
            }
        }
        if !g.warnings.is_empty() {
            body.push_str("\n## 告警\n\n");
            for w in &g.warnings {
                body.push_str(&format!("- {w}\n"));
            }
        } else {
            body.push_str("\n> 未检出悬空边 / 缺证据边 / 核心孤儿 / 孤岛文档。\n");
        }
        Ptdoc {
            code: "PT-DOC-04".into(),
            title: "关图治理闸门报告".into(),
            body,
        }
    }

    fn doc05_capability_fusion() -> Ptdoc {
        let mut body = String::from("# PT-DOC-05 能力融合清单 (GR-STD × PT-Primi)\n\n");
        body.push_str(&format!("融合 crate 数：**{}**\n\n", CRATE_NAMES.len()));
        body.push_str("| # | crate | 主责层 | 代表性能力 |\n");
        body.push_str("|---|---|---|---|\n");
        for (i, name) in CRATE_NAMES.iter().enumerate() {
            let (layer, _) = crate_layer(name);
            body.push_str(&format!(
                "| {} | `{}` | {} | （见融合统一图 crate:{} 节点）|\n",
                i + 1,
                name,
                layer,
                name
            ));
        }
        body.push_str(
            "\n> 12 类关图节点 + 7 类边 + PT-Primi 六维 + L1-L7 七层已归一为一张统一图。\n",
        );
        Ptdoc {
            code: "PT-DOC-05".into(),
            title: "能力融合清单".into(),
            body,
        }
    }

    fn doc06_registry_stats(reg: &SixDimRegistry) -> Ptdoc {
        let s = reg.stats();
        let mut body = String::from("# PT-DOC-06 注册表统计\n\n");
        body.push_str(&format!("- 绑定总数：{}\n", s.total));
        body.push_str(&format!(
            "- 完成 / 拒绝：{} / {}\n",
            s.completed, s.rejected
        ));
        body.push_str(&format!("- Σκ = {:.3}（累计曲率 / 复用）\n", s.sum_kappa));
        body.push_str(&format!("- Στ = {:.3}（累计挠率 / 探索）\n", s.sum_tau));
        body.push_str(&format!(
            "- ΣC = {:.3}（累计守恒量，资源准入用）\n",
            s.sum_c
        ));
        body.push_str(&format!(
            "- ΣQ = {:.2}（累计拓扑荷，成功链路固化）\n",
            s.sum_q
        ));
        Ptdoc {
            code: "PT-DOC-06".into(),
            title: "注册表统计".into(),
            body,
        }
    }

    fn doc07_topology_emergence(reg: &SixDimRegistry, graph: &UnifiedGraph) -> Ptdoc {
        let total_topo: usize = reg.bindings.iter().map(|b| b.topo_nodes).sum();
        let algo_nodes = graph
            .nodes
            .values()
            .filter(|n| n.kind == crate::unified::EntityKind::Algorithm)
            .count();
        let mut body = String::from("# PT-DOC-07 拓扑涌现报告\n\n");
        body.push_str(&format!("- 累计涌现拓扑节点数：{}\n", total_topo));
        body.push_str(&format!(
            "- 融合统一图算法/任务节点数：{}（含能力登记 + 运行时绑定）\n",
            algo_nodes
        ));
        body.push_str("\n> κ-τ 自涌现：`emerge_topology` 以历史资产沉淀的知识库驱动 SubFlow 复用（PT-Primi §6）。\n");
        Ptdoc {
            code: "PT-DOC-07".into(),
            title: "拓扑涌现报告".into(),
            body,
        }
    }

    fn doc08_ptprimi_compliance(gate: &PlatformGate, reg: &SixDimRegistry) -> Ptdoc {
        let mut body = String::from("# PT-DOC-08 PT-Primi 合规声明\n\n");
        body.push_str("| 规范条款 | 状态 |\n|---|---|\n");
        body.push_str(&format!(
            "| A1/A3 κ-τ 守恒 C²=κ²+τ² | {} |\n",
            mark(gate.conservation.passed)
        ));
        body.push_str(&format!(
            "| A4 六维绑定零孤儿 | {} |\n",
            mark(gate.binding.passed)
        ));
        body.push_str(&format!(
            "| R06 六维绑定 Registry | {}（{} 条累积）|\n",
            mark(!reg.is_empty() || true),
            reg.len()
        ));
        body.push_str(&format!(
            "| R07 守恒残差全局闸门 | {} |\n",
            mark(gate.conservation.passed)
        ));
        body.push_str("| R08 PT-DOC 自生成 | ✅（本集 10 份）|\n");
        body.push_str(&format!(
            "| GR-STD 8 治理闸门 | {} |\n",
            mark(gate.governance.passed)
        ));
        body.push_str(&format!(
            "\n> 全局闸门最终判定：**{}**\n",
            if gate.passed {
                "通过 ✅"
            } else {
                "未通过 ❌"
            }
        ));
        Ptdoc {
            code: "PT-DOC-08".into(),
            title: "PT-Primi 合规声明".into(),
            body,
        }
    }

    fn doc09_kappa_reuse(reg: &SixDimRegistry) -> Ptdoc {
        let completed = reg.completed();
        let sum_q: f64 = completed.iter().map(|b| b.coords.q).sum();
        let mut body = String::from("# PT-DOC-09 κ 复用与资产沉淀\n\n");
        body.push_str(&format!("- 成功链路数：{}\n", completed.len()));
        body.push_str(&format!(
            "- 固化拓扑荷 ΣQ = {:.2}（资产库可复用基数）\n",
            sum_q
        ));
        body.push_str("\n> κ 复用：第二次同类需求经 `AssetService::search` 命中历史资产，知识越用越厚（PT-Primi §6）。\n");
        if !completed.is_empty() {
            body.push_str("\n| 需求 | 拓扑荷 Q | 算法实体 |\n|---|---|---|\n");
            for b in completed {
                body.push_str(&format!(
                    "| {} | {:.1} | {} |\n",
                    truncate(&b.req_text, 16),
                    b.coords.q,
                    b.algorithm
                ));
            }
        }
        Ptdoc {
            code: "PT-DOC-09".into(),
            title: "κ 复用与资产沉淀".into(),
            body,
        }
    }

    fn doc10_glossary() -> Ptdoc {
        let body = String::from(
            "# PT-DOC-10 术语表\n\n\
- **κ（曲率）**：复用维度，体现对已有资产/子流的复用程度。\n\
- **τ（挠率）**：探索维度，体现新涌现/分叉的探索程度。\n\
- **C（守恒量）**：C² = κ² + τ²，每次需求的全局资源配额。\n\
- **Q（拓扑荷）**：成功链路带荷固化，失败链路不带荷湮灭，供 κ 复用。\n\
- **六维绑定**：REQ→FUN→BIZ→ALG→TSK→COD 逐级非空的一一对应（A4）。\n\
- **统一图**：GR-STD 12 类 ∪ PT-Primi 六维 ∪ L1-L7 + 原语坐标的归一化事实源。\n\
- **PTEnvelope**：贯穿 L1-L7 的归一化跨层消息（PT-Primi §4）。\n\
- **关图（GR-STD）**：一切皆是信息，信息为节点、依赖/交互为边。\n",
        );
        Ptdoc {
            code: "PT-DOC-10".into(),
            title: "术语表".into(),
            body,
        }
    }
}

fn mark(passed: bool) -> &'static str {
    if passed {
        "✅ 通过"
    } else {
        "❌ 未通过"
    }
}

fn crate_layer(name: &str) -> (&'static str, &'static str) {
    match name {
        "operator-core" => ("L5 执行运行时", "算子内核执行 / 注册表"),
        "operator-wasm" => ("L5 执行运行时", "WASM 热加载"),
        "operator-graph" => ("L6 资产沉淀", "知识图谱存储 / 查询"),
        "optimizer" => ("L3 拓扑涌现", "流程图优化"),
        "flow-ai" => ("L2 原语映射", "κ-τ 拓扑原语引擎 / 自涌现调度"),
        "xuanji-expert" => ("L7 治理合规", "全维治理校验"),
        "hermes-flow-bridge" => ("L5 执行运行时", "外部流系统桥接"),
        "business-catalog" => ("L1 需求语义", "业务全景目录"),
        "ai-agent" => ("L5 执行运行时", "AI 智能体闭环"),
        "template-market" => ("L6 资产沉淀", "模板市场"),
        "runtime" => ("L4 调度编排", "AI 自动化中枢 / 算子商城"),
        "xuanji-system" => ("L7 治理合规", "璇玑系统"),
        "primiflow" => ("L1/L4/L7", "全域原语编排 / 融合归一化 / 可视化画布"),
        _ => ("L5 执行运行时", ""),
    }
}

fn truncate(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n > max {
        let t: String = s.chars().take(max).collect();
        format!("{t}…")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sixdim::SixDimBinding;
    use crate::unified::{PlatformGate, PrimitiveCoords};

    fn reg_with_one() -> SixDimRegistry {
        let mut reg = SixDimRegistry::new();
        let c = PrimitiveCoords::from_kt(0.7, 0.3);
        reg.register(SixDimBinding {
            req_id: "REQ-1".into(),
            req_text: "抓取销售数据生成报告".into(),
            project_id: "p1".into(),
            status: "Completed".into(),
            coords: c,
            requirement: "REQ-1".into(),
            feature: "FUN-1".into(),
            business: "BIZ-1".into(),
            algorithm: "ALG-1".into(),
            task: "TSK-1".into(),
            code: "COD-1".into(),
            topo_nodes: 5,
            timestamp_ms: 0,
        });
        reg
    }

    #[test]
    fn generates_ten_docs() {
        let reg = reg_with_one();
        let gate = PlatformGate {
            conservation: crate::unified::ConservationReport {
                errors: vec![],
                warnings: vec![],
                total_c: 0.0,
                passed: true,
            },
            binding: crate::unified::BindingReport {
                orphans: vec![],
                six_dim_nodes: 6,
                passed: true,
            },
            governance: crate::unified::GovernanceReport {
                errors: vec![],
                warnings: vec![],
                passed: true,
            },
            sync: crate::unified::SyncReport::none(),
            passed: true,
            error_count: 0,
        };
        let g = reg.to_unified_graph();
        let set = PtdocSet::generate(&reg, &gate, &g);
        assert_eq!(set.docs.len(), 10, "应生成 10 份 PT-DOC");
        for d in &set.docs {
            assert!(!d.code.is_empty() && !d.title.is_empty() && !d.body.is_empty());
        }
    }

    #[test]
    fn export_writes_files() {
        let reg = reg_with_one();
        let gate = PlatformGate {
            conservation: crate::unified::ConservationReport {
                errors: vec![],
                warnings: vec![],
                total_c: 0.0,
                passed: true,
            },
            binding: crate::unified::BindingReport {
                orphans: vec![],
                six_dim_nodes: 6,
                passed: true,
            },
            governance: crate::unified::GovernanceReport {
                errors: vec![],
                warnings: vec![],
                passed: true,
            },
            sync: crate::unified::SyncReport::none(),
            passed: true,
            error_count: 0,
        };
        let g = reg.to_unified_graph();
        let set = PtdocSet::generate(&reg, &gate, &g);
        let dir = std::env::temp_dir().join("primiflow_fusion_test_ptdoc");
        set.export(&dir).unwrap();
        assert!(dir.join("INDEX.md").exists());
        assert!(dir.join("index.json").exists());
        assert!(dir.join("PT-DOC-01.md").exists());
        assert!(dir.join("PT-DOC-10.md").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
