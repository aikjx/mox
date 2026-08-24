use bytes::Bytes;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RSError {
    TooManyShardsMissing(String),
    ShardSizeMismatch(String),
    InvalidInput(String),
}

impl fmt::Display for RSError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RSError::TooManyShardsMissing(m) => write!(f, "Too many shards missing: {}", m),
            RSError::ShardSizeMismatch(m) => write!(f, "Shard size mismatch: {}", m),
            RSError::InvalidInput(m) => write!(f, "Invalid input: {}", m),
        }
    }
}

impl std::error::Error for RSError {}

pub type RSResult<T> = Result<T, RSError>;

/// 自研简化 2+1 XOR 纠删码
/// K=2 data shards, M=1 parity, total N=3 shards
/// parity = data0 XOR data1
#[derive(Debug, Clone, Default)]
pub struct ReedSolomon2Plus1;

impl ReedSolomon2Plus1 {
    /// 两个等长 bytes 做 XOR（左 oper 右）
    pub(crate) fn xor_bytes(&self, a: &[u8], b: &[u8]) -> Vec<u8> {
        assert_eq!(a.len(), b.len(), "xor_bytes requires equal length inputs");
        let mut out = Vec::with_capacity(a.len());
        for i in 0..a.len() {
            out.push(a[i] ^ b[i]);
        }
        out
    }

    /// 编码：输入 2 个 data shards，输出 [data0, data1, parity]
    /// 若 d0.len() != d1.len() → ShardSizeMismatch
    pub fn encode_2_1(&self, data: &[Bytes; 2]) -> RSResult<[Bytes; 3]> {
        if data[0].len() != data[1].len() {
            return Err(RSError::ShardSizeMismatch(format!(
                "data shards length mismatch: {} vs {}",
                data[0].len(),
                data[1].len()
            )));
        }
        let parity = self.xor_bytes(&data[0], &data[1]);
        Ok([data[0].clone(), data[1].clone(), Bytes::from(parity)])
    }

    /// 解码：输入 3 个 shards（任意 1 个为 None 表示丢失）
    /// 输出重建后的 [data0, data1]
    /// 若丢失 >= 2 返回 RSError::TooManyShardsMissing
    /// 若所有都存在，则直接返回 data0 / data1
    pub fn decode_2_1(&self, three_shards: [Option<Bytes>; 3]) -> RSResult<[Bytes; 2]> {
        let missing_count = three_shards.iter().filter(|s| s.is_none()).count();
        if missing_count >= 2 {
            return Err(RSError::TooManyShardsMissing(format!(
                "need at most 1 missing shard, got {}",
                missing_count
            )));
        }

        // 决定长度
        let len = three_shards
            .iter()
            .find_map(|s| s.as_ref().map(|b| b.len()))
            .ok_or_else(|| RSError::InvalidInput("all shards None".into()))?;

        // 补齐：把每个 shard 都变成 Vec<u8>（缺失的留空向量但长度占位）
        let mut owned: [Vec<u8>; 3] = [vec![], vec![], vec![]];
        for i in 0..3 {
            match &three_shards[i] {
                Some(b) => owned[i] = b.to_vec(),
                None => owned[i] = vec![0u8; len], // placeholder
            }
        }

        match missing_count {
            0 => {
                // 无丢失：直接返回 [d0, d1]
                Ok([Bytes::from(owned[0].clone()), Bytes::from(owned[1].clone())])
            }
            1 => {
                // 丢失 1 块：用 XOR 还原
                if three_shards[0].is_none() {
                    // missing data0: d0 = d1 XOR parity
                    let d0 = self.xor_bytes(&owned[1], &owned[2]);
                    Ok([Bytes::from(d0), Bytes::from(owned[1].clone())])
                } else if three_shards[1].is_none() {
                    // missing data1: d1 = d0 XOR parity
                    let d1 = self.xor_bytes(&owned[0], &owned[2]);
                    Ok([Bytes::from(owned[0].clone()), Bytes::from(d1)])
                } else {
                    // missing parity: 不需要 parity，返回 data0/1
                    Ok([Bytes::from(owned[0].clone()), Bytes::from(owned[1].clone())])
                }
            }
            _ => unreachable!(), // checked above
        }
    }
}
