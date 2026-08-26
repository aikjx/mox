//! 顶点/边的二进制 codec。
//!
//! Key 编码：
//!   Vertex Meta   : \[shard_id 2B][tag_id 1B][vid_len 1B][vid_bytes...]\
//!   Out Edge Idx  : \[shard_id 2B][b'o'][src_len 1B][src_bytes][etype_len 1B][etype_bytes][rank 8B][dst_len 1B][dst_bytes]\
//!   In Edge Idx   : \[shard_id 2B][b'i'][dst_len 1B][dst_bytes][etype_len 1B][etype_bytes][rank 8B][src_len 1B][src_bytes]\
//!
//! Value 编码：
//!   \[crc32c checksum 4B][prop_count 4B][(k_len 2B k_bytes v_len 2B v_bytes)...]\
//!
//! Roundtrip 基准：≥100 次 encode/decode 等价 GREEN。

use crate::error::{StorageError, StorageResult};
use std::collections::BTreeMap;

/// 类型化属性值：统一表达多种属性，编码时转为带 tag 的字节，解码恢复原始 Rust 类型。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum PropValue {
    Null,
    Bool(bool),
    Int(i64),
    F64(u64), // stored as to_bits for Eq
    Str(String),
    Bytes(Vec<u8>),
}
impl PropValue {
    pub fn from_str(s: &str) -> Self {
        PropValue::Str(s.to_string())
    }
    pub fn as_str(&self) -> Option<&str> {
        if let PropValue::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }
    fn tag(&self) -> u8 {
        match self {
            PropValue::Null => 0,
            PropValue::Bool(false) => 1,
            PropValue::Bool(true) => 2,
            PropValue::Int(_) => 3,
            PropValue::F64(_) => 4,
            PropValue::Str(_) => 5,
            PropValue::Bytes(_) => 6,
        }
    }
    pub fn encode_bytes(&self) -> Vec<u8> {
        match self {
            PropValue::Null | PropValue::Bool(_) => Vec::new(),
            PropValue::Int(i) => i.to_le_bytes().to_vec(),
            PropValue::F64(u) => u.to_le_bytes().to_vec(),
            PropValue::Str(s) => s.as_bytes().to_vec(),
            PropValue::Bytes(b) => b.clone(),
        }
    }
    pub fn decode(tag: u8, b: &[u8]) -> StorageResult<Self> {
        match tag {
            0 => Ok(PropValue::Null),
            1 => Ok(PropValue::Bool(false)),
            2 => Ok(PropValue::Bool(true)),
            3 => {
                if b.len() != 8 {
                    return Err(StorageError::CodecError("int bad len".into()));
                }
                let mut a = [0u8; 8];
                a.copy_from_slice(b);
                Ok(PropValue::Int(i64::from_le_bytes(a)))
            }
            4 => {
                if b.len() != 8 {
                    return Err(StorageError::CodecError("f64 bad len".into()));
                }
                let mut a = [0u8; 8];
                a.copy_from_slice(b);
                Ok(PropValue::F64(u64::from_le_bytes(a)))
            }
            5 => String::from_utf8(b.to_vec())
                .map(PropValue::Str)
                .map_err(|e| StorageError::CodecError(format!("str {e}"))),
            6 => Ok(PropValue::Bytes(b.to_vec())),
            other => Err(StorageError::CodecError(format!("unknown tag {other}"))),
        }
    }
}
// ---- CRC-32C (Castagnoli) 软件实现：零依赖边界 ----
const CRC32C_POLY: u32 = 0x82f6_3b78;
fn crc32c_table() -> [u32; 256] {
    let mut tbl = [0u32; 256];
    for i in 0..=255u32 {
        let mut crc = i;
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ CRC32C_POLY;
            } else {
                crc >>= 1;
            }
        }
        tbl[i as usize] = crc;
    }
    tbl
}

