// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS)
// Licensed under the MIT License.

//! 统一计算引擎 — 多计算模型统一执行
//!
//! 支持 BSP、GAS、流式、SIMD、多线程等多种计算模型，
//! 算法可以声明支持的计算模型，由计算引擎自动选择最优执行方式。

use crate::error::{AlgoError, AlgoResult};
use crate::types::ComputeModel;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    /// 低优先级
    Low = 0,
    /// 普通优先级
    Normal = 1,
    /// 高优先级
    High = 2,
    /// 实时优先级（最高）
    Realtime = 3,
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

/// 计算任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// 等待中
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 计算任务
pub struct ComputeTask {
    /// 任务 ID
    pub id: u64,
    /// 任务名称
    pub name: String,
    /// 优先级
    pub priority: TaskPriority,
    /// 状态
    pub status: TaskStatus,
    /// 期望的计算模型
    pub preferred_model: ComputeModel,
    /// 预计计算量（用于调度）
    pub estimated_work: u64,
}

/// 计算引擎配置
#[derive(Debug, Clone)]
pub struct ComputeEngineConfig {
    /// 工作线程数
    pub worker_threads: usize,
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
    /// 支持的计算模型
    pub supported_models: Vec<ComputeModel>,
    /// GPU 设备数（如果有）
    pub gpu_devices: usize,
}

impl Default for ComputeEngineConfig {
    fn default() -> Self {
        Self {
            worker_threads: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
            max_concurrent_tasks: 64,
            supported_models: vec![
                ComputeModel::SingleThread,
                ComputeModel::MultiThread,
                ComputeModel::SIMD,
                ComputeModel::BSP,
                ComputeModel::GAS,
            ],
            gpu_devices: 0,
        }
    }
}

/// 计算引擎统计
#[derive(Debug, Default)]
pub struct ComputeEngineStats {
    /// 已提交任务数
    pub tasks_submitted: AtomicU64,
    /// 已完成任务数
    pub tasks_completed: AtomicU64,
    /// 失败任务数
    pub tasks_failed: AtomicU64,
    /// 总计算量（估算）
    pub total_compute_units: AtomicU64,
}

impl ComputeEngineStats {
    /// 快照
    pub fn snapshot(&self) -> HashMap<String, u64> {
        let mut m = HashMap::new();
        m.insert(
            "tasks_submitted".to_string(),
            self.tasks_submitted.load(Ordering::SeqCst),
        );
        m.insert(
            "tasks_completed".to_string(),
            self.tasks_completed.load(Ordering::SeqCst),
        );
        m.insert(
            "tasks_failed".to_string(),
            self.tasks_failed.load(Ordering::SeqCst),
        );
        m.insert(
            "total_compute_units".to_string(),
            self.total_compute_units.load(Ordering::SeqCst),
        );
        m
    }
}

/// 统一计算引擎
///
/// 负责调度和执行所有算法的计算任务。
/// 支持多种计算模型的统一调度，自动选择最优执行路径。
pub struct ComputeEngine {
    /// 配置
    config: ComputeEngineConfig,
    /// 统计信息
    pub stats: Arc<ComputeEngineStats>,
    /// 下一个任务 ID
    next_task_id: AtomicU64,
    /// 运行中的任务
    active_tasks: RwLock<HashMap<u64, ComputeTask>>,
}

impl ComputeEngine {
    /// 使用默认配置创建计算引擎
    pub fn new() -> Self {
        Self::with_config(ComputeEngineConfig::default())
    }

    /// 使用指定配置创建计算引擎
    pub fn with_config(config: ComputeEngineConfig) -> Self {
        Self {
            config,
            stats: Arc::new(ComputeEngineStats::default()),
            next_task_id: AtomicU64::new(1),
            active_tasks: RwLock::new(HashMap::new()),
        }
    }

    /// 获取配置
    pub fn config(&self) -> &ComputeEngineConfig {
        &self.config
    }

    /// 是否支持指定的计算模型
    pub fn supports_model(&self, model: ComputeModel) -> bool {
        self.config.supported_models.contains(&model)
    }

    /// 获取支持的计算模型列表
    pub fn supported_models(&self) -> &[ComputeModel] {
        &self.config.supported_models
    }

    /// 工作线程数
    pub fn worker_count(&self) -> usize {
        self.config.worker_threads
    }

