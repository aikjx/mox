# -*- coding: utf-8 -*-
import io
p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_session.rs"
s = io.open(p, encoding="utf-8").read()
old = "&actor_from_opt_user(&user), AuditAction::"
n = s.count(old)
s = s.replace(old, "&actor_from_opt_user(&user), tenant, AuditAction::")
io.open(p, "w", encoding="utf-8", newline="").write(s)
print("session emit_audit patched", n)
