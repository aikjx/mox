// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

use mox_voice_core_svc::*;
use serde_json::json;
use std::sync::Arc;

// ─── 常量 ───

#[test]
fn constants_are_defined() {
    assert!(!XIAOBAI_CRATE_ID.is_empty());
    assert!(!XIAOBAI_ENGINE_NAME.is_empty());
    assert_eq!(XIAOBAI_PROTOCOL_VERSION, "AIS-FR13/V1.0");
    assert!((AMBIGUITY_THRESHOLD - 0.10).abs() < f32::EPSILON);
    assert!(S6_WEEKLY_CYCLE_MS > 0);
    assert_eq!(MAX_HOTWORD_LEN, 40);
    assert_eq!(HOTWORD_SCORE_MIN, 0.0);
    assert_eq!(HOTWORD_SCORE_MAX, 100.0);
}

// ─── Hotword ───

#[test]
fn hotword_new_default_values() {
    let hw = Hotword::new("测试词");
    assert_eq!(hw.word, "测试词");
    assert_eq!(hw.score, 50.0);
    assert_eq!(hw.category, "general");
}

#[test]
fn hotword_builder_pattern() {
    let hw = Hotword::new("璇玑")
        .with_score(95.0)
        .with_category("app");
    assert_eq!(hw.word, "璇玑");
    assert_eq!(hw.score, 95.0);
    assert_eq!(hw.category, "app");
}

#[test]
fn hotword_validate_valid() {
    let hw = Hotword::new("正常热词");
    assert!(hw.validate(1).is_ok());
}

#[test]
fn hotword_validate_empty_word() {
    let hw = Hotword::new("");
    let result = hw.validate(1);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("XB-002"), "错误码应为 XB-002，实际: {}", msg);
}

#[test]
fn hotword_validate_score_out_of_range() {
    let hw = Hotword::new("测试").with_score(150.0);
    assert!(hw.validate(1).is_err());

    let hw = Hotword::new("测试").with_score(-5.0);
    assert!(hw.validate(1).is_err());
}

#[test]
fn hotword_validate_empty_category() {
    let hw = Hotword::new("测试").with_category("   ");
    assert!(hw.validate(1).is_err());
}

#[test]
fn hotword_validate_long_word() {
    let long_word: String = std::iter::repeat("字").take(41).collect();
    let hw = Hotword::new(long_word);
    assert!(hw.validate(1).is_err());
}

#[test]
fn hotword_serde_roundtrip() {
    let hw = Hotword::new("你好").with_score(80.0).with_category("greeting");
    let json = serde_json::to_string(&hw).unwrap();
    let hw2: Hotword = serde_json::from_str(&json).unwrap();
    assert_eq!(hw, hw2);
}

// ─── ClearanceLevel ───

#[test]
fn clearance_level_ordering() {
    use ClearanceLevel::*;
    assert!(L0 < L1);
    assert!(L1 < L2);
    assert!(L2 < L3);
    assert_eq!(L3 as u8, 3);
    assert_eq!(L0 as u8, 0);
}

#[test]
fn clearance_level_as_u8_and_from_u8() {
    use ClearanceLevel::*;
    assert_eq!(L0.as_u8(), 0);
    assert_eq!(L1.as_u8(), 1);
    assert_eq!(L2.as_u8(), 2);
    assert_eq!(L3.as_u8(), 3);

    assert_eq!(ClearanceLevel::from_u8(0).unwrap(), L0);
    assert_eq!(ClearanceLevel::from_u8(3).unwrap(), L3);
    assert!(ClearanceLevel::from_u8(4).is_err());
    assert!(ClearanceLevel::from_u8(255).is_err());
}

#[test]
fn clearance_level_labels_zh() {
    use ClearanceLevel::*;
    assert_eq!(L0.label_zh(), "只读 (Auditor)");
    assert_eq!(L1.label_zh(), "非破坏 (Member)");
    assert_eq!(L2.label_zh(), "剪贴/键鼠 (Expert/Coordinator)");
    assert_eq!(L3.label_zh(), "破坏性 (MoxAdmin)");
}

