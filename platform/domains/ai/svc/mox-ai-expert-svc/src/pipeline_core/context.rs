// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 管线上下文（PipelineContext）
//!
//! 贯穿整个管线执行过程的共享状态容器。
//! 所有阶段处理器都通过它读取输入、写入输出、访问共享服务。
//!
//! 设计目标：
//! - 统一两套管线的上下文（GovernContext + AllianceRequest）
//! - 阶段间通过结果映射传递数据，而非硬编码字段
//! - 支持扩展 bag（类型化键值存储）
//! - 内置审计链（内部哈希链 + 外部 sink 双写）
//! - 与 HarnessCtx 解耦但可桥接（插件运行时作为可选服务注入）

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::pipeline_core::audit::UnifiedAuditChain;
use crate::pipeline_core::phase::{Phase, PhaseExecution, PhaseStatus};
use crate::pipeline_core::result::PhaseResult;

// ================== 上下文输入类型 ==================

/// 管线输入：统一两套管线的请求模型
///
/// mox 模块化系统架构管线的输入是 FlowGraph + GovernContext
/// 联盟管线的输入是 AllianceRequest (自然语言 query)
///
/// 这里用 enum 统一，各管线实现从中提取自己需要的部分。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PipelineInput {
    /// 流程图输入（mox 模块化系统架构管线）
    FlowGraph {
        flow_id: String,
        flow_name: String,
        /// 序列化的 FlowGraph（避免直接依赖 mox_ai_flow_svc）
        graph: serde_json::Value,
    },
    /// 自然语言查询输入（联盟管线）
    Query {
        query: String,
        session_id: Option<String>,
        context: HashMap<String, String>,
    },
    /// 混合输入（既有图又有查询描述）
    Mixed {
        flow_id: String,
        query: String,
        session_id: Option<String>,
        context: HashMap<String, String>,
    },
}

impl PipelineInput {
    pub fn id(&self) -> &str {
        match self {
            Self::FlowGraph { flow_id, .. } => flow_id,
            Self::Query { query, .. } => query, // query 作为临时 id
            Self::Mixed { flow_id, .. } => flow_id,
        }
    }
}

/// 运行时选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOptions {
    /// 是否启用详细日志
    #[serde(default)]
    pub verbose: bool,
    /// 闸门 C 级是否重试
    #[serde(default = "default_true")]
    pub retry_on_c: bool,
    /// 最大重试次数
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// 单阶段超时（毫秒），0 表示无超时
    #[serde(default)]
    pub phase_timeout_ms: u64,
    /// 是否启用审计
    #[serde(default = "default_true")]
    pub audit_enabled: bool,
}

fn default_true() -> bool {
    true
}

fn default_max_retries() -> u32 {
    1
}

impl Default for PipelineOptions {
    fn default() -> Self {
        Self {
            verbose: false,
            retry_on_c: true,
            max_retries: 1,
            phase_timeout_ms: 0,
            audit_enabled: true,
        }
    }
}

// ================== PipelineContext ==================

/// 管线上下文：贯穿所有阶段的共享状态
///
/// # 生命周期
///
/// 1. 创建：管线启动时创建，注入输入和选项
/// 2. 流动：每个阶段从 ctx 读取前序阶段结果，写入自己的结果
/// 3. 完成：管线结束后，调用方可从 ctx 提取最终结果
///
/// # 线程安全
///
/// 上下文本身不是线程安全的（非 Sync），因为阶段是顺序执行的。
/// 并行专家分析等并发场景应在阶段处理器内部管理自己的并发，
/// 结果汇总后再写回上下文。
pub struct PipelineContext {
    // ---- 标识 ----
    /// 全局唯一 trace id（UUID v4），贯穿整条管线
    pub trace_id: Uuid,

    // ---- 输入 ----
    /// 管线输入
    pub input: PipelineInput,
    /// 运行时选项
    pub options: PipelineOptions,

    // ---- 租户/主体/配额（治理上下文投影） ----
    /// 租户 ID
    pub tenant_id: String,
    /// 主体（谁在调用）
    pub principal: String,
    /// 角色列表
    pub roles: Vec<String>,

    // ---- 阶段结果 ----
    /// 各阶段结果映射（phase -> result）
    phase_results: HashMap<Phase, Box<dyn PhaseResult>>,
    /// 阶段执行记录（用于审计和进度展示）
    phase_executions: HashMap<Phase, PhaseExecution>,

    // ---- 审计 ----
    /// 审计链（内部哈希链 + 外部 sink 桥接）
    pub audit: UnifiedAuditChain,

    // ---- 扩展存储 ----
    /// 类型化服务注册表（从 harness 迁移，用于共享服务）
    services: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    /// 扩展 bag：插件/阶段间传递任意数据
    /// 使用 String key + type erased value，需手动 downcast
    bag: HashMap<String, Box<dyn Any + Send + Sync>>,

    // ---- 计时 ----
    /// 管线启动时间
    pub started_at: Instant,
    /// 当前阶段
    pub current_phase: Option<Phase>,
}

