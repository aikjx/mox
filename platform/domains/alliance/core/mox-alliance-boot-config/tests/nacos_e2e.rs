//! # Nacos 真实服务端 e2e（需要本机运行 Nacos 服务端）
//!
//! 前置：
//!   - 启动 rnacos（Rust Nacos 服务端）或 nacos-server 2.x：
//!     `platform/domains/alliance/tools/rnacos/rnacos.exe`（默认 127.0.0.1:8848 HTTP / 9848 gRPC）
//!   - 已通过 HTTP API 发布配置：
//!     `POST /nacos/v1/cs/configs` dataId=mox-alliance-scheduler.yml group=DEFAULT_GROUP content=<alliance-scheduler.yml>
//!
//! 运行（需 nacos feature）：
//!   `cargo test -p mox-alliance-boot-config --features nacos --test nacos_e2e -- --ignored --nocapture`
//!
//! 未运行 Nacos 时：connect 返回 Err（显式报错），本测试断言失败 —— 这是「禁止静默吞错」的预期行为。

use mox_alliance_boot_config::config_store::ConfigStore;
use mox_alliance_boot_config::nacos_config::NacosConfigStore;
use mox_alliance_boot_config::NacosSection;

fn section() -> NacosSection {
    NacosSection {
        enabled: true,
        server_addr: "127.0.0.1:8848".to_string(),
        namespace: String::new(),
        username: String::new(),
        password: String::new(),
        group: "DEFAULT_GROUP".to_string(),
        data_id: "mox-alliance-scheduler.yml".to_string(),
        ..Default::default()
    }
}

/// e2e-1：get_config 真实拉取 —— SDK 走 gRPC 协议向 rnacos 取回远程完整配置。
#[tokio::test]
#[ignore = "需本机运行 Nacos 服务端（rnacos 127.0.0.1:8848/9848）"]
async fn e2e_fetch_scheduler_config_from_real_nacos() {
    let store = NacosConfigStore::connect(&section())
        .await
        .expect("connect 失败：请确认 rnacos 已在 127.0.0.1:8848 运行并已发布 mox-alliance-scheduler.yml");
    let store = store.expect("enabled=true 且 dataId 非空，应返回 Some");

    let raw = store
        .load_raw("mox-alliance-scheduler.yml")
        .await
        .expect("load_raw 应成功")
        .expect("远程配置应有内容");
    assert!(
        raw.contains("port: 3100"),
        "远程配置应含 scheduler 端口 3100（实际前 400 字符：\n{})",
        &raw[..raw.len().min(400)]
    );
    assert!(raw.contains("nacos:"), "远程配置应含 nacos 段");
    assert!(raw.contains("expert_service:"), "远程配置应含 expert_service 段");
    println!(
        "[e2e-1 PASS] 真实 Nacos get_config 拉取 {} 字节，命中 port:3100 / nacos / expert_service",
        raw.len()
    );
}

/// e2e-2：watch 热更新 —— 修改远程配置后，changed() 应广播新内容。
///
/// 流程：连接并取初始内容 → 订阅 changed() → 通过 HTTP 发布新版本（追加标记行）→
/// 轮询等待 changed()/load_raw 收到变更 → 断言新内容命中标记。
#[tokio::test]
#[ignore = "需本机运行 Nacos 服务端"]
async fn e2e_watch_hot_update_from_real_nacos() {
    let store = NacosConfigStore::connect(&section())
        .await
        .expect("connect 失败：请确认 rnacos 已运行")
        .expect("应返回 Some");
    let initial = store
        .load_raw("mox-alliance-scheduler.yml")
        .await
        .unwrap()
        .expect("初始应有内容");

    // 发布标记版本（追加一行 e2e 标记，触发 watch 变更）
    let marker = "# e2e-watch-hot-update-marker";
    let updated = format!("{}\n{}", initial, marker);
    publish_config(&updated).await;

    let mut rx = store.changed().clone();
    let mut got: Option<String> = None;
    for _ in 0..20 {
        // changed() 是 watch::Receiver；borrow_and_update 返回 Ref<Option<String>>（直接最新值）
        let latest: Option<String> = rx.borrow_and_update().clone();
        if let Some(c) = latest {
            if c.contains(marker) {
                got = Some(c);
                break;
            }
        }
        // 主动重读缓存（add_listener 回调已更新 cache）
        if let Ok(Some(c)) = store.load_raw("mox-alliance-scheduler.yml").await {
            if c.contains(marker) {
                got = Some(c);
                break;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }

    let got = got.expect("20s 内未收到 Nacos watch 热更新（add_listener 未生效或服务端未推送）");
    assert!(got.contains(marker), "热更新内容应含标记");
    println!("[e2e-2 PASS] 真实 Nacos watch 热更新命中（add_listener 回调生效）");
}

/// 通过 rnacos HTTP 兼容 API 发布配置（Nacos 2.x 兼容；async，避免引入 reqwest blocking feature）
async fn publish_config(content: &str) {
    let resp = reqwest::Client::new()
        .post("http://127.0.0.1:8848/nacos/v1/cs/configs")
        .form(&[
            ("dataId", "mox-alliance-scheduler.yml"),
            ("group", "DEFAULT_GROUP"),
            ("content", content),
        ])
        .send()
        .await
        .expect("发布请求失败");
    assert!(
        resp.status().is_success(),
        "发布配置失败: {}",
        resp.status()
    );
}