#[test]
fn clearance_level_serde() {
    use ClearanceLevel::*;
    let json = serde_json::to_string(&L2).unwrap();
    let back: ClearanceLevel = serde_json::from_str(&json).unwrap();
    assert_eq!(back, L2);
}

// ─── DispatchMode ───

#[test]
fn dispatch_mode_default_is_local_first() {
    assert_eq!(DispatchMode::default(), DispatchMode::LocalFirst);
}

#[test]
fn dispatch_mode_variants() {
    let modes = vec![
        DispatchMode::LocalFirst,
        DispatchMode::CloudFallback,
        DispatchMode::CloudOnly,
    ];
    assert_eq!(modes.len(), 3);
}

#[test]
fn dispatch_mode_serde_snake_case() {
    let json = serde_json::to_string(&DispatchMode::CloudFallback).unwrap();
    assert_eq!(json, "\"cloud_fallback\"");

    let back: DispatchMode = serde_json::from_str("\"local_first\"").unwrap();
    assert_eq!(back, DispatchMode::LocalFirst);
}

#[test]
fn dispatch_mode_parse_loose() {
    use DispatchMode::*;
    assert_eq!(DispatchMode::parse_loose("local_first").unwrap(), LocalFirst);
    assert_eq!(DispatchMode::parse_loose("cloud_fallback").unwrap(), CloudFallback);
    assert_eq!(DispatchMode::parse_loose("cloud_only").unwrap(), CloudOnly);
    assert_eq!(DispatchMode::parse_loose("  Local-First  ").unwrap(), LocalFirst);
    assert!(DispatchMode::parse_loose("invalid_mode").is_err());
}

// ─── check_clearance ───

#[test]
fn check_clearance_admin_passes_all() {
    use ClearanceLevel::*;
    use identity::OperatorIdentity;
    let admin = OperatorIdentity::admin();
    let action = "test_action";

    assert!(check_clearance(action, L0, &admin, false).is_ok());
    assert!(check_clearance(action, L1, &admin, false).is_ok());
    assert!(check_clearance(action, L2, &admin, false).is_ok());
    assert!(check_clearance(action, L3, &admin, false).is_ok());
}

#[test]
fn check_clearance_auditor_fails_higher() {
    use ClearanceLevel::*;
    use identity::OperatorIdentity;
    let auditor = OperatorIdentity::auditor();
    let action = "test_action";

    assert!(check_clearance(action, L0, &auditor, false).is_ok());
    assert!(check_clearance(action, L1, &auditor, false).is_err());
    assert!(check_clearance(action, L2, &auditor, false).is_err());
    assert!(check_clearance(action, L3, &auditor, false).is_err());
}

#[test]
fn check_clearance_member_passes_l0_l1() {
    use ClearanceLevel::*;
    use identity::OperatorIdentity;
    let member = OperatorIdentity::member();
    let action = "test_action";

    assert!(check_clearance(action, L0, &member, false).is_ok());
    assert!(check_clearance(action, L1, &member, false).is_ok());
    assert!(check_clearance(action, L2, &member, false).is_err());
    assert!(check_clearance(action, L3, &member, false).is_err());
}

#[test]
fn check_clearance_owner_bonus_lifts_level() {
    use ClearanceLevel::*;
    // Expert (L2) + Owner = L3 等效
    let expert = identity::OperatorIdentity::new(
        "tester-expert",
        identity::RoleTag::Expert,
        true, // is_owner
    );
    let action = "move_to_trash_own";
    assert!(check_clearance(action, L3, &expert, true).is_ok());
}

// ─── OperatorCategory ───

#[test]
fn operator_category_all_eight() {
    use OperatorCategory::*;
    let all = vec![App, File, Volume, Input, Network, Display, Browser, Notify];
    assert_eq!(all.len(), 8);
}

