// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 归一化 IR 扩展：在 `mox_ai_flow_svc` 的 FlowGraph 之上叠加「维度」着色。
//!
//! 设计铁律：四种流程图（业务/算法/权限/资源）在内存里是**同一个 FlowGraph**，
//! 维度只是节点/边上的标签。物理节点唯一，因此「改一处，全维同步」天然成立。

use mox_ai_flow_svc::model::{FlowGraph, NodeKind, ToolKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 七个优化维度（璇玑的七个镜头）
///
/// 业务七维（分析 FlowGraph）+ 开发七维（分析 CodeIR）= 共十四专家，
/// 复用同一 `Expert` 引擎 / `reconcile` 裁决 / `verify` 验证 / `govern` 闸门 / SHA-256 审计链。
/// 业务与开发璇玑**非冗余、互补**：前者管「做什么/是否合规」，后者管「怎么做/是否优质」。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    // ---- 业务七维（分析流程图）----
    Business,
    Algorithm,
    Permission,
    Resource,
    Security,
    Data,
    Observability,
    // ---- 开发七维（分析代码）----
    Architecture,
    SecurityCode,
    CodeQuality,
    Performance,
    Testing,
    Documentation,
    Maintainability,
}

impl Dimension {
    /// 全维裁决时的优先级权重：值越大越优先被采纳（权限/安全不可被性能绕过）。
    /// 单一数据源：`crate::DIM_PRIORITY`（集中常量，避免散落魔法数字）。
    pub fn priority(&self) -> u8 {
        crate::dim_priority(*self) as u8
    }

    /// 是否为「开发璇玑」维度（分析代码，而非流程图）
    pub fn is_code_dimension(&self) -> bool {
        matches!(
            self,
            Dimension::Architecture
                | Dimension::SecurityCode
                | Dimension::CodeQuality
                | Dimension::Performance
                | Dimension::Testing
                | Dimension::Documentation
                | Dimension::Maintainability
        )
    }

    /// 是否为「业务璇玑」维度（分析流程图）
    pub fn is_business_dimension(&self) -> bool {
        matches!(
            self,
            Dimension::Business
                | Dimension::Algorithm
                | Dimension::Permission
                | Dimension::Resource
                | Dimension::Security
                | Dimension::Data
                | Dimension::Observability
        )
    }
}

/// 代码中间表示：开发璇玑的分析对象（对应业务流程图的 FlowGraph）
///
/// 与 `FlowGraph` 平行：`FlowGraph` 描述「做什么」，[`CodeIR`] 描述「怎么做」。
/// 二者经同一 `Expert` 引擎并行分析，互不覆盖。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CodeIR {
    /// 代码单元（函数/模块/类）列表
    pub units: Vec<CodeUnit>,
}

/// 单个代码单元的可分析属性（属性模型见《时间业务流程关系图元模型规范》）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeUnit {
    /// 唯一标识，如 `crate::module::fn_name`
    pub id: String,
    /// 函数/模块名称
    pub name: String,
    /// 语言：`rust` / `python` / ...
    pub language: String,
    /// 代码行数
    pub lines_of_code: usize,
    /// 源代码（用于模式分析）
    pub source_code: String,
    /// 圈复杂度
    pub cyclomatic_complexity: u32,
    /// 依赖列表
    pub dependencies: Vec<String>,
    /// 是否公共API
    pub is_public: bool,
    /// 是否入口模块
    pub is_entry_point: bool,
    /// 测试覆盖率 0..1
    pub test_coverage: f64,
    /// 测试用例列表
    pub test_cases: Vec<String>,
    /// 是否有集成测试
    pub has_integration_tests: bool,
    /// 注释行数
    pub comment_lines: usize,
    /// 是否有README
    pub has_readme: bool,
    /// 代码重复率 0..1
    pub duplication_score: f64,
    /// 是否有过去依赖
    pub has_outdated_deps: bool,
    /// 是否硬编码密钥/令牌
    pub hardcoded_secret: bool,
    /// 是否存在 SQL 注入风险（拼接 SQL）
    pub sql_injection_risk: bool,
    /// 是否使用弱哈希（md5/sha1 做密码）
    pub weak_hash: bool,
    /// 是否存在 N+1 查询
    pub n_plus_one: bool,
    /// 是否无测试覆盖
    pub uncovered: bool,
    /// 耦合度 0..1（入/出耦合失衡）
    pub coupling: f64,
    /// 是否缺少文档注释（公开 API）
    pub has_doc: bool,
    /// 创建/更新时间（ISO-8601，时间记录支柱）
    pub updated_at: Option<String>,
}

impl CodeIR {
    pub fn new(units: Vec<CodeUnit>) -> Self {
        Self { units }
    }
    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }
}

