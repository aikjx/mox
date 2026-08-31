// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家联盟通用领域类型
//!
//! 所有联盟子服务共享的核心数据结构定义于此。
//! 遵循 SSOT 原则：每个领域概念只有一个权威定义。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Task（任务） ───────────────────────────────────────────────────────────

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// 待处理
    Pending,
    /// 规划中（生成协作计划）
    Planning,
    /// 执行中
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

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Planning | Self::Running | Self::Paused)
    }
}

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum TaskPriority {
    Low = 1,
    Normal = 5,
    High = 8,
    Critical = 10,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// 协作模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllianceMode {
    /// 串行协作（专家按顺序执行）
    Sequential,
    /// 并行协作（多专家同时执行）
    Parallel,
    /// 辩论模式（专家相互辩论，最终裁决）
    Debate,
    /// 分层协作（分层递进，每层多位专家）
    Hierarchical,
    /// 迭代精炼（多轮迭代，逐步优化结果）
    Iterative,
    /// 投票裁决（多专家投票，多数决）
    Voting,
}

impl Default for AllianceMode {
    fn default() -> Self {
        Self::Parallel
    }
}

/// 融合策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FusionStrategy {
    /// 加权投票融合（按权重投票，适用于分类/决策任务）
    Voting,
    /// 加权融合（按专家权重的数值融合）
    Weighted,
    /// 置信度加权融合（基于动态置信度的加权平均）
    ConfidenceWeighted,
    /// 拼接融合（结果拼接）
    Concatenation,
    /// 择优融合（选最优结果）
    BestOf,
    /// 堆叠融合（元学习器组合多模型输出）
    Stacking,
    /// 辩论融合（多智能体辩论裁决）
    Debate,
    /// Map-Reduce 融合（分治式融合，适用于大规模数据）
    MapReduce,
    /// 迭代精炼融合（多轮迭代优化）
    Iterative,
}

impl Default for FusionStrategy {
    fn default() -> Self {
        Self::Weighted
    }
}

impl FusionStrategy {
    /// 获取策略的中文描述名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Voting => "加权投票融合",
            Self::Weighted => "加权融合",
            Self::ConfidenceWeighted => "置信度加权融合",
            Self::Concatenation => "拼接融合",
            Self::BestOf => "择优融合",
            Self::Stacking => "堆叠融合（元学习器）",
            Self::Debate => "辩论融合",
            Self::MapReduce => "Map-Reduce 融合",
            Self::Iterative => "迭代精炼融合",
        }
    }

    /// 判断该策略是否适用于标量（数值）融合
    pub fn supports_scalar(&self) -> bool {
        matches!(
            self,
            Self::Weighted
                | Self::ConfidenceWeighted
                | Self::Stacking
                | Self::MapReduce
                | Self::Iterative
                | Self::BestOf
        )
    }

    /// 判断该策略是否适用于分类（投票）融合
    pub fn supports_classification(&self) -> bool {
        matches!(
            self,
            Self::Voting
                | Self::Weighted
                | Self::ConfidenceWeighted
                | Self::Debate
                | Self::BestOf
                | Self::Iterative
        )
    }
}

/// 任务 — 用户提交的一次协作请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    // === 标识 ===
    pub task_id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,

    // === 内容 ===
    pub title: String,
    pub description: String,
    pub task_type: String,

    // === 状态 ===
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub progress: f32,
    pub current_node_id: Option<String>,

    // === 协作配置 ===
    pub mode: AllianceMode,
    pub fusion_strategy: FusionStrategy,

    // === 时间 ===
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
}

impl Task {
    pub fn new(tenant_id: Uuid, user_id: Uuid, title: String, description: String) -> Self {
        Self {
            task_id: Uuid::new_v4(),
            tenant_id,
            user_id,
            title,
            description,
            task_type: "custom".to_string(),
            status: TaskStatus::Pending,
            priority: TaskPriority::default(),
            progress: 0.0,
            current_node_id: None,
            mode: AllianceMode::default(),
            fusion_strategy: FusionStrategy::default(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            duration_ms: None,
        }
    }
}

// ─── Node（DAG 节点） ──────────────────────────────────────────────────────

/// 节点状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

impl NodeStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Skipped | Self::Cancelled
        )
    }
}

