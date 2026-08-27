// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use parking_lot::Mutex;
use serde_json::{Map, Value};
use std::sync::Arc;
use uuid::Uuid;

use mox_platform_datastore_core::{
    compute_hash, FieldSlotAllocator, Filter, SortSpec, TxManager, UniversalBizDAO,
};
use mox_platform_datastore_core::{FieldSpec, InMemoryIamRepo, InMemoryMetaRepo};

fn setup() -> (
    Arc<Mutex<rusqlite::Connection>>,
    UniversalBizDAO,
    InMemoryMetaRepo,
    InMemoryIamRepo,
    String,
    String,
) {
    let conn = Arc::new(Mutex::new(rusqlite::Connection::open_in_memory().unwrap()));
    let dao = UniversalBizDAO::new(conn.clone());
    dao.init_schema().unwrap();

    let meta = InMemoryMetaRepo::new();
    let iam = InMemoryIamRepo::new();

    let tenant_id = Uuid::new_v4().to_string();
    let user_id = Uuid::new_v4().to_string();

    iam.init_standard_user(&tenant_id, &user_id);
    meta.init_common_industry(&tenant_id);

    (conn, dao, meta, iam, tenant_id, user_id)
}

#[test]
fn test_slot_allocator_basic() {
    let fields = vec![
        FieldSpec {
            field_code: "title".into(),
            field_type: "string".into(),
            is_required: true,
            is_indexed: true,
            is_searchable: true,
            is_sortable: true,
            is_filterable: true,
            options_inline: None,
        },
        FieldSpec {
            field_code: "amount".into(),
            field_type: "decimal".into(),
            is_required: false,
            is_indexed: false,
            is_searchable: false,
            is_sortable: true,
            is_filterable: true,
            options_inline: None,
        },
    ];
    let map = FieldSlotAllocator::allocate("project", &fields);
    let title = map.get("title").unwrap();
    assert!(title.slot_name.starts_with("ext_str_"));
    assert!(title.priority_score >= 32 + 16 + 8 + 4 + 2);

    let amount = map.get("amount").unwrap();
    assert!(amount.slot_name.starts_with("ext_dec_"));
}

