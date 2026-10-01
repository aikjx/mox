# -*- coding: utf-8 -*-
import io
p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_session.rs"
s = io.open(p, encoding="utf-8").read()
old = "(State(state.clone()), "
n = s.count(old)
s = s.replace(old, '(State(state.clone()), TenantId("default".into()), ')
io.open(p, "w", encoding="utf-8", newline="").write(s)
print("inserted after State(state.clone()), :", n)
# also handle single-arg: handler(State(state.clone()))
import re
