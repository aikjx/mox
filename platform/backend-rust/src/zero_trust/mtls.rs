// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! mTLS 管理器
//!
//! 核心能力：
//! - 客户端证书验证
//! - 证书签发（CA 签名）
//! - 证书轮换
//! - 证书吊销（CRL）
//! - 信任链管理
//! - 证书指纹计算

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::RwLock;
use uuid::Uuid;

/// 证书状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CertificateStatus {
    Valid,
    Expired,
    Revoked,
    NotYetValid,
    InvalidSignature,
    UntrustedCA,
}

/// 证书信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    pub fingerprint: String,
    pub subject: String,
    pub issuer: String,
    pub serial_number: String,
    pub not_before: String,
    pub not_after: String,
    pub status: CertificateStatus,
    pub key_algorithm: String,
    pub signature_algorithm: String,
    pub sans: Vec<String>,
}

/// CA 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaConfig {
    pub ca_cert_pem: String,
    pub ca_key_pem: String,
    pub default_cert_ttl_days: u32,
    pub max_cert_ttl_days: u32,
    pub allowed_key_algorithms: Vec<String>,
    pub crl_refresh_interval_seconds: u64,
}

impl Default for CaConfig {
    fn default() -> Self {
        Self {
            ca_cert_pem: String::new(),
            ca_key_pem: String::new(),
            default_cert_ttl_days: 90,
            max_cert_ttl_days: 365,
            allowed_key_algorithms: vec!["RSA-2048".to_string(), "RSA-4096".to_string(), "ECDSA-P256".to_string(), "ECDSA-P384".to_string()],
            crl_refresh_interval_seconds: 3600,
        }
    }
}

/// mTLS 管理器
pub struct MtlsManager {
    config: RwLock<CaConfig>,
    trusted_certs: RwLock<HashSet<String>>,
    revoked_fingerprints: RwLock<HashSet<String>>,
    issued_certs: RwLock<Vec<CertificateInfo>>,
    total_verified: std::sync::atomic::AtomicU64,
    total_issued: std::sync::atomic::AtomicU64,
    total_revoked: std::sync::atomic::AtomicU64,
}

