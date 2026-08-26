//! S · 零信任安全
//!
//! 核心原则：永不信任，始终验证（Never Trust, Always Verify）
//!
//! 核心能力：
//! - 零信任中间件：每次请求都验证身份、权限、设备状态
//! - mTLS 管理：证书签发、轮换、验证、信任链
//! - SPIFFE 身份：SVID 签发、身份验证、信任域管理
//! - 网络策略生成：默认拒绝、最小权限、动态策略

pub mod mtls;
pub mod spiffe;
pub mod network_policy;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub use mtls::{MtlsManager, CertificateInfo, CertificateStatus};
pub use spiffe::{SpiffeIdentity, SpiffeId, SvidInfo, TrustDomain};
pub use network_policy::{NetworkPolicyGenerator, NetworkPolicy, PolicyRule};

/// 零信任策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroTrustPolicy {
    pub require_mtls: bool,
    pub require_spiffe_id: bool,
    pub require_device_health: bool,
    pub require_step_up_auth: bool,
    pub session_timeout_seconds: u64,
    pub max_session_duration_seconds: u64,
    pub allowed_trust_domains: Vec<String>,
    pub risk_based_access: bool,
    pub risk_threshold: f64,
}

impl Default for ZeroTrustPolicy {
    fn default() -> Self {
        Self {
            require_mtls: true,
            require_spiffe_id: true,
            require_device_health: false,
            require_step_up_auth: false,
            session_timeout_seconds: 900,
            max_session_duration_seconds: 28800,
            allowed_trust_domains: vec!["mox.infotopograph.io".to_string()],
            risk_based_access: false,
            risk_threshold: 0.7,
        }
    }
}

/// 会话信息
#[derive(Debug, Clone, Serialize)]
pub struct ZeroTrustSession {
    pub id: String,
    pub subject: String,
    pub spiffe_id: Option<String>,
    pub client_cert_fingerprint: Option<String>,
    pub device_id: Option<String>,
    pub created_at: Instant,
    pub last_activity: Instant,
    pub risk_score: f64,
    pub permissions: Vec<String>,
    pub auth_methods: Vec<String>,
}

/// 认证结果
#[derive(Debug, Clone, Serialize)]
pub struct AuthResult {
    pub authenticated: bool,
    pub subject: Option<String>,
    pub spiffe_id: Option<String>,
    pub risk_score: f64,
    pub permissions: Vec<String>,
    pub auth_methods: Vec<String>,
    pub reason: Option<String>,
    pub step_up_required: bool,
}

/// 零信任中间件
pub struct ZeroTrustMiddleware {
    policy: ZeroTrustPolicy,
    mtls_manager: Arc<MtlsManager>,
    spiffe_identity: Arc<SpiffeIdentity>,
    sessions: DashMap<String, ZeroTrustSession>,
    total_requests: std::sync::atomic::AtomicU64,
    authenticated_requests: std::sync::atomic::AtomicU64,
    denied_requests: std::sync::atomic::AtomicU64,
}

