// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use parking_lot::Mutex;
use serde_json::{Map, Value};
use std::sync::Arc;
use uuid::Uuid;

use mox_platform_datastore_core::{
    InMemoryIamRepo, InMemoryMetaRepo, TxManager, UniversalBizDAO, User,
};
use mox_platform_orchestrator_core::{BizAction, BusinessRequest, Orchestrator};

fn setup() -> (
    Arc<Mutex<rusqlite::Connection>>,
    Arc<UniversalBizDAO>,
    Arc<TxManager>,
    Arc<InMemoryMetaRepo>,
    Arc<InMemoryIamRepo>,
    Arc<Orchestrator>,
    String,
    String,
) {
    let conn = Arc::new(Mutex::new(rusqlite::Connection::open_in_memory().unwrap()));
    let dao = Arc::new(UniversalBizDAO::new(conn.clone()));
    dao.init_schema().unwrap();
    let tx = Arc::new(TxManager::new(conn.clone()));
    let meta = Arc::new(InMemoryMetaRepo::new());
    let iam = Arc::new(InMemoryIamRepo::new());
    let orc = Arc::new(Orchestrator::enterprise_default());

    let tenant_id = Uuid::new_v4().to_string();
    let user_id = Uuid::new_v4().to_string();
    iam.init_standard_user(&tenant_id, &user_id);
    meta.init_common_industry(&tenant_id);

    (conn, dao, tx, meta, iam, orc, tenant_id, user_id)
}

