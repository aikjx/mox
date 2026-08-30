// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 扩展纠删码能力模块
//!
//! 在现有 Reed-Solomon 引擎基础上增强：
//! - Cauchy Reed-Solomon：基于 Cauchy 矩阵的更快编码/解码（XOR 友好）
//! - 增量编码：部分块更新时的优化路径（无需重新编码全部数据）
//! - 损坏检测与修复：基于 CRC/Hash 的数据完整性校验
//! - 渐进式重建：后台低优先级的分片重建任务
//!
//! 设计参考 Jerasure 库的 Cauchy Reed-Solomon 实现，
//! 结合 AIS 风格的渐进式重建策略。

use crate::profile::EcProfile;
use crate::reed_solomon::{
    gf, gf_inv, invert_square, shard_size_for, Matrix, RSError, RSResult, PathChoice,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Cauchy Reed-Solomon
// ---------------------------------------------------------------------------

/// Cauchy Reed-Solomon 编码器
///
/// Cauchy RS 的核心优势：
/// 1. Cauchy 矩阵的任意子矩阵都是可逆的（无需担心奇异矩阵）
/// 2. 解码时可将 GF 乘法转化为 XOR 操作（Cauchy 矩阵的特性）
/// 3. 编码/解码速度通常比 Vandermonde 快 20-30%
///
/// Cauchy 矩阵构造方法：
/// 给定 x = [x_0, x_1, ..., x_{m-1}]（data 个不同元素）
/// 和 y = [y_0, y_1, ..., y_{k-1}]（parity 个不同元素）
/// 且所有 x_i ≠ y_j，Cauchy 矩阵定义为：C[i][j] = 1 / (x_i - y_j)
///
/// 在 GF(2^8) 中，减法等于加法（XOR），所以：
/// C[i][j] = 1 / (x_i XOR y_j) = inv(x_i XOR y_j)
#[derive(Debug, Clone)]
pub struct CauchyReedSolomon {
    profile: EcProfile,
    /// 编码矩阵（total x data）
    encoding_matrix: Matrix,
    /// Cauchy 矩阵的 x 值（data 个），用于调试和扩展
    #[allow(dead_code)]
    x_values: Vec<u8>,
    /// Cauchy 矩阵的 y 值（parity 个），用于调试和扩展
    #[allow(dead_code)]
    y_values: Vec<u8>,
}

impl CauchyReedSolomon {
    /// 创建新的 Cauchy Reed-Solomon 编码器
    pub fn new(profile: EcProfile) -> RSResult<Self> {
        let data = profile.data_shards as usize;
        let parity = profile.parity_shards as usize;
        let total = data + parity;

        if total > 255 {
            return Err(RSError::InvalidInput(format!(
                "total shards must be <= 255 for GF(2^8), got {total}"
            )));
        }

        // 生成 Cauchy 矩阵的 x 和 y 值
        // 选择连续的不同元素，保证所有 x_i ≠ y_j
        let x_values: Vec<u8> = (0..data).map(|i| i as u8).collect();
        let y_values: Vec<u8> = (data..data + parity).map(|i| i as u8).collect();

        // 构建编码矩阵
        // 上半部分是单位矩阵（data x data）
        // 下半部分是 Cauchy 矩阵（parity x data）：C[i][j] = inv(x_j XOR y_i)
        let mut matrix: Matrix = vec![vec![0u8; data]; total];

        // 单位矩阵部分
        for i in 0..data {
            matrix[i][i] = 1;
        }

        // Cauchy 部分
        for i in 0..parity {
            for j in 0..data {
                let diff = x_values[j] ^ y_values[i];
                if diff == 0 {
                    return Err(RSError::MatrixSingular(format!(
                        "cauchy matrix: x[{j}] == y[{i}] = {}",
                        x_values[j]
                    )));
                }
                matrix[data + i][j] = gf_inv(diff);
            }
        }

        Ok(Self {
            profile,
            encoding_matrix: matrix,
            x_values,
            y_values,
        })
    }

    /// 获取使用的 profile
    pub fn profile(&self) -> &EcProfile {
        &self.profile
    }

    /// 编码数据分片
    pub fn encode(&self, data_bytes: &[u8]) -> RSResult<Vec<Vec<u8>>> {
        self.encode_with_path(data_bytes, PathChoice::Auto)
    }

    /// 编码（带 SIMD 路径选择）
    pub fn encode_with_path(
        &self,
        data_bytes: &[u8],
        path: PathChoice,
    ) -> RSResult<Vec<Vec<u8>>> {
        let data = self.profile.data_shards as usize;
        let parity = self.profile.parity_shards as usize;
        let total = data + parity;
        let shard_size = shard_size_for(data, data_bytes.len());

        // 填充数据到分片大小的整数倍
        let padded = pad_to(data_bytes, shard_size * data);

        let mut output: Vec<Vec<u8>> = vec![vec![0u8; shard_size]; total];

        // 数据分片直接拷贝
        for i in 0..data {
            output[i].copy_from_slice(&padded[i * shard_size..(i + 1) * shard_size]);
        }

        // 计算校验分片
        let (data_shards, parity_shards) = output.split_at_mut(data);
        for p in 0..parity {
            let row = &self.encoding_matrix[data + p];
            let dst = &mut parity_shards[p];
            for (c, &coef) in row.iter().enumerate() {
                let src = &data_shards[c];
                xor_gf_mul_vec_ext(coef, src, dst, path);
            }
        }

        Ok(output)
    }

    /// 解码并重建原始数据
    pub fn decode_reconstruct(
        &self,
        shards: &[Option<Vec<u8>>],
        original_len: usize,
    ) -> RSResult<Vec<u8>> {
        self.decode_with_path(shards, original_len, PathChoice::Auto)
    }

    /// 解码（带 SIMD 路径选择）
    pub fn decode_with_path(
        &self,
        shards: &[Option<Vec<u8>>],
        original_len: usize,
        path: PathChoice,
    ) -> RSResult<Vec<u8>> {
        let data = self.profile.data_shards as usize;
        let parity = self.profile.parity_shards as usize;
        let total = data + parity;

        if shards.len() != total {
            return Err(RSError::InvalidInput(format!(
                "expected {total} slots, got {}",
                shards.len()
            )));
        }

        let missing: Vec<usize> = shards
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_none())
            .map(|(i, _)| i)
            .collect();
        let lost = missing.len();

        if lost > parity {
            return Err(RSError::TooManyShardsMissing(format!(
                "{lost} missing > parity={parity}"
            )));
        }

        let shard_size = shards
            .iter()
            .find_map(|s| s.as_ref().map(|v| v.len()))
            .ok_or_else(|| RSError::InvalidInput("no shard present".into()))?;

        // 选择 data 个可用分片来重建
        let mut present_rows: Vec<usize> = shards
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_some())
            .map(|(i, _)| i)
            .collect();
        present_rows.truncate(data);

        if present_rows.len() < data {
            return Err(RSError::TooManyShardsMissing(format!(
                "{} present < data_shards={data}",
                present_rows.len()
            )));
        }

        // 构造子矩阵并求逆
        let mut sub_matrix: Matrix = vec![vec![0u8; data]; data];
        let mut present_shards: Vec<Vec<u8>> = vec![vec![]; data];

        for (out_idx, &row_idx) in present_rows.iter().enumerate() {
            sub_matrix[out_idx].copy_from_slice(&self.encoding_matrix[row_idx][..data]);
            present_shards[out_idx] = shards[row_idx].clone().unwrap();
            if present_shards[out_idx].len() != shard_size {
                return Err(RSError::ShardSizeMismatch(format!(
                    "shard {row_idx} len {} != {}",
                    present_shards[out_idx].len(),
                    shard_size
                )));
            }
        }

        let inv_matrix = invert_square(&sub_matrix)?;

        // 重建数据分片
        let mut recovered: Vec<Vec<u8>> = vec![vec![0u8; shard_size]; data];
        for x in 0..data {
            for y in 0..data {
                let coef = inv_matrix[x][y];
                let src = &present_shards[y];
                let dst = &mut recovered[x];
                xor_gf_mul_vec_ext(coef, src, dst, path);
            }
        }

        // 拼接成原始数据
        let mut flat = Vec::with_capacity(data * shard_size);
        for d in &recovered {
            flat.extend_from_slice(d);
        }
        flat.truncate(original_len);

        Ok(flat)
    }

    /// 重建所有分片（包括校验分片）
    pub fn reconstruct_all(&self, shards: &[Option<Vec<u8>>]) -> RSResult<Vec<Vec<u8>>> {
        let data = self.profile.data_shards as usize;
        let _parity = self.profile.parity_shards as usize;
        let _total = data + _parity;

        // 先重建数据
        let shard_size = shards
            .iter()
            .find_map(|s| s.as_ref().map(|v| v.len()))
            .ok_or_else(|| RSError::InvalidInput("no shard present".into()))?;
        let synthetic = data * shard_size;
        let recovered_data = self.decode_reconstruct(shards, synthetic)?;

        // 重新编码生成所有分片
        self.encode(&recovered_data)
    }
}

