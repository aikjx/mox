//! 企业级上下文：租户 / 主体 / 策略 / 配额 / 兼容性注册表

use crate::ir::{CodeIR, Dimension, PolicyId};
use flow_ai::model::{FlowGraph, ResourcePool};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 租户：多租户隔离的根
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub namespace: String,
    /// 资源池上限（由配额翻译而来）
    pub pool_caps: HashMap<String, u32>,
    /// 是否政务/金融等强合规租户（影响权限/安全专家严格度）
    pub regulated: bool,
}

impl Tenant {
    pub fn new(id: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            namespace: namespace.into(),
            pool_caps: HashMap::new(),
            regulated: false,
        }
    }
    pub fn with_pool(mut self, pool: impl Into<String>, cap: u32) -> Self {
        self.pool_caps.insert(pool.into(), cap);
        self
    }
    pub fn regulated(mut self, v: bool) -> Self {
        self.regulated = v;
        self
    }
}

/// 主体（谁在调用）：RBAC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principal {
    pub subject: String,
    pub roles: Vec<String>,
}

impl Principal {
    pub fn new(subject: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            roles: vec!["viewer".into()],
        }
    }
    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }
}

/// 角色权限
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    ViewAudit,
    RunFlow,
    EditFlow,
    ApproveFlow,
}

/// 策略：轻量谓词式规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: PolicyId,
    pub dimension: Dimension,
    /// 人类可读描述
    pub description: String,
    /// 强合规策略违反即 Blocking
    pub blocking: bool,
}

/// 资源配额
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub max_parallel: u32,
    pub max_cost_budget: f64,
    /// 单流程 SLA 上限（毫秒）
    pub sla_ms: u64,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            max_parallel: 8,
            max_cost_budget: 1.0,
            sla_ms: 5_000,
        }
    }
}

// ===================== 兼容性注册表 =====================

/// MCP 工具描述（兼容 Model Context Protocol）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub server: String,
    pub name: String,
    /// JSON-Schema 字符串（入参）
    pub input_schema: String,
    /// 该工具对应的目标池（用于资源/冲突分析）
    pub pool: String,
}

/// Skill 引用（兼容 Skills 体系，与 flow-ai topology 关系网打通）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRef {
    pub id: String,
    pub keywords: Vec<String>,
    /// 可选：Skill 自带的流程模板，命中后可跳过完整推理
    pub flow_template: Option<FlowGraph>,
}

/// 循环/自省策略（兼容 Loops）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopPolicy {
    /// 有界循环
    Bounded { max_iter: u32 },
    /// 人在环
    HumanInLoop,
    /// 无界（需安全专家严格审批）
    Unbounded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopGuard {
    pub node: String,
    pub policy: LoopPolicy,
}

/// 外部能力注册表：把 MCP/Skills/Loops/LLM 归一化接入同一张图
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompatibilityRegistry {
    pub mcp_servers: HashMap<String, Vec<McpTool>>,
    pub skills: Vec<SkillRef>,
    pub loops: Vec<LoopGuard>,
}

impl CompatibilityRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register_mcp(&mut self, server: impl Into<String>, tools: Vec<McpTool>) {
        self.mcp_servers.insert(server.into(), tools);
    }
    pub fn register_skill(&mut self, s: SkillRef) {
        self.skills.push(s);
    }
    pub fn register_loop(&mut self, g: LoopGuard) {
        self.loops.push(g);
    }
    /// 把所有 MCP 工具对应的资源池并入租户配额（取 min 保证不超配）
    pub fn apply_to_pools(&self, pools: &mut Vec<ResourcePool>) {
        for tools in self.mcp_servers.values() {
            for t in tools {
                if !pools.iter().any(|p| p.name == t.pool) {
                    pools.push(ResourcePool {
                        name: t.pool.clone(),
                        capacity: 1,
                    });
                }
            }
        }
    }
}

/// 全量治理上下文（喂给流水线）
#[derive(Clone, Serialize, Deserialize)]
pub struct GovernContext {
    pub tenant: Tenant,
    pub principal: Principal,
    pub policies: Vec<Policy>,
    pub quota: ResourceQuota,
    pub registry: CompatibilityRegistry,
    /// 开发璇玑的分析对象（代码的 IR）；无代码时开发专家自动 skipped
    pub code_ir: Option<CodeIR>,
    /// 期望的大模型路由（LLM 兼容性）
    pub llm_tier: Option<flow_ai::schedule::ModelTier>,
    /// 外部审计上下文（内部哈希链 + 外部 sink 双写）。
    /// 序列化时跳过：运行时由 `with_audit` 注入，默认 NoopSink（开发/单测环境）。
    #[serde(skip, default)]
    pub audit: Option<std::sync::Arc<crate::audit::AuditContext>>,
}

impl std::fmt::Debug for GovernContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // audit 字段含 `Arc<dyn AuditSink>`（不可 Debug），以占位文本呈现
        f.debug_struct("GovernContext")
            .field("tenant", &self.tenant)
            .field("principal", &self.principal)
            .field("policies", &self.policies)
            .field("quota", &self.quota)
            .field("registry", &self.registry)
            .field("code_ir", &self.code_ir)
            .field("llm_tier", &self.llm_tier)
            .field("audit", &"<AuditContext>")
            .finish()
    }
}

impl GovernContext {
    pub fn new(tenant: Tenant, principal: Principal) -> Self {
        Self {
            tenant,
            principal,
            policies: Vec::new(),
            quota: ResourceQuota::default(),
            registry: CompatibilityRegistry::new(),
            code_ir: None,
            llm_tier: None,
            audit: None,
        }
    }

