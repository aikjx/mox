//! RuledRouter：把 RULE_REGEXES + app_alias_fallback 串起来 → 路由候选数组

use serde_json::Value;
use std::sync::Arc;

use crate::rules::{apply_increments, app_alias_fallback, build_rule_set, Rule};
use xiaobai_core::engine::RoutedAction;

#[derive(Debug, Clone)]
pub struct RuledRouter {
    rules: Arc<Vec<Rule>>,
}

impl Default for RuledRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl RuledRouter {
    pub fn new() -> Self {
        Self { rules: Arc::new(build_rule_set()) }
    }

    /// 核心调度：纯函数 `&str -> Vec<RoutedAction>`，top1 在 [0]
    pub fn dispatch(&self, text: &str) -> Vec<RoutedAction> {
        let t = text.trim();
        if t.is_empty() {
            return Vec::new();
        }
        let mut scored: Vec<(f32, String, String, Value)> = Vec::new();

        // 1) 正则逐条，多条规则可同时命中（命中多次取 score 最高那一次）
        for rule in self.rules.iter() {
            if let Some(caps) = rule.regex.captures(t) {
                let (action, mut param) = (rule.extractor)(&caps);
                apply_increments(&mut param);
                scored.push((rule.score, action, rule.category.to_string(), param));
            }
        }
        // 2) 应用别名兜底（没命中任何正则时生效）
        if scored.is_empty() {
            let fb = app_alias_fallback(t);
            for (action, score, cat, param) in fb.into_iter() {
                scored.push((score, action, cat.to_string(), param));
            }
        }
        // 3) 排序：score DESC；相同 score 保持插入稳定排序
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        // 4) 构造 RoutedAction + confidence_delta
        let top_score = scored.first().map(|(s, _, _, _)| *s).unwrap_or(0.0);
        let second_score = scored.get(1).map(|(s, _, _, _)| *s).unwrap_or(0.0);
        let delta = (top_score - second_score).max(0.0);
        scored
            .into_iter()
            .enumerate()
            .map(|(i, (score, action, category, param))| RoutedAction {
                action,
                category,
                score,
                confidence_delta: if i == 0 { delta } else { 0.0 },
                param,
            })
            .collect()
    }
}

/// Engine 可直接用的 DefaultRouter 实现在 lib.rs 这里导出别名（名字对齐 Python IntentRouterImpl）
pub type IntentRouterImpl = crate::DefaultRouter;

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn dispatch_mute_returns_top1_mute() {
        let r = RuledRouter::default();
        let out = r.dispatch("帮我静音");
        assert!(!out.is_empty(), "静音句子应该命中 mute");
        assert_eq!(out[0].action, "mute");
        assert!(out[0].confidence_delta >= 0.0);
    }

    #[test]
    fn dispatch_open_wechat_app_open_app() {
        let r = RuledRouter::default();
        let out = r.dispatch("启动 飞书");
        assert_eq!(out[0].action, "open_app");
        assert_eq!(out[0].param.get("app_name").unwrap().as_str().unwrap(), "飞书");
    }

    #[test]
    fn dispatch_volume_percent_extract_param() {
        let r = RuledRouter::default();
        let out = r.dispatch("音量调到 70");
        assert_eq!(out[0].action, "set_volume");
        assert_eq!(out[0].param.get("percent").unwrap().as_i64().unwrap(), 70);
    }

    #[test]
    fn dispatch_screenshot_exact_action() {
        let r = RuledRouter::default();
        let out = r.dispatch("帮我截个屏");
        assert_eq!(out[0].action, "screenshot");
    }

    #[test]
    fn dispatch_fallback_bare_app_name() {
        let r = RuledRouter::default();
        let out = r.dispatch("哎呀突然想看 WPS");
        // 兜底：包含 WPS → open_app(app_name=WPS) 0.55
        assert!(!out.is_empty(), "WPS 兜底应命中");
        assert_eq!(out[0].action, "open_app");
    }

    #[test]
    fn dispatch_unknown_empty() {
        let r = RuledRouter::default();
        let out = r.dispatch("阿巴顿撒肯德基卢浮宫");
        assert!(out.is_empty(), "无意义句子不应该命中任何规则");
    }
}
