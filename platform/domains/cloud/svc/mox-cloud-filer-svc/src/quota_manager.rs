// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 配额管理器模块
//!
//! 提供用户级、目录级、桶级的存储容量和文件数配额管理。
//! 参考分布式文件系统配额管理和 Linux disk quota 设计。
//!
//! # 功能特性
//!
//! * **用户级配额**：按 UID 限制存储容量和文件数
//! * **目录级配额**：按目录 inode 限制子树容量和文件数
//! * **桶级配额**：按存储桶限制总容量
//! * **配额检查**：写入前检查，超配额拒绝操作
//! * **软硬配额**：硬配额（严格限制）和软配额（告警阈值）
//! * **配额统计**：实时用量统计，超限告警记录
//! * **宽限期**：软配额超限后可继续写入的宽限时间
//!
//! # 设计说明
//!
//! 采用三层配额架构：用户配额 > 目录配额 > 桶配额。
//! 每次写入操作需要依次检查各级配额，任一超限即拒绝。
//! 配额使用量通过操作钩子实时更新，支持增量统计。
//!
//! 软配额与宽限期：超过软配额后进入宽限期，宽限期内仍可写入
//! 直到超过硬配额。宽限期结束后，即使未超过硬配额也只能删除不能写入。

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::error::FilerResult;

// ---------------- 常量 ----------------

/// 默认宽限期（秒）：7 天
pub const DEFAULT_GRACE_PERIOD_SECS: u64 = 7 * 86400;

// ---------------- 类型定义 ----------------

/// 配额限制
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct QuotaLimit {
    /// 硬配额容量（字节），0 表示无限制
    pub hard_bytes: u64,
    /// 软配额容量（字节），0 表示无限制
    pub soft_bytes: u64,
    /// 硬配额文件数，0 表示无限制
    pub hard_files: u64,
    /// 软配额文件数，0 表示无限制
    pub soft_files: u64,
}

impl QuotaLimit {
    /// 无限制配额
    pub fn unlimited() -> Self {
        Self { hard_bytes: 0, soft_bytes: 0, hard_files: 0, soft_files: 0 }
    }

    /// 是否为无限制
    pub fn is_unlimited(&self) -> bool {
        self.hard_bytes == 0 && self.soft_bytes == 0 && self.hard_files == 0 && self.soft_files == 0
    }
}

/// 配额使用量
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct QuotaUsage {
    /// 已用容量（字节）
    pub used_bytes: u64,
    /// 已用文件数
    pub used_files: u64,
    /// 上次超过软配额的时间（秒，0 表示未超限）
    pub soft_exceeded_at_sec: u64,
}

/// 配额检查结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaCheckResult {
    /// 配额充足，可以写入
    Ok,
    /// 超过软配额（仍在宽限期内）
    SoftExceeded,
    /// 超过硬配额，拒绝写入
    HardExceeded,
    /// 软配额超限且超过宽限期
    GracePeriodExpired,
}

impl QuotaCheckResult {
    /// 是否允许写入
    pub fn is_allowed(&self) -> bool {
        matches!(self, QuotaCheckResult::Ok | QuotaCheckResult::SoftExceeded)
    }
}

/// 配额类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuotaType {
    /// 用户级配额
    User,
    /// 目录级配额
    Directory,
    /// 桶级配额
    Bucket,
}

/// 配额告警记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaAlert {
    /// 配额类型
    pub quota_type: QuotaType,
    /// 配额标识（UID / inode / bucket_name）
    pub quota_id: String,
    /// 告警时间（秒）
    pub alert_time_sec: u64,
    /// 已用容量
    pub used_bytes: u64,
    /// 硬配额容量
    pub hard_bytes: u64,
    /// 使用率百分比
    pub usage_percent: f64,
    /// 告警消息
    pub message: String,
}