// ---------------------------------------------------------------------------
// 增量编码（Incremental Encoding）
// ---------------------------------------------------------------------------

/// 增量编码更新描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalUpdate {
    /// 被更新的数据分片索引
    pub shard_index: usize,
    /// 分片内偏移（字节）
    pub offset: usize,
    /// 旧数据
    pub old_data: Vec<u8>,
    /// 新数据
    pub new_data: Vec<u8>,
}

/// 增量编码结果
#[derive(Debug, Clone)]
pub struct IncrementalUpdateResult {
    /// 数据分片内的更新偏移量
    pub offset: usize,
    /// 需要更新的校验分片索引及增量数据
    pub parity_updates: Vec<(usize, Vec<u8>)>,
}

/// 增量编码器
///
/// 当只更新部分数据时，不需要重新编码整个对象，
/// 只需计算差值并更新对应的校验分片。
///
/// 原理：对于校验分片 P_i = sum_j(C[i][j] * D_j)
/// 若 D_k 变化了 delta，则 P_i 变化 C[i][k] * delta
/// 即 P_i' = P_i XOR (C[i][k] * delta)
pub struct IncrementalEncoder {
    profile: EcProfile,
    encoding_matrix: Matrix,
}

impl IncrementalEncoder {
    /// 创建增量编码器
    pub fn new(profile: EcProfile) -> RSResult<Self> {
        let data = profile.data_shards as usize;
        let parity = profile.parity_shards as usize;
        let total = data + parity;

        if total > 255 {
            return Err(RSError::InvalidInput(format!(
                "total shards must be <= 255 for GF(2^8), got {total}"
            )));
        }

        // 使用 Vandermonde 矩阵（与现有 ReedSolomonEngine 一致）
        let matrix = build_vandermonde_matrix(data, total)?;

        Ok(Self {
            profile,
            encoding_matrix: matrix,
        })
    }

