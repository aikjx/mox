// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! AI Providers 模块入口
//!
//! 新增Provider：在此文件添加 `pub mod xxx;` 和重导出，
//! 核心trait/registry/router零改动。

pub mod anthropic;
pub mod dto;
pub mod error;
pub mod openai;
pub mod qwen;
pub mod traits;

// 重导出核心类型
pub use dto::*;
pub use error::{AiError, AiResult};
pub use traits::AiProvider;

// 重导出内置Provider实现
pub use anthropic::AnthropicProvider;
pub use openai::OpenAiProvider;
pub use qwen::QwenProvider;
