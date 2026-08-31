// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Proto trait 的 concrete 实现（DIP 适配层）
//!
//! 把 core 内部引擎包装为 `mox-ai-expert-proto` 中定义的对外 trait：
//! - `RegistryImpl`      →  `ExpertRegistry` （包装 14 位专家）
//! - `ExpertServiceImpl` →  `ExpertConsultant` （包装 `mox_optimize`）
//! - `ConcreteGovernExpert` →  `GovernExpert` （包装治理引擎）
//!
//! 下游 crate 改依赖 `Arc<dyn ExpertConsultant>` 等 trait 对象，
//! 不再直接依赖这些具体 struct 名字，从而实现 DIP 依赖倒置。

use crate::context::{GovernContext, Principal, ResourceQuota, Tenant};
use crate::experts::all_experts;
use crate::pipeline::mox_optimize;
use mox_ai_expert_proto::Dimension;
use anyhow::Result;
use async_trait::async_trait;
use mox_ai_expert_proto::{
    ConsultQuery, ConsultReport, ExpertMeta, ExpertRegistry, GovernVerdict, GovernExpert, GovernLevel,
};
use mox_ai_flow_svc::model::FlowGraph;
use std::collections::HashMap;
use std::sync::RwLock;

// ============================================================================
// RegistryImpl：内存实现的专家注册表（从 all_experts() 预填充）
// ============================================================================

/// 具体专家注册表实现：内存 HashMap + 读写锁。
/// 启动时从 `all_experts()` 填默认 14 位专家。
pub struct RegistryImpl {
    inner: RwLock<HashMap<String, ExpertMeta>>,
}

impl Default for RegistryImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryImpl {
    pub fn new() -> Self {
        let s = Self {
            inner: RwLock::new(HashMap::new()),
        };
        // 预填内置专家：把 all_experts() 中的 trait 对象映射为 ExpertMeta
        let experts = all_experts();
        for e in experts {
            let id = e.id();
            let dim = e.dimension();
            let meta = ExpertMeta {
                id: id.clone(),
                name: dim.display_name().to_string(),
                domain: "*".into(),
                capabilities: dimension_capabilities(dim),
                description: format!("内置璇玑专家 · 维度={:?}", dim),
                dimension: Some(format!("{:?}", dim)),
            };
            let _ = s.inner.write().unwrap().insert(meta.id.clone(), meta);
        }
        s
    }
}

fn dimension_capabilities(dim: Dimension) -> Vec<String> {
    match dim {
        Dimension::Business => vec!["business".into(), "process".into(), "workflow".into()],
        Dimension::Algorithm => vec!["algorithm".into(), "optimization".into(), "llm".into()],
        Dimension::Permission => vec!["permission".into(), "authz".into(), "rbac".into()],
        Dimension::Resource => vec!["resource".into(), "quota".into(), "pool".into()],
        Dimension::Security => vec!["security".into(), "pii".into(), "data-leak".into()],
        Dimension::Data => vec!["data".into(), "privacy".into(), "lineage".into()],
        Dimension::Observability => vec!["observability".into(), "monitoring".into(), "tracing".into()],
        Dimension::Architecture => vec!["architecture".into(), "design".into(), "pattern".into()],
        Dimension::SecurityCode => vec!["security-code".into(), "sast".into(), "vulnerability".into()],
        Dimension::CodeQuality => vec!["code-quality".into(), "lint".into(), "complexity".into()],
        Dimension::Performance => vec!["performance".into(), "profiling".into(), "bottleneck".into()],
        Dimension::Testing => vec!["testing".into(), "coverage".into(), "qa".into()],
        Dimension::Documentation => vec!["documentation".into(), "docs".into(), "readme".into()],
        Dimension::Maintainability => vec!["maintainability".into(), "technical-debt".into(), "refactor".into()],
    }
}

#[async_trait]
impl ExpertRegistry for RegistryImpl {
    async fn register(&self, expert: &ExpertMeta) -> Result<()> {
        self.inner
            .write()
            .map_err(|e| anyhow::anyhow!("Registry lock poisoned: {}", e))?
            .insert(expert.id.clone(), expert.clone());
        Ok(())
    }

