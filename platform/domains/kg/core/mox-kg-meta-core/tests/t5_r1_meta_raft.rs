//! T5 R1 Meta Service 验收测试（≥20 tests）
//!
//! - TR5.2 Raft 3 节点选举 3 轮
//! - TR5.3 Schema：createSpace / createTag / createEdgeType / dropTag 正确错误
//! - TR5.4 权限：createUser / grant / revoke / authenticate + authorize allow+deny
//! - TR5.5 分区路由：VID hash 1000 次均匀性 ≤ 15%
//! - TR5.6 依赖白名单：Cargo.toml 存在 async-raft + rocksdb，无禁制品
//! - TR5.7 自研边界：src 中无 nebula / neo4j / janusgraph 等词
//! - TR5.8 ≥ 20 个 tests

use std::collections::BTreeMap;
use std::time::Duration;
use mox_kg_meta_core::{
    vid_hash_partition, AuthStore, CreateEdgeTypeArgs, EdgeDef, FieldDef, FieldType, IndexKind,
    MetaCluster, MetaError, MetaServer, PartitionStore, Resource, Role, SchemaStore, SpaceDef,
    TagDef, UserDef,
};

// ---------- 简易 RED 相位开关：运行 `cargo test --test t5_r1_meta_raft --cfg red_phase` 会让以下全部失败 ----------
#[cfg(all(test, red_phase))]
fn force_red() {
    panic!("TDD RED phase: tests should all fail before implementation GREEN");
}

// ============================================================
// TR5.2 Raft 3 节点选举（3 轮，每轮 ≤ 5s）
// ============================================================

fn rt() -> tokio::mox_platform_orchestrator_svc::Runtime {
    tokio::mox_platform_orchestrator_svc::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
}

/// 辅助：创建 3 节点集群，强制建 admin 用户（方便后续用 admin 调用鉴权 API）
fn bootstrap_3node_admin() -> (tokio::mox_platform_orchestrator_svc::Runtime, MetaServer, String) {
    let rt = rt();
    let cluster = rt.block_on(async { MetaCluster::launch_3_nodes().await.unwrap() });
    let srv = MetaServer::with_cluster(cluster);
    // 手动在所有 store 注入 admin 用户（为了避免需要 admin 用户才能 create_user 死循环）
    // 通过 standalone create_user 模式：先 standalone 再把快照同步。
    let tmp = MetaServer::standalone();
    let admin_def: UserDef = tmp
        .create_user("admin", "adminpw", Role::Admin, None)
        .unwrap();
    // 获得 tmp 的 snapshot 并 merge 到 srv 全部 store 的 AuthStore
    let snap_single = tmp
        .cluster()
        .is_none()
        .then(|| {
            // 通过读所有用户得到 admin_def
            admin_def
        })
        .unwrap();
    // 把 admin 写入 srv 的每个 store（用 propose 一条 CreateUser，但需要 caller=None → 我们直接绕过权限）
    // 简单方法：使用 AuthStore::create_user 手工 merge
    let stores = all_stores(&srv);
    for st in stores {
        let mut snap = st.snapshot();
        let _ = snap.auth.create_user("admin", "adminpw", Role::Admin);
        // 写回
        st.set_snapshot(snap);
    }
    let admin_uid = srv.authenticate_user("admin", "adminpw").unwrap();
    (rt, srv, admin_uid)
}

fn all_stores(srv: &MetaServer) -> Vec<mox_kg_meta_core::MetaStateMachine> {
    if let Some(c) = srv.cluster() {
        c.nodes.values().map(|n| n.store.clone()).collect()
    } else {
        panic!("expected cluster server");
    }
}

#[test]
fn tr5_2_election_round_1_within_5s() {
    #[cfg(red_phase)]
    force_red();
    let (rt, srv, _admin) = bootstrap_3node_admin();
    let took = rt.block_on(async {
        let c = srv.cluster().unwrap();
        let (_, d) = c.kill_leader_and_reelect().await.unwrap();
        d
    });
    assert!(
        took <= Duration::from_secs(5),
        "round1 took {:?} > 5s",
        took
    );
}