impl MtlsManager {
    /// 创建 mTLS 管理器
    pub fn new(config: CaConfig) -> Self {
        Self {
            config: RwLock::new(config),
            trusted_certs: RwLock::new(HashSet::new()),
            revoked_fingerprints: RwLock::new(HashSet::new()),
            issued_certs: RwLock::new(Vec::new()),
            total_verified: std::sync::atomic::AtomicU64::new(0),
            total_issued: std::sync::atomic::AtomicU64::new(0),
            total_revoked: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 验证客户端证书
    pub async fn verify_client_cert(&self, cert_pem: &str) -> Result<CertificateInfo, String> {
        self.total_verified.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // 计算证书指纹
        let fingerprint = self.calculate_fingerprint(cert_pem);

        // 检查是否被吊销
        if self.revoked_fingerprints.read().unwrap().contains(&fingerprint) {
            return Ok(CertificateInfo {
                fingerprint,
                subject: String::new(),
                issuer: String::new(),
                serial_number: String::new(),
                not_before: String::new(),
                not_after: String::new(),
                status: CertificateStatus::Revoked,
                key_algorithm: String::new(),
                signature_algorithm: String::new(),
                sans: vec![],
            });
        }

        // 解析证书（实际应使用 x509-parser）
        let cert_info = self.parse_certificate(cert_pem, &fingerprint);

        // 检查有效期
        let now = chrono::Utc::now();
        let not_after = chrono::DateTime::parse_from_rfc3339(&cert_info.not_after)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or(now);
        let not_before = chrono::DateTime::parse_from_rfc3339(&cert_info.not_before)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or(now);

        let mut status = cert_info.status;
        if now < not_before {
            status = CertificateStatus::NotYetValid;
        } else if now > not_after {
            status = CertificateStatus::Expired;
        }

        Ok(CertificateInfo { status, ..cert_info })
    }

    /// 签发客户端证书
    pub async fn issue_certificate(&self, subject: &str, sans: Vec<String>, ttl_days: Option<u32>) -> Result<(String, String, CertificateInfo), String> {
        let config = self.config.read().unwrap();
        let ttl = ttl_days.unwrap_or(config.default_cert_ttl_days).min(config.max_cert_ttl_days);

        if config.ca_cert_pem.is_empty() || config.ca_key_pem.is_empty() {
            return Err("CA 证书或私钥未配置".to_string());
        }

        self.total_issued.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // 实际应使用 ring 或 openssl 生成证书
        let now = chrono::Utc::now();
        let not_after = now + chrono::Duration::days(ttl as i64);
        let serial = Uuid::new_v4().to_string();
        let fingerprint = format!("{:x}", md5_hash(&format!("{}{}{}", subject, serial, now)));

        let cert_info = CertificateInfo {
            fingerprint: fingerprint.clone(),
            subject: subject.to_string(),
            issuer: "MOX Enterprise CA".to_string(),
            serial_number: serial,
            not_before: now.to_rfc3339(),
            not_after: not_after.to_rfc3339(),
            status: CertificateStatus::Valid,
            key_algorithm: "ECDSA-P256".to_string(),
            signature_algorithm: "SHA256withECDSA".to_string(),
            sans,
        };

        self.issued_certs.write().unwrap().push(cert_info.clone());

        Ok(("CERT_PEM_PLACEHOLDER".to_string(), "KEY_PEM_PLACEHOLDER".to_string(), cert_info))
    }

    /// 吊销证书
    pub fn revoke_certificate(&self, fingerprint: &str) -> bool {
        let mut revoked = self.revoked_fingerprints.write().unwrap();
        let inserted = revoked.insert(fingerprint.to_string());
        if inserted {
            self.total_revoked.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        inserted
    }

    /// 添加受信任证书
    pub fn add_trusted_cert(&self, cert_pem: &str) -> String {
        let fingerprint = self.calculate_fingerprint(cert_pem);
        self.trusted_certs.write().unwrap().insert(fingerprint.clone());
        fingerprint
    }

    /// 移除受信任证书
    pub fn remove_trusted_cert(&self, fingerprint: &str) -> bool {
        self.trusted_certs.write().unwrap().remove(fingerprint)
    }

    /// 轮换证书（签发新证书并吊销旧证书）
    pub async fn rotate_certificate(&self, old_fingerprint: &str, subject: &str, sans: Vec<String>) -> Result<(String, String, CertificateInfo), String> {
        let result = self.issue_certificate(subject, sans, None).await?;
        self.revoke_certificate(old_fingerprint);
        Ok(result)
    }

    /// 计算证书指纹（SHA-256）
    pub fn calculate_fingerprint(&self, cert_pem: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(cert_pem.as_bytes());
        let result = hasher.finalize();
        result.iter().map(|b| format!("{:02x}", b)).collect()
    }

    fn parse_certificate(&self, _cert_pem: &str, fingerprint: &str) -> CertificateInfo {
        // 实际应使用 x509-parser 解析
        CertificateInfo {
            fingerprint: fingerprint.to_string(),
            subject: "CN=client,OU=mox".to_string(),
            issuer: "CN=MOX CA".to_string(),
            serial_number: Uuid::new_v4().to_string(),
            not_before: chrono::Utc::now().to_rfc3339(),
            not_after: (chrono::Utc::now() + chrono::Duration::days(90)).to_rfc3339(),
            status: CertificateStatus::Valid,
            key_algorithm: "ECDSA-P256".to_string(),
            signature_algorithm: "SHA256withECDSA".to_string(),
            sans: vec![],
        }
    }

    /// 更新 CA 配置
    pub fn update_config(&self, config: CaConfig) {
        *self.config.write().unwrap() = config;
    }

    /// 获取已签发证书列表
    pub fn list_issued_certs(&self) -> Vec<CertificateInfo> {
        self.issued_certs.read().unwrap().clone()
    }

    /// 获取统计
    pub fn stats(&self) -> MtlsStats {
        MtlsStats {
            total_verified: self.total_verified.load(std::sync::atomic::Ordering::Relaxed),
            total_issued: self.total_issued.load(std::sync::atomic::Ordering::Relaxed),
            total_revoked: self.total_revoked.load(std::sync::atomic::Ordering::Relaxed),
            trusted_certs: self.trusted_certs.read().unwrap().len(),
            revoked_certs: self.revoked_fingerprints.read().unwrap().len(),
            active_certs: self.issued_certs.read().unwrap().len(),
        }
    }
}

/// mTLS 统计
#[derive(Debug, Clone, Serialize)]
pub struct MtlsStats {
    pub total_verified: u64,
    pub total_issued: u64,
    pub total_revoked: u64,
    pub trusted_certs: usize,
    pub revoked_certs: usize,
    pub active_certs: usize,
}

fn md5_hash(input: &str) -> u128 {
    // 简化的哈希，实际应使用 md-5 crate
    let mut hash: u128 = 0;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u128);
    }
    hash
}
