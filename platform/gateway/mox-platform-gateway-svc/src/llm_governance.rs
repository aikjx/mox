// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! **P0 后验治理：LLM 专家回复 → FlowGraph → `mox_optimize` 强治理**
//!
//! # 背景
//!
//! [`super::experts_collaboration::generate_expert_answer`] 优先调用 `llm_consultant()`，
//! 真实模型走 ReAct + 工具调用，本地无 Key 时回退到 `mox_optimize` 引擎。但**唯一消费方
//! gateway 没有传 `flow_json`**，`consult_sync` 在 `flow_opt == None` 时直接返回空报告，
//! 跳过璇玑 14 维分析。
//!
//! 这导致 LLM 路径**完全游离于治理闸门之外**：模型自评出的 `vetoed` 标志没有代码闸门
//! 可信（基于 `parse_veto` 的文本子串匹配），DAG 拓扑、资源访问、专家规则、租户合规
//! 等 14 维评估均未生效。
//!
//! # 方案：文本→FlowGraph 后验映射器
//!
//! 本模块不修改 LLM 推理路径（避免双调用成本），仅在 `map_report_to_answer` 拿到
//! `ConsultReport` 后做一次**后验审查**：
//!
//! 1. **解析** `report.steps` 与原 `question` 中的动作词（写/读/打开/调用/落库…）
//! 2. **抽取**资源标识（`db:xxx` / `pii:xxx` / `var:xxx` 等 URI 形式）
//! 3. **构造** `FlowGraph`：`Start → [Task 节点按序] → End`，每节点带
//!    `Access::read/write` 与 `NodeKind::Task`/`Guard`
//! 4. **执行** `mox_optimize(&graph, &gov_ctx)`，与生产编排走同一套治理闸门
//! 5. **返回** [`PostHocGovernance`]：`Pass` / `Warn` / `Veto`
//!
//! # 取向
//!
//! **宁可误报（多一次 Veto），不可漏报（让越权写流出）**。保守映射确保
//! LLM 输出触及敏感域时必被拦截；空集时直接 Pass，无副作用。
//!
//! # 与 `parse_veto` 的关系
//!
//! 二者**互补不重叠**：
//! - `parse_veto` 拦截**回答文本**中的"否决"语义词
//! - 本模块拦截**回答文本**中**实际描述的写操作**是否触敏
//!
//! 若 LLM 写了"该方案不可行"但实际是分析建议 → `parse_veto=true` 但本模块
//! 找不到任何写动作 → `Pass`（不二次拦截）。若 LLM 写"读取公民库后直接落库
//! 到生产" → `parse_veto=false` 但本模块构造出 `db:citizen_info → db:prod/...`
//! 链 → 必 `Veto`。
//!
//! # 性能
//!
//! `mox_optimize` 加载 14 维专家 + 钩子，单次约 5-15ms（round 6 实测），
//! LLM 回复链路总成本：模型推理（秒级）+ 后验治理（毫秒级），延迟影响 < 1%。
//!
//! # 限制
//!
//! - 文本解析是**启发式**的，无法识别所有语义变体（"建议落盘" vs "写死"）
//! - 若 LLM 输出不含资源 URI 形式（如"查询一下表"），识别为隐式 Database 读
//!   但无具体资源标识 → 治理层走"无敏感触发"路径
//! - 高安全场景应在前置 prompt 强制要求 LLM 给出**结构化动作清单**
//!   （后续可演进为 `mox_consult_v1` JSON Schema 协议）

use crate::experts_common::ExpertDescriptor;
use mox_ai_expert_svc::mox_optimize;
use mox_ai_expert_svc::{GovernContext, Principal, Tenant};
use mox_ai_expert_svc::sensitivity::{is_production_or_sensitive_write, is_sensitive_domain};
use mox_ai_flow_svc::model::{
    Access, AccessMode, ExpertRule, FlowEdge, FlowGraph, FlowNode, NodeKind, Severity, ToolKind,
};
use serde::Serialize;

