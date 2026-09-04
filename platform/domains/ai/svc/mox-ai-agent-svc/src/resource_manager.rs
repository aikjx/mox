// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 全资源管理系统
//!
//! 统一管理CPU、内存、GPU、插件、算子、工作流等所有资源
//! 实现资源分配、回收、监控、调度和配额管理

use super::types::*;
use chrono::Utc;
use mox_platform_operator_core::{OperatorError, Result};
use std::collections::HashMap;
use tracing;
use uuid::Uuid;

/// 全资源管理器 - 统一资源调度核心
pub struct ResourceManager {
    /// 资源总量配置
    totals: HashMap<ResourceType, f64>,
    /// 当前分配
    allocations: Vec<ResourceAllocation>,
    /// 资源限制
    limits: HashMap<ResourceType, f64>,
    /// 算子缓存
    operator_cache: HashMap<String, CachedOperator>,
    /// 插件注册表
    registered_plugins: HashMap<String, PluginInfo>,
    /// 运行中工作流
    active_workflows: HashMap<String, WorkflowContext>,
    /// 资源使用历史
    usage_history: Vec<ResourceUsageSnapshot>,
}

/// 缓存的算子
#[derive(Debug, Clone)]
struct CachedOperator {
    operator_id: String,
    loaded_at: chrono::DateTime<Utc>,
    last_used: chrono::DateTime<Utc>,
    usage_count: u64,
    memory_footprint: u64,
}

/// 工作流上下文
#[derive(Debug, Clone)]
struct WorkflowContext {
    workflow_id: String,
    started_at: chrono::DateTime<Utc>,
    resources_held: Vec<String>,
    status: String,
}

/// 资源使用快照（公开 API：`ResourceManager::get_usage_history` 以 `Vec<ResourceUsageSnapshot>` 返回，故需 `pub`）
#[derive(Debug, Clone)]
#[allow(dead_code)] // 公开快照结构：字段由外部消费方读取，库内仅构造
pub struct ResourceUsageSnapshot {
    timestamp: chrono::DateTime<Utc>,
    usage: HashMap<ResourceType, f64>,
}

impl ResourceManager {
    pub fn new() -> Self {
        let mut totals = HashMap::new();
        let mut limits = HashMap::new();

        // 初始化默认资源
        totals.insert(ResourceType::Cpu, num_cpus::get() as f64);
        totals.insert(ResourceType::Memory, 1024.0 * 1024.0 * 1024.0); // 1GB
        totals.insert(ResourceType::Gpu, 0.0); // 默认无GPU
        totals.insert(ResourceType::DiskIo, 100.0 * 1024.0 * 1024.0); // 100MB
        totals.insert(ResourceType::Network, 100.0 * 1024.0 * 1024.0); // 100MB
        totals.insert(ResourceType::Plugin, 64.0); // 最多64个插件
        totals.insert(ResourceType::Operator, 256.0); // 最多256个缓存算子
        totals.insert(ResourceType::Workflow, 32.0); // 最多32个并行工作流

        for (rt, total) in &totals {
            limits.insert(rt.clone(), *total * 0.9); // 90%软限制
        }

        Self {
            totals,
            allocations: Vec::new(),
            limits,
            operator_cache: HashMap::new(),
            registered_plugins: HashMap::new(),
            active_workflows: HashMap::new(),
            usage_history: Vec::new(),
        }
    }

    /// 分配资源
    pub fn allocate(
        &mut self,
        owner_id: &str,
        owner_type: &str,
        resource_type: ResourceType,
        amount: f64,
    ) -> Result<ResourceAllocation> {
        tracing::debug!(
            "分配资源: owner={}, type={:?}, amount={}",
            owner_id,
            resource_type,
            amount
        );

        let current_used = self.calculate_usage(&resource_type);
        let limit = self.limits.get(&resource_type).copied().unwrap_or(f64::MAX);
        let total = self.totals.get(&resource_type).copied().unwrap_or(f64::MAX);

        if current_used + amount > limit {
            return Err(OperatorError::ResourceExhausted {
                required: format!("{:?}:{}", resource_type, amount),
                available: format!("{:?}:{}", resource_type, total - current_used),
            });
        }

        let allocation = ResourceAllocation {
            id: Uuid::new_v4().to_string(),
            resource_type: resource_type.clone(),
            owner_id: owner_id.to_string(),
            owner_type: owner_type.to_string(),
            amount,
            allocated_at: Utc::now(),
            expires_at: None,
            metadata: HashMap::new(),
        };

        self.allocations.push(allocation.clone());
        self.take_snapshot();

        Ok(allocation)
    }