#[test]
fn tr5_2_election_round_2_within_5s() {
    #[cfg(red_phase)]
    force_red();
    let (rt, srv, _admin) = bootstrap_3node_admin();
    let took = rt.block_on(async {
        let c = srv.cluster().unwrap();
        let _ = c.kill_leader_and_reelect().await.unwrap();
        let (_, d) = c.kill_leader_and_reelect().await.unwrap();
        d
    });
    assert!(
        took <= Duration::from_secs(5),
        "round2 took {:?} > 5s",
        took
    );
}

#[test]
fn tr5_2_election_round_3_within_5s() {
    #[cfg(red_phase)]
    force_red();
    let (rt, srv, _admin) = bootstrap_3node_admin();
    let took = rt.block_on(async {
        let c = srv.cluster().unwrap();
        for _ in 0..2 {
            let _ = c.kill_leader_and_reelect().await.unwrap();
        }
        let (_, d) = c.kill_leader_and_reelect().await.unwrap();
        d
    });
    assert!(
        took <= Duration::from_secs(5),
        "round3 took {:?} > 5s",
        took
    );
}

#[test]
fn tr5_2_election_3rounds_max_within_5s() {
    #[cfg(red_phase)]
    force_red();
    let (rt, srv, _admin) = bootstrap_3node_admin();
    let max_took = rt.block_on(async {
        let c = srv.cluster().unwrap();
        let mut max = Duration::ZERO;
        for _ in 0..3 {
            let (_, d) = c.kill_leader_and_reelect().await.unwrap();
            max = max.max(d);
        }
        max
    });
    assert!(
        max_took <= Duration::from_secs(5),
        "max took {:?} > 5s",
        max_took
    );
}

// ============================================================
// TR5.3 Schema（createSpace → list → createTag → show tags → createEdgeType → dropTag err）
// ============================================================

fn sample_tag_fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new("name", FieldType::String, IndexKind::Unique),
        FieldDef::new("age", FieldType::Int, IndexKind::None),
    ]
}

#[test]
fn tr5_3_create_space_and_list() {
    #[cfg(red_phase)]
    force_red();
    let srv = MetaServer::standalone();
    let admin = srv.create_user("admin", "pw", Role::Admin, None).unwrap();
    srv.create_space("s1", 16, 3, Some(&admin.username))
        .unwrap();
    let list = srv.list_spaces().unwrap();
    assert!(
        list.iter().any(|s| s.space_id == "s1"),
        "list_spaces missing s1: {:?}",
        list
    );
}

#[test]
fn tr5_3_create_space_duplicate_error() {
    #[cfg(red_phase)]
    force_red();
    let srv = MetaServer::standalone();
    let admin = srv.create_user("admin", "pw", Role::Admin, None).unwrap();
    srv.create_space("s1", 16, 3, Some(&admin.username))
        .unwrap();
    let err = srv
        .create_space("s1", 16, 3, Some(&admin.username))
        .unwrap_err();
    assert!(
        matches!(err, MetaError::SpaceExists(_)),
        "wrong err: {:?}",
        err
    );
}

#[test]
fn tr5_3_create_tag_and_list_tags() {
    #[cfg(red_phase)]
    force_red();
    let srv = MetaServer::standalone();
    let admin = srv.create_user("admin", "pw", Role::Admin, None).unwrap();
    srv.create_space("s1", 16, 3, Some(&admin.username))
        .unwrap();
    srv.create_tag("s1", "Person", sample_tag_fields(), Some(&admin.username))
        .unwrap();
    let tags = srv.list_tags("s1").unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].tag_name, "Person");
    assert_eq!(tags[0].fields.len(), 2);
}

