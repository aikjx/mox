// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 缺口 P1.3 —— AuditChain 跨版本连续性测试
//!
//! 目的：验证不可篡改审计哈希链在多「版本」事件交错追加时仍保持：
//!  1) 任意时刻 `verify()` 为真（完整性守恒）；
//!  2) 哈希指针跨版本连续（每事件的 `prev_hash` == 上一事件的 `hash`，
//!     与 flow_id/版本 无关，形成单一不可断裂的链）；
//!  3) 篡改**任一版本**的历史事件都会被 `verify()` 捕获（防篡改无版本盲区）；
//!  4) 空链 `latest_hash() == None`，首事件 `prev_hash == "GENESIS"`。

use mox_ai_expert_svc::govern::AuditChain;

/// 模拟一次「流程图跨版本演进」的审计序列：
/// v1 起草 → v1 审批 → v2 修订 → v2 复核 → v3 部署
/// 不同版本用不同 flow_id 区分，但落入同一条链。
fn build_versioned_chain() -> AuditChain {
    let mut c = AuditChain::new();
    c.append("alice", "flow:v1", "draft", "ok");
    c.append("bob", "flow:v1", "approve", "ok");
    c.append("alice", "flow:v2", "revise", "ok");
    c.append("carol", "flow:v2", "review", "ok");
    c.append("dave", "flow:v3", "deploy", "ok");
    c
}

#[test]
fn cross_version_chain_consistent_after_each_append() {
    let mut c = AuditChain::new();
    let mut prev_hash = "GENESIS".to_string();
    let steps = [
        ("alice", "flow:v1", "draft", "ok"),
        ("bob", "flow:v1", "approve", "ok"),
        ("alice", "flow:v2", "revise", "ok"),
        ("carol", "flow:v2", "review", "ok"),
        ("dave", "flow:v3", "deploy", "ok"),
    ];
    for (subject, flow, action, decision) in steps {
        let ev = c.append(subject, flow, action, decision);
        // 每步追加后整体仍完整
        assert!(c.verify(), "追加 {}/{} 后链应仍完整", flow, action);
        // 哈希指针连续：本事件 prev_hash == 上一步哈希
        assert_eq!(
            ev.prev_hash, prev_hash,
            "事件 {}/{} 的 prev_hash 应等于上一事件哈希",
            flow, action
        );
        prev_hash = ev.hash.clone();
    }
    // 最终 latest_hash 等于链尾事件哈希
    assert_eq!(
        c.latest_hash().as_deref(),
        c.events.last().map(|e| e.hash.as_str())
    );
    assert_eq!(c.events.len(), 5);
}

#[test]
fn cross_version_tamper_earliest_version_detected() {
    let mut c = build_versioned_chain();
    assert!(c.verify());
    // 篡改 v1 的 draft 事件（链上第 0 个）
    c.events[0].action = "hacked".into();
    c.events[0].decision = "malicious".into();
    assert!(!c.verify(), "篡改最早版本(v1)事件必须被检测");
}

#[test]
fn cross_version_tamper_middle_version_detected() {
    let mut c = build_versioned_chain();
    assert!(c.verify());
    // 篡改 v2 的 review 事件（链上第 3 个）
    c.events[3].action = "tampered".into();
    assert!(!c.verify(), "篡改中间版本(v2)事件必须被检测");
}

#[test]
fn cross_version_hash_chain_continuous_across_versions() {
    let c = build_versioned_chain();
    // 不变量：events[i].hash == events[i+1].prev_hash，对全部相邻对成立（跨版本无关）
    for w in c.events.windows(2) {
        assert_eq!(
            w[1].prev_hash, w[0].hash,
            "相邻事件哈希指针必须连续（跨版本）"
        );
    }
    // 首事件锚定 GENESIS
    assert_eq!(c.events[0].prev_hash, "GENESIS");
}

#[test]
fn empty_chain_has_no_latest_hash_and_genesis_anchor() {
    let mut c = AuditChain::new();
    assert!(c.verify(), "空链应视为完整");
    assert!(c.latest_hash().is_none(), "空链 latest_hash 应为 None");
    let ev = c.append("alice", "flow:v1", "draft", "ok");
    assert_eq!(ev.prev_hash, "GENESIS", "首事件 prev_hash 必须锚定 GENESIS");
    assert!(c.latest_hash().is_some());
}

#[test]
fn tampered_hash_field_breaks_later_continuity() {
    let mut c = build_versioned_chain();
    // 仅篡改某事件的 hash 字段（不改内容），其后所有 prev_hash 失配
    c.events[1].hash = "deadbeef".into();
    assert!(!c.verify(), "篡改历史事件哈希字段必须破坏后续连续性");
}
