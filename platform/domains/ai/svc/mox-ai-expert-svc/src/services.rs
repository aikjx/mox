//! Concrete 服务实现：把 mox-expert 现有函数引擎包装为对外 domain trait 的实现。
//!
//! DIP 关系：
//! - `RegistryImpl`   impl `ExpertRegistry`   →  包装 `experts::all_experts()`，把内部 Expert 映射到 `ExpertMeta`。
//! - `ExpertServiceImpl` impl `ExpertConsultant` → 包装 `mox_optimize()`，把 `GovernanceReport` 归一化为 `ConsultReport`。
//! - `AllianceRouter` impl `AllianceOrchestrator` → 基于 Registry 做关键词 + 场景匹配路由。
//!
//! 下游 crate（hermes-flow-bridge / business-catalog）改造成依赖 `Arc<dyn ExpertConsultant>` 等，
//! 不再直接依赖这些具体 struct 名字，从而把依赖方向从「下游 → concrete」反转为「下游 → trait ← concrete」。

use crate::context::{GovernContext, Principal, ResourceQuota, Tenant};
use crate::expert_traits::{AllianceOrchestrator, ExpertConsultant, ExpertRegistry};
use crate::experts::all_experts;
use crate::ir::Dimension;
use crate::pipeline::mox_optimize;
use crate::types::{ConsultQuery, ConsultReport, ExpertMeta, Result, RoutingDecision, TaskSpec};
use async_trait::async_trait;
use mox_ai_flow_svc::model::FlowGraph;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ============================================================================
// RegistryImpl：内存实现的专家注册表（从 all_experts() 预填充）
// ============================================================================

/// 具体专家注册表实现：内存 HashMap + 读写锁。启动时从 `all_experts()` 填默认 14 位专家。
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
        // 预填内置专家：把 crate::experts::all_experts() 中的 trait 对象映射为 ExpertMeta。
        let experts = all_experts();
        for e in experts {
            let id = e.id();
            let dim = e.dimension();
            let meta = ExpertMeta {
                id: id.clone(),
                name: dimension_to_display_name(dim),
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
/// - `"flow_json"`：FlowGraph 的 JSON 字符串（优先使用；真实引擎必须有它才能分析）
/// - `"tenant"` / `"namespace"`：`Tenant` 信息（缺省为 `default/default`）
/// - `"principal"`：主体名（缺省 `consultant`）
/// - `"max_parallel"` / `"max_cost_budget"` / `"sla_ms"`：可选配额数值
///
/// 如果 ctx 中没有 flow_json，退化为「空咨询报告」（score=1.0，vetoed=false），
/// 这样 Mock / 轻量下游即便不传图，也能拿到合法报告。
pub struct ExpertServiceImpl {
    /// 可覆盖的默认租户配额（None 时使用内置默认）
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
        // 1) 从 ctx 中尝试解析 FlowGraph JSON
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
                // 2) 构造 GovernContext
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
                            .unwrap_or(100.0),
                        sla_ms: query
                            .ctx
                            .get("sla_ms")
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(50_000),
                    };
                }
                // 3) 调用璇玑优化
                let rep = mox_optimize(&flow, &ctx);
                // 4) 归一化为 ConsultReport
                let avg_score = if rep.expert_scores.is_empty() {
                    1.0
                } else {
                    let total: f64 = rep.expert_scores.iter().map(|(_, s)| *s).sum();
                    (total / rep.expert_scores.len() as f64).clamp(0.0, 1.0)
                };
                let mut steps = Vec::with_capacity(4);
                steps.push(format!(
                    "[1/3] 璇玑 14 专家并行诊断 · 参与专家={} · 平均分={:.3}",
                    rep.expert_scores.len(),
                    avg_score
                ));
                steps.push(format!(
                    "[2/3] 算法验证 {} · {}",
                    if rep.algo.vetoed {
                        "否决⛨"
                    } else {
                        "通过"
                    },
                    rep.algo.summary
                ));
                steps.push(format!(
                    "[3/3] 治理闸门 {}（通过={} · SLA={} · 预算={}）",
                    if rep.gate.approved {
                        "通过"
                    } else {
                        "驳回"
                    },
                    rep.gate.approved,
                    rep.gate.sla_ok,
                    rep.gate.budget_ok,
                ));
                let vetoed = rep.algo.vetoed || !rep.gate.approved;
                let reason = if vetoed {
                    Some(format!(
                        "否决组合原因: algo.vetoed={}, gate.approved={}；详情：{}",
                        rep.algo.vetoed, !rep.gate.approved, rep.algo.summary,
                    ))
                } else {
                    None
                };
                Ok(ConsultReport {
                    report_id: if query.id.is_empty() {
                        rep.flow_id.clone()
                    } else {
                        query.id.clone()
                    },
                    steps,
                    score: avg_score,
                    vetoed,
                    reason,
                })
            }
        }
    }
}

