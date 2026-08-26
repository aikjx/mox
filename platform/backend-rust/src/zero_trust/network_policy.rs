//! 网络策略生成器
//!
//! 零信任网络策略：默认拒绝，最小权限
//!
//! 核心能力：
//! - 基于身份的网络策略（不是基于 IP）
//! - 默认拒绝所有入站/出站流量
//! - 动态策略生成（基于 SPIFFE ID）
//! - 策略审计与验证
//! - Kubernetes NetworkPolicy 生成
//! - Istio AuthorizationPolicy 生成

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::RwLock;
use uuid::Uuid;

/// 策略规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub action: PolicyAction,
    pub source: PolicyPeer,
    pub destination: PolicyPeer,
    pub ports: Vec<PolicyPort>,
    pub protocols: Vec<String>,
    pub enabled: bool,
    pub priority: u32,
    pub created_at: String,
}

/// 策略动作
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PolicyAction {
    Allow,
    Deny,
    Log,
    RateLimit,
}

/// 策略对端
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyPeer {
    pub spiffe_ids: Vec<String>,
    pub namespaces: Vec<String>,
    pub service_accounts: Vec<String>,
    pub ip_blocks: Vec<String>,
    pub labels: std::collections::HashMap<String, String>,
}

impl Default for PolicyPeer {
    fn default() -> Self {
        Self {
            spiffe_ids: vec![],
            namespaces: vec![],
            service_accounts: vec![],
            ip_blocks: vec![],
            labels: std::collections::HashMap::new(),
        }
    }
}

/// 策略端口
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyPort {
    pub port: u16,
    pub protocol: String,
}

/// 网络策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub description: String,
    pub default_deny_inbound: bool,
    pub default_deny_outbound: bool,
    pub rules: Vec<PolicyRule>,
    pub pod_selector: std::collections::HashMap<String, String>,
    pub policy_types: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 网络策略生成器
pub struct NetworkPolicyGenerator {
    policies: RwLock<Vec<NetworkPolicy>>,
    rules: RwLock<Vec<PolicyRule>>,
    trusted_spiffe_ids: RwLock<HashSet<String>>,
    default_deny: RwLock<bool>,
    total_policies_generated: std::sync::atomic::AtomicU64,
    total_rules_evaluated: std::sync::atomic::AtomicU64,
}