/// DAG 节点 — 协作计划中的一个执行单元
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub node_id: String,
    pub task_id: Uuid,
    pub expert_id: String,

    pub name: String,
    pub description: Option<String>,

    pub status: NodeStatus,
    pub retry_count: u32,

    /// 依赖的上游节点 ID 列表
    pub dependencies: Vec<String>,

    /// 节点输入数据引用
    pub input_refs: Vec<String>,

    /// 节点输出数据引用
    pub output_ref: Option<String>,

    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,

    pub error_message: Option<String>,
}

// ─── CollaborationPlan（协作计划） ────────────────────────────────────────

/// 协作计划 — 任务的 DAG 执行图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationPlan {
    pub task_id: Uuid,
    pub mode: AllianceMode,
    pub fusion_strategy: FusionStrategy,
    pub nodes: Vec<Node>,
    pub version: u32,
    pub created_at: DateTime<Utc>,
}

impl CollaborationPlan {
    /// 检查计划是否有效（无环 + 所有依赖存在）
    pub fn validate(&self) -> Result<(), String> {
        use std::collections::{HashMap, HashSet};

        let node_ids: HashSet<&str> = self.nodes.iter().map(|n| n.node_id.as_str()).collect();

        // 检查所有依赖都存在
        for node in &self.nodes {
            for dep in &node.dependencies {
                if !node_ids.contains(dep.as_str()) {
                    return Err(format!("Node {} depends on non-existent node {}", node.node_id, dep));
                }
            }
        }

        // 简单环检测（拓扑排序）
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for node in &self.nodes {
            in_degree.entry(&node.node_id).or_insert(0);
            for dep in &node.dependencies {
                *in_degree.entry(dep).or_insert(0) += 1;
            }
        }

        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut count = 0;
        while let Some(node_id) = queue.pop() {
            count += 1;
            if let Some(node) = self.nodes.iter().find(|n| n.node_id == node_id) {
                for dep in &node.dependencies {
                    if let Some(deg) = in_degree.get_mut(dep.as_str()) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(dep);
                        }
                    }
                }
            }
        }

        if count != self.nodes.len() {
            return Err("Cycle detected in collaboration plan".to_string());
        }

        Ok(())
    }
}

// ─── Expert（专家） ────────────────────────────────────────────────────────

/// 专家状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpertStatus {
    /// 未激活
    Inactive,
    /// 激活可用
    Active,
    /// 维护中
    Maintenance,
    /// 已下线
    Deprecated,
}

/// 专家健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertHealth {
    pub is_healthy: bool,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub error_count: u64,
}

impl Default for ExpertHealth {
    fn default() -> Self {
        Self {
            is_healthy: true,
            last_heartbeat: None,
            success_rate: 1.0,
            avg_latency_ms: 0.0,
            error_count: 0,
        }
    }
}

/// 能力声明 — 专家可执行的一类操作的抽象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub capability_id: String,
    pub name: String,
    pub description: String,
    pub domain: String,
    pub version: String,
}

/// 工具绑定 — 能力对应的具体可调用方法
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolBinding {
    pub tool_id: String,
    pub name: String,
    pub description: String,
    pub protocol: String, // gRPC / HTTP / MCP
    pub endpoint: String,
    pub input_schema: Option<String>,
    pub output_schema: Option<String>,
}

/// 领域 — 知识/业务的分类范畴
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub domain_id: String,
    pub name: String,
    pub description: String,
    pub parent_id: Option<String>,
}

