// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # xiaobai-intent · 小白语音 PPR 意图路由 Rust 实现
//!
//! 与 Python `intent/router.py` 功能 1:1 对齐：
//! - 40+ 中文/拼音正则规则，命中即 (action, category, score, param) 四元组
//! - 40 条应用别名映射（"打开微信" → `open_app(app_name="wechat.exe")`）
//! - 多个候选都命中时，按 `score` DESC 排序，top1-top2 ≤ AMBIGUITY_THRESHOLD → Engine 触发联盟裁决分支
//! - 规则抽取参数（如数字音量、中文路径百分比、按键名、文件名）
//!
//! 实现方式：`RuledRouter` 纯函数式 + `IntentRouter` async trait（Engine 可调用）

pub mod rules;
pub mod router;

pub use router::{RuledRouter, IntentRouterImpl};
pub use rules::{build_rule_set, APP_ALIAS_EXACT_LIST, COMMON_KEY_NAMES};

use async_trait::async_trait;
use mox_voice_core_svc::engine::{IntentRouter as EngineRouterTrait, RoutedAction, XiaobaiResult};

/// 默认对外意图路由实现（内置 RULE_REGEXES + APP_ALIAS）
#[derive(Debug, Default, Clone)]
pub struct DefaultRouter {
    inner: RuledRouter,
}

impl DefaultRouter {
    pub fn new() -> Self {
        Self { inner: RuledRouter::default() }
    }
}

#[async_trait]
impl EngineRouterTrait for DefaultRouter {
    async fn dispatch(&self, text: &str) -> XiaobaiResult<Vec<RoutedAction>> {
        Ok(self.inner.dispatch(text))
    }
}
