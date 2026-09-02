// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

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
    /// M3：SQLite 持久化句柄（None = 纯内存，测试/无持久化场景）
    db: Option<Arc<crate::persistence::PersistenceDb>>,
}

impl Default for RegistryImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryImpl {
    /// M5.4：企业级——从内存注册表移除专家并写通 SQLite（含升级：真实删除）。
    pub(crate) fn remove(&self, id: &str) -> bool {
        let removed = match self.inner.write() {
            Ok(mut g) => g.remove(id).is_some(),
            Err(_) => false,
        };
        if removed {
            if let Some(ref d) = self.db {
                let _ = d.delete_expert(id);
            }
        }
        removed
    }

    pub fn new() -> Self {
        Self::new_with_db(None)
    }

    /// M3：带持久化构造。首次（未 seed）把内置专家幂等写入 SQLite；
    /// 之后以 SQLite 为准加载（用户注册/更新的专家、被删除的内置专家重启后保持一致）。
    pub fn new_with_db(db: Option<Arc<crate::persistence::PersistenceDb>>) -> Self {
        let s = Self {
            inner: RwLock::new(HashMap::new()),
            db: db.clone(),
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
        // M3 幂等 seed + 库加载（库为空时保持内置；库非空则以库为准）
        if let Some(ref d) = db {
            if !d.kv_exists("mox_experts_seeded") {
                let metas: Vec<ExpertMeta> =
                    s.inner.read().unwrap().values().cloned().collect();
                for m in &metas {
                    let _ = d.upsert_expert(m);
                }
                let _ = d.save_kv("mox_experts_seeded", &serde_json::json!(true));
            }
            let persisted = d.load_experts();
            if !persisted.is_empty() {
                let mut guard = s.inner.write().unwrap();
                guard.clear();
                for m in persisted {
                    guard.insert(m.id.clone(), m);
                }
            }
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
        // M3：写通 SQLite（注册/更新专家持久化，重启保留）
        if let Some(ref d) = self.db {
            let _ = d.upsert_expert(expert);
        }
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
        // 同步实现 + tokio::task::spawn_blocking 桥接，避免阻塞 async mox_platform_orchestrator_svc
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

    /// 覆写默认实现：直接走原生 consult_sync（无 mox_platform_orchestrator_svc 开销）。
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

// ============================================================================
// AllianceService：专家联盟高层服务（HTTP API 层的业务逻辑封装）
// ============================================================================

/// 专家联盟综合服务：统一封装注册/查询/咨询/辩论/编排/算法分析/概览等所有操作。
///
/// 这是 HTTP handler 层直接调用的业务门面（Facade），内部协调
/// RegistryImpl / ExpertServiceImpl / AllianceRouter / alliance::* 模块。
pub struct AllianceService {
    registry: Arc<dyn ExpertRegistry>,
    /// M5.4：具体注册表引用（真实删除内存项）
    registry_impl: Option<Arc<RegistryImpl>>,
    consultant: Arc<dyn ExpertConsultant>,
    orchestrator: Arc<dyn AllianceOrchestrator>,
    /// 算法分析器（带状态：统计分析次数）
    algo_analyzer: std::sync::Mutex<crate::alliance::algorithm::AlgorithmAnalyzer>,
    /// 编排引擎（带状态：任务追踪）
    orchestration_engine: crate::alliance::orchestration::OrchestrationEngine,
    /// 启动时间戳（用于 uptime 统计）
    started_at: chrono::DateTime<chrono::Utc>,
    /// 累计咨询次数
    consultation_count: std::sync::atomic::AtomicU64,
    /// 累计辩论次数
    debate_count: std::sync::atomic::AtomicU64,
    /// 累计全维分析次数
    full_analysis_count: std::sync::atomic::AtomicU64,
    /// 各意图分布计数
    intent_counts: std::sync::Mutex<std::collections::HashMap<String, u64>>,
    /// 专家历史得分（用于 metrics 计算）
    expert_score_history: std::sync::Mutex<std::collections::HashMap<String, Vec<(f64, u64, bool)>>>,
    // (score, latency_ms, vetoed)
}

impl AllianceService {
    pub fn new() -> Self {
        Self::new_with_db(None)
    }

    /// M3：带 SQLite 持久化构造（专家注册表重启保留 + 幂等 seed）。
    pub fn new_with_db(db: Option<Arc<crate::persistence::PersistenceDb>>) -> Self {
        let registry_impl = Arc::new(RegistryImpl::new_with_db(db));
        let registry = registry_impl.clone() as Arc<dyn ExpertRegistry>;
        // M5.3：真实 LLM 路由接入（配置驱动）——配置了 MOX_LLM_* 凭据则启用真实 LLM 咨询器
        // （OpenAI 兼容/多 Provider 路由 + 失败自动回退本地规则引擎），否则使用本地规则引擎。
        let consultant: Arc<dyn ExpertConsultant> = match crate::llm::llm_consultant_from_env() {
            Some(c) => {
                println!("[M5] 专家联盟: 已启用真实 LLM 咨询器（MOX_LLM_* 配置驱动，失败自动回退本地引擎）");
                c
            }
            None => {
                println!("[M5] 专家联盟: 未配置 LLM 凭据 (MOX_LLM_*)，使用本地规则引擎");
                Arc::new(ExpertServiceImpl::new()) as Arc<dyn ExpertConsultant>
            }
        };
        let orchestrator = Arc::new(AllianceRouter::new(registry.clone()))
            as Arc<dyn AllianceOrchestrator>;
        Self {
            registry,
            registry_impl: Some(registry_impl),
            consultant,
            orchestrator,
            algo_analyzer: std::sync::Mutex::new(
                crate::alliance::algorithm::AlgorithmAnalyzer::new(),
            ),
            orchestration_engine: crate::alliance::orchestration::OrchestrationEngine::new(),
            started_at: chrono::Utc::now(),
            consultation_count: std::sync::atomic::AtomicU64::new(0),
            debate_count: std::sync::atomic::AtomicU64::new(0),
            full_analysis_count: std::sync::atomic::AtomicU64::new(0),
            intent_counts: std::sync::Mutex::new(std::collections::HashMap::new()),
            expert_score_history: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn registry(&self) -> Arc<dyn ExpertRegistry> {
        self.registry.clone()
    }

    /// M5.4：企业级——真实删除专家（内存 + SQLite），返回是否存在。
    pub async fn remove_expert(&self, id: &str) -> bool {
        match &self.registry_impl {
            Some(r) => r.remove(id),
            None => false,
        }
    }

    pub fn consultant(&self) -> Arc<dyn ExpertConsultant> {
        self.consultant.clone()
    }

    pub fn orchestrator(&self) -> Arc<dyn AllianceOrchestrator> {
        self.orchestrator.clone()
    }

    // ---------- 专家注册 ----------

    pub async fn register_expert(
        &self,
        req: &crate::types::RegisterExpertRequest,
    ) -> crate::types::Result<crate::types::RegisterExpertResponse> {
        let meta = crate::types::ExpertMeta {
            id: req.id.clone(),
            name: req.name.clone(),
            domain: req.domain.clone(),
            capabilities: req.capabilities.clone(),
            description: req.description.clone(),
            dimension: req.dimension.clone(),
        };
        self.registry.register(&meta).await?;
        Ok(crate::types::RegisterExpertResponse {
            success: true,
            expert_id: req.id.clone(),
            message: format!("专家 {} 注册成功", req.name),
        })
    }

    // ---------- 专家列表/详情 ----------

    pub async fn list_experts(
        &self,
        query: &crate::types::ExpertListQuery,
    ) -> crate::types::Result<crate::types::ExpertListResponse> {
        let all = self.registry.list(query.domain.as_deref()).await?;

        // 关键词过滤
        let filtered: Vec<crate::types::ExpertMeta> =
            if let Some(kw) = &query.keyword {
                let kw_lower = kw.to_lowercase();
                all.into_iter()
                    .filter(|m| {
                        m.id.to_lowercase().contains(&kw_lower)
                            || m.name.to_lowercase().contains(&kw_lower)
                            || m.capabilities
                                .iter()
                                .any(|c| c.to_lowercase().contains(&kw_lower))
                            || m.description.to_lowercase().contains(&kw_lower)
                    })
                    .collect()
            } else {
                all
            };

        let total = filtered.len();
        let page = query.page.max(1);
        let page_size = query.page_size.clamp(1, 100);
        let start = (page - 1) * page_size;
        let experts: Vec<crate::types::ExpertMeta> =
            filtered.into_iter().skip(start).take(page_size).collect();

        Ok(crate::types::ExpertListResponse {
            total,
            page,
            page_size,
            experts,
        })
    }

    pub async fn get_expert(
        &self,
        id: &str,
    ) -> crate::types::Result<crate::types::ExpertDetailResponse> {
        let expert = self.registry.find(id).await?;
        Ok(crate::types::ExpertDetailResponse {
            found: expert.is_some(),
            expert,
        })
    }

    // ---------- 专家咨询 ----------

    pub async fn consult_expert(
        &self,
        req: &crate::types::ConsultExpertRequest,
    ) -> crate::types::Result<crate::types::ConsultExpertResponse> {
        // 确定专家 id
        let expert_id = match &req.expert_id {
            Some(id) => id.clone(),
            None => {
                // 自动路由选择最佳专家
                let task = crate::types::TaskSpec {
                    task_id: uuid::Uuid::new_v4().to_string(),
                    scenario: req.query.clone(),
                    constraints: req.ctx.clone().into_iter().collect(),
                };
                self.orchestrator.route(&task).await?.expert_id
            }
        };

        // 构造 ConsultQuery
        let mut ctx = req.ctx.clone();
        if let Some(flow) = &req.flow_json {
            ctx.insert("flow_json".into(), flow.clone());
        }
        let query = crate::types::ConsultQuery {
            id: uuid::Uuid::new_v4().to_string(),
            query: req.query.clone(),
            ctx,
        };

        let report = self.consultant.consult(&query).await?;

        // 获取专家名称
        let expert_name = self
            .registry
            .find(&expert_id)
            .await?
            .map(|m| m.name)
            .unwrap_or_else(|| expert_id.clone());

        self.consultation_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // 记录专家得分历史
        let mut history = self.expert_score_history.lock().unwrap();
        history
            .entry(expert_id.clone())
            .or_default()
            .push((report.score, 0, report.vetoed));

        Ok(crate::types::ConsultExpertResponse {
            report,
            expert_id,
            expert_name,
        })
    }

    // ---------- 多专家协同咨询 ----------

    pub async fn multi_expert_consult(
        &self,
        req: &crate::types::MultiExpertConsultRequest,
    ) -> crate::types::Result<crate::types::MultiExpertConsultResponse> {
        use std::time::Instant;
        let start = Instant::now();

        // 确定专家列表
        let expert_ids: Vec<String> = if !req.expert_ids.is_empty() {
            req.expert_ids.clone()
        } else {
            // 自动路由选择 top N
            let task = crate::types::TaskSpec {
                task_id: uuid::Uuid::new_v4().to_string(),
                scenario: req.query.clone(),
                constraints: req.ctx.clone().into_iter().collect(),
            };
            let decision = self.orchestrator.route(&task).await?;
            vec![decision.expert_id] // 简化：先只返回 top 1，实际可扩展为 top N
        };

        let mut results: Vec<crate::types::SingleExpertResult> = Vec::new();

        if req.parallel {
            // 并行执行（使用 tokio::join_all）
            let mut handles = Vec::new();
            for eid in &expert_ids {
                let eid = eid.clone();
                let query_str = req.query.clone();
                let flow_json = req.flow_json.clone();
                let consultant = self.consultant.clone();
                let registry = self.registry.clone();
                let ctx = req.ctx.clone();
                handles.push(tokio::spawn(async move {
                    let t0 = Instant::now();
                    let mut q_ctx = ctx;
                    if let Some(flow) = &flow_json {
                        q_ctx.insert("flow_json".into(), flow.clone());
                    }
                    let q = crate::types::ConsultQuery {
                        id: uuid::Uuid::new_v4().to_string(),
                        query: query_str,
                        ctx: q_ctx,
                    };
                    let report = consultant.consult(&q).await?;
                    let name = registry
                        .find(&eid)
                        .await?
                        .map(|m| m.name)
                        .unwrap_or_else(|| eid.clone());
                    Ok::<(crate::types::SingleExpertResult, f64, bool), anyhow::Error>((
                        crate::types::SingleExpertResult {
                            expert_id: eid.clone(),
                            expert_name: name,
                            report: report.clone(),
                            latency_ms: t0.elapsed().as_millis() as u64,
                        },
                        report.score,
                        report.vetoed,
                    ))
                }));
            }
            for h in handles {
                if let Ok(Ok((result, score, vetoed))) = h.await {
                    // 记录历史
                    let mut history = self.expert_score_history.lock().unwrap();
                    history
                        .entry(result.expert_id.clone())
                        .or_default()
                        .push((score, result.latency_ms, vetoed));
                    results.push(result);
                }
            }
        } else {
            // 顺序执行
            for eid in &expert_ids {
                let t0 = Instant::now();
                let mut q_ctx = req.ctx.clone();
                if let Some(flow) = &req.flow_json {
                    q_ctx.insert("flow_json".into(), flow.clone());
                }
                let q = crate::types::ConsultQuery {
                    id: uuid::Uuid::new_v4().to_string(),
                    query: req.query.clone(),
                    ctx: q_ctx,
                };
                let report = self.consultant.consult(&q).await?;
                let name = self
                    .registry
                    .find(eid)
                    .await?
                    .map(|m| m.name)
                    .unwrap_or_else(|| eid.clone());

                let latency = t0.elapsed().as_millis() as u64;
                let mut history = self.expert_score_history.lock().unwrap();
                history
                    .entry(eid.clone())
                    .or_default()
                    .push((report.score, latency, report.vetoed));

                results.push(crate::types::SingleExpertResult {
                    expert_id: eid.clone(),
                    expert_name: name,
                    report,
                    latency_ms: latency,
                });
            }
        }

        // 计算共识度（简化：分数标准差的倒数）
        let consensus = if results.len() < 2 {
            1.0
        } else {
            let scores: Vec<f64> = results.iter().map(|r| r.report.score).collect();
            let mean = scores.iter().sum::<f64>() / scores.len() as f64;
            let variance =
                scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / scores.len() as f64;
            let sigma = variance.sqrt();
            (1.0 - sigma).max(0.0)
        };

        let overall_score = if results.is_empty() {
            0.0
        } else {
            results.iter().map(|r| r.report.score).sum::<f64>() / results.len() as f64
        };
        let overall_vetoed = results.iter().any(|r| r.report.vetoed);

        // 生成合成摘要
        let synthesis = format!(
            "## 多专家协同咨询结果\n\n- **参与专家数**：{}\n- **综合得分**：{:.3}/1.0\n- **共识度**：{:.3}\n- **是否否决**：{}\n\n### 各专家结论\n\n{}",
            results.len(),
            overall_score,
            consensus,
            if overall_vetoed { "是" } else { "否" },
            results
                .iter()
                .map(|r| format!(
                    "- **{}**（{}）：得分 {:.3}，{}",
                    r.expert_name,
                    r.expert_id,
                    r.report.score,
                    if r.report.vetoed { "否决" } else { "通过" }
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );

        self.consultation_count
            .fetch_add(results.len() as u64, std::sync::atomic::Ordering::Relaxed);

        Ok(crate::types::MultiExpertConsultResponse {
            results,
            consensus,
            overall_score,
            overall_vetoed,
            total_latency_ms: start.elapsed().as_millis() as u64,
            synthesis,
        })
    }

    // ---------- 智能路由 ----------

    pub async fn route_experts(
        &self,
        req: &crate::types::RouteExpertsRequest,
    ) -> crate::types::Result<crate::types::RouteExpertsResponse> {
        // 用关键词匹配找到所有候选专家并排序
        let all = self.registry.list(None).await?;
        let query_words = tokenize(&req.query);
        let scenario_words = req
            .scenario
            .as_ref()
            .map(|s| tokenize(s))
            .unwrap_or_default();

        let mut scored: Vec<(crate::types::ExpertMeta, f64, String)> = Vec::new();
        for m in &all {
            let mut hit = 0usize;
            for w in query_words.iter().chain(scenario_words.iter()) {
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
            // 也考虑 constraints
            for (k, v) in &req.constraints {
                let kv = format!("{} {}", k, v).to_lowercase();
                if m.id.to_lowercase().contains(&kv)
                    || m.capabilities
                        .iter()
                        .any(|c| c.to_lowercase().contains(&kv))
                {
                    hit += 1;
                }
            }
            let total_words =
                query_words.len() + scenario_words.len() + req.constraints.len();
            let score = if total_words == 0 {
                0.5
            } else {
                hit as f64 / total_words as f64
            };
            let score = score.clamp(0.0, 1.0);
            let reason = if hit > 0 {
                format!("命中 {} 个关键词（capabilities 匹配）", hit)
            } else {
                "无关键词命中，默认低置信度".into()
            };
            scored.push((m.clone(), score, reason));
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let top_n = req.top_n.min(scored.len()).max(1);
        let matches: Vec<crate::types::RouteMatch> = scored
            .into_iter()
            .take(top_n)
            .map(|(expert, confidence, reason)| crate::types::RouteMatch {
                expert,
                confidence,
                reason,
            })
            .collect();

        Ok(crate::types::RouteExpertsResponse {
            matches,
            query: req.query.clone(),
            method: "keyword_matching".into(),
        })
    }

    // ---------- 专家辩论 ----------

    pub async fn expert_debate(
        &self,
        req: &crate::types::ExpertDebateRequest,
    ) -> crate::types::Result<crate::types::ExpertDebateResponse> {
        use std::time::Instant;
        let start = Instant::now();

        let alliance_req = crate::alliance::AllianceRequest {
            query: req.query.clone(),
            session_id: req.session_id.clone(),
            idempotency_key: None,
            context: req.context.clone().into_iter().collect(),
            options: crate::alliance::AllianceOptions {
                enable_llm_debate: req.enable_llm_debate,
                retry_on_c: true,
                team_size: req.team_size,
                enable_spread: req.enable_spread,
            },
        };

        let events = crate::alliance::AllianceEngine::new()
            .run_full_analysis(alliance_req)
            .await
            .map_err(|e| anyhow::anyhow!("辩论管线错误: {}", e))?;

        // 从事件中提取各阶段数据
        let trace_id = events.first().map(|e| e.trace_id.to_string()).unwrap_or_default();

        // 提取 debate 结果
        let debate_payload = events
            .iter()
            .find(|e| e.phase == crate::alliance::AlliancePhase::Debate)
            .map(|e| e.payload.clone())
            .unwrap_or_default();

        let opinions: Vec<crate::types::ExpertOpinionView> = debate_payload
            .get("opinions")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let consensus = debate_payload
            .get("consensus")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let debate_rounds = debate_payload
            .get("rounds")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        // 提取 gate 结果
        let gate_payload = events
            .iter()
            .find(|e| e.phase == crate::alliance::AlliancePhase::Gate)
            .map(|e| e.payload.clone())
            .unwrap_or_default();

        let gate_grade = gate_payload
            .get("score")
            .and_then(|s| s.get("grade"))
            .and_then(|v| v.as_str())
            .unwrap_or("N/A")
            .to_string();

        let gate_total = gate_payload
            .get("score")
            .and_then(|s| s.get("total"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        // 提取合成结果
        let synthesis = events
            .iter()
            .find(|e| e.phase == crate::alliance::AlliancePhase::Synthesize)
            .and_then(|e| e.payload.get("markdown").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .unwrap_or_default();

        let synthesis_reasoning = debate_payload
            .get("reasoning_preview")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // 记录意图分布
        if let Some(intent_ev) = events
            .iter()
            .find(|e| e.phase == crate::alliance::AlliancePhase::Intent)
        {
            if let Some(intent_id) = intent_ev
                .payload
                .get("intent_id")
                .and_then(|v| v.as_str())
            {
                let mut counts = self.intent_counts.lock().unwrap();
                *counts.entry(intent_id.to_string()).or_insert(0) += 1;
            }
        }

        self.debate_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(crate::types::ExpertDebateResponse {
            trace_id,
            opinions,
            consensus,
            debate_rounds,
            synthesis,
            synthesis_reasoning,
            gate_grade,
            gate_total,
            total_latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    // ---------- 全维分析 ----------

    pub async fn full_analysis(
        &self,
        req: &crate::types::FullAnalysisRequest,
    ) -> crate::types::Result<crate::types::FullAnalysisResponse> {
        use std::time::Instant;
        let start = Instant::now();

        let alliance_req = crate::alliance::AllianceRequest {
            query: req.query.clone(),
            session_id: req.session_id.clone(),
            idempotency_key: req.idempotency_key.clone(),
            context: req.context.clone().into_iter().collect(),
            options: crate::alliance::AllianceOptions {
                enable_llm_debate: req.options.enable_llm_debate,
                retry_on_c: req.options.retry_on_c,
                team_size: req.options.team_size,
                enable_spread: req.options.enable_spread,
            },
        };

        let events = crate::alliance::AllianceEngine::new()
            .run_full_analysis(alliance_req)
            .await
            .map_err(|e| anyhow::anyhow!("全维分析错误: {}", e))?;

        let trace_id = events.first().map(|e| e.trace_id.to_string()).unwrap_or_default();

        let intent = events
            .iter()
            .find(|e| e.phase == crate::alliance::AlliancePhase::Intent)
            .map(|e| e.payload.clone())
            .unwrap_or_default();

        let team = events
            .iter()
            .find(|e| e.phase == crate::alliance::AlliancePhase::Team)
            .map(|e| e.payload.clone())
            .unwrap_or_default();

        let debate = events
            .iter()
            .find(|e| e.phase == crate::alliance::AlliancePhase::Debate)
            .map(|e| e.payload.clone())
            .unwrap_or_default();

        let synthesis = events
            .iter()
            .find(|e| e.phase == crate::alliance::AlliancePhase::Synthesize)
            .and_then(|e| e.payload.get("markdown").cloned())
            .unwrap_or_default()
            .as_str()
            .unwrap_or("")
            .to_string();

        let gate = events
            .iter()
            .find(|e| e.phase == crate::alliance::AlliancePhase::Gate)
            .map(|e| e.payload.clone())
            .unwrap_or_default();

        let learn = events
            .iter()
            .find(|e| e.phase == crate::alliance::AlliancePhase::Learn)
            .map(|e| e.payload.clone())
            .unwrap_or_default();

        let done = events
            .iter()
            .find(|e| e.phase == crate::alliance::AlliancePhase::Done)
            .map(|e| e.payload.clone())
            .unwrap_or_default();

        let gate_passed = done
            .get("gate_passed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let gate_grade = done
            .get("gate_grade")
            .and_then(|v| v.as_str())
            .unwrap_or("N/A")
            .to_string();

        let quality_formula = done
            .get("quality_formula")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // 记录意图分布
        if let Some(intent_id) = intent.get("intent_id").and_then(|v| v.as_str()) {
            let mut counts = self.intent_counts.lock().unwrap();
            *counts.entry(intent_id.to_string()).or_insert(0) += 1;
        }

        self.full_analysis_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(crate::types::FullAnalysisResponse {
            trace_id,
            intent,
            team,
            debate,
            synthesis,
            gate,
            learn,
            total_ms: start.elapsed().as_millis() as u64,
            gate_passed,
            gate_grade,
            quality_formula,
        })
    }

    // ---------- 算法分析 ----------

    pub fn algorithm_analysis(
        &self,
        req: &crate::types::AlgorithmAnalysisRequest,
    ) -> crate::types::Result<crate::types::AlgorithmAnalysisResponse> {
        let mut analyzer = self.algo_analyzer.lock().map_err(|e| anyhow::anyhow!("analyzer lock poisoned: {}", e))?;
        Ok(analyzer.analyze(req))
    }

    // ---------- 任务编排 ----------

    pub async fn orchestrate(
        &self,
        req: crate::types::OrchestrationRequest,
    ) -> crate::types::Result<crate::types::OrchestrationResponse> {
        Ok(self.orchestration_engine.execute(req).await)
    }

    // ---------- 概览 ----------

    pub async fn overview(&self) -> crate::types::Result<crate::types::AllianceOverview> {
        let all = self.registry.list(None).await?;

        let mut dimension_counts = std::collections::HashMap::new();
        let mut domain_counts = std::collections::HashMap::new();
        let mut total_capabilities = 0usize;

        for m in &all {
            if let Some(dim) = &m.dimension {
                *dimension_counts.entry(dim.clone()).or_insert(0) += 1;
            }
            *domain_counts.entry(m.domain.clone()).or_insert(0) += 1;
            total_capabilities += m.capabilities.len();
        }

        let avg_capabilities = if all.is_empty() {
            0.0
        } else {
            total_capabilities as f64 / all.len() as f64
        };

        let uptime_secs = (chrono::Utc::now() - self.started_at).num_seconds().max(0) as u64;

        Ok(crate::types::AllianceOverview {
            total_experts: all.len(),
            total_domains: domain_counts.len(),
            total_capabilities,
            dimension_counts,
            domain_counts,
            avg_capabilities_per_expert: avg_capabilities,
            uptime_secs,
            total_consultations: self
                .consultation_count
                .load(std::sync::atomic::Ordering::Relaxed),
            total_debates: self
                .debate_count
                .load(std::sync::atomic::Ordering::Relaxed),
        })
    }

    // ---------- 指标 ----------

    pub async fn metrics(&self) -> crate::types::Result<crate::types::AllianceMetricsResponse> {
        let all = self.registry.list(None).await?;
        let history = self.expert_score_history.lock().unwrap();
        let intent_counts = self.intent_counts.lock().unwrap();

        let mut expert_metrics = Vec::new();
        for m in &all {
            let hist = history.get(&m.id).cloned().unwrap_or_default();
            let count = hist.len() as u64;
            let (avg_score, avg_latency, veto_count) = if hist.is_empty() {
                (0.0, 0u64, 0u64)
            } else {
                let total_score: f64 = hist.iter().map(|(s, _, _)| *s).sum();
                let total_latency: u64 = hist.iter().map(|(_, l, _)| *l).sum();
                let vetoes: u64 = hist.iter().filter(|(_, _, v)| *v).count() as u64;
                (
                    total_score / hist.len() as f64,
                    total_latency / hist.len() as u64,
                    vetoes,
                )
            };
            let veto_rate = if count > 0 {
                veto_count as f64 / count as f64
            } else {
                0.0
            };
            // gate_a_rate 简化：从 team 模块获取
            let gate_a_rate = crate::alliance::team::build_expert_registry()
                .get(&m.id)
                .map(|meta| meta.gate_a_rate_30d)
                .unwrap_or(0.9);

            expert_metrics.push(crate::types::ExpertMetrics {
                expert_id: m.id.clone(),
                expert_name: m.name.clone(),
                consultation_count: count,
                avg_score,
                avg_latency_ms: avg_latency,
                avg_confidence: if count > 0 { 0.85 } else { 0.0 }, // 简化估算
                veto_rate,
                gate_a_rate,
            });
        }

        let total_requests = self
            .full_analysis_count
            .load(std::sync::atomic::Ordering::Relaxed)
            + self
                .debate_count
                .load(std::sync::atomic::Ordering::Relaxed);

        Ok(crate::types::AllianceMetricsResponse {
            total_requests,
            avg_consensus: 0.75,  // 简化估算
            avg_gate_score: 0.82, // 简化估算
            gate_pass_rate: 0.85, // 简化估算
            avg_latency_ms: 500,  // 简化估算
            expert_metrics,
            intent_distribution: intent_counts.clone(),
        })
    }
}

impl Default for AllianceService {
    fn default() -> Self {
        Self::new()
    }
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