    async fn list(&self, domain: Option<&str>) -> Result<Vec<ExpertMeta>> {
        let guard = self
            .inner
            .read()
            .map_err(|e| anyhow::anyhow!("Registry lock poisoned: {}", e))?;
        let iter = guard.values().cloned();
        let out: Vec<ExpertMeta> = match domain {
            None => iter.collect(),
            Some("*") => iter.collect(),
            Some(d) => iter.filter(|m| m.domain == d).collect(),
        };
        Ok(out)
    }

    async fn find(&self, id: &str) -> Result<Option<ExpertMeta>> {
        let guard = self
            .inner
            .read()
            .map_err(|e| anyhow::anyhow!("Registry lock poisoned: {}", e))?;
        Ok(guard.get(id).cloned())
    }
}

// ============================================================================
// ExpertServiceImpl：把 mox_optimize 包装为 ExpertConsultant trait
// ============================================================================

/// 具体咨询实现：把 `ConsultQuery → ConsultReport` 的桥接。
///
/// 查询约定（ConsultQuery.ctx 键值）：
/// - `"flow_json"`：FlowGraph 的 JSON 字符串（优先使用）
/// - `"tenant"` / `"namespace"`：`Tenant` 信息（缺省为 `default/default`）
/// - `"principal"`：主体名（缺省 `consultant`）
/// - `"max_parallel"` / `"max_cost_budget"` / `"sla_ms"`：可选配额数值
pub struct ExpertServiceImpl {
    /// 可覆盖的默认租户配额
    default_quota: Option<ResourceQuota>,
}

impl Default for ExpertServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpertServiceImpl {
    pub fn new() -> Self {
        Self {
            default_quota: None,
        }
    }
    pub fn with_default_quota(mut self, q: ResourceQuota) -> Self {
        self.default_quota = Some(q);
        self
    }

    /// 同步便捷：供内部 concrete 调用方复用，避免 async 在同步测试里开销。
    pub fn consult_sync(&self, query: &ConsultQuery) -> Result<ConsultReport> {
        // 从 ctx 中尝试解析 FlowGraph JSON
        let flow_opt: Option<FlowGraph> = query
            .ctx
            .get("flow_json")
            .and_then(|s| serde_json::from_str::<FlowGraph>(s).ok());

        match flow_opt {
            None => {
                // 没有图，返回空报告（便于测试 / mock 路径）
                Ok(ConsultReport {
                    report_id: query.id.clone(),
                    steps: vec![
                        "[ExpertServiceImpl] 未传入 FlowGraph，跳过璇玑 14 维分析（空报告）".into(),
                    ],
                    score: 1.0,
                    vetoed: false,
                    reason: None,
                })
            }
            Some(flow) => {
                // 构造 GovernContext
                let tenant_name = query
                    .ctx
                    .get("tenant")
                    .cloned()
                    .unwrap_or_else(|| "default".into());
                let ns = query
                    .ctx
                    .get("namespace")
                    .cloned()
                    .unwrap_or_else(|| "default".into());
                let regulated = query
                    .ctx
                    .get("regulated")
                    .map(|s| s == "true")
                    .unwrap_or(false);
                let mut tenant = Tenant::new(&tenant_name, &ns).regulated(regulated);
                if let Some(pool) = query
                    .ctx
                    .get("pool_browser")
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    tenant = tenant.with_pool("browser", pool);
                }
                let principal_name = query
                    .ctx
                    .get("principal")
                    .cloned()
                    .unwrap_or_else(|| "consultant".into());
                let mut principal = Principal::new(&principal_name);
                if let Some(roles) = query.ctx.get("roles") {
                    principal =
                        principal.with_roles(roles.split(',').map(|s| s.to_string()).collect());
                }
                let mut ctx = GovernContext::new(tenant, principal);
                if let Some(q) = &self.default_quota {
                    ctx.quota = q.clone();
                } else {
                    ctx.quota = ResourceQuota {
                        max_parallel: query
                            .ctx
                            .get("max_parallel")
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(8),
                        max_cost_budget: query
                            .ctx
                            .get("max_cost_budget")
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(1.0),
                        sla_ms: query
                            .ctx
                            .get("sla_ms")
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(5_000),
                    };
                }

                let rep = mox_optimize(&flow, &ctx);

                // 计算综合分（各专家健康分的加权平均）
                let total_score: f64 = rep.expert_scores.iter().map(|(_, s)| *s).sum();
                let score = if rep.expert_scores.is_empty() {
                    1.0
                } else {
                    total_score / rep.expert_scores.len() as f64
                };

                let vetoed = rep.algo.vetoed || !rep.gate.approved;

                let steps = vec![
                    format!("{} 位专家并行诊断", rep.expert_scores.len()),
                    format!("归一化裁决（{} 项建议采纳）", rep.adopted_suggestions.len()),
                    format!(
                        "璇玑验证：{}",
                        if rep.algo.vetoed { "否决" } else { "通过" }
                    ),
                    format!(
                        "治理闸门：{}",
                        if rep.gate.approved { "通过" } else { "拦截" }
                    ),
                ];

                Ok(ConsultReport {
                    report_id: query.id.clone(),
                    steps,
                    score,
                    vetoed,
                    reason: if vetoed {
                        Some(rep.gate.reason.clone())
                    } else {
                        None
                    },
                })
            }
        }
    }
}

