//! 企业级 Rust 冒烟测试（11 步断言，对齐 Node smoke-enterprise.js）
//!
//! 流程：AppState(:memory:, ["common","finance"]) → 定义实体 → 种子 IAM →
//!       CRUD → 版本 → 指标 → 审计链 → 删除验证

use std::collections::BTreeMap;
use std::sync::Arc;

use enterprise_svc_lib::app_state::AppState;
use mox_platform_meta_core::{EnumOption, FieldDef, FieldType};

async fn build_state() -> Arc<AppState> {
    Arc::new(
        AppState::open_memory_or_file(":memory:", &["common", "finance"])
            .await
            .expect("AppState init"),
    )
}

#[tokio::test]
async fn t01_app_state_start_ok() {
    // T1: 启动 OK：AppState 构建成功，4 个核心仓储非空指针 + 预置管道注册
    let s = build_state().await;
    assert!(Arc::strong_count(&s) >= 1);
    assert!(s.iam.find_tenant_by_code("T001").is_some(), "IAM 种子租户 T001 必须存在");
    let pipelines = s.orch.list_pipelines();
    assert!(!pipelines.is_empty(), "默认 pipeline 必须注册");
}

#[tokio::test]
async fn t02_define_entity_project_fields_3() {
    // T2: define_entity project(title str required, amount decimal, status enum) → fields 3
    let s = build_state().await;
    let fields = vec![
        FieldDef {
            code: "title".to_string(),
            name: "项目标题".to_string(),
            r#type: FieldType::String,
            required: true,
            indexed: true,
            searchable: true,
            sortable: true,
            filterable: true,
            ui_component: Some("Input".to_string()),
            options_inline: None,
        },
        FieldDef {
            code: "amount".to_string(),
            name: "项目金额".to_string(),
            r#type: FieldType::Decimal,
            required: false,
            indexed: false,
            searchable: false,
            sortable: true,
            filterable: true,
            ui_component: Some("InputNumber".to_string()),
            options_inline: None,
        },
        FieldDef {
            code: "status".to_string(),
            name: "项目状态".to_string(),
            r#type: FieldType::Enum,
            required: false,
            indexed: true,
            searchable: false,
            sortable: true,
            filterable: true,
            ui_component: Some("Select".to_string()),
            options_inline: Some(vec![
                EnumOption { value: "draft".to_string(), label: "草稿".to_string(), color: Some("#999".to_string()) },
                EnumOption { value: "doing".to_string(), label: "进行中".to_string(), color: Some("#1677ff".to_string()) },
                EnumOption { value: "done".to_string(), label: "已完成".to_string(), color: Some("#52c41a".to_string()) },
            ]),
        },
    ];

    let (entity_id, slot_map) = s
        .meta
        .define_entity(None, "project".to_string(), "项目".to_string(), fields)
        .expect("define_entity");

    assert!(!entity_id.is_empty(), "entity_id 必须非空");
    assert_eq!(slot_map.len(), 3, "slot_map 长度必须等于 3（3 个字段）");
    assert!(slot_map.contains_key("title"), "slot_map 必须包含 title");
    assert!(slot_map.contains_key("amount"), "slot_map 必须包含 amount");
    assert!(slot_map.contains_key("status"), "slot_map 必须包含 status");

    let ent = s.meta.get_entity("default", "project").expect("entity must exist").expect("entity not None");
    assert_eq!(ent.fields.len(), 3, "entity fields 数量必须 = 3");
}

#[tokio::test]
async fn t03_seed_iam_tenant_dept_user_role() {
    // T3: 种子租户 T001 + 部门 + 用户 + 角色
    let s = build_state().await;
    let tnt = s.iam.find_tenant_by_code("T001").expect("T001 租户");
    assert_eq!(tnt.tenant_code, "T001");

    // 部门存在（至少 1 个）
    let depts: Vec<_> = s.iam.list_departments(&tnt.tenant_id).expect("list departments");
    assert!(!depts.is_empty(), "至少 1 个部门（种子 D001）");

    // 用户 admin 存在
    let admin = s
        .iam
        .find_user_by_tenant_username(&tnt.tenant_id, "admin")
        .expect("admin 用户必须存在");
    assert_eq!(admin.username, "admin");
    assert_eq!(admin.real_name.as_deref(), Some("系统管理员"));

    // 角色绑定：admin 具备 admin 角色
    let roles = s.iam.user_roles(&admin.user_id);
    assert!(!roles.is_empty(), "admin 必须至少有 1 个角色");
    let admin_role = roles.iter().find(|r| r.code == "tenant_admin");
    assert!(admin_role.is_some(), "admin 必须有 tenant_admin 角色");
}