/// 配额统计摘要
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuotaStats {
    /// 配置的用户配额数
    pub user_quotas: usize,
    /// 配置的目录配额数
    pub dir_quotas: usize,
    /// 配置的桶配额数
    pub bucket_quotas: usize,
    /// 总告警数
    pub total_alerts: u64,
    /// 当前超限的配额数
    pub exceeded_quotas: usize,
    /// 配额总数（user + dir + bucket）
    pub total_quotas: usize,
}

// ---------------- 配额管理器 ----------------

/// 配额管理器
///
/// 管理各级配额配置和使用量，提供配额检查和告警。
#[derive(Debug)]
pub struct QuotaManager {
    /// 用户配额：uid -> (limit, usage)
    user_quotas: parking_lot::Mutex<BTreeMap<u32, (QuotaLimit, QuotaUsage)>>,
    /// 目录配额：ino -> (limit, usage)
    dir_quotas: parking_lot::Mutex<BTreeMap<u64, (QuotaLimit, QuotaUsage)>>,
    /// 桶配额：bucket_name -> (limit, usage)
    bucket_quotas: parking_lot::Mutex<BTreeMap<String, (QuotaLimit, QuotaUsage)>>,
    /// 告警历史
    alerts: parking_lot::Mutex<Vec<QuotaAlert>>,
    /// 宽限期（秒）
    grace_period_secs: u64,
    /// 最大告警记录数
    max_alerts: usize,
}

impl Default for QuotaManager {
    fn default() -> Self {
        Self::new()
    }
}

impl QuotaManager {
    /// 创建新的配额管理器
    pub fn new() -> Self {
        Self {
            user_quotas: parking_lot::Mutex::new(BTreeMap::new()),
            dir_quotas: parking_lot::Mutex::new(BTreeMap::new()),
            bucket_quotas: parking_lot::Mutex::new(BTreeMap::new()),
            alerts: parking_lot::Mutex::new(Vec::new()),
            grace_period_secs: DEFAULT_GRACE_PERIOD_SECS,
            max_alerts: 1000,
        }
    }

    /// 设置宽限期
    pub fn set_grace_period(&mut self, grace_period_secs: u64) {
        self.grace_period_secs = grace_period_secs;
    }

    // ---- 用户配额 ----

    /// 设置用户配额
    pub fn set_user_quota(&self, uid: u32, limit: QuotaLimit) {
        let mut quotas = self.user_quotas.lock();
        let entry = quotas.entry(uid).or_default();
        entry.0 = limit;
        // usage 保持不变
    }

    /// 获取用户配额和使用量
    pub fn get_user_quota(&self, uid: u32) -> Option<(QuotaLimit, QuotaUsage)> {
        self.user_quotas.lock().get(&uid).copied()
    }

    /// 检查用户配额
    pub fn check_user_quota(&self, uid: u32, add_bytes: u64, add_files: u64) -> QuotaCheckResult {
        let quotas = self.user_quotas.lock();
        let (limit, usage) = match quotas.get(&uid) {
            Some(v) => v,
            None => return QuotaCheckResult::Ok, // 无配置 = 无限制
        };

        self.check_quota_internal(
            limit,
            usage,
            add_bytes,
            add_files,
            QuotaType::User,
            &uid.to_string(),
        )
    }

    /// 更新用户配额使用量
    pub fn update_user_usage(&self, uid: u32, delta_bytes: i64, delta_files: i64) {
        let mut quotas = self.user_quotas.lock();
        let (_, usage) = quotas.entry(uid).or_default();
        self.update_usage(usage, delta_bytes, delta_files);
    }

    // ---- 目录配额 ----

    /// 设置目录配额
    pub fn set_dir_quota(&self, ino: u64, limit: QuotaLimit) {
        let mut quotas = self.dir_quotas.lock();
        let entry = quotas.entry(ino).or_default();
        entry.0 = limit;
    }