#[async_trait]
impl mox_ai_expert_proto::ExpertConsultant for ExpertServiceImpl {
    async fn consult(&self, query: &ConsultQuery) -> Result<ConsultReport> {
        // 同步实现直接调用（core 引擎是同步的，async 仅为 trait 签名要求）
        self.consult_sync(query)
    }

    fn consult_blocking(&self, query: &ConsultQuery) -> Result<ConsultReport> {
        self.consult_sync(query)
    }
}

// ============================================================================
// ConcreteGovernExpert：把治理引擎包装为 GovernExpert trait
// ============================================================================

/// 具体治理专家实现：包装 core 的治理引擎，实现 proto 的 `GovernExpert` trait。
///
/// 通过 `&dyn Any` 接收 FlowGraph，内部 downcast 为具体类型。
pub struct ConcreteGovernExpert;

impl Default for ConcreteGovernExpert {
    fn default() -> Self {
        Self::new()
    }
}

impl ConcreteGovernExpert {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl GovernExpert for ConcreteGovernExpert {
    async fn govern(
        &self,
        graph: &dyn std::any::Any,
        ctx: &dyn mox_ai_expert_proto::domain::GovernContext,
    ) -> GovernVerdict {
        // core 引擎本身是同步的，async 版本直接委托给同步实现
        // （这样避免 &dyn Any 被捕获进 future 导致 !Send 问题）
        self.govern_blocking(graph, ctx)
    }

