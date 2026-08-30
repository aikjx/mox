// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

use mox_market_api::*;
use serde_json::json;

// ─── PluginStatus 枚举测试 ───────────────────────────────────────────────────

#[test]
fn plugin_status_variants_are_distinct() {
    use PluginStatus::*;
    assert_ne!(Available, Installed);
    assert_ne!(Installed, Enabled);
    assert_ne!(Enabled, Disabled);
    assert_ne!(Disabled, Updating);
    assert_ne!(Updating, Error);
}

#[test]
fn plugin_status_serialization_roundtrip() {
    let statuses = vec![
        PluginStatus::Available,
        PluginStatus::Installed,
        PluginStatus::Enabled,
        PluginStatus::Disabled,
        PluginStatus::Updating,
        PluginStatus::Error,
    ];
    for s in statuses {
        let json = serde_json::to_string(&s).unwrap();
        let deserialized: PluginStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(s, deserialized);
    }
}

// ─── PluginType 枚举测试 ─────────────────────────────────────────────────────

#[test]
fn plugin_type_variants_cover_all_categories() {
    use PluginType::*;
    let all = vec![Source, Transform, Sink, Filter, Enrich, Auth, Storage, Analytics, Ui, Other];
    assert_eq!(all.len(), 10);
    // 验证 Clone + Copy
    let t = Source;
    let t2 = t;
    assert_eq!(t, t2);
}

#[test]
fn plugin_type_serialization_roundtrip() {
    let types = vec![
        PluginType::Source,
        PluginType::Transform,
        PluginType::Sink,
        PluginType::Filter,
        PluginType::Enrich,
        PluginType::Auth,
        PluginType::Storage,
        PluginType::Analytics,
        PluginType::Ui,
        PluginType::Other,
    ];
    for t in types {
        let json = serde_json::to_string(&t).unwrap();
        let deserialized: PluginType = serde_json::from_str(&json).unwrap();
        assert_eq!(t, deserialized);
    }
}

// ─── PluginInfo 结构体测试 ───────────────────────────────────────────────────

#[test]
fn plugin_info_construction_and_fields() {
    let info = PluginInfo {
        id: "plugin-001".to_string(),
        name: "测试插件".to_string(),
        version: "1.0.0".to_string(),
        description: "一个测试用的插件".to_string(),
        author: "mox-team".to_string(),
        plugin_type: PluginType::Source,
        status: PluginStatus::Available,
        tags: vec!["test".to_string(), "demo".to_string()],
        config_schema: json!({"type": "object"}),
        installed_at: None,
        enabled: false,
    };
    assert_eq!(info.id, "plugin-001");
    assert_eq!(info.name, "测试插件");
    assert_eq!(info.version, "1.0.0");
    assert_eq!(info.plugin_type, PluginType::Source);
    assert_eq!(info.status, PluginStatus::Available);
    assert_eq!(info.tags.len(), 2);
    assert!(!info.enabled);
    assert!(info.installed_at.is_none());
}

#[test]
fn plugin_info_clone_and_debug() {
    let info = PluginInfo {
        id: "p1".into(),
        name: "n1".into(),
        version: "v1".into(),
        description: "d1".into(),
        author: "a1".into(),
        plugin_type: PluginType::Transform,
        status: PluginStatus::Enabled,
        tags: vec!["t1".into()],
        config_schema: json!({}),
        installed_at: Some("2026-01-01".into()),
        enabled: true,
    };
    let cloned = info.clone();
    assert_eq!(cloned.id, info.id);
    assert_eq!(cloned.name, info.name);
    // Debug 输出
    let debug_str = format!("{:?}", info);
    assert!(debug_str.contains("PluginInfo"));
    assert!(debug_str.contains("p1"));
}

#[test]
fn plugin_info_serialization_roundtrip() {
    let info = PluginInfo {
        id: "test-plugin".into(),
        name: "Test Plugin".into(),
        version: "0.1.0".into(),
        description: "A test plugin for serialization".into(),
        author: "test-author".into(),
        plugin_type: PluginType::Sink,
        status: PluginStatus::Installed,
        tags: vec!["test".into(), "serialization".into()],
        config_schema: json!({"fields": ["host", "port"]}),
        installed_at: Some("2026-06-15T10:00:00Z".into()),
        enabled: true,
    };
    let json_str = serde_json::to_string(&info).unwrap();
    let deserialized: PluginInfo = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.id, info.id);
    assert_eq!(deserialized.name, info.name);
    assert_eq!(deserialized.plugin_type, info.plugin_type);
    assert_eq!(deserialized.status, info.status);
    assert_eq!(deserialized.tags, info.tags);
    assert_eq!(deserialized.enabled, info.enabled);
}

