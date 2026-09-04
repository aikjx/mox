// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 专家联盟共享基础模块（Experts Common）
//!
//! 提供专家联盟全域共享的领域模型、持久化基础设施、通用类型与工具函数。
//! 所有专家联盟子模块（registry / collaboration / session / dispatcher / graph / orchestration）
//! 均通过 `use super::experts_common::*;` 引用本模块，确保类型归一化。
//!
//! 设计原则：
//! - 领域模型集中定义，避免各模块重复声明导致契约不一致
//! - SQLite 持久化（data/experts.db，WAL + 事务，经 `experts_db` 模块），
//!   历史 JSON 文件在启动时自动一次性导入并归档
//! - 响应信封统一使用 mox_api_protocol::{ApiResponse, api_ok, api_error}
//! - 时间戳统一 RFC3339（秒精度，UTC）

use axum::{Json, Router};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use mox_api_protocol::{ApiResponse, api_ok, api_error};
use mox_audit::{
    AuditAction, AuditActor, AuditContext, AuditError, AuditEvent, AuditOutcome, AuditResource,
    AuditSeverity, AuditSink, MultiSink, NoopSink,
};

// =====================================================================
// 一、核心领域模型：ExpertDescriptor（专家描述符）
// 对齐 docs/expert-alliance/expert-registry-and-protocol.md Schema
// =====================================================================

/// 专家能力标签
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertCapability {
    /// 能力唯一标识，如 "rust-backend"、"ml-architecture"
    pub id: String,
    /// 能力名称（中文展示）
    pub name: String,
    /// 能力领域分类
    pub domain: String,
    /// 熟练度 0-100
    pub proficiency: u8,
    /// 能力描述
    #[serde(default)]
    pub description: String,
}

/// 专家联系与在线状态
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExpertAvailability {
    /// online / busy / offline / away
    pub status: String,
    /// 上次活跃时间（RFC3339）
    #[serde(default)]
    pub last_active: String,
    /// 平均响应时间（分钟）
    #[serde(default)]
    pub avg_response_minutes: f64,
    /// 当前并发任务数
    #[serde(default)]
    pub current_load: u32,
    /// 最大并发任务数
    #[serde(default)]
    pub max_concurrent: u32,
}

/// 专家绩效指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExpertMetrics {
    /// 累计咨询次数
    #[serde(default)]
    pub total_consultations: u64,
    /// 今日咨询次数
    #[serde(default)]
    pub today_consultations: u64,
    /// 平均评分 0-5
    #[serde(default)]
    pub avg_rating: f64,
    /// 评分总数
    #[serde(default)]
    pub rating_count: u64,
    /// 解决率 0-1
    #[serde(default)]
    pub resolution_rate: f64,
    /// 首次响应正确率 0-1
    #[serde(default)]
    pub first_response_accuracy: f64,
    /// 累计服务时长（分钟）
    #[serde(default)]
    pub total_service_minutes: u64,
}

/// 专家描述符——专家联盟全域核心实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertDescriptor {
    /// 专家唯一 ID（注册时生成或由调用方指定）
    pub id: String,
    /// 专家名称
    pub name: String,
    /// 专家头像 URL（可选）
    #[serde(default)]
    pub avatar: String,
    /// 专家头衔/职称
    #[serde(default)]
    pub title: String,
    /// 所属组织/团队
    #[serde(default)]
    pub organization: String,
    /// 专家简介
    #[serde(default)]
    pub bio: String,
    /// 主要领域标签（如 ["backend", "ai", "architecture"]）
    #[serde(default)]
    pub domains: Vec<String>,
    /// 技术栈/技能标签
    #[serde(default)]
    pub skills: Vec<String>,
    /// 详细能力列表
    #[serde(default)]
    pub capabilities: Vec<ExpertCapability>,
    /// 在线与可用性状态
    #[serde(default)]
    pub availability: ExpertAvailability,
    /// 绩效指标
    #[serde(default)]
    pub metrics: ExpertMetrics,
    /// 专家类型：human / ai / hybrid
    #[serde(default = "default_expert_type")]
    pub expert_type: String,
    /// 计费模式：free / paid / subscription
    #[serde(default = "default_pricing")]
    pub pricing_model: String,
    /// 每小时费率（分）
    #[serde(default)]
    pub hourly_rate_cents: u32,
    /// 语言能力
    #[serde(default)]
    pub languages: Vec<String>,
    /// 地理位置/时区
    #[serde(default)]
    pub timezone: String,
    /// 认证状态：unverified / verified / certified
    #[serde(default = "default_verification")]
    pub verification_status: String,
    /// 标签（自由标签，用于检索和图谱聚类）
    #[serde(default)]
    pub tags: Vec<String>,
    /// 注册时间（RFC3339）
    #[serde(default)]
    pub created_at: String,
    /// 最后更新时间（RFC3339）
    #[serde(default)]
    pub updated_at: String,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 扩展元数据（KV）
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

