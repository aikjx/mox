// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 迁移任务管理模块
//!
//! 负责数据迁移任务的生命周期管理、限速、断点续传、
//! 数据一致性校验等功能。
//!
//! 迁移类型：
//! - 容量均衡迁移：平衡各节点使用率
//! - 故障恢复迁移：重建丢失的副本
//! - 分层迁移：数据在热/温/冷层之间移动
//! - 节点下线迁移：将数据从待下线节点迁出

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// 迁移类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MigrationType {
    /// 容量均衡
    CapacityRebalance,
    /// 故障恢复
    FailureRecovery,
    /// 分层降级（热->温->冷）
    TierDown,
    /// 分层升级（冷->温->热）
    TierUp,
    /// 节点下线
    NodeDecommission,
    /// 手动触发
    Manual,
}

impl MigrationType {
    /// 获取默认优先级
    pub fn default_priority(&self) -> u8 {
        match self {
            MigrationType::FailureRecovery => 10,
            MigrationType::NodeDecommission => 9,
            MigrationType::Manual => 7,
            MigrationType::TierUp => 6,
            MigrationType::CapacityRebalance => 4,
            MigrationType::TierDown => 2,
        }
    }

    /// 类型名称
    pub fn name(&self) -> &'static str {
        match self {
            MigrationType::CapacityRebalance => "capacity_rebalance",
            MigrationType::FailureRecovery => "failure_recovery",
            MigrationType::TierDown => "tier_down",
            MigrationType::TierUp => "tier_up",
            MigrationType::NodeDecommission => "node_decommission",
            MigrationType::Manual => "manual",
        }
    }
}

impl std::fmt::Display for MigrationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// 迁移状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MigrationStatus {
    /// 等待执行
    Pending,
    /// 正在执行
    Running,
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

impl MigrationStatus {
    /// 是否处于终态
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            MigrationStatus::Completed | MigrationStatus::Failed | MigrationStatus::Cancelled
        )
    }

    /// 是否可以取消
    pub fn is_cancellable(&self) -> bool {
        matches!(
            self,
            MigrationStatus::Pending | MigrationStatus::Running | MigrationStatus::Paused
        )
    }
}

/// 迁移阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationPhase {
    /// 初始化阶段
    Init,
    /// 全量同步阶段
    FullSync,
    /// 增量同步阶段
    IncrementalSync,
    /// 流量切换阶段
    TrafficSwitch,
    /// 清理旧数据阶段
    Cleanup,
    /// 验证阶段
    Verification,
}

/// 数据一致性校验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// 是否通过
    pub passed: bool,
    /// 校验方法
    pub method: VerificationMethod,
    /// 源端校验和
    pub source_checksum: String,
    /// 目标端校验和
    pub target_checksum: String,
    /// 校验的字节数
    pub bytes_verified: u64,
    /// 耗时（ms）
    pub duration_ms: u64,
    /// 错误信息
    pub error: Option<String>,
}

/// 校验方法
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationMethod {
    /// CRC-32C
    Crc32c,
    /// CRC-64
    Crc64,
    /// SHA-256
    Sha256,
    /// 字节级对比（最严格）
    ByteCompare,
    /// 抽样校验
    Sampling,
}

/// 迁移断点信息（用于断点续传）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationCheckpoint {
    /// 已完成的字节偏移
    pub completed_offset: u64,
    /// 最后一个成功的块索引
    pub last_block_index: u64,
    /// 已验证的字节偏移
    pub verified_offset: u64,
    /// 当前阶段
    pub current_phase: MigrationPhase,
    /// 保存时间（ms）
    pub saved_at_ms: u64,
}