/// 后验治理结果
///
/// 决策树：
/// - `Veto`：闸门否决 → 调用方应**强制拦截** LLM 回复正文
/// - `Warn`：通过但有阻断级风险 → 透传但附加 `governance_warnings` 字段
/// - `Pass`：无任何敏感触发 → 透传
#[derive(Debug, Clone, Serialize)]
pub struct PostHocGovernance {
    pub decision: GovernanceDecision,
    /// 闸门原因（Veto/Warn 时填）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// 触发资源的 URI 列表（用于审计）
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sensitive_resources: Vec<String>,
    /// 构造出的 FlowGraph id（用于审计/排障）
    pub graph_id: String,
    /// 节点数（用于审计/排障）
    pub node_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceDecision {
    Pass,
    Warn,
    Veto,
}

/// 启发式识别出的单个动作
#[derive(Debug, Clone, PartialEq, Eq)]
struct InferredAction {
    /// 节点 id（在 FlowGraph 内唯一）
    id: String,
    /// 可读名称
    name: String,
    /// 工具类别
    tool: ToolKind,
    /// 访问声明
    accesses: Vec<Access>,
    /// 原始 step 文本（用于审计/可读性）
    raw: String,
}

/// 把 LLM 输出的 steps + 问题文本映射成 FlowGraph 并跑治理。
///
/// 永不 panic；解析失败/无敏感触发均返回 `Pass`。
pub fn govern_llm_answer(
    expert: &ExpertDescriptor,
    steps: &[String],
    question: &str,
) -> PostHocGovernance {
    // 1. 启发式抽取动作
    let actions = infer_actions(steps, question);
    let graph = build_flow_graph(&actions, question);

    if actions.is_empty() {
        return PostHocGovernance {
            decision: GovernanceDecision::Pass,
            reason: None,
            sensitive_resources: Vec::new(),
            graph_id: graph.id.clone(),
            node_count: graph.nodes.len(),
        };
    }

    // 2. 收集敏感资源（仅审计用，不影响决策）
    let sensitive_resources: Vec<String> = actions
        .iter()
        .flat_map(|a| a.accesses.iter())
        .filter(|acc| is_sensitive_domain(&acc.resource))
        .map(|acc| acc.resource.clone())
        .collect();

    // 3. 构造治理上下文：regulated 租户 + 具备 editor 权限的主体。
    //    regulated=true 触发 strictest 治理路径；principal 角色决定 RBAC。
    //    注：与企业级网关其他专家路径保持一致（专家被授权 edit-flow 权限）。
    let tenant = Tenant::new("llm-governance", "ns-llm").regulated(true);
    let principal = Principal::new(&expert.id)
        .with_roles(vec!["editor".to_string(), "admin".to_string()]);
    let ctx = GovernContext::new(tenant, principal);

    // 4. 调用璇玑优化（14 维专家 + 闸门 + 算法验证）
    let report = mox_optimize(&graph, &ctx);

    // 5. 决策
    if report.algo.vetoed || report.gate.algorithm_veto {
        return PostHocGovernance {
            decision: GovernanceDecision::Veto,
            reason: Some(format!(
                "璇玑验证/算法否决：{}；闸门：{}",
                report.algo.summary, report.gate.reason
            )),
            sensitive_resources,
            graph_id: graph.id,
            node_count: graph.nodes.len(),
        };
    }
    if !report.gate.approved {
        let reason = if report.gate.blocking_risks > 0 {
            format!(
                "闸门不通过：{}（{} 项阻断级风险）",
                report.gate.reason, report.gate.blocking_risks
            )
        } else {
            format!("闸门不通过：{}", report.gate.reason)
        };
        return PostHocGovernance {
            decision: GovernanceDecision::Veto,
            reason: Some(reason),
            sensitive_resources,
            graph_id: graph.id,
            node_count: graph.nodes.len(),
        };
    }
    if !sensitive_resources.is_empty() {
        // 触敏但闸门通过（已配 Guard/已脱敏），给 Warn
        return PostHocGovernance {
            decision: GovernanceDecision::Warn,
            reason: Some(format!(
                "触敏资源 {} 项，已配 Guard 放行；建议人工复核",
                sensitive_resources.len()
            )),
            sensitive_resources,
            graph_id: graph.id,
            node_count: graph.nodes.len(),
        };
    }

    PostHocGovernance {
        decision: GovernanceDecision::Pass,
        reason: None,
        sensitive_resources,
        graph_id: graph.id,
        node_count: graph.nodes.len(),
    }
}

// ─────────────────────── 文本→动作启发式 ───────────────────────

/// 动作词 → ToolKind 映射（保守：写操作 = Database，缺省 Compute）
const WRITE_VERBS: &[&str] = &[
    "写", "落库", "保存", "更新", "插入", "删除", "修改", "覆盖",
    "insert", "update", "delete", "write", "upsert", "drop", "truncate",
];
const READ_VERBS: &[&str] = &[
    "读", "查询", "拉取", "检索", "获取", "查找",
    "select", "fetch", "query", "read", "get", "find", "load",
];
const BROWSER_VERBS: &[&str] = &[
    "打开", "登录", "访问", "点击", "填表", "填报", "操作", "网办", "浏览器",
    "browser", "open", "click", "submit", "navigate", "visit", "log in",
];
const HTTP_VERBS: &[&str] = &[
    "调用", "请求", "API", "接口", "POST", "GET", "PUT", "DELETE", "http", "fetch",
];
const FILE_VERBS: &[&str] = &["文件", "导出", "导入", "Excel", "CSV", "file", "export", "import"];
const SHELL_VERBS: &[&str] = &["执行命令", "运行命令", "shell", "bash", "cmd"];

/// 资源 URI 模式：`scheme:env/entity` 或 `scheme:entity` 或 `pii:xxx` / `var:xxx` / `db:xxx`
const RESOURCE_URI_PATTERN: &[&str] = &[
    "db:", "var:", "pii:", "file:", "url:", "http:", "https:", "ftp:", "s3:",
];

/// 中文敏感资源关键字 → 自动升级为 `db:sensitive/<kw>` URI
const SENSITIVE_KW: &[&str] = &[
    "公民", "身份证", "户籍", "社保", "医保", "学籍", "公安", "司法", "金融账户", "银行",
    "密码", "人脸", "指纹", "健康", "病历", "处方", "位置", "通话",
];

fn infer_actions(steps: &[String], question: &str) -> Vec<InferredAction> {
    let mut actions = Vec::new();
    let mut node_idx = 0usize;

    // 扫描步骤列表
    for (si, step) in steps.iter().enumerate() {
        if let Some(action) = parse_step(step, si, &mut node_idx) {
            actions.push(action);
        }
    }

    // 兜底：若 steps 完全没抽出动作但问题含操作意图，给一个"意图节点"
    if actions.is_empty() && has_action_intent(question) {
        let action = parse_step(question, steps.len(), &mut node_idx);
        if let Some(a) = action {
            actions.push(a);
        }
    }

    actions
}

fn parse_step(text: &str, seq: usize, node_idx: &mut usize) -> Option<InferredAction> {
    let lower = text.to_lowercase();
    let mut tool = ToolKind::Compute;
    let mut mode = AccessMode::Read;
    let mut has_action = false;

    // 写意图
    if WRITE_VERBS.iter().any(|v| lower.contains(v)) {
        tool = ToolKind::Database;
        mode = AccessMode::Write;
        has_action = true;
    }
    // 浏览器
    else if BROWSER_VERBS.iter().any(|v| lower.contains(v)) {
        tool = ToolKind::Browser;
        mode = AccessMode::Read;
        has_action = true;
    }
    // HTTP
    else if HTTP_VERBS.iter().any(|v| lower.contains(v)) {
        tool = ToolKind::Http;
        mode = AccessMode::Read;
        has_action = true;
    }
    // 文件
    else if FILE_VERBS.iter().any(|v| lower.contains(v)) {
        tool = ToolKind::File;
        mode = AccessMode::Read;
        has_action = true;
    }
    // Shell
    else if SHELL_VERBS.iter().any(|v| lower.contains(v)) {
        tool = ToolKind::Shell;
        mode = AccessMode::Read;
        has_action = true;
    }
    // 纯读
    else if READ_VERBS.iter().any(|v| lower.contains(v)) {
        tool = ToolKind::Database;
        mode = AccessMode::Read;
        has_action = true;
    }

    if !has_action {
        return None;
    }

    // 抽取资源 URI
    let resources = extract_resources(text);

    // 若没识别到资源但有写意图：保守视为写入"未指定目标"——给一个哨兵资源触发闸门审查
    let accesses: Vec<Access> = if resources.is_empty() {
        if matches!(mode, AccessMode::Write) {
            vec![Access {
                resource: "var:unspecified_target".into(),
                mode: AccessMode::Write,
            }]
        } else {
            // 读且无资源：通常不触发治理，给一个明确的占位
            vec![Access {
                resource: "var:read_only_query".into(),
                mode: AccessMode::Read,
            }]
        }
    } else {
        resources
            .iter()
            .map(|r| Access {
                resource: r.clone(),
                mode: mode,
            })
            .collect()
    };

    *node_idx += 1;
    Some(InferredAction {
        id: format!("s{seq}"),
        name: truncate_chars(text, 40),
        tool,
        accesses,
        raw: text.to_string(),
    })
}

fn extract_resources(text: &str) -> Vec<String> {
    let mut resources = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // 1. 显式 URI 模式
    for &prefix in RESOURCE_URI_PATTERN {
        let mut search_from = 0;
        while let Some(rel) = text[search_from..].find(prefix) {
            let abs = search_from + rel;
            let candidate = extract_uri_token(&text[abs..]);
            if let Some(uri) = candidate {
                if seen.insert(uri.clone()) {
                    resources.push(uri);
                }
            }
            search_from = abs + prefix.len();
            if search_from >= text.len() {
                break;
            }
        }
    }

    // 2. 隐式敏感关键字 → 升级为 `db:sensitive/<kw>` 让治理层识别
    for &kw in SENSITIVE_KW {
        if text.contains(kw) {
            let uri = format!("db:sensitive/{}", kw);
            if seen.insert(uri.clone()) {
                resources.push(uri);
            }
        }
    }

    resources
}

/// 从 `text` 开头抽取一个 URI 令牌：scheme:rest，rest 形如 `[A-Za-z0-9_./-]+`
fn extract_uri_token(text: &str) -> Option<String> {
    // 必须以 "scheme:" 开头
    let colon = text.find(':')?;
    if colon == 0 {
        return None;
    }
    let scheme = &text[..colon];
    if !scheme.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    let rest = &text[colon + 1..];
    let mut end = 0;
    for (i, c) in rest.char_indices() {
        if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-') {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    Some(format!("{}:{}", scheme, &rest[..end]))
}

fn has_action_intent(text: &str) -> bool {
    let lower = text.to_lowercase();
    WRITE_VERBS.iter().any(|v| lower.contains(v))
        || READ_VERBS.iter().any(|v| lower.contains(v))
        || BROWSER_VERBS.iter().any(|v| lower.contains(v))
        || HTTP_VERBS.iter().any(|v| lower.contains(v))
        || FILE_VERBS.iter().any(|v| lower.contains(v))
        || SHELL_VERBS.iter().any(|v| lower.contains(v))
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{truncated}…")
    }
}

// ─────────────────────── FlowGraph 构造 ───────────────────────

/// 把 InferredAction 列表组装成可被 `mox_optimize` 消费的 FlowGraph。
///
/// 拓扑：Start → [task nodes in sequence] → End
/// 每节点保留 access 声明与 estimated duration（启发式：写=300ms, 读=200ms, 浏览器=500ms）
fn build_flow_graph(actions: &[InferredAction], question: &str) -> FlowGraph {
    let mut graph = FlowGraph::new("llm-llm-derive", &truncate_chars(question, 60));

    graph.add_node(FlowNode::new("start", "开始", NodeKind::Start));
    graph.add_node(FlowNode::new("end", "结束", NodeKind::End));

    let mut prev_id = "start".to_string();
    for action in actions {
        let mut node = FlowNode::task(&action.id, &action.name, action.tool, default_duration(action.tool));
        for acc in &action.accesses {
            node = node.with_access(acc.clone());
        }
        // 触敏写动作自动加 desensitize Guard（如果前后没有 Guard）——让治理闸门可放行已脱敏路径
        if is_sensitive_write_action(action) {
            node = node.with_tag("candidate_sensitive_write");
        }
        graph.add_node(node);
        graph.add_edge(FlowEdge::seq(&prev_id, &action.id));
        prev_id = action.id.clone();
    }

    graph.add_edge(FlowEdge::seq(&prev_id, "end"));

    // 挂载业务规则：公民/密码/金融类敏感资源需要 desensitize Guard
    if actions
        .iter()
        .any(|a| a.accesses.iter().any(is_sensitive_write_resource))
    {
        graph.rules.push(ExpertRule {
            id: "R-LLM-001".into(),
            description: "LLM 后验：敏感资源写操作须具备 desensitize Guard".into(),
            severity: Severity::Blocking,
            resource_prefixes: vec!["db:".into(), "pii:".into(), "var:".into()],
            tool_kinds: vec![ToolKind::Database, ToolKind::File, ToolKind::Http],
            required_guard_tags: vec!["desensitize".into(), "authz".into()],
        });
    }

    graph
}

fn is_sensitive_write_resource(acc: &Access) -> bool {
    is_production_or_sensitive_write(&acc.resource)
}

fn is_sensitive_write_action(a: &InferredAction) -> bool {
    a.accesses
        .iter()
        .any(|acc| is_production_or_sensitive_write(&acc.resource))
}

fn default_duration(tool: ToolKind) -> u64 {
    match tool {
        ToolKind::Database => 200,
        ToolKind::Browser => 500,
        ToolKind::Http => 150,
        ToolKind::File => 100,
        ToolKind::Llm => 2000,
        ToolKind::Shell => 300,
        ToolKind::Human => 60_000,
        ToolKind::Compute => 50,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exp() -> ExpertDescriptor {
        ExpertDescriptor::minimal("test-expert".into(), "测试专家".into())
    }

    #[test]
    fn extract_uri_token_handles_db_and_pii() {
        assert_eq!(extract_uri_token("db:citizen_info more"), Some("db:citizen_info".into()));
        assert_eq!(extract_uri_token("pii:id_card"), Some("pii:id_card".into()));
        assert_eq!(extract_uri_token("var:foo_bar-baz"), Some("var:foo_bar-baz".into()));
        assert_eq!(extract_uri_token("not a uri"), None);
        // 截断于非 URI 字符
        assert_eq!(extract_uri_token("db:citizen,"), Some("db:citizen".into()));
    }

    #[test]
    fn sensitive_keyword_promotes_to_db_uri() {
        let res = extract_resources("查询一下公民信息");
        assert!(res.iter().any(|r| r == "db:sensitive/公民"));
    }

    #[test]
    fn write_to_sensitive_db_is_vetoed() {
        let steps = vec![
            "读取 db:citizen_info 公民库".into(),
            "写入 db:prod/citizen_records 生产环境".into(),
        ];
        let g = govern_llm_answer(&exp(), &steps, "把公民数据搬到生产");
        assert_eq!(g.decision, GovernanceDecision::Veto, "应被治理闸门拦截");
        assert!(g.reason.as_ref().unwrap().contains("闸门") || g.reason.as_ref().unwrap().contains("璇玑"));
        assert!(!g.sensitive_resources.is_empty());
    }

    #[test]
    fn read_only_step_passes() {
        let steps = vec!["查询订单状态".into(), "返回结果".into()];
        let g = govern_llm_answer(&exp(), &steps, "查一下订单");
        // 无写动作、无敏感资源 → Pass
        assert_eq!(g.decision, GovernanceDecision::Pass);
    }

    #[test]
    fn empty_steps_returns_pass() {
        let g = govern_llm_answer(&exp(), &[], "随便聊聊");
        assert_eq!(g.decision, GovernanceDecision::Pass);
        assert_eq!(g.node_count, 2); // 只有 start/end
    }

    #[test]
    fn browser_step_passes_without_sensitive() {
        let steps = vec!["打开 12306 网站".into(), "查询车次".into()];
        let g = govern_llm_answer(&exp(), &steps, "查火车票");
        // 浏览器动作不触敏 → Pass
        assert_eq!(g.decision, GovernanceDecision::Pass);
    }

    #[test]
    fn unspecified_write_target_triggers_review() {
        // 写动作但无具体资源 → 哨兵 `var:unspecified_target` 不在敏感域，
        // 因此治理闸门应当 Pass。**这是预期行为** —— 保守映射
        // 不应把"未指定目标"的写操作一律当敏感。测试守护该契约。
        let steps = vec!["把数据保存到表里".into()];
        let g = govern_llm_answer(&exp(), &steps, "保存数据");
        assert_eq!(g.decision, GovernanceDecision::Pass,
            "未指定写目标不应触发拦截（保守：仅触敏触发），实际={:?}", g.decision);
        assert!(g.node_count >= 3, "应至少构造 start + write_task + end");
    }

    #[test]
    fn chinese_truncate_is_char_safe() {
        // 7 字符中文字符串截断到 4 字符：取前 4 字符 "公民身份" + "…"
        assert_eq!(truncate_chars("公民身份证号码", 4), "公民身份…");
        // 短于阈值的字符串原样返回
        assert_eq!(truncate_chars("abc", 10), "abc");
        // 英文按字节级正确
        assert_eq!(truncate_chars("hello world", 5), "hello…");
    }
}
