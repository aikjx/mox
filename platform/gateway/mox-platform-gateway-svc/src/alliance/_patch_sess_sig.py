# -*- coding: utf-8 -*-
import io
p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_session.rs"
s = io.open(p, encoding="utf-8").read()
old = "    State(state): State<Arc<ExpertsSharedState>>,\n"
n = s.count(old)
s = s.replace(old, "    State(state): State<Arc<ExpertsSharedState>>,\n    TenantId(tenant): TenantId,\n")
io.open(p, "w", encoding="utf-8", newline="").write(s)
print("session handlers got TenantId:", n)