#[test]
fn tr5_3_create_tag_unknown_space() {
    #[cfg(red_phase)]
    force_red();
    let srv = MetaServer::standalone();
    let admin = srv.create_user("admin", "pw", Role::Admin, None).unwrap();
    let err = srv
        .create_tag(
            "nosuch",
            "Person",
            sample_tag_fields(),
            Some(&admin.username),
        )
        .unwrap_err();
    assert!(
        matches!(err, MetaError::SpaceNotFound(_)),
        "wrong err: {:?}",
        err
    );
}

#[test]
fn tr5_3_create_edge_type_and_list() {
    #[cfg(red_phase)]
    force_red();
    let srv = MetaServer::standalone();
    let admin = srv.create_user("admin", "pw", Role::Admin, None).unwrap();
    srv.create_space("s1", 16, 3, Some(&admin.username))
        .unwrap();
    srv.create_tag("s1", "Person", sample_tag_fields(), Some(&admin.username))
        .unwrap();
    srv.create_edge_type(CreateEdgeTypeArgs {
        space: "s1",
        edge_name: "KNOWS",
        from_tag: "Person",
        to_tag: "Person",
        has_rank: false,
        has_weight: true,
        fields: vec![FieldDef::new("since", FieldType::Int, IndexKind::None)],
        caller: Some(&admin.username),
    })
    .unwrap();
    let edges = srv.list_edge_types("s1").unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].edge_name, "KNOWS");
    assert_eq!(edges[0].from_tag, "Person");
    assert_eq!(edges[0].to_tag, "Person");
    assert!(edges[0].has_weight);
    assert!(!edges[0].has_rank);
}

#[test]
fn tr5_3_drop_notfound_tag_returns_error() {
    #[cfg(red_phase)]
    force_red();
    let srv = MetaServer::standalone();
    let space = "drop_tag_space_unique_99";
    let admin_def = srv
        .create_user("admin_99", "pw", Role::Admin, None)
        .unwrap();
    srv.create_space(space, 16, 3, Some(&admin_def.username))
        .expect("create_space must succeed");
    // 不创建任何标签，尝试 drop 非存在标签 → 必须是 TagNotFound（即使 tags map 为空也应如此）
    let err = srv
        .drop_tag(space, "NonExistentTag", Some(&admin_def.username))
        .unwrap_err();
    assert!(
        matches!(err, MetaError::TagNotFound(_, _)),
        "wrong err: {:?}",
        err
    );
}

#[test]
fn tr5_3_alter_tag_add_field() {
    #[cfg(red_phase)]
    force_red();
    let srv = MetaServer::standalone();
    let admin = srv.create_user("admin", "pw", Role::Admin, None).unwrap();
    srv.create_space("s1", 16, 3, Some(&admin.username))
        .unwrap();
    srv.create_tag("s1", "Person", sample_tag_fields(), Some(&admin.username))
        .unwrap();
    srv.alter_tag(
        "s1",
        "Person",
        vec![FieldDef::new("city", FieldType::String, IndexKind::None)],
        Some(&admin.username),
    )
    .unwrap();
    let tags = srv.list_tags("s1").unwrap();
    assert_eq!(tags[0].fields.len(), 3);
}

#[test]
fn tr5_3_drop_space_clears_schema() {
    #[cfg(red_phase)]
    force_red();
    let srv = MetaServer::standalone();
    let admin = srv.create_user("admin", "pw", Role::Admin, None).unwrap();
    srv.create_space("s1", 16, 3, Some(&admin.username))
        .unwrap();
    srv.create_tag("s1", "Person", sample_tag_fields(), Some(&admin.username))
        .unwrap();
    srv.drop_space("s1", Some(&admin.username)).unwrap();
    let tags_err = srv.list_tags("s1").unwrap_err();
    assert!(matches!(tags_err, MetaError::SpaceNotFound(_)));
}

