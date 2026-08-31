// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 审计哈希链
//!
//! 基于 SHA-256 的不可篡改审计链，用于内部自验证。
//!
//! 与原 `govern::AuditChain` 的区别：
//! - 哈希算法从 DefaultHasher（64 位，不可跨平台）升级为 SHA-256（256 位，标准加密级）
//! - 事件类型从简化的 `govern::AuditEvent` 升级为统一的 `AuditEvent`
//! - 支持 `append` / `verify` / `latest_hash` 标准接口
//! - `prev_hash` 命名统一（原 `govern::AuditEvent.hash` → `prev_hash` + `content_hash` 双字段）

use crate::error::AuditError;
use crate::event::AuditEvent;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 审计链起始哈希常量
pub const GENESIS_HASH: &str = "GENESIS";

/// 不可篡改审计链（SHA-256 哈希链）
///
/// 每个事件的 `prev_hash` 指向前一事件的 `content_hash`，
/// 形成链式结构。任意中间事件被篡改，后续所有哈希都会断裂，
/// `verify()` 可检测到。
///
/// 用途：
/// - 内存中自验证：进程内审计事件的防篡改检测
/// - 导出到外部 Sink 前：确保链式完整性
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditChain {
    events: Vec<AuditEvent>,
}

impl AuditChain {
    /// 创建空的审计链
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一个审计事件到链尾
    ///
    /// 自动计算 prev_hash（取链尾事件的 content_hash，空链则用 "GENESIS"），
    /// 并重新计算事件的 content_hash（因为 prev_hash 会影响内容）。
    ///
    /// 返回追加后的事件引用。
    pub fn append(&mut self, mut event: AuditEvent) -> &AuditEvent {
        let prev_hash = self
            .events
            .last()
            .map(|e| e.content_hash.clone())
            .unwrap_or_else(|| GENESIS_HASH.to_string());

        event.prev_hash = prev_hash;
        event.recompute_hash();

        self.events.push(event);
        self.events.last().unwrap()
    }

    /// 返回链上最新一个事件的哈希（作为下一个事件的 prev_hash 基准）。
    /// 空链返回 None（调用方应以 "GENESIS" 作为起点）。
    pub fn latest_hash(&self) -> Option<&str> {
        self.events.last().map(|e| e.content_hash.as_str())
    }

    /// 事件数量
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// 获取所有事件（只读）
    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    /// 校验链完整性（防篡改）
    ///
    /// 逐事件验证：
    /// 1. 每个事件的 prev_hash 等于前一事件的 content_hash
    /// 2. 每个事件的 content_hash 等于重新计算的哈希值
    ///
    /// 返回 `Ok(())` 表示完整，`Err(AuditError::ChainInconsistency)` 表示被篡改。
    pub fn verify(&self) -> Result<(), AuditError> {
        let mut prev = GENESIS_HASH.to_string();

        for (i, event) in self.events.iter().enumerate() {
            // 验证 prev_hash 链接
            if event.prev_hash != prev {
                return Err(AuditError::ChainInconsistency(format!(
                    "事件 #{} 的 prev_hash 不匹配：期望 '{}'，实际 '{}'",
                    i, prev, event.prev_hash
                )));
            }

            // 验证 content_hash 完整性
            let re = event.clone();
            let expected = re.compute_content_hash();
            if event.content_hash != expected {
                return Err(AuditError::ChainInconsistency(format!(
                    "事件 #{} (id={}) 的 content_hash 不匹配：期望 '{}'，实际 '{}'",
                    i, event.event_id, expected, event.content_hash
                )));
            }

            prev = event.content_hash.clone();
        }

        Ok(())
    }

    /// 计算链式哈希（prev_hash + content_hash → 链哈希）
    ///
    /// 用于需要更强链接保证的场景。标准 verify 已足够，
    /// 此方法供扩展使用。
    pub fn compute_chain_hash(prev_hash: &str, content_hash: &str) -> String {
        let mut h = Sha256::new();
        h.update(prev_hash.as_bytes());
        h.update(content_hash.as_bytes());
        hex::encode(h.finalize())
    }
}