fn default_expert_type() -> String { "ai".into() }
fn default_pricing() -> String { "free".into() }
fn default_verification() -> String { "verified".into() }
fn default_true() -> bool { true }

impl ExpertDescriptor {
    /// 创建最小化专家描述符（注册用）
    pub fn minimal(id: String, name: String) -> Self {
        let now = now_iso();
        Self {
            id,
            name,
            avatar: String::new(),
            title: String::new(),
            organization: String::new(),
            bio: String::new(),
            domains: Vec::new(),
            skills: Vec::new(),
            capabilities: Vec::new(),
            availability: ExpertAvailability {
                status: "online".into(),
                last_active: now.clone(),
                avg_response_minutes: 5.0,
                current_load: 0,
                max_concurrent: 5,
            },
            metrics: ExpertMetrics::default(),
            expert_type: default_expert_type(),
            pricing_model: default_pricing(),
            hourly_rate_cents: 0,
            languages: vec!["zh-CN".into(), "en".into()],
            timezone: "Asia/Shanghai".into(),
            verification_status: default_verification(),
            tags: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
            enabled: true,
            metadata: HashMap::new(),
        }
    }
}

// =====================================================================
// 二、会话领域模型
// =====================================================================

/// 会话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// 消息唯一 ID
    pub id: String,
    /// 角色：user / expert / system / assistant
    pub role: String,
    /// 发送者 ID（专家 ID 或用户 ID）
    #[serde(default)]
    pub sender_id: String,
    /// 发送者名称
    #[serde(default)]
    pub sender_name: String,
    /// 消息内容
    pub content: String,
    /// 消息类型：text / markdown / code / image / file
    #[serde(default = "default_msg_type")]
    pub msg_type: String,
    /// 附加数据（如引用、工具调用结果）
    #[serde(default)]
    pub attachments: Vec<Value>,
    /// 评分（用户对专家回复的评分，0-5）
    #[serde(default)]
    pub rating: Option<u8>,
    /// 发送时间（RFC3339）
    pub created_at: String,
}

fn default_msg_type() -> String { "text".into() }

/// 专家会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertSession {
    /// 会话唯一 ID
    pub id: String,
    /// 会话标题
    #[serde(default)]
    pub title: String,
    /// 关联专家 ID 列表
    #[serde(default)]
    pub expert_ids: Vec<String>,
    /// 发起用户 ID
    #[serde(default)]
    pub user_id: String,
    /// 会话类型：single / multi / debate / enterprise
    #[serde(default = "default_session_type")]
    pub session_type: String,
    /// 会话状态：active / archived / closed
    #[serde(default = "default_session_status")]
    pub status: String,
    /// 主题/问题描述
    #[serde(default)]
    pub topic: String,
    /// 消息列表
    #[serde(default)]
    pub messages: Vec<SessionMessage>,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
    /// 创建时间
    pub created_at: String,
    /// 最后活跃时间
    #[serde(default)]
    pub last_active_at: String,
    /// 归档时间
    #[serde(default)]
    pub archived_at: Option<String>,
}

fn default_session_type() -> String { "single".into() }
fn default_session_status() -> String { "active".into() }

// =====================================================================
// 三、调度引擎模型
// =====================================================================

/// 调度策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatcherConfig {
    /// 调度策略：round_robin / least_load / best_match / weighted_random
    #[serde(default = "default_strategy")]
    pub strategy: String,
    /// 是否启用智能匹配（基于能力相似度）
    #[serde(default = "default_true")]
    pub intelligent_matching: bool,
    /// 匹配阈值 0-1（低于此值不分配）
    #[serde(default = "default_match_threshold")]
    pub match_threshold: f64,
    /// 最大重试次数
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// 超时时间（秒）
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// 负载均衡权重（专家ID -> 权重）
    #[serde(default)]
    pub weights: HashMap<String, f64>,
    /// 熔断阈值（连续失败次数）
    #[serde(default = "default_circuit_breaker")]
    pub circuit_breaker_threshold: u32,
    /// 是否启用并发控制
    #[serde(default = "default_true")]
    pub concurrency_control: bool,
}

