# -*- coding: utf-8 -*-
import io
p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_orchestration.rs"
s = io.open(p, "r", encoding="utf-8").read()
n1 = s.count("    State(state): State<Arc<ExpertsSharedState>>,\n")
s = s.replace(
    "    State(state): State<Arc<ExpertsSharedState>>,\n",
    "    State(state): State<Arc<ExpertsSharedState>>,\n    TenantId(tenant): TenantId,\n")
print("signatures:", n1)
n2 = s.count("let registry = state.registry.lock();")
s = s.replace(
    "let registry = state.registry.lock();",
    "let all_reg = state.registry.lock();\n        let registry = all_reg.get(tenant.as_str()).unwrap_or(empty_registry());")
print("registry locks:", n2)
io.open(p, "w", encoding="utf-8", newline="").write(s)
print("DONE")