// ─── PluginInstallation 结构体测试 ───────────────────────────────────────────

#[test]
fn plugin_installation_construction() {
    let inst = PluginInstallation {
        plugin_id: "plugin-001".to_string(),
        tenant_id: "tenant-abc".to_string(),
        config: json!({"interval": 30}),
        installed_at: "2026-08-01T00:00:00Z".to_string(),
        installed_by: "user-123".to_string(),
    };
    assert_eq!(inst.plugin_id, "plugin-001");
    assert_eq!(inst.tenant_id, "tenant-abc");
    assert_eq!(inst.installed_by, "user-123");
    assert_eq!(inst.config["interval"], 30);
}

#[test]
fn plugin_installation_serialization() {
    let inst = PluginInstallation {
        plugin_id: "p1".into(),
        tenant_id: "t1".into(),
        config: json!({"enabled": true}),
        installed_at: "2026-01-01".into(),
        installed_by: "admin".into(),
    };
    let json_val = serde_json::to_value(&inst).unwrap();
    assert_eq!(json_val["plugin_id"], "p1");
    assert_eq!(json_val["tenant_id"], "t1");
    assert_eq!(json_val["config"]["enabled"], true);
}

// ─── ExtensionPoint 结构体测试 ───────────────────────────────────────────────

#[test]
fn extension_point_construction() {
    let ep = ExtensionPoint {
        id: "ep-data-source".to_string(),
        name: "数据源扩展点".to_string(),
        description: "用于注册数据源插件".to_string(),
        domain: "market".to_string(),
        required_interface: "DataSourcePlugin".to_string(),
        registered_plugins: vec!["plugin-a".to_string(), "plugin-b".to_string()],
    };
    assert_eq!(ep.id, "ep-data-source");
    assert_eq!(ep.domain, "market");
    assert_eq!(ep.registered_plugins.len(), 2);
    assert!(ep.registered_plugins.contains(&"plugin-a".to_string()));
}

#[test]
fn extension_point_clone_and_debug() {
    let ep = ExtensionPoint {
        id: "ep1".into(),
        name: "n1".into(),
        description: "d1".into(),
        domain: "dm1".into(),
        required_interface: "if1".into(),
        registered_plugins: vec!["p1".into()],
    };
    let cloned = ep.clone();
    assert_eq!(cloned.id, ep.id);
    let debug_str = format!("{:?}", ep);
    assert!(debug_str.contains("ExtensionPoint"));
}

// ─── MarketApiError 错误类型测试 ─────────────────────────────────────────────

#[test]
fn market_api_error_variants_display() {
    let err = MarketApiError::NotFound("plugin-x".to_string());
    assert!(format!("{}", err).contains("plugin not found"));

    let err = MarketApiError::Conflict("dup".to_string());
    assert!(format!("{}", err).contains("plugin conflict"));

    let err = MarketApiError::Validation("bad input".to_string());
    assert!(format!("{}", err).contains("validation failed"));

    let err = MarketApiError::Installation("fail".to_string());
    assert!(format!("{}", err).contains("installation failed"));

    let err = MarketApiError::Internal("oops".to_string());
    assert!(format!("{}", err).contains("internal"));
}

#[test]
fn market_api_error_is_std_error() {
    let err: Box<dyn std::error::Error> = Box::new(MarketApiError::NotFound("x".into()));
    assert!(err.source().is_none()); // thiserror 没有 source
    assert!(format!("{}", err).contains("plugin not found"));
}

#[test]
fn market_api_result_type_alias_works() {
    let ok: MarketApiResult<i32> = Ok(42);
    assert_eq!(ok.unwrap(), 42);

    let err: MarketApiResult<i32> = Err(MarketApiError::Validation("test".into()));
    assert!(err.is_err());
    assert!(matches!(err.unwrap_err(), MarketApiError::Validation(_)));
}