/// 专家 — 具备领域知识和工具能力的自治 Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expert {
    // === 标识 ===
    pub expert_id: String,
    pub tenant_id: String, // "system" = 系统内置
    pub name: String,
    pub version: String,

    // === 描述 ===
    pub description: String,
    pub domains: Vec<String>,
    pub capabilities: Vec<Capability>,

    // === 工具绑定 ===
    pub tools: Vec<ToolBinding>,

    // === 状态 ===
    pub status: ExpertStatus,
    pub health: ExpertHealth,
    pub priority: u8,

    // === 时间 ===
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Expert {
    pub fn new_system(name: String, description: String) -> Self {
        let now = Utc::now();
        Self {
            expert_id: Uuid::new_v4().to_string(),
            tenant_id: "system".to_string(),
            name,
            version: "0.1.0".to_string(),
            description,
            domains: vec![],
            capabilities: vec![],
            tools: vec![],
            status: ExpertStatus::Active,
            health: ExpertHealth::default(),
            priority: 5,
            created_at: now,
            updated_at: now,
        }
    }
}

// ─── ModuleLlmConfig（模块LLM配置） ────────────────────────────────────────

/// LLM 路由策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmRoutingStrategy {
    /// 优先级路由（按优先级顺序选择）
    Priority,
    /// 轮询路由
    RoundRobin,
    /// 延迟优先路由
    LatencyPriority,
    /// 成本优先路由
    CostPriority,
}

impl Default for LlmRoutingStrategy {
    fn default() -> Self {
        Self::Priority
    }
}

/// LLM 模型推理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// 温度 (0.0 - 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Top-p 采样 (0.0 - 1.0)
    #[serde(default = "default_top_p")]
    pub top_p: f32,
    /// 最大生成 token 数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// 频率惩罚 (-2.0 - 2.0)
    #[serde(default)]
    pub frequency_penalty: f32,
    /// 存在惩罚 (-2.0 - 2.0)
    #[serde(default)]
    pub presence_penalty: f32,
    /// 停止序列
    #[serde(default)]
    pub stop_sequences: Vec<String>,
}

fn default_temperature() -> f32 { 0.7 }
fn default_top_p() -> f32 { 0.9 }
fn default_max_tokens() -> u32 { 2048 }

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            temperature: default_temperature(),
            top_p: default_top_p(),
            max_tokens: default_max_tokens(),
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            stop_sequences: vec![],
        }
    }
}

/// API Key 来源类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeySource {
    /// 从环境变量读取
    EnvVar {
        /// 环境变量名
        env_name: String,
    },
    /// 直接配置（明文，仅用于开发测试）
    #[serde(rename = "plain_text")]
    PlainText {
        /// API Key 明文
        api_key: String,
    },
    /// 从密钥管理服务读取
    #[serde(rename = "secret_ref")]
    SecretRef {
        /// 密钥引用路径/ID
        secret_id: String,
        /// 密钥版本（可选）
        version: Option<String>,
    },
    /// 从全局默认配置继承
    Inherit,
}

impl Default for ApiKeySource {
    fn default() -> Self {
        Self::Inherit
    }
}

impl ApiKeySource {
    /// 从环境变量名创建
    pub fn from_env(env_name: impl Into<String>) -> Self {
        Self::EnvVar { env_name: env_name.into() }
    }

    /// 从明文创建（仅开发用）
    pub fn from_plain(api_key: impl Into<String>) -> Self {
        Self::PlainText { api_key: api_key.into() }
    }

    /// 从密钥引用创建
    pub fn from_secret(secret_id: impl Into<String>) -> Self {
        Self::SecretRef {
            secret_id: secret_id.into(),
            version: None,
        }
    }

    /// 是否为继承模式
    pub fn is_inherit(&self) -> bool {
        matches!(self, Self::Inherit)
    }

    /// 尝试解析 API Key（仅支持环境变量和明文模式）
    /// 密钥引用模式需要外部密钥管理器配合
    pub fn resolve_api_key(&self) -> Option<String> {
        match self {
            Self::EnvVar { env_name } => std::env::var(env_name).ok(),
            Self::PlainText { api_key } => Some(api_key.clone()),
            Self::SecretRef { .. } => None, // 需要外部密钥管理器
            Self::Inherit => None,
        }
    }
}

