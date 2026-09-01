//! # Nacos 注册中心真实服务端 e2e（需要本机运行 Nacos 服务端）
//!
//! 前置：启动 rnacos（`platform/domains/alliance/tools/rnacos/rnacos.exe`，127.0.0.1:8848/9848）。
//!
//! 运行（需 naming feature）：
//!   `cargo test -p mox-alliance-boot-config --features naming --test naming_e2e -- --ignored --nocapture`
//!
//! 未运行 Nacos 时：connect 返回 Ok(None)（告警降级），注册/注销验证无法执行 → 断言失败提示先启动 rnacos。

use mox_alliance_boot_config::naming::NamingRegistry;
use mox_alliance_boot_config::{NacosSection, NamingSection};

fn nacos() -> NacosSection {
    NacosSection {
        enabled: true,
        server_addr: "127.0.0.1:8848".to_string(),
        namespace: String::new(),
        ..Default::default()
    }
}

fn naming() -> NamingSection {
    NamingSection {
        enabled: true,
        service_name: "mox-alliance-e2e-svc".to_string(),
        group: "DEFAULT_GROUP".to_string(),
        ip: "127.0.0.1".to_string(),
        port: 3100,
        weight: 1.0,
        metadata: vec!["protocol=http".into(), "domain=alliance".into()],
        ..Default::default()
    }
}

/// 通过 rnacos HTTP 兼容 API 查询实例列表
async fn list_instances(service_name: &str) -> Vec<serde_json::Value> {
    let url = format!(
        "http://127.0.0.1:8848/nacos/v1/ns/instance/list?serviceName={}",
        service_name
    );
    let resp = reqwest::get(&url).await.expect("查询实例列表失败");
    assert!(resp.status().is_success(), "查询实例列表 HTTP {}", resp.status());
    let v: serde_json::Value = resp.json().await.unwrap();
    v["hosts"].as_array().cloned().unwrap_or_default()
}

/// e2e-1：register 真实注册 → 实例列表可查询到（含 metadata）
#[tokio::test]
#[ignore = "需本机运行 Nacos 服务端（rnacos 127.0.0.1:8848/9848）"]
async fn e2e_register_instance_real_nacos() {
    let reg = NamingRegistry::connect(&nacos(), &naming())
        .await
        .expect("connect 失败：请确认 rnacos 已在 127.0.0.1:8848 运行")
        .expect("enabled=true 且 service_name 非空，应返回 Some");

    reg.register().await;

    // 稍等 gRPC 注册同步到 HTTP 查询
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    let hosts = list_instances("mox-alliance-e2e-svc").await;
    let hit = hosts.iter().find(|h| {
        h["ip"].as_str() == Some("127.0.0.1") && h["port"].as_i64() == Some(3100)
    });
    assert!(hit.is_some(), "注册后实例应可查询（当前 hosts={:?}）", hosts);
    let h = hit.unwrap();
    let meta = h.get("metadata").cloned().unwrap_or_default();
    assert_eq!(meta.get("protocol").and_then(|x| x.as_str()), Some("http"));
    assert_eq!(meta.get("domain").and_then(|x| x.as_str()), Some("alliance"));
    println!("[e2e-1 PASS] NamingService 真实注册成功，实例 {}:{} 可发现，metadata 命中", h["ip"], h["port"]);

    // 清理：注销（不阻塞断言）
    reg.deregister().await;
}

/// e2e-2：deregister 真实注销 → 实例从列表移除
#[tokio::test]
#[ignore = "需本机运行 Nacos 服务端"]
async fn e2e_deregister_instance_real_nacos() {
    let reg = NamingRegistry::connect(&nacos(), &naming())
        .await
        .expect("connect 失败：请确认 rnacos 已运行")
        .expect("应返回 Some");

    reg.register().await;
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    let before = list_instances("mox-alliance-e2e-svc").await;
    assert!(
        before.iter().any(|h| h["ip"].as_str() == Some("127.0.0.1") && h["port"].as_i64() == Some(3100)),
        "注销前实例应存在"
    );

    reg.deregister().await;
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    let after = list_instances("mox-alliance-e2e-svc").await;
    assert!(
        !after.iter().any(|h| h["ip"].as_str() == Some("127.0.0.1") && h["port"].as_i64() == Some(3100)),
        "注销后实例应移除（当前 hosts={:?}）",
        after
    );
    println!("[e2e-2 PASS] NamingService 真实注销成功，实例已从注册中心移除");
}