#[tokio::test]
async fn t04_create_project_success() {
    // T4: POST /data/project/create {title,amount,status} → success
    let s = build_state().await;
    let fields = vec![
        FieldDef {
            code: "title".to_string(), name: "项目标题".to_string(),
            r#type: FieldType::String, required: true, indexed: true,
            searchable: true, sortable: true, filterable: true,
            ui_component: None, options_inline: None,
        },
        FieldDef {
            code: "amount".to_string(), name: "项目金额".to_string(),
            r#type: FieldType::Decimal, required: false, indexed: false,
            searchable: false, sortable: true, filterable: true,
            ui_component: None, options_inline: None,
        },
        FieldDef {
            code: "status".to_string(), name: "项目状态".to_string(),
            r#type: FieldType::Enum, required: false, indexed: true,
            searchable: false, sortable: true, filterable: true,
            ui_component: None,
            options_inline: Some(vec![
                EnumOption { value: "draft".to_string(), label: "草稿".to_string(), color: None },
                EnumOption { value: "doing".to_string(), label: "进行中".to_string(), color: None },
                EnumOption { value: "done".to_string(), label: "已完成".to_string(), color: None },
            ]),
        },
    ];
    s.meta.define_entity(None, "project".to_string(), "项目".to_string(), fields).unwrap();

    let mut data: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    data.insert("title".to_string(), serde_json::json!("XX产业园信息化建设"));
    data.insert("amount".to_string(), serde_json::json!(1234567.89));
    data.insert("status".to_string(), serde_json::json!("draft"));

    let rec = s
        .orch
        .create_sync("project", None, data, "tester")
        .expect("create project");
    assert!(!rec.biz_id.is_empty(), "biz_id 非空");
    assert_eq!(rec.version, 1, "初始版本 = 1");
    assert_eq!(rec.entity_code, "project");
    assert_eq!(
        rec.data.get("title").and_then(|v| v.as_str()),
        Some("XX产业园信息化建设"),
        "title 字段必须正确写入"
    );
}

#[tokio::test]
async fn t05_list_project_total_eq_1() {
    // T5: list total=1
    let s = build_state().await;
    seed_project_entity(&s);
    s.orch.create_sync("project", None, sample_data(), "tester").unwrap();

    let list = s.orch.list_sync("project", None).expect("list project");
    assert_eq!(list.len(), 1, "list total 必须 = 1");
}

#[tokio::test]
async fn t06_update_amount_and_status_version_up() {
    // T6: update 修改 amount=9999999.99 + status done → version up
    let s = build_state().await;
    seed_project_entity(&s);
    let rec = s.orch.create_sync("project", None, sample_data(), "tester").unwrap();
    let biz_id = rec.biz_id.clone();
    let old_version = rec.version;

    let mut patch: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    patch.insert("amount".to_string(), serde_json::json!(9999999.99));
    patch.insert("status".to_string(), serde_json::json!("done"));

    let updated = s
        .orch
        .update_sync(&biz_id, patch, "tester")
        .expect("update project");

    assert_eq!(updated.version, old_version + 1, "version 必须 +1");
    assert_eq!(
        updated.data.get("amount").and_then(|v| v.as_f64()),
        Some(9999999.99),
        "amount 必须更新为 9999999.99"
    );
    assert_eq!(
        updated.data.get("status").and_then(|v| v.as_str()),
        Some("done"),
        "status 必须更新为 done"
    );
}

#[tokio::test]
async fn t07_get_title_and_new_amount_status_label() {
    // T7: get → title 与新 amount/status label
    let s = build_state().await;
    seed_project_entity(&s);
    let rec = s.orch.create_sync("project", None, sample_data(), "tester").unwrap();
    let biz_id = rec.biz_id.clone();

    let mut patch: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    patch.insert("amount".to_string(), serde_json::json!(9999999.99));
    patch.insert("status".to_string(), serde_json::json!("done"));
    s.orch.update_sync(&biz_id, patch, "tester").unwrap();

    let got = s.orch.get_sync(&biz_id).expect("get").expect("must exist");
    assert_eq!(
        got.data.get("title").and_then(|v| v.as_str()),
        Some("XX产业园信息化建设"),
        "title 保持不变"
    );
    assert_eq!(
        got.data.get("amount").and_then(|v| v.as_f64()),
        Some(9999999.99),
        "amount 必须是更新值"
    );
    assert_eq!(
        got.data.get("status").and_then(|v| v.as_str()),
        Some("done"),
        "status 必须是更新值"
    );
}

#[tokio::test]
async fn t08_version_count_ge_2() {
    // T8: version count >= 2（create + update）
    let s = build_state().await;
    seed_project_entity(&s);
    let rec = s.orch.create_sync("project", None, sample_data(), "tester").unwrap();
    let biz_id = rec.biz_id.clone();

    let mut patch: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    patch.insert("amount".to_string(), serde_json::json!(9999999.99));
    s.orch.update_sync(&biz_id, patch, "tester").unwrap();

    let n = s.orch.version_count_sync(&biz_id);
    assert!(n >= 2, "version_count 必须 >= 2，实际 = {}", n);
}

