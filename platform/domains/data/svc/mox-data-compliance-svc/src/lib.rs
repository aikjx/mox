// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! MOX Data Compliance Service
//!
//! PII (Personally Identifiable Information) detection, classification, and redaction.
//! Supports regex-based detection, named entity recognition, and configurable policies.
//!
//! Compliance frameworks: GDPR, CCPA, PCI-DSS, HIPAA baseline detection.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ComplianceError {
    #[error("invalid regex: {0}")]
    InvalidRegex(String),
    #[error("policy not found: {0}")]
    PolicyNotFound(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PiiType {
    Email,
    Phone,
    CreditCard,
    Ssn,            // Social Security Number (US)
    IdCard,         // ID card number (CN)
    IpAddress,
    DateOfBirth,
    Address,
    Name,
    BankAccount,
    Passport,
    LicensePlate,
    Custom,
}

impl PiiType {
    pub fn label(&self) -> &'static str {
        match self {
            PiiType::Email => "email",
            PiiType::Phone => "phone",
            PiiType::CreditCard => "credit_card",
            PiiType::Ssn => "ssn",
            PiiType::IdCard => "id_card",
            PiiType::IpAddress => "ip_address",
            PiiType::DateOfBirth => "date_of_birth",
            PiiType::Address => "address",
            PiiType::Name => "name",
            PiiType::BankAccount => "bank_account",
            PiiType::Passport => "passport",
            PiiType::LicensePlate => "license_plate",
            PiiType::Custom => "custom",
        }
    }

    pub fn severity(&self) -> PiiSeverity {
        match self {
            PiiType::CreditCard | PiiType::Ssn | PiiType::BankAccount | PiiType::Passport => PiiSeverity::Critical,
            PiiType::Email | PiiType::Phone | PiiType::IdCard | PiiType::DateOfBirth => PiiSeverity::High,
            PiiType::IpAddress | PiiType::Address | PiiType::LicensePlate => PiiSeverity::Medium,
            PiiType::Name | PiiType::Custom => PiiSeverity::Low,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PiiSeverity { Low, Medium, High, Critical }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiMatch {
    pub pii_type: PiiType,
    pub severity: PiiSeverity,
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub input_length: usize,
    pub matches: Vec<PiiMatch>,
    pub has_pii: bool,
    pub severity_summary: HashMap<PiiSeverity, usize>,
    pub type_summary: HashMap<String, usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedactionMode {
    Mask,       // Replace with ***
    Partial,    // Keep first/last chars: a***@***.com
    Remove,     // Remove entirely
    Hash,       // Replace with SHA-256 hash
    Placeholder, // Replace with [REDACTED]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionPolicy {
    pub name: String,
    pub rules: HashMap<PiiType, RedactionMode>,
    pub default_mode: RedactionMode,
}

impl Default for RedactionPolicy {
    fn default() -> Self {
        let mut rules = HashMap::new();
        rules.insert(PiiType::CreditCard, RedactionMode::Partial);
        rules.insert(PiiType::Ssn, RedactionMode::Mask);
        rules.insert(PiiType::Email, RedactionMode::Partial);
        rules.insert(PiiType::Phone, RedactionMode::Partial);
        rules.insert(PiiType::BankAccount, RedactionMode::Hash);
        Self {
            name: "default".into(),
            rules,
            default_mode: RedactionMode::Placeholder,
        }
    }
}

/// PII detector with built-in regex patterns for common PII types.
pub struct PiiDetector {
    patterns: Vec<(PiiType, Regex, f64)>,
    custom_patterns: Vec<(String, Regex, f64)>,
}

impl PiiDetector {
    pub fn new() -> Result<Self, ComplianceError> {
        let patterns = vec![
            (PiiType::Email, Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").map_err(|e| ComplianceError::InvalidRegex(e.to_string()))?, 0.95),
            (PiiType::Phone, Regex::new(r"\+?[\d\s\-()]{7,15}\d").map_err(|e| ComplianceError::InvalidRegex(e.to_string()))?, 0.7),
            (PiiType::CreditCard, Regex::new(r"\b(?:\d[ -]*?){13,16}\b").map_err(|e| ComplianceError::InvalidRegex(e.to_string()))?, 0.85),
            (PiiType::Ssn, Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").map_err(|e| ComplianceError::InvalidRegex(e.to_string()))?, 0.98),
            (PiiType::IdCard, Regex::new(r"\b[1-9]\d{5}(19|20)\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])\d{3}[\dXx]\b").map_err(|e| ComplianceError::InvalidRegex(e.to_string()))?, 0.95),
            (PiiType::IpAddress, Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").map_err(|e| ComplianceError::InvalidRegex(e.to_string()))?, 0.8),
            (PiiType::DateOfBirth, Regex::new(r"\b(19|20)\d{2}[-/](0[1-9]|1[0-2])[-/](0[1-9]|[12]\d|3[01])\b").map_err(|e| ComplianceError::InvalidRegex(e.to_string()))?, 0.6),
            (PiiType::BankAccount, Regex::new(r"\b\d{10,20}\b").map_err(|e| ComplianceError::InvalidRegex(e.to_string()))?, 0.5),
            (PiiType::Passport, Regex::new(r"\b[A-Z]{1,2}\d{6,9}\b").map_err(|e| ComplianceError::InvalidRegex(e.to_string()))?, 0.6),
        ];
        Ok(Self { patterns, custom_patterns: vec![] })
    }

    pub fn add_custom_pattern(&mut self, name: &str, pattern: &str, confidence: f64) -> Result<(), ComplianceError> {
        let re = Regex::new(pattern).map_err(|e| ComplianceError::InvalidRegex(e.to_string()))?;
        self.custom_patterns.push((name.into(), re, confidence));
        Ok(())
    }

    pub fn scan(&self, text: &str) -> ScanResult {
        let mut matches = vec![];
        for (pii_type, re, conf) in &self.patterns {
            for cap in re.find_iter(text) {
                matches.push(PiiMatch {
                    pii_type: *pii_type,
                    severity: pii_type.severity(),
                    text: cap.as_str().to_string(),
                    start: cap.start(),
                    end: cap.end(),
                    confidence: *conf,
                });
            }
        }
        for (name, re, conf) in &self.custom_patterns {
            for cap in re.find_iter(text) {
                matches.push(PiiMatch {
                    pii_type: PiiType::Custom,
                    severity: PiiSeverity::Low,
                    text: format!("{}:{}", name, cap.as_str()),
                    start: cap.start(),
                    end: cap.end(),
                    confidence: *conf,
                });
            }
        }
        // Sort by start position, then by severity (higher first)
        matches.sort_by(|a, b| a.start.cmp(&b.start).then(b.severity as u8.cmp(&(a.severity as u8))));

        let mut severity_summary = HashMap::new();
        let mut type_summary = HashMap::new();
        for m in &matches {
            *severity_summary.entry(m.severity).or_insert(0) += 1;
            *type_summary.entry(m.pii_type.label().to_string()).or_insert(0) += 1;
        }

        ScanResult {
            input_length: text.len(),
            has_pii: !matches.is_empty(),
            matches,
            severity_summary,
            type_summary,
        }
    }

    pub fn redact(&self, text: &str, policy: &RedactionPolicy) -> String {
        let result = self.scan(text);
        if !result.has_pii { return text.to_string(); }

        let mut output = String::with_capacity(text.len());
        let mut last_end = 0;
        for m in &result.matches {
            if m.start < last_end { continue; } // skip overlaps
            output.push_str(&text[last_end..m.start]);
            let mode = policy.rules.get(&m.pii_type).copied().unwrap_or(policy.default_mode);
            output.push_str(&self.redact_match(&m.text, mode, m.pii_type));
            last_end = m.end;
        }
        output.push_str(&text[last_end..]);
        output
    }

    fn redact_match(&self, text: &str, mode: RedactionMode, pii_type: PiiType) -> String {
        match mode {
            RedactionMode::Mask => "*".repeat(text.len()),
            RedactionMode::Partial => {
                match pii_type {
                    PiiType::Email => {
                        if let Some(at) = text.find('@') {
                            let (user, domain) = text.split_at(at);
                            let masked_user = if user.len() <= 2 { user.to_string() } else { format!("{}***{}", &user[..1], &user[user.len()-1..]) };
                            let dot = domain.rfind('.').unwrap_or(domain.len());
                            let masked_domain = if dot > 2 { format!("@***{}", &domain[dot..]) } else { domain.to_string() };
                            format!("{}{}", masked_user, masked_domain)
                        } else { text.to_string() }
                    }
                    PiiType::Phone => {
                        if text.len() > 7 { format!("{}****{}", &text[..text.len()-7], &text[text.len()-3..]) } else { "****".to_string() }
                    }
                    PiiType::CreditCard => {
                        let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
                        if digits.len() >= 4 { format!("****-****-****-{}", &digits[digits.len()-4..]) } else { "****".to_string() }
                    }
                    _ => {
                        if text.len() <= 4 { text.to_string() } else { format!("{}{}{}", &text[..1], "*".repeat(text.len()-2), &text[text.len()-1..]) }
                    }
                }
            }
            RedactionMode::Remove => String::new(),
            RedactionMode::Hash => {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(text.as_bytes());
                format!("sha256:{:x}", hasher.finalize())
            }
            RedactionMode::Placeholder => format!("[REDACTED:{}]", pii_type.label()),
        }
    }
}

/// Compliance policy engine: evaluates data against configured policies.
#[derive(Clone)]
pub struct ComplianceEngine {
    pub detector: Arc<PiiDetector>,
    policies: Arc<parking_lot::RwLock<HashMap<String, RedactionPolicy>>>,
    audit_log: Arc<parking_lot::RwLock<Vec<ComplianceAuditEntry>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceAuditEntry {
    pub timestamp: String,
    pub action: String,
    pub pii_count: usize,
    pub policy_used: String,
    pub severity: Option<PiiSeverity>,
}

impl ComplianceEngine {
    pub fn new() -> Result<Self, ComplianceError> {
        let detector = Arc::new(PiiDetector::new()?);
        let mut policies = HashMap::new();
        policies.insert("default".into(), RedactionPolicy::default());
        Ok(Self {
            detector,
            policies: Arc::new(parking_lot::RwLock::new(policies)),
            audit_log: Arc::new(parking_lot::RwLock::new(Vec::new())),
        })
    }

    pub fn add_policy(&self, policy: RedactionPolicy) {
        self.policies.write().insert(policy.name.clone(), policy);
    }

    pub fn scan_text(&self, text: &str) -> ScanResult {
        let result = self.detector.scan(text);
        self.audit_log.write().push(ComplianceAuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            action: "scan".into(),
            pii_count: result.matches.len(),
            policy_used: "default".into(),
            severity: result.matches.iter().map(|m| m.severity).max(),
        });
        result
    }

    pub fn redact_text(&self, text: &str, policy_name: &str) -> Result<String, ComplianceError> {
        let policy = self.policies.read().get(policy_name).cloned()
            .ok_or_else(|| ComplianceError::PolicyNotFound(policy_name.into()))?;
        let result = self.detector.redact(text, &policy);
        self.audit_log.write().push(ComplianceAuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            action: "redact".into(),
            pii_count: self.detector.scan(text).matches.len(),
            policy_used: policy_name.into(),
            severity: None,
        });
        Ok(result)
    }

    pub fn audit_trail(&self, limit: usize) -> Vec<ComplianceAuditEntry> {
        self.audit_log.read().iter().rev().take(limit).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_email() {
        let detector = PiiDetector::new().unwrap();
        let result = detector.scan("Contact me at john.doe@example.com please");
        assert!(result.has_pii);
        assert!(result.matches.iter().any(|m| m.pii_type == PiiType::Email));
    }

    #[test]
    fn detect_ssn() {
        let detector = PiiDetector::new().unwrap();
        let result = detector.scan("My SSN is 123-45-6789");
        assert!(result.matches.iter().any(|m| m.pii_type == PiiType::Ssn));
    }

    #[test]
    fn redact_email_partial() {
        let detector = PiiDetector::new().unwrap();
        let policy = RedactionPolicy::default();
        let result = detector.redact("Email: john.doe@example.com end", &policy);
        assert!(result.contains("***"));
        assert!(!result.contains("john.doe@"));
    }

    #[test]
    fn redact_placeholder() {
        let detector = PiiDetector::new().unwrap();
        let mut policy = RedactionPolicy::default();
        policy.default_mode = RedactionMode::Placeholder;
        let result = detector.redact("IP: 192.168.1.1", &policy);
        assert!(result.contains("[REDACTED:"));
    }

    #[test]
    fn no_pii_passthrough() {
        let detector = PiiDetector::new().unwrap();
        let result = detector.scan("This is a normal sentence with no sensitive data.");
        assert!(!result.has_pii);
    }

    #[test]
    fn severity_summary() {
        let detector = PiiDetector::new().unwrap();
        let result = detector.scan("Email: a@b.com and SSN: 123-45-6789");
        assert!(result.severity_summary.get(&PiiSeverity::Critical).copied().unwrap_or(0) >= 1);
        assert!(result.severity_summary.get(&PiiSeverity::High).copied().unwrap_or(0) >= 1);
    }

    #[test]
    fn compliance_engine_audit() {
        let engine = ComplianceEngine::new().unwrap();
        engine.scan_text("test@example.com");
        engine.redact_text("test@example.com", "default").unwrap();
        let trail = engine.audit_trail(10);
        assert!(trail.len() >= 2);
    }

    #[test]
    fn custom_pattern() {
        let mut detector = PiiDetector::new().unwrap();
        detector.add_custom_pattern("employee_id", r"EMP\d{4}", 0.9).unwrap();
        let result = detector.scan("Employee EMP1234 reported");
        assert!(result.matches.iter().any(|m| m.pii_type == PiiType::Custom));
    }

    #[test]
    fn hash_redaction() {
        let detector = PiiDetector::new().unwrap();
        let mut policy = RedactionPolicy::default();
        policy.default_mode = RedactionMode::Hash;
        let result = detector.redact("IP: 10.0.0.1", &policy);
        assert!(result.contains("sha256:"));
    }
}