pub fn crc32c(bytes: &[u8]) -> u32 {
    let tbl = crc32c_table();
    let mut crc: u32 = 0xffff_ffff;
    for &b in bytes {
        crc = tbl[((crc ^ b as u32) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffff_ffff
}

// --- Key builder 辅助 ---
fn write_u16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}
#[allow(dead_code)] // 预留：当前编解码路径只用 u16/i64/f64，u64 提供给未来版本号/时间戳字段
fn write_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn write_i64(buf: &mut Vec<u8>, v: i64) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn read_u16(buf: &[u8], off: &mut usize) -> StorageResult<u16> {
    if buf.len() < *off + 2 {
        return Err(StorageError::CodecError("short buf u16".into()));
    }
    let mut a = [0u8; 2];
    a.copy_from_slice(&buf[*off..*off + 2]);
    *off += 2;
    Ok(u16::from_le_bytes(a))
}
#[allow(dead_code)] // 预留：对应 write_u64，供未来 u64 字段解码
fn read_u64(buf: &[u8], off: &mut usize) -> StorageResult<u64> {
    if buf.len() < *off + 8 {
        return Err(StorageError::CodecError("short buf u64".into()));
    }
    let mut a = [0u8; 8];
    a.copy_from_slice(&buf[*off..*off + 8]);
    *off += 8;
    Ok(u64::from_le_bytes(a))
}
fn read_i64(buf: &[u8], off: &mut usize) -> StorageResult<i64> {
    if buf.len() < *off + 8 {
        return Err(StorageError::CodecError("short buf i64".into()));
    }
    let mut a = [0u8; 8];
    a.copy_from_slice(&buf[*off..*off + 8]);
    *off += 8;
    Ok(i64::from_le_bytes(a))
}
fn read_u32(buf: &[u8], off: &mut usize) -> StorageResult<u32> {
    if buf.len() < *off + 4 {
        return Err(StorageError::CodecError("short buf u32".into()));
    }
    let mut a = [0u8; 4];
    a.copy_from_slice(&buf[*off..*off + 4]);
    *off += 4;
    Ok(u32::from_le_bytes(a))
}
fn write_len_bytes(buf: &mut Vec<u8>, b: &[u8], size_len: usize) -> StorageResult<()> {
    if b.len() > (1 << (8 * size_len)) - 1 {
        return Err(StorageError::CodecError("bytes too long".into()));
    }
    match size_len {
        1 => buf.push(b.len() as u8),
        2 => write_u16(buf, b.len() as u16),
        _ => return Err(StorageError::CodecError("bad size_len".into())),
    }
    buf.extend_from_slice(b);
    Ok(())
}
fn read_len_bytes<'a>(buf: &'a [u8], off: &mut usize, size_len: usize) -> StorageResult<&'a [u8]> {
    let len = match size_len {
        1 => {
            if buf.len() < *off + 1 {
                return Err(StorageError::CodecError("short lb1".into()));
            }
            let v = buf[*off] as usize;
            *off += 1;
            v
        }
        2 => read_u16(buf, off)? as usize,
        _ => return Err(StorageError::CodecError("bad size_len".into())),
    };
    if buf.len() < *off + len {
        return Err(StorageError::CodecError("short lb body".into()));
    }
    let start = *off;
    *off += len;
    Ok(&buf[start..start + len])
}

// ------------ Vertex key ------------
pub fn encode_vertex_key(shard: u16, tag: &str, vid: &str) -> StorageResult<Vec<u8>> {
    let mut buf = Vec::with_capacity(32);
    write_u16(&mut buf, shard);
    // tag_id 1B: FNV-1a mod 255+1 确定性哈希（保证 tag→tag_id 稳定）
    let tag_hash = fnv1a_8(tag.as_bytes());
    buf.push(tag_hash);
    write_len_bytes(&mut buf, vid.as_bytes(), 1)?;
    Ok(buf)
}
pub fn decode_vertex_key(key: &[u8]) -> StorageResult<(u16, u8, String)> {
    let mut off = 0;
    let shard = read_u16(key, &mut off)?;
    if key.len() < off + 1 {
        return Err(StorageError::CodecError("short tag".into()));
    }
    let tag_hash = key[off];
    off += 1;
    let vid_bytes = read_len_bytes(key, &mut off, 1)?;
    let vid = String::from_utf8(vid_bytes.to_vec())
        .map_err(|e| StorageError::CodecError(format!("vid utf8: {e}")))?;
    Ok((shard, tag_hash, vid))
}

fn fnv1a_8(data: &[u8]) -> u8 {
    let mut h: u8 = 0x81;
    for &b in data {
        h ^= b;
        h = h.wrapping_mul(0x93u8);
    }
    // 0xff reserve，保证 tag_id ≠ b'o' | b'i'（只是为了避免与 edge key 前缀冲突，
    // 真正的分流是基于列族，但一致性防御更稳）。
    if h == b'o' || h == b'i' {
        h = h.wrapping_add(1);
    }
    h
}