/// 迁移任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationTask {
    /// 任务 ID
    pub task_id: String,
    /// 迁移类型
    pub migration_type: MigrationType,
    /// 源节点 ID
    pub source_node_id: String,
    /// 源节点地址
    pub source_addr: String,
    /// 目标节点 ID
    pub target_node_id: String,
    /// 目标节点地址
    pub target_addr: String,
    /// 关联的卷/对象 ID
    pub object_id: String,
    /// 总数据大小（字节）
    pub total_bytes: u64,
    /// 已迁移字节数
    pub migrated_bytes: u64,
    /// 已验证字节数
    pub verified_bytes: u64,
    /// 优先级（0-10，越高越紧急）
    pub priority: u8,
    /// 当前状态
    pub status: MigrationStatus,
    /// 当前阶段
    pub phase: MigrationPhase,
    /// 创建时间（ms）
    pub created_at_ms: u64,
    /// 开始时间（ms）
    pub started_at_ms: Option<u64>,
    /// 完成时间（ms）
    pub completed_at_ms: Option<u64>,
    /// 限速（bytes/s，0 表示不限速）
    pub bandwidth_limit_bps: u64,
    /// 失败重试次数
    pub retry_count: u32,
    /// 最大重试次数
    pub max_retries: u32,
    /// 最后一次错误
    pub last_error: Option<String>,
    /// 断点信息
    pub checkpoint: Option<MigrationCheckpoint>,
    /// 迁移后是否删除源端数据
    pub delete_source_after: bool,
    /// 是否启用迁移后校验
    pub verify_after_migration: bool,
    /// 校验方法
    pub verification_method: VerificationMethod,
}

/// 迁移任务管理器
pub struct MigrationTaskManager {
    /// 待执行队列
    pending_queue: parking_lot::Mutex<VecDeque<MigrationTask>>,
    /// 执行中的任务
    running_tasks: parking_lot::Mutex<HashMap<String, MigrationTask>>,
    /// 已完成/失败的任务（保留用于查询）
    completed_tasks: parking_lot::Mutex<Vec<MigrationTask>>,
    /// 最大并发迁移数
    max_concurrent: parking_lot::Mutex<usize>,
    /// 全局带宽限制（bytes/s）
    global_bandwidth_limit: parking_lot::Mutex<u64>,
    /// 统计信息
    stats: Arc<MigrationStats>,
    /// 已完成任务保留上限
    max_completed_tasks: usize,
}

/// 迁移统计
#[derive(Debug, Default)]
pub struct MigrationStats {
    /// 总提交任务数
    pub tasks_submitted: parking_lot::Mutex<u64>,
    /// 已完成任务数
    pub tasks_completed: parking_lot::Mutex<u64>,
    /// 失败任务数
    pub tasks_failed: parking_lot::Mutex<u64>,
    /// 已取消任务数
    pub tasks_cancelled: parking_lot::Mutex<u64>,
    /// 总迁移字节数
    pub bytes_migrated: parking_lot::Mutex<u64>,
    /// 总验证字节数
    pub bytes_verified: parking_lot::Mutex<u64>,
    /// 失败重试总次数
    pub retries_total: parking_lot::Mutex<u64>,
}

impl MigrationStats {
    pub fn snapshot(&self) -> HashMap<String, u64> {
        let mut m = HashMap::new();
        m.insert(
            "migration_tasks_submitted".into(),
            *self.tasks_submitted.lock(),
        );
        m.insert(
            "migration_tasks_completed".into(),
            *self.tasks_completed.lock(),
        );
        m.insert(
            "migration_tasks_failed".into(),
            *self.tasks_failed.lock(),
        );
        m.insert(
            "migration_tasks_cancelled".into(),
            *self.tasks_cancelled.lock(),
        );
        m.insert(
            "migration_bytes_total".into(),
            *self.bytes_migrated.lock(),
        );
        m.insert(
            "migration_bytes_verified".into(),
            *self.bytes_verified.lock(),
        );
        m.insert(
            "migration_retries_total".into(),
            *self.retries_total.lock(),
        );
        m
    }
}