    /// 注入外部审计上下文（Syslog/S3/自定义 sink，经 `MultiSink` 组合）。
    /// 未注入时 RBAC 越权等审计事件写入内部哈希链（`AuditChain`），
    /// 注入后双写：内部链自验 + 外部持久化（SOC2/GDPR 合规证据）。
    pub fn with_audit(mut self, audit: std::sync::Arc<crate::audit::AuditContext>) -> Self {
        self.audit = Some(audit);
        self
    }
}

/// 专家只读上下文（并行派发用）
pub struct ExpertContext<'a> {
    pub flow: &'a flow_ai::model::FlowGraph,
    pub tenant: &'a Tenant,
    pub principal: &'a Principal,
    pub policies: &'a [Policy],
    pub quota: &'a ResourceQuota,
    pub registry: &'a CompatibilityRegistry,
    /// 开发璇玑的分析对象（借用于 GovernContext，避免克隆）
    pub code_ir: &'a Option<CodeIR>,
    /// 外部审计上下文（借用于 GovernContext；`can()` 越权判定时双写审计）
    pub audit: Option<&'a std::sync::Arc<crate::audit::AuditContext>>,
}

impl<'a> ExpertContext<'a> {
    pub fn new(flow: &'a flow_ai::model::FlowGraph, gctx: &'a GovernContext) -> Self {
        Self {
            flow,
            tenant: &gctx.tenant,
            principal: &gctx.principal,
            policies: &gctx.policies,
            quota: &gctx.quota,
            registry: &gctx.registry,
            code_ir: &gctx.code_ir,
            audit: gctx.audit.as_ref(),
        }
    }

    pub fn policies_of(&self, dim: Dimension) -> Vec<&Policy> {
        self.policies
            .iter()
            .filter(|p| p.dimension == dim)
            .collect()
    }

    /// 鉴权单入口：所有能力判定统一委托 RBAC 引擎（`rbac::check_with_audit`）。
    ///
    /// 此前 `can()` 是硬编码的角色字符串匹配（`EditFlow => "editor"`），
    /// 与 `rbac/policy.rs` 完全脱节——任何对 RBAC 策略的修改都不会反映到专家鉴权。
    /// 现在 `Capability` 被映射为 `(action, resource)` 资源级权限，统一走 `rbac::check`，
    /// 让内置角色矩阵、继承链、通配符与跨租户隔离对专家鉴权真正生效。
    /// 越权（denied）时若配置了外部审计上下文，则同步写入 RBACDenied 审计事件。
    pub fn can(&self, cap: Capability) -> bool {
        let (action, resource) = match cap {
            // 审计查看：resource 级 read:audit:*
            Capability::ViewAudit => ("read", "audit:*"),
            // 运行流程：execute:flow:*
            Capability::RunFlow => ("execute", "flow:*"),
            // 编辑流程（所有专家的前置门禁）：write:flow:*（editor/operator 等角色已覆盖）
            Capability::EditFlow => ("write", "flow:*"),
            // 审批流程：admin:flow:gov-pii/*（safety_approver 等角色已覆盖）
            Capability::ApproveFlow => ("admin", "flow:gov-pii/*"),
        };
        let check_ctx = crate::rbac::PermissionCheck::new(
            &self.principal.subject,
            self.principal.roles.clone(),
            action,
            crate::rbac::check::Resource::with_tenant(resource, &self.tenant.id),
        );
        crate::rbac::check_with_audit(&check_ctx, self.audit).is_granted()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rbac_admin_passes_all() {
        let p = Principal::new("u").with_roles(vec!["admin".into()]);
        let g = GovernContext::new(Tenant::new("t", "ns"), p);
        let fg = FlowGraph::new("x", "t");
        let c = ExpertContext::new(&fg, &g);
        assert!(c.can(Capability::ApproveFlow));
        assert!(c.can(Capability::RunFlow));
    }

    #[test]
    fn rbac_viewer_denied_run() {
        let p = Principal::new("u");
        let g = GovernContext::new(Tenant::new("t", "ns"), p);
        let fg = FlowGraph::new("x", "t");
        let c = ExpertContext::new(&fg, &g);
        assert!(!c.can(Capability::RunFlow));
    }

    #[test]
    fn rbac_editor_can_edit_flow() {
        // editor 继承 viewer 并拥有 write:flow:*，应能通过 EditFlow 门禁
        let p = Principal::new("u").with_roles(vec!["editor".into()]);
        let g = GovernContext::new(Tenant::new("t", "ns"), p);
        let fg = FlowGraph::new("x", "t");
        let c = ExpertContext::new(&fg, &g);
        assert!(c.can(Capability::EditFlow));
        // editor 无 admin:flow:gov-pii/*，不能通过审批
        assert!(!c.can(Capability::ApproveFlow));
    }

    #[test]
    fn rbac_viewer_cannot_edit_flow() {
        // viewer 仅 read，无 write:flow:*，专家分析前置门禁应拒绝
        let p = Principal::new("u");
        let g = GovernContext::new(Tenant::new("t", "ns"), p);
        let fg = FlowGraph::new("x", "t");
        let c = ExpertContext::new(&fg, &g);
        assert!(!c.can(Capability::EditFlow));
    }

    #[test]
    fn mcp_tools_register_pools() {
        let mut reg = CompatibilityRegistry::new();
        reg.register_mcp(
            "fs",
            vec![McpTool {
                server: "fs".into(),
                name: "read".into(),
                input_schema: "{}".into(),
                pool: "mcp_fs".into(),
            }],
        );
        let mut pools = vec![ResourcePool {
            name: "cpu".into(),
            capacity: 8,
        }];
        reg.apply_to_pools(&mut pools);
        assert!(pools.iter().any(|p| p.name == "mcp_fs"));
    }
}