// ------------ Out/In Edge keys ------------
pub fn encode_out_edge_key(
    shard: u16,
    src: &str,
    etype: &str,
    rank: i64,
    dst: &str,
) -> StorageResult<Vec<u8>> {
    let mut buf = Vec::with_capacity(64);
    write_u16(&mut buf, shard);
    buf.push(b'o');
    write_len_bytes(&mut buf, src.as_bytes(), 1)?;
    write_len_bytes(&mut buf, etype.as_bytes(), 1)?;
    write_i64(&mut buf, rank);
    write_len_bytes(&mut buf, dst.as_bytes(), 1)?;
    Ok(buf)
}
pub fn decode_out_edge_key(key: &[u8]) -> StorageResult<(u16, String, String, i64, String)> {
    let mut off = 0;
    let shard = read_u16(key, &mut off)?;
    if key.len() < off + 1 || key[off] != b'o' {
        return Err(StorageError::CodecError("not out key".into()));
    }
    off += 1;
    let s = read_len_bytes(key, &mut off, 1)?;
    let e = read_len_bytes(key, &mut off, 1)?;
    let r = read_i64(key, &mut off)?;
    let d = read_len_bytes(key, &mut off, 1)?;
    Ok((
        shard,
        String::from_utf8_lossy(s).into_owned(),
        String::from_utf8_lossy(e).into_owned(),
        r,
        String::from_utf8_lossy(d).into_owned(),
    ))
}

pub fn encode_in_edge_key(
    shard: u16,
    dst: &str,
    etype: &str,
    rank: i64,
    src: &str,
) -> StorageResult<Vec<u8>> {
    let mut buf = Vec::with_capacity(64);
    write_u16(&mut buf, shard);
    buf.push(b'i');
    write_len_bytes(&mut buf, dst.as_bytes(), 1)?;
    write_len_bytes(&mut buf, etype.as_bytes(), 1)?;
    write_i64(&mut buf, rank);
    write_len_bytes(&mut buf, src.as_bytes(), 1)?;
    Ok(buf)
}
pub fn decode_in_edge_key(key: &[u8]) -> StorageResult<(u16, String, String, i64, String)> {
    let mut off = 0;
    let shard = read_u16(key, &mut off)?;
    if key.len() < off + 1 || key[off] != b'i' {
        return Err(StorageError::CodecError("not in key".into()));
    }
    off += 1;
    let d = read_len_bytes(key, &mut off, 1)?;
    let e = read_len_bytes(key, &mut off, 1)?;
    let r = read_i64(key, &mut off)?;
    let s = read_len_bytes(key, &mut off, 1)?;
    Ok((
        shard,
        String::from_utf8_lossy(d).into_owned(),
        String::from_utf8_lossy(e).into_owned(),
        r,
        String::from_utf8_lossy(s).into_owned(),
    ))
}

// Edge key prefix：用于扫描某顶点的出边/入边
pub fn out_edge_prefix(shard: u16, src: &str) -> StorageResult<Vec<u8>> {
    let mut buf = Vec::with_capacity(32);
    write_u16(&mut buf, shard);
    buf.push(b'o');
    write_len_bytes(&mut buf, src.as_bytes(), 1)?;
    Ok(buf)
}
pub fn in_edge_prefix(shard: u16, dst: &str) -> StorageResult<Vec<u8>> {
    let mut buf = Vec::with_capacity(32);
    write_u16(&mut buf, shard);
    buf.push(b'i');
    write_len_bytes(&mut buf, dst.as_bytes(), 1)?;
    Ok(buf)
}

// ------------ Value 编码（Props）------------
pub fn encode_props(props: &BTreeMap<String, PropValue>) -> StorageResult<Vec<u8>> {
    if props.len() > u32::MAX as usize {
        return Err(StorageError::CodecError("too many props".into()));
    }
    let mut body = Vec::with_capacity(64);
    body.extend_from_slice(&(props.len() as u32).to_le_bytes());
    for (k, v) in props {
        write_len_bytes(&mut body, k.as_bytes(), 2)?;
        let tag = v.tag();
        let vb = v.encode_bytes();
        let mut combined = Vec::with_capacity(1 + vb.len());
        combined.push(tag);
        combined.extend_from_slice(&vb);
        write_len_bytes(&mut body, &combined, 2)?;
    }
    let chk = crc32c(&body);
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&chk.to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}
pub fn decode_props(buf: &[u8]) -> StorageResult<BTreeMap<String, PropValue>> {
    if buf.len() < 8 {
        return Err(StorageError::CodecError("short value".into()));
    }
    let mut off = 0;
    let expected = read_u32(buf, &mut off)?;
    let body = &buf[off..];
    let actual = crc32c(body);
    if expected != actual {
        return Err(StorageError::CodecError(format!(
            "crc32c mismatch {expected:08x}!={actual:08x}"
        )));
    }
    let mut o2 = 0;
    let count = read_u32(body, &mut o2)?;
    let mut out = BTreeMap::new();
    for _ in 0..count {
        let kb = read_len_bytes(body, &mut o2, 2)?;
        let tagged = read_len_bytes(body, &mut o2, 2)?;
        let k = String::from_utf8(kb.to_vec())
            .map_err(|e| StorageError::CodecError(format!("prop k utf8 {e}")))?;
        if tagged.is_empty() {
            return Err(StorageError::CodecError("prop val empty".into()));
        }
        let tag = tagged[0];
        let val_bytes = &tagged[1..];
        out.insert(k, PropValue::decode(tag, val_bytes)?);
    }
    Ok(out)
}