#[test]
fn test_create_get_update_list_delete() {
    let (_conn, dao, meta, iam, tenant_id, user_id) = setup();

    // 1) CREATE
    let mut data = Map::new();
    data.insert("title".into(), Value::String("项目A".into()));
    data.insert("amount".into(), Value::from(10000.50));
    data.insert("status".into(), Value::String("draft".into()));

    let (biz_id, biz_code, version) = dao
        .create(
            &meta, &iam, &tenant_id, "project", &user_id, &data, None, None,
        )
        .expect("create success");

    assert!(!biz_id.is_empty(), "biz_id 非空");
    assert!(!biz_code.is_empty(), "biz_code 非空");
    assert_eq!(version, 1, "初始版本号=1");

    // 2) GET + verify hash 非空
    let get_result = dao
        .get(&meta, &tenant_id, "project", &biz_id)
        .expect("get success");
    assert!(get_result.is_some(), "get 应返回 Some");
    let val = get_result.unwrap();
    let obj = val.as_object().unwrap();
    assert_eq!(obj.get("title").unwrap().as_str(), Some("项目A"));
    assert_eq!(obj.get("version").unwrap().as_i64(), Some(1));
    let hash = obj.get("curr_hash").unwrap().as_str().unwrap();
    assert!(!hash.is_empty(), "curr_hash 非空");
    assert_eq!(hash.len(), 64, "SHA256 hex 长度=64");

    // 3) UPDATE → verify version=2
    let mut patch = Map::new();
    patch.insert("amount".into(), Value::from(20000.99));
    patch.insert("status".into(), Value::String("active".into()));

    let new_version = dao
        .update(&meta, &tenant_id, "project", &biz_id, &user_id, &patch)
        .expect("update success");
    assert_eq!(new_version, 2, "更新后版本号=2");

    let get_after = dao
        .get(&meta, &tenant_id, "project", &biz_id)
        .unwrap()
        .unwrap();
    let obj_after = get_after.as_object().unwrap();
    assert_eq!(obj_after.get("version").unwrap().as_i64(), Some(2));

    // 4) LIST → total=1
    let list = dao
        .list(
            &meta,
            &tenant_id,
            "project",
            vec![],
            SortSpec::default(),
            1,
            10,
        )
        .expect("list success");
    assert_eq!(list.total, 1, "list total=1");
    assert_eq!(list.items.len(), 1, "items len=1");

    // 5) 带 filter 的 list
    let filters = vec![Filter {
        field_code: "status".into(),
        operator: "eq".into(),
        value: Value::String("active".into()),
    }];
    let list_filtered = dao
        .list(
            &meta,
            &tenant_id,
            "project",
            filters,
            SortSpec::default(),
            1,
            10,
        )
        .expect("filtered list success");
    assert_eq!(list_filtered.total, 1, "filtered total=1");

    // 6) DELETE (软删)
    dao.delete(
        &tenant_id,
        "project",
        &biz_id,
        &user_id,
        Some("test delete"),
    )
    .expect("delete success");

    // 7) LIST after delete → total=0
    let list_after_del = dao
        .list(
            &meta,
            &tenant_id,
            "project",
            vec![],
            SortSpec::default(),
            1,
            10,
        )
        .expect("list after delete");
    assert_eq!(list_after_del.total, 0, "软删后 list total=0");

    // 8) compute_hash 可重现
    let h2 = compute_hash(
        Some("abc"),
        "biz-001",
        3,
        &serde_json::json!({"k":"v"}),
        "u1",
        "2024-01-01T00:00:00Z",
    );
    let h3 = compute_hash(
        Some("abc"),
        "biz-001",
        3,
        &serde_json::json!({"k":"v"}),
        "u1",
        "2024-01-01T00:00:00Z",
    );
    assert_eq!(h2, h3, "相同输入 hash 保持一致");
    assert_eq!(h2.len(), 64);
}

#[test]
fn test_nested_transaction_with_savepoint() {
    let conn = Arc::new(Mutex::new(rusqlite::Connection::open_in_memory().unwrap()));
    let dao = UniversalBizDAO::new(conn.clone());
    dao.init_schema().unwrap();
    let tx = TxManager::new(conn.clone());

    let meta = InMemoryMetaRepo::new();
    let iam = InMemoryIamRepo::new();
    let tenant_id = Uuid::new_v4().to_string();
    let user_id = Uuid::new_v4().to_string();
    iam.init_standard_user(&tenant_id, &user_id);
    meta.init_common_industry(&tenant_id);

    let result: anyhow::Result<(String, String, i64)> = tx.run(|| {
        let mut data = Map::new();
        data.insert("title".into(), Value::String("tx-test".into()));
        data.insert("amount".into(), Value::from(100.0));
        dao.create(
            &meta, &iam, &tenant_id, "project", &user_id, &data, None, None,
        )
    });
    let (bid, _, _) = result.unwrap();

    let inner_rollback = tx.run(|| {
        let mut p = Map::new();
        p.insert("amount".into(), Value::from(999.0));
        let v = dao.update(&meta, &tenant_id, "project", &bid, &user_id, &p)?;
        assert_eq!(v, 2);
        anyhow::Result::<()>::Err(anyhow::anyhow!("force rollback"))
    });
    assert!(inner_rollback.is_err(), "内层事务失败");

    let got = dao
        .get(&meta, &tenant_id, "project", &bid)
        .unwrap()
        .unwrap();
    // 外层未提交或回滚,内层 SAVEPOINT 回滚后 version 应仍为 1
    assert_eq!(
        got.as_object().unwrap().get("version").unwrap().as_i64(),
        Some(1)
    );
}