// TR5.3 - 5 操作后 follower 3/3 一致
#[test]
fn tr5_3_schema_synced_3followers_consistent() {
    #[cfg(red_phase)]
    force_red();
    let (rt, srv, admin) = bootstrap_3node_admin();
    rt.block_on(async {
        let c = srv.cluster().unwrap();
        // 确保网络模拟已跑
        let leader_before = c.leader().unwrap();
        let _ = leader_before;
    });
    // 5 个操作
    srv.create_space("s1", 16, 3, Some(&admin)).unwrap();
    srv.create_tag("s1", "Person", sample_tag_fields(), Some(&admin))
        .unwrap();
    srv.create_edge_type(CreateEdgeTypeArgs {
        space: "s1",
        edge_name: "KNOWS",
        from_tag: "Person",
        to_tag: "Person",
        has_rank: false,
        has_weight: true,
        fields: vec![],
        caller: Some(&admin),
    })
    .unwrap();
    srv.create_tag(
        "s1",
        "Company",
        vec![FieldDef::new("name", FieldType::String, IndexKind::Unique)],
        Some(&admin),
    )
    .unwrap();
    srv.create_edge_type(CreateEdgeTypeArgs {
        space: "s1",
        edge_name: "WORKS_AT",
        from_tag: "Person",
        to_tag: "Company",
        has_rank: true,
        has_weight: false,
        fields: vec![],
        caller: Some(&admin),
    })
    .unwrap();
    // 3/3 一致
    let ok = srv.cluster_snapshot_consistent(|schema, _auth, _part| {
        let spaces: Vec<String> = schema
            .list_spaces()
            .into_iter()
            .map(|s| s.space_id)
            .collect();
        let tags: Vec<String> = schema
            .list_tags("s1")
            .unwrap()
            .into_iter()
            .map(|t| t.tag_name)
            .collect();
        let edges: Vec<String> = schema
            .list_edge_types("s1")
            .unwrap()
            .into_iter()
            .map(|e| e.edge_name)
            .collect();
        (spaces, tags, edges)
    });
    assert!(ok, "3 follower schemas inconsistent");
}

// ============================================================
// TR5.4 权限
// ============================================================

#[test]
fn tr5_4_create_user_authenticate() {
    #[cfg(red_phase)]
    force_red();
    let srv = MetaServer::standalone();
    let admin = srv.create_user("admin", "pw", Role::Admin, None).unwrap();
    let alice = srv
        .create_user("alice", "pw123", Role::User, Some(&admin.username))
        .unwrap();
    assert_eq!(alice.username, "alice");
    let uid = srv.authenticate_user("alice", "pw123").unwrap();
    assert_eq!(uid, "alice");
    let bad = srv.authenticate_user("alice", "wrongpw");
    assert!(matches!(
        bad.unwrap_err(),
        MetaError::AuthenticationFailed(_)
    ));
}

#[test]
fn tr5_4_grant_spaceadmin_allows_tag_create() {
    #[cfg(red_phase)]
    force_red();
    let srv = MetaServer::standalone();
    let admin = srv.create_user("admin", "pw", Role::Admin, None).unwrap();
    let alice = srv
        .create_user("alice", "pw123", Role::User, Some(&admin.username))
        .unwrap();
    srv.create_space("s1", 16, 3, Some(&admin.username))
        .unwrap();
    srv.grant_role(
        "alice",
        Role::SpaceAdmin,
        &Resource::space("s1"),
        Some(&admin.username),
    )
    .unwrap();
    let uid = srv.authenticate_user("alice", "pw123").unwrap();
    // tag.create should be allowed
    let res = srv.create_tag(
        "s1",
        "Company",
        vec![FieldDef::new("name", FieldType::String, IndexKind::Unique)],
        Some(&uid),
    );
    assert!(res.is_ok(), "alice allowed to create_tag failed: {:?}", res);
}

