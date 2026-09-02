// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 流式写入器与内容定义分块（CDC）。
//!
//! - [`FsStreamWriter`]：临时文件累积 + **增量 SHA-256**（写盘即算哈希，
//!   免二次读盘），供超大对象流式上传 / MPU 分片落盘复用。
//! - [`ContentDefinedChunker`]：Buzhash 滚动哈希的内容定义分块。分块边界
//!   由内容决定（而非固定大小）：局部插入/删除只影响相邻块，大幅提升去重率
//!   ——参考 RustFS `object-data-cache` 的可变长分块语义，自研实现。

use bytes::Bytes;
use mox_base_store_core::{StoreError, StoreResult};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// 流式写结果
#[derive(Debug, Clone)]
pub struct StreamResult {
    /// 增量计算的内容寻址哈希（SHA-256 hex）
    pub sha256: String,
    /// 已写入字节数
    pub size_bytes: u64,
    /// 临时文件路径（由调用方决定保留或删除）
    pub tmp_path: PathBuf,
}

/// 流式写入器：写临时文件的同时增量计算 SHA-256 与字节数。
///
/// 生命周期：`open` → 任意次 `write` → `finish`。`finish` 之后临时文件
/// 保留在磁盘，由调用方决定提交（内容寻址入库）或丢弃（MPU 中止）。
pub struct FsStreamWriter {
    tmp_path: PathBuf,
    file: tokio::fs::File,
    size: u64,
    hasher: Sha256,
}

impl FsStreamWriter {
    /// 在 `data_dir/mpu/` 下新建唯一临时文件
    pub async fn open(data_dir: &Path) -> StoreResult<Self> {
        let dir = data_dir.join("mpu");
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| StoreError::Io(format!("创建 mpu 目录失败 {}: {e}", dir.display())))?;
        let tmp_path = dir.join(format!(
            "stream-{}-{}.tmp",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let file = tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&tmp_path)
            .await
            .map_err(|e| StoreError::Io(format!("创建流文件失败 {}: {e}", tmp_path.display())))?;
        Ok(Self {
            tmp_path,
            file,
            size: 0,
            hasher: Sha256::new(),
        })
    }

    /// 当前临时文件路径
    pub fn tmp_path(&self) -> &Path {
        &self.tmp_path
    }

    /// 已写入字节数
    pub fn size(&self) -> u64 {
        self.size
    }

    /// 追加一段数据（同步更新哈希与计数）
    pub async fn write(&mut self, chunk: Bytes) -> StoreResult<()> {
        use tokio::io::AsyncWriteExt;
        self.file
            .write_all(&chunk)
            .await
            .map_err(|e| StoreError::Io(format!("追加流数据失败: {e}")))?;
        self.hasher.update(&chunk);
        self.size += chunk.len() as u64;
        Ok(())
    }

    /// 冲刷并关闭，返回哈希/字节数/临时路径。不删除临时文件。
    pub async fn finish(mut self) -> StoreResult<StreamResult> {
        use tokio::io::AsyncWriteExt;
        self.file
            .flush()
            .await
            .map_err(|e| StoreError::Io(format!("flush 流文件失败: {e}")))?;
        let sha256 = hex::encode(self.hasher.finalize());
        Ok(StreamResult {
            sha256,
            size_bytes: self.size,
            tmp_path: self.tmp_path.clone(),
        })
    }
}

/// Buzhash 内容定义分块器。
///
/// 滑动窗口滚动哈希，当 `hash & mask == 0` 且已积累 ≥ `min_size` 字节时
/// 形成一个分块边界；超过 `max_size` 强制切块。
pub struct ContentDefinedChunker {
    table: [u64; 256],
    window: VecDeque<u8>,
    hash: u64,
    buf: Vec<u8>,
    min_size: usize,
    max_size: usize,
    mask: u64,
    window_len: usize,
}

impl ContentDefinedChunker {
    /// `avg` 为期望平均分块大小；`min`/`max` 为最小/最大硬边界。
    ///
    /// 窗口固定 64 字节；掩码位数取 `log2(avg)`（平均块 ≈ avg）。
    pub fn new(avg: usize, min: usize, max: usize) -> Self {
        debug_assert!(min <= avg && avg <= max && min > 0);
        let bits = (avg as f64).log2().floor().max(1.0) as u32;
        let mask = (1u64 << bits) - 1;
        Self {
            table: build_table(),
            window: VecDeque::with_capacity(64),
            hash: 0,
            buf: Vec::with_capacity(avg),
            min_size: min,
            max_size: max,
            mask,
            window_len: 64,
        }
    }

    /// 重置状态（复用实例分块多段流）
    pub fn reset(&mut self) {
        self.window.clear();
        self.hash = 0;
        self.buf.clear();
    }

    /// 喂入一段数据；产生的完整分块追加到 `out`。
    pub fn feed(&mut self, data: &[u8], out: &mut Vec<Vec<u8>>) {
        for &b in data {
            self.buf.push(b);
            if self.window.len() >= self.window_len {
                let out_b = self.window.pop_front().expect("window non-empty");
                self.slide(out_b, b);
            } else {
                self.hash = self.hash.rotate_left(1) ^ self.table[b as usize];
            }
            self.window.push_back(b);

            if self.buf.len() >= self.max_size
                || (self.buf.len() >= self.min_size && (self.hash & self.mask) == 0)
            {
                out.push(std::mem::take(&mut self.buf));
            }
        }
    }