fn default_strategy() -> String { "best_match".into() }
fn default_match_threshold() -> f64 { 0.3 }
fn default_max_retries() -> u32 { 3 }
fn default_timeout() -> u64 { 120 }
fn default_circuit_breaker() -> u32 { 5 }

impl Default for DispatcherConfig {
    fn default() -> Self {
        Self {
            strategy: default_strategy(),
            intelligent_matching: true,
            match_threshold: default_match_threshold(),
            max_retries: default_max_retries(),
            timeout_seconds: default_timeout(),
            weights: HashMap::new(),
            circuit_breaker_threshold: default_circuit_breaker(),
            concurrency_control: true,
        }
    }
}

/// 调度任务记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchRecord {
    pub dispatch_id: String,
    pub task_type: String,
    pub input_summary: String,
    pub assigned_expert_ids: Vec<String>,
    pub strategy_used: String,
    pub match_scores: HashMap<String, f64>,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
}

// =====================================================================
// 四、能力图谱模型
// =====================================================================

/// 图谱节点（专家或能力）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: String, // expert / capability / domain
    #[serde(default)]
    pub properties: HashMap<String, Value>,
}

/// 图谱边（协作关系/能力关联）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub edge_type: String, // collaborates_with / has_capability / similar_to
    #[serde(default)]
    pub weight: f64,
    #[serde(default)]
    pub properties: HashMap<String, Value>,
}

/// 专家能力图谱
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExpertGraph {
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub edges: Vec<GraphEdge>,
    #[serde(default)]
    pub built_at: String,
    #[serde(default)]
    pub version: u64,
}

// =====================================================================
// 五、编排引擎模型
// =====================================================================

/// 编排计划步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub step_id: String,
    pub name: String,
    pub description: String,
    pub expert_id: Option<String>,
    pub step_type: String, // consult / analyze / debate / review / synthesize
    pub depends_on: Vec<String>,
    pub status: String, // pending / running / completed / failed / skipped
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
}

/// 协作计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationPlan {
    pub plan_id: String,
    pub task_id: Option<String>,
    pub title: String,
    pub description: String,
    pub expert_ids: Vec<String>,
    pub steps: Vec<PlanStep>,
    pub status: String, // draft / ready / running / completed / failed
    pub fusion_strategy: String, // majority_vote / weighted / best_of / consensus
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

/// 编排执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationRecord {
    pub execution_id: String,
    pub plan_id: String,
    pub task_type: String,
    pub status: String,
    pub expert_ids: Vec<String>,
    pub steps_completed: u32,
    pub steps_total: u32,
    pub result_summary: String,
    #[serde(default)]
    pub result: Option<Value>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: u64,
}

// =====================================================================
// 六、共享状态：专家注册中心（全域共享，Arc<Mutex<>>）
// =====================================================================

/// 专家联盟全域共享状态
#[derive(Clone)]
pub struct ExpertsSharedState {
    /// 专家注册表（ID -> ExpertDescriptor）
    pub registry: Arc<Mutex<HashMap<String, ExpertDescriptor>>>,
    /// 会话存储（ID -> ExpertSession）
    pub sessions: Arc<Mutex<HashMap<String, ExpertSession>>>,
    /// 调度配置
    pub dispatcher_config: Arc<Mutex<DispatcherConfig>>,
    /// 调度记录
    pub dispatch_records: Arc<Mutex<Vec<DispatchRecord>>>,
    /// 能力图谱
    pub graph: Arc<Mutex<ExpertGraph>>,
    /// 编排计划
    pub plans: Arc<Mutex<HashMap<String, CollaborationPlan>>>,
    /// 编排执行历史
    pub orchestration_history: Arc<Mutex<Vec<OrchestrationRecord>>>,
    /// 收藏集（专家ID集合）
    pub favorites: Arc<Mutex<std::collections::HashSet<String>>>,
    /// 审计上下文（专家写操作审计留痕：注册/编辑/禁用/咨询/会话）
    pub audit: Arc<AuditContext>,
}

