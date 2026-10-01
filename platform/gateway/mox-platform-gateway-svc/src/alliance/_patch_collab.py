# -*- coding: utf-8 -*-
"""experts_collaboration.rs：handler 接入 TenantId，reg 读面取租户内层。"""
import io

p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_collaboration.rs"
s = io.open(p, "r", encoding="utf-8").read()

# 1. 所有 handler 签名：State(state) 后插 TenantId
n1 = s.count("    State(state): State<Arc<ExpertsSharedState>>,\n")
s = s.replace(
    "    State(state): State<Arc<ExpertsSharedState>>,\n",
    "    State(state): State<Arc<ExpertsSharedState>>,\n    TenantId(tenant): TenantId,\n")
print("signatures patched:", n1)

# 2. reg lock：取租户内层（8 空格锚点，覆盖嵌套行）
n2 = s.count("let reg = state.registry.lock();")
s = s.replace(
    "let reg = state.registry.lock();",
    "let all_reg = state.registry.lock();\n        let reg = all_reg.get(tenant.as_str()).unwrap_or(empty_registry());")
print("reg locks patched:", n2)

io.open(p, "w", encoding="utf-8", newline="").write(s)
print("DONE")