/// LLM Provider 配置选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderOption {
    /// Provider ID
    pub provider_id: String,
    /// Provider 显示名称
    #[serde(default)]
    pub display_name: Option<String>,
    /// API Key 来源配置
    #[serde(default)]
    pub api_key_source: ApiKeySource,
    /// Base URL (可选，用于自定义端点)
    #[serde(default)]
    pub base_url: Option<String>,
    /// 默认模型名
    #[serde(default)]
    pub default_model: Option<String>,
    /// 支持的模型列表
    #[serde(default)]
    pub supported_models: Vec<String>,
    /// 单价（每 1K token，美元）
    #[serde(default)]
    pub price_per_1k_tokens: Option<f64>,
    /// 速率限制（每分钟请求数）
    #[serde(default)]
    pub rpm_limit: Option<u32>,
    /// 速率限制（每分钟 token 数）
    #[serde(default)]
    pub tpm_limit: Option<u64>,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool { true }

impl LlmProviderOption {
    /// 获取显示名称（如果没有则返回 provider_id）
    pub fn name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.provider_id)
    }

    /// 是否有有效的 API Key 配置（非 Inherit）
    pub fn has_api_key_configured(&self) -> bool {
        !self.api_key_source.is_inherit()
    }
}

/// 全局默认 LLM 配置
///
/// 作为所有模块的默认回退配置。
/// 模块配置中未指定的部分会从这里继承。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalLlmConfig {
    /// 全局主 Provider ID
    pub primary_provider: String,
    /// 全局主模型名
    pub primary_model: String,
    /// 全局降级链
    #[serde(default)]
    pub fallback_chain: Vec<String>,
    /// 全局路由策略
    #[serde(default)]
    pub routing_strategy: LlmRoutingStrategy,
    /// 全局默认模型推理参数
    #[serde(default)]
    pub model_config: ModelConfig,
    /// 全局 Provider 配置列表（提供默认 API Key 等）
    #[serde(default)]
    pub provider_options: Vec<LlmProviderOption>,
    /// 全局系统提示词前缀（会加在专家级提示词前面）
    #[serde(default)]
    pub global_system_prompt_prefix: Option<String>,
    /// 配置版本号
    #[serde(default = "default_version")]
    pub version: u32,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
}

impl Default for GlobalLlmConfig {
    fn default() -> Self {
        Self {
            primary_provider: "default".to_string(),
            primary_model: "default-model".to_string(),
            fallback_chain: vec![],
            routing_strategy: LlmRoutingStrategy::default(),
            model_config: ModelConfig::default(),
            provider_options: vec![],
            global_system_prompt_prefix: None,
            version: 1,
            updated_at: Utc::now(),
        }
    }
}

/// 模块 LLM 配置 — 每个专家模块独立的LLM设置
///
/// 每个模块可以独立配置：
/// - 使用哪个 Provider 和模型
/// - 独立的 API Key（从环境变量、密钥管理或直接配置）
/// - 独立的推理参数
/// - 独立的系统提示词
///
/// 模块未配置的部分会从全局默认配置继承。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleLlmConfig {
    /// 所属模块 ID
    pub module_id: String,
    /// 主 Provider ID
    pub primary_provider: String,
    /// 主模型名
    pub primary_model: String,
    /// 降级链 — 主模型不可用时按顺序切换
    #[serde(default)]
    pub fallback_chain: Vec<String>,
    /// 路由策略
    #[serde(default)]
    pub routing_strategy: LlmRoutingStrategy,
    /// 模型推理参数
    #[serde(default)]
    pub model_config: ModelConfig,
    /// Provider 选项（模块级覆盖，未设置的 Provider 从全局继承）
    #[serde(default)]
    pub provider_options: Vec<LlmProviderOption>,
    /// 系统提示词模板（专家级）
    #[serde(default)]
    pub system_prompt_template: Option<String>,
    /// 是否使用全局系统提示词前缀
    #[serde(default = "default_true")]
    pub use_global_prompt_prefix: bool,
    /// 配置版本号
    #[serde(default = "default_version")]
    pub version: u32,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
}

