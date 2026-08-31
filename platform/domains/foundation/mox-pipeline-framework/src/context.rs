// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 管线上下文（PipelineContext）
//!
//! 贯穿整个管线执行过程的共享状态容器。
//! 所有阶段处理器都通过它读取输入、写入输出、访问共享服务。
//!
//! # 设计目标
//!
//! - 通用的管线上下文模型，不依赖任何领域类型
//! - 阶段间通过结果映射传递数据，而非硬编码字段
//! - 支持扩展 bag（类型化键值存储）
//! - 内置审计链（内部哈希链 + 外部 sink 桥接）
//! - 类型化服务注册表（共享服务注入）

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::audit::UnifiedAuditChain;
use crate::phase::{Phase, PhaseExecution, PhaseStatus};
use crate::result::PhaseResult;

// ================== 上下文输入类型 ==================

/// 管线输入：通用的请求模型
///
/// 支持多种输入类型，各管线实现从中提取自己需要的部分。
/// 使用 JSON Value 承载领域特定数据，避免直接依赖领域 crate。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PipelineInput {
    /// 结构化对象输入（如图、配置等）
    Structured {
        /// 对象 ID
        id: String,
        /// 对象名称
        name: String,
        /// 序列化的对象数据（JSON）
        data: serde_json::Value,
    },
    /// 自然语言/文本查询输入
    Query {
        query: String,
        session_id: Option<String>,
        context: HashMap<String, String>,
    },
    /// 混合输入（既有结构化数据又有查询描述）
    Mixed {
        id: String,
        query: String,
        session_id: Option<String>,
        context: HashMap<String, String>,
        data: serde_json::Value,
    },
}

impl PipelineInput {
    /// 获取输入的标识
    pub fn id(&self) -> &str {
        match self {
            Self::Structured { id, .. } => id,
            Self::Query { query, .. } => query, // query 作为临时 id
            Self::Mixed { id, .. } => id,
        }
    }

    /// 获取会话 ID（如果有）
    pub fn session_id(&self) -> Option<&str> {
        match self {
            Self::Structured { .. } => None,
            Self::Query { session_id, .. } => session_id.as_deref(),
            Self::Mixed { session_id, .. } => session_id.as_deref(),
        }
    }
}

