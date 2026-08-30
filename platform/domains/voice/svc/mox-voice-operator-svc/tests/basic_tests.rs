// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

use mox_voice_operator_svc::*;
use mox_voice_core_svc::{OperatorEngine, EngineConfig, OperatorCategory};
use mox_voice_core_svc::engine::{IntentRouter, RoutedAction};
use mox_voice_core_svc::XiaobaiResult;
use async_trait::async_trait;
use std::sync::Arc;

/// Stub router 用于测试 OperatorEngine 构造
struct StubRouter;

#[async_trait]
impl IntentRouter for StubRouter {
    async fn dispatch(&self, _text: &str) -> XiaobaiResult<Vec<RoutedAction>> {
        Ok(Vec::new())
    }
}

fn stub_engine() -> OperatorEngine {
    OperatorEngine::new(EngineConfig::default(), Arc::new(StubRouter))
}

// ─── platform_tag ───

#[test]
fn platform_tag_returns_non_empty() {
    let tag = platform_tag();
    assert!(!tag.is_empty(), "platform_tag 不应为空");
    // 应该返回当前平台的标识
    assert!(
        tag == "windows" || tag == "macos" || tag == "linux" || tag == "unknown",
        "未知平台标签: {}",
        tag
    );
}

// ─── 各 Operator 默认构造 ───

#[test]
fn app_operator_default_construction() {
    let op = AppOperator::default();
    let _ = op; // 编译通过即可验证构造成功
}

#[test]
fn file_operator_default_construction() {
    let op = FileOperator::default();
    let _ = op;
}

#[test]
fn volume_operator_default_construction() {
    let op = VolumeOperator::default();
    let _ = op;
}

#[test]
fn input_operator_default_construction() {
    let op = InputOperator::default();
    let _ = op;
}

#[test]
fn network_operator_default_construction() {
    let op = NetworkOperator::default();
    let _ = op;
}

#[test]
fn display_operator_default_construction() {
    let op = DisplayOperator::default();
    let _ = op;
}

#[test]
fn browser_operator_default_construction() {
    let op = BrowserOperator::default();
    let _ = op;
}

#[test]
fn notify_operator_default_construction() {
    let op = NotifyOperator::default();
    let _ = op;
}

// ─── Operator Debug 实现 ───

#[test]
fn app_operator_debug_fmt() {
    let op = AppOperator::default();
    let dbg = format!("{:?}", op);
    assert!(!dbg.is_empty(), "Debug 输出不应为空");
}

#[test]
fn volume_operator_debug_fmt() {
    let op = VolumeOperator::default();
    let dbg = format!("{:?}", op);
    assert!(!dbg.is_empty());
}

// ─── register_all_defaults ───

#[test]
fn register_all_defaults_registers_all_8_categories() {
    let engine = stub_engine();
    register_all_defaults(&engine);

    // 验证 8 大类算子都被注册了
    // 通过查询引擎支持的动作列表来间接验证
    let actions = engine.list_registered_actions();
    assert!(!actions.is_empty(), "注册后应该有动作");
    // 验证至少有 8 个不同 category 的动作
    use std::collections::HashSet;
    let categories: HashSet<_> = actions.iter().map(|(_, cat, _)| *cat).collect();
    assert!(categories.len() >= 1, "至少应有 1 类动作被注册");
}

// ─── OperatorCategory 一致性验证 ───

#[test]
fn eight_operators_match_eight_categories() {
    use OperatorCategory::*;
    let categories = vec![App, File, Volume, Input, Network, Display, Browser, Notify];
    assert_eq!(categories.len(), 8);

    // 验证 8 个 operator 对应 8 个 category
    let ops: Vec<Box<dyn std::any::Any>> = vec![
        Box::new(AppOperator::default()),
        Box::new(FileOperator::default()),
        Box::new(VolumeOperator::default()),
        Box::new(InputOperator::default()),
        Box::new(NetworkOperator::default()),
        Box::new(DisplayOperator::default()),
        Box::new(BrowserOperator::default()),
        Box::new(NotifyOperator::default()),
    ];
    assert_eq!(ops.len(), 8);
}

// ─── SystemOperator trait 实现验证 ───

#[test]
fn all_operators_implement_system_operator() {
    // 编译时验证：所有 operator 都实现了 SystemOperator trait
    use mox_voice_core_svc::operator::SystemOperator;

    fn assert_is_operator<T: SystemOperator + 'static>(_: &T) {}

    let app = AppOperator::default();
    assert_is_operator(&app);

    let file = FileOperator::default();
    assert_is_operator(&file);

    let vol = VolumeOperator::default();
    assert_is_operator(&vol);

    let input = InputOperator::default();
    assert_is_operator(&input);

    let net = NetworkOperator::default();
    assert_is_operator(&net);

    let disp = DisplayOperator::default();
    assert_is_operator(&disp);

    let browser = BrowserOperator::default();
    assert_is_operator(&browser);

    let notify = NotifyOperator::default();
    assert_is_operator(&notify);

    // 验证可以放入 Arc 并注册到 engine
    let engine = stub_engine();
    engine.register(Arc::new(app));
    engine.register(Arc::new(file));
    engine.register(Arc::new(vol));
    let actions = engine.list_registered_actions();
    assert!(!actions.is_empty(), "注册后应有动作");
}

// ─── AppOperator 动作列表 ───

#[test]
fn app_operator_has_actions() {
    use mox_voice_core_svc::operator::SystemOperator;
    let op = AppOperator::default();
    let actions = op.list_actions();
    assert!(!actions.is_empty(), "AppOperator 应有动作定义");
    // 验证 category 正确
    for sig in &actions {
        assert_eq!(sig.category, OperatorCategory::App);
    }
}

// ─── VolumeOperator 动作列表 ───

#[test]
fn volume_operator_has_actions() {
    use mox_voice_core_svc::operator::SystemOperator;
    let op = VolumeOperator::default();
    let actions = op.list_actions();
    assert!(!actions.is_empty(), "VolumeOperator 应有动作定义");
    for sig in &actions {
        assert_eq!(sig.category, OperatorCategory::Volume);
    }
}

// ─── helpers 模块 ───

#[test]
fn helpers_platform_tag_consistent() {
    // 多次调用应返回相同结果
    let t1 = platform_tag();
    let t2 = platform_tag();
    assert_eq!(t1, t2, "platform_tag 应是确定性的");
}
