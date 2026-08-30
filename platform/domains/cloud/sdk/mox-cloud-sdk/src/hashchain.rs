// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::client::CloudClient;
use crate::error::{CloudError, Result};
use crate::types::HashBlock;
use crate::utils::fxhash;

impl CloudClient {
    // ========== DengBao HashChain (3) ==========

    pub async fn dbhc_create_chain(&self, chain_id: &str) -> Result<()> {
        let mut s = self.lock()?;
        if s.hashchains.contains_key(chain_id) {
            return Err(CloudError::InvalidRequest(format!(
                "chain {chain_id} exists"
            )));
        }
        let genesis = HashBlock {
            index: 0,
            data: b"GENESIS".to_vec(),
            prev_hash: "0".repeat(16),
            hash: format!("{:016x}", fxhash(b"GENESIS")),
        };
        s.hashchains.insert(chain_id.to_string(), vec![genesis]);
        Ok(())
    }

    /// Append N blocks of ~1KiB each. Creates chain if not present.
    pub async fn dbhc_append_blocks(&self, chain_id: &str, count: u32) -> Result<u64> {
        let mut s = self.lock()?;
        if !s.hashchains.contains_key(chain_id) {
            let genesis = HashBlock {
                index: 0,
                data: b"GENESIS".to_vec(),
                prev_hash: "0".repeat(16),
                hash: format!("{:016x}", fxhash(b"GENESIS")),
            };
            s.hashchains.insert(chain_id.to_string(), vec![genesis]);
        }
        let chain = s.hashchains.get_mut(chain_id).unwrap();
        for i in 0..count {
            let prev = chain.last().unwrap();
            let mut data = vec![0u8; 1024];
            // embed index to make each block unique
            let idx_bytes = ((prev.index + 1) as u32 + i).to_le_bytes();
            data[..4].copy_from_slice(&idx_bytes);
            let prev_hash = prev.hash.clone();
            let hashed = {
                let mut concat = prev_hash.as_bytes().to_vec();
                concat.extend_from_slice(&data);
                format!("{:016x}", fxhash(&concat))
            };
            chain.push(HashBlock {
                index: prev.index + 1,
                data,
                prev_hash,
                hash: hashed,
            });
        }
        Ok(s.hashchains.get(chain_id).unwrap().last().unwrap().index)
    }

    pub async fn dbhc_verify_chain(&self, chain_id: &str) -> Result<bool> {
        let s = self.lock()?;
        let chain = s
            .hashchains
            .get(chain_id)
            .ok_or_else(|| CloudError::NotFound(format!("chain {chain_id}")))?;
        if chain.is_empty() {
            return Err(CloudError::HashChainVerifyFailed("empty".into()));
        }
        for i in 1..chain.len() {
            let prev = &chain[i - 1];
            let cur = &chain[i];
            if cur.prev_hash != prev.hash {
                return Err(CloudError::HashChainVerifyFailed(format!(
                    "link break at index {}",
                    cur.index
                )));
            }
            let mut concat = prev.hash.as_bytes().to_vec();
            concat.extend_from_slice(&cur.data);
            let expected = format!("{:016x}", fxhash(&concat));
            if cur.hash != expected {
                return Err(CloudError::HashChainVerifyFailed(format!(
                    "hash mismatch at index {}",
                    cur.index
                )));
            }
        }
        Ok(true)
    }
}