impl ExpertsSharedState {
    pub fn new() -> Self {
        // 启动期一次性迁移：历史 JSON（data/experts_*.json）→ SQLite（data/experts.db）
        // 幂等：SQLite 已有数据则跳过导入；JSON 解析失败则保留原文件不归档
        crate::experts_db::migrate_json_to_sqlite();
        let registry = Arc::new(Mutex::new(load_registry()));
        let sessions = Arc::new(Mutex::new(load_sessions()));
        let graph = Arc::new(Mutex::new(load_graph()));
        // 若注册表为空，种子化内置专家（确保非空启动）
        {
            let mut reg = registry.lock();
            if reg.is_empty() {
                seed_builtin_experts(&mut reg);
                save_registry(&reg);
            }
        }
        // 若图谱为空，从注册表构建初始图谱
        {
            let mut g = graph.lock();
            if g.nodes.is_empty() {
                let reg = registry.lock();
                *g = build_graph_from_registry(&reg);
                save_graph(&g);
            }
        }
        Self {
            registry,
            sessions,
            dispatcher_config: Arc::new(Mutex::new(DispatcherConfig::default())),
            dispatch_records: Arc::new(Mutex::new(Vec::new())),
            graph,
            plans: Arc::new(Mutex::new(HashMap::new())),
            orchestration_history: Arc::new(Mutex::new(Vec::new())),
            favorites: Arc::new(Mutex::new(std::collections::HashSet::new())),
            audit: build_audit_context(),
        }
    }
}

// =====================================================================
// 六-B、企业级审计链路（接入 mox-audit）
// =====================================================================
//
// 设计：专家联盟所有写操作（注册/编辑/禁用/咨询/会话创建·消息·归档·删除/
// 调度·咨询）统一经 `emit_audit` 发射审计事件。审计事件写入 SHA-256 哈希链
// 防篡改，并分发到外部 Sink。默认 Sink 为文件 NDJSON（合规可追溯），路径与
// HMAC 密钥可由环境变量覆盖，失败静默不阻断业务。

/// 文件审计 Sink：将审计事件以 NDJSON 逐行追加写入文件
pub struct FileAuditSink {
    path: String,
}

impl AuditSink for FileAuditSink {
    fn append_sync(&self, event: &AuditEvent) -> Result<(), AuditError> {
        let line = serde_json::to_string(event)
            .map_err(|e| AuditError::WriteFailed(format!("序列化审计事件失败: {e}")))?;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| AuditError::WriteFailed(format!("打开审计文件 {} 失败: {}", self.path, e)))?;
        writeln!(f, "{}", line)
            .map_err(|e| AuditError::WriteFailed(format!("写入审计文件 {} 失败: {}", self.path, e)))?;
        Ok(())
    }

    fn name(&self) -> &str {
        "file_audit_sink"
    }
}

/// 构造专家联盟审计上下文
///
/// - 默认文件 Sink 路径：`data/audit/experts-audit.ndjson`（可由 `MOX_AUDIT_LOG_PATH` 覆盖）
/// - 默认 HMAC 签名密钥：`mox-experts-alliance-audit`（可由 `MOX_AUDIT_HMAC_SECRET` 覆盖）
/// - 若 `MOX_AUDIT_SINK=noop` 则仅用 NoopSink（开发/CI 不落盘）
pub fn build_audit_context() -> Arc<AuditContext> {
    let use_noop = std::env::var("MOX_AUDIT_SINK")
        .map(|v| v.eq_ignore_ascii_case("noop"))
        .unwrap_or(false);

    let sink: Box<dyn AuditSink> = if use_noop {
        Box::new(NoopSink)
    } else {
        let path = std::env::var("MOX_AUDIT_LOG_PATH")
            .unwrap_or_else(|_| "data/audit/experts-audit.ndjson".to_string());
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Box::new(FileAuditSink { path })
    };

    let secret = std::env::var("MOX_AUDIT_HMAC_SECRET")
        .unwrap_or_else(|_| "mox-experts-alliance-audit".to_string());

    let multi = MultiSink::new().with_sink(sink);
    Arc::new(AuditContext::new(Arc::new(multi)).with_hmac_secret(secret))
}

