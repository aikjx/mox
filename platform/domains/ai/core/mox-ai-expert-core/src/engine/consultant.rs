// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/mox

//! 专家咨询器实现：把 ConsultQuery 转为 ConsultReport
//!
//! 设计要点：
//! - 实现 proto::ExpertConsultant trait（DIP）
//! - 内部调用 mox_optimize 核心管线
//! - 支持同步 consult_blocking（core 引擎是同步的）
//! - 从 ConsultQuery.ctx 中解析 FlowGraph JSON / 租户信息 / 配额

use crate::context::{GovernContext, Principal, ResourceQuota, Tenant};
use crate::pipeline::mox_optimize;
use anyhow::Result;
use async_trait::async_trait;
use mox_ai_expert_proto::{ConsultQuery, ConsultReport, ExpertConsultant};
use mox_ai_flow_svc::model::FlowGraph;
use std::collections::HashMap;

/// 专家咨询器（实现 proto::ExpertConsultant trait）
///
/// 把 `ConsultQuery → ConsultReport` 的桥接。
/// 查询约定（ConsultQuery.ctx 键值）：
/// - `"flow_json"`：FlowGraph 的 JSON 字符串（优先使用）
/// - `"tenant"` / `"namespace"`：`Tenant` 信息（缺省为 `default/default`）
/// - `"principal"`：主体名（缺省 `consultant`）
/// - `"roles"`：逗号分隔的角色列表
/// - `"regulated"`：是否强合规租户（"true"/"false"）
/// - `"max_parallel"` / `"max_cost_budget"` / `"sla_ms"`：可选配额数值
pub struct ExpertConsultantImpl {
    /// 可覆盖的默认租户配额
    default_quota: Option<ResourceQuota>,
}

impl Default for ExpertConsultantImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpertConsultantImpl {
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
                        "[ExpertConsultantImpl] 未传入 FlowGraph，跳过璇玑 14 维分析（空报告）"
                            .into(),
                    ],
                    score: 1.0,
                    vetoed: false,
                    reason: None,
                })
            }
            Some(flow) => {
                let ctx = self.build_context(query)?;
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
                    format!(
                        "归一化裁决（{} 项建议采纳）",
                        rep.adopted_suggestions.len()
                    ),
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

    /// 从 ConsultQuery 构建 GovernContext
    fn build_context(&self, query: &ConsultQuery) -> Result<GovernContext> {
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

        Ok(ctx)
    }
}

#[async_trait]
impl ExpertConsultant for ExpertConsultantImpl {
    async fn consult(&self, query: &ConsultQuery) -> Result<ConsultReport> {
        // 同步实现直接调用（core 引擎是同步的，async 仅为 trait 签名要求）
        self.consult_sync(query)
    }

    fn consult_blocking(&self, query: &ConsultQuery) -> Result<ConsultReport> {
        self.consult_sync(query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_expert_proto::ExpertConsultant;

    #[test]
    fn empty_query_returns_healthy() {
        let svc = ExpertConsultantImpl::new();
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
    fn with_flow_graph_returns_report() {
        let svc = ExpertConsultantImpl::new();
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

    #[tokio::test]
    async fn async_consult_works() {
        let svc = ExpertConsultantImpl::new();
        let query = ConsultQuery {
            id: "async-1".into(),
            query: "test".into(),
            ctx: HashMap::new(),
        };
        let rep = svc.consult(&query).await.unwrap();
        assert_eq!(rep.report_id, "async-1");
    }

    #[test]
    fn consult_blocking_works() {
        let svc = ExpertConsultantImpl::new();
        let query = ConsultQuery {
            id: "block-1".into(),
            query: "test".into(),
            ctx: HashMap::new(),
        };
        let rep = svc.consult_blocking(&query).unwrap();
        assert_eq!(rep.report_id, "block-1");
    }

    #[test]
    fn custom_quota_applied() {
        let svc = ExpertConsultantImpl::new().with_default_quota(ResourceQuota {
            max_parallel: 16,
            max_cost_budget: 10.0,
            sla_ms: 10_000,
        });
        let g = FlowGraph::new("f", "t");
        let flow_json = serde_json::to_string(&g).unwrap();
        let mut ctx = HashMap::new();
        ctx.insert("flow_json".into(), flow_json);
        let query = ConsultQuery {
            id: "q".into(),
            query: "analyze".into(),
            ctx,
        };
        let rep = svc.consult_sync(&query).unwrap();
        assert!(!rep.vetoed);
    }
}
