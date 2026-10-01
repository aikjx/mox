# -*- coding: utf-8 -*-
"""experts_graph.rs：所有 handler 接入 TenantId，graph 读面取租户图、写面 entry，落库/enforce/audit 加 tenant。"""
import io

p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_graph.rs"
s = io.open(p, "r", encoding="utf-8").read()

def rep(old, new, expect=1):
    global s
    n = s.count(old)
    assert n == expect, f"expected {expect}, got {n}: {old[:70]!r}"
    s = s.replace(old, new)
    print("ok:", old.splitlines()[0][:60])

# ---- A. 签名加 TenantId ----
rep("async fn get_graph(State(state): State<Arc<ExpertsSharedState>>) -> ApiResponse<Value> {",
    "async fn get_graph(State(state): State<Arc<ExpertsSharedState>>, TenantId(tenant): TenantId) -> ApiResponse<Value> {")
rep("async fn get_graph_stats(State(state): State<Arc<ExpertsSharedState>>) -> ApiResponse<Value> {",
    "async fn get_graph_stats(State(state): State<Arc<ExpertsSharedState>>, TenantId(tenant): TenantId) -> ApiResponse<Value> {")
rep(
'''async fn get_neighbors(
    Path(id): Path<String>,
    State(state): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {''',
'''async fn get_neighbors(
    Path(id): Path<String>,
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
) -> ApiResponse<Value> {''')
rep(
'''async fn get_collaborators(
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {''',
'''async fn get_collaborators(
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
) -> ApiResponse<Value> {''')
rep(
'''async fn get_path(
    Path((source, target)): Path<(String, String)>,
    State(state): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {''',
'''async fn get_path(
    Path((source, target)): Path<(String, String)>,
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
) -> ApiResponse<Value> {''')
rep("async fn get_communities(State(state): State<Arc<ExpertsSharedState>>) -> ApiResponse<Value> {",
    "async fn get_communities(State(state): State<Arc<ExpertsSharedState>>, TenantId(tenant): TenantId) -> ApiResponse<Value> {")
rep(
'''async fn post_optimal_team(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<OptimalTeamBody>,
) -> ApiResponse<Value> {
    let registry = state.registry.lock();''',
'''async fn post_optimal_team(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    Json(body): Json<OptimalTeamBody>,
) -> ApiResponse<Value> {
    let all_reg = state.registry.lock();
    let registry = all_reg.get(tenant.as_str()).unwrap_or(empty_registry());''')
rep(
'''async fn post_rebuild(
    State(state): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
) -> ApiResponse<Value> {''',
'''async fn post_rebuild(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
) -> ApiResponse<Value> {''')

# write handlers signatures
rep(
'''async fn post_graph_node(
    State(state): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<CreateNodeBody>,
) -> ApiResponse<Value> {''',
'''async fn post_graph_node(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<CreateNodeBody>,
) -> ApiResponse<Value> {''')
rep(
'''async fn put_graph_node(
    Path(id): Path<String>,
    State(state): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<UpdateNodeBody>,
) -> ApiResponse<Value> {''',
'''async fn put_graph_node(
    Path(id): Path<String>,
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<UpdateNodeBody>,
) -> ApiResponse<Value> {''')
rep(
'''async fn delete_graph_node(
    Path(id): Path<String>,
    State(state): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
) -> ApiResponse<Value> {''',
'''async fn delete_graph_node(
    Path(id): Path<String>,
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
) -> ApiResponse<Value> {''')
rep(
'''async fn post_graph_edge(
    State(state): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<CreateEdgeBody>,
) -> ApiResponse<Value> {''',
'''async fn post_graph_edge(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<CreateEdgeBody>,
) -> ApiResponse<Value> {''')
rep(
'''async fn put_graph_edge(
    Path(seq): Path<i64>,
    State(state): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<UpdateEdgeBody>,
) -> ApiResponse<Value> {''',
'''async fn put_graph_edge(
    Path(seq): Path<i64>,
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<UpdateEdgeBody>,
) -> ApiResponse<Value> {''')
rep(
'''async fn delete_graph_edge(
    Path(seq): Path<i64>,
    State(state): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
) -> ApiResponse<Value> {''',
'''async fn delete_graph_edge(
    Path(seq): Path<i64>,
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
) -> ApiResponse<Value> {''')
rep(
'''async fn post_rag_expand(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<RagExpandBody>,
) -> ApiResponse<Value> {''',
'''async fn post_rag_expand(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    Json(body): Json<RagExpandBody>,
) -> ApiResponse<Value> {''')

