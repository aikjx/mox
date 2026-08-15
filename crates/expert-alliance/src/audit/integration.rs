//! 审计集成层：包装内部 AuditChain + 外部 AuditSink
//! 
//! 职责：
//! 1. 双写：内部 AuditChain 验证一致性 + 外部 sink 满足合规
//! 2. 在专家联盟关键节点自动注入审计事件
//! 3. 对外提供统一的 AuditContext

use crate::govern::AuditChain;
use super::{AuditSink, AuditError};
use super::event::{ExtAuditEvent, AuditAction, AuditOutcome, AuditSeverity, AuditActor, AuditResource};
use std::sync::Arc;

/// 审计上下文：内部链 + 外部 sink 双写
pub struct AuditContext {
    /// 内部哈希链（自验一致性）
    internal_chain: std::sync::Mutex<AuditChain>,
    /// 外部持久化 sink（S3/Syslog/Kafka，任选或组合）
    external_sink: Arc<dyn AuditSink>,
    /// HMAC 签名密钥
    hmac_secret: Option<String>,
    enabled: std::sync::atomic::AtomicBool,
}

impl AuditContext {
    pub fn new(external_sink: Arc<dyn AuditSink>) -> Self {
        Self {
            internal_chain: std::sync::Mutex::new(AuditChain::new()),
            external_sink,
            hmac_secret: None,
            enabled: std::sync::atomic::AtomicBool::new(true),
        }
    }

    pub fn with_hmac_secret(mut self, secret: String) -> Self {
        self.hmac_secret = Some(secret);
        self
    }

    pub fn enable(&self) { self.enabled.store(true, std::sync::atomic::Ordering::SeqCst); }
    pub fn disable(&self) { self.enabled.store(false, std::sync::atomic::Ordering::SeqCst); }

    /// 核心方法：双写审计事件
    pub fn log(&self, event: ExtAuditEvent) -> Result<(), AuditError> {
        if !self.enabled.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }

        // 1. 取内部链最新 hash
        let chain_hash = {
            let chain = self.internal_chain.lock().unwrap();
            chain.events.last().map(|e| e.hash.clone()).unwrap_or_else(|| "GENESIS".into())
        };

        // 2. 填充链哈希 + 签名
        let event = event
            .with_chain_hash(chain_hash)
            .sign(self.hmac_secret.as_deref().unwrap_or(""));

        // 3. 写入内部链（append 返回新事件，有 hash）
        let internal_event = {
            let mut chain = self.internal_chain.lock().unwrap();
            chain.append(&event.event_id, &event.resource.resource_id, &format!("{:#?}", event.action), &format!("{:#?}", event.outcome))
        };

        // 4. 写入外部 sink（使用内部链计算的 hash 作为 chain_hash）
        let mut ext_event = event;
        ext_event.chain_hash = internal_event.prev_hash;
        self.external_sink.append_sync(&ext_event)?;

        Ok(())
    }

    // ── 便捷方法 ───────────────────────────────────────────────

    pub fn log_success(&self, action: AuditAction, resource: AuditResource, tenant: &str) -> Result<(), AuditError> {
        self.log(ExtAuditEvent::new(
            AuditActor::system(), action, resource, AuditOutcome::Success,
            AuditSeverity::Info, tenant.into(),
        ))
    }

    pub fn log_veto(&self, resource: AuditResource, tenant: &str, expert: &str, reason: &str) -> Result<(), AuditError> {
        self.log(ExtAuditEvent::new(
            AuditActor::ai_agent(expert),
            AuditAction::ExpertVeto,
            resource,
            AuditOutcome::Blocked,
            AuditSeverity::Critical,
            tenant.into(),
        ).with_extra("reason", reason.into()))
    }

    pub fn log_security_violation(&self, resource: AuditResource, tenant: &str, detail: &str) -> Result<(), AuditError> {
        self.log(ExtAuditEvent::new(
            AuditActor::system(),
            AuditAction::SecurityViolation,
            resource,
            AuditOutcome::Blocked,
            AuditSeverity::Critical,
            tenant.into(),
        ).with_extra("detail", detail.into()))
    }

    pub fn log_rbac_denied(&self, resource: AuditResource, tenant: &str, actor_id: &str, permission: &str) -> Result<(), AuditError> {
        self.log(ExtAuditEvent::new(
            AuditActor::human(actor_id, "unknown"),
            AuditAction::RBACDenied,
            resource,
            AuditOutcome::Blocked,
            AuditSeverity::Warning,
            tenant.into(),
        ).with_extra("permission", permission.into()))
    }

    /// 验证内部链完整性
    pub fn verify_chain(&self) -> bool {
        self.internal_chain.lock().unwrap().verify()
    }

    /// 外部 sink 健康检查
    pub fn sink_health(&self) -> Result<(), AuditError> {
        self.external_sink.health_check()
    }

    pub fn flush(&self) -> Result<(), AuditError> {
        self.external_sink.flush()
    }
}

/// 带审计的执行装饰器
pub struct Audited<'a> {
    ctx: &'a AuditContext,
    action: AuditAction,
    resource: AuditResource,
    tenant: &'a str,
    actor: AuditActor,
}

impl<'a> Audited<'a> {
    pub fn new(ctx: &'a AuditContext, action: AuditAction, resource: AuditResource, tenant: &'a str) -> Self {
        Self { ctx, action, resource, tenant, actor: AuditActor::system() }
    }

    pub fn actor(mut self, actor: AuditActor) -> Self {
        self.actor = actor;
        self
    }

    /// 执行代码块，自动记录成功/失败
    pub fn execute<F, T>(&self, f: F) -> Result<T, AuditError>
    where F: FnOnce() -> Result<T, String> {
        let start = std::time::Instant::now();
        match f() {
            Ok(result) => {
                let _ = self.ctx.log(ExtAuditEvent::new(
                    self.actor.clone(), self.action.clone(), self.resource.clone(),
                    AuditOutcome::Success, AuditSeverity::Info, self.tenant.into(),
                ).with_extra("duration_ms", (start.elapsed().as_millis() as i64).into()));
                Ok(result)
            }
            Err(e) => {
                let err_clone = e.clone();
                let _ = self.ctx.log(ExtAuditEvent::new(
                    self.actor.clone(), self.action.clone(), self.resource.clone(),
                    AuditOutcome::Failure, AuditSeverity::Warning, self.tenant.into(),
                ).with_extra("error", err_clone.into()));
                Err(AuditError::WriteFailed(e))
            }
        }
    }
}