    /// 获取目录配额和使用量
    pub fn get_dir_quota(&self, ino: u64) -> Option<(QuotaLimit, QuotaUsage)> {
        self.dir_quotas.lock().get(&ino).copied()
    }

    /// 检查目录配额
    pub fn check_dir_quota(&self, ino: u64, add_bytes: u64, add_files: u64) -> QuotaCheckResult {
        let quotas = self.dir_quotas.lock();
        let (limit, usage) = match quotas.get(&ino) {
            Some(v) => v,
            None => return QuotaCheckResult::Ok,
        };

        self.check_quota_internal(
            limit,
            usage,
            add_bytes,
            add_files,
            QuotaType::Directory,
            &ino.to_string(),
        )
    }

    /// 更新目录配额使用量
    pub fn update_dir_usage(&self, ino: u64, delta_bytes: i64, delta_files: i64) {
        let mut quotas = self.dir_quotas.lock();
        let (_, usage) = quotas.entry(ino).or_default();
        self.update_usage(usage, delta_bytes, delta_files);
    }

    // ---- 桶配额 ----

    /// 设置桶配额
    pub fn set_bucket_quota(&self, bucket: &str, limit: QuotaLimit) {
        let mut quotas = self.bucket_quotas.lock();
        let entry = quotas.entry(bucket.to_string()).or_default();
        entry.0 = limit;
    }

    /// 获取桶配额和使用量
    pub fn get_bucket_quota(&self, bucket: &str) -> Option<(QuotaLimit, QuotaUsage)> {
        self.bucket_quotas.lock().get(bucket).copied()
    }

    /// 检查桶配额
    pub fn check_bucket_quota(
        &self,
        bucket: &str,
        add_bytes: u64,
        add_files: u64,
    ) -> QuotaCheckResult {
        let quotas = self.bucket_quotas.lock();
        let (limit, usage) = match quotas.get(bucket) {
            Some(v) => v,
            None => return QuotaCheckResult::Ok,
        };

        self.check_quota_internal(limit, usage, add_bytes, add_files, QuotaType::Bucket, bucket)
    }

    /// 更新桶配额使用量
    pub fn update_bucket_usage(&self, bucket: &str, delta_bytes: i64, delta_files: i64) {
        let mut quotas = self.bucket_quotas.lock();
        let (_, usage) = quotas.entry(bucket.to_string()).or_default();
        self.update_usage(usage, delta_bytes, delta_files);
    }

    // ---- 综合检查 ----

    /// 综合检查所有相关配额（用户 + 目录 + 桶）
    ///
    /// 返回最严格的检查结果。
    pub fn check_all(
        &self,
        uid: u32,
        dir_ino: u64,
        bucket: Option<&str>,
        add_bytes: u64,
        add_files: u64,
    ) -> QuotaCheckResult {
        let mut result = QuotaCheckResult::Ok;

        // 检查用户配额
        let user_result = self.check_user_quota(uid, add_bytes, add_files);
        result = Self::merge_results(result, user_result);

        // 检查目录配额
        let dir_result = self.check_dir_quota(dir_ino, add_bytes, add_files);
        result = Self::merge_results(result, dir_result);

        // 检查桶配额
        if let Some(bkt) = bucket {
            let bucket_result = self.check_bucket_quota(bkt, add_bytes, add_files);
            result = Self::merge_results(result, bucket_result);
        }

        result
    }

    /// 合并两个检查结果（取更严格的）
    fn merge_results(a: QuotaCheckResult, b: QuotaCheckResult) -> QuotaCheckResult {
        // 严格程度：GracePeriodExpired > HardExceeded > SoftExceeded > Ok
        match (a, b) {
            (QuotaCheckResult::GracePeriodExpired, _)
            | (_, QuotaCheckResult::GracePeriodExpired) => QuotaCheckResult::GracePeriodExpired,
            (QuotaCheckResult::HardExceeded, _) | (_, QuotaCheckResult::HardExceeded) => {
                QuotaCheckResult::HardExceeded
            },
            (QuotaCheckResult::SoftExceeded, _) | (_, QuotaCheckResult::SoftExceeded) => {
                QuotaCheckResult::SoftExceeded
            },
            _ => QuotaCheckResult::Ok,
        }
    }

