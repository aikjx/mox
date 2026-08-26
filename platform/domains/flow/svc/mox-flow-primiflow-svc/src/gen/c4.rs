//! 代码骨架 · 由关联图谱自动生成（mox_flow_primiflow_svc::assoc::primiflow_seed）
//! 溯源链路: R1 → F4 → B1 → A1 → T2 → C4
//! 数据设计: S3(Topology)
//! 说明: 需求结构化 + 拓扑涌现（复用 flow_ai κ‑τ 引擎 generate）。
//! 规格: primiflow/SPEC.md（§7 模块 / §10 DoD）

/// 依赖模块: C2
use mox_ai_flow_svc::model::ToolKind;
use mox_ai_flow_svc::primitive::{
    generate, CandidateTopology, KnowledgeBase, PrimitiveState, Requirement as PrimiRequirement,
    SubTask,
};

/// 业务域（用于 κ 检索硬过滤与域白名单）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Domain {
    BusinessSoftware,
    Unknown,
}

impl Domain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Domain::BusinessSoftware => "business_software",
            Domain::Unknown => "unknown",
        }
    }
    pub fn parse(s: &str) -> Domain {
        match s {
            "business_software" => Domain::BusinessSoftware,
            _ => Domain::Unknown,
        }
    }
}

/// 需求结构化结果
#[derive(Debug, Clone)]
pub struct StructuredRequirement {
    pub id: String,
    pub name: String,
    pub domain: Domain,
    /// 拆解出的原始子任务短语（用于六维溯源的 R 维度）
    pub clauses: Vec<String>,
    /// 转换为 κ‑τ 引擎可消费的 Requirement
    pub primi: PrimiRequirement,
}

impl StructuredRequirement {
    /// 业务域白名单：仅 business_software 域放行，其余显式拒绝（收敛幻觉面）
    pub fn is_in_scope(&self) -> bool {
        self.domain == Domain::BusinessSoftware
    }
}

/// 拓扑算子：需求结构化 + 拓扑涌现
#[derive(Debug, Default)]
pub struct TopologyOperator;

impl TopologyOperator {
    pub fn new() -> Self {
        Self
    }
    /// 需求结构化：自然语言 → 子任务树（无 LLM，规则分词 + 关键词分类）
    ///
    /// 仅依赖标点切分与关键词词典，离线可跑；真实环境可替换为 llm‑gateway 提示链，
    /// 但输出契约（`StructuredRequirement`）保持不变。
    pub fn structure_requirement(
        &self,
        text: &str,
        req_id: impl Into<String>,
    ) -> StructuredRequirement {
        let domain = detect_domain(text);
        let clauses: Vec<String> = split_clauses(text);
        let mut primi = PrimiRequirement::new(req_id.into(), truncate_name(text));
        for (i, clause) in clauses.iter().enumerate() {
            let (tool, ms) = classify(clause);
            primi = primi.with_subtask(SubTask::new(format!("st{}", i), clause.clone(), tool, ms));
        }
        // 空需求兜底：至少保留一个入口任务，避免拓扑退化为空
        if primi.subtasks.is_empty() {
            primi = primi.with_subtask(SubTask::new("st0", "处理输入需求", ToolKind::Compute, 200));
        }
        StructuredRequirement {
            id: primi.id.clone(),
            name: primi.name.clone(),
            domain,
            clauses,
            primi,
        }
    }

    /// 拓扑涌现：调用 κ‑τ 引擎 generate（自动复用/探索分叉）
    pub fn emerge_topology(
        &self,
        req: &PrimiRequirement,
        state: &PrimitiveState,
        kb: &KnowledgeBase,
    ) -> CandidateTopology {
        generate(req, state, kb)
    }
}

