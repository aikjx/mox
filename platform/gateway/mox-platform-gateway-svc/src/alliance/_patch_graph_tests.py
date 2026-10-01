# -*- coding: utf-8 -*-
"""experts_graph.rs 测试：crud_state 图归 default 租户；handler 调用补 TenantId；断言取 default 租户图。"""
import io

p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_graph.rs"
s = io.open(p, "r", encoding="utf-8").read()

def rep(old, new, expect=1):
    global s
    n = s.count(old)
    assert n == expect, f"expected {expect}, got {n}: {old[:70]!r}"
    s = s.replace(old, new)
    print("ok:", old.splitlines()[0][:60])

# 1. crud_state：图归入 default 租户
rep("            graph: Arc::new(Mutex::new(make_test_graph())),",
    "            graph: Arc::new(Mutex::new(HashMap::from([(\"default\".to_string(), make_test_graph())]))),")

# 2. handler 调用：State(state.clone()) 后插 TenantId（同行/多行均适用）
rep("State(state.clone()),",
    "State(state.clone()), TenantId(\"default\".into()),", expect=16)

# 3. 断言：state.graph.lock().xxx → 取 default 租户
rep("state.graph.lock().",
    "state.graph.lock().get(\"default\").unwrap().", expect=10)

# 4. 独立 let g = state.graph.lock();
rep("        let g = state.graph.lock();",
    "        let g = state.graph.lock().get(\"default\").unwrap();")

io.open(p, "w", encoding="utf-8", newline="").write(s)
print("ALL DONE")