impl std::fmt::Debug for PipelineContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineContext")
            .field("trace_id", &self.trace_id)
            .field("input", &self.input)
            .field("tenant_id", &self.tenant_id)
            .field("principal", &self.principal)
            .field("phase_count", &self.phase_results.len())
            .field("audit", &"<UnifiedAuditChain>")
            .field("elapsed_ms", &self.started_at.elapsed().as_millis())
            .finish()
    }
}

impl PipelineContext {
    /// 创建新的管线上下文
    pub fn new(input: PipelineInput, options: PipelineOptions) -> Self {
        let trace_id = Uuid::new_v4();
        Self {
            trace_id,
            input,
            options,
            tenant_id: String::new(),
            principal: String::new(),
            roles: Vec::new(),
            phase_results: HashMap::new(),
            phase_executions: HashMap::new(),
            audit: UnifiedAuditChain::new(),
            services: HashMap::new(),
            bag: HashMap::new(),
            started_at: Instant::now(),
            current_phase: None,
        }
    }

    /// 设置租户和主体信息
    pub fn with_identity(
        mut self,
        tenant_id: impl Into<String>,
        principal: impl Into<String>,
        roles: Vec<String>,
    ) -> Self {
        self.tenant_id = tenant_id.into();
        self.principal = principal.into();
        self.roles = roles;
        self
    }

    // ---- 阶段结果管理 ----

    /// 存储阶段结果
    pub fn set_result(&mut self, result: Box<dyn PhaseResult>) {
        let phase = result.phase();
        self.phase_executions
            .insert(phase, result.execution().clone());
        self.phase_results.insert(phase, result);
    }

    /// 获取阶段结果（类型擦除）
    pub fn get_result(&self, phase: Phase) -> Option<&dyn PhaseResult> {
        self.phase_results.get(&phase).map(|b| b.as_ref())
    }

    /// 获取阶段结果（类型安全的 downcast）
    pub fn get_result_typed<T: 'static>(&self, phase: Phase) -> Option<&T>
    where
        T: PhaseResult,
    {
        self.phase_results
            .get(&phase)
            .and_then(|r| r.as_any().downcast_ref::<T>())
    }

    /// 获取所有已完成的阶段
    pub fn completed_phases(&self) -> Vec<Phase> {
        let mut phases: Vec<Phase> = self.phase_results.keys().copied().collect();
        // 按执行时间排序（此处用插入顺序近似，可优化为按 latency 排序）
        phases.sort_by_key(|p| self.phase_executions.get(p).map(|e| e.latency_ms).unwrap_or(0));
        phases
    }

    /// 获取阶段执行记录
    pub fn get_execution(&self, phase: Phase) -> Option<&PhaseExecution> {
        self.phase_executions.get(&phase)
    }

    /// 记录阶段开始
    pub fn mark_phase_start(&mut self, phase: Phase) {
        self.current_phase = Some(phase);
        self.phase_executions.insert(
            phase,
            PhaseExecution {
                phase,
                status: PhaseStatus::Running,
                latency_ms: 0,
                degraded: false,
                degrade_reason: None,
                error: None,
            },
        );
    }

    /// 记录阶段结束
    pub fn mark_phase_end(&mut self, phase: Phase, status: PhaseStatus, latency_ms: u64) {
        if let Some(exec) = self.phase_executions.get_mut(&phase) {
            exec.status = status;
            exec.latency_ms = latency_ms;
        }
        if self.current_phase == Some(phase) {
            self.current_phase = None;
        }
    }

    // ---- 服务注册表 ----

    /// 注册一个类型化服务
    pub fn provide_service<T: Any + Send + Sync + 'static>(&mut self, svc: T) {
        self.services
            .insert(TypeId::of::<T>(), Box::new(svc));
    }

    /// 获取一个类型化服务
    pub fn get_service<T: Any + Send + Sync + 'static>(&self) -> Option<&T> {
        self.services
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<T>())
    }

    // ---- 扩展 Bag ----

    /// 向 bag 中存入一个值
    pub fn set_bag<T: Any + Send + Sync + 'static>(&mut self, key: impl Into<String>, value: T) {
        self.bag.insert(key.into(), Box::new(value));
    }

    /// 从 bag 中取出一个值
    pub fn get_bag<T: Any + Send + Sync + 'static>(&self, key: &str) -> Option<&T> {
        self.bag.get(key).and_then(|b| b.downcast_ref::<T>())
    }

    /// 检查 bag 中是否有指定 key
    pub fn has_bag(&self, key: &str) -> bool {
        self.bag.contains_key(key)
    }

    // ---- 辅助方法 ----

    /// 管线总耗时
    pub fn total_elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// 是否所有阶段都成功
    pub fn all_succeeded(&self) -> bool {
        self.phase_executions
            .values()
            .all(|e| e.status.is_success() || e.status == PhaseStatus::Skipped)
    }

    /// 是否被某个阶段阻断
    pub fn is_blocked(&self) -> bool {
        self.phase_executions
            .values()
            .any(|e| e.status.is_blocked())
    }
}
