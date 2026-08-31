// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 审计上下文（AuditContext）
//!
//! 整合内部哈希链 + 外部 Sink，提供统一的审计入口。
//!
//! 职责：
//! 1. 双写：内部 AuditChain 验证一致性 + 外部 sink 满足合规
//! 2. 自动填充 prev_hash 和 HMAC 签名
//! 3. 提供便捷方法（log_success / log_veto / log_security_violation 等）
//!
//! 使用示例：
//! ```rust,ignore
//! use std::sync::Arc;
//! use mox_audit::{AuditContext, MultiSink, NoopSink, AuditEvent,
//!     AuditActor, AuditAction, AuditResource, AuditOutcome, AuditSeverity};
//!
//! let multi = MultiSink::new()
//!     .with_sink(Box::new(NoopSink));
//!
//! let ctx = AuditContext::new(Arc::new(multi))
//!     .with_hmac_secret("your-secret".into());
//!
//! let event = AuditEvent::new(
//!     AuditActor::human("alice", "admin"),
//!     AuditAction::MoxOptimize,
//!     AuditResource::flow("gov-pii", "gov"),
//!     AuditOutcome::Success,
//!     AuditSeverity::Info,
//!     "gov".into(),
//! );
//! ctx.emit(event).unwrap();
//! ```

use crate::chain::AuditChain;
use crate::error::AuditError;
use crate::event::{
    AuditAction, AuditActor, AuditEvent, AuditOutcome, AuditResource, AuditSeverity,
};
use crate::sink::AuditSink;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// 审计上下文：内部链 + 外部 sink 双写
///
/// 线程安全（内部链使用 Mutex，外部 sink 使用 Arc）。
pub struct AuditContext {
    /// 内部哈希链（自验一致性）
    internal_chain: Mutex<AuditChain>,
    /// 外部持久化 sink（S3/Syslog/自定义，任选或组合）
    external_sink: Arc<dyn AuditSink>,
    /// HMAC 签名密钥
    hmac_secret: Option<String>,
    /// 是否启用
    enabled: AtomicBool,
}

impl AuditContext {
    /// 创建新的审计上下文
    pub fn new(external_sink: Arc<dyn AuditSink>) -> Self {
        Self {
            internal_chain: Mutex::new(AuditChain::new()),
            external_sink,
            hmac_secret: None,
            enabled: AtomicBool::new(true),
        }
    }

    /// 设置 HMAC 签名密钥
    pub fn with_hmac_secret(mut self, secret: String) -> Self {
        self.hmac_secret = Some(secret);
        self
    }

    /// 启用审计
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// 禁用审计
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    // ── 核心方法 ───────────────────────────────────────────────

    /// 发射审计事件（核心方法）
    ///
    /// 流程：
    /// 1. 若禁用，直接返回 Ok
    /// 2. 获取内部链最新 hash 作为 prev_hash
    /// 3. 填充 prev_hash + HMAC 签名
    /// 4. 追加到内部哈希链
    /// 5. 写入外部 sink
    pub fn emit(&self, event: AuditEvent) -> Result<(), AuditError> {
        if !self.is_enabled() {
            return Ok(());
        }

        // 1. 取内部链最新 hash
        let prev_hash = {
            let chain = self.internal_chain.lock().unwrap();
            chain
                .latest_hash()
                .unwrap_or("GENESIS")
                .to_string()
        };

        // 2. 填充 prev_hash + 签名
        let mut event = event.with_prev_hash(prev_hash);
        if let Some(ref secret) = self.hmac_secret {
            event = event.sign(secret);
        }

        // 3. 写入内部链（append 返回新事件，有完整的 content_hash 和 prev_hash）
        let event_in_chain = {
            let mut chain = self.internal_chain.lock().unwrap();
            chain.append(event).clone()
        };

        // 4. 写入外部 sink
        self.external_sink.append_sync(&event_in_chain)?;

        Ok(())
    }

    /// 核心方法别名（兼容 audit::AuditContext::log）
    pub fn log(&self, event: AuditEvent) -> Result<(), AuditError> {
        self.emit(event)
    }

    // ── 便捷方法 ───────────────────────────────────────────────

    /// 记录成功操作
    pub fn log_success(
        &self,
        action: AuditAction,
        resource: AuditResource,
        tenant: &str,
    ) -> Result<(), AuditError> {
        self.emit(AuditEvent::new(
            AuditActor::system(),
            action,
            resource,
            AuditOutcome::Success,
            AuditSeverity::Info,
            tenant.into(),
        ))
    }

    /// 记录专家否决
    pub fn log_veto(
        &self,
        resource: AuditResource,
        tenant: &str,
        expert: &str,
        reason: &str,
    ) -> Result<(), AuditError> {
        self.emit(
            AuditEvent::new(
                AuditActor::ai_agent(expert),
                AuditAction::ExpertVeto,
                resource,
                AuditOutcome::Blocked,
                AuditSeverity::Critical,
                tenant.into(),
            )
            .with_extra("reason", reason.into()),
        )
    }