# ---- B. enforce_admin 加 tenant（6 处 MutateGraph + 1 Rebuild）----
rep("enforce_admin_or_respond(&state, &user, RbacAction::RebuildGraph)",
    "enforce_admin_or_respond(&state, &user, tenant.as_str(), RbacAction::RebuildGraph)")
rep("enforce_admin_or_respond(&state, &user, RbacAction::MutateGraph)",
    "enforce_admin_or_respond(&state, &user, tenant.as_str(), RbacAction::MutateGraph)", expect=6)

# ---- C. post_rebuild 内部块（8 空格缩进）先整块替换，避免与 CRUD 写锁混淆 ----
rep(
'''    {
        let registry = state.registry.lock();
        let mut graph = state.graph.lock();
        previous_version = graph.version;
        new_graph = build_graph_from_registry(&registry);
        *graph = ExpertGraph {
            version: previous_version + 1,
            ..new_graph
        };
        save_graph(&graph);
    }''',
'''    {
        let all_reg = state.registry.lock();
        let registry = all_reg.get(tenant.as_str()).unwrap_or(empty_registry());
        let mut all_g = state.graph.lock();
        let graph = all_g.entry(tenant.0.clone()).or_default();
        previous_version = graph.version;
        new_graph = build_graph_from_registry(registry);
        *graph = ExpertGraph {
            version: previous_version + 1,
            ..new_graph
        };
        save_graph(tenant.as_str(), graph);
    }''')

# ---- D. CRUD 写面 graph lock：entry（6 个，均函数体 4 空格缩进）----
rep("    let mut graph = state.graph.lock();",
    "    let mut all_g = state.graph.lock();\n    let graph = all_g.entry(tenant.0.clone()).or_default();", expect=6)

# ---- E. 读面 graph lock：取租户图（全部 `let graph = state.graph.lock();`）----
rep("    let graph = state.graph.lock();",
    "    let all_g = state.graph.lock();\n    let graph = all_g.get(tenant.as_str()).unwrap_or(empty_graph());", expect=14)

# ---- F. 增量落库加 tenant ----
rep("crate::alliance::experts_db::upsert_graph_node(&node);",
    "crate::alliance::experts_db::upsert_graph_node(tenant.as_str(), &node);")
rep("crate::alliance::experts_db::upsert_graph_node(&node_clone);",
    "crate::alliance::experts_db::upsert_graph_node(tenant.as_str(), &node_clone);")
rep("crate::alliance::experts_db::delete_graph_node_cascade(&id);",
    "crate::alliance::experts_db::delete_graph_node_cascade(tenant.as_str(), &id);")
rep("crate::alliance::experts_db::replace_graph_edges(&edges_snapshot);",
    "crate::alliance::experts_db::replace_graph_edges(tenant.as_str(), &edges_snapshot);", expect=2)
rep("crate::alliance::experts_db::upsert_graph_edge(seq, &edge);",
    "crate::alliance::experts_db::upsert_graph_edge(tenant.as_str(), seq, &edge);")
rep("crate::alliance::experts_db::upsert_graph_edge(seq, &edge_clone);",
    "crate::alliance::experts_db::upsert_graph_edge(tenant.as_str(), seq, &edge_clone);")

# ---- G. emit_audit 加 tenant（&state, &actor..., 后插 tenant）----
# 六处，模式统一：`&actor_from_opt_user(&user),\n        AuditAction::` → `&actor_from_opt_user(&user),\n        tenant.as_str(),\n        AuditAction::`
rep("""        &actor_from_opt_user(&user),
        AuditAction::""",
    """        &actor_from_opt_user(&user),
        tenant.as_str(),
        AuditAction::""", expect=6)

io.open(p, "w", encoding="utf-8", newline="").write(s)
print("ALL DONE")
