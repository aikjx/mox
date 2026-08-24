//! 等保三级 hash_chain 审计日志（替换原 dengbao_skeleton）
//!
//! # 设计目标（GB/T 22239-2019 三级 8.3.3 安全审计）
//! 1. **链式哈希**：每块 block_hash 依赖 `prev_hash`，篡改任一字段 → 从该块开始校验全断
//! 2. **WORM 语义**：`append` 成功后，用户不可修改/删除已写块（本层通过内部结构不可变 + pub API 只读保证；SQLite 触发器由 A-6 补充）
//! 3. **独立可验证**：`examples/verify-hash-chain.rs` 读 JSON 文件 → 输出 JSON 结果，exit 0 ⇔ integrity=true
//! 4. **HMAC-SHA256 签名**：每块携带 HMAC（由审计根密钥签发），防止离线替换整条链
//! 5. **全精度**：所有字段以原值串联哈希（不截断，不 toFixed）

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// 审计事件结局
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Outcome {
    Allow,
    Deny,
    Success,
    Failure,
    Error,
}

impl Outcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Outcome::Allow => "ALLOW",
            Outcome::Deny => "DENY",
            Outcome::Success => "SUCCESS",
            Outcome::Failure => "FAILURE",
            Outcome::Error => "ERROR",
        }
    }
}

/// 单个链式块（全字段参与 hash，hmac_signature 不参与 block_hash）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HashChainBlock {
    /// 单调递增序号，从 0 (genesis) 起
    pub idx: u64,
    /// 时间戳 ms
    pub ts_ms: u64,
    /// 主体（用户/服务/系统）
    pub actor: String,
    /// 动作（IAM Authorize / STS AssumeRole / PUT / Quota / ...）
    pub action: String,
    /// 资源 ARN 或路径
    pub resource: String,
    /// 处理结果
    pub outcome: Outcome,
    /// 业务载荷 hash（对真实请求体独立取 sha256-hex；块内不保存原数据以避免链膨胀）
    pub payload_hash: String,
    /// 前一块 block_hash（genesis 固定值 `"GENESIS"`）
    pub prev_hash: String,
    /// 本块 hash：`sha256(prev_hash || idx || ts_ms || actor || action || resource || outcome || payload_hash)`
    pub block_hash: String,
    /// HMAC-SHA256(ROOT_KEY, block_hash) —— 独立签名，防离线全链替换
    pub hmac_signature: String,
}

/// Hash Chain 校验结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainVerifyResult {
    pub blocks: u64,
    pub integrity: bool,
    /// 第一个校验失败的块 idx（当 integrity=false 时）
    pub broken_at: Option<u64>,
    pub last_ts_ms: Option<u64>,
}

