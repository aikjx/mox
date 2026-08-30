// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

use mox_voice_intent_svc::*;

// ─── RuledRouter 构造 ───

#[test]
fn ruled_router_default_construction() {
    let router = RuledRouter::default();
    let result = router.dispatch("静音");
    assert!(!result.is_empty(), "默认路由器应能命中'静音'指令");
}

#[test]
fn ruled_router_new_construction() {
    let router = RuledRouter::new();
    let result = router.dispatch("打开微信");
    assert!(!result.is_empty());
}

#[test]
fn ruled_router_clone() {
    let router = RuledRouter::default();
    let cloned = router.clone();
    let r1 = router.dispatch("音量调到 50");
    let r2 = cloned.dispatch("音量调到 50");
    assert_eq!(r1.len(), r2.len());
    if !r1.is_empty() && !r2.is_empty() {
        assert_eq!(r1[0].action, r2[0].action);
    }
}

// ─── 空输入 ───

#[test]
fn dispatch_empty_text_returns_empty() {
    let router = RuledRouter::default();
    assert!(router.dispatch("").is_empty());
    assert!(router.dispatch("   ").is_empty());
}

// ─── 核心指令命中测试 ───

#[test]
fn dispatch_mute_command() {
    let router = RuledRouter::default();
    let result = router.dispatch("帮我静音");
    assert!(!result.is_empty(), "'帮我静音' 应命中 mute 动作");
    assert_eq!(result[0].action, "mute");
    assert!(result[0].score > 0.0);
    assert!(result[0].confidence_delta >= 0.0);
}

#[test]
fn dispatch_unmute_command() {
    let router = RuledRouter::default();
    // "打开声音" 只匹配 unmute 规则
    let result = router.dispatch("打开声音");
    assert!(!result.is_empty(), "'打开声音' 应命中 unmute 动作");
    assert_eq!(result[0].action, "unmute");
}

#[test]
fn dispatch_set_volume_extracts_percent() {
    let router = RuledRouter::default();
    let result = router.dispatch("音量调到 70");
    assert!(!result.is_empty(), "'音量调到 70' 应命中 set_volume");
    let top = &result[0];
    assert!(top.action == "set_volume" || top.action.contains("volume"));
    // 检查是否提取了音量百分比参数
    let vol = top.param.get("volume_pct")
        .or_else(|| top.param.get("percent"))
        .or_else(|| top.param.get("volume"));
    assert!(vol.is_some(), "应提取音量参数: {:?}", top.param);
}

#[test]
fn dispatch_open_app_wechat() {
    let router = RuledRouter::default();
    let result = router.dispatch("打开微信");
    assert!(!result.is_empty(), "'打开微信' 应命中 open_app");
    assert_eq!(result[0].action, "open_app");
    assert_eq!(
        result[0].param.get("app_name").unwrap().as_str().unwrap(),
        "微信"
    );
}

#[test]
fn dispatch_open_app_feishu() {
    let router = RuledRouter::default();
    let result = router.dispatch("启动 飞书");
    assert!(!result.is_empty());
    assert_eq!(result[0].action, "open_app");
    assert_eq!(
        result[0].param.get("app_name").unwrap().as_str().unwrap(),
        "飞书"
    );
}

// ─── DefaultRouter ───

#[test]
fn default_router_new_and_default() {
    // 验证 DefaultRouter 两种构造方式都能正常编译
    let _r1 = DefaultRouter::new();
    let _r2 = DefaultRouter::default();
    // 用 RuledRouter 来验证构造等价性和功能一致
    let rr1 = RuledRouter::default();
    let rr2 = RuledRouter::default();
    let out1 = rr1.dispatch("静音");
    let out2 = rr2.dispatch("静音");
    assert_eq!(out1.len(), out2.len());
}

#[tokio::test]
async fn default_router_dispatch_async() {
    use mox_voice_core_svc::engine::IntentRouter;
    let router = DefaultRouter::new();
    let result = router.dispatch("打开计算器").await;
    assert!(result.is_ok());
    let actions = result.unwrap();
    assert!(!actions.is_empty());
    assert_eq!(actions[0].action, "open_app");
}

// ─── IntentRouterImpl 类型别名 ───

#[test]
fn intent_router_impl_is_default_router() {
    // 类型别名应能正常使用
    let _router: IntentRouterImpl = DefaultRouter::new();
    let _router: IntentRouterImpl = DefaultRouter::default();
}

// ─── 规则常量 ───

#[test]
fn app_alias_exact_list_not_empty() {
    assert!(!APP_ALIAS_EXACT_LIST.is_empty());
    // 验证至少有一些常见应用
    let aliases: Vec<&str> = APP_ALIAS_EXACT_LIST.iter().map(|(k, _)| *k).collect();
    assert!(aliases.contains(&"微信"));
    assert!(aliases.contains(&"飞书"));
    assert!(aliases.contains(&"Chrome"));
    assert!(aliases.contains(&"VS Code"));
    assert!(aliases.contains(&"记事本"));
    assert!(aliases.contains(&"计算器"));
}

#[test]
fn app_alias_exact_list_mapping() {
    // 每个别名映射应该是非空的
    for (alias, key) in APP_ALIAS_EXACT_LIST {
        assert!(!alias.is_empty(), "别名不能为空");
        assert!(!key.is_empty(), "key 不能为空");
    }
}

#[test]
fn common_key_names_not_empty() {
    assert!(!COMMON_KEY_NAMES.is_empty());
}

// ─── build_rule_set ───

#[test]
fn build_rule_set_returns_rules() {
    let rules = build_rule_set();
    assert!(!rules.is_empty(), "规则集不应为空");
    // 所有规则的 regex 应能正常编译（函数返回即说明编译通过）
    for rule in &rules {
        assert!(rule.score > 0.0 && rule.score <= 1.0, "score 应在 (0, 1] 范围内");
        assert!(!rule.category.is_empty(), "category 不应为空");
    }
}

// ─── 结果排序（score 降序） ───

#[test]
fn dispatch_results_sorted_by_score_desc() {
    let router = RuledRouter::default();
    let result = router.dispatch("打开音量调到 50 微信");
    if result.len() >= 2 {
        for i in 0..result.len() - 1 {
            assert!(
                result[i].score >= result[i + 1].score,
                "结果应按 score 降序排列: {} < {}",
                result[i].score,
                result[i + 1].score
            );
        }
    }
}

#[test]
fn dispatch_top1_has_confidence_delta() {
    let router = RuledRouter::default();
    let result = router.dispatch("帮我静音");
    if !result.is_empty() {
        // top1 的 confidence_delta 应该是 top1 - top2
        assert!(result[0].confidence_delta >= 0.0);
    }
    if result.len() >= 2 {
        // 非 top1 的 confidence_delta 应为 0
        for item in result.iter().skip(1) {
            assert_eq!(item.confidence_delta, 0.0);
        }
    }
}

// ─── 截图命令 ───

#[test]
fn dispatch_screenshot_command() {
    let router = RuledRouter::default();
    let result = router.dispatch("截屏");
    assert!(!result.is_empty(), "'截屏' 应命中截图动作");
    assert!(
        result[0].action.contains("screenshot") || result[0].action.contains("screen"),
        "动作名应包含截图相关: {}",
        result[0].action
    );
}