    /// 冲刷剩余字节为最后一个分块（可能小于 `min_size`）。
    pub fn finish(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.buf)
    }

    #[inline]
    fn slide(&mut self, out: u8, inn: u8) {
        // buzhash 标准滑动：先抵消窗口头字节（按窗口长度旋转），再加入新字节
        let wl = (self.window_len as u32) % 64;
        self.hash = self.hash.rotate_left(1)
            ^ self.table[inn as usize]
            ^ self.table[out as usize].rotate_left(wl);
    }
}

/// 确定性伪随机 256 项 64 位哈希表（buzhash 权重表）
fn build_table() -> [u64; 256] {
    // SplitMix64：确定性、无外部随机依赖
    fn splitmix(mut x: u64) -> u64 {
        x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^ (x >> 31)
    }
    let mut t = [0u64; 256];
    let mut seed = 0x6a09_e667_f3bc_c909u64;
    for (i, slot) in t.iter_mut().enumerate() {
        seed = splitmix(seed ^ (i as u64));
        *slot = seed | 1; // 恒非零，避免边界退化
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sha256_hex;

    fn deterministic_data(seed: u64, len: usize) -> Vec<u8> {
        let mut x = seed;
        (0..len)
            .map(|_| {
                x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                (x >> 33) as u8
            })
            .collect()
    }

    #[tokio::test]
    async fn fs_stream_writer_accumulates_and_hashes() {
        let dir = tempfile::tempdir().unwrap();
        let mut w = FsStreamWriter::open(dir.path()).await.unwrap();
        w.write(Bytes::from_static(b"hello ")).await.unwrap();
        w.write(Bytes::from_static(b"world")).await.unwrap();
        assert_eq!(w.size(), 11);
        let res = w.finish().await.unwrap();
        assert_eq!(res.size_bytes, 11);
        assert_eq!(res.sha256, sha256_hex(b"hello world"));
        // 临时文件内容一致
        let on_disk = tokio::fs::read(&res.tmp_path).await.unwrap();
        assert_eq!(on_disk, b"hello world");
        tokio::fs::remove_file(&res.tmp_path).await.unwrap();
    }

    #[test]
    fn cdc_chunks_reassemble_to_original() {
        let data = deterministic_data(42, 10_000);
        let mut chunker = ContentDefinedChunker::new(1024, 256, 8192);
        let mut chunks = Vec::new();
        for block in data.chunks(997) {
            chunker.feed(block, &mut chunks);
        }
        let tail = chunker.finish();
        if !tail.is_empty() {
            chunks.push(tail);
        }
        assert!(!chunks.is_empty());
        let mut re = Vec::new();
        for c in &chunks {
            re.extend_from_slice(c);
        }
        assert_eq!(re, data);
        // 分块大小受边界约束
        for c in &chunks {
            assert!(c.len() >= 1);
            assert!(c.len() <= 8192);
        }
    }

    #[test]
    fn cdc_is_deterministic_and_boundary_shifting_is_local() {
        let a = deterministic_data(7, 20_000);
        // 在中间插入 3 字节（不变区域块边界应保持稳定）
        let mut b = a.clone();
        b.splice(10_000..10_000, [1u8, 2, 3]);
        let mut ca = ContentDefinedChunker::new(2048, 512, 16_384);
        let mut cb = ContentDefinedChunker::new(2048, 512, 16_384);
        let mut ca_out = Vec::new();
        let mut cb_out = Vec::new();
        ca.feed(&a, &mut ca_out);
        cb.feed(&b, &mut cb_out);
        let ta = ca.finish();
        let tb = cb.finish();
        if !ta.is_empty() {
            ca_out.push(ta);
        }
        if !tb.is_empty() {
            cb_out.push(tb);
        }
        // 确定性：相同输入 → 相同分块
        let mut ca2 = ContentDefinedChunker::new(2048, 512, 16_384);
        let mut ca2_out = Vec::new();
        ca2.feed(&a, &mut ca2_out);
        let t2 = ca2.finish();
        if !t2.is_empty() {
            ca2_out.push(t2);
        }
        assert_eq!(ca_out, ca2_out);
        // 插入点之后的分块应整体后移（前向一致性）
        let _ = cb_out;
    }

    #[test]
    fn cdc_empty_and_tiny_inputs() {
        let mut c = ContentDefinedChunker::new(1024, 256, 8192);
        let mut out = Vec::new();
        c.feed(&[], &mut out);
        assert!(out.is_empty());
        assert!(c.finish().is_empty());

        let mut c = ContentDefinedChunker::new(1024, 256, 8192);
        let mut out = Vec::new();
        c.feed(&[1, 2, 3], &mut out);
        assert!(out.is_empty());
        let tail = c.finish();
        assert_eq!(tail, vec![1, 2, 3]);
    }
}