    /// 记录安全违规
    pub fn log_security_violation(
        &self,
        resource: AuditResource,
        tenant: &str,
        detail: &str,
    ) -> Result<(), AuditError> {
        self.emit(
            AuditEvent::new(
                AuditActor::system(),
                AuditAction::SecurityViolation,
                resource,
                AuditOutcome::Blocked,
                AuditSeverity::Critical,
                tenant.into(),
            )
            .with_extra("detail", detail.into()),
        )
    }

    /// 记录 RBAC 拒绝
    pub fn log_rbac_denied(
        &self,
        resource: AuditResource,
        tenant: &str,
        actor_id: &str,
        permission: &str,
    ) -> Result<(), AuditError> {
        self.emit(
            AuditEvent::new(
                AuditActor::human(actor_id, "unknown"),
                AuditAction::RBACDenied,
                resource,
                AuditOutcome::Blocked,
                AuditSeverity::Warning,
                tenant.into(),
            )
            .with_extra("permission", permission.into()),
        )
    }

    // ── 链与 Sink 管理 ─────────────────────────────────────────

    /// 验证内部链完整性
    pub fn verify_chain(&self) -> Result<(), AuditError> {
        self.internal_chain.lock().unwrap().verify()
    }

    /// 外部 sink 健康检查
    pub fn sink_health(&self) -> Result<(), AuditError> {
        self.external_sink.health_check()
    }

    /// 刷新外部 sink 缓冲
    pub fn flush(&self) -> Result<(), AuditError> {
        self.external_sink.flush()
    }

    /// 获取链长度
    pub fn chain_len(&self) -> usize {
        self.internal_chain.lock().unwrap().len()
    }
}

// =============================================================================
// Audited — 带审计的执行装饰器
// =============================================================================

/// 带审计的执行装饰器
///
/// 执行代码块，自动记录成功/失败审计事件。
pub struct Audited<'a> {
    ctx: &'a AuditContext,
    action: AuditAction,
    resource: AuditResource,
    tenant: &'a str,
    actor: AuditActor,
}

impl<'a> Audited<'a> {
    pub fn new(
        ctx: &'a AuditContext,
        action: AuditAction,
        resource: AuditResource,
        tenant: &'a str,
    ) -> Self {
        Self {
            ctx,
            action,
            resource,
            tenant,
            actor: AuditActor::system(),
        }
    }

    pub fn actor(mut self, actor: AuditActor) -> Self {
        self.actor = actor;
        self
    }

    /// 执行代码块，自动记录成功/失败
    pub fn execute<F, T>(&self, f: F) -> Result<T, AuditError>
    where
        F: FnOnce() -> Result<T, String>,
    {
        let start = std::time::Instant::now();
        match f() {
            Ok(result) => {
                let _ = self.ctx.emit(
                    AuditEvent::new(
                        self.actor.clone(),
                        self.action.clone(),
                        self.resource.clone(),
                        AuditOutcome::Success,
                        AuditSeverity::Info,
                        self.tenant.into(),
                    )
                    .with_extra("duration_ms", (start.elapsed().as_millis() as i64).into()),
                );
                Ok(result)
            }
            Err(e) => {
                let err_clone = e.clone();
                let _ = self.ctx.emit(
                    AuditEvent::new(
                        self.actor.clone(),
                        self.action.clone(),
                        self.resource.clone(),
                        AuditOutcome::Failure,
                        AuditSeverity::Warning,
                        self.tenant.into(),
                    )
                    .with_extra("error", err_clone.into()),
                );
                Err(AuditError::WriteFailed(e))
            }
        }
    }
}

