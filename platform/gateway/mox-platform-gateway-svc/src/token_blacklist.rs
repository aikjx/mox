// ====================================================================
// token_blacklist.rs — 令牌吊销（黑名单）+ 活跃会话注册表（P0-2）
// ====================================================================
// JWT 本身无状态，8h 有效期内无法作废。本模块在网关进程内维护：
// - revoked: jti -> 过期时间戳(sec)，登出/踢人时写入；验签命中即拒。
// - sessions: jti -> 会话记录（sub/exp），鉴权成功时登记，供管理员列出/吊销。
// 条目 TTL 对齐令牌过期时间，读取时惰性清扫，避免无限增长。
//
// 注：单二进制网关进程内有效；多副本部署需换共享存储（Redis），属后续工程。

use parking_lot::RwLock;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SessionRec {
    pub jti: String,
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Default)]
pub struct TokenBlacklist {
    revoked: RwLock<HashMap<String, i64>>,
    sessions: RwLock<HashMap<String, SessionRec>>,
}

impl TokenBlacklist {
    pub fn new() -> Self {
        Self::default()
    }

    fn now() -> i64 {
        chrono::Utc::now().timestamp()
    }

    /// 惰性清扫：删除所有已过期条目（revoked 与 sessions 同步）。
    fn purge(&self) {
        let now = Self::now();
        self.revoked.write().retain(|_, exp| *exp > now);
        self.sessions.write().retain(|_, rec| rec.exp > now);
    }

    /// 登记一次成功鉴权的会话（middleware 调用）。
    pub fn record(&self, jti: &str, sub: &str, exp: i64) {
        if jti.is_empty() {
            return;
        }
        self.purge();
        self.sessions.write().insert(
            jti.to_string(),
            SessionRec { jti: jti.to_string(), sub: sub.to_string(), exp, iat: Self::now() },
        );
    }

    /// 吊销指定 jti（登出/踢人）。exp 决定黑名单留存时长。
    pub fn revoke(&self, jti: &str, exp: i64) {
        if jti.is_empty() {
            return;
        }
        self.revoked.write().insert(jti.to_string(), exp);
        self.sessions.write().remove(jti);
    }

    /// 是否已被吊销（已过期的自动视为未吊销，便于回收）。
    pub fn is_revoked(&self, jti: &str) -> bool {
        if jti.is_empty() {
            return false;
        }
        self.purge();
        self.revoked.read().contains_key(jti)
    }

    /// 列出某用户的活跃会话（未过期）。
    pub fn list_user_sessions(&self, sub: &str) -> Vec<SessionRec> {
        self.purge();
        self.sessions
            .read()
            .values()
            .filter(|r| r.sub == sub)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revoke_then_reject() {
        let bl = TokenBlacklist::new();
        bl.record("jti-1", "alice", 9_999_999_999);
        assert!(!bl.is_revoked("jti-1"));
        bl.revoke("jti-1", 9_999_999_999);
        assert!(bl.is_revoked("jti-1"));
    }

    #[test]
    fn expired_revoked_entry_auto_purged() {
        let bl = TokenBlacklist::new();
        // 过期时间设在过去 → 视同未吊销
        bl.revoke("jti-old", 1);
        assert!(!bl.is_revoked("jti-old"), "过期条目应被惰性清扫");
    }

    #[test]
    fn admin_lists_and_kicks_user_sessions() {
        let bl = TokenBlacklist::new();
        bl.record("a1", "bob", 9_999_999_999);
        bl.record("a2", "bob", 9_999_999_999);
        bl.record("c1", "carol", 9_999_999_999);
        let bob = bl.list_user_sessions("bob");
        assert_eq!(bob.len(), 2);
        bl.revoke("a1", 9_999_999_999);
        let bob2 = bl.list_user_sessions("bob");
        assert_eq!(bob2.len(), 1, "被踢会话应从活跃列表移除");
        assert!(bl.is_revoked("a1"));
    }
}