/// 节点上的维度着色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionTag {
    pub dimension: Dimension,
    pub owner_expert: ExpertId,
    pub policy_refs: Vec<PolicyId>,
    /// 该维度在此节点的相对重要性 0..1
    pub weight: f64,
}

/// 扩展后的流程图：持有原始 mox_ai_flow_svc 图 + 维度标注
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionedFlow {
    pub base: FlowGraph,
    /// node_id -> 维度标签
    pub node_dimensions: HashMap<String, Vec<DimensionTag>>,
}

impl DimensionedFlow {
    pub fn from_base(base: FlowGraph) -> Self {
        Self {
            base,
            node_dimensions: HashMap::new(),
        }
    }

    pub fn tag(&mut self, node_id: impl Into<String>, tag: DimensionTag) {
        self.node_dimensions
            .entry(node_id.into())
            .or_default()
            .push(tag);
    }

    pub fn dimensions_of(&self, node_id: &str) -> &[DimensionTag] {
        self.node_dimensions
            .get(node_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

/// 专家标识
pub type ExpertId = String;
/// 策略标识
pub type PolicyId = String;

/// 把业务/算法/权限/资源四类图的语义标签挂到归一化图上。
///
/// 这是一个示例性的「维度着色器」：把 `tags` 中含 `dim:<x>` 的节点标到对应维度，
/// 让后续专家能按维度分镜评估。真实系统可由设计器或 AI 自动标注。
pub fn auto_dimension(base: &FlowGraph) -> DimensionedFlow {
    let mut df = DimensionedFlow::from_base(base.clone());
    for n in &base.nodes {
        // 默认所有节点至少属于业务维度
        df.tag(
            n.id.clone(),
            DimensionTag {
                dimension: Dimension::Business,
                owner_expert: "business".into(),
                policy_refs: Vec::new(),
                weight: 1.0,
            },
        );
        for t in &n.tags {
            if let Some(dim) = t.strip_prefix("dim:") {
                let (dim, expert) = match dim {
                    "algo" => (Dimension::Algorithm, "algorithm"),
                    "perm" => (Dimension::Permission, "permission"),
                    "res" => (Dimension::Resource, "resource"),
                    "sec" => (Dimension::Security, "security"),
                    "data" => (Dimension::Data, "data"),
                    "obs" => (Dimension::Observability, "observability"),
                    _ => continue,
                };
                df.tag(
                    n.id.clone(),
                    DimensionTag {
                        dimension: dim,
                        owner_expert: expert.into(),
                        policy_refs: Vec::new(),
                        weight: 1.0,
                    },
                );
            }
        }
        // 工具类自动带资源维度；LLM 带算法维度
        match n.tool {
            Some(ToolKind::Llm) => {
                df.tag(
                    n.id.clone(),
                    DimensionTag {
                        dimension: Dimension::Algorithm,
                        owner_expert: "algorithm".into(),
                        policy_refs: Vec::new(),
                        weight: 1.0,
                    },
                );
            }
            Some(_) => {
                df.tag(
                    n.id.clone(),
                    DimensionTag {
                        dimension: Dimension::Resource,
                        owner_expert: "resource".into(),
                        policy_refs: Vec::new(),
                        weight: 1.0,
                    },
                );
            }
            None => {}
        }
        if n.kind == NodeKind::Guard {
            df.tag(
                n.id.clone(),
                DimensionTag {
                    dimension: Dimension::Permission,
                    owner_expert: "permission".into(),
                    policy_refs: Vec::new(),
                    weight: 1.0,
                },
            );
        }
    }
    df
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_flow_svc::model::{FlowEdge, FlowNode, NodeKind, ToolKind};

    #[test]
    fn priority_ordering() {
        assert!(Dimension::Permission.priority() > Dimension::Algorithm.priority());
        assert!(Dimension::Security.priority() >= Dimension::Resource.priority());
    }

    #[test]
    fn auto_dimension_tags_llm_as_algo() {
        let mut g = FlowGraph::new("x", "t");
        g.add_node(FlowNode::task("n1", "推理", ToolKind::Llm, 100));
        let df = auto_dimension(&g);
        let tags = df.dimensions_of("n1");
        assert!(tags.iter().any(|t| t.dimension == Dimension::Algorithm));
        assert!(tags.iter().any(|t| t.dimension == Dimension::Business));
    }

    #[test]
    fn auto_dimension_explicit_tag() {
        let mut g = FlowGraph::new("x", "t");
        let mut n = FlowNode::task("n1", "取数", ToolKind::Browser, 200);
        n.tags.push("dim:sec".into());
        g.add_node(n);
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_edge(FlowEdge::seq("s", "n1"));
        let df = auto_dimension(&g);
        assert!(df
            .dimensions_of("n1")
            .iter()
            .any(|t| t.dimension == Dimension::Security));
    }
}
