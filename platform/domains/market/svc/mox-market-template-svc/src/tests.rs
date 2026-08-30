// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use std::collections::BTreeMap;
use uuid::Uuid;

use crate::template_market::market::TemplateMarket;
use crate::template_market::types::{Domain, SystemTemplate};

fn market() -> TemplateMarket {
    let dir = std::env::temp_dir().join(format!("caomei_market_{}", Uuid::new_v4()));
    TemplateMarket::open(&dir).unwrap()
}

#[test]
fn publish_then_load_roundtrip() {
    let m = market();
    let tpl = SystemTemplate::new(
        "商城模板",
        "标准电商系统",
        vec![Domain::Mall],
        serde_json::json!({"name":"mall","nodes":["home","cart","pay"]}),
    );
    m.publish(&tpl).unwrap();
    let loaded = m.load(&tpl.id).unwrap();
    assert_eq!(loaded.name, "商城模板");
    assert_eq!(loaded.reuse_count, 1);
}

#[test]
fn fork_keeps_parent_link_and_artifacts() {
    let m = market();
    let parent = SystemTemplate::new(
        "父模板",
        "基础系统",
        vec![Domain::SystemDesign],
        serde_json::json!({"nodes":["a","b"]}),
    )
    .with_artifacts({
        let mut a = BTreeMap::new();
        a.insert("schema.sql".into(), "CREATE TABLE t();".into());
        a
    });
    m.publish(&parent).unwrap();

    let child = parent.fork("子模板-二开", "在父模板基础上扩展");
    assert_eq!(child.derived_from.as_deref(), Some(parent.id.as_str()));
    assert!(child.artifacts.contains_key("schema.sql"));
    m.publish(&child).unwrap();

    let list = m.list(Some(&Domain::SystemDesign), None).unwrap();
    assert_eq!(list.len(), 2);
}

#[test]
fn list_filter_by_domain_and_keyword() {
    let m = market();
    m.publish(&SystemTemplate::new(
        "小说平台",
        "网文创作",
        vec![Domain::Novel],
        serde_json::json!({}),
    ))
    .unwrap();
    m.publish(&SystemTemplate::new(
        "商城A",
        "电商",
        vec![Domain::Mall],
        serde_json::json!({}),
    ))
    .unwrap();
    m.publish(&SystemTemplate::new(
        "商城B",
        "零售电商",
        vec![Domain::Mall],
        serde_json::json!({}),
    ))
    .unwrap();

    let novels = m.list(Some(&Domain::Novel), None).unwrap();
    assert_eq!(novels.len(), 1);
    let malls = m.list(Some(&Domain::Mall), None).unwrap();
    assert_eq!(malls.len(), 2);
    let kw = m.list(None, Some("零售")).unwrap();
    assert_eq!(kw.len(), 1);
}

#[test]
fn ranking_reflects_reuse_and_rating() {
    let m = market();
    let a = SystemTemplate::new("A", "a", vec![Domain::Mall], serde_json::json!({}));
    let b = SystemTemplate::new("B", "b", vec![Domain::Mall], serde_json::json!({}));
    m.publish(&a).unwrap();
    m.publish(&b).unwrap();
    let _ = m.load(&a.id); // A 复用 +1
    m.rate(&a.id, 5.0).unwrap();
    let ranked = m.ranked(Some(&Domain::Mall)).unwrap();
    assert_eq!(ranked[0].id, a.id);
}

#[test]
fn remove_works() {
    let m = market();
    let t = SystemTemplate::new("临时", "x", vec![Domain::Book], serde_json::json!({}));
    m.publish(&t).unwrap();
    m.remove(&t.id).unwrap();
    assert!(m.load(&t.id).is_err());
}

#[test]
fn seed_mall_templates_produces_reusable_mall_seeds() {
    let m = market();
    let seeded = m.seed_mall_templates().unwrap();
    assert_eq!(seeded.len(), 2);
    assert!(seeded.iter().all(|t| t.domains.contains(&Domain::Mall)));

    // 商城种子携带 DDL 与前端骨架
    let mall = m.list(Some(&Domain::Mall), Some("标准电商")).unwrap();
    assert_eq!(mall.len(), 1);
    assert!(mall[0].artifacts.contains_key("generated/schema.sql"));
    assert!(mall[0].artifacts.contains_key("generated/App.vue"));

    // 会员种子包含会员/积分扩展表
    let member = m.list(Some(&Domain::Mall), Some("会员")).unwrap();
    assert_eq!(member.len(), 1);
    let ddl = member[0].artifacts.get("generated/schema.sql").unwrap();
    // DDL 使用 `CREATE TABLE IF NOT EXISTS` 风格，断言需匹配完整建表语句
    assert!(ddl.contains("CREATE TABLE IF NOT EXISTS member"));
    assert!(ddl.contains("CREATE TABLE IF NOT EXISTS point_log"));

    // 种子按热度排序应在前面
    let ranked = m.ranked(Some(&Domain::Mall)).unwrap();
    assert!(!ranked.is_empty());
    assert!(ranked[0].reuse_count >= 64);
}

#[test]
fn ensure_seeded_is_idempotent() {
    let m = market();
    assert_eq!(m.ensure_seeded().unwrap(), 2);
    // 第二次调用不应重复写入（市场非空）
    assert_eq!(m.ensure_seeded().unwrap(), 0);
}