fn default_version() -> u32 { 1 }

impl ModuleLlmConfig {
    /// 获取指定 Provider 的配置选项
    pub fn get_provider_option(&self, provider_id: &str) -> Option<&LlmProviderOption> {
        self.provider_options.iter().find(|p| p.provider_id == provider_id)
    }

    /// 获取启用的 Provider 列表
    pub fn enabled_providers(&self) -> Vec<&LlmProviderOption> {
        self.provider_options.iter().filter(|p| p.enabled).collect()
    }

    /// 与全局默认配置合并，生成最终生效的配置
    ///
    /// 合并规则：
    /// - 模块配置优先，全局配置作为回退
    /// - Provider 配置：
    ///   - 模块级有配置且非 Inherit → 使用模块级
    ///   - 模块级配置为 Inherit → API Key 从全局继承，其他字段模块级优先
    ///   - 模块级没配置 → 完全使用全局级
    /// - 系统提示词：全局前缀 + 专家级提示词（如果启用）
    pub fn merge_with_global(&self, global: &GlobalLlmConfig) -> MergedLlmConfig {
        // 合并 Provider 配置
        let mut merged_providers: std::collections::HashMap<String, LlmProviderOption> =
            std::collections::HashMap::new();

        // 先加全局的
        for provider in &global.provider_options {
            merged_providers.insert(provider.provider_id.clone(), provider.clone());
        }

        // 再处理模块级的
        for module_provider in &self.provider_options {
            let pid = &module_provider.provider_id;

            if module_provider.api_key_source.is_inherit() {
                // Inherit 模式：API Key 从全局继承，其他字段模块级覆盖
                if let Some(global_provider) = merged_providers.get(pid).cloned() {
                    let merged = LlmProviderOption {
                        provider_id: global_provider.provider_id.clone(),
                        display_name: module_provider
                            .display_name
                            .clone()
                            .or(global_provider.display_name.clone()),
                        api_key_source: global_provider.api_key_source.clone(), // 继承全局的
                        base_url: module_provider
                            .base_url
                            .clone()
                            .or(global_provider.base_url.clone()),
                        default_model: module_provider
                            .default_model
                            .clone()
                            .or(global_provider.default_model.clone()),
                        supported_models: if !module_provider.supported_models.is_empty() {
                            module_provider.supported_models.clone()
                        } else {
                            global_provider.supported_models.clone()
                        },
                        price_per_1k_tokens: module_provider
                            .price_per_1k_tokens
                            .or(global_provider.price_per_1k_tokens),
                        rpm_limit: module_provider.rpm_limit.or(global_provider.rpm_limit),
                        tpm_limit: module_provider.tpm_limit.or(global_provider.tpm_limit),
                        enabled: module_provider.enabled,
                    };
                    merged_providers.insert(pid.clone(), merged);
                } else {
                    // 全局没有这个 Provider，Inherit 模式无效，保留模块级配置（但 API Key 不可用）
                    merged_providers.insert(pid.clone(), module_provider.clone());
                }
            } else {
                // 非 Inherit 模式：完全覆盖
                merged_providers.insert(pid.clone(), module_provider.clone());
            }
        }

        let provider_options: Vec<LlmProviderOption> = merged_providers.into_values().collect();

        // 合并系统提示词
        let system_prompt = match (
            &global.global_system_prompt_prefix,
            &self.system_prompt_template,
            self.use_global_prompt_prefix,
        ) {
            // 有全局前缀 + 有专家提示词 + 启用前缀 → 合并
            (Some(prefix), Some(specific), true) => {
                Some(format!("{}\n\n{}", prefix, specific))
            }
            // 有全局前缀 + 无专家提示词 + 启用前缀 → 只用全局
            (Some(prefix), None, true) => Some(prefix.clone()),
            // 有专家提示词（无论是否启用前缀，都用专家的）→ 只用专家
            (_, Some(specific), _) => Some(specific.clone()),
            // 其他情况 → 无提示词
            _ => None,
        };

        MergedLlmConfig {
            module_id: self.module_id.clone(),
            primary_provider: self.primary_provider.clone(),
            primary_model: self.primary_model.clone(),
            fallback_chain: if self.fallback_chain.is_empty() {
                global.fallback_chain.clone()
            } else {
                self.fallback_chain.clone()
            },
            routing_strategy: self.routing_strategy,
            model_config: self.model_config.clone(),
            provider_options,
            system_prompt,
        }
    }
}