/// 运行时选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOptions {
    /// 是否启用详细日志
    #[serde(default)]
    pub verbose: bool,
    /// 失败是否重试
    #[serde(default = "default_true")]
    pub retry_on_failure: bool,
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
            retry_on_failure: true,
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
/// 并行处理等并发场景应在阶段处理器内部管理自己的并发，
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

    // ---- 租户/主体/配额 ----
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
    /// 类型化服务注册表（用于共享服务注入）
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
            .field("input_kind", &match self.input {
                PipelineInput::Structured { .. } => "structured",
                PipelineInput::Query { .. } => "query",
                PipelineInput::Mixed { .. } => "mixed",
            })
            .field("tenant_id", &self.tenant_id)
            .field("principal", &self.principal)
            .field("phase_count", &self.phase_results.len())
            .field("audit_events", &self.audit.len())
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
        // 按阶段序号排序
        phases.sort_by_key(|p| p.order());
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

    /// 获取失败的阶段列表
    pub fn failed_phases(&self) -> Vec<Phase> {
        self.phase_executions
            .iter()
            .filter(|(_, e)| e.status == PhaseStatus::Failed)
            .map(|(p, _)| *p)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::GenericPhaseResult;

    fn make_ctx() -> PipelineContext {
        PipelineContext::new(
            PipelineInput::Query {
                query: "test query".into(),
                session_id: Some("sess-123".into()),
                context: HashMap::new(),
            },
            PipelineOptions::default(),
        )
    }

    #[test]
    fn context_creation() {
        let ctx = make_ctx();
        assert!(!ctx.trace_id.is_nil());
        assert_eq!(ctx.input.id(), "test query");
        assert!(ctx.phase_results.is_empty());
        assert!(ctx.audit.is_empty());
        assert!(ctx.all_succeeded()); // 没有阶段也算都成功
        assert!(!ctx.is_blocked());
    }

    #[test]
    fn context_with_identity() {
        let ctx = make_ctx().with_identity(
            "tenant-1",
            "user:alice",
            vec!["admin".into(), "editor".into()],
        );
        assert_eq!(ctx.tenant_id, "tenant-1");
        assert_eq!(ctx.principal, "user:alice");
        assert_eq!(ctx.roles.len(), 2);
    }

    #[test]
    fn set_and_get_result() {
        let mut ctx = make_ctx();
        let result = GenericPhaseResult::success(Phase::Analyze, serde_json::json!({"ok": true}), 100);
        ctx.set_result(Box::new(result));

        assert!(ctx.get_result(Phase::Analyze).is_some());
        assert_eq!(ctx.completed_phases(), vec![Phase::Analyze]);
        assert!(ctx.all_succeeded());
    }

    #[test]
    fn get_result_typed_works() {
        let mut ctx = make_ctx();
        let result = GenericPhaseResult::success(Phase::Analyze, serde_json::json!({"data": 42}), 100);
        ctx.set_result(Box::new(result));

        let typed = ctx.get_result_typed::<GenericPhaseResult>(Phase::Analyze);
        assert!(typed.is_some());
        assert_eq!(typed.unwrap().payload()["data"], 42);
    }

    #[test]
    fn mark_phase_start_and_end() {
        let mut ctx = make_ctx();

        ctx.mark_phase_start(Phase::Analyze);
        assert_eq!(ctx.current_phase, Some(Phase::Analyze));
        let exec = ctx.get_execution(Phase::Analyze).unwrap();
        assert_eq!(exec.status, PhaseStatus::Running);

        ctx.mark_phase_end(Phase::Analyze, PhaseStatus::Success, 100);
        assert_eq!(ctx.current_phase, None);
        let exec = ctx.get_execution(Phase::Analyze).unwrap();
        assert_eq!(exec.status, PhaseStatus::Success);
        assert_eq!(exec.latency_ms, 100);
    }

    #[test]
    fn service_registry() {
        let mut ctx = make_ctx();

        struct MyService {
            value: i32,
        }

        ctx.provide_service(MyService { value: 42 });

        let svc = ctx.get_service::<MyService>();
        assert!(svc.is_some());
        assert_eq!(svc.unwrap().value, 42);

        // 不存在的服务返回 None
        assert!(ctx.get_service::<String>().is_none());
    }

    #[test]
    fn bag_storage() {
        let mut ctx = make_ctx();

        ctx.set_bag("count", 42i32);
        ctx.set_bag("name", "hello".to_string());

        assert_eq!(ctx.get_bag::<i32>("count"), Some(&42));
        assert_eq!(ctx.get_bag::<String>("name"), Some(&"hello".to_string()));
        assert!(ctx.has_bag("count"));
        assert!(!ctx.has_bag("nonexistent"));

        // 类型不匹配返回 None
        assert!(ctx.get_bag::<String>("count").is_none());
    }

    #[test]
    fn is_blocked_detection() {
        let mut ctx = make_ctx();

        // 初始状态：未阻断
        assert!(!ctx.is_blocked());

        // 添加一个阻断阶段
        ctx.mark_phase_start(Phase::Gate);
        ctx.mark_phase_end(Phase::Gate, PhaseStatus::Blocked, 50);

        assert!(ctx.is_blocked());
        assert!(!ctx.all_succeeded());
    }

    #[test]
    fn failed_phases_list() {
        let mut ctx = make_ctx();

        ctx.mark_phase_start(Phase::Analyze);
        ctx.mark_phase_end(Phase::Analyze, PhaseStatus::Failed, 100);

        ctx.mark_phase_start(Phase::Normalize);
        ctx.mark_phase_end(Phase::Normalize, PhaseStatus::Success, 50);

        let failed = ctx.failed_phases();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0], Phase::Analyze);
    }

    #[test]
    fn pipeline_input_variants() {
        let structured = PipelineInput::Structured {
            id: "flow-1".into(),
            name: "My Flow".into(),
            data: serde_json::json!({"nodes": []}),
        };
        assert_eq!(structured.id(), "flow-1");
        assert!(structured.session_id().is_none());

        let query = PipelineInput::Query {
            query: "hello".into(),
            session_id: Some("sess-1".into()),
            context: HashMap::new(),
        };
        assert_eq!(query.id(), "hello");
        assert_eq!(query.session_id(), Some("sess-1"));

        let mixed = PipelineInput::Mixed {
            id: "flow-2".into(),
            query: "optimize this".into(),
            session_id: None,
            context: HashMap::new(),
            data: serde_json::json!({}),
        };
        assert_eq!(mixed.id(), "flow-2");
    }

    #[test]
    fn pipeline_options_defaults() {
        let opts = PipelineOptions::default();
        assert!(!opts.verbose);
        assert!(opts.retry_on_failure);
        assert_eq!(opts.max_retries, 1);
        assert_eq!(opts.phase_timeout_ms, 0);
        assert!(opts.audit_enabled);
    }

    #[test]
    fn context_debug_format() {
        let ctx = make_ctx();
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("PipelineContext"));
        assert!(debug.contains("trace_id"));
        assert!(debug.contains("phase_count"));
    }
}
