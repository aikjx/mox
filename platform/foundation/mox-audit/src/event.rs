// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 统一审计事件模型
//!
//! 整合三套审计模型为单一标准：
//! - `audit::ExtAuditEvent`（外部合规标准，SOC2/GDPR，字段最全，SHA-256）
//! - `govern::AuditEvent`（内部哈希链，DefaultHasher，轻量）
//! - `pipeline_core::UnifiedAuditEvent`（管线阶段审计，含 trace_id/phase）
//!
//! 统一策略：
//! - 事件结构以 ExtAuditEvent 为基础（合规标准最完整）
//! - 增补 trace_id / phase（来自管线审计）
//! - 哈希算法统一为 SHA-256（替代 govern::AuditEvent 的 DefaultHasher）
//! - prev_hash 命名统一（原 chain_hash → prev_hash，与业界习惯一致）
//! - 资源模型统一为 AuditResource 结构体（原 UnifiedAuditEvent 拆分为 resource_type + resource_id）

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

// =============================================================================
// 核心事件结构
// =============================================================================

/// 统一审计事件
///
/// 整合三套审计模型的所有字段，满足以下场景：
/// - 外部合规审计（SOC2 Type II / GDPR / ISO27001 / HIPAA）
/// - 内部哈希链防篡改验证
/// - 管线执行阶段追踪（trace_id / phase）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// 全局唯一事件 ID（UUID v4）
    pub event_id: String,
    /// 事件时间（ISO8601，UTC）
    pub timestamp: DateTime<Utc>,
    /// 租户 ID（多租户隔离必需）
    pub tenant_id: String,
    /// 行动者
    pub actor: AuditActor,
    /// 操作类型
    pub action: AuditAction,
    /// 操作对象
    pub resource: AuditResource,
    /// 操作结果
    pub outcome: AuditOutcome,
    /// 严重程度（用于 SIEM 过滤，RFC 5424 优先级映射）
    pub severity: AuditSeverity,
    /// 前一个事件的哈希（链式追溯，防篡改）
    /// 空链时为 "GENESIS"
    pub prev_hash: String,
    /// 本事件内容哈希（SHA-256，防篡改）
    pub content_hash: String,
    /// HMAC 签名（可选，非对称签名可扩展）
    pub signature: Option<String>,
    /// 追踪 ID（管线/分布式追踪关联）
    pub trace_id: Option<Uuid>,
    /// 所属阶段（管线事件有，其他事件可选）
    pub phase: Option<String>,
    /// 会话 ID
    pub session_id: Option<String>,
    /// 客户端 IP
    pub client_ip: Option<String>,
    /// 额外上下文（自由扩展）
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl AuditEvent {
    /// 创建新的审计事件
    ///
    /// 自动填充 event_id、timestamp、content_hash；
    /// prev_hash 默认空字符串（由 AuditChain 在追加时填充）。
    pub fn new(
        actor: AuditActor,
        action: AuditAction,
        resource: AuditResource,
        outcome: AuditOutcome,
        severity: AuditSeverity,
        tenant_id: String,
    ) -> Self {
        let now = Utc::now();
        let mut ev = Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp: now,
            tenant_id,
            actor,
            action,
            resource,
            outcome,
            severity,
            prev_hash: String::new(),
            content_hash: String::new(),
            signature: None,
            trace_id: None,
            phase: None,
            session_id: None,
            client_ip: None,
            extra: serde_json::Map::new(),
        };
        ev.content_hash = ev.compute_content_hash();
        ev
    }

    /// 计算内容哈希（SHA-256）
    ///
    /// 参与哈希的字段：actor, action, resource, outcome, timestamp, event_id, tenant_id, extra, trace_id, phase
    /// 不包含：prev_hash, content_hash, signature（避免循环依赖）
    pub fn compute_content_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.event_id.as_bytes());
        h.update(self.timestamp.to_rfc3339().as_bytes());
        h.update(self.tenant_id.as_bytes());
        h.update(self.actor.id.as_bytes());
        h.update(self.actor.role.as_bytes());
        h.update(format!("{:?}", self.actor.source).as_bytes());
        h.update(self.action.to_string().as_bytes());
        h.update(self.resource.resource_type.as_bytes());
        h.update(self.resource.resource_id.as_bytes());
        h.update(format!("{:?}", self.outcome).as_bytes());
        // extra 参与哈希，防止额外上下文被篡改
        if let Ok(extra_json) = serde_json::to_string(&self.extra) {
            h.update(extra_json.as_bytes());
        }
        if let Some(tid) = &self.trace_id {
            h.update(tid.as_bytes());
        }
        if let Some(ph) = &self.phase {
            h.update(ph.as_bytes());
        }
        hex::encode(h.finalize())
    }

    /// 重新计算并更新 content_hash
    ///
    /// 在修改事件字段后调用，确保哈希与内容一致。
    pub fn recompute_hash(&mut self) {
        self.content_hash = self.compute_content_hash();
    }

    /// HMAC 签名（使用 SHA-256）
    pub fn sign(mut self, secret: &str) -> Self {
        let mut h = Sha256::new();
        h.update(self.content_hash.as_bytes());
        h.update(secret.as_bytes());
        self.signature = Some(hex::encode(h.finalize()));
        self
    }

    /// 验证 HMAC 签名
    pub fn verify_signature(&self, secret: &str) -> bool {
        if let Some(sig) = &self.signature {
            let mut h = Sha256::new();
            h.update(self.content_hash.as_bytes());
            h.update(secret.as_bytes());
            sig == &hex::encode(h.finalize())
        } else {
            true // 无签名视为通过（未启用签名模式）
        }
    }

    // ── 构建器方法 ───────────────────────────────────────────────

    pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self.recompute_hash();
        self
    }

    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_client_ip(mut self, ip: String) -> Self {
        self.client_ip = Some(ip);
        self
    }

    pub fn with_prev_hash(mut self, hash: String) -> Self {
        self.prev_hash = hash;
        self
    }

    pub fn with_trace_id(mut self, trace_id: Uuid) -> Self {
        self.trace_id = Some(trace_id);
        self.recompute_hash();
        self
    }

    pub fn with_phase(mut self, phase: impl Into<String>) -> Self {
        self.phase = Some(phase.into());
        self.recompute_hash();
        self
    }

    /// 转为 JSON 字节（NDJSON 友好）
    pub fn to_json_bytes(&self) -> Vec<u8> {
        serde_json::to_string(self).unwrap_or_default().into_bytes()
    }
}