/// 合并后的 LLM 配置（模块配置 + 全局默认）
///
/// 这是实际执行时使用的配置，已经完成了所有继承和覆盖的计算。
#[derive(Debug, Clone)]
pub struct MergedLlmConfig {
    /// 所属模块 ID
    pub module_id: String,
    /// 主 Provider ID
    pub primary_provider: String,
    /// 主模型名
    pub primary_model: String,
    /// 降级链
    pub fallback_chain: Vec<String>,
    /// 路由策略
    pub routing_strategy: LlmRoutingStrategy,
    /// 模型推理参数
    pub model_config: ModelConfig,
    /// 合并后的 Provider 配置
    pub provider_options: Vec<LlmProviderOption>,
    /// 合并后的系统提示词
    pub system_prompt: Option<String>,
}

impl MergedLlmConfig {
    /// 获取指定 Provider 的配置
    pub fn get_provider(&self, provider_id: &str) -> Option<&LlmProviderOption> {
        self.provider_options.iter().find(|p| p.provider_id == provider_id)
    }

    /// 获取主 Provider 配置
    pub fn primary_provider_config(&self) -> Option<&LlmProviderOption> {
        self.get_provider(&self.primary_provider)
    }

    /// 获取启用的 Provider 列表
    pub fn enabled_providers(&self) -> Vec<&LlmProviderOption> {
        self.provider_options.iter().filter(|p| p.enabled).collect()
    }

    /// 按路由策略获取 Provider 选择顺序
    pub fn provider_route_order(&self) -> Vec<String> {
        let mut order = vec![self.primary_provider.clone()];
        order.extend(self.fallback_chain.clone());
        order
    }

    /// 获取第一个有可用 API Key 的 Provider
    pub fn first_available_provider(&self) -> Option<&LlmProviderOption> {
        for provider_id in self.provider_route_order() {
            if let Some(provider) = self.get_provider(&provider_id) {
                if provider.enabled && provider.has_api_key_configured() {
                    // 检查是否能解析到 API Key
                    if provider.api_key_source.resolve_api_key().is_some() {
                        return Some(provider);
                    }
                }
            }
        }
        None
    }
}

// ─── ModuleGraphConfig（模块Graph配置） ─────────────────────────────────────

/// Graph 引擎类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphEngineType {
    /// Neo4j 图数据库
    Neo4j,
    /// 关系图谱（RelGraph 自研）
    RelGraph,
    /// 内存图（轻量场景）
    InMemory,
    /// 自定义图引擎
    Custom,
}

impl Default for GraphEngineType {
    fn default() -> Self {
        Self::RelGraph
    }
}

/// Graph 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConnectionConfig {
    /// URI 环境变量名
    pub uri_env: String,
    /// 用户名环境变量名
    #[serde(default)]
    pub user_env: Option<String>,
    /// 密码环境变量名
    #[serde(default)]
    pub password_env: Option<String>,
    /// 默认数据库/图空间
    #[serde(default)]
    pub database: Option<String>,
}

/// Graph 查询配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQueryConfig {
    /// 查询超时时间（毫秒）
    #[serde(default = "default_query_timeout")]
    pub timeout_ms: u32,
    /// 最大返回结果数
    #[serde(default = "default_max_results")]
    pub max_results: u32,
    /// 缓存 TTL（秒）
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_s: u32,
    /// 是否启用事务
    #[serde(default = "default_true")]
    pub enable_transaction: bool,
}