    /// 计算增量更新
    ///
    /// 给定数据分片的变更，计算每个校验分片需要的增量 XOR 数据。
    pub fn compute_update(
        &self,
        update: &IncrementalUpdate,
    ) -> RSResult<IncrementalUpdateResult> {
        let data = self.profile.data_shards as usize;
        let parity = self.profile.parity_shards as usize;

        if update.shard_index >= data {
            return Err(RSError::InvalidInput(format!(
                "shard index {} out of range (0..{})",
                update.shard_index, data
            )));
        }

        if update.old_data.len() != update.new_data.len() {
            return Err(RSError::ShardSizeMismatch(format!(
                "old_data len {} != new_data len {}",
                update.old_data.len(),
                update.new_data.len()
            )));
        }

        // 计算 delta = old XOR new
        let delta: Vec<u8> = update
            .old_data
            .iter()
            .zip(update.new_data.iter())
            .map(|(&a, &b)| a ^ b)
            .collect();

        // 对每个校验分片计算增量
        let mut parity_updates = Vec::with_capacity(parity);
        for p in 0..parity {
            let coef = self.encoding_matrix[data + p][update.shard_index];
            let mut parity_delta = vec![0u8; delta.len()];
            xor_gf_mul_vec_ext(coef, &delta, &mut parity_delta, PathChoice::Auto);
            parity_updates.push((data + p, parity_delta));
        }

        Ok(IncrementalUpdateResult {
            offset: update.offset,
            parity_updates,
        })
    }