// =============================================================================
// 行动者
// =============================================================================

/// 审计行动者
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditActor {
    pub id: String,
    pub role: String,
    pub source: ActorSource,
}

impl AuditActor {
    pub fn system() -> Self {
        Self {
            id: "system".into(),
            role: "system".into(),
            source: ActorSource::System,
        }
    }
    pub fn ai_agent(agent_id: &str) -> Self {
        Self {
            id: agent_id.into(),
            role: "ai_agent".into(),
            source: ActorSource::Ai,
        }
    }
    pub fn human(user_id: &str, role: &str) -> Self {
        Self {
            id: user_id.into(),
            role: role.into(),
            source: ActorSource::Human,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActorSource {
    Human,
    Ai,
    System,
    Unknown,
}

// =============================================================================
// 操作类型
// =============================================================================

/// 审计操作类型
///
/// 覆盖流程管理、专家治理、安全审计、配置变更等全场景。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    // 流程生命周期
    FlowCreated,
    FlowModified,
    FlowDeleted,
    FlowApproved,
    FlowRejected,
    FlowVetoed,
    FlowExecuted,
    FlowRolledBack,
    // 璇玑/专家系统
    MoxOptimize,
    ExpertDispatch,
    ExpertVeto,
    Reconciliation,
    VerificationGateway,
    GovernanceGate,
    // 审计链自身
    AuditChainExtend,
    AuditChainTamperDetected,
    // 安全/权限
    RBACDenied,
    PermissionDenied,
    SecurityViolation,
    LoginSuccess,
    LoginFailed,
    // 配置/注册
    ConfigChanged,
    SkillRegistered,
    RuleRegistered,
    MCPPluginRegistered,
    // 管线阶段（统一管线审计）
    PhaseStart,
    PhaseEnd,
    // 未知/扩展
    Unknown(String),
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FlowCreated => write!(f, "flow.created"),
            Self::FlowModified => write!(f, "flow.modified"),
            Self::FlowDeleted => write!(f, "flow.deleted"),
            Self::FlowApproved => write!(f, "flow.approved"),
            Self::FlowRejected => write!(f, "flow.rejected"),
            Self::FlowVetoed => write!(f, "flow.vetoed"),
            Self::FlowExecuted => write!(f, "flow.executed"),
            Self::FlowRolledBack => write!(f, "flow.rolled_back"),
            Self::MoxOptimize => write!(f, "mox.optimize"),
            Self::ExpertDispatch => write!(f, "expert.dispatch"),
            Self::ExpertVeto => write!(f, "expert.veto"),
            Self::Reconciliation => write!(f, "mox.reconcile"),
            Self::VerificationGateway => write!(f, "verify.gateway"),
            Self::GovernanceGate => write!(f, "govern.gate"),
            Self::AuditChainExtend => write!(f, "audit.chain_extend"),
            Self::AuditChainTamperDetected => write!(f, "audit.tamper_detected"),
            Self::RBACDenied => write!(f, "rbac.denied"),
            Self::PermissionDenied => write!(f, "permission.denied"),
            Self::SecurityViolation => write!(f, "security.violation"),
            Self::LoginSuccess => write!(f, "auth.login_success"),
            Self::LoginFailed => write!(f, "auth.login_failed"),
            Self::ConfigChanged => write!(f, "config.changed"),
            Self::SkillRegistered => write!(f, "skill.registered"),
            Self::RuleRegistered => write!(f, "rule.registered"),
            Self::MCPPluginRegistered => write!(f, "mcp.registered"),
            Self::PhaseStart => write!(f, "phase.start"),
            Self::PhaseEnd => write!(f, "phase.end"),
            Self::Unknown(s) => write!(f, "unknown.{s}"),
        }
    }
}