impl MigrationTaskManager {
    /// 创建迁移任务管理器
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            pending_queue: parking_lot::Mutex::new(VecDeque::new()),
            running_tasks: parking_lot::Mutex::new(HashMap::new()),
            completed_tasks: parking_lot::Mutex::new(Vec::new()),
            max_concurrent: parking_lot::Mutex::new(max_concurrent),
            global_bandwidth_limit: parking_lot::Mutex::new(0),
            stats: Arc::new(MigrationStats::default()),
            max_completed_tasks: 1000,
        }
    }

    /// 获取统计信息
    pub fn stats(&self) -> Arc<MigrationStats> {
        self.stats.clone()
    }

    /// 设置最大并发迁移数
    pub fn set_max_concurrent(&self, max: usize) {
        *self.max_concurrent.lock() = max;
    }

    /// 获取最大并发迁移数
    pub fn get_max_concurrent(&self) -> usize {
        *self.max_concurrent.lock()
    }

    /// 设置全局带宽限制
    pub fn set_global_bandwidth_limit(&self, limit_bps: u64) {
        *self.global_bandwidth_limit.lock() = limit_bps;
    }

    /// 获取全局带宽限制
    pub fn get_global_bandwidth_limit(&self) -> u64 {
        *self.global_bandwidth_limit.lock()
    }

    /// 提交迁移任务
    pub fn submit_task(&self, mut task: MigrationTask) -> String {
        // 设置默认优先级
        if task.priority == 0 {
            task.priority = task.migration_type.default_priority();
        }

        let task_id = task.task_id.clone();
        *self.stats.tasks_submitted.lock() += 1;

        let mut queue = self.pending_queue.lock();
        // 按优先级插入（高优先级在前）
        let mut insert_idx = queue.len();
        for (i, t) in queue.iter().enumerate() {
            if task.priority > t.priority {
                insert_idx = i;
                break;
            }
        }
        queue.insert(insert_idx, task);
        drop(queue);

        task_id
    }

    /// 获取下一个待执行的任务
    pub fn get_next_task(&self) -> Option<MigrationTask> {
        let running_count = self.running_tasks.lock().len();
        let max = *self.max_concurrent.lock();

        if running_count >= max {
            return None;
        }

        let mut queue = self.pending_queue.lock();
        let mut task = queue.pop_front()?;

        task.status = MigrationStatus::Running;
        task.started_at_ms = Some(now_ms());
        task.phase = MigrationPhase::Init;

        let task_id = task.task_id.clone();
        self.running_tasks.lock().insert(task_id, task.clone());

        Some(task)
    }

    /// 更新任务进度
    pub fn update_progress(&self, task_id: &str, migrated_bytes: u64) {
        let mut running = self.running_tasks.lock();
        if let Some(task) = running.get_mut(task_id) {
            task.migrated_bytes = migrated_bytes.min(task.total_bytes);
        }
    }

    /// 报告迁移进度（含阶段更新）
    pub fn report_progress(
        &self,
        task_id: &str,
        migrated_bytes: u64,
        phase: MigrationPhase,
    ) {
        let mut running = self.running_tasks.lock();
        if let Some(task) = running.get_mut(task_id) {
            task.migrated_bytes = migrated_bytes.min(task.total_bytes);
            task.phase = phase;
        }
    }

    /// 完成任务（成功）
    pub fn complete_task(&self, task_id: &str, verified_bytes: u64) {
        let mut running = self.running_tasks.lock();
        let mut task = match running.remove(task_id) {
            Some(t) => t,
            None => return,
        };
        drop(running);

        task.status = MigrationStatus::Completed;
        task.completed_at_ms = Some(now_ms());
        task.verified_bytes = verified_bytes;
        task.migrated_bytes = task.total_bytes;

        *self.stats.tasks_completed.lock() += 1;
        *self.stats.bytes_migrated.lock() += task.total_bytes;
        *self.stats.bytes_verified.lock() += verified_bytes;

        self.add_completed_task(task);
    }

    /// 任务失败
    pub fn fail_task(&self, task_id: &str, error: String) {
        let mut running = self.running_tasks.lock();
        let mut task = match running.remove(task_id) {
            Some(t) => t,
            None => return,
        };
        drop(running);

        task.retry_count += 1;
        task.last_error = Some(error);

        if task.retry_count < task.max_retries {
            // 重试：放回队列
            task.status = MigrationStatus::Pending;
            task.phase = MigrationPhase::Init;
            *self.stats.retries_total.lock() += 1;

            let mut queue = self.pending_queue.lock();
            queue.push_back(task);
        } else {
            // 超过最大重试次数，标记失败
            task.status = MigrationStatus::Failed;
            task.completed_at_ms = Some(now_ms());
            *self.stats.tasks_failed.lock() += 1;
            self.add_completed_task(task);
        }
    }

    /// 取消任务
    pub fn cancel_task(&self, task_id: &str) -> bool {
        // 先从 pending 队列中找
        {
            let mut queue = self.pending_queue.lock();
            let pos = queue.iter().position(|t| t.task_id == task_id);
            if let Some(idx) = pos {
                let mut task = queue.remove(idx).unwrap();
                task.status = MigrationStatus::Cancelled;
                task.completed_at_ms = Some(now_ms());
                drop(queue);
                *self.stats.tasks_cancelled.lock() += 1;
                self.add_completed_task(task);
                return true;
            }
        }

        // 再从 running 中找
        let mut running = self.running_tasks.lock();
        if let Some(mut task) = running.remove(task_id) {
            task.status = MigrationStatus::Cancelled;
            task.completed_at_ms = Some(now_ms());
            drop(running);
            *self.stats.tasks_cancelled.lock() += 1;
            self.add_completed_task(task);
            return true;
        }

        false
    }

    /// 暂停任务
    pub fn pause_task(&self, task_id: &str) -> bool {
        let mut running = self.running_tasks.lock();
        if let Some(task) = running.get_mut(task_id) {
            task.status = MigrationStatus::Paused;
            return true;
        }
        false
    }

    /// 恢复暂停的任务
    pub fn resume_task(&self, task_id: &str) -> bool {
        // 从 running 中找暂停的
        let mut running = self.running_tasks.lock();
        if let Some(task) = running.get_mut(task_id) {
            if task.status == MigrationStatus::Paused {
                task.status = MigrationStatus::Running;
                return true;
            }
        }
        false
    }

    /// 获取任务详情
    pub fn get_task(&self, task_id: &str) -> Option<MigrationTask> {
        // 先查 running
        if let Some(t) = self.running_tasks.lock().get(task_id) {
            return Some(t.clone());
        }
        // 再查 pending
        if let Some(t) = self
            .pending_queue
            .lock()
            .iter()
            .find(|t| t.task_id == task_id)
        {
            return Some(t.clone());
        }
        // 最后查 completed
        if let Some(t) = self
            .completed_tasks
            .lock()
            .iter()
            .find(|t| t.task_id == task_id)
        {
            return Some(t.clone());
        }
        None
    }

    /// 列出所有 pending 任务
    pub fn list_pending(&self) -> Vec<MigrationTask> {
        self.pending_queue.lock().iter().cloned().collect()
    }

    /// 列出所有 running 任务
    pub fn list_running(&self) -> Vec<MigrationTask> {
        self.running_tasks.lock().values().cloned().collect()
    }

    /// 列出已完成的任务
    pub fn list_completed(&self, limit: usize) -> Vec<MigrationTask> {
        let completed = self.completed_tasks.lock();
        completed.iter().rev().take(limit).cloned().collect()
    }

    /// 保存断点（用于断点续传）
    pub fn save_checkpoint(&self, task_id: &str, checkpoint: MigrationCheckpoint) {
        let mut running = self.running_tasks.lock();
        if let Some(task) = running.get_mut(task_id) {
            task.checkpoint = Some(checkpoint);
        }
    }

    /// 获取任务的断点信息
    pub fn get_checkpoint(&self, task_id: &str) -> Option<MigrationCheckpoint> {
        self.get_task(task_id).and_then(|t| t.checkpoint)
    }

    /// 计算当前整体迁移速度（bytes/s）
    /// 简化实现：返回 0，实际应基于时间窗口计算
    pub fn current_throughput_bps(&self) -> u64 {
        let running = self.running_tasks.lock();
        if running.is_empty() {
            return 0;
        }
        // 简化：假设每个任务平均 10MB/s
        running.len() as u64 * 10 * 1024 * 1024
    }

    /// 获取任务进度百分比
    pub fn get_progress_pct(&self, task_id: &str) -> Option<f64> {
        let task = self.get_task(task_id)?;
        if task.total_bytes == 0 {
            return Some(100.0);
        }
        Some(task.migrated_bytes as f64 / task.total_bytes as f64 * 100.0)
    }

    /// 清理已完成的任务（保留最近 N 个）
    fn add_completed_task(&self, task: MigrationTask) {
        let mut completed = self.completed_tasks.lock();
        completed.push(task);
        // 超出上限则移除最早的
        if completed.len() > self.max_completed_tasks {
            let excess = completed.len() - self.max_completed_tasks;
            completed.drain(0..excess);
        }
    }

    /// 计算每个运行中任务的可用带宽（均分全局限制）
    pub fn per_task_bandwidth_bps(&self) -> u64 {
        let global_limit = *self.global_bandwidth_limit.lock();
        if global_limit == 0 {
            return 0; // 0 表示不限制
        }
        let running_count = self.running_tasks.lock().len().max(1) as u64;
        global_limit / running_count
    }
}

