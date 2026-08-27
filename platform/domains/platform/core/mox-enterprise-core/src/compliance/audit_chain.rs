//! 审计哈希链 — 不可篡改的审计日志链
//!
//! 每条审计记录包含前一条记录的哈希，形成链式结构。
//! 篡改任何一条记录都会导致后续所有哈希不匹配。

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use parking_lot::RwLock;

/// 审计记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    /// 记录序号（从0开始递增）
    pub index: u64,
    /// 时间戳（Unix毫秒）
    pub timestamp: i64,
    /// 租户ID
    pub tenant_id: String,
    /// 操作者ID
    pub actor_id: String,
    /// 操作者类型（user/system/service）
    pub actor_type: String,
    /// 操作类型（create/update/delete/login/access/...）
    pub action: String,
    /// 资源类型
    pub resource_type: String,
    /// 资源ID
    pub resource_id: String,
    /// 操作结果（success/failure/denied）
    pub result: String,
    /// 详细信息（JSON）
    pub details: serde_json::Value,
    /// IP地址
    pub ip_address: Option<String>,
    /// 前一条记录的哈希（SHA-256，十六进制）
    pub previous_hash: String,
    /// 本条记录的哈希（SHA-256，十六进制）
    pub hash: String,
}

impl AuditRecord {
    /// 计算记录哈希（不含hash字段本身）
    pub fn calculate_hash(&self) -> String {
        let content = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.index,
            self.timestamp,
            self.tenant_id,
            self.actor_id,
            self.actor_type,
            self.action,
            self.resource_type,
            self.resource_id,
            self.result,
            self.details,
            self.ip_address.as_deref().unwrap_or(""),
            self.previous_hash,
            "Mox-Audit-Chain-v1"
        );
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// 验证记录哈希是否正确
    pub fn verify_hash(&self) -> bool {
        self.hash == self.calculate_hash()
    }
}

/// 审计哈希链
pub struct AuditChain {
    records: RwLock<Vec<AuditRecord>>,
    /// 链ID（用于多链隔离）
    chain_id: String,
    /// 最大内存记录数（超过后持久化并截断）
    max_memory_records: usize,
}

impl AuditChain {
    /// 创建新链（创世块）
    pub fn new(chain_id: impl Into<String>) -> Self {
        let chain = Self {
            records: RwLock::new(Vec::new()),
            chain_id: chain_id.into(),
            max_memory_records: 100_000,
        };
        // 创建创世块
        let genesis = AuditRecord {
            index: 0,
            timestamp: chrono::Utc::now().timestamp_millis(),
            tenant_id: "system".into(),
            actor_id: "system".into(),
            actor_type: "system".into(),
            action: "genesis".into(),
            resource_type: "chain".into(),
            resource_id: chain.chain_id.clone(),
            result: "success".into(),
            details: serde_json::json!({ "chain_id": chain.chain_id }),
            ip_address: None,
            previous_hash: "0".repeat(64),
            hash: String::new(),
        };
        let hash = genesis.calculate_hash();
        let mut genesis = genesis;
        genesis.hash = hash;
        chain.records.write().push(genesis);
        chain
    }

    pub fn with_max_memory_records(mut self, max: usize) -> Self {
        self.max_memory_records = max;
        self
    }

    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }

    /// 追加审计记录
    pub fn append(
        &self,
        tenant_id: &str,
        actor_id: &str,
        actor_type: &str,
        action: &str,
        resource_type: &str,
        resource_id: &str,
        result: &str,
        details: serde_json::Value,
        ip_address: Option<String>,
    ) -> AuditRecord {
        let mut records = self.records.write();
        let last_hash = records.last().map(|r| r.hash.clone()).unwrap_or_else(|| "0".repeat(64));
        let index = records.len() as u64;

        let mut record = AuditRecord {
            index,
            timestamp: chrono::Utc::now().timestamp_millis(),
            tenant_id: tenant_id.into(),
            actor_id: actor_id.into(),
            actor_type: actor_type.into(),
            action: action.into(),
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
            result: result.into(),
            details,
            ip_address,
            previous_hash: last_hash,
            hash: String::new(),
        };
        record.hash = record.calculate_hash();
        records.push(record.clone());

        // 内存截断（超过上限时保留最新的N条）
        if records.len() > self.max_memory_records {
            let drain_count = records.len() - self.max_memory_records;
            records.drain(0..drain_count);
        }

        record
    }

    /// 验证整条链的完整性
    pub fn verify_chain(&self) -> ChainVerificationResult {
        let records = self.records.read();
        let mut invalid_count = 0;
        let mut first_invalid: Option<u64> = None;

        for (i, record) in records.iter().enumerate() {
            if !record.verify_hash() {
                invalid_count += 1;
                if first_invalid.is_none() {
                    first_invalid = Some(record.index);
                }
            }
            // 验证前向链接（除了创世块）
            if i > 0 {
                let prev = &records[i - 1];
                if record.previous_hash != prev.hash {
                    invalid_count += 1;
                    if first_invalid.is_none() {
                        first_invalid = Some(record.index);
                    }
                }
            }
        }

        ChainVerificationResult {
            valid: invalid_count == 0,
            total_records: records.len() as u64,
            invalid_count,
            first_invalid_index: first_invalid,
        }
    }

    /// 获取记录数量
    pub fn len(&self) -> usize {
        self.records.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.read().is_empty()
    }

    /// 获取最新记录
    pub fn latest(&self) -> Option<AuditRecord> {
        self.records.read().last().cloned()
    }

    /// 按租户查询记录
    pub fn query_by_tenant(&self, tenant_id: &str, limit: usize) -> Vec<AuditRecord> {
        self.records.read()
            .iter()
            .rev()
            .filter(|r| r.tenant_id == tenant_id)
            .take(limit)
            .cloned()
            .collect()
    }
}

/// 链验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerificationResult {
    pub valid: bool,
    pub total_records: u64,
    pub invalid_count: u64,
    pub first_invalid_index: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_block() {
        let chain = AuditChain::new("test-chain");
        assert_eq!(chain.len(), 1);
        let genesis = chain.latest().unwrap();
        assert_eq!(genesis.index, 0);
        assert!(genesis.verify_hash());
    }

    #[test]
    fn test_append_and_verify() {
        let chain = AuditChain::new("test-chain");
        chain.append("tenant1", "user1", "user", "login", "session", "s1", "success", serde_json::json!({}), None);
        chain.append("tenant1", "user1", "user", "create", "document", "d1", "success", serde_json::json!({"title": "test"}), None);
        assert_eq!(chain.len(), 3);
        let result = chain.verify_chain();
        assert!(result.valid);
        assert_eq!(result.invalid_count, 0);
    }

    #[test]
    fn test_tamper_detection() {
        let chain = AuditChain::new("test-chain");
        chain.append("t1", "u1", "user", "login", "session", "s1", "success", serde_json::json!({}), None);
        let record = chain.append("t1", "u1", "user", "create", "doc", "d1", "success", serde_json::json!({}), None);

        // 篡改记录
        let mut records = chain.records.write();
        if let Some(r) = records.iter_mut().find(|r| r.index == record.index) {
            r.action = "delete".into();
        }
        drop(records);

        let result = chain.verify_chain();
        assert!(!result.valid);
        assert!(result.invalid_count > 0);
    }
}
