//! SPIFFE 身份管理
//!
//! SPIFFE (Secure Production Identity Framework for Everyone)
//! 为工作负载提供可验证的身份标识
//!
//! 核心能力：
//! - SVID (SPIFFE Verifiable Identity Document) 签发
//! - SVID 验证
//! - 信任域管理
//! - 身份联邦
//! - 工作负载身份映射

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::RwLock;
use uuid::Uuid;

/// SPIFFE ID
/// 格式: spiffe://<trust-domain>/<path>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SpiffeId {
    pub trust_domain: String,
    pub path: String,
}

impl SpiffeId {
    /// 创建 SPIFFE ID
    pub fn new(trust_domain: &str, path: &str) -> Self {
        Self {
            trust_domain: trust_domain.to_string(),
            path: if path.starts_with('/') { path.to_string() } else { format!("/{}", path) },
        }
    }

    /// 解析 SPIFFE ID 字符串
    pub fn parse(spiffe_id: &str) -> Result<Self, String> {
        if !spiffe_id.starts_with("spiffe://") {
            return Err("无效的 SPIFFE ID 格式".to_string());
        }
        let rest = &spiffe_id[9..];
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if parts.len() < 2 {
            return Err("SPIFFE ID 缺少路径".to_string());
        }
        Ok(Self {
            trust_domain: parts[0].to_string(),
            path: format!("/{}", parts[1]),
        })
    }

    /// 转换为字符串
    pub fn to_string(&self) -> String {
        format!("spiffe://{}{}", self.trust_domain, self.path)
    }

    /// 检查是否在指定信任域
    pub fn in_trust_domain(&self, trust_domain: &str) -> bool {
        self.trust_domain == trust_domain
    }
}

impl std::fmt::Display for SpiffeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// 信任域
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustDomain {
    pub name: String,
    pub description: String,
    pub bundle_endpoint: String,
    pub federated_with: Vec<String>,
    pub enabled: bool,
    pub created_at: String,
}

/// SVID 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvidInfo {
    pub spiffe_id: String,
    pub trust_domain: String,
    pub subject: String,
    pub serial_number: String,
    pub not_before: String,
    pub not_after: String,
    pub issued_at: String,
    pub ttl_seconds: u64,
    pub claims: std::collections::HashMap<String, String>,
    pub valid: bool,
}

/// 工作负载注册
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadRegistration {
    pub id: String,
    pub spiffe_id: SpiffeId,
    pub selector: WorkloadSelector,
    pub ttl_seconds: u64,
    pub enabled: bool,
    pub created_at: String,
}

/// 工作负载选择器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadSelector {
    pub kubernetes_namespace: Option<String>,
    pub kubernetes_service_account: Option<String>,
    pub kubernetes_pod_label: Option<std::collections::HashMap<String, String>>,
    pub unix_uid: Option<String>,
    pub unix_gid: Option<String>,
    pub process_name: Option<String>,
    pub custom_attributes: Option<std::collections::HashMap<String, String>>,
}

/// SPIFFE 身份管理器
pub struct SpiffeIdentity {
    trust_domains: RwLock<Vec<TrustDomain>>,
    workload_registrations: RwLock<Vec<WorkloadRegistration>>,
    issued_svids: RwLock<Vec<SvidInfo>>,
    revoked_serials: RwLock<HashSet<String>>,
    default_ttl_seconds: RwLock<u64>,
    total_svids_issued: std::sync::atomic::AtomicU64,
    total_svids_verified: std::sync::atomic::AtomicU64,
}