// =============================================================================
// 单元测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{
        test_event, AuditAction, AuditActor, AuditOutcome, AuditResource, AuditSeverity,
    };

    fn make_event(i: u32) -> AuditEvent {
        AuditEvent::new(
            AuditActor::system(),
            AuditAction::FlowCreated,
            AuditResource::flow(&format!("flow-{i}"), "test-tenant"),
            AuditOutcome::Success,
            AuditSeverity::Info,
            "test-tenant".into(),
        )
    }

    #[test]
    fn empty_chain_is_empty() {
        let c = AuditChain::new();
        assert!(c.is_empty());
        assert_eq!(c.len(), 0);
        assert!(c.latest_hash().is_none());
    }

    #[test]
    fn first_event_prev_hash_is_genesis() {
        let mut c = AuditChain::new();
        let ev = make_event(1);
        c.append(ev);
        assert_eq!(c.len(), 1);
        assert_eq!(c.events()[0].prev_hash, GENESIS_HASH);
    }

    #[test]
    fn chain_links_prev_hash() {
        let mut c = AuditChain::new();
        c.append(make_event(1));
        c.append(make_event(2));
        c.append(make_event(3));

        assert_eq!(c.len(), 3);

        // 每个事件的 prev_hash = 前一事件的 content_hash
        assert_eq!(c.events()[0].prev_hash, GENESIS_HASH);
        assert_eq!(c.events()[1].prev_hash, c.events()[0].content_hash);
        assert_eq!(c.events()[2].prev_hash, c.events()[1].content_hash);
    }

    #[test]
    fn verify_intact_chain() {
        let mut c = AuditChain::new();
        c.append(make_event(1));
        c.append(make_event(2));
        c.append(make_event(3));
        assert!(c.verify().is_ok(), "完整链应通过验证");
    }

    #[test]
    fn tamper_detected_middle_event() {
        let mut c = AuditChain::new();
        c.append(make_event(1));
        c.append(make_event(2));
        c.append(make_event(3));

        // 篡改中间事件的 action
        c.events[1].action = AuditAction::FlowDeleted;

        let result = c.verify();
        assert!(result.is_err(), "篡改后应验证失败");
        let err = result.unwrap_err();
        assert!(matches!(err, AuditError::ChainInconsistency(_)));
        assert!(err.to_string().contains("content_hash"), "应检测到 content_hash 不一致");
    }

    #[test]
    fn tamper_detected_prev_hash_link() {
        let mut c = AuditChain::new();
        c.append(make_event(1));
        c.append(make_event(2));
        c.append(make_event(3));

        // 直接修改中间事件的 prev_hash（破坏链接）
        c.events[1].prev_hash = "FAKE_HASH".into();

        let result = c.verify();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("prev_hash"));
    }

    #[test]
    fn latest_hash_returns_last_content_hash() {
        let mut c = AuditChain::new();
        c.append(make_event(1));
        let last = c.append(make_event(2)).clone();
        assert_eq!(c.latest_hash(), Some(last.content_hash.as_str()));
    }

    #[test]
    fn verify_after_signature() {
        // 签名不影响链完整性
        let mut c = AuditChain::new();
        let ev = test_event().sign("chain-secret");
        c.append(ev);
        assert!(c.verify().is_ok());
    }

    #[test]
    fn compute_chain_hash_is_deterministic() {
        let h1 = AuditChain::compute_chain_hash("a", "b");
        let h2 = AuditChain::compute_chain_hash("a", "b");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn chain_serialization_roundtrip() {
        let mut c = AuditChain::new();
        c.append(make_event(1));
        c.append(make_event(2));

        let json = serde_json::to_string(&c).unwrap();
        let parsed: AuditChain = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.len(), c.len());
        assert!(parsed.verify().is_ok());
    }

    #[test]
    fn genesis_hash_constant() {
        assert_eq!(GENESIS_HASH, "GENESIS");
    }
}