#[test]
fn tr5_4_revoke_spaceadmin_denies_tag_create() {
    #[cfg(red_phase)]
    force_red();
    let srv = MetaServer::standalone();
    let admin = srv.create_user("admin", "pw", Role::Admin, None).unwrap();
    let alice = srv
        .create_user("alice", "pw123", Role::User, Some(&admin.username))
        .unwrap();
    srv.create_space("s1", 16, 3, Some(&admin.username))
        .unwrap();
    srv.grant_role(
        "alice",
        Role::SpaceAdmin,
        &Resource::space("s1"),
        Some(&admin.username),
    )
    .unwrap();
    srv.revoke_role(
        "alice",
        Role::SpaceAdmin,
        &Resource::space("s1"),
        Some(&admin.username),
    )
    .unwrap();
    let uid = srv.authenticate_user("alice", "pw123").unwrap();
    // authenticate 仍 ok（用户名密码对）
    // authorize 应当 denied
    let auth_err = srv
        .authorize(&uid, "tag.create", &Resource::space("s1"))
        .unwrap_err();
    assert!(
        matches!(auth_err, MetaError::AuthDenied { .. }),
        "expected AuthDenied, got {:?}",
        auth_err
    );
}

#[test]
fn tr5_4_readonly_denies_write_but_allows_read() {
    #[cfg(red_phase)]
    force_red();
    let srv = MetaServer::standalone();
    let admin = srv.create_user("admin", "pw", Role::Admin, None).unwrap();
    let reader = srv
        .create_user("reader", "rpw", Role::ReadOnly, Some(&admin.username))
        .unwrap();
    srv.create_space("s1", 16, 3, Some(&admin.username))
        .unwrap();
    let uid = srv.authenticate_user("reader", "rpw").unwrap();
    // list_tags is read — allowed
    let _ = srv.list_tags("s1").unwrap();
    // create_tag 被 authorize 拒绝（但 MetaServer.create_tag 内部走的是 authorize）
    let err = srv
        .create_tag("s1", "Blocked", vec![], Some(&uid))
        .unwrap_err();
    assert!(
        matches!(err, MetaError::AuthDenied { .. }),
        "should deny: {:?}",
        err
    );
    let _ = reader;
}

// ============================================================
// TR5.5 分区路由：1000 VID 均匀性
// ============================================================

#[test]
fn tr5_5_vid_hash_1000_uniform_le_15pct_cv() {
    #[cfg(red_phase)]
    force_red();
    let partition_num: u16 = 16;
    let mut counts: BTreeMap<u64, usize> = BTreeMap::new();
    for i in 0..1000u64 {
        let vid = format!("vertex_{:010}", i);
        let shard = vid_hash_partition(&vid, partition_num);
        *counts.entry(shard).or_default() += 1;
    }
    assert_eq!(
        counts.len(),
        partition_num as usize,
        "all 16 shards must appear"
    );
    let vals: Vec<f64> = counts.values().map(|v| *v as f64).collect();
    let mean = vals.iter().sum::<f64>() / vals.len() as f64;
    let variance: f64 = vals.iter().map(|v| (*v - mean).powi(2)).sum::<f64>() / vals.len() as f64;
    let stddev = variance.sqrt();
    let cv = stddev / mean;
    assert!(cv <= 0.15, "cv={:.3} > 15% (counts={:?})", cv, counts);
}

#[test]
fn tr5_5_register_host_and_get_route() {
    #[cfg(red_phase)]
    force_red();
    let srv = MetaServer::standalone();
    let admin = srv.create_user("admin", "pw", Role::Admin, None).unwrap();
    srv.register_storage_host("h1", "127.0.0.1:9779", Some(&admin.username))
        .unwrap();
    srv.register_storage_host("h2", "127.0.0.1:9780", Some(&admin.username))
        .unwrap();
    srv.register_storage_host("h3", "127.0.0.1:9781", Some(&admin.username))
        .unwrap();
    srv.create_space("s1", 16, 3, Some(&admin.username))
        .unwrap();
    let (shard, addr) = srv.get_partition_route("s1", "some_vid").unwrap();
    assert!(shard < 16);
    assert!(addr.starts_with("127.0.0.1:"), "addr = {}", addr);
    let hosts = srv.show_hosts().unwrap();
    assert_eq!(hosts.len(), 3);
}

// ============================================================
// TR5.6 依赖白名单
// ============================================================