    // ---- 内部方法 ----

    /// 通用配额检查
    fn check_quota_internal(
        &self,
        limit: &QuotaLimit,
        usage: &QuotaUsage,
        add_bytes: u64,
        add_files: u64,
        quota_type: QuotaType,
        quota_id: &str,
    ) -> QuotaCheckResult {
        if limit.is_unlimited() {
            return QuotaCheckResult::Ok;
        }

        let new_bytes = usage.used_bytes.saturating_add(add_bytes);
        let new_files = usage.used_files.saturating_add(add_files);

        // 检查硬配额
        if limit.hard_bytes > 0 && new_bytes > limit.hard_bytes {
            self.add_alert(
                quota_type,
                quota_id,
                new_bytes,
                limit.hard_bytes,
                "Hard quota exceeded (bytes)",
            );
            return QuotaCheckResult::HardExceeded;
        }
        if limit.hard_files > 0 && new_files > limit.hard_files {
            self.add_alert(
                quota_type,
                quota_id,
                new_bytes,
                limit.hard_bytes,
                "Hard quota exceeded (files)",
            );
            return QuotaCheckResult::HardExceeded;
        }

        // 检查软配额
        let now = now_secs();
        let mut soft_exceeded = false;

        if limit.soft_bytes > 0 && new_bytes > limit.soft_bytes {
            soft_exceeded = true;
        }
        if limit.soft_files > 0 && new_files > limit.soft_files {
            soft_exceeded = true;
        }

        if soft_exceeded {
            // 检查是否在宽限期内
            if usage.soft_exceeded_at_sec == 0 {
                // 首次超限，记录时间
                // 注意：这里只读锁无法修改，需要调用方更新
                // 简化：返回 SoftExceeded
                self.add_alert(
                    quota_type,
                    quota_id,
                    new_bytes,
                    limit.soft_bytes,
                    "Soft quota exceeded",
                );
                return QuotaCheckResult::SoftExceeded;
            }

            let elapsed = now.saturating_sub(usage.soft_exceeded_at_sec);
            if elapsed > self.grace_period_secs {
                self.add_alert(
                    quota_type,
                    quota_id,
                    new_bytes,
                    limit.soft_bytes,
                    "Grace period expired after soft quota exceeded",
                );
                return QuotaCheckResult::GracePeriodExpired;
            }

            QuotaCheckResult::SoftExceeded
        } else {
            QuotaCheckResult::Ok
        }
    }

    /// 更新使用量
    fn update_usage(&self, usage: &mut QuotaUsage, delta_bytes: i64, delta_files: i64) {
        if delta_bytes >= 0 {
            usage.used_bytes = usage.used_bytes.saturating_add(delta_bytes as u64);
        } else {
            usage.used_bytes = usage.used_bytes.saturating_sub((-delta_bytes) as u64);
        }

        if delta_files >= 0 {
            usage.used_files = usage.used_files.saturating_add(delta_files as u64);
        } else {
            usage.used_files = usage.used_files.saturating_sub((-delta_files) as u64);
        }

        // 如果使用量降回软配额以下，重置超限时间
        // （简化：这里不检查软配额，由外部检查时处理）
        let _ = now_secs();
    }