#[test]
fn test_orchestrator_create_get_update_list_delete() {
    let (_conn, dao, tx, meta, iam, orc, tenant_id, user_id) = setup();

    // 1. CREATE
    let mut data = Map::new();
    data.insert("title".into(), Value::String("Alpha 项目".into()));
    data.insert("amount".into(), Value::from(50000.00));
    data.insert("status".into(), Value::String("draft".into()));

    let create_req = BusinessRequest {
        tenant_id: tenant_id.clone(),
        user_id: user_id.clone(),
        entity_code: "project".into(),
        action: BizAction::Create,
        biz_id: None,
        biz_code: None,
        workflow_instance_id: None,
        data: Some(data),
        filters: vec![],
        sort: Default::default(),
        page: 1,
        page_size: 10,
    };
    let create_res = orc.execute(
        &create_req,
        dao.as_ref(),
        Some(tx.as_ref()),
        meta.as_ref(),
        iam.as_ref(),
    );

    assert!(create_res.success, "Create 成功: {:?}", create_res.error);
    let biz_id = create_res.biz_id.clone().expect("biz_id 非空");
    assert!(!biz_id.is_empty(), "biz_id 非空");
    assert_eq!(create_res.version, Some(1), "Create version=1");

    // 2. GET
    let get_req = BusinessRequest {
        tenant_id: tenant_id.clone(),
        user_id: user_id.clone(),
        entity_code: "project".into(),
        action: BizAction::Get,
        biz_id: Some(biz_id.clone()),
        ..Default::default()
    };
    let get_res = orc.execute(&get_req, dao.as_ref(), None, meta.as_ref(), iam.as_ref());
    assert!(get_res.success, "Get 成功");
    let obj = get_res.data.unwrap();
    let obj_map = obj.as_object().unwrap();
    assert_eq!(obj_map.get("title").unwrap().as_str(), Some("Alpha 项目"));
    // enrich 字典翻译 status_label
    assert!(
        obj_map.contains_key("status_label"),
        "存在 status_label 字典翻译"
    );
    assert_eq!(obj_map.get("status_label").unwrap().as_str(), Some("草稿"));

    // 3. UPDATE amount/status, expect version=2
    let mut patch = Map::new();
    patch.insert("amount".into(), Value::from(66666.66));
    patch.insert("status".into(), Value::String("active".into()));

    let update_req = BusinessRequest {
        tenant_id: tenant_id.clone(),
        user_id: user_id.clone(),
        entity_code: "project".into(),
        action: BizAction::Update,
        biz_id: Some(biz_id.clone()),
        data: Some(patch),
        ..Default::default()
    };
    let update_res = orc.execute(
        &update_req,
        dao.as_ref(),
        Some(tx.as_ref()),
        meta.as_ref(),
        iam.as_ref(),
    );
    assert!(update_res.success, "Update 成功: {:?}", update_res.error);
    assert_eq!(update_res.version, Some(2), "Update version=2");

    // GET 再次验证 version=2 + active_label
    let get2 = orc.execute(&get_req, dao.as_ref(), None, meta.as_ref(), iam.as_ref());
    assert!(get2.success);
    let obj2 = get2.data.unwrap().as_object().unwrap().clone();
    assert_eq!(obj2.get("version").unwrap().as_i64(), Some(2));
    assert_eq!(obj2.get("status_label").unwrap().as_str(), Some("进行中"));

    // 4. LIST → total=1
    let list_req = BusinessRequest {
        tenant_id: tenant_id.clone(),
        user_id: user_id.clone(),
        entity_code: "project".into(),
        action: BizAction::List,
        page: 1,
        page_size: 10,
        ..Default::default()
    };
    let list_res = orc.execute(&list_req, dao.as_ref(), None, meta.as_ref(), iam.as_ref());
    assert!(list_res.success, "List 成功");
    assert_eq!(list_res.total, Some(1), "List total=1");
    let items = list_res.data.unwrap().as_array().unwrap().clone();
    assert_eq!(items.len(), 1);

    // 5. DELETE
    let del_req = BusinessRequest {
        tenant_id: tenant_id.clone(),
        user_id: user_id.clone(),
        entity_code: "project".into(),
        action: BizAction::Delete,
        biz_id: Some(biz_id.clone()),
        ..Default::default()
    };
    let del_res = orc.execute(
        &del_req,
        dao.as_ref(),
        Some(tx.as_ref()),
        meta.as_ref(),
        iam.as_ref(),
    );
    assert!(del_res.success, "Delete 成功");

    // 6. LIST after delete → total=0
    let list_after = orc.execute(&list_req, dao.as_ref(), None, meta.as_ref(), iam.as_ref());
    assert_eq!(list_after.total, Some(0), "软删后 List total=0");

    // 7. 指标: fail_rate = 0
    let metrics = &orc.metrics;
    assert_eq!(metrics.fail_rate(), 0.0, "fail_rate=0");
    assert!(metrics.total() >= 6, "累计调用 >= 6");
    assert!(metrics.p50().is_some(), "p50 可计算");

    // 8. 事件总线: 至少 publish 了 Create / Update / Delete
    let qlen = orc.event_bus.queue_len();
    assert!(qlen >= 3, "事件队列长度 >= 3 (created/updated/deleted)");

    // 9. pipeline stages 包含 10 个阶段（成功路径）
    let first_success_stages = &create_res.pipeline_stages;
    assert_eq!(first_success_stages.len(), 10, "成功路径跑满 10 阶段");

    // 10. 审计日志条目数
    let audit_count = iam.audit_logs.lock().unwrap().len();
    assert!(
        audit_count >= 6,
        "审计日志 >= 6 条（6次 orchestrator 调用）"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_orchestrator_async_execute_with_tokio() {
    let (_conn, dao, tx, meta, iam, orc, tenant_id, user_id) = setup();

    let mut data = Map::new();
    data.insert("title".into(), Value::String("AsyncProj".into()));
    data.insert("amount".into(), Value::from(42.0));
    data.insert("status".into(), Value::String("draft".into()));

    let req = BusinessRequest {
        tenant_id,
        user_id,
        entity_code: "project".into(),
        action: BizAction::Create,
        data: Some(data),
        page: 1,
        page_size: 10,
        ..Default::default()
    };
    let orch_clone = orc.clone();
    let dao_clone = dao.clone();
    let tx_clone = tx.clone();
    let meta_clone = meta.clone();
    let iam_clone = iam.clone();
    let r = tokio::task::spawn_blocking(move || {
        orch_clone.execute(
            &req,
            dao_clone.as_ref(),
            Some(tx_clone.as_ref()),
            meta_clone.as_ref(),
            iam_clone.as_ref(),
        )
    })
    .await
    .unwrap();
    assert!(r.success, "async create ok");
    assert!(r.biz_id.clone().unwrap().len() > 0);
}

#[test]
fn test_permission_denied_auth_stage() {
    let (_conn, dao, tx, meta, iam, orc, tenant_id, _user_id) = setup();

    let no_perm_user = Uuid::new_v4().to_string();
    iam.add_user(User {
        user_id: no_perm_user.clone(),
        tenant_id: tenant_id.clone(),
        username: "noperm".into(),
        dept_id: "dept-002".into(),
    });

    let req = BusinessRequest {
        tenant_id,
        user_id: no_perm_user,
        entity_code: "project".into(),
        action: BizAction::Create,
        data: Some({
            let mut m = Map::new();
            m.insert("title".into(), Value::String("x".into()));
            m.insert("amount".into(), Value::from(1.0));
            m
        }),
        ..Default::default()
    };
    let r = orc.execute(
        &req,
        dao.as_ref(),
        Some(tx.as_ref()),
        meta.as_ref(),
        iam.as_ref(),
    );
    assert!(!r.success, "无权限用户创建失败");
    let err = r.error.unwrap();
    assert!(
        err.contains("Permission denied"),
        "错误信息包含 Permission denied"
    );
}