#[async_trait]
impl ExpertConsultant for ExpertServiceImpl {
    async fn consult(&self, query: &ConsultQuery) -> Result<ConsultReport> {
        // 同步实现 + tokio::task::spawn_blocking 桥接，避免阻塞 async runtime
        let q = query.clone();
        let self_arc = Arc::new(self.default_quota.clone());
        let owned_q = q.clone();
        // 这里 sync 实现是纯 CPU 计算，使用 spawn_blocking 是规范
        let rep = tokio::task::spawn_blocking(move || {
            // 重建一个临时 Self 并调用 consult_sync
            let svc = ExpertServiceImpl {
                default_quota: (*self_arc).clone(),
            };
            svc.consult_sync(&owned_q)
        })
        .await
        .map_err(|e| anyhow::anyhow!("spawn_blocking join error: {}", e))??;
        Ok(rep)
    }

    /// 覆写默认实现：直接走原生 consult_sync（无 runtime 开销）。
    fn consult_blocking(&self, query: &ConsultQuery) -> Result<ConsultReport> {
        self.consult_sync(query)
    }
}

// ============================================================================
// AllianceRouter：基于 Registry 的简单编排路由
// ============================================================================

/// 联盟编排路由实现：用 TaskSpec.scenario + constraints 匹配注册表中的专家。
///
/// 策略（简单可替换；真实生产可换成语义相似度 / 学习到的先验概率）：
/// 1) 若 TaskSpec.constraints["prefer_expert"] 存在，直接命中（confidence=1.0）。
/// 2) 否则：把 scenario + constraints.values 拼成关键词集合，与 ExpertMeta.capabilities 做匹配。
/// 3) 最高分胜出；若无人匹配，返回 id="default" 的兜底决策。
pub struct AllianceRouter {
    registry: Arc<dyn ExpertRegistry>,
}

