// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! L3 领域层对外统一数据类型（SSOT）
//!
//! 三个对外抽象 trait（`ExpertRegistry` / `ExpertConsultant` / `AllianceOrchestrator`）
//! 统一使用本模块的类型，避免下游 crate 直接依赖 mox-expert 的内部 concrete struct
//! （如 `GovernanceReport` / `Dimension` 等实现细节），完成 DIP 反转：下游只依赖 trait 抽象，
//! 不依赖具体实现。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// ExpertRegistry 相关类型：专家元数据
// ---------------------------------------------------------------------------

/// 专家元数据：可注册 / 可查询 / 可路由的最小专家画像。
///
/// 对应内部 `crate::ir::Dimension` + `crate::expert::Expert` 的对外投影，
/// 但只暴露「id/名称/域/能力标签」等最小信息，隐藏引擎内部分析细节。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExpertMeta {
    /// 全局唯一专家 id，如 `security` / `algorithm` / `architecture-code`
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 所属领域（如 `gov` / `finance`；`*` 表示通用）
    pub domain: String,
    /// 能力标签：用于 find/list 的关键词匹配（如 `["security","pii","authz"]`）
    pub capabilities: Vec<String>,
    /// 可选：描述文本
    #[serde(default)]
    pub description: String,
    /// 可选：维度（对齐 crate::ir::Dimension；缺省为空）
    #[serde(default)]
    pub dimension: Option<String>,
}

impl ExpertMeta {
    pub fn new(id: impl Into<String>, name: impl Into<String>, domain: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            domain: domain.into(),
            capabilities: Vec::new(),
            description: String::new(),
            dimension: None,
        }
    }
    pub fn with_capabilities(mut self, caps: impl IntoIterator<Item = String>) -> Self {
        self.capabilities = caps.into_iter().collect();
        self
    }
}

// ---------------------------------------------------------------------------
// ExpertConsultant 相关类型：咨询输入 / 输出
// ---------------------------------------------------------------------------

/// 咨询查询：把外部咨询请求抽象成最小可计算请求。
///
/// 真实请求可能是 FlowGraph 或代码片段；trait 不直接依赖 mox_ai_flow_svc::FlowGraph，
/// 下游可按需构造。`ctx` 携带主体/租户/配额等治理上下文的序列化投影。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsultQuery {
    /// 请求唯一 id（调用方生成，ConsultReport 原样返回以匹配）
    pub id: String,
    /// 自然语言 / DSL 查询字符串
    pub query: String,
    /// 可选：附加上下文键值对（租户、主体、配额等治理参数的通用载体）
    #[serde(default)]
    pub ctx: HashMap<String, String>,
}

/// 咨询报告：专家咨询后的对外归一化输出。
///
/// 隐藏内部 `GovernanceReport` 的复杂字段（expert_scores / optimization / algo / gate / audit），
/// 只暴露：报告 id、执行步骤（可读摘要）、综合分 0..1、以及是否否决。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsultReport {
    /// 报告 id（通常等于 ConsultQuery.id，便于关联）
    pub report_id: String,
    /// 执行步骤摘要（可读文本，例如"14 位专家并行诊断 → 权限/安全裁决 → 算法验证通过"）
    pub steps: Vec<String>,
    /// 综合健康分 0..1（1 = 完全健康；0 = 完全否决）
    pub score: f64,
    /// 是否被算法验证或治理闸门否决（veto=true 时下游应强制拦截）
    pub vetoed: bool,
    /// 可选：否决/警告的原因
    #[serde(default)]
    pub reason: Option<String>,
}

impl Default for ConsultReport {
    fn default() -> Self {
        Self {
            report_id: String::new(),
            steps: Vec::new(),
            score: 1.0,
            vetoed: false,
            reason: None,
        }
    }
}

// ---------------------------------------------------------------------------
// AllianceOrchestrator 相关类型：任务路由输入 / 输出
// ---------------------------------------------------------------------------

/// 任务规格：联盟编排器（AllianceOrchestrator）的路由输入。
///
/// 把"要做什么"（scenario / constraints）最小化表达为可路由请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    /// 任务唯一 id
    pub task_id: String,
    /// 业务场景：如 `gov-pii` / `etl` / `mcp-orchestration`
    pub scenario: String,
    /// 约束键值对：如 `{"regulated":"true","sla_ms":"30000"}`
    #[serde(default)]
    pub constraints: HashMap<String, String>,
}

/// 路由决策：联盟编排器选择最合适的专家后的返回。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// 选中的专家 id（对应 ExpertMeta.id）
    pub expert_id: String,
    /// 路由置信度 0..1（1 = 确信匹配）
    pub confidence: f64,
    /// 路由理由（可读文本，便于审计与调试）
    pub reason: String,
}

// ---------------------------------------------------------------------------
// 通用 Result（trait 签名统一使用）
// ---------------------------------------------------------------------------

/// 对外 trait 的统一 Result：任何错误退化为 anyhow::Error，保持 trait 签名简洁。
pub type Result<T> = anyhow::Result<T>;
