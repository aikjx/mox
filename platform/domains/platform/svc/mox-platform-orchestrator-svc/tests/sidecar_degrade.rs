// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! TR-6.3: sidecar 不可用 → 可诊断错误 + sidecar_fail 指标增加
//!
//! 覆盖：NodeSidecarClient 的 fallback 路径、错误结构、metrics 计数。
//! 启动 Node 侧 sidecar 端口：我们故意指向未监听的 127.0.0.1:1（必然失败）。

use mox_platform_orchestrator_svc::sidecar::node_sidecar::{IntentReq, NodeSidecarClient, SidecarError};

#[tokio::test]
async fn sidecar_down_produces_diagnostic_unavailable_and_sidecar_fail_counter() {
    // 故意设置 1ms 超时 + 监听端口 1 → 必然 Unreachable
    let client = NodeSidecarClient::new("http://127.0.0.1:1")
        .with_timeout(1)
        .with_fallback(false);
    let before = client.metrics.snapshot();
    let err = client
        .intent(IntentReq {
            query: "列出 Project 节点".to_string(),
            context: Default::default(),
        })
        .await
        .unwrap_err();
    let after = client.metrics.snapshot();
    assert_eq!(after.calls, before.calls + 1);
    assert_eq!(after.fail, before.fail + 1);
    assert_eq!(after.success, before.success);
    match err {
        SidecarError::Unavailable { base, msg } => {
            assert!(base.contains("127.0.0.1:1"), "诊断应保留 base URL：{base}");
            assert!(!msg.is_empty(), "诊断消息非空，便于排障：{msg}");
        }
        other => panic!("预期 SidecarError::Unavailable，实际 {other:?}"),
    }
}

#[tokio::test]
async fn sidecar_down_fallback_returns_ok_and_bumps_fallback_used() {
    let client = NodeSidecarClient::new("http://127.0.0.1:1")
        .with_timeout(1)
        .with_fallback(true);
    let before = client.metrics.snapshot();
    let resp = client
        .intent(IntentReq {
            query: "查询所有文件".to_string(),
            context: Default::default(),
        })
        .await
        .expect("fallback=true 应返回 Ok(IntentResp)");
    assert!(resp.ok);
    assert_eq!(resp.intent, "file_search"); // 关键词"文件"命中
    let after = client.metrics.snapshot();
    assert_eq!(after.calls, before.calls + 1);
    assert_eq!(after.fail, before.fail + 1);
    assert_eq!(after.fallback_used, before.fallback_used + 1);
}
