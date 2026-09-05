// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use std::collections::HashMap;
use std::sync::Arc;

use mox_ai_expert_svc::expert_traits::{ExpertConsultant, ExpertRegistry};
use mox_ai_expert_svc::types::{ConsultQuery, ConsultReport, ExpertMeta};
use mox_ai_flow_sdk::model::FlowGraph;

/// 一条业务 = (id, 名称, 域, 受监管?, 流程图构造器)
///
/// DIP：优化入口 `optimize` / `optimize_with` 不再直接依赖 `mox_ai_expert_svc::pipeline::mox_optimize`，
/// 而是走 `Arc<dyn ExpertConsultant>` trait；默认实现通过 `default_consultant()` 工厂注入。
pub struct Business {
    pub id: &'static str,
    pub name: &'static str,
    pub domain: &'static str,
    pub regulated: bool,
    pub build: fn() -> FlowGraph,
}

impl Business {
    /// 七维着色后交给璇玑优化（DIP 版：通过 ExpertConsultant trait，不出现 concrete struct）。
    ///
    /// 返回 `ConsultReport`（归一化投影报告：steps / score / vetoed），
    /// 替代此前直接暴露 `mox_ai_expert_svc::pipeline::GovernanceReport` 这一内部 concrete 类型。
    pub fn optimize(&self) -> ConsultReport {
        self.optimize_with(mox_ai_expert_svc::expert_traits::default_consultant())
    }

    /// 指定 consultant（DIP 证据：测试可替换 Mock 实现，无需真实璇玑引擎）。
    pub fn optimize_with(&self, consultant: Arc<dyn ExpertConsultant>) -> ConsultReport {
        let q = build_query(self);
        consultant
            .consult_blocking(&q)
            .unwrap_or_else(|e| ConsultReport {
                report_id: q.id.clone(),
                steps: vec![format!("[business-catalog] optimize 失败: {}", e)],
                score: 0.0,
                vetoed: true,
                reason: Some(format!("error: {}", e)),
            })
    }
}

/// 把业务配置（domain / regulated）编码成 ConsultQuery.ctx，供 ExpertServiceImpl 解析。
///
/// ctx 键与 `mox_ai_expert_svc::services::ExpertServiceImpl::consult_sync` 约定一致。
fn build_query(biz: &Business) -> ConsultQuery {
    let raw = (biz.build)();
    let mut ctx: HashMap<String, String> = HashMap::new();
    ctx.insert(
        "flow_json".into(),
        serde_json::to_string(&raw).unwrap_or_default(),
    );
    ctx.insert("tenant".into(), biz.domain.into());
    ctx.insert("namespace".into(), "ns".into());
    ctx.insert("principal".into(), "architect".into());
    ctx.insert("roles".into(), "admin".into());
    ctx.insert("pool_browser".into(), "1".into());
    ctx.insert(
        "regulated".into(),
        if biz.regulated {
            "true".into()
        } else {
            "false".into()
        },
    );
    ctx.insert("max_parallel".into(), "8".into());
    ctx.insert("max_cost_budget".into(), "100".into());
    ctx.insert("sla_ms".into(), "50000".into());
    ConsultQuery {
        id: biz.id.into(),
        query: biz.name.into(),
        ctx,
    }
}

/// 基于 Arc<dyn ExpertRegistry>（DIP）为每条业务注册其对应领域专家元信息。
///
/// 【归一化】架构层不再维护 "业务 ID → 专属关键词" 的硬编码 switch 表。
/// 专家元信息统一从 `Business` 自身字段（id / name / domain / regulated）泛化推导：
/// - 专家 id    → `biz-<id>`
/// - 专家名    → `<name>·领域专家`
/// - 能力集合  → `default_caps_for(&b)`（基于 域 + regulated flag 给出通用能力词，
///   不包含任何 政务/财务 等具体业务专属关键词）
///
/// 业务专属能力（政务的 pii/authz、财务的对账）
/// 由对应 `projects/business-*/` crate 自行 `registry.register(&custom_meta)` 外部注入，
/// 不再污染架构 business-catalog 源码。
pub async fn register_business_experts(
    registry: Arc<dyn ExpertRegistry>,
) -> mox_ai_expert_svc::types::Result<()> {
    use crate::flows::all_businesses;
    for b in all_businesses() {
        let meta = ExpertMeta {
            id: format!("biz-{}", b.id),
            name: format!("{}·领域专家", b.name),
            domain: b.domain.into(),
            capabilities: default_caps_for(&b),
            description: format!("业务目录泛化注册 · 业务={}/{}", b.id, b.name),
            dimension: Some("Business".into()),
        };
        registry.register(&meta).await?;
    }
    Ok(())
}

/// 【归一化】架构级通用能力推导：禁止出现任何具体业务专属关键词
/// （pii / 对账 / 留痕 / 政务 … 一律不得写入此处）。
///
/// | 条件 | 注入能力（通用抽象） |
/// |---|---|
/// | `regulated=true` | compliance / permission / security（强监管三件套） |
/// | domain = data/gov    | data / governance / observability（数据治理类域） |
/// | domain = finance     | resource / data / reconciliation（资源 + 数据一致性） |
/// | domain = service     | knowledge / routing / observability（对话服务类域） |
/// | domain = integration/mcp | resource / plugin / permission（插件编排域） |
/// | domain = science/algo    | algorithm / validation / compliance（科学计算域） |
/// | 其它兜底             | business（通用业务） |
///
/// 业务专属能力在 `projects/business-*/src/lib.rs::expert_meta()` 中自声明并外部注入。
fn default_caps_for(b: &Business) -> Vec<String> {
    let mut caps: Vec<String> = Vec::new();

    if b.regulated {
        caps.extend(
            ["compliance", "permission", "security"]
                .iter()
                .map(|s| s.to_string()),
        );
    }

    match b.domain {
        "data" | "gov" => {
            caps.extend(
                ["data", "governance", "observability"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        "finance" => {
            caps.extend(
                ["resource", "data", "reconciliation"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        "service" => {
            caps.extend(
                ["knowledge", "routing", "observability"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        "integration" | "mcp" => {
            caps.extend(
                ["resource", "plugin", "permission"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        "science" | "algo" => {
            caps.extend(
                ["algorithm", "validation", "compliance"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        _ => {
            caps.push("business".into());
        }
    }

    caps.sort();
    caps.dedup();
    caps
}