impl NetworkPolicyGenerator {
    /// 创建网络策略生成器
    pub fn new() -> Self {
        Self {
            policies: RwLock::new(Vec::new()),
            rules: RwLock::new(Vec::new()),
            trusted_spiffe_ids: RwLock::new(HashSet::new()),
            default_deny: RwLock::new(true),
            total_policies_generated: std::sync::atomic::AtomicU64::new(0),
            total_rules_evaluated: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 创建默认拒绝策略
    pub fn create_default_deny_policy(&self, namespace: &str) -> NetworkPolicy {
        let policy = NetworkPolicy {
            id: Uuid::new_v4().to_string(),
            name: format!("{}-default-deny", namespace),
            namespace: namespace.to_string(),
            description: "零信任默认拒绝所有流量".to_string(),
            default_deny_inbound: true,
            default_deny_outbound: true,
            rules: vec![],
            pod_selector: std::collections::HashMap::new(),
            policy_types: vec!["Ingress".to_string(), "Egress".to_string()],
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        self.policies.write().unwrap().push(policy.clone());
        self.total_policies_generated.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        policy
    }

    /// 添加允许规则
    pub fn add_allow_rule(&self, name: &str, source: PolicyPeer, destination: PolicyPeer, ports: Vec<PolicyPort>) -> PolicyRule {
        let rule = PolicyRule {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: format!("允许 {} -> {}", name, name),
            action: PolicyAction::Allow,
            source,
            destination,
            ports,
            protocols: vec!["TCP".to_string()],
            enabled: true,
            priority: 100,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.rules.write().unwrap().push(rule.clone());
        rule
    }

    /// 添加拒绝规则
    pub fn add_deny_rule(&self, name: &str, source: PolicyPeer, destination: PolicyPeer) -> PolicyRule {
        let rule = PolicyRule {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: format!("拒绝 {} -> {}", name, name),
            action: PolicyAction::Deny,
            source,
            destination,
            ports: vec![],
            protocols: vec![],
            enabled: true,
            priority: 10,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.rules.write().unwrap().push(rule.clone());
        rule
    }

    /// 评估流量是否允许
    pub fn evaluate_traffic(&self, source_spiffe: &str, dest_spiffe: &str, port: u16, protocol: &str) -> TrafficDecision {
        self.total_rules_evaluated.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let rules = self.rules.read().unwrap();
        let mut sorted_rules: Vec<&PolicyRule> = rules.iter()
            .filter(|r| r.enabled)
            .collect();
        sorted_rules.sort_by_key(|r| r.priority);

        for rule in sorted_rules {
            if Self::rule_matches(rule, source_spiffe, dest_spiffe, port, protocol) {
                return match rule.action {
                    PolicyAction::Allow => TrafficDecision { allowed: true, matched_rule: rule.name.clone(), reason: "匹配允许规则".to_string() },
                    PolicyAction::Deny => TrafficDecision { allowed: false, matched_rule: rule.name.clone(), reason: "匹配拒绝规则".to_string() },
                    PolicyAction::Log => TrafficDecision { allowed: true, matched_rule: rule.name.clone(), reason: "匹配日志规则（允许但记录）".to_string() },
                    PolicyAction::RateLimit => TrafficDecision { allowed: true, matched_rule: rule.name.clone(), reason: "匹配限流规则".to_string() },
                };
            }
        }

        // 默认策略
        if *self.default_deny.read().unwrap() {
            TrafficDecision { allowed: false, matched_rule: "default-deny".to_string(), reason: "默认拒绝（零信任）".to_string() }
        } else {
            TrafficDecision { allowed: true, matched_rule: "default-allow".to_string(), reason: "默认允许".to_string() }
        }
    }

    fn rule_matches(rule: &PolicyRule, source_spiffe: &str, dest_spiffe: &str, port: u16, protocol: &str) -> bool {
        // 检查源
        if !rule.source.spiffe_ids.is_empty() && !rule.source.spiffe_ids.iter().any(|s| Self::spiffe_match(s, source_spiffe)) {
            return false;
        }

        // 检查目标
        if !rule.destination.spiffe_ids.is_empty() && !rule.destination.spiffe_ids.iter().any(|s| Self::spiffe_match(s, dest_spiffe)) {
            return false;
        }

        // 检查端口
        if !rule.ports.is_empty() && !rule.ports.iter().any(|p| p.port == port) {
            return false;
        }

        // 检查协议
        if !rule.protocols.is_empty() && !rule.protocols.iter().any(|p| p.eq_ignore_ascii_case(protocol)) {
            return false;
        }

        true
    }

    fn spiffe_match(pattern: &str, actual: &str) -> bool {
        if pattern.ends_with('*') {
            actual.starts_with(&pattern[..pattern.len() - 1])
        } else {
            pattern == actual
        }
    }

    /// 生成 Kubernetes NetworkPolicy YAML
    pub fn generate_k8s_network_policy(&self, policy: &NetworkPolicy) -> String {
        let mut yaml = String::new();
        yaml.push_str("apiVersion: networking.k8s.io/v1\n");
        yaml.push_str("kind: NetworkPolicy\n");
        yaml.push_str(&format!("metadata:\n  name: {}\n  namespace: {}\n", policy.name, policy.namespace));
        yaml.push_str("spec:\n");

        if policy.pod_selector.is_empty() {
            yaml.push_str("  podSelector: {}\n");
        } else {
            yaml.push_str("  podSelector:\n    matchLabels:\n");
            for (k, v) in &policy.pod_selector {
                yaml.push_str(&format!("      {}: {}\n", k, v));
            }
        }

        yaml.push_str(&format!("  policyTypes:\n"));
        for pt in &policy.policy_types {
            yaml.push_str(&format!("  - {}\n", pt));
        }

        if policy.default_deny_inbound {
            yaml.push_str("  ingress: []\n");
        }
        if policy.default_deny_outbound {
            yaml.push_str("  egress: []\n");
        }

        yaml
    }

    /// 生成 Istio AuthorizationPolicy YAML
    pub fn generate_istio_authorization_policy(&self, policy: &NetworkPolicy) -> String {
        let mut yaml = String::new();
        yaml.push_str("apiVersion: security.istio.io/v1beta1\n");
        yaml.push_str("kind: AuthorizationPolicy\n");
        yaml.push_str(&format!("metadata:\n  name: {}\n  namespace: {}\n", policy.name, policy.namespace));
        yaml.push_str("spec:\n");
        yaml.push_str("  selector:\n    matchLabels:\n");
        if policy.pod_selector.is_empty() {
            yaml.push_str("      app: mox\n");
        } else {
            for (k, v) in &policy.pod_selector {
                yaml.push_str(&format!("      {}: {}\n", k, v));
            }
        }

        if policy.default_deny_inbound {
            yaml.push_str("  action: DENY\n");
            yaml.push_str("  rules:\n  - from:\n    - source:\n        principals: [\"*\"]\n");
        }

        yaml
    }

    /// 添加受信任 SPIFFE ID
    pub fn add_trusted_spiffe_id(&self, spiffe_id: &str) {
        self.trusted_spiffe_ids.write().unwrap().insert(spiffe_id.to_string());
    }

    /// 移除受信任 SPIFFE ID
    pub fn remove_trusted_spiffe_id(&self, spiffe_id: &str) -> bool {
        self.trusted_spiffe_ids.write().unwrap().remove(spiffe_id)
    }

    /// 设置默认拒绝
    pub fn set_default_deny(&self, deny: bool) {
        *self.default_deny.write().unwrap() = deny;
    }

    /// 获取所有策略
    pub fn list_policies(&self) -> Vec<NetworkPolicy> {
        self.policies.read().unwrap().clone()
    }

    /// 获取所有规则
    pub fn list_rules(&self, enabled_only: bool) -> Vec<PolicyRule> {
        self.rules.read().unwrap()
            .iter()
            .filter(|r| !enabled_only || r.enabled)
            .cloned()
            .collect()
    }

    /// 获取统计
    pub fn stats(&self) -> NetworkPolicyStats {
        NetworkPolicyStats {
            total_policies: self.policies.read().unwrap().len(),
            total_rules: self.rules.read().unwrap().len(),
            enabled_rules: self.rules.read().unwrap().iter().filter(|r| r.enabled).count(),
            allow_rules: self.rules.read().unwrap().iter().filter(|r| r.action == PolicyAction::Allow).count(),
            deny_rules: self.rules.read().unwrap().iter().filter(|r| r.action == PolicyAction::Deny).count(),
            trusted_spiffe_ids: self.trusted_spiffe_ids.read().unwrap().len(),
            default_deny: *self.default_deny.read().unwrap(),
            total_policies_generated: self.total_policies_generated.load(std::sync::atomic::Ordering::Relaxed),
            total_rules_evaluated: self.total_rules_evaluated.load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

impl Default for NetworkPolicyGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// 流量决策
#[derive(Debug, Clone, Serialize)]
pub struct TrafficDecision {
    pub allowed: bool,
    pub matched_rule: String,
    pub reason: String,
}

/// 网络策略统计
#[derive(Debug, Clone, Serialize)]
pub struct NetworkPolicyStats {
    pub total_policies: usize,
    pub total_rules: usize,
    pub enabled_rules: usize,
    pub allow_rules: usize,
    pub deny_rules: usize,
    pub trusted_spiffe_ids: usize,
    pub default_deny: bool,
    pub total_policies_generated: u64,
    pub total_rules_evaluated: u64,
}
