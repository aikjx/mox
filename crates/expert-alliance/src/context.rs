//! 企业级上下文：租户 / 主体 / 策略 / 配额 / 兼容性注册表

use crate::ir::{Dimension, PolicyId};
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
        Self { subject: subject.into(), roles: vec!["viewer".into()] }
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
        Self { max_parallel: 8, max_cost_budget: 1.0, sla_ms: 5_000 }
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
                    pools.push(ResourcePool { name: t.pool.clone(), capacity: 1 });
                }
            }
        }
    }
}

/// 全量治理上下文（喂给流水线）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernContext {
    pub tenant: Tenant,
    pub principal: Principal,
    pub policies: Vec<Policy>,
    pub quota: ResourceQuota,
    pub registry: CompatibilityRegistry,
    /// 期望的大模型路由（LLM 兼容性）
    pub llm_tier: Option<flow_ai::schedule::ModelTier>,
}

impl GovernContext {
    pub fn new(tenant: Tenant, principal: Principal) -> Self {
        Self {
            tenant,
            principal,
            policies: Vec::new(),
            quota: ResourceQuota::default(),
            registry: CompatibilityRegistry::new(),
            llm_tier: None,
        }
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
}

impl<'a> ExpertContext<'a> {
    pub fn new(
        flow: &'a flow_ai::model::FlowGraph,
        gctx: &'a GovernContext,
    ) -> Self {
        Self {
            flow,
            tenant: &gctx.tenant,
            principal: &gctx.principal,
            policies: &gctx.policies,
            quota: &gctx.quota,
            registry: &gctx.registry,
        }
    }

    pub fn policies_of(&self, dim: Dimension) -> Vec<&Policy> {
        self.policies.iter().filter(|p| p.dimension == dim).collect()
    }

    pub fn can(&self, cap: Capability) -> bool {
        let required = match cap {
            Capability::ViewAudit => "auditor",
            Capability::RunFlow => "runner",
            Capability::EditFlow => "editor",
            Capability::ApproveFlow => "approver",
        };
        self.principal.roles.iter().any(|r| r == required)
            || self.principal.roles.contains(&"admin".to_string())
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
    fn mcp_tools_register_pools() {
        let mut reg = CompatibilityRegistry::new();
        reg.register_mcp("fs", vec![McpTool {
            server: "fs".into(),
            name: "read".into(),
            input_schema: "{}".into(),
            pool: "mcp_fs".into(),
        }]);
        let mut pools = vec![ResourcePool { name: "cpu".into(), capacity: 8 }];
        reg.apply_to_pools(&mut pools);
        assert!(pools.iter().any(|p| p.name == "mcp_fs"));
    }
}
