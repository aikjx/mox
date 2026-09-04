// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 企业级上下文：租户 / 主体 / 策略 / 配额 / 兼容性注册表
//!
//! P2 架构解耦 · 阶段 4：
//! - `GovernContext` 结构体实现 `mox_ai_expert_proto::domain::GovernContext` trait（DIP）
//! - 审计上下文从内部 `AuditChain` 升级为 `mox-audit` 的 `AuditContext`（SHA-256）
//! - `can()` 方法使用简化角色映射（完整 RBAC 引擎待后续阶段迁移）

use crate::ir::CodeIR;
use mox_ai_expert_proto::{Dimension, PolicyId};
use mox_ai_flow_core::model::{FlowGraph, ResourcePool};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

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
    pub llm_tier: Option<mox_ai_expert_proto::ModelTier>,
    /// 外部审计上下文（mox-audit 统一审计，SHA-256 哈希链 + 多 Sink）。
    /// 序列化时跳过：运行时由 `with_audit` 注入，默认 None（开发/单测环境）。
    #[serde(skip, default)]
    pub audit: Option<Arc<mox_audit::AuditContext>>,
}

impl std::fmt::Debug for GovernContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // audit 字段含 `Arc<AuditContext>`（不可 Debug），以占位文本呈现
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

    /// 注入 mox-audit 审计上下文（SHA-256 哈希链 + Syslog/S3 等 Sink）。
    /// 未注入时审计事件写入内部 noop，注入后双写：链自验 + 外部持久化（SOC2/GDPR 合规）。
    pub fn with_audit(mut self, audit: Arc<mox_audit::AuditContext>) -> Self {
        self.audit = Some(audit);
        self
    }
}

/// 实现 proto 的 GovernContext trait（DIP：下游通过 trait 访问，不依赖具体结构体）
impl mox_ai_expert_proto::domain::GovernContext for GovernContext {
    fn tenant(&self) -> &str {
        &self.tenant.id
    }
    fn namespace(&self) -> &str {
        &self.tenant.namespace
    }
    fn principal(&self) -> &str {
        &self.principal.subject
    }
    fn roles(&self) -> &[String] {
        &self.principal.roles
    }
    fn is_regulated(&self) -> bool {
        self.tenant.regulated
    }
    fn max_parallel(&self) -> u32 {
        self.quota.max_parallel
    }
    fn cost_budget(&self) -> f64 {
        self.quota.max_cost_budget
    }
    fn sla_ms(&self) -> u64 {
        self.quota.sla_ms
    }
}

/// 专家只读上下文（并行派发用）
pub struct ExpertContext<'a> {
    pub flow: &'a mox_ai_flow_core::model::FlowGraph,
    pub tenant: &'a Tenant,
    pub principal: &'a Principal,
    pub policies: &'a [Policy],
    pub quota: &'a ResourceQuota,
    pub registry: &'a CompatibilityRegistry,
    /// 开发璇玑的分析对象（借用于 GovernContext，避免克隆）
    pub code_ir: &'a Option<CodeIR>,
    /// 外部审计上下文（借用于 GovernContext；`can()` 越权判定时写审计）
    pub audit: Option<&'a Arc<mox_audit::AuditContext>>,
}

impl<'a> ExpertContext<'a> {
    pub fn new(flow: &'a mox_ai_flow_core::model::FlowGraph, gctx: &'a GovernContext) -> Self {
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

    /// 鉴权单入口：角色 → 能力映射（简化版，完整 RBAC 引擎待后续迁移）。
    ///
    /// 角色继承关系：admin > editor > operator > viewer
    /// - admin: 全部权限
    /// - editor: 继承 viewer + write:flow:*
    /// - operator: 继承 viewer + execute:flow:*
    /// - viewer: read:audit:*
    /// - safety_approver: 额外拥有 admin:flow:gov-pii/*（审批权限）
    pub fn can(&self, cap: Capability) -> bool {
        let roles = &self.principal.roles;
        let has = |r: &str| roles.iter().any(|x| x == r);

        // admin 拥有所有权限
        if has("admin") {
            return true;
        }
        // safety_approver 拥有审批权限
        if has("safety_approver") && matches!(cap, Capability::ApproveFlow) {
            return true;
        }

        match cap {
            Capability::ViewAudit => {
                // viewer 及以上都能查看审计
                has("viewer") || has("editor") || has("operator") || has("admin")
            }
            Capability::RunFlow => {
                // operator / editor / admin 可运行流程
                has("operator") || has("editor") || has("admin")
            }
            Capability::EditFlow => {
                // editor / admin 可编辑流程
                has("editor") || has("admin")
            }
            Capability::ApproveFlow => {
                // admin 或 safety_approver 可审批（上面已判 admin，此处补 safety_approver）
                has("safety_approver")
            }
        }
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
        assert!(c.can(Capability::EditFlow));
        assert!(c.can(Capability::ViewAudit));
    }

    #[test]
    fn rbac_viewer_denied_run() {
        let p = Principal::new("u");
        let g = GovernContext::new(Tenant::new("t", "ns"), p);
        let fg = FlowGraph::new("x", "t");
        let c = ExpertContext::new(&fg, &g);
        assert!(!c.can(Capability::RunFlow));
        assert!(!c.can(Capability::EditFlow));
        assert!(!c.can(Capability::ApproveFlow));
        assert!(c.can(Capability::ViewAudit));
    }

    #[test]
    fn rbac_editor_can_edit_flow() {
        // editor 继承 viewer 并拥有 write:flow:*，应能通过 EditFlow 门禁
        let p = Principal::new("u").with_roles(vec!["editor".into()]);
        let g = GovernContext::new(Tenant::new("t", "ns"), p);
        let fg = FlowGraph::new("x", "t");
        let c = ExpertContext::new(&fg, &g);
        assert!(c.can(Capability::EditFlow));
        assert!(c.can(Capability::RunFlow));
        assert!(c.can(Capability::ViewAudit));
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
    fn safety_approver_can_approve() {
        let p = Principal::new("u").with_roles(vec!["safety_approver".into()]);
        let g = GovernContext::new(Tenant::new("t", "ns"), p);
        let fg = FlowGraph::new("x", "t");
        let c = ExpertContext::new(&fg, &g);
        assert!(c.can(Capability::ApproveFlow));
        // safety_approver 不能编辑（只有审批权）
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

    #[test]
    fn govern_context_impl_proto_trait() {
        // DIP 验证：core 的 GovernContext 实现了 proto 的 GovernContext trait
        let p = Principal::new("alice").with_roles(vec!["admin".into()]);
        let t = Tenant::new("tenant-1", "ns-gov").regulated(true);
        let g = GovernContext::new(t, p);
        let proto_ctx: &dyn mox_ai_expert_proto::domain::GovernContext = &g;
        assert_eq!(proto_ctx.tenant(), "tenant-1");
        assert_eq!(proto_ctx.namespace(), "ns-gov");
        assert_eq!(proto_ctx.principal(), "alice");
        assert!(proto_ctx.is_regulated());
        assert_eq!(proto_ctx.max_parallel(), 8);
    }
}