fn default_query_timeout() -> u32 { 30000 }
fn default_max_results() -> u32 { 1000 }
fn default_cache_ttl() -> u32 { 300 }

impl Default for GraphQueryConfig {
    fn default() -> Self {
        Self {
            timeout_ms: default_query_timeout(),
            max_results: default_max_results(),
            cache_ttl_s: default_cache_ttl(),
            enable_transaction: true,
        }
    }
}

/// Graph Schema 信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphSchemaConfig {
    /// 节点标签
    #[serde(default)]
    pub node_labels: Vec<String>,
    /// 关系类型
    #[serde(default)]
    pub relationship_types: Vec<String>,
    /// 索引名称
    #[serde(default)]
    pub indexes: Vec<String>,
}

/// 模块 Graph 配置 — 每个专家模块独立的图引擎设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleGraphConfig {
    /// 所属模块 ID
    pub module_id: String,
    /// 图引擎类型
    pub engine_type: GraphEngineType,
    /// 连接配置
    pub connection: GraphConnectionConfig,
    /// 查询配置
    #[serde(default)]
    pub query_config: GraphQueryConfig,
    /// Schema 信息
    #[serde(default)]
    pub schema: GraphSchemaConfig,
    /// 自定义引擎端点（engine_type=custom 时使用）
    #[serde(default)]
    pub custom_endpoint: Option<String>,
    /// 配置版本号
    #[serde(default = "default_version")]
    pub version: u32,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
}

// ─── ExpertModuleConfig（专家模块完整配置） ─────────────────────────────────

/// 专家模块完整配置 — 整合 LLM + Graph + 专家属性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertModuleConfig {
    /// 模块 ID
    pub module_id: String,
    /// 专家 ID（关联到 Expert）
    pub expert_id: String,
    /// 模块名称
    pub name: String,
    /// 模块版本
    pub version: String,
    /// LLM 配置
    pub llm_config: ModuleLlmConfig,
    /// Graph 配置
    pub graph_config: ModuleGraphConfig,
    /// 能力权重配置 (capability_id -> weight)
    #[serde(default)]
    pub capability_weights: std::collections::HashMap<String, f32>,
    /// 匹配权重配置（影响专家匹配算法的权重）
    #[serde(default)]
    pub matching_weights: MatchingWeights,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 配置标签（用于筛选）
    #[serde(default)]
    pub tags: Vec<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
}

/// 匹配权重配置 — 控制专家匹配算法的各维度权重
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingWeights {
    /// 领域匹配权重
    #[serde(default = "w_domain")]
    pub domain: f32,
    /// 能力匹配权重
    #[serde(default = "w_capability")]
    pub capability: f32,
    /// 专家评分权重
    #[serde(default = "w_rating")]
    pub rating: f32,
    /// 历史性能权重
    #[serde(default = "w_performance")]
    pub performance: f32,
    /// 健康状态权重
    #[serde(default = "w_health")]
    pub health: f32,
}

fn w_domain() -> f32 { 0.35 }
fn w_capability() -> f32 { 0.30 }
fn w_rating() -> f32 { 0.20 }
fn w_performance() -> f32 { 0.10 }
fn w_health() -> f32 { 0.05 }

impl Default for MatchingWeights {
    fn default() -> Self {
        Self {
            domain: w_domain(),
            capability: w_capability(),
            rating: w_rating(),
            performance: w_performance(),
            health: w_health(),
        }
    }
}

// ─── ConfigVersion（配置版本记录） ──────────────────────────────────────────

/// 配置版本记录 — 用于配置历史追溯与回滚
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigVersion {
    pub version: u32,
    pub module_id: String,
    pub config_type: ConfigType,
    pub config_snapshot: serde_json::Value,
    pub changed_by: String,
    pub change_reason: String,
    pub created_at: DateTime<Utc>,
}

/// 配置类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigType {
    Llm,
    Graph,
    Expert,
    Full,
}