/// 专家联盟写操作审计发射辅助（失败静默，不阻断业务）
#[allow(clippy::too_many_arguments)]
pub fn emit_audit(
    state: &ExpertsSharedState,
    action: AuditAction,
    resource_type: &str,
    resource_id: &str,
    outcome: AuditOutcome,
    detail: Option<&str>,
) {
    let resource = AuditResource {
        resource_type: resource_type.to_string(),
        resource_id: resource_id.to_string(),
        tenant_id: "experts-alliance".to_string(),
        name: None,
    };
    let mut ev = AuditEvent::new(
        AuditActor::system(),
        action,
        resource,
        outcome,
        AuditSeverity::Info,
        "experts-alliance".to_string(),
    );
    if let Some(d) = detail {
        ev = ev.with_extra("detail", d.to_string().into());
    }
    let _ = state.audit.emit(ev);
}

// =====================================================================
// 七、持久化基础设施（SQLite：data/experts.db，WAL + 事务）
// =====================================================================
//
// 历史实现为 JSON 文件（data/experts_registry.json 等），已迁移到 SQLite
// 持久化层 `experts_db`（WAL 并发 + 事务化全量同步 + 列投影/JSON 文档混合
// 建模）。此处保留同名 load_/save_* API——全部 14 个调用点零改动；历史
// JSON 文件由 `ExpertsSharedState::new()` 启动时自动一次性导入并归档。
// 详细设计见 `experts_db` 模块文档。

pub fn load_registry() -> HashMap<String, ExpertDescriptor> {
    crate::experts_db::load_registry()
}

pub fn save_registry(registry: &HashMap<String, ExpertDescriptor>) {
    crate::experts_db::save_registry(registry)
}

pub fn load_sessions() -> HashMap<String, ExpertSession> {
    crate::experts_db::load_sessions()
}

pub fn save_sessions(sessions: &HashMap<String, ExpertSession>) {
    crate::experts_db::save_sessions(sessions)
}

pub fn load_graph() -> ExpertGraph {
    crate::experts_db::load_graph()
}

pub fn save_graph(graph: &ExpertGraph) {
    crate::experts_db::save_graph(graph)
}

// =====================================================================
// 八、内置种子专家（确保系统启动即有可检索专家）
// =====================================================================

fn seed_builtin_experts(registry: &mut HashMap<String, ExpertDescriptor>) {
    let builtins = vec![
        ("exp-architecture-001", "架构师·玄枢", "系统架构", vec!["architecture", "backend", "distributed"], vec!["Rust", "Go", "Kubernetes", "微服务"]),
        ("exp-ai-001", "AI算法·灵玑", "人工智能", vec!["ai", "ml", "nlp"], vec!["PyTorch", "Transformer", "RAG", "LLM"]),
        ("exp-data-001", "数据工程·衡宇", "数据工程", vec!["data", "database", "etl"], vec!["PostgreSQL", "ClickHouse", "Spark", "Flink"]),
        ("exp-security-001", "安全专家·镇岳", "信息安全", vec!["security", "crypto", "devsecops"], vec!["渗透测试", "零信任", "国密", "WAF"]),
        ("exp-cloud-001", "云原生·凌霄", "云原生", vec!["cloud", "devops", "sre"], vec!["K8s", "Terraform", "Prometheus", "Istio"]),
        ("exp-product-001", "产品战略·明鉴", "产品管理", vec!["product", "strategy", "ux"], vec!["需求分析", "用户研究", "增长", "OKR"]),
        ("exp-frontend-001", "前端工程·织锦", "前端开发", vec!["frontend", "web", "ui"], vec!["Vue", "React", "TypeScript", "WebGL"]),
        ("exp-math-001", "数学建模·璇玑", "数学与算法", vec!["math", "algorithm", "optimization"], vec!["拓扑学", "泛函分析", "最优化", "数值计算"]),
        ("exp-finance-001", "金融量化·泉通", "金融科技", vec!["finance", "quant", "risk"], vec!["量化交易", "风险模型", "R²拟合", "时间序列"]),
        ("exp-enterprise-001", "企业架构·鼎元", "企业咨询", vec!["enterprise", "consulting", "transformation"], vec!["数字化转型", "TOGAF", "流程再造", "组织变革"]),
    ];

    for (id, name, title, domains, skills) in builtins {
        let mut exp = ExpertDescriptor::minimal(id.into(), name.into());
        exp.title = title.into();
        exp.organization = "璇玑 RelGraph · 专家联盟".into();
        exp.bio = format!("{}领域资深专家，提供企业级咨询与技术支持。", title);
        exp.domains = domains.into_iter().map(String::from).collect();
        exp.skills = skills.into_iter().map(String::from).collect();
        exp.tags = exp.domains.clone();
        exp.capabilities = exp.skills.iter().map(|s| ExpertCapability {
            id: format!("cap-{}", s.to_lowercase().replace(' ', "-")),
            name: s.clone(),
            domain: exp.domains.first().cloned().unwrap_or_else(|| "general".into()),
            proficiency: 85,
            description: format!("{} 专业能力", s),
        }).collect();
        exp.metrics.avg_rating = 4.8;
        exp.metrics.rating_count = 128;
        exp.metrics.total_consultations = 356;
        exp.metrics.resolution_rate = 0.92;
        registry.insert(id.into(), exp);
    }
}