impl SpiffeIdentity {
    /// 创建 SPIFFE 身份管理器
    pub fn new(default_trust_domain: &str) -> Self {
        let trust_domains = vec![TrustDomain {
            name: default_trust_domain.to_string(),
            description: "MOX Enterprise 默认信任域".to_string(),
            bundle_endpoint: format!("https://spire.{}/bundle", default_trust_domain),
            federated_with: vec![],
            enabled: true,
            created_at: chrono::Utc::now().to_rfc3339(),
        }];

        Self {
            trust_domains: RwLock::new(trust_domains),
            workload_registrations: RwLock::new(Vec::new()),
            issued_svids: RwLock::new(Vec::new()),
            revoked_serials: RwLock::new(HashSet::new()),
            default_ttl_seconds: RwLock::new(3600),
            total_svids_issued: std::sync::atomic::AtomicU64::new(0),
            total_svids_verified: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 注册工作负载
    pub fn register_workload(&self, spiffe_id: SpiffeId, selector: WorkloadSelector, ttl_seconds: Option<u64>) -> WorkloadRegistration {
        let registration = WorkloadRegistration {
            id: Uuid::new_v4().to_string(),
            spiffe_id,
            selector,
            ttl_seconds: ttl_seconds.unwrap_or(*self.default_ttl_seconds.read().unwrap()),
            enabled: true,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.workload_registrations.write().unwrap().push(registration.clone());
        registration
    }

    /// 签发 SVID
    pub async fn issue_svid(&self, spiffe_id: &SpiffeId, claims: Option<std::collections::HashMap<String, String>>) -> Result<(String, SvidInfo), String> {
        // 检查信任域
        if !self.is_trust_domain_enabled(&spiffe_id.trust_domain) {
            return Err(format!("信任域未启用: {}", spiffe_id.trust_domain));
        }

        self.total_svids_issued.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let now = chrono::Utc::now();
        let ttl = *self.default_ttl_seconds.read().unwrap();
        let not_after = now + chrono::Duration::seconds(ttl as i64);
        let serial = Uuid::new_v4().to_string();

        let svid_info = SvidInfo {
            spiffe_id: spiffe_id.to_string(),
            trust_domain: spiffe_id.trust_domain.clone(),
            subject: spiffe_id.path.clone(),
            serial_number: serial.clone(),
            not_before: now.to_rfc3339(),
            not_after: not_after.to_rfc3339(),
            issued_at: now.to_rfc3339(),
            ttl_seconds: ttl,
            claims: claims.unwrap_or_default(),
            valid: true,
        };

        self.issued_svids.write().unwrap().push(svid_info.clone());

        Ok(("SVID_JWT_PLACEHOLDER".to_string(), svid_info))
    }

    /// 验证 SVID
    pub async fn verify_svid(&self, svid_token: &str) -> Result<SvidInfo, String> {
        self.total_svids_verified.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // 实际应验证 JWT 签名和过期时间
        // 这里简化处理
        let now = chrono::Utc::now();

        // 查找已签发的 SVID
        let svids = self.issued_svids.read().unwrap();
        let svid = svids.iter()
            .find(|s| s.serial_number == svid_token || s.spiffe_id == svid_token)
            .cloned();

        match svid {
            Some(mut info) => {
                // 检查是否被吊销
                if self.revoked_serials.read().unwrap().contains(&info.serial_number) {
                    info.valid = false;
                    return Err("SVID 已被吊销".to_string());
                }

                // 检查过期
                let not_after = chrono::DateTime::parse_from_rfc3339(&info.not_after)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or(now);
                if now > not_after {
                    info.valid = false;
                    return Err("SVID 已过期".to_string());
                }

                // 检查信任域
                if !self.is_trust_domain_enabled(&info.trust_domain) {
                    return Err(format!("不信任的信任域: {}", info.trust_domain));
                }

                Ok(info)
            }
            None => Err("SVID 不存在或无效".to_string()),
        }
    }

    /// 吊销 SVID
    pub fn revoke_svid(&self, serial_number: &str) -> bool {
        let inserted = self.revoked_serials.write().unwrap().insert(serial_number.to_string());
        if inserted {
            // 更新 SVID 状态
            if let Some(mut svids) = self.issued_svids.write().ok() {
                for svid in svids.iter_mut() {
                    if svid.serial_number == serial_number {
                        svid.valid = false;
                    }
                }
            }
        }
        inserted
    }

    /// 添加信任域
    pub fn add_trust_domain(&self, domain: TrustDomain) {
        self.trust_domains.write().unwrap().push(domain);
    }

    /// 启用/禁用信任域
    pub fn set_trust_domain_enabled(&self, name: &str, enabled: bool) -> bool {
        if let Some(domains) = self.trust_domains.write().as_mut() {
            if let Some(domain) = domains.iter_mut().find(|d| d.name == name) {
                domain.enabled = enabled;
                return true;
            }
        }
        false
    }

    /// 检查信任域是否启用
    pub fn is_trust_domain_enabled(&self, name: &str) -> bool {
        self.trust_domains.read().unwrap()
            .iter()
            .any(|d| d.name == name && d.enabled)
    }

    /// 设置默认 TTL
    pub fn set_default_ttl(&self, ttl_seconds: u64) {
        *self.default_ttl_seconds.write().unwrap() = ttl_seconds;
    }

    /// 获取工作负载的 SPIFFE ID
    pub fn get_workload_spiffe_id(&self, selector: &WorkloadSelector) -> Option<SpiffeId> {
        self.workload_registrations.read().unwrap()
            .iter()
            .find(|r| r.enabled && Self::selector_matches(&r.selector, selector))
            .map(|r| r.spiffe_id.clone())
    }

    fn selector_matches(registered: &WorkloadSelector, provided: &WorkloadSelector) -> bool {
        if let Some(ns) = &registered.kubernetes_namespace {
            if Some(ns) != provided.kubernetes_namespace.as_ref() { return false; }
        }
        if let Some(sa) = &registered.kubernetes_service_account {
            if Some(sa) != provided.kubernetes_service_account.as_ref() { return false; }
        }
        true
    }

    /// 获取统计
    pub fn stats(&self) -> SpiffeStats {
        SpiffeStats {
            trust_domains: self.trust_domains.read().unwrap().len(),
            enabled_trust_domains: self.trust_domains.read().unwrap().iter().filter(|d| d.enabled).count(),
            workload_registrations: self.workload_registrations.read().unwrap().len(),
            total_svids_issued: self.total_svids_issued.load(std::sync::atomic::Ordering::Relaxed),
            total_svids_verified: self.total_svids_verified.load(std::sync::atomic::Ordering::Relaxed),
            active_svids: self.issued_svids.read().unwrap().iter().filter(|s| s.valid).count(),
            revoked_svids: self.revoked_serials.read().unwrap().len(),
            default_ttl_seconds: *self.default_ttl_seconds.read().unwrap(),
        }
    }
}

/// SPIFFE 统计
#[derive(Debug, Clone, Serialize)]
pub struct SpiffeStats {
    pub trust_domains: usize,
    pub enabled_trust_domains: usize,
    pub workload_registrations: usize,
    pub total_svids_issued: u64,
    pub total_svids_verified: u64,
    pub active_svids: usize,
    pub revoked_svids: usize,
    pub default_ttl_seconds: u64,
}
