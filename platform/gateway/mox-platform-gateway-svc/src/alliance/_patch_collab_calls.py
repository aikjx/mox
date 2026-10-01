# -*- coding: utf-8 -*-
import io
p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_collaboration.rs"
s = io.open(p, encoding="utf-8").read()
old = "                State(state.clone()),\n"
new = "                State(state.clone()),\n                TenantId(\"default\".into()),\n"
n = s.count(old)
s = s.replace(old, new)
io.open(p, "w", encoding="utf-8", newline="").write(s)
print("collab test calls", n)