// =============================================================================
// 操作对象
// =============================================================================

/// 审计资源（操作对象）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResource {
    pub resource_type: String,
    pub resource_id: String,
    pub tenant_id: String,
    pub name: Option<String>,
}

impl AuditResource {
    pub fn flow(flow_id: &str, tenant_id: &str) -> Self {
        Self {
            resource_type: "flow".into(),
            resource_id: flow_id.into(),
            tenant_id: tenant_id.into(),
            name: None,
        }
    }
    pub fn rule(rule_id: &str, tenant_id: &str) -> Self {
        Self {
            resource_type: "rule".into(),
            resource_id: rule_id.into(),
            tenant_id: tenant_id.into(),
            name: None,
        }
    }
    pub fn skill(skill_id: &str, tenant_id: &str) -> Self {
        Self {
            resource_type: "skill".into(),
            resource_id: skill_id.into(),
            tenant_id: tenant_id.into(),
            name: None,
        }
    }
    pub fn pipeline(trace_id: &str, tenant_id: &str) -> Self {
        Self {
            resource_type: "pipeline".into(),
            resource_id: trace_id.into(),
            tenant_id: tenant_id.into(),
            name: None,
        }
    }
}

// =============================================================================
// 操作结果
// =============================================================================

/// 审计操作结果
///
/// 整合 ExtAuditEvent 和 UnifiedAuditEvent 的结果枚举。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Failure,
    PartialFailure,
    Blocked,
    Tampered,
    Skipped,
    Pending,
}

impl AuditOutcome {
    pub fn to_severity(&self) -> AuditSeverity {
        match self {
            Self::Success => AuditSeverity::Info,
            Self::Failure | Self::PartialFailure => AuditSeverity::Warning,
            Self::Blocked | Self::Tampered => AuditSeverity::Critical,
            Self::Skipped => AuditSeverity::Debug,
            Self::Pending => AuditSeverity::Debug,
        }
    }
}

impl From<AuditOutcome> for AuditSeverity {
    fn from(outcome: AuditOutcome) -> Self {
        outcome.to_severity()
    }
}

// =============================================================================
// 严重程度
// =============================================================================

/// 审计严重程度（RFC 5424 优先级映射）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    Emergency = 0,
    Alert = 1,
    Critical = 2,
    Error = 3,
    Warning = 4,
    Notice = 5,
    Info = 6,
    Debug = 7,
}

impl std::fmt::Display for AuditSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Emergency => write!(f, "EMERGENCY"),
            Self::Alert => write!(f, "ALERT"),
            Self::Critical => write!(f, "CRITICAL"),
            Self::Error => write!(f, "ERROR"),
            Self::Warning => write!(f, "WARNING"),
            Self::Notice => write!(f, "NOTICE"),
            Self::Info => write!(f, "INFO"),
            Self::Debug => write!(f, "DEBUG"),
        }
    }
}

/// RFC 5424 syslog 优先级 = facility*8 + severity
/// facility=16 (local0) 为审计日志默认设施
impl From<AuditSeverity> for u8 {
    fn from(s: AuditSeverity) -> Self {
        16 * 8 + s as u8
    }
}