// ------------ 顶点值：tag + props ------------
pub fn encode_vertex_value(
    tag: &str,
    props: &BTreeMap<String, PropValue>,
) -> StorageResult<Vec<u8>> {
    let mut body = Vec::new();
    write_len_bytes(&mut body, tag.as_bytes(), 2)?;
    let props_bytes = encode_props(props)?;
    body.extend_from_slice(&props_bytes);
    let chk = crc32c(&body);
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&chk.to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}
pub fn decode_vertex_value(buf: &[u8]) -> StorageResult<(String, BTreeMap<String, PropValue>)> {
    if buf.len() < 4 {
        return Err(StorageError::CodecError("short vv".into()));
    }
    let mut off = 0;
    let expected = read_u32(buf, &mut off)?;
    let body = &buf[off..];
    if crc32c(body) != expected {
        return Err(StorageError::CodecError("v crc fail".into()));
    }
    let mut o2 = 0;
    let tagb = read_len_bytes(body, &mut o2, 2)?;
    let tag = String::from_utf8(tagb.to_vec())
        .map_err(|e| StorageError::CodecError(format!("tag utf8 {e}")))?;
    let props = decode_props(&body[o2..])?;
    Ok((tag, props))
}

// ------------ 边值：weight + props ------------
pub fn encode_edge_value(
    weight: Option<f64>,
    props: &BTreeMap<String, PropValue>,
) -> StorageResult<Vec<u8>> {
    let mut body = Vec::new();
    body.push(if weight.is_some() { 1 } else { 0 });
    if let Some(w) = weight {
        body.extend_from_slice(&w.to_le_bytes());
    }
    let pb = encode_props(props)?;
    body.extend_from_slice(&pb);
    let chk = crc32c(&body);
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&chk.to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}
pub fn decode_edge_value(buf: &[u8]) -> StorageResult<(Option<f64>, BTreeMap<String, PropValue>)> {
    if buf.len() < 5 {
        return Err(StorageError::CodecError("short ev".into()));
    }
    let mut off = 0;
    let expected = read_u32(buf, &mut off)?;
    let body = &buf[off..];
    if crc32c(body) != expected {
        return Err(StorageError::CodecError("e crc fail".into()));
    }
    let mut o2 = 0;
    let has_w = body[o2];
    o2 += 1;
    let w = if has_w == 1 {
        if body.len() < o2 + 8 {
            return Err(StorageError::CodecError("short w".into()));
        }
        let mut a = [0u8; 8];
        a.copy_from_slice(&body[o2..o2 + 8]);
        o2 += 8;
        Some(f64::from_le_bytes(a))
    } else {
        None
    };
    let props = decode_props(&body[o2..])?;
    Ok((w, props))
}

// --- VID 分片哈希：sha256(u64 LE mod N) ---
use sha2::{Digest, Sha256};
pub fn vid_hash_shard(vid: &str, shard_count: u16) -> u16 {
    assert!(
        shard_count.is_power_of_two(),
        "shard_count must be power of two"
    );
    let mut h = Sha256::new();
    h.update(vid.as_bytes());
    let d = h.finalize();
    // 取 8 字节 → u64 → mod (使用位运算，因为 N=2^n)
    let mut a = [0u8; 8];
    a.copy_from_slice(&d[..8]);
    let v = u64::from_le_bytes(a);
    (v & (shard_count as u64 - 1)) as u16
}

pub fn vid_hash_u64(vid: &str) -> u64 {
    let mut h = Sha256::new();
    h.update(vid.as_bytes());
    let d = h.finalize();
    let mut a = [0u8; 8];
    a.copy_from_slice(&d[..8]);
    u64::from_le_bytes(a)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crc32c_basic() {
        // 已知向量：crc32c("123456789") = 0xE3069283
        let got = crc32c(b"123456789");
        assert_eq!(got, 0xE306_9283);
    }
}