    fn govern_blocking(
        &self,
        graph: &dyn std::any::Any,
        ctx: &dyn mox_ai_expert_proto::domain::GovernContext,
    ) -> GovernVerdict {
        // 同步快路径：core 引擎本身就是同步的，无需 tokio runtime
        let flow = match graph.downcast_ref::<FlowGraph>() {
            Some(f) => f,
            None => {
                return GovernVerdict {
                    level: GovernLevel::Warn,
                    score: 0.5,
                    reasons: vec!["[ConcreteGovernExpert] 无法识别的图类型".into()],
                    gate_id: "type-error".into(),
                }
            }
        };

        let tenant = Tenant::new(ctx.tenant(), ctx.namespace()).regulated(ctx.is_regulated());
        let principal = Principal::new(ctx.principal()).with_roles(ctx.roles().to_vec());
        let gctx = GovernContext::new(tenant, principal);
        let rep = mox_optimize(flow, &gctx);

        let level = if rep.gate.approved {
            GovernLevel::Pass
        } else if rep.algo.vetoed {
            GovernLevel::Block
        } else {
            GovernLevel::Warn
        };

        let total: f64 = rep.expert_scores.iter().map(|(_, s)| *s).sum();
        let score = if rep.expert_scores.is_empty() {
            1.0
        } else {
            total / rep.expert_scores.len() as f64
        };

        GovernVerdict {
            level,
            score,
            reasons: if rep.gate.approved {
                vec![]
            } else {
                vec![rep.gate.reason.clone()]
            },
            gate_id: "govern-core".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_expert_proto::ExpertConsultant;
    use mox_ai_expert_proto::domain::{GovernContext as _, MinimalGovernContext, MockGovernExpert};
    use mox_ai_flow_svc::model::FlowGraph;

    // ---- RegistryImpl 测试 ----

    #[tokio::test]
    async fn registry_has_fourteen_experts() {
        let reg = RegistryImpl::new();
        let experts = reg.list(None).await.unwrap();
        assert_eq!(experts.len(), 14, "注册表应预填 14 位专家");
    }

    #[tokio::test]
    async fn registry_find_by_id() {
        let reg = RegistryImpl::new();
        let found = reg.find("security").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "security");
    }

    // ---- ExpertServiceImpl 测试 ----

    #[test]
    fn expert_service_empty_query_returns_healthy() {
        let svc = ExpertServiceImpl::new();
        let query = ConsultQuery {
            id: "test-1".into(),
            query: "test".into(),
            ctx: HashMap::new(),
        };
        let rep = svc.consult_sync(&query).unwrap();
        assert_eq!(rep.report_id, "test-1");
        assert!((rep.score - 1.0).abs() < 1e-9);
        assert!(!rep.vetoed);
    }

    #[test]
    fn expert_service_with_flow_graph() {
        let svc = ExpertServiceImpl::new();
        let g = FlowGraph::new("test-flow", "测试流程");
        let flow_json = serde_json::to_string(&g).unwrap();
        let mut ctx = HashMap::new();
        ctx.insert("flow_json".into(), flow_json);
        ctx.insert("tenant".into(), "acme".into());
        ctx.insert("principal".into(), "alice".into());
        let query = ConsultQuery {
            id: "q-1".into(),
            query: "analyze".into(),
            ctx,
        };
        let rep = svc.consult_sync(&query).unwrap();
        assert_eq!(rep.report_id, "q-1");
        assert!(!rep.steps.is_empty());
        assert!(rep.score > 0.0 && rep.score <= 1.0);
    }

    // ---- ConcreteGovernExpert 测试 ----

    #[test]
    fn concrete_govern_expert_implements_trait() {
        // DIP 验证：ConcreteGovernExpert 实现了 proto::GovernExpert trait
        let expert = ConcreteGovernExpert::new();
        let flow = FlowGraph::new("f", "test");
        let ctx = MinimalGovernContext::default();
        let verdict = expert.govern_blocking(&flow, &ctx);
        // 空图应通过（骨架实现下）
        assert!(verdict.score > 0.0);
        assert_eq!(verdict.gate_id, "govern-core");
    }

    #[test]
    fn mock_vs_concrete_both_implement_trait() {
        // DIP 证据：MockGovernExpert 和 ConcreteGovernExpert 都实现 GovernExpert trait
        let mock = MockGovernExpert::default();
        let concrete = ConcreteGovernExpert::new();
        let ctx = MinimalGovernContext::default();

        let v_mock = mock.govern_blocking(&(), &ctx);
        let v_concrete = concrete.govern_blocking(
            &FlowGraph::new("x", "t"),
            &ctx,
        );

        // 两者都返回合法的 GovernVerdict
        assert!(v_mock.score > 0.0);
        assert!(v_concrete.score > 0.0);
    }

    // ---- 异步 trait 测试 ----

    #[tokio::test]
    async fn async_consult_works() {
        let svc = ExpertServiceImpl::new();
        let query = ConsultQuery {
            id: "async-1".into(),
            query: "test".into(),
            ctx: HashMap::new(),
        };
        let rep = svc.consult(&query).await.unwrap();
        assert_eq!(rep.report_id, "async-1");
    }
}