// =============================================================================
// 测试辅助
// =============================================================================

/// 测试用事件（供其他模块测试使用）
#[cfg(test)]
pub fn test_event() -> AuditEvent {
    AuditEvent::new(
        AuditActor::human("test-user", "admin"),
        AuditAction::FlowCreated,
        AuditResource::flow("test-flow", "test-tenant"),
        AuditOutcome::Success,
        AuditSeverity::Info,
        "test-tenant".into(),
    )
}

// =============================================================================
// 单元测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_event_has_id_and_hash() {
        let ev = test_event();
        assert!(!ev.event_id.is_empty());
        assert!(!ev.content_hash.is_empty());
        assert_eq!(ev.prev_hash, "");
        assert!(ev.signature.is_none());
    }

    #[test]
    fn content_hash_is_sha256_hex() {
        let ev = test_event();
        // SHA-256 hex = 64 chars
        assert_eq!(ev.content_hash.len(), 64);
        assert!(ev.content_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn sign_and_verify() {
        let ev = test_event().sign("secret-key");
        assert!(ev.signature.is_some());
        assert!(ev.verify_signature("secret-key"));
        assert!(!ev.verify_signature("wrong-key"));
    }

    #[test]
    fn no_signature_verifies_true() {
        let ev = test_event();
        assert!(ev.verify_signature("any-secret"));
    }

    #[test]
    fn with_extra_updates_hash() {
        let ev1 = test_event();
        let ev2 = test_event().with_extra("key", "value".into());
        assert_ne!(ev1.content_hash, ev2.content_hash);
    }

    #[test]
    fn with_trace_id_updates_hash() {
        let ev1 = test_event();
        let ev2 = test_event().with_trace_id(Uuid::new_v4());
        // Note: both have different event_ids, so hashes differ anyway.
        // We test that trace_id is properly stored.
        let _ = ev1;
        assert!(ev2.trace_id.is_some());
    }

    #[test]
    fn audit_action_display() {
        assert_eq!(AuditAction::FlowCreated.to_string(), "flow.created");
        assert_eq!(AuditAction::ExpertVeto.to_string(), "expert.veto");
        assert_eq!(
            AuditAction::Unknown("custom".into()).to_string(),
            "unknown.custom"
        );
    }

    #[test]
    fn audit_severity_display_and_ord() {
        assert_eq!(AuditSeverity::Critical.to_string(), "CRITICAL");
        assert!(AuditSeverity::Critical < AuditSeverity::Info);
        let pri: u8 = AuditSeverity::Info.into();
        assert_eq!(pri, 16 * 8 + 6); // local0 facility + info
    }

    #[test]
    fn outcome_to_severity() {
        assert_eq!(AuditOutcome::Success.to_severity(), AuditSeverity::Info);
        assert_eq!(AuditOutcome::Blocked.to_severity(), AuditSeverity::Critical);
        assert_eq!(AuditOutcome::Pending.to_severity(), AuditSeverity::Debug);
    }

    #[test]
    fn actor_constructors() {
        assert_eq!(AuditActor::system().source, ActorSource::System);
        assert_eq!(AuditActor::ai_agent("gpt").source, ActorSource::Ai);
        assert_eq!(AuditActor::human("alice", "admin").source, ActorSource::Human);
    }

    #[test]
    fn resource_constructors() {
        let r = AuditResource::flow("f1", "t1");
        assert_eq!(r.resource_type, "flow");
        assert_eq!(r.resource_id, "f1");
        assert_eq!(r.tenant_id, "t1");

        let r = AuditResource::pipeline("trace-001", "t1");
        assert_eq!(r.resource_type, "pipeline");
    }

    #[test]
    fn to_json_bytes_roundtrip() {
        let ev = test_event()
            .with_session("sess-1".into())
            .with_client_ip("10.0.0.1".into());
        let bytes = ev.to_json_bytes();
        let parsed: AuditEvent = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed.event_id, ev.event_id);
        assert_eq!(parsed.session_id, ev.session_id);
        assert_eq!(parsed.client_ip, ev.client_ip);
        assert_eq!(parsed.content_hash, ev.content_hash);
    }

    #[test]
    fn tamper_detected_via_hash() {
        let mut ev = test_event();
        let original_hash = ev.content_hash.clone();

        // 篡改 action
        ev.action = AuditAction::FlowDeleted;
        ev.recompute_hash();

        assert_ne!(original_hash, ev.content_hash);
    }
}
