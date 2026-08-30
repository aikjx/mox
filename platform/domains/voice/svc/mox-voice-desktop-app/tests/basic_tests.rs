// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

use mox_voice_desktop_app::*;

// ─── BallWidgetState ───

#[test]
fn ball_widget_state_default_is_idle() {
    assert_eq!(BallWidgetState::default(), BallWidgetState::Idle);
}

#[test]
fn ball_widget_state_all_five_states() {
    use BallWidgetState::*;
    let all = BallWidgetState::ALL;
    assert_eq!(all.len(), 5);
    assert!(all.contains(&Idle));
    assert!(all.contains(&Listen));
    assert!(all.contains(&Think));
    assert!(all.contains(&Speak));
    assert!(all.contains(&Executing));
}

#[test]
fn ball_widget_state_names() {
    use BallWidgetState::*;
    assert_eq!(Idle.name(), "idle");
    assert_eq!(Listen.name(), "listen");
    assert_eq!(Think.name(), "think");
    assert_eq!(Speak.name(), "speak");
    assert_eq!(Executing.name(), "executing");
}

#[test]
fn ball_widget_state_suggested_hex() {
    use BallWidgetState::*;
    assert_eq!(Idle.suggested_hex(), "#9CA3AF");
    assert_eq!(Listen.suggested_hex(), "#EF4444");
    assert_eq!(Think.suggested_hex(), "#3B82F6");
    assert_eq!(Speak.suggested_hex(), "#10B981");
    assert_eq!(Executing.suggested_hex(), "#F97316");
}

#[test]
fn ball_widget_state_discriminant_values() {
    use BallWidgetState::*;
    assert_eq!(Idle as u8, 0);
    assert_eq!(Listen as u8, 1);
    assert_eq!(Think as u8, 2);
    assert_eq!(Speak as u8, 3);
    assert_eq!(Executing as u8, 4);
}

#[test]
fn ball_widget_state_try_from_u8() {
    use BallWidgetState::*;
    assert_eq!(BallWidgetState::try_from(0).unwrap(), Idle);
    assert_eq!(BallWidgetState::try_from(1).unwrap(), Listen);
    assert_eq!(BallWidgetState::try_from(2).unwrap(), Think);
    assert_eq!(BallWidgetState::try_from(3).unwrap(), Speak);
    assert_eq!(BallWidgetState::try_from(4).unwrap(), Executing);
    // 非法值应返回 Err
    assert!(BallWidgetState::try_from(5).is_err());
    assert!(BallWidgetState::try_from(255).is_err());
}

#[test]
fn ball_widget_state_clone_copy_eq() {
    use BallWidgetState::*;
    let s = Listen;
    let copied = s; // Copy
    assert_eq!(copied, Listen);
    let cloned = s.clone();
    assert_eq!(cloned, s);
}

#[test]
fn ball_widget_state_debug_fmt() {
    let dbg = format!("{:?}", BallWidgetState::Executing);
    assert!(dbg.contains("Executing"));
}

#[test]
fn ball_widget_state_serde_roundtrip() {
    use BallWidgetState::*;
    let json = serde_json::to_string(&Think).unwrap();
    let back: BallWidgetState = serde_json::from_str(&json).unwrap();
    assert_eq!(back, Think);
}

// ─── WidgetMode ───

#[test]
fn widget_mode_default_is_floating_ball() {
    assert_eq!(WidgetMode::default(), WidgetMode::FloatingBall);
}

#[test]
fn widget_mode_variants() {
    use WidgetMode::*;
    let modes = vec![FloatingBall, TrayOnly, Sidebar];
    assert_eq!(modes.len(), 3);
}

#[test]
fn widget_mode_discriminant_values() {
    use WidgetMode::*;
    assert_eq!(FloatingBall as u8, 0);
    assert_eq!(TrayOnly as u8, 1);
    assert_eq!(Sidebar as u8, 2);
}

#[test]
fn widget_mode_clone_copy_eq_debug() {
    use WidgetMode::*;
    let m = TrayOnly;
    let copied = m; // Copy
    assert_eq!(copied, TrayOnly);
    let cloned = m.clone();
    assert_eq!(cloned, m);
    let dbg = format!("{:?}", Sidebar);
    assert!(dbg.contains("Sidebar"));
}

#[test]
fn widget_mode_serde_roundtrip() {
    use WidgetMode::*;
    let json = serde_json::to_string(&TrayOnly).unwrap();
    let back: WidgetMode = serde_json::from_str(&json).unwrap();
    assert_eq!(back, TrayOnly);
}

// ─── HotkeyBindings ───

#[test]
fn hotkey_bindings_default_construction() {
    let bindings = HotkeyBindings::default();
    let _ = bindings; // 编译通过即可
}

#[test]
fn hotkey_bindings_debug_fmt() {
    let bindings = HotkeyBindings::default();
    let dbg = format!("{:?}", bindings);
    assert!(!dbg.is_empty(), "Debug 输出不应为空");
}

// ─── DesktopApp ───

#[test]
fn desktop_app_type_exists() {
    // 验证 DesktopApp 类型存在且可引用
    // 我们不实际构造它（因为它依赖 GUI/窗口系统），
    // 只验证类型可以在编译时被引用
    let _: Option<DesktopApp> = None;
}

// ─── voice_engine 重导出 ───

#[test]
fn voice_engine_module_reexported() {
    // 验证 voice_engine 模块从重导出路径可用
    // （具体类型取决于 feature，但模块本身应存在）
    // 我们只验证模块路径可编译
    let _ = std::any::TypeId::of::<&voice_engine::Recorder>();
}

// ─── 状态机转换（基本验证） ───

#[test]
fn all_states_have_unique_names() {
    use std::collections::HashSet;
    let mut names = HashSet::new();
    for s in BallWidgetState::ALL.iter() {
        assert!(names.insert(s.name()), "状态名不应重复: {}", s.name());
    }
    assert_eq!(names.len(), 5);
}

#[test]
fn all_states_have_unique_hex_colors() {
    use std::collections::HashSet;
    let mut colors = HashSet::new();
    for s in BallWidgetState::ALL.iter() {
        assert!(
            colors.insert(s.suggested_hex()),
            "颜色不应重复: {}",
            s.suggested_hex()
        );
    }
    assert_eq!(colors.len(), 5);
}

#[test]
fn state_transitions_from_idle() {
    // Idle 状态应能转到 Listen（开始录音）
    // 这里我们只验证状态枚举的基本属性
    use BallWidgetState::*;
    assert_ne!(Idle, Listen);
    assert_ne!(Idle, Think);
    assert_ne!(Idle, Speak);
    assert_ne!(Idle, Executing);
}