    /// 提交计算任务（同步执行包装）
    pub fn submit_task(&self, name: &str, priority: TaskPriority, estimated_work: u64) -> u64 {
        let id = self.next_task_id.fetch_add(1, Ordering::SeqCst);
        let task = ComputeTask {
            id,
            name: name.to_string(),
            priority,
            status: TaskStatus::Running,
            preferred_model: ComputeModel::MultiThread,
            estimated_work,
        };

        self.stats.tasks_submitted.fetch_add(1, Ordering::SeqCst);
        self.active_tasks.write().insert(id, task);
        id
    }

    /// 标记任务完成
    pub fn complete_task(&self, task_id: u64, success: bool) {
        if success {
            self.stats.tasks_completed.fetch_add(1, Ordering::SeqCst);
        } else {
            self.stats.tasks_failed.fetch_add(1, Ordering::SeqCst);
        }
        self.active_tasks.write().remove(&task_id);
    }

    /// 更新任务计算量
    pub fn add_compute_units(&self, units: u64) {
        self.stats.total_compute_units.fetch_add(units, Ordering::SeqCst);
    }

    /// 活跃任务数
    pub fn active_task_count(&self) -> usize {
        self.active_tasks.read().len()
    }

    /// 选择最优计算模型
    ///
    /// 从算法支持的模型列表中选择当前引擎支持的最优模型
    pub fn select_best_model(&self, supported: &[ComputeModel]) -> AlgoResult<ComputeModel> {
        // 优先级排序：GPU > SIMD > MultiThread > GAS > BSP > Streaming > SingleThread
        let priority_order = [
            ComputeModel::GPU,
            ComputeModel::SIMD,
            ComputeModel::MultiThread,
            ComputeModel::GAS,
            ComputeModel::BSP,
            ComputeModel::Streaming,
            ComputeModel::SingleThread,
        ];

        for model in priority_order.iter() {
            if supported.contains(model) && self.supports_model(*model) {
                return Ok(*model);
            }
        }

        // 最低要求：单线程
        if supported.contains(&ComputeModel::SingleThread) {
            return Ok(ComputeModel::SingleThread);
        }

        Err(AlgoError::ComputeEngineError(
            "no supported compute model found".to_string(),
        ))
    }

    /// 执行一个计算闭包（带统计）
    pub fn execute<F, R>(&self, name: &str, priority: TaskPriority, f: F) -> AlgoResult<R>
    where
        F: FnOnce() -> R,
    {
        let task_id = self.submit_task(name, priority, 100);
        let result = f();
        self.complete_task(task_id, true);
        self.add_compute_units(100);
        Ok(result)
    }
}

impl Default for ComputeEngine {
    fn default() -> Self {
        Self::new()
    }
}

// 需要 num_cpus 或使用 std::thread::available_parallelism
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_engine_new() {
        let engine = ComputeEngine::new();
        assert!(engine.worker_count() >= 1);
        assert!(engine.supports_model(ComputeModel::SingleThread));
        assert!(engine.supports_model(ComputeModel::MultiThread));
    }

    #[test]
    fn test_submit_and_complete() {
        let engine = ComputeEngine::new();

        let id = engine.submit_task("test", TaskPriority::Normal, 100);
        assert_eq!(id, 1);
        assert_eq!(engine.active_task_count(), 1);

        engine.complete_task(id, true);
        assert_eq!(engine.active_task_count(), 0);

        let stats = engine.stats.snapshot();
        assert_eq!(stats["tasks_submitted"], 1);
        assert_eq!(stats["tasks_completed"], 1);
        assert_eq!(stats["tasks_failed"], 0);
    }

    #[test]
    fn test_select_best_model() {
        let engine = ComputeEngine::new();

        // 支持 SIMD 时选 SIMD
        let models = vec![ComputeModel::SingleThread, ComputeModel::SIMD];
        let best = engine.select_best_model(&models).unwrap();
        assert_eq!(best, ComputeModel::SIMD);

        // 只支持单线程时选单线程
        let models = vec![ComputeModel::SingleThread];
        let best = engine.select_best_model(&models).unwrap();
        assert_eq!(best, ComputeModel::SingleThread);

        // GPU 不支持时降级
        let models = vec![ComputeModel::GPU, ComputeModel::MultiThread];
        let best = engine.select_best_model(&models).unwrap();
        assert_eq!(best, ComputeModel::MultiThread);
    }

    #[test]
    fn test_execute_closure() {
        let engine = ComputeEngine::new();
        let result = engine
            .execute("add", TaskPriority::Normal, || 1 + 2)
            .unwrap();
        assert_eq!(result, 3);

        let stats = engine.stats.snapshot();
        assert_eq!(stats["tasks_completed"], 1);
        assert_eq!(stats["total_compute_units"], 100);
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Realtime > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }
}