// =====================================================================
// 九、能力图谱构建（从注册表自动推导）
// =====================================================================

pub fn build_graph_from_registry(registry: &HashMap<String, ExpertDescriptor>) -> ExpertGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut domain_nodes: HashMap<String, bool> = HashMap::new();

    for exp in registry.values() {
        // 专家节点
        nodes.push(GraphNode {
            id: exp.id.clone(),
            label: exp.name.clone(),
            node_type: "expert".into(),
            properties: {
                let mut p = HashMap::new();
                p.insert("title".into(), json!(exp.title));
                p.insert("domains".into(), json!(exp.domains));
                p.insert("avg_rating".into(), json!(exp.metrics.avg_rating));
                p.insert("status".into(), json!(exp.availability.status));
                p
            },
        });

        // 领域节点 + 专家-领域边
        for domain in &exp.domains {
            if !domain_nodes.contains_key(domain) {
                domain_nodes.insert(domain.clone(), true);
                nodes.push(GraphNode {
                    id: format!("domain-{}", domain),
                    label: domain.clone(),
                    node_type: "domain".into(),
                    properties: HashMap::new(),
                });
            }
            edges.push(GraphEdge {
                source: exp.id.clone(),
                target: format!("domain-{}", domain),
                edge_type: "has_domain".into(),
                weight: 1.0,
                properties: HashMap::new(),
            });
        }
    }

    // 专家间协作边（基于共享领域计算相似度）
    let experts: Vec<&ExpertDescriptor> = registry.values().collect();
    for i in 0..experts.len() {
        for j in (i + 1)..experts.len() {
            let shared: Vec<&String> = experts[i].domains.iter()
                .filter(|d| experts[j].domains.contains(d))
                .collect();
            if !shared.is_empty() {
                let total_domains = experts[i].domains.len() + experts[j].domains.len() - shared.len();
                let similarity = if total_domains > 0 {
                    shared.len() as f64 / total_domains as f64
                } else { 0.0 };
                if similarity > 0.1 {
                    edges.push(GraphEdge {
                        source: experts[i].id.clone(),
                        target: experts[j].id.clone(),
                        edge_type: "collaborates_with".into(),
                        weight: similarity,
                        properties: {
                            let mut p = HashMap::new();
                            p.insert("shared_domains".into(), json!(shared));
                            p.insert("similarity".into(), json!(similarity));
                            p
                        },
                    });
                }
            }
        }
    }

    ExpertGraph {
        nodes,
        edges,
        built_at: now_iso(),
        version: 1,
    }
}

// =====================================================================
// 十、算法工具：能力匹配评分（TF-IDF 风格 + Jaccard 相似度）
// =====================================================================