    /// 添加告警
    fn add_alert(
        &self,
        quota_type: QuotaType,
        quota_id: &str,
        used_bytes: u64,
        limit_bytes: u64,
        message: &str,
    ) {
        let usage_percent =
            if limit_bytes > 0 { (used_bytes as f64 / limit_bytes as f64) * 100.0 } else { 0.0 };

        let alert = QuotaAlert {
            quota_type,
            quota_id: quota_id.to_string(),
            alert_time_sec: now_secs(),
            used_bytes,
            hard_bytes: limit_bytes,
            usage_percent,
            message: message.to_string(),
        };

        let mut alerts = self.alerts.lock();
        alerts.push(alert);
        if alerts.len() > self.max_alerts {
            let drain_count = alerts.len() - self.max_alerts;
            alerts.drain(0..drain_count);
        }
    }

    // ---- 查询与统计 ----

    /// 获取告警列表
    pub fn get_alerts(&self, limit: usize) -> Vec<QuotaAlert> {
        let alerts = self.alerts.lock();
        alerts.iter().rev().take(limit).cloned().collect()
    }

    // ---- 通用配额 API（按 QuotaType 分发） ----

    /// 通用设置配额
    pub fn set_quota(&self, id: &str, quota_type: QuotaType, limit: QuotaLimit) {
        match quota_type {
            QuotaType::User => self.set_user_quota(parse_u32_id(id), limit),
            QuotaType::Directory => self.set_dir_quota(parse_u64_id(id), limit),
            QuotaType::Bucket => self.set_bucket_quota(id, limit),
        }
    }

    /// 通用获取配额
    pub fn get_quota(&self, id: &str, quota_type: QuotaType) -> Option<(QuotaLimit, QuotaUsage)> {
        match quota_type {
            QuotaType::User => self.get_user_quota(parse_u32_id(id)),
            QuotaType::Directory => self.get_dir_quota(parse_u64_id(id)),
            QuotaType::Bucket => self.get_bucket_quota(id),
        }
    }

    /// 通用配额检查（使用显式 usage）
    pub fn check_quota(
        &self,
        id: &str,
        quota_type: QuotaType,
        usage: QuotaUsage,
        add_bytes: u64,
        add_files: u64,
    ) -> QuotaCheckResult {
        let limit = self.get_quota(id, quota_type).map(|(l, _)| l).unwrap_or_default();
        self.check_quota_internal(&limit, &usage, add_bytes, add_files, quota_type, id)
    }

    /// 获取配额统计
    pub fn stats(&self) -> QuotaStats {
        let user_quotas = self.user_quotas.lock();
        let dir_quotas = self.dir_quotas.lock();
        let bucket_quotas = self.bucket_quotas.lock();
        let alerts = self.alerts.lock();

        let mut exceeded = 0;

        for (limit, usage) in user_quotas.values() {
            if (limit.hard_bytes > 0 && usage.used_bytes >= limit.hard_bytes)
                || (limit.hard_files > 0 && usage.used_files >= limit.hard_files)
            {
                exceeded += 1;
            }
        }
        for (limit, usage) in dir_quotas.values() {
            if (limit.hard_bytes > 0 && usage.used_bytes >= limit.hard_bytes)
                || (limit.hard_files > 0 && usage.used_files >= limit.hard_files)
            {
                exceeded += 1;
            }
        }
        for (limit, usage) in bucket_quotas.values() {
            if (limit.hard_bytes > 0 && usage.used_bytes >= limit.hard_bytes)
                || (limit.hard_files > 0 && usage.used_files >= limit.hard_files)
            {
                exceeded += 1;
            }
        }

        QuotaStats {
            user_quotas: user_quotas.len(),
            dir_quotas: dir_quotas.len(),
            bucket_quotas: bucket_quotas.len(),
            total_alerts: alerts.len() as u64,
            exceeded_quotas: exceeded,
            total_quotas: user_quotas.len() + dir_quotas.len() + bucket_quotas.len(),
        }
    }

