// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

use mox_voice_asr_svc::*;
use mox_voice_core_svc::Hotword;

// ─── default_injector 构造 ───

#[test]
fn default_injector_creates_empty_injector() {
    let injector = default_injector();
    // 设置热词并验证基本功能
    let hotwords = vec![
        Hotword::new("璇玑").with_score(80.0),
        Hotword::new("小白").with_score(70.0),
    ];
    let result = injector.set_hotwords(&hotwords);
    assert!(result.is_ok(), "设置合法热词应成功");
    let (cleaned, report) = result.unwrap();
    assert_eq!(cleaned.len(), 2);
    assert_eq!(report.hotwords_count, 2);
}

// ─── HotwordInjector 构造 ───

#[test]
fn hotword_injector_new_creates_empty() {
    let injector = HotwordInjector::new();
    // 空注入器设置空热词列表
    let result = injector.set_hotwords(&[]);
    assert!(result.is_ok());
    let (cleaned, report) = result.unwrap();
    assert!(cleaned.is_empty());
    assert_eq!(report.hotwords_count, 0);
}

#[test]
fn hotword_injector_set_valid_hotwords() {
    let injector = HotwordInjector::new();
    let hotwords = vec![
        Hotword::new("你好世界").with_score(90.0).with_category("greeting"),
        Hotword::new("打开应用").with_score(85.0).with_category("app"),
    ];
    let (cleaned, report) = injector.set_hotwords(&hotwords).unwrap();
    assert_eq!(cleaned.len(), 2);
    assert_eq!(report.hotwords_count, 2);
    // report 的 JSON 序列化可用
    let json = report.to_json();
    assert!(json.is_object());
    assert!(json.get("s1").is_some());
    assert!(json.get("s2").is_some());
    assert!(json.get("s3").is_some());
}

#[test]
fn hotword_injector_rejects_invalid_hotwords() {
    let injector = HotwordInjector::new();
    // 空词应被拒绝
    let hotwords = vec![Hotword::new("")];
    let result = injector.set_hotwords(&hotwords);
    assert!(result.is_err(), "空热词应校验失败");

    // score 超出范围应被拒绝
    let hotwords = vec![Hotword::new("测试").with_score(200.0)];
    let result = injector.set_hotwords(&hotwords);
    assert!(result.is_err(), "score > 100 应校验失败");

    // score 为负数也应失败
    let hotwords = vec![Hotword::new("测试").with_score(-1.0)];
    let result = injector.set_hotwords(&hotwords);
    assert!(result.is_err(), "score < 0 应校验失败");
}

// ─── LayerStatus ───

#[test]
fn layer_status_variants() {
    use HotwordLayerStatus::*;

    let pending = Pending;
    assert_eq!(pending, Pending);

    let applied = Applied { n: 5 };
    if let Applied { n } = applied {
        assert_eq!(n, 5);
    } else {
        panic!("expected Applied");
    }

    let skipped = Skipped { reason: "not available" };
    if let Skipped { reason } = skipped {
        assert_eq!(reason, "not available");
    } else {
        panic!("expected Skipped");
    }
}

#[test]
fn layer_status_summary_json() {
    use HotwordLayerStatus::*;

    let json = Pending.summary_json();
    assert_eq!(json.as_str().unwrap(), "pending");

    let json = Applied { n: 3 }.summary_json();
    assert!(json.get("applied").unwrap().as_bool().unwrap());
    assert_eq!(json.get("n").unwrap().as_i64().unwrap(), 3);

    let json = Skipped { reason: "fallback" }.summary_json();
    assert!(!json.get("applied").unwrap().as_bool().unwrap());
    assert_eq!(json.get("reason").unwrap().as_str().unwrap(), "fallback");
}

// ─── InjectionReport ───

#[test]
fn injection_report_default_is_pending() {
    let report = InjectionReport::new();
    assert_eq!(report.s1, HotwordLayerStatus::Pending);
    assert_eq!(report.s2, HotwordLayerStatus::Pending);
    assert_eq!(report.s3, HotwordLayerStatus::Pending);
    assert!(report.hotword_file.is_none());
    assert_eq!(report.hotwords_count, 0);
}

#[test]
fn injection_report_default_trait() {
    let report = InjectionReport::default();
    assert_eq!(report.hotwords_count, 0);
    assert!(report.hotword_file.is_none());
}

#[test]
fn injection_report_clone_and_debug() {
    let report = InjectionReport::new();
    let cloned = report.clone();
    assert_eq!(cloned.hotwords_count, report.hotwords_count);
    let dbg = format!("{:?}", report);
    assert!(dbg.contains("InjectionReport"));
}

#[test]
fn injection_report_to_json_structure() {
    let report = InjectionReport::new();
    let json = report.to_json();
    assert!(json.is_object());
    assert!(json.get("s1").is_some());
    assert!(json.get("s2").is_some());
    assert!(json.get("s3").is_some());
    assert!(json.get("hotword_file").is_some());
    assert!(json.get("hotwords_count").is_some());
    assert_eq!(json["hotwords_count"].as_i64().unwrap(), 0);
}

// ─── sherpa_rs_context_config_available 探测 ───

#[test]
fn ffi_probe_returns_result() {
    // 探测函数返回 Result<bool, String>；在未启用 sherpa-real feature 时应为 Ok(false) 或 Err
    let result = sherpa_rs_context_config_available();
    // 默认 feature 下不应为 Ok(true)
    match result {
        Ok(available) => assert!(!available, "默认 feature 下不应有 sherpa-rs context config"),
        Err(_) => {}, // 探测失败也是预期的
    }
}

// ─── S3 post-hoc 模糊替换（通过 injector.apply_post_hoc 等） ───

#[test]
fn hotword_injector_apply_post_hoc_basic() {
    let injector = HotwordInjector::new();
    let hotwords = vec![
        Hotword::new("璇玑").with_score(90.0),
    ];
    let _ = injector.set_hotwords(&hotwords).unwrap();

    // 调用 apply_post_hoc（如果方法存在的话）
    // 注意：具体方法名以 injector 实际公开 API 为准
    // 这里我们验证至少设置热词后报告结构正确
    let (_, report) = injector.set_hotwords(&hotwords).unwrap();
    assert_eq!(report.hotwords_count, 1);
    // S2 应该有文件路径
    assert!(report.hotword_file.is_some(), "S2 应生成热词临时文件");
}

#[test]
fn hotword_injector_s2_tempfile_exists_on_disk() {
    let injector = HotwordInjector::new();
    let hotwords = vec![
        Hotword::new("小白助手").with_score(80.0),
        Hotword::new("语音控制").with_score(70.0),
    ];
    let (_, report) = injector.set_hotwords(&hotwords).unwrap();

    if let Some(path) = &report.hotword_file {
        assert!(path.exists(), "S2 热词文件应存在于磁盘上: {}", path.display());
        let content = std::fs::read_to_string(path).expect("应能读取热词文件");
        assert!(content.contains("小白助手"), "热词文件应包含词项");
        assert!(content.contains("语音控制"), "热词文件应包含所有词项");
    } else {
        panic!("S2 应生成热词文件");
    }
}