#[tokio::test]
async fn t09_metrics_fail_rate_eq_0() {
    // T9: metrics failRate=0
    let s = build_state().await;
    seed_project_entity(&s);
    s.orch.create_sync("project", None, sample_data(), "tester").unwrap();
    let list = s.orch.list_sync("project", None).expect("list");
    assert!(!list.is_empty());

    let fr = s.orch.metrics.fail_rate();
    assert_eq!(fr, 0.0, "failRate 必须 = 0，实际 = {}", fr);
    let snap = s.orch.metrics.snapshot();
    assert_eq!(snap.fail_ops, 0, "fail_ops 必须 = 0");
    assert!(snap.total_ops >= snap.success_ops, "total >= success");
}

#[tokio::test]
async fn t10_audit_chain_continuous_3() {
    // T10: audit_chain 连续至少 3 条 prev->curr 非 null
    let s = build_state().await;
    seed_project_entity(&s);
    let rec = s.orch.create_sync("project", None, sample_data(), "tester").unwrap();
    let biz_id = rec.biz_id.clone();

    let mut p1: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    p1.insert("amount".to_string(), serde_json::json!(2222222.22));
    s.orch.update_sync(&biz_id, p1, "tester").unwrap();

    let mut p2: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    p2.insert("status".to_string(), serde_json::json!("doing"));
    s.orch.update_sync(&biz_id, p2, "tester").unwrap();

    let chain = s.orch.audit_chain_sync(&biz_id);
    assert!(chain.len() >= 3, "审计链至少 3 条，实际 = {}", chain.len());

    let mut prev_curr: Option<String> = None;
    for (i, node) in chain.iter().enumerate() {
        let prev_hash = node.get("prev_hash").and_then(|v| v.as_str()).filter(|s| !s.is_empty());
        let curr_hash = node.get("curr_hash").and_then(|v| v.as_str()).unwrap_or("");
        if i == 0 {
            // 链首可以无 prev_hash
        } else {
            assert!(
                prev_hash.is_some(),
                "第 {} 条 prev_hash 必须非空（实际 {:?}）",
                i,
                prev_hash
            );
            assert_eq!(
                prev_hash,
                prev_curr.as_deref(),
                "第 {} 条 prev_hash 必须等于前一条 curr_hash，prev={:?} expect={:?}",
                i,
                prev_hash,
                prev_curr
            );
        }
        assert!(!curr_hash.is_empty(), "curr_hash 必须非空（第 {} 条）", i);
        prev_curr = Some(curr_hash.to_string());
    }
}

#[tokio::test]
async fn t11_delete_and_list_total_eq_0() {
    // T11: delete + list total=0
    let s = build_state().await;
    seed_project_entity(&s);
    let rec = s.orch.create_sync("project", None, sample_data(), "tester").unwrap();
    let biz_id = rec.biz_id.clone();

    let before = s.orch.list_sync("project", None).unwrap();
    assert_eq!(before.len(), 1, "删除前 list=1");

    s.orch.delete_sync(&biz_id, "tester").expect("delete");

    let after = s.orch.list_sync("project", None).unwrap();
    assert_eq!(after.len(), 0, "删除后 list=0");

    let got = s.orch.get_sync(&biz_id).expect("get");
    assert!(got.is_none(), "get 必须返回 None（软删除）");
}

fn seed_project_entity(s: &Arc<AppState>) {
    let fields = vec![
        FieldDef {
            code: "title".to_string(), name: "项目标题".to_string(),
            r#type: FieldType::String, required: true, indexed: true,
            searchable: true, sortable: true, filterable: true,
            ui_component: None, options_inline: None,
        },
        FieldDef {
            code: "amount".to_string(), name: "项目金额".to_string(),
            r#type: FieldType::Decimal, required: false, indexed: false,
            searchable: false, sortable: true, filterable: true,
            ui_component: None, options_inline: None,
        },
        FieldDef {
            code: "status".to_string(), name: "项目状态".to_string(),
            r#type: FieldType::Enum, required: false, indexed: true,
            searchable: false, sortable: true, filterable: true,
            ui_component: None,
            options_inline: Some(vec![
                EnumOption { value: "draft".to_string(), label: "草稿".to_string(), color: None },
                EnumOption { value: "doing".to_string(), label: "进行中".to_string(), color: None },
                EnumOption { value: "done".to_string(), label: "已完成".to_string(), color: None },
            ]),
        },
    ];
    s.meta
        .define_entity(None, "project".to_string(), "项目".to_string(), fields)
        .unwrap();
}

fn sample_data() -> BTreeMap<String, serde_json::Value> {
    let mut d: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    d.insert("title".to_string(), serde_json::json!("XX产业园信息化建设"));
    d.insert("amount".to_string(), serde_json::json!(1234567.89));
    d.insert("status".to_string(), serde_json::json!("draft"));
    d
}