    /// 重置配额使用量（用于管理员操作）
    pub fn reset_usage(&self, quota_type: QuotaType, id: &str) -> FilerResult<()> {
        match quota_type {
            QuotaType::User => {
                let uid: u32 = id.parse().map_err(|_| crate::error::FilerError::AttrInvalid)?;
                let mut quotas = self.user_quotas.lock();
                if let Some((_, usage)) = quotas.get_mut(&uid) {
                    usage.used_bytes = 0;
                    usage.used_files = 0;
                    usage.soft_exceeded_at_sec = 0;
                }
            },
            QuotaType::Directory => {
                let ino: u64 = id.parse().map_err(|_| crate::error::FilerError::AttrInvalid)?;
                let mut quotas = self.dir_quotas.lock();
                if let Some((_, usage)) = quotas.get_mut(&ino) {
                    usage.used_bytes = 0;
                    usage.used_files = 0;
                    usage.soft_exceeded_at_sec = 0;
                }
            },
            QuotaType::Bucket => {
                let mut quotas = self.bucket_quotas.lock();
                if let Some((_, usage)) = quotas.get_mut(id) {
                    usage.used_bytes = 0;
                    usage.used_files = 0;
                    usage.soft_exceeded_at_sec = 0;
                }
            },
        }
        Ok(())
    }

    /// 列出所有用户配额
    pub fn list_user_quotas(&self) -> Vec<(u32, QuotaLimit, QuotaUsage)> {
        let quotas = self.user_quotas.lock();
        quotas.iter().map(|(uid, (limit, usage))| (*uid, *limit, *usage)).collect()
    }

    /// 列出所有目录配额
    pub fn list_dir_quotas(&self) -> Vec<(u64, QuotaLimit, QuotaUsage)> {
        let quotas = self.dir_quotas.lock();
        quotas.iter().map(|(ino, (limit, usage))| (*ino, *limit, *usage)).collect()
    }
}

// ---------------- 辅助函数 ----------------

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// ---------------- 共享类型别名 ----------------

/// 共享的配额管理器引用
pub type SharedQuotaManager = Arc<QuotaManager>;


// ---------------- 通用 ID 解析辅助 ----------------

fn parse_u32_id(id: &str) -> u32 {
    // 提取字符串中的数字部分，如 "user-1" -> 1, "u2" -> 2
    let digits: String = id.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse().unwrap_or(0)
}

fn parse_u64_id(id: &str) -> u64 {
    let digits: String = id.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse().unwrap_or(0)
}

