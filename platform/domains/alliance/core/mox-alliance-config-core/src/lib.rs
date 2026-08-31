// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox Alliance Config Core — 专家联盟模块化配置引擎
//!
//! 为每个 AI 处理模块提供独立的 LLM 配置和 Graph 引擎配置管理，
//! 支持热更新、版本管理、配置回滚和变更事件通知。
//!
//! ## 核心特性
//! - **每模块独立配置**：每个专家模块拥有独立的 LLM 和 Graph 配置
//! - **独立 API Key**：每个模块可独立配置 LLM API Key（环境变量/明文/密钥引用）
//! - **全局默认回退**：未配置的模块自动使用全局默认大模型
//! - **配置合并**：模块级配置覆盖全局配置，自动合并生效
//! - **热更新**：配置变更实时生效，无需重启
//! - **版本管理**：配置变更自动版本化，支持历史追溯
//! - **配置回滚**：一键回滚到任意历史版本
//! - **变更事件**：通过 broadcast 通道发布配置变更事件
//! - **验证机制**：配置写入前自动验证合法性
//! - **存储抽象**：`ConfigStore` trait 支持多种后端实现
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use mox_alliance_config_core::*;
//! use mox_alliance_common_proto::*;
//! use std::sync::Arc;
//! use chrono::Utc;
//!
//! # async fn example() -> ConfigResult<()> {
//! // 1. 创建内存存储
//! let store = MemoryConfigStore::arc();
//!
//! // 2. 创建配置引擎
//! let engine = ConfigEngine::new(store.clone());
//!
//! // 3. 注册模块配置
//! let config = ExpertModuleConfig {
//!     module_id: "expert-arch".to_string(),
//!     expert_id: "arch-expert".to_string(),
//!     name: "架构专家".to_string(),
//!     version: "1.0.0".to_string(),
//!     llm_config: ModuleLlmConfig {
//!         module_id: "expert-arch".to_string(),
//!         primary_provider: "openai".to_string(),
//!         primary_model: "gpt-4o".to_string(),
//!         fallback_chain: vec!["anthropic".to_string()],
//!         routing_strategy: LlmRoutingStrategy::Priority,
//!         model_config: ModelConfig::default(),
//!         provider_options: vec![],
//!         system_prompt_template: None,
//!         version: 1,
//!         updated_at: Utc::now(),
//!     },
//!     graph_config: ModuleGraphConfig {
//!         module_id: "expert-arch".to_string(),
//!         engine_type: GraphEngineType::RelGraph,
//!         connection: GraphConnectionConfig {
//!             uri_env: "RELGRAPH_URI".to_string(),
//!             user_env: None,
//!             password_env: None,
//!             database: None,
//!         },
//!         query_config: GraphQueryConfig::default(),
//!         schema: GraphSchemaConfig::default(),
//!         custom_endpoint: None,
//!         version: 1,
//!         updated_at: Utc::now(),
//!     },
//!     capability_weights: Default::default(),
//!     matching_weights: MatchingWeights::default(),
//!     enabled: true,
//!     tags: vec!["architecture".to_string()],
//!     created_at: Utc::now(),
//!     updated_at: Utc::now(),
//! };
//!
//! engine.register_module(config, "admin", "Initial setup").await?;
//!
//! // 4. 获取 LLM 配置
//! let llm_config = engine.get_llm_config("expert-arch").await?;
//!
//! // 5. 订阅配置变更
//! let mut rx = engine.subscribe();
//!
//! # Ok(())
//! # }
//! ```

pub mod engine;
pub mod error;
pub mod events;
pub mod store;
pub mod validator;
pub mod examples;

// ─── 重导出 ──────────────────────────────────────────────────────────────────

pub use engine::ConfigEngine;
pub use error::{ConfigError, ConfigResult};
pub use events::{ConfigChangeEvent, ConfigChangeType};
pub use store::{ConfigStore, MemoryConfigStore};
pub use validator::ConfigValidator;

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