#[test]
fn operator_category_as_str() {
    use OperatorCategory::*;
    assert_eq!(App.as_str(), "app");
    assert_eq!(File.as_str(), "file");
    assert_eq!(Volume.as_str(), "volume");
    assert_eq!(Input.as_str(), "input");
    assert_eq!(Network.as_str(), "network");
    assert_eq!(Display.as_str(), "display");
    assert_eq!(Browser.as_str(), "browser");
    assert_eq!(Notify.as_str(), "notify");
}

#[test]
fn operator_category_label_zh() {
    use OperatorCategory::*;
    assert_eq!(App.label_zh(), "应用控制");
    assert_eq!(File.label_zh(), "文件操作");
    assert_eq!(Volume.label_zh(), "音量控制");
    assert_eq!(Notify.label_zh(), "系统通知");
}

#[test]
fn operator_category_serde() {
    use OperatorCategory::*;
    let json = serde_json::to_string(&Volume).unwrap();
    assert_eq!(json, "\"volume\"");
    let back: OperatorCategory = serde_json::from_str("\"browser\"").unwrap();
    assert_eq!(back, Browser);
}

// ─── ActionParam ───

#[test]
fn action_param_new_and_getters() {
    let param = ActionParam::new(json!({
        "name": "chrome",
        "count": 5,
        "ratio": 0.75,
        "enabled": true,
    }));
    assert_eq!(param.get_str("name"), Some("chrome"));
    assert_eq!(param.get_i64("count"), Some(5));
    assert_eq!(param.get_f64("ratio"), Some(0.75));
    assert_eq!(param.get_bool("enabled"), Some(true));
    assert_eq!(param.get_str("missing"), None);
}

#[test]
fn action_param_null() {
    let param = ActionParam::null();
    assert!(param.0.is_null());
    assert_eq!(param.get_str("anything"), None);
}

#[test]
fn action_param_default_is_null() {
    let param = ActionParam::default();
    assert!(param.0.is_null());
}

// ─── OperatorOutput ───

#[test]
fn operator_output_quick_construction() {
    let output = OperatorOutput::quick("操作成功");
    assert_eq!(output.message, "操作成功");
    assert!(output.payload.is_none());
    assert!(output.fallbacks_used.is_empty());
    assert_eq!(output.elapsed_ms, 0);
}

#[test]
fn operator_output_builder_pattern() {
    let output = OperatorOutput::quick("操作成功")
        .with_payload(json!({"result": "ok"}))
        .with_fallbacks(vec!["win32".into()])
        .with_elapsed(42);

    assert_eq!(output.message, "操作成功");
    assert_eq!(output.payload.as_ref().unwrap()["result"], "ok");
    assert_eq!(output.fallbacks_used, vec!["win32"]);
    assert_eq!(output.elapsed_ms, 42);
}

#[test]
fn operator_output_push_fallback() {
    let mut output = OperatorOutput::quick("test");
    output.push_fallback("pycaw");
    output.push_fallback("waveOut");
    assert_eq!(output.fallbacks_used.len(), 2);
}

#[test]
fn operator_output_serde_roundtrip() {
    let output = OperatorOutput::quick("成功").with_payload(json!({"ok": true}));
    let json_str = serde_json::to_string(&output).unwrap();
    assert!(json_str.contains("成功"));
    // 反序列化回来
    let back: OperatorOutput = serde_json::from_str(&json_str).unwrap();
    assert_eq!(back.message, "成功");
    assert_eq!(back.payload.unwrap()["ok"], true);
}

// ─── XiaobaiError ───

#[test]
fn xiaobai_error_display_contains_error_code() {
    let err = XiaobaiError::IntentUnknown("测试指令".into());
    let msg = format!("{}", err);
    assert!(msg.contains("XB-004"), "应包含 XB-004: {}", msg);
    assert!(msg.contains("测试指令"));
}