impl ZeroTrustMiddleware {
    /// 创建零信任中间件
    pub fn new(policy: ZeroTrustPolicy, mtls_manager: Arc<MtlsManager>, spiffe_identity: Arc<SpiffeIdentity>) -> Self {
        Self {
            policy,
            mtls_manager,
            spiffe_identity,
            sessions: DashMap::new(),
            total_requests: std::sync::atomic::AtomicU64::new(0),
            authenticated_requests: std::sync::atomic::AtomicU64::new(0),
            denied_requests: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 验证请求
    pub async fn authenticate(&self, request: &ZeroTrustRequest) -> AuthResult {
        self.total_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // 1. mTLS 验证
        if self.policy.require_mtls {
            if let Some(cert_pem) = &request.client_certificate {
                match self.mtls_manager.verify_client_cert(cert_pem).await {
                    Ok(cert_info) => {
                        if cert_info.status != CertificateStatus::Valid {
                            return self.deny("客户端证书无效", None);
                        }
                    }
                    Err(e) => return self.deny(&format!("mTLS 验证失败: {}", e), None),
                }
            } else {
                return self.deny("缺少客户端证书（mTLS 必需）", None);
            }
        }

        // 2. SPIFFE ID 验证
        let mut spiffe_id = None;
        if self.policy.require_spiffe_id {
            if let Some(svid) = &request.svid {
                match self.spiffe_identity.verify_svid(svid).await {
                    Ok(svid_info) => {
                        // 检查信任域
                        if !self.policy.allowed_trust_domains.contains(&svid_info.trust_domain) {
                            return self.deny(&format!("不信任的信任域: {}", svid_info.trust_domain), None);
                        }
                        spiffe_id = Some(svid_info.spiffe_id);
                    }
                    Err(e) => return self.deny(&format!("SVID 验证失败: {}", e), None),
                }
            } else {
                return self.deny("缺少 SPIFFE SVID", None);
            }
        }

        // 3. 设备健康检查
        if self.policy.require_device_health {
            if let Some(device_health) = &request.device_health {
                if !device_health.healthy {
                    return self.deny("设备健康检查未通过", spiffe_id.clone());
                }
            } else {
                return self.deny("缺少设备健康证明", spiffe_id.clone());
            }
        }

        // 4. 风险评估
        let risk_score = self.assess_risk(request);
        if self.policy.risk_based_access && risk_score > self.policy.risk_threshold {
            return AuthResult {
                authenticated: false,
                subject: None,
                spiffe_id: spiffe_id.clone(),
                risk_score,
                permissions: vec![],
                auth_methods: vec![],
                reason: Some(format!("风险分数 {:.2} 超过阈值 {:.2}", risk_score, self.policy.risk_threshold)),
                step_up_required: true,
            };
        }

        // 5. 会话管理
        let subject = spiffe_id.clone().unwrap_or_else(|| request.subject.clone().unwrap_or_else(|| "unknown".to_string()));
        let session = self.get_or_create_session(&subject, spiffe_id.clone(), request);

        // 6. 检查会话超时
        if session.last_activity.elapsed() > Duration::from_secs(self.policy.session_timeout_seconds) {
            self.sessions.remove(&session.id);
            return self.deny("会话已超时", spiffe_id.clone());
        }

        // 7. 检查最大会话时长
        if session.created_at.elapsed() > Duration::from_secs(self.policy.max_session_duration_seconds) {
            self.sessions.remove(&session.id);
            return self.deny("会话已达到最大持续时间", spiffe_id.clone());
        }

        self.authenticated_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        AuthResult {
            authenticated: true,
            subject: Some(subject),
            spiffe_id,
            risk_score,
            permissions: session.permissions.clone(),
            auth_methods: session.auth_methods.clone(),
            reason: None,
            step_up_required: false,
        }
    }

    fn deny(&self, reason: &str, spiffe_id: Option<String>) -> AuthResult {
        self.denied_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        AuthResult {
            authenticated: false,
            subject: None,
            spiffe_id,
            risk_score: 0.0,
            permissions: vec![],
            auth_methods: vec![],
            reason: Some(reason.to_string()),
            step_up_required: false,
        }
    }

    fn assess_risk(&self, request: &ZeroTrustRequest) -> f64 {
        let mut risk = 0.0;

        // 地理位置异常
        if request.geo_anomaly { risk += 0.3; }

        // 异常时间访问
        if request.off_hours_access { risk += 0.15; }

        // 新设备
        if request.new_device { risk += 0.25; }

        // 失败登录历史
        risk += (request.recent_failed_logins.min(5) as f64) * 0.05;

        // 敏感资源访问
        if request.sensitive_resource { risk += 0.2; }

        risk.min(1.0)
    }

    fn get_or_create_session(&self, subject: &str, spiffe_id: Option<String>, request: &ZeroTrustRequest) -> ZeroTrustSession {
        // 查找现有会话
        for session in self.sessions.iter() {
            if session.subject == subject {
                let mut s = session.clone();
                s.last_activity = Instant::now();
                self.sessions.insert(s.id.clone(), s.clone());
                return s;
            }
        }

        // 创建新会话
        let session = ZeroTrustSession {
            id: Uuid::new_v4().to_string(),
            subject: subject.to_string(),
            spiffe_id,
            client_cert_fingerprint: request.client_cert_fingerprint.clone(),
            device_id: request.device_id.clone(),
            created_at: Instant::now(),
            last_activity: Instant::now(),
            risk_score: 0.0,
            permissions: request.permissions.clone(),
            auth_methods: request.auth_methods.clone(),
        };

        self.sessions.insert(session.id.clone(), session.clone());
        session
    }

    /// 撤销会话
    pub fn revoke_session(&self, session_id: &str) -> bool {
        self.sessions.remove(session_id).is_some()
    }

    /// 获取活跃会话数
    pub fn active_sessions(&self) -> usize {
        self.sessions.len()
    }

    /// 获取统计
    pub fn stats(&self) -> ZeroTrustStats {
        ZeroTrustStats {
            policy: self.policy.clone(),
            total_requests: self.total_requests.load(std::sync::atomic::Ordering::Relaxed),
            authenticated_requests: self.authenticated_requests.load(std::sync::atomic::Ordering::Relaxed),
            denied_requests: self.denied_requests.load(std::sync::atomic::Ordering::Relaxed),
            active_sessions: self.sessions.len(),
            auth_rate: if self.total_requests.load(std::sync::atomic::Ordering::Relaxed) > 0 {
                self.authenticated_requests.load(std::sync::atomic::Ordering::Relaxed) as f64
                    / self.total_requests.load(std::sync::atomic::Ordering::Relaxed) as f64 * 100.0
            } else { 100.0 },
        }
    }
}

/// 零信任请求
#[derive(Debug, Clone, Default)]
pub struct ZeroTrustRequest {
    pub subject: Option<String>,
    pub client_certificate: Option<String>,
    pub client_cert_fingerprint: Option<String>,
    pub svid: Option<String>,
    pub device_id: Option<String>,
    pub device_health: Option<DeviceHealth>,
    pub permissions: Vec<String>,
    pub auth_methods: Vec<String>,
    pub geo_anomaly: bool,
    pub off_hours_access: bool,
    pub new_device: bool,
    pub recent_failed_logins: u32,
    pub sensitive_resource: bool,
    pub source_ip: String,
    pub user_agent: String,
}

/// 设备健康
#[derive(Debug, Clone)]
pub struct DeviceHealth {
    pub healthy: bool,
    pub os_version: String,
    pub firewall_enabled: bool,
    pub encryption_enabled: bool,
    pub screen_lock_enabled: bool,
    pub last_scan_at: String,
}

/// 零信任统计
#[derive(Debug, Clone, Serialize)]
pub struct ZeroTrustStats {
    pub policy: ZeroTrustPolicy,
    pub total_requests: u64,
    pub authenticated_requests: u64,
    pub denied_requests: u64,
    pub active_sessions: usize,
    pub auth_rate: f64,
}