impl Default for MigrationTaskManager {
    fn default() -> Self {
        Self::new(4)
    }
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: &str, mtype: MigrationType, size: u64) -> MigrationTask {
        MigrationTask {
            task_id: id.to_string(),
            migration_type: mtype,
            source_node_id: "src-node".to_string(),
            source_addr: "127.0.0.1:8000".to_string(),
            target_node_id: "tgt-node".to_string(),
            target_addr: "127.0.0.1:8001".to_string(),
            object_id: "obj-1".to_string(),
            total_bytes: size,
            migrated_bytes: 0,
            verified_bytes: 0,
            priority: 0,
            status: MigrationStatus::Pending,
            phase: MigrationPhase::Init,
            created_at_ms: now_ms(),
            started_at_ms: None,
            completed_at_ms: None,
            bandwidth_limit_bps: 0,
            retry_count: 0,
            max_retries: 3,
            last_error: None,
            checkpoint: None,
            delete_source_after: false,
            verify_after_migration: true,
            verification_method: VerificationMethod::Crc32c,
        }
    }

    #[test]
    fn test_migration_type_priority() {
        assert_eq!(MigrationType::FailureRecovery.default_priority(), 10);
        assert_eq!(MigrationType::NodeDecommission.default_priority(), 9);
        assert_eq!(MigrationType::CapacityRebalance.default_priority(), 4);
        assert_eq!(MigrationType::TierDown.default_priority(), 2);
    }

    #[test]
    fn test_migration_status_terminal() {
        assert!(MigrationStatus::Completed.is_terminal());
        assert!(MigrationStatus::Failed.is_terminal());
        assert!(MigrationStatus::Cancelled.is_terminal());
        assert!(!MigrationStatus::Pending.is_terminal());
        assert!(!MigrationStatus::Running.is_terminal());
    }

    #[test]
    fn test_migration_status_cancellable() {
        assert!(MigrationStatus::Pending.is_cancellable());
        assert!(MigrationStatus::Running.is_cancellable());
        assert!(MigrationStatus::Paused.is_cancellable());
        assert!(!MigrationStatus::Completed.is_cancellable());
    }

    #[test]
    fn test_submit_and_get_task() {
        let mgr = MigrationTaskManager::new(2);
        let id = mgr.submit_task(make_task("task-1", MigrationType::CapacityRebalance, 1024));
        assert_eq!(id, "task-1");

        let task = mgr.get_task("task-1").unwrap();
        assert_eq!(task.status, MigrationStatus::Pending);
        // 应该自动设置了优先级
        assert_eq!(task.priority, MigrationType::CapacityRebalance.default_priority());
    }

    #[test]
    fn test_priority_ordering() {
        let mgr = MigrationTaskManager::new(1);

        // 低优先级先提交
        mgr.submit_task(make_task("low", MigrationType::TierDown, 100));
        // 高优先级后提交
        mgr.submit_task(make_task("high", MigrationType::FailureRecovery, 100));

        // 应该先取到高优先级的
        let task = mgr.get_next_task().unwrap();
        assert_eq!(task.task_id, "high");
        assert_eq!(task.status, MigrationStatus::Running);
    }

    #[test]
    fn test_max_concurrent() {
        let mgr = MigrationTaskManager::new(2);

        for i in 0..5 {
            mgr.submit_task(make_task(&format!("task-{}", i), MigrationType::Manual, 100));
        }

        // 只能取 2 个
        assert!(mgr.get_next_task().is_some());
        assert!(mgr.get_next_task().is_some());
        assert!(mgr.get_next_task().is_none());

        assert_eq!(mgr.list_running().len(), 2);
        assert_eq!(mgr.list_pending().len(), 3);
    }

    #[test]
    fn test_complete_task() {
        let mgr = MigrationTaskManager::new(2);
        mgr.submit_task(make_task("task-1", MigrationType::Manual, 1024));

        let task = mgr.get_next_task().unwrap();
        mgr.complete_task(&task.task_id, 1024);

        let completed = mgr.list_completed(10);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].status, MigrationStatus::Completed);
        assert!(completed[0].completed_at_ms.is_some());
        assert_eq!(*mgr.stats().tasks_completed.lock(), 1);
    }

    #[test]
    fn test_fail_task_with_retry() {
        let mgr = MigrationTaskManager::new(2);
        let mut task = make_task("task-1", MigrationType::Manual, 1024);
        task.max_retries = 2;
        mgr.submit_task(task);

        // 第一次执行，失败
        let t = mgr.get_next_task().unwrap();
        mgr.fail_task(&t.task_id, "error 1".into());

        // 应该被放回 pending 队列（等待重试）
        let pending = mgr.list_pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].retry_count, 1);
        assert_eq!(*mgr.stats().retries_total.lock(), 1);

        // 第二次执行，再失败
        let t = mgr.get_next_task().unwrap();
        mgr.fail_task(&t.task_id, "error 2".into());

        // 超过最大重试，应该失败
        let completed = mgr.list_completed(10);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].status, MigrationStatus::Failed);
        assert_eq!(completed[0].retry_count, 2);
        assert_eq!(*mgr.stats().tasks_failed.lock(), 1);
    }

    #[test]
    fn test_cancel_pending_task() {
        let mgr = MigrationTaskManager::new(2);
        mgr.submit_task(make_task("task-1", MigrationType::Manual, 1024));

        assert!(mgr.cancel_task("task-1"));
        let completed = mgr.list_completed(10);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].status, MigrationStatus::Cancelled);
        assert_eq!(*mgr.stats().tasks_cancelled.lock(), 1);
    }

    #[test]
    fn test_cancel_running_task() {
        let mgr = MigrationTaskManager::new(2);
        mgr.submit_task(make_task("task-1", MigrationType::Manual, 1024));

        let task = mgr.get_next_task().unwrap();
        assert!(mgr.cancel_task(&task.task_id));
        assert_eq!(*mgr.stats().tasks_cancelled.lock(), 1);
    }

    #[test]
    fn test_cancel_nonexistent_task() {
        let mgr = MigrationTaskManager::new(2);
        assert!(!mgr.cancel_task("nonexistent"));
    }

    #[test]
    fn test_pause_and_resume() {
        let mgr = MigrationTaskManager::new(2);
        mgr.submit_task(make_task("task-1", MigrationType::Manual, 1024));

        let task = mgr.get_next_task().unwrap();
        assert!(mgr.pause_task(&task.task_id));

        let t = mgr.get_task(&task.task_id).unwrap();
        assert_eq!(t.status, MigrationStatus::Paused);

        assert!(mgr.resume_task(&task.task_id));
        let t = mgr.get_task(&task.task_id).unwrap();
        assert_eq!(t.status, MigrationStatus::Running);
    }

    #[test]
    fn test_update_progress() {
        let mgr = MigrationTaskManager::new(2);
        mgr.submit_task(make_task("task-1", MigrationType::Manual, 1000));

        let task = mgr.get_next_task().unwrap();
        mgr.update_progress(&task.task_id, 500);

        let t = mgr.get_task(&task.task_id).unwrap();
        assert_eq!(t.migrated_bytes, 500);

        let pct = mgr.get_progress_pct(&task.task_id).unwrap();
        assert!((pct - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_report_progress_with_phase() {
        let mgr = MigrationTaskManager::new(2);
        mgr.submit_task(make_task("task-1", MigrationType::Manual, 1000));

        let task = mgr.get_next_task().unwrap();
        mgr.report_progress(&task.task_id, 300, MigrationPhase::FullSync);

        let t = mgr.get_task(&task.task_id).unwrap();
        assert_eq!(t.phase, MigrationPhase::FullSync);
        assert_eq!(t.migrated_bytes, 300);
    }

    #[test]
    fn test_checkpoint_save_and_get() {
        let mgr = MigrationTaskManager::new(2);
        mgr.submit_task(make_task("task-1", MigrationType::Manual, 1000));

        let task = mgr.get_next_task().unwrap();

        let cp = MigrationCheckpoint {
            completed_offset: 500,
            last_block_index: 10,
            verified_offset: 400,
            current_phase: MigrationPhase::FullSync,
            saved_at_ms: now_ms(),
        };

        mgr.save_checkpoint(&task.task_id, cp);

        let retrieved = mgr.get_checkpoint(&task.task_id).unwrap();
        assert_eq!(retrieved.completed_offset, 500);
        assert_eq!(retrieved.last_block_index, 10);
    }

    #[test]
    fn test_migration_stats_snapshot() {
        let mgr = MigrationTaskManager::new(2);
        mgr.submit_task(make_task("t1", MigrationType::Manual, 100));
        mgr.submit_task(make_task("t2", MigrationType::Manual, 200));

        let t1 = mgr.get_next_task().unwrap();
        mgr.complete_task(&t1.task_id, 100);

        let snap = mgr.stats().snapshot();
        assert_eq!(snap["migration_tasks_submitted"], 2);
        assert_eq!(snap["migration_tasks_completed"], 1);
        assert_eq!(snap["migration_bytes_total"], 100);
    }

    #[test]
    fn test_set_max_concurrent() {
        let mgr = MigrationTaskManager::new(2);
        assert_eq!(mgr.get_max_concurrent(), 2);
        mgr.set_max_concurrent(10);
        assert_eq!(mgr.get_max_concurrent(), 10);
    }

    #[test]
    fn test_global_bandwidth_limit() {
        let mgr = MigrationTaskManager::new(2);
        assert_eq!(mgr.get_global_bandwidth_limit(), 0);

        mgr.set_global_bandwidth_limit(100 * 1024 * 1024); // 100MB/s
        assert_eq!(mgr.get_global_bandwidth_limit(), 100 * 1024 * 1024);
    }

    #[test]
    fn test_per_task_bandwidth() {
        let mgr = MigrationTaskManager::new(4);
        mgr.set_global_bandwidth_limit(100 * 1024 * 1024); // 100MB/s

        mgr.submit_task(make_task("t1", MigrationType::Manual, 100));
        mgr.submit_task(make_task("t2", MigrationType::Manual, 100));

        let _ = mgr.get_next_task();
        let _ = mgr.get_next_task();

        // 2 个任务，均分 100MB/s = 每个 50MB/s
        let per_task = mgr.per_task_bandwidth_bps();
        assert_eq!(per_task, 50 * 1024 * 1024);
    }

    #[test]
    fn test_migration_type_display() {
        assert_eq!(MigrationType::CapacityRebalance.to_string(), "capacity_rebalance");
        assert_eq!(MigrationType::FailureRecovery.to_string(), "failure_recovery");
    }

    #[test]
    fn test_zero_size_progress() {
        let mgr = MigrationTaskManager::new(2);
        let mut task = make_task("t1", MigrationType::Manual, 0);
        task.total_bytes = 0;
        mgr.submit_task(task);
        mgr.complete_task("t1", 0);

        // 0 字节任务进度应该是 100%
        let pct = mgr.get_progress_pct("t1").unwrap();
        assert_eq!(pct, 100.0);
    }

    #[test]
    fn test_verification_result() {
        let result = VerificationResult {
            passed: true,
            method: VerificationMethod::Crc32c,
            source_checksum: "abc123".to_string(),
            target_checksum: "abc123".to_string(),
            bytes_verified: 1024,
            duration_ms: 5,
            error: None,
        };
        assert!(result.passed);
        assert_eq!(result.bytes_verified, 1024);
    }

    #[test]
    fn test_completed_tasks_limit() {
        let mgr = MigrationTaskManager::new(10);
        // 提交并完成 2000 个任务（超过默认 1000 上限）
        for i in 0..2000 {
            let id = format!("task-{}", i);
            mgr.submit_task(make_task(&id, MigrationType::Manual, 100));
            let t = mgr.get_next_task().unwrap();
            mgr.complete_task(&t.task_id, 100);
        }

        let completed = mgr.list_completed(5000);
        assert_eq!(completed.len(), 1000); // 最多保留 1000 个
    }
}