#[test]
fn xiaobai_error_invalid_argument_format() {
    let err = XiaobaiError::InvalidArgument {
        action: "test_action".into(),
        param: "param1".into(),
        value: "bad".into(),
        hint: "应为正整数".into(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("XB-009"), "应包含 XB-009: {}", msg);
    assert!(msg.contains("test_action"));
    assert!(msg.contains("param1"));
}

#[test]
fn xiaobai_error_permission_denied_format() {
    use identity::OperatorIdentity;
    let id = OperatorIdentity::auditor();
    let err = XiaobaiError::PermissionDenied {
        action: "close_app".into(),
        required: errors::ClearanceLevelRepr(3),
        identity: id.stable_display(),
        reason: "动作属于破坏性，需要 MoxAdmin 角色或 Owner",
    };
    let msg = format!("{}", err);
    assert!(msg.contains("XB-001"), "应包含 XB-001: {}", msg);
    assert!(msg.contains("close_app"));
}

// ─── Envelope / EnvelopeKind ───

#[test]
fn envelope_kind_all_variants() {
    use EnvelopeKind::*;
    let kinds = vec![Intent, Exec, Audit, Ack, Ping, Error];
    assert_eq!(kinds.len(), 6);
}

#[test]
fn envelope_kind_as_str() {
    use EnvelopeKind::*;
    assert_eq!(Intent.as_str(), "intent");
    assert_eq!(Exec.as_str(), "exec");
    assert_eq!(Audit.as_str(), "audit");
    assert_eq!(Ack.as_str(), "ack");
    assert_eq!(Ping.as_str(), "ping");
    assert_eq!(Error.as_str(), "error");
}

#[test]
fn envelope_kind_serde() {
    use EnvelopeKind::*;
    let json = serde_json::to_string(&Intent).unwrap();
    assert_eq!(json, "\"intent\"");
    let back: EnvelopeKind = serde_json::from_str("\"error\"").unwrap();
    assert_eq!(back, Error);
}

#[test]
fn envelope_new_intent_structure() {
    use identity::OperatorIdentity;
    let identity = OperatorIdentity::member();
    let env = Envelope::new_intent("test-sender", "打开微信", &identity, DispatchMode::LocalFirst);
    assert_eq!(env.version, XIAOBAI_PROTOCOL_VERSION);
    assert_eq!(env.kind, EnvelopeKind::Intent);
    assert_eq!(env.sender, "test-sender");
    assert!(!env.id.is_empty());
    assert!(env.ts_ms > 0);
    assert!(env.reply_to.is_none());
    // payload 应该包含 text 字段
    assert_eq!(env.payload["text"], "打开微信");
    assert_eq!(env.payload["mode"], json!("local_first"));
}

// ─── OperatorIdentity ───

#[test]
fn operator_identity_default_is_member() {
    let identity = identity::OperatorIdentity::default();
    assert_eq!(identity.role.to_clearance_level(), ClearanceLevel::L1);
    assert!(!identity.user_id.is_empty());
    assert!(!identity.is_owner);
}

#[test]
fn operator_identity_member_role() {
    let identity = identity::OperatorIdentity::member();
    assert_eq!(identity.role.to_clearance_level(), ClearanceLevel::L1);
}

#[test]
fn operator_identity_admin_role() {
    let identity = identity::OperatorIdentity::admin();
    assert_eq!(identity.role.to_clearance_level(), ClearanceLevel::L3);
}

#[test]
fn operator_identity_auditor_role() {
    let identity = identity::OperatorIdentity::auditor();
    assert_eq!(identity.role.to_clearance_level(), ClearanceLevel::L0);
}

#[test]
fn operator_identity_expert_and_coord_both_l2() {
    let expert = identity::OperatorIdentity::expert();
    let coord = identity::OperatorIdentity::coord();
    assert_eq!(expert.role.to_clearance_level(), ClearanceLevel::L2);
    assert_eq!(coord.role.to_clearance_level(), ClearanceLevel::L2);
}

#[test]
fn operator_identity_new_custom() {
    let id = identity::OperatorIdentity::new(
        "custom_user",
        identity::RoleTag::Expert,
        true,
    );
    assert_eq!(id.user_id, "custom_user");
    assert_eq!(id.role, identity::RoleTag::Expert);
    assert!(id.is_owner);
}

#[test]
fn operator_identity_stable_display() {
    let id = identity::OperatorIdentity::member();
    let display = id.stable_display();
    assert!(display.contains("uid="));
    assert!(display.contains("clearance="));
    assert!(display.contains("owner="));
}

#[test]
fn operator_identity_serde_roundtrip() {
    let id = identity::OperatorIdentity::admin();
    let json = serde_json::to_string(&id).unwrap();
    let back: identity::OperatorIdentity = serde_json::from_str(&json).unwrap();
    assert_eq!(back.user_id, id.user_id);
    assert_eq!(back.role, id.role);
    assert_eq!(back.is_owner, id.is_owner);
}

// ─── RoleTag ───

#[test]
fn role_tag_parse_loose_variants() {
    use identity::RoleTag;
    use identity::RoleTag::*;
    assert_eq!(RoleTag::parse_loose("mox_admin").unwrap(), MoxAdmin);
    assert_eq!(RoleTag::parse_loose("璇玑管理员").unwrap(), MoxAdmin);
    assert_eq!(RoleTag::parse_loose("admin").unwrap(), MoxAdmin);
    assert_eq!(RoleTag::parse_loose("Expert").unwrap(), Expert);
    assert_eq!(RoleTag::parse_loose("专家").unwrap(), Expert);
    assert_eq!(RoleTag::parse_loose("Member").unwrap(), Member);
    assert_eq!(RoleTag::parse_loose("Auditor").unwrap(), Auditor);
    assert_eq!(RoleTag::parse_loose("Coordinator").unwrap(), Coordinator);
    assert!(RoleTag::parse_loose("unknown_role").is_err());
}

#[test]
fn role_tag_label_zh() {
    use identity::RoleTag::*;
    assert_eq!(MoxAdmin.label_zh(), "璇玑管理员");
    assert_eq!(Coordinator.label_zh(), "协调员");
    assert_eq!(Expert.label_zh(), "专家");
    assert_eq!(Member.label_zh(), "成员");
    assert_eq!(Auditor.label_zh(), "审计员");
}

// ─── EngineConfig / OperatorEngine ───

#[test]
fn engine_config_default_values() {
    let config = EngineConfig::default();
    assert_eq!(config.mode, DispatchMode::LocalFirst);
    assert!(config.nonce_ttl.as_millis() > 0);
    assert!(config.cloud_deadline.as_millis() > 0);
}

#[test]
fn operator_engine_new_with_router() {
    use engine::{IntentRouter, RoutedAction};
    use async_trait::async_trait;

    // 构造一个 stub router
    struct StubRouter;
    #[async_trait]
    impl IntentRouter for StubRouter {
        async fn dispatch(&self, _text: &str) -> XiaobaiResult<Vec<RoutedAction>> {
            Ok(Vec::new())
        }
    }

    let engine = OperatorEngine::new(
        EngineConfig::default(),
        Arc::new(StubRouter),
    );
    // 空引擎注册的动作应为空
    let actions = engine.list_registered_actions();
    assert!(actions.is_empty());
}

// ─── RoutedAction（IntentRouter 回调结果） ───

#[test]
fn routed_action_serde_roundtrip() {
    use engine::RoutedAction;
    let action = RoutedAction {
        action: "open_app".into(),
        category: "app".into(),
        score: 0.95,
        confidence_delta: 0.3,
        param: json!({"app_name": "chrome"}),
    };
    let json = serde_json::to_string(&action).unwrap();
    let back: RoutedAction = serde_json::from_str(&json).unwrap();
    assert_eq!(back.action, "open_app");
    assert_eq!(back.category, "app");
    assert!((back.score - 0.95).abs() < f32::EPSILON);
    assert_eq!(back.param["app_name"], "chrome");
}
