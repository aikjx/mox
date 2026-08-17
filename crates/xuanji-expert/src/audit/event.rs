//! 外部合规标准审计事件
//! 
//! 与内部 govern::AuditEvent 并行写入：
//! - 内部 AuditChain：验证自身一致性（防篡改检测）
//! - 外部 ExtAuditEvent：满足 SOC2/GDPR/ISO27001 合规要求
//! 
//! 事件结构标准化后，可对接任意合规框架。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// 外部合规标准审计事件
/// 
/// 命名 ExtAuditEvent 以区别于内部 govern::AuditEvent，
/// 两者同时写入，互不干扰。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtAuditEvent {
    /// 全局唯一事件 ID
    pub event_id: String,
    /// 事件时间（ISO8601，UTC）
    pub timestamp: DateTime<Utc>,
    /// 行动者
    pub actor: AuditActor,
    /// 操作类型
    pub action: AuditAction,
    /// 操作对象
    pub resource: AuditResource,
    /// 操作结果
    pub outcome: AuditOutcome,
    /// 严重程度（用于 SIEM 过滤）
    pub severity: AuditSeverity,
    /// 审计链哈希（前一个事件的 hash，用于链式追溯）
    pub chain_hash: String,
    /// 本事件内容哈希（防篡改）
    pub content_hash: String,
    /// HMAC 签名（非对称签名可选）
    pub signature: Option<String>,
    /// 租户 ID（多租户隔离必需）
    pub tenant_id: String,
    /// 会话 ID
    pub session_id: Option<String>,
    /// 客户端 IP
    pub client_ip: Option<String>,
    /// 额外上下文
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl ExtAuditEvent {
    pub fn new(
        actor: AuditActor,
        action: AuditAction,
        resource: AuditResource,
        outcome: AuditOutcome,
        severity: AuditSeverity,
        tenant_id: String,
    ) -> Self {
        let now = Utc::now();
        let content_hash = Self::compute_hash(&actor, &action, &resource, &outcome, &now);
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp: now,
            actor,
            action,
            resource,
            outcome,
            severity,
            chain_hash: String::new(),
            content_hash,
            signature: None,
            tenant_id,
            session_id: None,
            client_ip: None,
            extra: serde_json::Map::new(),
        }
    }

    fn compute_hash(
        actor: &AuditActor,
        action: &AuditAction,
        resource: &AuditResource,
        outcome: &AuditOutcome,
        ts: &DateTime<Utc>,
    ) -> String {
        let mut h = Sha256::new();
        h.update(actor.id.as_bytes());
        h.update(actor.role.as_bytes());
        h.update(format!("{action:?}").as_bytes());
        h.update(format!("{resource:?}").as_bytes());
        h.update(format!("{outcome:?}").as_bytes());
        h.update(ts.to_rfc3339().as_bytes());
        format!("{:032x}", h.finalize())
    }

    /// HMAC 签名
    pub fn sign(mut self, secret: &str) -> Self {
        let mut h = Sha256::new();
        h.update(self.content_hash.as_bytes());
        h.update(secret.as_bytes());
        self.signature = Some(format!("{:032x}", h.finalize()));
        self
    }

    pub fn verify_signature(&self, secret: &str) -> bool {
        if let Some(sig) = &self.signature {
            let mut h = Sha256::new();
            h.update(self.content_hash.as_bytes());
            h.update(secret.as_bytes());
            sig == &format!("{:032x}", h.finalize())
        } else {
            true
        }
    }

    pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
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

    pub fn with_chain_hash(mut self, hash: String) -> Self {
        self.chain_hash = hash;
        self
    }

    /// 转为 JSON 字节（NDJSON 友好）
    pub fn to_json_bytes(&self) -> Vec<u8> {
        serde_json::to_string(self).unwrap_or_default().into_bytes()
    }
}

/// 行动者
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditActor {
    pub id: String,
    pub role: String,
    pub source: ActorSource,
}

impl AuditActor {
    pub fn system() -> Self {
        Self { id: "system".into(), role: "system".into(), source: ActorSource::System }
    }
    pub fn ai_agent(agent_id: &str) -> Self {
        Self { id: agent_id.into(), role: "ai_agent".into(), source: ActorSource::Ai }
    }
    pub fn human(user_id: &str, role: &str) -> Self {
        Self { id: user_id.into(), role: role.into(), source: ActorSource::Human }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActorSource { Human, Ai, System, Unknown }

/// 操作类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditAction {
    FlowCreated, FlowModified, FlowDeleted, FlowApproved, FlowRejected,
    FlowVetoed, FlowExecuted, FlowRolledBack,
    XuanjiOptimize, ExpertDispatch, ExpertVeto,
    Reconciliation, VerificationGateway, GovernanceGate,
    AuditChainExtend, AuditChainTamperDetected,
    RBACDenied, PermissionDenied, SecurityViolation,
    LoginSuccess, LoginFailed,
    ConfigChanged, SkillRegistered, RuleRegistered, MCPPluginRegistered,
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
            Self::XuanjiOptimize => write!(f, "xuanji.optimize"),
            Self::ExpertDispatch => write!(f, "expert.dispatch"),
            Self::ExpertVeto => write!(f, "expert.veto"),
            Self::Reconciliation => write!(f, "xuanji.reconcile"),
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
            Self::Unknown(s) => write!(f, "unknown.{s}"),
        }
    }
}

/// 操作对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResource {
    pub resource_type: String,
    pub resource_id: String,
    pub tenant_id: String,
    pub name: Option<String>,
}

impl AuditResource {
    pub fn flow(flow_id: &str, tenant_id: &str) -> Self {
        Self { resource_type: "flow".into(), resource_id: flow_id.into(), tenant_id: tenant_id.into(), name: None }
    }
    pub fn rule(rule_id: &str, tenant_id: &str) -> Self {
        Self { resource_type: "rule".into(), resource_id: rule_id.into(), tenant_id: tenant_id.into(), name: None }
    }
    pub fn skill(skill_id: &str, tenant_id: &str) -> Self {
        Self { resource_type: "skill".into(), resource_id: skill_id.into(), tenant_id: tenant_id.into(), name: None }
    }
}

/// 操作结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditOutcome { Success, Failure, PartialFailure, Blocked, Tampered, Pending }

impl AuditOutcome {
    pub fn to_severity(&self) -> AuditSeverity {
        match self {
            Self::Success => AuditSeverity::Info,
            Self::Failure | Self::PartialFailure => AuditSeverity::Warning,
            Self::Blocked | Self::Tampered => AuditSeverity::Critical,
            Self::Pending => AuditSeverity::Debug,
        }
    }
}

/// 审计严重程度（RFC 5424 优先级映射）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditSeverity {
    Emergency = 0, Alert = 1, Critical = 2, Error = 3,
    Warning = 4, Notice = 5, Info = 6, Debug = 7,
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
impl From<AuditSeverity> for u8 {
    fn from(s: AuditSeverity) -> Self { 16 * 8 + s as u8 }
}

/// 测试用事件
#[cfg(test)]
pub fn test_event() -> ExtAuditEvent {
    ExtAuditEvent::new(
        AuditActor::human("test-user", "admin"),
        AuditAction::FlowCreated,
        AuditResource::flow("test-flow", "test-tenant"),
        AuditOutcome::Success,
        AuditSeverity::Info,
        "test-tenant".into(),
    )
}

/// 模块内统一以 `AuditEvent` 这个名字对外暴露（避免各处耦合 `ExtAuditEvent`）。
pub use ExtAuditEvent as AuditEvent;