/// 按中文/英文标点切分需求为子句
fn split_clauses(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in text.chars() {
        if [
            '，', '。', '；', ';', '、', '\n', '；', '.', '！', '!', '？', '?', ',', ':',
        ]
        .contains(&c)
        {
            let t = cur.trim().to_string();
            if !t.is_empty() {
                out.push(t);
            }
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    let t = cur.trim().to_string();
    if !t.is_empty() {
        out.push(t);
    }
    out
}

/// 关键词 → 工具类别 + 预估耗时（毫秒）
fn classify(clause: &str) -> (ToolKind, u64) {
    let rules: &[(&[&str], ToolKind, u64)] = &[
        (
            &["抓取", "采集", "爬", "下载", "拉取", "http", "接口", "调用"],
            ToolKind::Http,
            300,
        ),
        (
            &["入库", "存储", "写入", "保存", "落库", "数据库", "db"],
            ToolKind::Database,
            250,
        ),
        (
            &[
                "清洗", "对账", "计算", "统计", "汇总", "解析", "转换", "抽取",
            ],
            ToolKind::Compute,
            200,
        ),
        (
            &[
                "生成", "报告", "图表", "分析", "摘要", "总结", "写作", "问答", "分类",
            ],
            ToolKind::Llm,
            400,
        ),
        (
            &["读取", "导出", "文件", "excel", "csv", "文档"],
            ToolKind::File,
            150,
        ),
        (&["审批", "人工", "确认", "复核"], ToolKind::Human, 500),
    ];
    for (kws, tool, ms) in rules {
        if kws.iter().any(|kw| clause.contains(*kw)) {
            return (*tool, *ms);
        }
    }
    (ToolKind::Compute, 200)
}

/// 业务域探测：命中业务软件关键词则放行
fn detect_domain(text: &str) -> Domain {
    let kw = [
        "报表", "审批", "流程", "管理", "系统", "订单", "用户", "数据", "接口", "入库", "生成",
    ];
    if kw.iter().any(|k| text.contains(k)) {
        Domain::BusinessSoftware
    } else {
        Domain::Unknown
    }
}

fn truncate_name(text: &str) -> String {
    let t = text.trim();
    let chars: Vec<char> = t.chars().collect();
    if chars.len() > 40 {
        format!("{}…", chars.iter().take(40).collect::<String>())
    } else {
        t.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_flow_svc::primitive::PrimitiveState;

    #[test]
    fn structures_into_subtasks() {
        let op = TopologyOperator::new();
        let s = op.structure_requirement("请抓取销售数据。清洗对账。生成图表报告。", "r1");
        assert_eq!(s.clauses.len(), 3);
        assert!(s.primi.subtasks.len() >= 3);
        assert_eq!(s.primi.subtasks[0].tool, ToolKind::Http);
        assert_eq!(s.primi.subtasks[1].tool, ToolKind::Compute);
        assert_eq!(s.primi.subtasks[2].tool, ToolKind::Llm);
    }

    #[test]
    fn domain_whitelist() {
        let op = TopologyOperator::new();
        let in_scope = op.structure_requirement("做一个报销审批流程系统", "r2");
        assert_eq!(in_scope.domain, Domain::BusinessSoftware);
        assert!(in_scope.is_in_scope());
        let out = op.structure_requirement("帮我写一首诗", "r3");
        assert_eq!(out.domain, Domain::Unknown);
        assert!(!out.is_in_scope());
    }

    #[test]
    fn empty_requirement_gets_fallback() {
        let op = TopologyOperator::new();
        let s = op.structure_requirement("。，；", "r4");
        assert_eq!(s.primi.subtasks.len(), 1);
    }

    #[test]
    fn emerge_produces_acyclic_topology() {
        let op = TopologyOperator::new();
        let s = op.structure_requirement("抓取销售数据。清洗对账。生成报告。", "r5");
        let state =
            PrimitiveState::from_policy(10.0, mox_ai_flow_svc::primitive::DeliveryPolicy::Exploratory, 0.0);
        let topo = op.emerge_topology(&s.primi, &state, &KnowledgeBase::new());
        assert!(topo.graph.topo_order().is_ok());
    }
}