impl AllianceRouter {
    pub fn new(registry: Arc<dyn ExpertRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl AllianceOrchestrator for AllianceRouter {
    async fn route(&self, task: &TaskSpec) -> Result<RoutingDecision> {
        // 1) 优先命中 prefer_expert 约束
        if let Some(preferred) = task.constraints.get("prefer_expert") {
            if self.registry.find(preferred).await?.is_some() {
                return Ok(RoutingDecision {
                    expert_id: preferred.clone(),
                    confidence: 1.0,
                    reason: format!("命中约束 prefer_expert={}", preferred),
                });
            }
        }
        // 2) 基于关键词匹配
        let scenario_words = tokenize(&task.scenario);
        let constraint_words: Vec<String> = task
            .constraints
            .values()
            .flat_map(|v| tokenize(v))
            .collect();
        let all_words: Vec<&str> = scenario_words
            .iter()
            .chain(constraint_words.iter())
            .map(|s| s.as_str())
            .collect();
        let list = self
            .registry
            .list(task.constraints.get("domain").map(|s| s.as_str()))
            .await?;
        let mut best: Option<(ExpertMeta, f64)> = None;
        for m in list {
            let mut hit = 0usize;
            for w in &all_words {
                let wl = w.to_lowercase();
                if m.id.to_lowercase().contains(&wl)
                    || m.name.to_lowercase().contains(&wl)
                    || m.capabilities
                        .iter()
                        .any(|c| c.to_lowercase().contains(&wl))
                {
                    hit += 1;
                }
            }
            if hit == 0 {
                continue;
            }
            let score = hit as f64 / all_words.len().max(1) as f64;
            let score = score.clamp(0.0, 1.0);
            match best {
                None => best = Some((m, score)),
                Some((_, bs)) if score > bs => best = Some((m, score)),
                _ => {}
            }
        }
        Ok(match best {
            Some((m, conf)) => RoutingDecision {
                expert_id: m.id.clone(),
                confidence: conf,
                reason: format!("关键词匹配命中 expert={}，capabilities={:?}", m.id, m.capabilities),
            },
            None => RoutingDecision {
                expert_id: "default".into(),
                confidence: 0.0,
                reason: "未匹配到注册专家，返回 default（请在 Registry 注册对应专家或设置 prefer_expert）"
                    .into(),
            },
        })
    }
}

// ============================================================================
// 辅助
// ============================================================================

fn dimension_to_display_name(dim: Dimension) -> String {
    let s = match dim {
        Dimension::Business => "业务专家",
        Dimension::Algorithm => "算法专家",
        Dimension::Permission => "权限专家",
        Dimension::Resource => "资源专家",
        Dimension::Security => "安全专家",
        Dimension::Data => "数据专家",
        Dimension::Observability => "可观测专家",
        Dimension::Architecture => "架构专家",
        Dimension::SecurityCode => "代码安全专家",
        Dimension::CodeQuality => "代码质量专家",
        Dimension::Performance => "性能专家",
        Dimension::Testing => "测试专家",
        Dimension::Documentation => "文档专家",
        Dimension::Maintainability => "可维护性专家",
    };
    s.into()
}

fn dimension_capabilities(dim: Dimension) -> Vec<String> {
    let raw: &[&str] = match dim {
        Dimension::Business => &["business", "流程", "业务规则", "scenario"],
        Dimension::Algorithm => &["algorithm", "算法", "复杂度", "优化", "route"],
        Dimension::Permission => &["permission", "rbac", "权限", "authz", "allow", "deny"],
        Dimension::Resource => &["resource", "资源", "池", "quota", "cost"],
        Dimension::Security => &["security", "安全", "pii", "漏洞", "vuln", "auth"],
        Dimension::Data => &["data", "数据", "schema", "etl", "脱敏"],
        Dimension::Observability => &["observability", "观测", "log", "trace", "metric"],
        Dimension::Architecture => &["architecture", "架构", "设计", "pattern", "模块"],
        Dimension::SecurityCode => &["security-code", "代码安全", "sec", "注入", "secret"],
        Dimension::CodeQuality => &["code-quality", "质量", "lint", "重复", "clean"],
        Dimension::Performance => &["performance", "性能", "perf", "latency", "bench"],
        Dimension::Testing => &["testing", "测试", "覆盖率", "coverage", "case"],
        Dimension::Documentation => &["documentation", "文档", "doc", "注释", "readme"],
        Dimension::Maintainability => &["maintainability", "可维护性", "deps", "outdated"],
    };
    raw.iter().map(|s| s.to_string()).collect()
}

fn tokenize(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TaskSpec;

    #[tokio::test]
    async fn registry_impl_prefills_14_experts() {
        let r = RegistryImpl::new();
        let all = r.list(None).await.unwrap();
        assert!(all.len() >= 14, "璇玑至少 14 位专家，但实有 {}", all.len());
        assert!(r.find("security").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn registry_register_and_find() {
        let r = RegistryImpl::new();
        let m = ExpertMeta::new("x", "X", "demo").with_capabilities(["hello".into()]);
        r.register(&m).await.unwrap();
        let f = r.find("x").await.unwrap().unwrap();
        assert_eq!(f.name, "X");
        assert_eq!(f.domain, "demo");
    }

    #[test]
    fn consultant_sync_empty_returns_healthy() {
        let svc = ExpertServiceImpl::new();
        let q = ConsultQuery {
            id: "q".into(),
            query: "hello".into(),
            ctx: HashMap::new(),
        };
        let rep = svc.consult_sync(&q).unwrap();
        assert_eq!(rep.report_id, "q");
        assert!((rep.score - 1.0).abs() < 1e-9);
        assert!(!rep.vetoed);
    }

    #[tokio::test]
    async fn alliance_router_uses_prefer_expert() {
        let r = Arc::new(RegistryImpl::new());
        let router = AllianceRouter::new(r.clone());
        r.register(&ExpertMeta::new("demo-exp", "Demo", "t"))
            .await
            .unwrap();
        let task = TaskSpec {
            task_id: "t1".into(),
            scenario: "anything".into(),
            constraints: std::collections::HashMap::from_iter([(
                "prefer_expert".into(),
                "demo-exp".into(),
            )]),
        };
        let d = router.route(&task).await.unwrap();
        assert_eq!(d.expert_id, "demo-exp");
        assert!((d.confidence - 1.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn alliance_router_keyword_match() {
        let r = Arc::new(RegistryImpl::new());
        let router = AllianceRouter::new(r);
        let task = TaskSpec {
            task_id: "t2".into(),
            scenario: "security pii 政务脱敏".into(),
            constraints: HashMap::new(),
        };
        let d = router.route(&task).await.unwrap();
        // 预期命中 security 专家（或其他安全相关，只要不是 default 和 0.0 就行）
        assert!(
            d.confidence > 0.0,
            "route 应该匹配到关键词，但得到 default：{:?}",
            d
        );
    }
}