    /// 释放资源
    pub fn release(&mut self, allocation_id: &str) -> bool {
        let before = self.allocations.len();
        self.allocations.retain(|a| a.id != allocation_id);
        let released = self.allocations.len() < before;
        if released {
            self.take_snapshot();
        }
        released
    }

    /// 释放所有者的所有资源
    pub fn release_all_owner(&mut self, owner_id: &str) -> usize {
        let before = self.allocations.len();
        self.allocations.retain(|a| a.owner_id != owner_id);
        let released = before - self.allocations.len();
        if released > 0 {
            self.take_snapshot();
        }
        released
    }

    /// 注册插件
    pub fn register_plugin(&mut self, plugin: PluginInfo) -> Result<()> {
        // 分配插件资源
        self.allocate(&plugin.id, "plugin", ResourceType::Plugin, 1.0)?;

        // 分配内存资源给插件
        self.allocate(
            &plugin.id,
            "plugin",
            ResourceType::Memory,
            10.0 * 1024.0 * 1024.0, // 10MB per plugin
        )?;

        self.registered_plugins.insert(plugin.id.clone(), plugin);
        tracing::info!(
            "插件注册成功，当前插件数: {}",
            self.registered_plugins.len()
        );
        Ok(())
    }

    /// 注销插件
    pub fn unregister_plugin(&mut self, plugin_id: &str) -> bool {
        self.release_all_owner(plugin_id);
        self.registered_plugins.remove(plugin_id).is_some()
    }

    /// 缓存算子
    pub fn cache_operator(&mut self, operator_id: &str, memory_footprint: u64) -> Result<()> {
        if self.operator_cache.len() as f64
            >= *self.totals.get(&ResourceType::Operator).unwrap_or(&256.0)
        {
            // LRU淘汰
            self.evict_lru_operator();
        }

        self.allocate(
            operator_id,
            "operator",
            ResourceType::Memory,
            memory_footprint as f64,
        )?;

        let now = Utc::now();
        self.operator_cache.insert(
            operator_id.to_string(),
            CachedOperator {
                operator_id: operator_id.to_string(),
                loaded_at: now,
                last_used: now,
                usage_count: 0,
                memory_footprint,
            },
        );

        Ok(())
    }

    /// 使用算子（更新LRU）
    pub fn touch_operator(&mut self, operator_id: &str) {
        if let Some(cached) = self.operator_cache.get_mut(operator_id) {
            cached.last_used = Utc::now();
            cached.usage_count += 1;
        }
    }

    /// LRU淘汰算子
    fn evict_lru_operator(&mut self) {
        if let Some(lru_id) = self
            .operator_cache
            .values()
            .min_by_key(|c| c.last_used)
            .map(|c| c.operator_id.clone())
        {
            tracing::info!("LRU淘汰算子: {}", lru_id);
            self.release_all_owner(&lru_id);
            self.operator_cache.remove(&lru_id);
        }
    }

    /// 开始工作流
    pub fn start_workflow(&mut self, workflow_id: &str) -> Result<()> {
        if self.active_workflows.len() as f64
            >= *self.totals.get(&ResourceType::Workflow).unwrap_or(&32.0)
        {
            return Err(OperatorError::ResourceExhausted {
                required: "Workflow slot".to_string(),
                available: "0".to_string(),
            });
        }

        self.allocate(workflow_id, "workflow", ResourceType::Workflow, 1.0)?;

        self.active_workflows.insert(
            workflow_id.to_string(),
            WorkflowContext {
                workflow_id: workflow_id.to_string(),
                started_at: Utc::now(),
                resources_held: Vec::new(),
                status: "running".to_string(),
            },
        );

        tracing::info!(
            "工作流启动: {}, 当前活动数: {}",
            workflow_id,
            self.active_workflows.len()
        );
        Ok(())
    }

    /// 结束工作流
    pub fn end_workflow(&mut self, workflow_id: &str) {
        self.release_all_owner(workflow_id);
        self.active_workflows.remove(workflow_id);
        tracing::info!(
            "工作流结束: {}, 当前活动数: {}",
            workflow_id,
            self.active_workflows.len()
        );
    }

    /// 获取资源全景
    pub fn get_panorama(&self) -> ResourcePanorama {
        let mut resources = HashMap::new();

        for (rt, total) in &self.totals {
            let used = self.calculate_usage(rt);
            resources.insert(
                format!("{:?}", rt),
                ResourceUsageStats {
                    resource_type: rt.clone(),
                    total: *total,
                    used,
                    available: total - used,
                    utilization_percent: if *total > 0.0 {
                        used / total * 100.0
                    } else {
                        0.0
                    },
                    allocations: self
                        .allocations
                        .iter()
                        .filter(|a| a.resource_type == *rt)
                        .cloned()
                        .collect(),
                },
            );
        }

        ResourcePanorama {
            timestamp: Utc::now(),
            resources,
            active_plugins: self.registered_plugins.len(),
            active_workflows: self.active_workflows.len(),
            cached_operators: self.operator_cache.len(),
            total_allocations: self.allocations.len(),
        }
    }

