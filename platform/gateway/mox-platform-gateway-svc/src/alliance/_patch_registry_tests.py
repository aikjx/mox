# -*- coding: utf-8 -*-
"""registry.rs 测试：state 构造、seed_expert、handler 调用补 TenantId、外层 map 断言。"""
import io

p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_registry.rs"
s = io.open(p, "r", encoding="utf-8").read()

def rep(old, new, expect=1):
    global s
    n = s.count(old)
    assert n == expect, f"expected {expect}, got {n}: {old[:60]!r}"
    s = s.replace(old, new)
    print("ok:", old.splitlines()[0][:50])

# 1. state graph 字段
rep(
    "            graph: Arc::new(Mutex::new(ExpertGraph::default())),",
    "            graph: Arc::new(Mutex::new(HashMap::new())),")

# 2. seed_expert 取默认租户内层
rep(
'''    fn seed_expert(state: &Arc<ExpertsSharedState>, id: &str, name: &str, domains: Vec<&str>) {
        let mut reg = state.registry.lock();
        let mut exp = ExpertDescriptor::minimal(id.into(), name.into());''',
'''    fn seed_expert(state: &Arc<ExpertsSharedState>, id: &str, name: &str, domains: Vec<&str>) {
        let mut all = state.registry.lock();
        let reg = all.entry("default".to_string()).or_default();
        let mut exp = ExpertDescriptor::minimal(id.into(), name.into());''')

# 3. handler 调用补 TenantId("default")
rep(
    "create_expert(State(state.clone()), admin_user(), Json(body)).await;",
    "create_expert(State(state.clone()), TenantId(\"default\".into()), admin_user(), Json(body)).await;")
rep(
    "get_expert(State(state), Path(\"nonexistent-999\".into())).await;",
    "get_expert(State(state), TenantId(\"default\".into()), Path(\"nonexistent-999\".into())).await;")
rep(
    "update_expert(State(state.clone()), admin_user(), Path(\"exp-update-001\".into()), Json(body)).await;",
    "update_expert(State(state.clone()), TenantId(\"default\".into()), admin_user(), Path(\"exp-update-001\".into()), Json(body)).await;")
rep(
    "delete_expert(State(state.clone()), admin_user(), Path(\"exp-del-001\".into())).await;",
    "delete_expert(State(state.clone()), TenantId(\"default\".into()), admin_user(), Path(\"exp-del-001\".into())).await;")
rep(
    "list_experts(State(state), Query(params)).await",
    "list_experts(State(state), TenantId(\"default\".into()), Query(params)).await",
    expect=2)
rep(
    "let resp = platform_metrics(State(state)).await;",
    "let resp = platform_metrics(State(state), TenantId(\"default\".into())).await;")

# 4. test_create_expert 断言外层 map
rep(
'''        // 验证持久化到注册表
        let reg = state.registry.lock();
        let id = d["id"].as_str().unwrap();
        assert!(reg.contains_key(id));''',
'''        // 验证持久化到默认租户注册表
        let reg = state.registry.lock();
        let id = d["id"].as_str().unwrap();
        assert!(reg.get("default").unwrap().contains_key(id));''')

# 5. test_soft_delete 断言外层 map
rep(
'''        // 验证 enabled=false 且 deleted_at 已记录
        let reg = state.registry.lock();
        let exp = reg.get("exp-del-001").unwrap();''',
'''        // 验证 enabled=false 且 deleted_at 已记录
        let reg = state.registry.lock();
        let exp = reg.get("default").unwrap().get("exp-del-001").unwrap();''')

io.open(p, "w", encoding="utf-8", newline="").write(s)
print("ALL DONE")