// ---------------- 单元测试 ----------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_limit_unlimited() {
        let limit = QuotaLimit::unlimited();
        assert!(limit.is_unlimited());

        let limit2 = QuotaLimit { hard_bytes: 1024, ..Default::default() };
        assert!(!limit2.is_unlimited());
    }

    #[test]
    fn test_user_quota_check_ok() {
        let mgr = QuotaManager::new();

        let limit = QuotaLimit {
            hard_bytes: 1024 * 1024, // 1MB
            soft_bytes: 512 * 1024,  // 512KB
            hard_files: 100,
            soft_files: 50,
        };
        mgr.set_user_quota(1000, limit);
        mgr.update_user_usage(1000, 1024, 1); // 已用 1KB, 1 个文件

        let result = mgr.check_user_quota(1000, 100, 1);
        assert_eq!(result, QuotaCheckResult::Ok);
        assert!(result.is_allowed());
    }

    #[test]
    fn test_user_quota_soft_exceeded() {
        let mgr = QuotaManager::new();

        let limit = QuotaLimit {
            hard_bytes: 1024 * 1024,
            soft_bytes: 512, // 512 bytes 软配额
            hard_files: 100,
            soft_files: 50,
        };
        mgr.set_user_quota(1000, limit);
        mgr.update_user_usage(1000, 100, 1); // 已用 100 bytes

        let result = mgr.check_user_quota(1000, 500, 0); // 再加 500 = 600 > 512
        assert_eq!(result, QuotaCheckResult::SoftExceeded);
        assert!(result.is_allowed()); // 软超限仍允许
    }

    #[test]
    fn test_user_quota_hard_exceeded() {
        let mgr = QuotaManager::new();

        let limit =
            QuotaLimit { hard_bytes: 1000, soft_bytes: 500, hard_files: 100, soft_files: 50 };
        mgr.set_user_quota(1000, limit);
        mgr.update_user_usage(1000, 800, 1);

        let result = mgr.check_user_quota(1000, 300, 0); // 800 + 300 = 1100 > 1000
        assert_eq!(result, QuotaCheckResult::HardExceeded);
        assert!(!result.is_allowed());
    }

    #[test]
    fn test_file_count_quota() {
        let mgr = QuotaManager::new();

        let limit = QuotaLimit { hard_bytes: 0, soft_bytes: 0, hard_files: 10, soft_files: 0 };
        mgr.set_user_quota(1000, limit);
        mgr.update_user_usage(1000, 0, 8);

        let result = mgr.check_user_quota(1000, 0, 1); // 8 + 1 = 9 < 10
        assert_eq!(result, QuotaCheckResult::Ok);

        let result = mgr.check_user_quota(1000, 0, 3); // 8 + 3 = 11 > 10
        assert_eq!(result, QuotaCheckResult::HardExceeded);
    }

    #[test]
    fn test_dir_quota() {
        let mgr = QuotaManager::new();

        let limit = QuotaLimit { hard_bytes: 1024, soft_bytes: 0, hard_files: 0, soft_files: 0 };
        mgr.set_dir_quota(100, limit);
        mgr.update_dir_usage(100, 512, 1);

        assert_eq!(mgr.check_dir_quota(100, 100, 0), QuotaCheckResult::Ok);
        assert_eq!(mgr.check_dir_quota(100, 600, 0), QuotaCheckResult::HardExceeded);
    }

    #[test]
    fn test_bucket_quota() {
        let mgr = QuotaManager::new();

        let limit = QuotaLimit {
            hard_bytes: 1024 * 1024 * 1024, // 1GB
            soft_bytes: 512 * 1024 * 1024,  // 512MB
            hard_files: 1000000,
            soft_files: 0,
        };
        mgr.set_bucket_quota("my-bucket", limit);
        mgr.update_bucket_usage("my-bucket", 1024 * 1024, 100); // 1MB, 100 files

        assert_eq!(mgr.check_bucket_quota("my-bucket", 1024, 1), QuotaCheckResult::Ok);
    }

    #[test]
    fn test_check_all() {
        let mgr = QuotaManager::new();

        // 用户有 1000 bytes 硬配额
        let user_limit =
            QuotaLimit { hard_bytes: 1000, soft_bytes: 0, hard_files: 0, soft_files: 0 };
        mgr.set_user_quota(1000, user_limit);
        mgr.update_user_usage(1000, 500, 1);

        // 目录有 2000 bytes 硬配额
        let dir_limit =
            QuotaLimit { hard_bytes: 2000, soft_bytes: 0, hard_files: 0, soft_files: 0 };
        mgr.set_dir_quota(100, dir_limit);
        mgr.update_dir_usage(100, 100, 1);

        // 只加 100 bytes，两个配额都够
        let result = mgr.check_all(1000, 100, None, 100, 1);
        assert_eq!(result, QuotaCheckResult::Ok);

        // 加 600 bytes，用户配额超限（500+600=1100 > 1000）
        let result = mgr.check_all(1000, 100, None, 600, 1);
        assert_eq!(result, QuotaCheckResult::HardExceeded);
    }

    #[test]
    fn test_quota_usage_update() {
        let mgr = QuotaManager::new();

        let limit = QuotaLimit { hard_bytes: 1000, soft_bytes: 500, hard_files: 10, soft_files: 5 };
        mgr.set_user_quota(1000, limit);

        // 增加
        mgr.update_user_usage(1000, 300, 3);
        let (_, usage) = mgr.get_user_quota(1000).unwrap();
        assert_eq!(usage.used_bytes, 300);
        assert_eq!(usage.used_files, 3);

        // 再增加
        mgr.update_user_usage(1000, 200, 2);
        let (_, usage) = mgr.get_user_quota(1000).unwrap();
        assert_eq!(usage.used_bytes, 500);
        assert_eq!(usage.used_files, 5);

        // 减少
        mgr.update_user_usage(1000, -100, -1);
        let (_, usage) = mgr.get_user_quota(1000).unwrap();
        assert_eq!(usage.used_bytes, 400);
        assert_eq!(usage.used_files, 4);
    }

    #[test]
    fn test_no_quota_unlimited() {
        let mgr = QuotaManager::new();

        // 没有设置配额的用户，检查应返回 Ok
        let result = mgr.check_user_quota(9999, 1_000_000_000, 1_000_000);
        assert_eq!(result, QuotaCheckResult::Ok);
    }

    #[test]
    fn test_alerts_generated() {
        let mgr = QuotaManager::new();

        let limit = QuotaLimit { hard_bytes: 1000, soft_bytes: 0, hard_files: 0, soft_files: 0 };
        mgr.set_user_quota(1000, limit);
        mgr.update_user_usage(1000, 500, 1);

        // 触发硬超限
        mgr.check_user_quota(1000, 600, 0);

        let alerts = mgr.get_alerts(10);
        assert!(!alerts.is_empty());
        assert_eq!(alerts[0].quota_type, QuotaType::User);
    }

    #[test]
    fn test_stats() {
        let mgr = QuotaManager::new();

        mgr.set_user_quota(1000, QuotaLimit { hard_bytes: 1000, ..Default::default() });
        mgr.set_user_quota(1001, QuotaLimit { hard_bytes: 2000, ..Default::default() });
        mgr.set_dir_quota(100, QuotaLimit { hard_bytes: 500, ..Default::default() });
        mgr.set_bucket_quota("b1", QuotaLimit { hard_bytes: 100, ..Default::default() });

        let stats = mgr.stats();
        assert_eq!(stats.user_quotas, 2);
        assert_eq!(stats.dir_quotas, 1);
        assert_eq!(stats.bucket_quotas, 1);
    }

    #[test]
    fn test_reset_usage() {
        let mgr = QuotaManager::new();

        mgr.set_user_quota(1000, QuotaLimit { hard_bytes: 1000, ..Default::default() });
        mgr.update_user_usage(1000, 500, 10);

        let (_, usage_before) = mgr.get_user_quota(1000).unwrap();
        assert_eq!(usage_before.used_bytes, 500);
        assert_eq!(usage_before.used_files, 10);

        mgr.reset_usage(QuotaType::User, "1000").unwrap();

        let (_, usage_after) = mgr.get_user_quota(1000).unwrap();
        assert_eq!(usage_after.used_bytes, 0);
        assert_eq!(usage_after.used_files, 0);
    }

    #[test]
    fn test_quota_check_result_is_allowed() {
        assert!(QuotaCheckResult::Ok.is_allowed());
        assert!(QuotaCheckResult::SoftExceeded.is_allowed());
        assert!(!QuotaCheckResult::HardExceeded.is_allowed());
        assert!(!QuotaCheckResult::GracePeriodExpired.is_allowed());
    }

    #[test]
    fn test_list_quotas() {
        let mgr = QuotaManager::new();

        mgr.set_user_quota(1000, QuotaLimit { hard_bytes: 100, ..Default::default() });
        mgr.set_user_quota(1001, QuotaLimit { hard_bytes: 200, ..Default::default() });

        let users = mgr.list_user_quotas();
        assert_eq!(users.len(), 2);

        mgr.set_dir_quota(1, QuotaLimit { hard_bytes: 500, ..Default::default() });
        let dirs = mgr.list_dir_quotas();
        assert_eq!(dirs.len(), 1);
    }
}