/// Hash Chain 根
pub struct HashChain {
    blocks: parking_lot::Mutex<Vec<HashChainBlock>>,
    /// HMAC 根密钥；任意字节串
    root_key: Vec<u8>,
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 等保三级 Genesis 固定值：链根锚点
pub const GENESIS_PREV_HASH: &str = "GENESIS";
pub const GENESIS_ACTOR: &str = "SYSTEM";
pub const GENESIS_ACTION: &str = "CHAIN_INIT";
pub const GENESIS_RESOURCE: &str = "urn:xuanji:dengbao:chain";

impl HashChain {
    /// 创建新链并写入 genesis 块
    pub fn new(root_key: impl AsRef<[u8]>) -> Self {
        let root_key = root_key.as_ref().to_vec();
        let idx: u64 = 0;
        let ts = now_ms();
        let outcome = Outcome::Success;
        let payload_hash = sha256_hex(b"genesis");
        let (block_hash, hmac_signature) =
            Self::compute_block(&root_key, GENESIS_PREV_HASH, idx, ts, GENESIS_ACTOR, GENESIS_ACTION, GENESIS_RESOURCE, outcome, &payload_hash);
        let genesis = HashChainBlock {
            idx,
            ts_ms: ts,
            actor: GENESIS_ACTOR.into(),
            action: GENESIS_ACTION.into(),
            resource: GENESIS_RESOURCE.into(),
            outcome,
            payload_hash,
            prev_hash: GENESIS_PREV_HASH.into(),
            block_hash,
            hmac_signature,
        };
        Self {
            blocks: parking_lot::Mutex::new(vec![genesis]),
            root_key,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn compute_block(
        root_key: &[u8],
        prev_hash: &str,
        idx: u64,
        ts_ms: u64,
        actor: &str,
        action: &str,
        resource: &str,
        outcome: Outcome,
        payload_hash: &str,
    ) -> (String, String) {
        // block_hash = sha256(prev_hash || idx(LE8) || ts_ms(LE8) || actor || action || resource || outcome || payload_hash)
        let mut h = Sha256::new();
        h.update(prev_hash.as_bytes());
        h.update(idx.to_le_bytes());
        h.update(ts_ms.to_le_bytes());
        h.update(actor.as_bytes());
        h.update(action.as_bytes());
        h.update(resource.as_bytes());
        h.update(outcome.as_str().as_bytes());
        h.update(payload_hash.as_bytes());
        let block_hash = hex::encode(h.finalize());

        let mut mac = HmacSha256::new_from_slice(root_key).expect("Hmac accepts any key size");
        mac.update(block_hash.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());
        (block_hash, sig)
    }

    /// 追加审计事件（原子；WORM：追加成功后不可修改）
    ///
    /// - payload_hash：业务请求体的 sha256-hex；若为空会对空串取 hash
    /// - 返回克隆的新块
    pub fn append(
        &self,
        actor: &str,
        action: &str,
        resource: &str,
        outcome: Outcome,
        payload_hash: Option<&str>,
    ) -> HashChainBlock {
        self.append_with_ts(actor, action, resource, outcome, payload_hash, now_ms())
    }

    /// 测试友好：指定时间戳
    pub fn append_with_ts(
        &self,
        actor: &str,
        action: &str,
        resource: &str,
        outcome: Outcome,
        payload_hash: Option<&str>,
        ts_ms: u64,
    ) -> HashChainBlock {
        let mut blocks = self.blocks.lock();
        let prev = blocks.last().expect("chain non-empty (genesis exists)");
        let idx = prev.idx + 1;
        let ph = payload_hash.unwrap_or_else(|| sha256_hex(b"").as_str()).to_string();
        let (block_hash, hmac_signature) = Self::compute_block(
            &self.root_key,
            &prev.block_hash,
            idx,
            ts_ms,
            actor,
            action,
            resource,
            outcome,
            &ph,
        );
        let block = HashChainBlock {
            idx,
            ts_ms,
            actor: actor.into(),
            action: action.into(),
            resource: resource.into(),
            outcome,
            payload_hash: ph,
            prev_hash: prev.block_hash.clone(),
            block_hash,
            hmac_signature,
        };
        blocks.push(block.clone());
        block
    }

    pub fn len(&self) -> usize {
        self.blocks.lock().len()
    }
    pub fn is_empty(&self) -> bool {
        self.blocks.lock().is_empty()
    }

    /// 全链校验
    pub fn verify(&self) -> ChainVerifyResult {
        let blocks = self.blocks.lock();
        verify_blocks(&blocks, &self.root_key)
    }

    /// 读视图：克隆所有块
    pub fn snapshot(&self) -> Vec<HashChainBlock> {
        self.blocks.lock().clone()
    }

    /// 校验（静态版本，供独立 verify CLI 使用）
    pub fn verify_blocks_static(blocks: &[HashChainBlock], root_key: &[u8]) -> ChainVerifyResult {
        verify_blocks(blocks, root_key)
    }
}

fn sha256_hex(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    hex::encode(h.finalize())
}

fn verify_blocks(blocks: &[HashChainBlock], root_key: &[u8]) -> ChainVerifyResult {
    let n = blocks.len() as u64;
    let mut last_ts: Option<u64> = None;
    if blocks.is_empty() {
        return ChainVerifyResult {
            blocks: 0,
            integrity: false,
            broken_at: Some(0),
            last_ts_ms: None,
        };
    }
    // genesis 校验
    let g = &blocks[0];
    if g.prev_hash != GENESIS_PREV_HASH {
        return ChainVerifyResult {
            blocks: n,
            integrity: false,
            broken_at: Some(0),
            last_ts_ms: Some(g.ts_ms),
        };
    }
    let (exp_hash, exp_sig) = HashChain::compute_block(
        root_key,
        GENESIS_PREV_HASH,
        g.idx,
        g.ts_ms,
        &g.actor,
        &g.action,
        &g.resource,
        g.outcome,
        &g.payload_hash,
    );
    if g.block_hash != exp_hash || g.hmac_signature != exp_sig {
        return ChainVerifyResult {
            blocks: n,
            integrity: false,
            broken_at: Some(0),
            last_ts_ms: Some(g.ts_ms),
        };
    }
    last_ts = Some(g.ts_ms);
    // 后续块
    for i in 1..blocks.len() {
        let prev = &blocks[i - 1];
        let cur = &blocks[i];
        let (exp_hash, exp_sig) = HashChain::compute_block(
            root_key,
            &prev.block_hash,
            cur.idx,
            cur.ts_ms,
            &cur.actor,
            &cur.action,
            &cur.resource,
            cur.outcome,
            &cur.payload_hash,
        );
        if cur.prev_hash != prev.block_hash
            || cur.block_hash != exp_hash
            || cur.hmac_signature != exp_sig
            || cur.idx != prev.idx + 1
        {
            return ChainVerifyResult {
                blocks: n,
                integrity: false,
                broken_at: Some(i as u64),
                last_ts_ms,
            };
        }
        last_ts = Some(cur.ts_ms);
    }
    ChainVerifyResult {
        blocks: n,
        integrity: true,
        broken_at: None,
        last_ts_ms: last_ts,
    }
}

/// 独立 CLI 工具核心逻辑（供 `examples/verify-hash-chain.rs` 直接复用）
pub fn verify_json_file(
    json_bytes: &[u8],
    root_key_hex: &str,
) -> Result<ChainVerifyResult, String> {
    let root_key = if root_key_hex.is_empty() {
        b"xuanji-dengbao-v3-default-root-key".to_vec()
    } else {
        hex::decode(root_key_hex).map_err(|e| format!("root_key not hex: {e}"))?
    };
    let blocks: Vec<HashChainBlock> =
        serde_json::from_slice(json_bytes).map_err(|e| format!("json parse: {e}"))?;
    Ok(verify_blocks(&blocks, &root_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"unit-test-root-0000000000000000000000";

    #[test]
    fn t_a5_1_append_1000_verify_ok() {
        let c = HashChain::new(KEY);
        for i in 0..1000u64 {
            c.append(
                "alice",
                "s3:GetObject",
                &format!("arn:cloud:::bucket/a/obj-{i}"),
                if i % 7 == 0 { Outcome::Deny } else { Outcome::Allow },
                None,
            );
        }
        let r = c.verify();
        assert_eq!(r.blocks, 1001); // genesis + 1000
        assert!(r.integrity);
        assert!(r.broken_at.is_none());
    }

    #[test]
    fn t_a5_2_tamper_block_500_detected_at_500() {
        let c = HashChain::new(KEY);
        for i in 0..1000u64 {
            c.append("u", "a", &format!("r{i}"), Outcome::Success, None);
        }
        // 外部篡改（绕过 pub API，直接改内部 mutex → 使用 snapshot）
        let mut snap = c.snapshot();
        snap[500].payload_hash = "0".repeat(64);
        let r = verify_blocks(&snap, KEY);
        assert!(!r.integrity);
        assert_eq!(r.broken_at, Some(500));
    }

    #[test]
    fn t_a5_3_genesis_anchor_fixed() {
        let c = HashChain::new(KEY);
        let snap = c.snapshot();
        assert_eq!(snap[0].idx, 0);
        assert_eq!(snap[0].prev_hash, GENESIS_PREV_HASH);
        assert_eq!(snap[0].actor, GENESIS_ACTOR);
        assert_eq!(snap[0].action, GENESIS_ACTION);
        let r = c.verify();
        assert!(r.integrity);
    }

    #[test]
    fn t_a5_4_empty_chain_fail() {
        let r = verify_blocks(&[], KEY);
        assert!(!r.integrity);
        assert_eq!(r.broken_at, Some(0));
    }

    #[test]
    fn t_a5_5_hmac_signature_different_keys_mismatch() {
        let c = HashChain::new(KEY);
        c.append("a", "b", "c", Outcome::Success, None);
        let snap = c.snapshot();
        let r1 = verify_blocks(&snap, KEY);
        assert!(r1.integrity);
        let r2 = verify_blocks(&snap, b"different-key-1234567890123456789");
        assert!(!r2.integrity);
        assert_eq!(r2.broken_at, Some(0));
    }

    #[test]
    fn t_a5_6_idx_monotonicity_enforced() {
        // 手工构造 idx 跳号
        let c = HashChain::new(KEY);
        let b0 = c.snapshot()[0].clone();
        // b1 idx 直接 = 2 (跳过 1)
        let (bh, sig) = HashChain::compute_block(
            KEY,
            &b0.block_hash,
            2,
            now_ms(),
            "a",
            "x",
            "r",
            Outcome::Success,
            &sha256_hex(b"p"),
        );
        let b1 = HashChainBlock {
            idx: 2,
            ts_ms: now_ms(),
            actor: "a".into(),
            action: "x".into(),
            resource: "r".into(),
            outcome: Outcome::Success,
            payload_hash: sha256_hex(b"p"),
            prev_hash: b0.block_hash.clone(),
            block_hash: bh,
            hmac_signature: sig,
        };
        let arr = vec![b0, b1];
        let r = verify_blocks(&arr, KEY);
        assert!(!r.integrity);
        // idx 跳号在 i=1 判 cur.idx != prev.idx + 1 → broken_at=Some(1)
        assert_eq!(r.broken_at, Some(1));
    }

    #[test]
    fn t_a5_7_verify_json_file_happy() {
        let c = HashChain::new(b""); // empty key → default logic path 一致
        c.append("x", "y", "z", Outcome::Allow, None);
        let snap = c.snapshot();
        let json = serde_json::to_vec(&snap).unwrap();
        let r = verify_json_file(&json, "").unwrap();
        assert!(r.integrity);
        assert_eq!(r.blocks, 2);
    }
}
