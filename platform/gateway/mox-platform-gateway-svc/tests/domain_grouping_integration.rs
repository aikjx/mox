// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 域归并分组集成测试
//!
//! 验证 43 个域描述符按 9 个能力组（platform/knowledge/ai/orchestration/
//! storage/data/media/commerce/streaming）归并，且 /api/v1/domains 端点
//! 输出 group 字段。这是企业级模块化治理的门禁测试：新增域必须归入正确能力组。

use axum::{routing::get, Router};
use mox_platform_gateway_svc::domains_handler;
use mox_platform_gateway_svc::routes::DOMAINS;
use serde_json::Value;
use std::collections::HashSet;

/// 9 个权威能力组（与 routes.rs GROUP_MAP 一一对应）
const EXPECTED_GROUPS: &[&str] = &[
    "platform",
    "knowledge",
    "ai",
    "orchestration",
    "storage",
    "data",
    "media",
    "commerce",
    "streaming",
];

/// 在临时端口上启动路由，返回基址
async fn spawn(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{}", addr)
}

// ====================================================================
// 1. 静态常量级验证（单元测试级，快速门禁）
// ====================================================================

#[test]
fn test_domain_count_is_43() {
    assert_eq!(
        DOMAINS.len(),
        43,
        "域数量变更：当前 {}，预期 43。若为有意新增/删除，请同步更新本测试与文档。",
        DOMAINS.len()
    );
}

#[test]
fn test_every_domain_has_group() {
    let missing: Vec<&str> = DOMAINS
        .iter()
        .filter(|d| d.group.is_empty())
        .map(|d| d.name)
        .collect();
    assert!(
        missing.is_empty(),
        "以下域缺少能力组归属：{:?}",
        missing
    );
}

#[test]
fn test_all_9_groups_present() {
    let groups: HashSet<&str> = DOMAINS.iter().map(|d| d.group).collect();
    for expected in EXPECTED_GROUPS {
        assert!(
            groups.contains(expected),
            "能力组 '{}' 不存在于域描述符中",
            expected
        );
    }
    assert_eq!(
        groups.len(),
        EXPECTED_GROUPS.len(),
        "能力组数量不符：当前 {:?}，预期 {:?}",
        groups,
        EXPECTED_GROUPS
    );
}

#[test]
fn test_group_domain_mapping_correct() {
    // 验证关键域的能力组归属（防止误归并）
    let expected: &[(&str, &str)] = &[
        ("Health", "platform"),
        ("RBAC", "platform"),
        ("Audit", "platform"),
        ("KG", "knowledge"),
        ("KB", "knowledge"),
        ("Expert", "ai"),
        ("Alliance", "ai"),
        ("Flow", "orchestration"),
        ("Cloud", "storage"),
        ("Voice", "media"),
        ("Melody", "media"),
        ("Market", "commerce"),
        ("Kafka", "streaming"),
        ("Data", "data"),
    ];
    for (name, expected_group) in expected {
        let domain = DOMAINS
            .iter()
            .find(|d| d.name == *name)
            .unwrap_or_else(|| panic!("域 '{}' 不存在", name));
        assert_eq!(
            domain.group, *expected_group,
            "域 '{}' 应归入能力组 '{}'，实际为 '{}'",
            name, expected_group, domain.group
        );
    }
}

#[test]
fn test_no_unknown_groups() {
    let allowed: HashSet<&str> = EXPECTED_GROUPS.iter().copied().collect();
    let unknown: Vec<&str> = DOMAINS
        .iter()
        .filter(|d| !allowed.contains(d.group))
        .map(|d| d.group)
        .collect();
    assert!(
        unknown.is_empty(),
        "发现未登记的能力组：{:?}（请先在 EXPECTED_GROUPS 登记）",
        unknown
    );
}

// ====================================================================
// 2. HTTP 集成测试（端到端验证 /api/v1/domains 输出）
// ====================================================================

#[tokio::test]
async fn test_domains_endpoint_returns_group_field() {
    let router = Router::new().route("/api/v1/domains", get(domains_handler));
    let base = spawn(router).await;

    let resp = reqwest::get(format!("{}/api/v1/domains", base))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: Value = resp.json().await.unwrap();
    let domains = body["data"]["domains"].as_array().expect("domains 应为数组");
    assert_eq!(domains.len(), 43);

    // 每个域都必须有 group 字段且非空
    for (i, d) in domains.iter().enumerate() {
        let group = d["group"].as_str().unwrap_or("");
        assert!(
            !group.is_empty(),
            "第 {} 个域 ({:?}) 缺少 group 字段",
            i,
            d["name"]
        );
    }

    // 验证 total 字段
    assert_eq!(body["data"]["total"].as_i64().unwrap(), 43);
}

#[tokio::test]
async fn test_domains_endpoint_group_distribution() {
    let router = Router::new().route("/api/v1/domains", get(domains_handler));
    let base = spawn(router).await;

    let resp = reqwest::get(format!("{}/api/v1/domains", base))
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let domains = body["data"]["domains"].as_array().unwrap();

    // 统计各能力组的域数量
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for d in domains {
        let group = d["group"].as_str().unwrap().to_string();
        *counts.entry(group).or_insert(0) += 1;
    }

    // 9 个组都应有域
    assert_eq!(counts.len(), 9, "能力组数量应为 9，实际 {}", counts.len());

    // 关键组的域数量 sanity check
    assert!(counts.get("platform").unwrap() >= &5, "platform 组应至少 5 个域");
    assert!(counts.get("knowledge").unwrap() >= &3, "knowledge 组应至少 3 个域");
    assert!(counts.get("ai").unwrap() >= &3, "ai 组应至少 3 个域");
    assert!(counts.get("media").unwrap() >= &2, "media 组应至少 2 个域");
}