// =============================================================================
// 单元测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sink::{MultiSink, NoopSink};

    fn make_ctx() -> AuditContext {
        let multi = MultiSink::new().with_sink(Box::new(NoopSink));
        AuditContext::new(Arc::new(multi)).with_hmac_secret("test-secret".into())
    }

    #[test]
    fn emit_basic_event() {
        let ctx = make_ctx();
        let ev = AuditEvent::new(
            AuditActor::system(),
            AuditAction::FlowCreated,
            AuditResource::flow("f1", "t1"),
            AuditOutcome::Success,
            AuditSeverity::Info,
            "t1".into(),
        );
        assert!(ctx.emit(ev).is_ok());
        assert_eq!(ctx.chain_len(), 1);
    }

    #[test]
    fn emit_updates_chain_hash() {
        let ctx = make_ctx();
        let ev1 = AuditEvent::new(
            AuditActor::system(),
            AuditAction::FlowCreated,
            AuditResource::flow("f1", "t1"),
            AuditOutcome::Success,
            AuditSeverity::Info,
            "t1".into(),
        );
        let ev2 = AuditEvent::new(
            AuditActor::system(),
            AuditAction::FlowModified,
            AuditResource::flow("f1", "t1"),
            AuditOutcome::Success,
            AuditSeverity::Info,
            "t1".into(),
        );

        ctx.emit(ev1).unwrap();
        ctx.emit(ev2).unwrap();

        assert_eq!(ctx.chain_len(), 2);
        assert!(ctx.verify_chain().is_ok());
    }

    #[test]
    fn disabled_skips_emit() {
        let ctx = make_ctx();
        ctx.disable();
        assert!(!ctx.is_enabled());

        let ev = AuditEvent::new(
            AuditActor::system(),
            AuditAction::FlowCreated,
            AuditResource::flow("f1", "t1"),
            AuditOutcome::Success,
            AuditSeverity::Info,
            "t1".into(),
        );
        assert!(ctx.emit(ev).is_ok());
        assert_eq!(ctx.chain_len(), 0); // 未写入

        ctx.enable();
        assert!(ctx.is_enabled());
    }

    #[test]
    fn log_success_convenience() {
        let ctx = make_ctx();
        ctx.log_success(
            AuditAction::ConfigChanged,
            AuditResource::flow("f1", "t1"),
            "t1",
        )
        .unwrap();
        assert_eq!(ctx.chain_len(), 1);
    }

    #[test]
    fn log_veto_convenience() {
        let ctx = make_ctx();
        ctx.log_veto(
            AuditResource::flow("f1", "t1"),
            "t1",
            "expert-1",
            "语义不一致",
        )
        .unwrap();
        assert_eq!(ctx.chain_len(), 1);
    }

    #[test]
    fn log_security_violation_convenience() {
        let ctx = make_ctx();
        ctx.log_security_violation(
            AuditResource::flow("f1", "t1"),
            "t1",
            "越权访问",
        )
        .unwrap();
        assert_eq!(ctx.chain_len(), 1);
    }

    #[test]
    fn log_rbac_denied_convenience() {
        let ctx = make_ctx();
        ctx.log_rbac_denied(
            AuditResource::flow("f1", "t1"),
            "t1",
            "user1",
            "flow.delete",
        )
        .unwrap();
        assert_eq!(ctx.chain_len(), 1);
    }

    #[test]
    fn verify_chain_after_multiple_emits() {
        let ctx = make_ctx();
        for i in 0..5 {
            let ev = AuditEvent::new(
                AuditActor::system(),
                AuditAction::FlowCreated,
                AuditResource::flow(&format!("f{i}"), "t1"),
                AuditOutcome::Success,
                AuditSeverity::Info,
                "t1".into(),
            );
            ctx.emit(ev).unwrap();
        }
        assert_eq!(ctx.chain_len(), 5);
        assert!(ctx.verify_chain().is_ok());
    }

    #[test]
    fn audited_decorator_success() {
        let ctx = make_ctx();
        let audited = Audited::new(
            &ctx,
            AuditAction::FlowCreated,
            AuditResource::flow("f1", "t1"),
            "t1",
        );
        let result = audited.execute(|| Ok(42));
        assert_eq!(result.unwrap(), 42);
        assert_eq!(ctx.chain_len(), 1);
    }

    #[test]
    fn audited_decorator_failure() {
        let ctx = make_ctx();
        let audited = Audited::new(
            &ctx,
            AuditAction::FlowCreated,
            AuditResource::flow("f1", "t1"),
            "t1",
        );
        let result: Result<i32, AuditError> = audited.execute(|| Err("something went wrong".into()));
        assert!(result.is_err());
        assert_eq!(ctx.chain_len(), 1);
    }

    #[test]
    fn flush_ok() {
        let ctx = make_ctx();
        assert!(ctx.flush().is_ok());
    }

    #[test]
    fn sink_health_ok() {
        let ctx = make_ctx();
        assert!(ctx.sink_health().is_ok());
    }

    #[test]
    fn log_alias_works() {
        let ctx = make_ctx();
        let ev = AuditEvent::new(
            AuditActor::system(),
            AuditAction::FlowCreated,
            AuditResource::flow("f1", "t1"),
            AuditOutcome::Success,
            AuditSeverity::Info,
            "t1".into(),
        );
        assert!(ctx.log(ev).is_ok()); // log 是 emit 的别名
        assert_eq!(ctx.chain_len(), 1);
    }

    #[test]
    fn hmac_signature_is_applied() {
        let ctx = make_ctx();
        let ev = AuditEvent::new(
            AuditActor::system(),
            AuditAction::FlowCreated,
            AuditResource::flow("f1", "t1"),
            AuditOutcome::Success,
            AuditSeverity::Info,
            "t1".into(),
        );
        ctx.emit(ev).unwrap();

        // 验证：链中事件应有签名
        let chain = ctx.internal_chain.lock().unwrap();
        let last = chain.events().last().unwrap();
        assert!(last.signature.is_some());
        assert!(last.verify_signature("test-secret"));
    }
}