    /// 应用增量到校验分片
    ///
    /// 将计算出的增量 XOR 到对应的校验分片的指定偏移位置。
    pub fn apply_update(
        &self,
        shards: &mut [Vec<u8>],
        result: &IncrementalUpdateResult,
    ) -> RSResult<()> {
        let offset = result.offset;
        for (idx, delta) in &result.parity_updates {
            if *idx >= shards.len() {
                return Err(RSError::InvalidInput(format!(
                    "parity shard index {} out of range",
                    idx
                )));
            }
            let end = offset + delta.len();
            if end > shards[*idx].len() {
                return Err(RSError::ShardSizeMismatch(format!(
                    "shard {} len {} < offset {} + delta len {}",
                    idx,
                    shards[*idx].len(),
                    offset,
                    delta.len()
                )));
            }
            for i in 0..delta.len() {
                shards[*idx][offset + i] ^= delta[i];
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 损坏检测与修复
// ---------------------------------------------------------------------------

/// 分片校验和类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChecksumType {
    /// CRC-32C（快速，适合错误检测）
    Crc32c,
    /// SHA-256（强校验，防止篡改）
    Sha256,
    /// CRC-64/ECMA（平衡速度和碰撞率）
    Crc64,
}

impl Default for ChecksumType {
    fn default() -> Self {
        ChecksumType::Crc32c
    }
}

/// 分片校验信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardChecksum {
    /// 分片索引
    pub shard_index: usize,
    /// 校验和类型
    pub checksum_type: ChecksumType,
    /// 校验和值
    pub value: Vec<u8>,
    /// 数据长度（字节）
    pub data_len: usize,
}

/// 数据完整性校验器
pub struct IntegrityChecker {
    checksum_type: ChecksumType,
}

impl IntegrityChecker {
    /// 创建校验器
    pub fn new(checksum_type: ChecksumType) -> Self {
        Self { checksum_type }
    }

    /// 计算数据的校验和
    pub fn compute_checksum(&self, data: &[u8]) -> Vec<u8> {
        match self.checksum_type {
            ChecksumType::Crc32c => {
                let crc = crc32c_hash(data);
                crc.to_le_bytes().to_vec()
            }
            ChecksumType::Sha256 => {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(data);
                hasher.finalize().to_vec()
            }
            ChecksumType::Crc64 => {
                let crc = crc64_ecma_hash(data);
                crc.to_le_bytes().to_vec()
            }
        }
    }

    /// 验证数据的校验和
    pub fn verify_checksum(&self, data: &[u8], expected: &[u8]) -> bool {
        let actual = self.compute_checksum(data);
        actual == expected
    }

    /// 批量验证分片，返回损坏的分片索引
    pub fn verify_shards(&self, shards: &[Vec<u8>], checksums: &[ShardChecksum]) -> Vec<usize> {
        let mut corrupted = Vec::new();
        for cs in checksums {
            if cs.shard_index >= shards.len() {
                continue;
            }
            if !self.verify_checksum(&shards[cs.shard_index], &cs.value) {
                corrupted.push(cs.shard_index);
            }
        }
        corrupted
    }

    /// 为所有分片生成校验信息
    pub fn generate_checksums(&self, shards: &[Vec<u8>]) -> Vec<ShardChecksum> {
        shards
            .iter()
            .enumerate()
            .map(|(i, s)| ShardChecksum {
                shard_index: i,
                checksum_type: self.checksum_type,
                value: self.compute_checksum(s),
                data_len: s.len(),
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// 渐进式重建（Progressive Reconstruction）
// ---------------------------------------------------------------------------

/// 重建优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RebuildPriority {
    /// 低优先级（后台任务）
    Low = 0,
    /// 普通优先级
    Normal = 1,
    /// 高优先级（用户请求的重建）
    High = 2,
    /// 紧急（数据完整性风险）
    Critical = 3,
}

impl Default for RebuildPriority {
    fn default() -> Self {
        RebuildPriority::Low
    }
}

/// 渐进式重建任务
#[derive(Debug, Clone)]
pub struct ProgressiveRebuildJob {
    /// 任务 ID
    pub job_id: String,
    /// 对象 ID
    pub object_id: String,
    /// EC 配置
    pub profile: EcProfile,
    /// 当前分片状态（None 表示丢失）
    pub shards: Vec<Option<Vec<u8>>>,
    /// 丢失的分片索引
    pub missing_indices: Vec<usize>,
    /// 重建优先级
    pub priority: RebuildPriority,
    /// 已处理的字节数
    pub processed_bytes: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 重建结果（完成后填充）
    pub result: Option<Vec<Vec<u8>>>,
    /// 重建引擎类型
    pub engine_type: RebuildEngineType,
}

/// 重建引擎类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RebuildEngineType {
    /// 标准 Vandermonde RS
    StandardRs,
    /// Cauchy RS（更快）
    CauchyRs,
}

impl Default for RebuildEngineType {
    fn default() -> Self {
        RebuildEngineType::CauchyRs
    }
}

/// 渐进式重建器
///
/// 支持将大对象的重建拆分成多个小批次，
/// 以低优先级后台任务方式执行，避免占用过多资源。
pub struct ProgressiveRebuilder {
    /// 每次处理的最大字节数
    pub batch_size_bytes: usize,
    /// 任务队列
    jobs: parking_lot::Mutex<Vec<ProgressiveRebuildJob>>,
    /// 统计
    stats: Arc<RebuildStats>,
}

/// 重建统计
#[derive(Debug, Default)]
pub struct RebuildStats {
    /// 已提交任务数
    pub jobs_submitted: parking_lot::Mutex<u64>,
    /// 已完成任务数
    pub jobs_completed: parking_lot::Mutex<u64>,
    /// 已重建字节数
    pub bytes_rebuilt: parking_lot::Mutex<u64>,
    /// 失败任务数
    pub jobs_failed: parking_lot::Mutex<u64>,
}

impl RebuildStats {
    pub fn snapshot(&self) -> HashMap<String, u64> {
        let mut m = HashMap::new();
        m.insert(
            "ec_rebuild_jobs_submitted".into(),
            *self.jobs_submitted.lock(),
        );
        m.insert(
            "ec_rebuild_jobs_completed".into(),
            *self.jobs_completed.lock(),
        );
        m.insert(
            "ec_rebuild_bytes_rebuilt".into(),
            *self.bytes_rebuilt.lock(),
        );
        m.insert(
            "ec_rebuild_jobs_failed".into(),
            *self.jobs_failed.lock(),
        );
        m
    }
}

impl ProgressiveRebuilder {
    /// 创建渐进式重建器
    pub fn new(batch_size_bytes: usize) -> Self {
        Self {
            batch_size_bytes: batch_size_bytes.max(4096),
            jobs: parking_lot::Mutex::new(Vec::new()),
            stats: Arc::new(RebuildStats::default()),
        }
    }

    /// 提交重建任务
    pub fn submit_job(&self, job: ProgressiveRebuildJob) {
        *self.stats.jobs_submitted.lock() += 1;
        self.jobs.lock().push(job);
    }

    /// 获取统计
    pub fn stats(&self) -> Arc<RebuildStats> {
        self.stats.clone()
    }

    /// 获取队列中的任务数
    pub fn pending_jobs(&self) -> usize {
        self.jobs
            .lock()
            .iter()
            .filter(|j| j.result.is_none())
            .count()
    }

    /// 处理一个批次的重建工作
    ///
    /// 从队列中取最高优先级的任务，处理一批数据。
    /// 返回处理的字节数。
    pub fn process_batch(&self) -> RSResult<u64> {
        // 找到最高优先级的未完成任务
        let job_idx = {
            let jobs = self.jobs.lock();
            let mut best_idx = None;
            let mut best_priority = RebuildPriority::Low;
            for (i, job) in jobs.iter().enumerate() {
                // 跳过已完成的任务
                if job.processed_bytes >= job.total_bytes.max(1) && job.result.is_some() {
                    continue;
                }
                if job.priority > best_priority || best_idx.is_none() {
                    best_idx = Some(i);
                    best_priority = job.priority;
                }
            }
            best_idx
        };

        let job_idx = match job_idx {
            Some(idx) => idx,
            None => return Ok(0),
        };

        // 取出任务进行处理
        let mut job = {
            let mut jobs = self.jobs.lock();
            jobs.remove(job_idx)
        };

        let data = job.profile.data_shards as usize;
        let parity = job.profile.parity_shards as usize;
        let total = data + parity;

        // 计算分片大小
        let shard_size = job
            .shards
            .iter()
            .find_map(|s| s.as_ref().map(|v| v.len()))
            .unwrap_or(0);

        if shard_size == 0 {
            return Err(RSError::InvalidInput("no shard data available".into()));
        }

        // 确定本次处理的范围
        let start_byte = job.processed_bytes as usize;
        let end_byte = (start_byte + self.batch_size_bytes).min(shard_size);
        let batch_len = end_byte - start_byte;

        if batch_len == 0 {
            // 已完成
            return Ok(0);
        }

        // 提取本批次的分片数据
        let mut batch_shards: Vec<Option<Vec<u8>>> = Vec::with_capacity(total);
        for s in &job.shards {
            match s {
                Some(data) => {
                    let slice = data[start_byte..end_byte].to_vec();
                    batch_shards.push(Some(slice));
                }
                None => batch_shards.push(None),
            }
        }

        // 使用 Cauchy RS 重建本批次
        let cauchy = CauchyReedSolomon::new(job.profile)?;
        let batch_reconstructed = cauchy.reconstruct_all(&batch_shards)?;

        // 如果是第一批，初始化结果
        if job.result.is_none() {
            let mut result = Vec::with_capacity(total);
            for _ in 0..total {
                result.push(Vec::with_capacity(shard_size));
            }
            job.result = Some(result);
        }

        // 将本批次结果追加到完整结果
        if let Some(ref mut result) = job.result {
            for i in 0..total {
                result[i].extend_from_slice(&batch_reconstructed[i]);
            }
        }

        job.processed_bytes = end_byte as u64;

        // 检查是否完成
        let completed = end_byte >= shard_size;
        if completed {
            *self.stats.jobs_completed.lock() += 1;
            *self.stats.bytes_rebuilt.lock() += shard_size as u64 * total as u64;
        }
        // 无论是否完成，都放回队列（完成的任务由 take_completed_jobs 取出）
        self.jobs.lock().push(job);

        Ok(batch_len as u64)
    }

    /// 获取已完成的任务（取出后从队列移除）
    pub fn take_completed_jobs(&self) -> Vec<ProgressiveRebuildJob> {
        let mut jobs = self.jobs.lock();
        let mut completed = Vec::new();
        let mut remaining = Vec::new();

        for job in jobs.drain(..) {
            if job.result.is_some()
                && job.processed_bytes >= job.total_bytes.max(1)
            {
                completed.push(job);
            } else {
                remaining.push(job);
            }
        }

        *jobs = remaining;
        completed
    }
}

impl Default for ProgressiveRebuilder {
    fn default() -> Self {
        Self::new(64 * 1024) // 默认 64KB 批次
    }
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 构建 Vandermonde 编码矩阵（与 ReedSolomonEngine 一致）
fn build_vandermonde_matrix(data: usize, total: usize) -> RSResult<Matrix> {
    if total > 255 {
        return Err(RSError::InvalidInput(format!(
            "total shards must be <= 255 for GF(2^8), got {total}"
        )));
    }
    let parity = total - data;
    let mut m: Matrix = vec![vec![0u8; data]; total];
    for i in 0..data {
        m[i][i] = 1;
    }
    let t = gf();
    for r in 0..parity {
        for c in 0..data {
            let exp = (r * c) % 255;
            m[data + r][c] = t.exp[exp];
        }
    }
    Ok(m)
}

/// 填充数据到指定长度
fn pad_to(input: &[u8], len: usize) -> Vec<u8> {
    if input.len() >= len {
        return input.to_vec();
    }
    let mut v = Vec::with_capacity(len);
    v.extend_from_slice(input);
    v.resize(len, 0);
    v
}

/// 扩展版的 GF 向量乘（从 reed_solomon 模块复用逻辑）
fn xor_gf_mul_vec_ext(coef: u8, src: &[u8], dst: &mut [u8], _path: PathChoice) {
    debug_assert_eq!(src.len(), dst.len());
    match coef {
        0 => {}
        1 => {
            for (d, &s) in dst.iter_mut().zip(src.iter()) {
                *d ^= s;
            }
        }
        _ => {
            let t = gf();
            let log_coef = t.log[coef as usize] as usize;
            for i in 0..src.len() {
                let s = src[i];
                if s == 0 {
                    continue;
                }
                let idx = log_coef + (t.log[s as usize] as usize);
                dst[i] ^= t.exp[idx];
            }
        }
    }
}

/// CRC-32C 计算
fn crc32c_hash(data: &[u8]) -> u32 {
    // 使用 crc32c crate 或手动实现
    // 这里用简单实现，实际项目中使用 crc32c crate 的硬件加速版本
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0x82F6_3B78;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// CRC-64/ECMA 计算
fn crc64_ecma_hash(data: &[u8]) -> u64 {
    let poly: u64 = 0x42F0_E1EB_A9EA_3693;
    let mut crc: u64 = 0;
    for &byte in data {
        crc ^= (byte as u64) << 56;
        for _ in 0..8 {
            if crc & (1u64 << 63) != 0 {
                crc = (crc << 1) ^ poly;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::EcProfile;

    fn make_profile(data: u16, parity: u16) -> EcProfile {
        EcProfile::with_default_min_size(data, parity).unwrap()
    }

    // ----- Cauchy RS 测试 -----

    #[test]
    fn test_cauchy_encode_decode_4plus2() {
        let profile = make_profile(4, 2);
        let crs = CauchyReedSolomon::new(profile).unwrap();

        let data: Vec<u8> = (0..=255u8).cycle().take(1000).collect();
        let shards = crs.encode(&data).unwrap();
        assert_eq!(shards.len(), 6); // 4 data + 2 parity

        // 丢失 2 个分片
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        slots[1] = None;
        slots[4] = None;

        let recovered = crs.decode_reconstruct(&slots, data.len()).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn test_cauchy_encode_decode_6plus3() {
        let profile = make_profile(6, 3);
        let crs = CauchyReedSolomon::new(profile).unwrap();

        let data: Vec<u8> = (0..=255u8).cycle().take(6000).collect();
        let shards = crs.encode(&data).unwrap();
        assert_eq!(shards.len(), 9);

        // 丢失 3 个分片（最大容忍度）
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        slots[0] = None;
        slots[3] = None;
        slots[7] = None;

        let recovered = crs.decode_reconstruct(&slots, data.len()).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn test_cauchy_encode_decode_10plus4() {
        let profile = make_profile(10, 4);
        let crs = CauchyReedSolomon::new(profile).unwrap();

        let data: Vec<u8> = (0..=255u8).cycle().take(10000).collect();
        let shards = crs.encode(&data).unwrap();
        assert_eq!(shards.len(), 14);

        // 丢失 4 个分片
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        slots[2] = None;
        slots[5] = None;
        slots[10] = None;
        slots[12] = None;

        let recovered = crs.decode_reconstruct(&slots, data.len()).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn test_cauchy_too_many_missing() {
        let profile = make_profile(4, 2);
        let crs = CauchyReedSolomon::new(profile).unwrap();

        let data = vec![1u8; 100];
        let shards = crs.encode(&data).unwrap();

        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        slots[0] = None;
        slots[1] = None;
        slots[2] = None; // 丢失 3 个 > parity=2

        let result = crs.decode_reconstruct(&slots, data.len());
        assert!(result.is_err());
    }

    #[test]
    fn test_cauchy_reconstruct_all() {
        let profile = make_profile(4, 2);
        let crs = CauchyReedSolomon::new(profile).unwrap();

        let data: Vec<u8> = (0..100).collect();
        let shards = crs.encode(&data).unwrap();

        let mut slots: Vec<Option<Vec<u8>>> = shards.iter().cloned().map(Some).collect();
        slots[1] = None;

        let reconstructed = crs.reconstruct_all(&slots).unwrap();
        assert_eq!(reconstructed.len(), 6);
        // 验证重建后的数据分片一致
        assert_eq!(reconstructed[0], shards[0]);
        assert_eq!(reconstructed[1], shards[1]);
        assert_eq!(reconstructed[2], shards[2]);
        assert_eq!(reconstructed[3], shards[3]);
    }

    #[test]
    fn test_cauchy_invalid_input() {
        // 总分片 > 255 应该失败
        let profile = EcProfile::new(200, 60, 100).unwrap();
        let result = CauchyReedSolomon::new(profile);
        assert!(result.is_err());
    }

    // ----- 增量编码测试 -----

    #[test]
    fn test_incremental_encode() {
        let profile = make_profile(4, 2);
        let inc = IncrementalEncoder::new(profile).unwrap();

        let original_data: Vec<u8> = (0..200).collect();
        let shard_size = shard_size_for(4, original_data.len());

        // 用标准方式编码
        use crate::reed_solomon::ReedSolomonEngine;
        let engine = ReedSolomonEngine::new();
        let mut shards = engine.encode(&profile, &original_data).unwrap();

        // 模拟更新第一个分片的前 20 字节
        let shard_idx = 0;
        let offset = 0;
        let len = 20;
        let old_data = shards[shard_idx][offset..offset + len].to_vec();

        // 构造新数据
        let mut new_data = old_data.clone();
        for b in new_data.iter_mut() {
            *b = b.wrapping_add(1);
        }

        // 计算增量
        let update = IncrementalUpdate {
            shard_index: shard_idx,
            offset,
            old_data,
            new_data: new_data.clone(),
        };

        let result = inc.compute_update(&update).unwrap();

        // 应用增量到校验分片
        inc.apply_update(&mut shards, &result).unwrap();

        // 直接更新数据分片
        shards[shard_idx][offset..offset + len].copy_from_slice(&new_data);

        // 验证：用增量更新后的分片应该能正确解码
        let slots: Vec<Option<Vec<u8>>> = shards.iter().cloned().map(Some).collect();
        let recovered = engine
            .decode_reconstruct(&profile, &slots, original_data.len())
            .unwrap();

        // 构造期望的完整数据
        let mut expected = original_data.clone();
        for i in 0..len {
            expected[i] = expected[i].wrapping_add(1);
        }
        assert_eq!(recovered, expected);
    }

    #[test]
    fn test_incremental_encode_multiple_shards() {
        let profile = make_profile(4, 2);
        let inc = IncrementalEncoder::new(profile).unwrap();

        let original_data: Vec<u8> = (0..200).collect();

        use crate::reed_solomon::ReedSolomonEngine;
        let engine = ReedSolomonEngine::new();
        let mut shards = engine.encode(&profile, &original_data).unwrap();

        // 更新分片 0 和分片 2
        for shard_idx in [0, 2] {
            let old_data = shards[shard_idx][..20].to_vec();
            let mut new_data = old_data.clone();
            for b in new_data.iter_mut() {
                *b = b.wrapping_add(0xAA);
            }

            let update = IncrementalUpdate {
                shard_index: shard_idx,
                offset: 0,
                old_data,
                new_data: new_data.clone(),
            };

            let result = inc.compute_update(&update).unwrap();
            inc.apply_update(&mut shards, &result).unwrap();
            shards[shard_idx][..20].copy_from_slice(&new_data);
        }

        // 验证
        let slots: Vec<Option<Vec<u8>>> = shards.iter().cloned().map(Some).collect();
        let recovered = engine
            .decode_reconstruct(&profile, &slots, original_data.len())
            .unwrap();

        let mut expected = original_data.clone();
        let shard_size = shard_size_for(4, original_data.len());
        // 分片 0 的更新影响前 20 字节
        for i in 0..20 {
            expected[i] = expected[i].wrapping_add(0xAA);
        }
        // 分片 2 的更新影响第 3 个分片的前 20 字节
        let shard2_start = 2 * shard_size;
        for i in 0..20 {
            if shard2_start + i < expected.len() {
                expected[shard2_start + i] = expected[shard2_start + i].wrapping_add(0xAA);
            }
        }
        assert_eq!(recovered, expected);
    }

    #[test]
    fn test_incremental_invalid_shard_index() {
        let profile = make_profile(4, 2);
        let inc = IncrementalEncoder::new(profile).unwrap();

        let update = IncrementalUpdate {
            shard_index: 10, // 超出范围
            offset: 0,
            old_data: vec![0u8; 10],
            new_data: vec![1u8; 10],
        };

        let result = inc.compute_update(&update);
        assert!(result.is_err());
    }

    // ----- 完整性校验测试 -----

    #[test]
    fn test_integrity_crc32c() {
        let checker = IntegrityChecker::new(ChecksumType::Crc32c);
        let data = b"hello world";
        let cs = checker.compute_checksum(data);
        assert_eq!(cs.len(), 4);
        assert!(checker.verify_checksum(data, &cs));

        let bad_data = b"hello worle";
        assert!(!checker.verify_checksum(bad_data, &cs));
    }

    #[test]
    fn test_integrity_sha256() {
        let checker = IntegrityChecker::new(ChecksumType::Sha256);
        let data = b"hello world";
        let cs = checker.compute_checksum(data);
        assert_eq!(cs.len(), 32);
        assert!(checker.verify_checksum(data, &cs));
    }

    #[test]
    fn test_integrity_crc64() {
        let checker = IntegrityChecker::new(ChecksumType::Crc64);
        let data = b"hello world";
        let cs = checker.compute_checksum(data);
        assert_eq!(cs.len(), 8);
        assert!(checker.verify_checksum(data, &cs));
    }

    #[test]
    fn test_integrity_batch_verify() {
        let checker = IntegrityChecker::new(ChecksumType::Crc32c);
        let shards: Vec<Vec<u8>> = vec![
            b"shard 0 data".to_vec(),
            b"shard 1 data".to_vec(),
            b"shard 2 data".to_vec(),
        ];

        let checksums = checker.generate_checksums(&shards);
        assert_eq!(checksums.len(), 3);

        // 没有损坏
        let corrupted = checker.verify_shards(&shards, &checksums);
        assert!(corrupted.is_empty());

        // 损坏一个分片
        let mut bad_shards = shards.clone();
        bad_shards[1][0] ^= 0xFF;
        let corrupted = checker.verify_shards(&bad_shards, &checksums);
        assert_eq!(corrupted, vec![1]);
    }

    // ----- 渐进式重建测试 -----

    #[test]
    fn test_progressive_rebuild() {
        let profile = make_profile(4, 2);
        let rebuilder = ProgressiveRebuilder::new(100); // 100 字节小批次

        let data: Vec<u8> = (0..=255u8).cycle().take(1000).collect();
        let cauchy = CauchyReedSolomon::new(profile).unwrap();
        let shards = cauchy.encode(&data).unwrap();
        let shard_size = shards[0].len();

        // 丢失一个分片
        let mut slots: Vec<Option<Vec<u8>>> = shards.iter().cloned().map(Some).collect();
        let missing_idx = 1;
        slots[missing_idx] = None;

        let job = ProgressiveRebuildJob {
            job_id: "job-1".to_string(),
            object_id: "obj-1".to_string(),
            profile,
            shards: slots.clone(),
            missing_indices: vec![missing_idx],
            priority: RebuildPriority::Normal,
            processed_bytes: 0,
            total_bytes: shard_size as u64,
            result: None,
            engine_type: RebuildEngineType::CauchyRs,
        };

        rebuilder.submit_job(job);
        assert_eq!(rebuilder.pending_jobs(), 1);

        // 逐步处理批次直到完成
        let mut total_processed = 0u64;
        let mut iterations = 0;
        loop {
            let processed = rebuilder.process_batch().unwrap();
            if processed == 0 {
                break;
            }
            total_processed += processed;
            iterations += 1;
            assert!(iterations < 100, "too many iterations");
        }

        // 取出已完成的任务
        let completed = rebuilder.take_completed_jobs();
        assert_eq!(completed.len(), 1);

        let result = &completed[0].result.as_ref().unwrap()[missing_idx];
        assert_eq!(*result, shards[missing_idx]);
        assert_eq!(rebuilder.pending_jobs(), 0);
    }

    #[test]
    fn test_progressive_rebuild_stats() {
        let rebuilder = ProgressiveRebuilder::new(1024);
        let stats = rebuilder.stats();

        assert_eq!(*stats.jobs_submitted.lock(), 0);
        assert_eq!(*stats.jobs_completed.lock(), 0);

        let snap = stats.snapshot();
        assert!(snap.contains_key("ec_rebuild_jobs_submitted"));
        assert!(snap.contains_key("ec_rebuild_bytes_rebuilt"));
    }

    #[test]
    fn test_rebuild_priority_order() {
        let profile = make_profile(4, 2);
        let rebuilder = ProgressiveRebuilder::new(100);

        let data: Vec<u8> = (0..200).collect();
        let cauchy = CauchyReedSolomon::new(profile).unwrap();
        let shards = cauchy.encode(&data).unwrap();

        // 提交低优先级任务
        let mut low_slots: Vec<Option<Vec<u8>>> = shards.iter().cloned().map(Some).collect();
        low_slots[0] = None;
        rebuilder.submit_job(ProgressiveRebuildJob {
            job_id: "low".to_string(),
            object_id: "low-obj".to_string(),
            profile,
            shards: low_slots,
            missing_indices: vec![0],
            priority: RebuildPriority::Low,
            processed_bytes: 0,
            total_bytes: shards[0].len() as u64,
            result: None,
            engine_type: RebuildEngineType::CauchyRs,
        });

        // 提交高优先级任务
        let mut high_slots: Vec<Option<Vec<u8>>> = shards.iter().cloned().map(Some).collect();
        high_slots[1] = None;
        rebuilder.submit_job(ProgressiveRebuildJob {
            job_id: "high".to_string(),
            object_id: "high-obj".to_string(),
            profile,
            shards: high_slots,
            missing_indices: vec![1],
            priority: RebuildPriority::Critical,
            processed_bytes: 0,
            total_bytes: shards[0].len() as u64,
            result: None,
            engine_type: RebuildEngineType::CauchyRs,
        });

        // 处理一个批次，应该先处理高优先级的
        rebuilder.process_batch().unwrap();

        // 高优先级任务应该已经开始处理（有 result）
        let jobs = rebuilder.jobs.lock();
        let high_job = jobs.iter().find(|j| j.job_id == "high");
        let low_job = jobs.iter().find(|j| j.job_id == "low");

        // high 应该被处理了（processed_bytes > 0 或 result is_some）
        // 或者它已经完成并被移除
        let high_processed = high_job.map(|j| j.processed_bytes).unwrap_or(shards[0].len() as u64);
        let low_processed = low_job.map(|j| j.processed_bytes).unwrap_or(0);

        assert!(
            high_processed >= low_processed,
            "high priority should be processed first"
        );
    }

    #[test]
    fn test_checksum_type_default() {
        assert_eq!(ChecksumType::default(), ChecksumType::Crc32c);
    }

    #[test]
    fn test_rebuild_priority_default() {
        assert_eq!(RebuildPriority::default(), RebuildPriority::Low);
    }

    #[test]
    fn test_rebuild_engine_type_default() {
        assert_eq!(RebuildEngineType::default(), RebuildEngineType::CauchyRs);
    }

    #[test]
    fn test_rebuild_priority_ordering() {
        assert!(RebuildPriority::Critical > RebuildPriority::High);
        assert!(RebuildPriority::High > RebuildPriority::Normal);
        assert!(RebuildPriority::Normal > RebuildPriority::Low);
    }

    #[test]
    fn test_cauchy_small_data() {
        let profile = make_profile(4, 2);
        let crs = CauchyReedSolomon::new(profile).unwrap();

        // 小数据（会被填充）
        let data = vec![42u8; 10];
        let shards = crs.encode(&data).unwrap();
        assert_eq!(shards.len(), 6);

        let slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        let recovered = crs.decode_reconstruct(&slots, data.len()).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn test_cauchy_8plus4() {
        let profile = make_profile(8, 4);
        let crs = CauchyReedSolomon::new(profile).unwrap();

        let data: Vec<u8> = (0..=255u8).cycle().take(8000).collect();
        let shards = crs.encode(&data).unwrap();
        assert_eq!(shards.len(), 12);

        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        slots[1] = None;
        slots[3] = None;
        slots[9] = None;
        slots[11] = None;

        let recovered = crs.decode_reconstruct(&slots, data.len()).unwrap();
        assert_eq!(recovered, data);
    }
}