    /// 计算资源使用量
    fn calculate_usage(&self, resource_type: &ResourceType) -> f64 {
        self.allocations
            .iter()
            .filter(|a| a.resource_type == *resource_type)
            .map(|a| a.amount)
            .sum()
    }

    /// 记录使用快照
    fn take_snapshot(&mut self) {
        let mut usage = HashMap::new();
        for rt in self.totals.keys() {
            usage.insert(rt.clone(), self.calculate_usage(rt));
        }
        self.usage_history.push(ResourceUsageSnapshot {
            timestamp: Utc::now(),
            usage,
        });
        // 只保留最近1000个快照
        if self.usage_history.len() > 1000 {
            self.usage_history.remove(0);
        }
    }

    /// 获取资源使用历史
    pub fn get_usage_history(&self, limit: usize) -> Vec<ResourceUsageSnapshot> {
        self.usage_history
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// 检查资源健康状态
    pub fn health_check(&self) -> ResourceHealthReport {
        let mut warnings = Vec::new();
        let mut critical = Vec::new();

        for (rt, total) in &self.totals {
            let used = self.calculate_usage(rt);
            let utilization = if *total > 0.0 { used / total } else { 0.0 };

            if utilization > 0.95 {
                critical.push(format!(
                    "{:?} 资源严重不足: {:.1}%",
                    rt,
                    utilization * 100.0
                ));
            } else if utilization > 0.8 {
                warnings.push(format!(
                    "{:?} 资源使用较高: {:.1}%",
                    rt,
                    utilization * 100.0
                ));
            }
        }

        // 僵尸工作流检测：running 超过 30 分钟的未结束工作流（消费 WorkflowContext 运行态字段）
        let stale_timeout = chrono::Duration::minutes(30);
        for (wf_id, ctx) in &self.active_workflows {
            let _ = wf_id; // key 即 id；字段本身用于状态审计
            if ctx.status == "running"
                && Utc::now().signed_duration_since(ctx.started_at) > stale_timeout
            {
                warnings.push(format!(
                    "僵尸工作流: {} 已运行超过 30 分钟（持有 {} 项资源）",
                    ctx.workflow_id,
                    ctx.resources_held.len()
                ));
            }
        }

        // 缓存算子老化审计：loaded_at 超过 24h 且近期未使用的算子建议重载
        // （消费 CachedOperator.loaded_at / last_used 字段）
        let stale_cache = chrono::Duration::hours(24);
        for (op_id, cached) in &self.operator_cache {
            let idle = Utc::now().signed_duration_since(cached.last_used);
            if Utc::now().signed_duration_since(cached.loaded_at) > stale_cache
                && idle > stale_cache
            {
                tracing::warn!(
                    "缓存算子 {} 已加载超 24h 且闲置（loaded_at={}, last_used={}），建议重载",
                    op_id,
                    cached.loaded_at,
                    cached.last_used
                );
            }
        }

        // 缓存算子内存审计：汇总 loaded 算子的 memory_footprint（消费 CachedOperator 字段）
        let cached_mem_bytes: u64 = self
            .operator_cache
            .values()
            .map(|c| c.memory_footprint)
            .sum();
        if cached_mem_bytes > 256 * 1024 * 1024 {
            warnings.push(format!(
                "算子缓存内存占用偏高: {:.1} MB（{} 个缓存算子）",
                cached_mem_bytes as f64 / 1024.0 / 1024.0,
                self.operator_cache.len()
            ));
        }

        ResourceHealthReport {
            healthy: critical.is_empty(),
            warnings,
            critical,
            active_plugins: self.registered_plugins.len(),
            active_workflows: self.active_workflows.len(),
            cached_operators: self.operator_cache.len(),
        }
    }

    /// 获取所有注册插件
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.registered_plugins.values().cloned().collect()
    }

    /// 更新资源配额
    pub fn set_quota(&mut self, resource_type: ResourceType, total: f64) {
        self.totals.insert(resource_type.clone(), total);
        self.limits.insert(resource_type, total * 0.9);
    }
}

/// 资源健康报告
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceHealthReport {
    pub healthy: bool,
    pub warnings: Vec<String>,
    pub critical: Vec<String>,
    pub active_plugins: usize,
    pub active_workflows: usize,
    pub cached_operators: usize,
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

// CPU核心数检测模块
mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    }
}