/// 计算查询与专家的匹配分数（0-1）
/// 综合考虑：领域匹配、技能匹配、标签匹配、可用性、绩效
pub fn compute_match_score(query: &str, expert: &ExpertDescriptor) -> f64 {
    let query_lower = query.to_lowercase();
    let query_tokens: Vec<&str> = query_lower.split(|c: char| c.is_whitespace() || c == ',' || c == '、' || c == '/').collect();

    let mut score = 0.0f64;
    let mut weight_sum = 0.0f64;

    // 领域匹配（权重 0.30）
    let domain_hits = expert.domains.iter()
        .filter(|d| query_tokens.iter().any(|q| d.to_lowercase().contains(q) || q.contains(&d.to_lowercase())))
        .count() as f64;
    let domain_score = if expert.domains.is_empty() { 0.0 } else { domain_hits / expert.domains.len() as f64 };
    score += domain_score * 0.30;
    weight_sum += 0.30;

    // 技能匹配（权重 0.30）
    let skill_hits = expert.skills.iter()
        .filter(|s| query_tokens.iter().any(|q| s.to_lowercase().contains(q) || q.contains(&s.to_lowercase())))
        .count() as f64;
    let skill_score = if expert.skills.is_empty() { 0.0 } else { (skill_hits / expert.skills.len() as f64).min(1.0) };
    score += skill_score * 0.30;
    weight_sum += 0.30;

    // 名称/头衔/简介包含（权重 0.15）
    let bio_text = format!("{} {} {}", expert.name, expert.title, expert.bio).to_lowercase();
    let bio_hits = query_tokens.iter().filter(|q| !q.is_empty() && bio_text.contains(*q)).count() as f64;
    let bio_score = if query_tokens.is_empty() { 0.0 } else { (bio_hits / query_tokens.len() as f64).min(1.0) };
    score += bio_score * 0.15;
    weight_sum += 0.15;

    // 可用性加成（权重 0.10）
    let availability_score = match expert.availability.status.as_str() {
        "online" => 1.0,
        "busy" => 0.5,
        "away" => 0.3,
        _ => 0.1,
    };
    score += availability_score * 0.10;
    weight_sum += 0.10;

    // 绩效加成（权重 0.10）
    let perf_score = (expert.metrics.avg_rating / 5.0).min(1.0) * 0.5
        + expert.metrics.resolution_rate.min(1.0) * 0.5;
    score += perf_score * 0.10;
    weight_sum += 0.10;

    // 启用状态（权重 0.05）
    let enabled_score = if expert.enabled { 1.0 } else { 0.0 };
    score += enabled_score * 0.05;
    weight_sum += 0.05;

    if weight_sum > 0.0 { score / weight_sum } else { 0.0 }
}

/// 文本相似度（Jaccard + 字符 n-gram，用于会话相似搜索）
pub fn text_similarity(a: &str, b: &str) -> f64 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    if a_lower.is_empty() || b_lower.is_empty() {
        return 0.0;
    }
    // 字符 bigram Jaccard
    let a_bigrams: std::collections::HashSet<String> = a_lower
        .chars()
        .collect::<Vec<_>>()
        .windows(2)
        .map(|w| w.iter().collect())
        .collect();
    let b_bigrams: std::collections::HashSet<String> = b_lower
        .chars()
        .collect::<Vec<_>>()
        .windows(2)
        .map(|w| w.iter().collect())
        .collect();
    if a_bigrams.is_empty() || b_bigrams.is_empty() {
        return if a_lower == b_lower { 1.0 } else { 0.0 };
    }
    let intersection = a_bigrams.intersection(&b_bigrams).count() as f64;
    let union = a_bigrams.union(&b_bigrams).count() as f64;
    if union > 0.0 { intersection / union } else { 0.0 }
}

// =====================================================================
// 十一、通用工具函数
// =====================================================================

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn gen_id(prefix: &str) -> String {
    format!("{}-{}", prefix, uuid::Uuid::new_v4().simple())
}

pub fn ok(data: Value) -> ApiResponse<Value> {
    api_ok(data)
}

pub fn err(code: u16, msg: impl Into<String>) -> ApiResponse<Value> {
    api_error(code.into(), msg.into())
}

/// 分页参数解析
pub fn parse_pagination(params: &HashMap<String, String>) -> (usize, usize) {
    let page = params.get("page").and_then(|v| v.parse().ok()).unwrap_or(1);
    let page_size = params.get("page_size").or_else(|| params.get("limit"))
        .and_then(|v| v.parse().ok()).unwrap_or(20);
    let page = page.max(1);
    let page_size = page_size.clamp(1, 200);
    ((page - 1) * page_size, page_size)
}

// =====================================================================
// 十二、空路由占位（确保模块可独立编译，实际路由由各子模块注册）
// =====================================================================

pub fn build_experts_common_router() -> Router {
    Router::new()
}