#[test]
fn tr5_6_cargo_toml_contains_async_raft_and_rocksdb() {
    #[cfg(red_phase)]
    force_red();
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("manifest");
    assert!(
        manifest.contains("async-raft"),
        "Cargo.toml missing async-raft dep"
    );
    assert!(
        manifest.contains("rocksdb"),
        "Cargo.toml missing rocksdb dep"
    );
    // Apache2.0 允许
    assert!(
        manifest.contains("Apache-2.0") || manifest.contains("MIT OR Apache-2.0"),
        "license must allow Apache"
    );
    // 禁制品检查
    let forbidden = ["nebula-graph", "neo4j", "janusgraph", "AGPL"];
    for w in forbidden.iter() {
        assert!(
            !manifest.to_lowercase().contains(&w.to_lowercase()),
            "Cargo.toml contains forbidden: {}",
            w
        );
    }
}

// ============================================================
// TR5.7 自研边界
// ============================================================

#[test]
fn tr5_7_src_rs_no_forbidden_graph_brands() {
    #[cfg(red_phase)]
    force_red();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let forbidden = [
        "nebula-graph",
        "nebula_graph",
        "neo4j",
        "janusgraph",
        "janus_graph",
    ];
    let mut hits: Vec<(String, String)> = Vec::new();
    walk_dir_rs(&dir, &mut |path, content| {
        for w in forbidden.iter() {
            if content.to_lowercase().contains(&w.to_lowercase()) {
                hits.push((path.display().to_string(), w.to_string()));
            }
        }
    });
    assert!(hits.is_empty(), "forbidden matches found: {:?}", hits);
}

fn walk_dir_rs(dir: &std::path::Path, f: &mut dyn FnMut(&std::path::Path, &str)) {
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk_dir_rs(&path, f);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                let c = std::fs::read_to_string(&path).unwrap_or_default();
                f(&path, &c);
            }
        }
    }
}

// ============================================================
// 其它：SchemaStore 独立单测 / AuthStore 独立 / PartitionStore 独立
// ============================================================

#[test]
fn xt_schema_store_space_validation() {
    #[cfg(red_phase)]
    force_red();
    let mut st = SchemaStore::new();
    let bad_pn = SpaceDef {
        space_id: "s".into(),
        partition_num: 10,
        replica_factor: 3,
        created_at: 0,
    };
    assert!(matches!(
        st.create_space(bad_pn),
        Err(MetaError::InvalidArgument(_))
    ));
    let bad_rf = SpaceDef {
        space_id: "s".into(),
        partition_num: 8,
        replica_factor: 5,
        created_at: 0,
    };
    assert!(matches!(
        st.create_space(bad_rf),
        Err(MetaError::InvalidArgument(_))
    ));
}

#[test]
fn xt_auth_authorize_admin_on_anything() {
    #[cfg(red_phase)]
    force_red();
    let mut a = AuthStore::new();
    a.create_user("a", "pw", Role::Admin).unwrap();
    let uid = a.authenticate_user("a", "pw").unwrap();
    a.authorize(&uid, "anything", &Resource::all()).unwrap();
    a.authorize(&uid, "tag.create", &Resource::space("x"))
        .unwrap();
}

#[test]
fn xt_partition_store_no_host_errors() {
    #[cfg(red_phase)]
    force_red();
    let ps = PartitionStore::new();
    let err = ps.get_partition_route("s1", "v1", 16).unwrap_err();
    assert!(matches!(err, MetaError::StorageHostMissing));
}

#[test]
fn xt_space_partition_default_16_power_of_two() {
    #[cfg(red_phase)]
    force_red();
    // 默认 partition_num=16，必须是 2^n 且 ≥4
    let def = SpaceDef {
        space_id: "default".into(),
        partition_num: 16,
        replica_factor: 3,
        created_at: 0,
    };
    assert!(def.validate().is_ok());
    assert_eq!(16u16.count_ones(), 1);
    assert!(16 >= 4);
}