// =====================================================================
// 单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expert_descriptor_minimal() {
        let exp = ExpertDescriptor::minimal("test-001".into(), "测试专家".into());
        assert_eq!(exp.id, "test-001");
        assert_eq!(exp.name, "测试专家");
        assert!(exp.enabled);
        assert_eq!(exp.availability.status, "online");
    }

    #[test]
    fn test_dispatcher_config_default() {
        let cfg = DispatcherConfig::default();
        assert_eq!(cfg.strategy, "best_match");
        assert!(cfg.intelligent_matching);
        assert_eq!(cfg.max_retries, 3);
    }

    #[test]
    fn test_compute_match_score() {
        let exp = ExpertDescriptor::minimal("exp-1".into(), "架构师".into());
        let score = compute_match_score("架构 后端 Rust", &exp);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_text_similarity() {
        let s1 = text_similarity("如何设计微服务架构", "微服务架构设计方法");
        let s2 = text_similarity("如何设计微服务架构", "今天天气很好");
        assert!(s1 > s2);
        assert!(s1 >= 0.0 && s1 <= 1.0);
    }

    #[test]
    fn test_gen_id() {
        let id = gen_id("test");
        assert!(id.starts_with("test-"));
        assert!(id.len() > 5);
    }

    #[test]
    fn test_parse_pagination() {
        let mut params = HashMap::new();
        params.insert("page".into(), "2".into());
        params.insert("page_size".into(), "10".into());
        let (offset, limit) = parse_pagination(&params);
        assert_eq!(offset, 10);
        assert_eq!(limit, 10);
    }

    // ── 企业级审计链路验证 ──────────────────────────────────────

    /// 构造一个审计上下文（NoopSink，不落盘，仅验证内存哈希链增长）
    fn noop_audit_ctx() -> Arc<AuditContext> {
        Arc::new(
            AuditContext::new(Arc::new(MultiSink::new().with_sink(Box::new(NoopSink))))
                .with_hmac_secret("test-secret".into()),
        )
    }

    /// 验证 emit_audit 会把事件追加进哈希链且链自洽
    #[test]
    fn test_emit_audit_appends_to_chain() {
        let state = ExpertsSharedState {
            registry: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            dispatcher_config: Arc::new(Mutex::new(DispatcherConfig::default())),
            dispatch_records: Arc::new(Mutex::new(Vec::new())),
            graph: Arc::new(Mutex::new(ExpertGraph::default())),
            plans: Arc::new(Mutex::new(HashMap::new())),
            orchestration_history: Arc::new(Mutex::new(Vec::new())),
            favorites: Arc::new(Mutex::new(std::collections::HashSet::new())),
            audit: noop_audit_ctx(),
        };

        let before = state.audit.chain_len();
        emit_audit(
            &state,
            AuditAction::Unknown("expert.register".into()),
            "expert",
            "exp-test-001",
            AuditOutcome::Success,
            Some("name=测试"),
        );
        emit_audit(
            &state,
            AuditAction::Unknown("expert.disable".into()),
            "expert",
            "exp-test-001",
            AuditOutcome::Success,
            None,
        );
        assert_eq!(state.audit.chain_len(), before + 2);
        // 哈希链自洽（防篡改）
        assert!(state.audit.verify_chain().is_ok());
    }

    /// 验证 build_audit_context 默认构造的上下文可正常发射（文件 Sink 不阻断）
    #[test]
    fn test_build_audit_context_emits() {
        let state = ExpertsSharedState {
            registry: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            dispatcher_config: Arc::new(Mutex::new(DispatcherConfig::default())),
            dispatch_records: Arc::new(Mutex::new(Vec::new())),
            graph: Arc::new(Mutex::new(ExpertGraph::default())),
            plans: Arc::new(Mutex::new(HashMap::new())),
            orchestration_history: Arc::new(Mutex::new(Vec::new())),
            favorites: Arc::new(Mutex::new(std::collections::HashSet::new())),
            audit: build_audit_context(),
        };
        let before = state.audit.chain_len();
        emit_audit(
            &state,
            AuditAction::ExpertDispatch,
            "dispatch",
            "disp-test",
            AuditOutcome::Success,
            Some("smoke"),
        );
        assert_eq!(state.audit.chain_len(), before + 1);
        assert!(state.audit.verify_chain().is_ok());
    }
}
